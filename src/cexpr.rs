// Tip denetleyici: ifadeler.

use crate::ast::*;
use crate::check::*;
use crate::diag::{err, CResult, Diag, Span};
use crate::lexer::{Mode, BUILTIN_NAMES};
use crate::tir::*;
use crate::types::*;
use num_bigint::BigInt;
use num_traits::ToPrimitive;

pub(crate) enum PathRes {
    Module(usize),
    Type(TypeId),
}

pub(crate) struct PlaceInfo {
    pub place: TPlace,
    /// Aynı yerin değerini okuyan ifade (alıcı tekrar denetlenmesin diye)
    pub expr: TExpr,
    pub ty: Type,
    pub immut: Option<String>,
    pub rooted_bu: bool,
}

pub(crate) struct BInfo {
    pub bm: BM,
    pub params: Vec<(String, Type)>,
    pub ret: Type,
    pub mutating: bool,
    pub resizes: bool,
}

const BUILTIN_FUNCS: &[&str] = &["yazdır", "oku"];

fn is_num_lit(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Int(_) | ExprKind::Float(_))
}

/// Sonlu bir sabit, verilen boyuttaki ondalık sayıya sonsuza dönüşmeden sığar mı?
fn float_fits(x: f64, bits: u32) -> bool {
    !x.is_finite() || crate::values::round_float(x, bits).is_finite()
}

impl Checker {
    pub(crate) fn loc(&self, span: Span) -> Loc {
        Loc { file: span.file, line: span.line }
    }

    pub(crate) fn mk(&self, kind: TK, ty: Type, span: Span) -> TExpr {
        TExpr { kind, ty, loc: self.loc(span) }
    }

    pub(crate) fn shown(&self, name: &str) -> String {
        if self.mode() == Mode::Us { to_us(name) } else { name.to_string() }
    }

    // ------------------------------------------------------------ değişkenler

    pub(crate) fn lookup_var(&self, name: &str) -> Option<VarInfo> {
        for s in self.cx.scopes.iter().rev() {
            if let Some(v) = s.get(name) {
                return Some(v.clone());
            }
        }
        self.cx.snapshot.as_ref().and_then(|s| s.get(name).cloned())
    }

    pub(crate) fn flow_get(&self, key: usize) -> FlowSt {
        if key == ALWAYS { FlowSt::Assigned } else { self.cx.flow[key] }
    }

    pub(crate) fn flow_set(&mut self, key: usize, st: FlowSt) {
        if key != ALWAYS {
            self.cx.flow[key] = st;
        }
    }

    pub(crate) fn new_slot(&mut self) -> Slot {
        if self.cx.func.is_none() {
            self.nglobals += 1;
            Slot::Global(self.nglobals - 1)
        } else {
            self.cx.nslots += 1;
            Slot::Local(self.cx.nslots - 1)
        }
    }

    pub(crate) fn declare_var(&mut self, name: &str, ty: Type, sabit: bool, assigned: bool) -> VarInfo {
        let slot = self.new_slot();
        let key = self.cx.flow.len();
        self.cx.flow.push(if assigned { FlowSt::Assigned } else { FlowSt::Unassigned });
        let info = VarInfo { slot, ty, sabit, key, loop_depth: self.cx.loop_depth };
        self.cx.scopes.last_mut().unwrap().insert(name.to_string(), info.clone());
        info
    }

    pub(crate) fn check_readable(&self, name: &str, v: &VarInfo, span: Span) -> CResult<()> {
        match self.flow_get(v.key) {
            FlowSt::Assigned => Ok(()),
            FlowSt::Unassigned => err(span, format!("'{name}' henüz belirsiz. Kullanmadan önce bir değer atayın.")),
            FlowSt::Maybe => err(
                span,
                format!("'{name}' bazı durumlarda belirsiz kalabiliyor: her yolda bir değer atanmıyor. Kullanmadan önce her durumda bir değer atayın."),
            ),
        }
    }

    pub(crate) fn owner_field(&self, name: &str) -> Option<FieldInfo> {
        let owner = self.cx.owner?;
        self.fields_of(owner).iter().find(|f| &*f.name == name).cloned()
    }

    fn visible_names(&self) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for s in &self.cx.scopes {
            v.extend(s.keys().cloned());
        }
        if let Some(s) = &self.cx.snapshot {
            v.extend(s.keys().cloned());
        }
        let m = &self.mods[self.cx.module];
        v.extend(m.funcs.keys().cloned());
        v.extend(m.types.keys().cloned());
        v.extend(BUILTIN_FUNCS.iter().map(|s| s.to_string()));
        if let Some(o) = self.cx.owner {
            v.extend(self.fields_of(o).iter().map(|f| f.name.to_string()));
            v.extend(self.types[o].methods.keys().cloned());
        }
        v
    }

    pub(crate) fn unknown_name(&self, name: &str, span: Span) -> Diag {
        if self.mode() == Mode::Tr {
            for (tr, us) in BUILTIN_NAMES {
                if name == *us {
                    return Diag::new(
                        span,
                        format!("'{name}' tanımlı değil. '{tr}' mı demek istediniz? (Türkçe karakter olmadan yazmak için dosyanın başına klavye(\"us\") ekleyin.)"),
                    );
                }
            }
        }
        if name == "klavye" {
            return Diag::new(span, "'klavye(...)' sadece dosyanın ilk satırında kullanılabilir.");
        }
        if self.cx.func.is_some() && self.mods[self.cx.module].globals.contains_key(name) {
            return Diag::new(
                span,
                format!("'{name}' bu fonksiyondan sonra tanımlanıyor. Fonksiyonlar sadece kendilerinden önce tanımlanan değişkenleri görebilir; 'ata {name}' satırını fonksiyonun üstüne taşıyın."),
            );
        }
        match closest(name, self.visible_names().iter()) {
            Some(s) => Diag::new(span, format!("'{name}' tanımlı değil. '{}' mı demek istediniz?", self.shown(&s))),
            None => Diag::new(span, format!("'{name}' tanımlı değil.")),
        }
    }

    // ------------------------------------------------------------ tip uyumu

    pub(crate) fn assignable(&self, from: &Type, to: &Type) -> bool {
        if from == to || *from == Type::Never {
            return true;
        }
        match (from, to) {
            // 'veya' içindeki değerler aynı biçimde saklandığı için sadece tam sayı genişletme ve
            // üst türe çevirme serbest (Tam -> Ondalık gibi biçim değiştiren çeviriler değil).
            (Type::Union(fs), Type::Union(ts)) => fs.iter().all(|f| {
                ts.contains(f)
                    || ts.iter().any(|t| match (f, t) {
                        (Type::Named(a), Type::Named(b)) => self.is_subtype(*a, *b),
                        (Type::Int { bits: a, sign: sa }, Type::Int { bits: b, sign: sb }) => int_subset((*a, *sa), (*b, *sb)),
                        _ => false,
                    })
            }),
            (Type::Union(_), _) => false,
            (_, Type::Union(ms)) => ms.iter().any(|m| self.assignable(from, m)),
            (Type::Int { bits: a, sign: sa }, Type::Int { bits: b, sign: sb }) => int_subset((*a, *sa), (*b, *sb)),
            // Tam -> Ondalık(64) otomatik; Ondalık(16/32) sığmayabileceği için 'olarak' gerekir.
            (Type::Int { .. }, Type::Float(b)) => *b == 64,
            (Type::Float(a), Type::Float(b)) => a <= b,
            (Type::Named(c), Type::Named(p)) => self.is_subtype(*c, *p),
            // Esnek liste sabit boyutlu listeye atanabilir; boyutu çalışma anında kontrol edilir.
            (Type::List(a, sa), Type::List(b, sb)) => a == b && (sa.is_none() || sb.is_none() || sa == sb),
            _ => false,
        }
    }

    pub(crate) fn coerce(&self, e: TExpr, to: &Type, span: Span) -> CResult<TExpr> {
        if e.ty == *to {
            return Ok(e);
        }
        if e.ty == Type::Never {
            return Ok(TExpr { ty: to.clone(), ..e });
        }
        if !self.assignable(&e.ty, to) {
            return Err(self.mismatch(&e, to, span));
        }
        if let (Type::Union(ms), false) = (to, matches!(e.ty, Type::Union(_))) {
            if !ms.contains(&e.ty) {
                let m = ms.iter().find(|m| self.assignable(&e.ty, m)).unwrap().clone();
                let inner = self.coerce(e, &m, span)?;
                let loc = inner.loc;
                return Ok(TExpr { kind: TK::Convert(Box::new(inner)), ty: to.clone(), loc });
            }
        }
        if let (TK::List(items), Type::List(_, Some(n))) = (&e.kind, to) {
            if items.len() != *n {
                return err(span, format!("{} tam {n} eleman almalı, ama {} eleman verildi.", self.type_name(to), items.len()));
            }
        }
        if let TK::List(_) | TK::Dict(_) = e.kind {
            if let (Type::List(a, _), Type::List(b, _)) = (&e.ty, to) {
                if a == b {
                    return Ok(TExpr { ty: to.clone(), ..e });
                }
            }
        }
        let loc = e.loc;
        Ok(TExpr { kind: TK::Convert(Box::new(e)), ty: to.clone(), loc })
    }

    pub(crate) fn mismatch(&self, e: &TExpr, to: &Type, span: Span) -> Diag {
        let from = &e.ty;
        let (f, t) = (self.type_name(from), self.type_name(to));
        let msg = match (from, to) {
            (Type::Hic, _) if matches!(e.kind, TK::Call(..) | TK::Method { .. } | TK::Direct { .. } | TK::Builtin(..)) => {
                "Bu fonksiyon bir değer döndürmüyor, sonucu kullanılamaz.".to_string()
            }
            (Type::Int { .. }, Type::Int { .. }) => {
                format!("{f} değeri {t} tipine otomatik çevrilemez (veri kaybı olabilir). Bilerek çevirmek için: x olarak {t}")
            }
            (Type::Float(_), Type::Int { .. }) => {
                format!("Ondalık değer {t} tipine otomatik çevrilemez. Bilerek çevirmek için: x olarak {t}")
            }
            (Type::Float(_), Type::Float(_)) => {
                format!("{f} değeri {t} tipine otomatik çevrilemez (hassasiyet kaybı olabilir). Bilerek çevirmek için: x olarak {t}")
            }
            (Type::Int { .. }, Type::Float(_)) => {
                format!("{f} değeri {t} tipine otomatik çevrilemez ({t} küçük bir tip, değer sığmayabilir). Bilerek çevirmek için: x olarak {t}")
            }
            (Type::Union(ms), _) if ms.contains(&Type::Hic) && self.assignable(&make_union(ms.iter().filter(|m| **m != Type::Hic).cloned().collect()), to) => {
                format!("Bu değer Hiç olabilir ({f}). Kullanmadan önce eşleştir ile kontrol edin.")
            }
            (Type::Union(_), _) => format!("{f} tipindeki değer {t} olarak kullanılamaz. Önce eşleştir ile kontrol edin."),
            (Type::List(a, Some(x)), Type::List(b, Some(y))) if a == b && x != y => {
                format!("{f} değeri {t} tipine atanamaz: biri {x}, diğeri {y} elemanlı.")
            }
            _ => format!("Tip uyuşmuyor: {t} bekleniyordu, ama {f} verildi."),
        };
        Diag::new(span, msg)
    }

    fn num_join(&self, a: &Type, b: &Type) -> Type {
        match (a, b) {
            (Type::Int { bits: x, sign: sx }, Type::Int { bits: y, sign: sy }) => {
                let (bits, sign) = int_join((*x, *sx), (*y, *sy));
                Type::Int { bits, sign }
            }
            (Type::Float(x), Type::Float(y)) => Type::Float(*x.max(y)),
            (Type::Float(x), _) | (_, Type::Float(x)) => Type::Float(*x),
            _ => a.clone(),
        }
    }

    // ------------------------------------------------------------ ifadeler

    pub(crate) fn expr(&mut self, e: &Expr, expected: Option<&Type>) -> CResult<TExpr> {
        let span = e.span;
        Ok(match &e.kind {
            ExprKind::Int(n) => self.int_lit(n, expected, span)?,
            ExprKind::Float(f) => {
                let ty = match expected {
                    Some(Type::Float(b)) => Type::Float(*b),
                    Some(Type::Union(ms)) => ms.iter().find(|m| m.is_float()).cloned().unwrap_or(Type::Float(64)),
                    _ => Type::Float(64),
                };
                let Type::Float(bits) = ty else { unreachable!() };
                if !float_fits(*f, bits) {
                    return err(span, format!("{f}, {} sınırlarını aşıyor ({}).", self.type_name(&ty), float_limit_text(bits)));
                }
                self.mk(TK::Float(*f), ty, span)
            }
            ExprKind::Str(s) => self.mk(TK::Str(std::rc::Rc::new(s.clone())), Type::Str, span),
            ExprKind::FStr(parts) => {
                let mut out = Vec::new();
                for p in parts {
                    match p {
                        FPartE::Lit(s) => out.push(TFPart::Lit(s.as_str().into())),
                        FPartE::Expr(x) => {
                            let t = self.expr(x, None)?;
                            if t.ty == Type::Hic && !matches!(t.kind, TK::Hic) {
                                return Err(self.mismatch(&t, &Type::Str, x.span));
                            }
                            out.push(TFPart::Expr(t));
                        }
                    }
                }
                self.mk(TK::FStr(out), Type::Str, span)
            }
            ExprKind::Bool(b) => self.mk(TK::Bool(*b), Type::Bool, span),
            ExprKind::Hic => self.mk(TK::Hic, Type::Hic, span),
            ExprKind::Bu => {
                let Some(o) = self.cx.owner else {
                    return err(span, "'bu' sadece bir tür ya da seçenek içindeki fonksiyonlarda kullanılabilir.");
                };
                self.mk(TK::Bu, Type::Named(o), span)
            }
            ExprKind::Ident(name) => self.ident_expr(name, span)?,
            ExprKind::List(items) => self.list_lit(items, expected, span)?,
            ExprKind::Dict(items) => self.dict_lit(items, expected, span)?,
            ExprKind::Neg(x) => {
                let t = self.expr(x, expected.filter(|t| t.is_num()))?;
                if !t.ty.is_num() {
                    return err(span, format!("'-' sadece sayılarla kullanılabilir, ama {} verildi.", self.type_name(&t.ty)));
                }
                let ty = t.ty.clone();
                self.mk(TK::Neg(Box::new(t)), ty, span)
            }
            ExprKind::Not(x) => {
                let t = self.expr(x, Some(&Type::Bool))?;
                let t = self.bool_cond(t, x.span, "'değil'")?;
                self.mk(TK::Not(Box::new(t)), Type::Bool, span)
            }
            ExprKind::Binary(op, l, r) => self.binary(*op, l, r, span, expected)?,
            ExprKind::Range { start, end, inclusive } => {
                let bound = |x: &Option<Box<Expr>>, me: &mut Self| -> CResult<Option<Box<TExpr>>> {
                    match x {
                        None => Ok(None),
                        Some(x) => {
                            let t = me.expr(x, Some(&TAM64))?;
                            if !t.ty.is_int() {
                                return err(x.span, format!("Aralık sınırları tam sayı olmalı, ama {} verildi.", me.type_name(&t.ty)));
                            }
                            if !me.assignable(&t.ty, &TAM64) {
                                return err(x.span, "Aralık sınırları Tam(64)'e sığmalı.");
                            }
                            Ok(Some(Box::new(me.coerce(t, &TAM64, x.span)?)))
                        }
                    }
                };
                let s = bound(start, self)?;
                let en = bound(end, self)?;
                self.mk(TK::Range { start: s, end: en, inclusive: *inclusive }, Type::Range, span)
            }
            ExprKind::Call { callee, args } => self.call_expr(callee, args, span)?,
            ExprKind::Field(base, name, nspan) => self.field_expr(base, name, *nspan, span)?,
            ExprKind::Index(base, idx) => self.index_expr(base, idx, span)?,
            ExprKind::Olarak(x, te) => self.olarak(x, te, span)?,
            ExprKind::Match(m) => {
                let (tm, ty, _) = self.match_(m, true, expected)?;
                self.mk(TK::Match(Box::new(tm)), ty, span)
            }
        })
    }

    pub(crate) fn bool_cond(&self, t: TExpr, span: Span, what: &str) -> CResult<TExpr> {
        if t.ty == Type::Bool || t.ty == Type::Never {
            return Ok(t);
        }
        let extra = if t.ty.is_int() { " Bir işi N kez tekrarlamak için '5 kere yap' yazın." } else { "" };
        err(
            span,
            format!("{what} bir Mantık (doğru/yanlış) değeri bekler, ama {} verildi.{extra}", self.type_name(&t.ty)),
        )
    }

    fn int_lit(&self, n: &BigInt, expected: Option<&Type>, span: Span) -> CResult<TExpr> {
        match expected {
            Some(Type::Int { bits, sign }) => {
                if !fits(n, *bits, *sign) {
                    let t = Type::Int { bits: *bits, sign: *sign };
                    return err(
                        span,
                        format!("{}, {} sınırlarını aşıyor ({}).", group_digits(&n.to_string()), self.type_name(&t), bounds_text(*bits, *sign)),
                    );
                }
                Ok(self.mk(TK::Int(n.clone(), n.to_i64()), Type::Int { bits: *bits, sign: *sign }, span))
            }
            Some(Type::Float(b)) => {
                let x = n.to_f64().unwrap_or(f64::INFINITY);
                if !x.is_finite() || !float_fits(x, *b) {
                    let t = Type::Float(*b);
                    return err(span, format!("{}, {} sınırlarını aşıyor ({}).", group_digits(&n.to_string()), self.type_name(&t), float_limit_text(*b)));
                }
                Ok(self.mk(TK::Float(x), Type::Float(*b), span))
            }
            Some(Type::Union(ms)) => {
                if let Some(m) = ms.iter().find(|m| m.is_int()) {
                    return self.int_lit(n, Some(m), span);
                }
                if let Some(m) = ms.iter().find(|m| m.is_float()) {
                    return self.int_lit(n, Some(m), span);
                }
                self.int_lit(n, None, span)
            }
            _ => {
                let ty = if fits(n, Some(64), Sign::Signed) { TAM64 } else { ESNEK };
                Ok(self.mk(TK::Int(n.clone(), n.to_i64()), ty, span))
            }
        }
    }

    fn ident_expr(&mut self, name: &str, span: Span) -> CResult<TExpr> {
        if let Some(v) = self.lookup_var(name) {
            self.check_readable(name, &v, span)?;
            return Ok(self.mk(TK::Var(v.slot), v.ty, span));
        }
        if let Some(f) = self.owner_field(name) {
            let bu = self.mk(TK::Bu, Type::Named(self.cx.owner.unwrap()), span);
            let fid = self.field_id(&f.name);
            return Ok(self.mk(TK::Field(Box::new(bu), fid), f.ty, span));
        }
        let m = &self.mods[self.cx.module];
        if m.funcs.contains_key(name) || BUILTIN_FUNCS.contains(&name) {
            return err(span, format!("'{}' bir fonksiyon; çağırmak için {}(...) yazın.", self.shown(name), self.shown(name)));
        }
        if let Some(&tid) = m.types.get(name) {
            return err(span, match &self.types[tid].kind {
                TypeDefKind::Secenek { variants } => format!("'{name}' bir seçenek; {name}.{} gibi yazın.", variants[0].name),
                _ => format!("'{name}' bir tür; nesne oluşturmak için {name}(...) yazın."),
            });
        }
        if m.namespaces.contains_key(name) {
            return err(span, format!("'{name}' bir dosya adı; {name}.isim şeklinde kullanın."));
        }
        Err(self.unknown_name(name, span))
    }

    fn list_lit(&mut self, items: &[Expr], expected: Option<&Type>, span: Span) -> CResult<TExpr> {
        if let Some(Type::List(elem, size)) = expected {
            if let Some(n) = size {
                if items.len() != *n {
                    return err(
                        span,
                        format!("{} tam {n} eleman almalı, ama {} eleman verildi.", self.type_name(expected.unwrap()), items.len()),
                    );
                }
            }
            let mut vals = Vec::new();
            for it in items {
                let t = self.expr(it, Some(elem))?;
                vals.push(self.coerce(t, elem, it.span)?);
            }
            return Ok(self.mk(TK::List(vals), expected.unwrap().clone(), span));
        }
        if items.is_empty() {
            return err(span, "Boş listenin tipi belli değil. Tipini yazın: ata liste: Liste<Tam> = []");
        }
        let mut typed = Vec::new();
        for it in items {
            typed.push(self.expr(it, None)?);
        }
        let elem = self.unify(&typed.iter().map(|t| t.ty.clone()).collect::<Vec<_>>(), span, "listedeki")?;
        let mut vals = Vec::new();
        for (t, it) in typed.into_iter().zip(items) {
            vals.push(self.coerce(t, &elem, it.span)?);
        }
        Ok(self.mk(TK::List(vals), Type::list(elem), span))
    }

    fn unify(&self, ts: &[Type], span: Span, what: &str) -> CResult<Type> {
        let first = ts[0].clone();
        if ts.iter().all(|t| t.is_num()) {
            return Ok(ts.iter().skip(1).fold(first, |a, b| self.num_join(&a, b)));
        }
        if ts.iter().all(|t| *t == first) {
            return Ok(first);
        }
        if let Some(t) = ts.iter().find(|cand| ts.iter().all(|x| self.assignable(x, cand))) {
            return Ok(t.clone());
        }
        let mut names: Vec<String> = Vec::new();
        for t in ts {
            let n = self.type_name(t);
            if !names.contains(&n) {
                names.push(n);
            }
        }
        err(
            span,
            format!(
                "Bir {what} tüm elemanlar aynı tipte olmalı. {} karıştırılamaz. Karışık liste için tipi yazın: Liste<{}>",
                names.join(", "),
                names.join(" veya ")
            ),
        )
    }

    fn dict_lit(&mut self, items: &[(Expr, Expr)], expected: Option<&Type>, span: Span) -> CResult<TExpr> {
        if let Some(Type::Dict(k, v)) = expected {
            let mut vals = Vec::new();
            for (ke, ve) in items {
                let kt = self.expr(ke, Some(k))?;
                let kt = self.coerce(kt, k, ke.span)?;
                let vt = self.expr(ve, Some(v))?;
                let vt = self.coerce(vt, v, ve.span)?;
                vals.push((kt, vt));
            }
            return Ok(self.mk(TK::Dict(vals), expected.unwrap().clone(), span));
        }
        if items.is_empty() {
            return err(span, "Boş sözlüğün tipi belli değil. Tipini yazın: ata s: Sözlük<Yazı, Tam> = {}");
        }
        let mut ks = Vec::new();
        let mut vs = Vec::new();
        for (ke, ve) in items {
            ks.push(self.expr(ke, None)?);
            vs.push(self.expr(ve, None)?);
        }
        let kt = self.unify(&ks.iter().map(|t| t.ty.clone()).collect::<Vec<_>>(), span, "sözlükteki")?;
        let vt = self.unify(&vs.iter().map(|t| t.ty.clone()).collect::<Vec<_>>(), span, "sözlükteki")?;
        if let Err(why) = self.key_type_check(&kt) {
            return err(span, why);
        }
        let mut vals = Vec::new();
        for ((k, v), (ke, ve)) in ks.into_iter().zip(vs).zip(items) {
            vals.push((self.coerce(k, &kt, ke.span)?, self.coerce(v, &vt, ve.span)?));
        }
        Ok(self.mk(TK::Dict(vals), Type::Dict(Box::new(kt), Box::new(vt)), span))
    }

    fn num_operands(&mut self, l: &Expr, r: &Expr, hint: Option<&Type>, lenient: bool) -> CResult<(TExpr, TExpr)> {
        let try_lit = |me: &mut Self, e: &Expr, h: Option<&Type>| -> CResult<TExpr> {
            // Tam(N, -) ile işlemde pozitif sabit (x - 10): sabitin büyüklüğü sığıyorsa x'in tipinde hesaplanır,
            // sonuç çalışma anında kontrol edilir.
            if let (Some(Type::Int { bits: Some(b), sign: Sign::Neg }), ExprKind::Int(n)) = (h, &e.kind) {
                if fits(n, Some(*b), Sign::Pos) {
                    return Ok(me.mk(TK::Int(n.clone(), n.to_i64()), h.unwrap().clone(), e.span));
                }
            }
            match me.expr(e, h) {
                Ok(t) => Ok(t),
                Err(_) if lenient => me.expr(e, None),
                Err(d) => Err(d),
            }
        };
        let (ll, rl) = (is_num_lit(l), is_num_lit(r));
        if ll && !rl {
            let rt = self.expr(r, None)?;
            let h = if rt.ty.is_num() { Some(rt.ty.clone()) } else { None };
            let lt = try_lit(self, l, h.as_ref())?;
            Ok((lt, rt))
        } else if rl && !ll {
            let lt = self.expr(l, None)?;
            let h = if lt.ty.is_num() { Some(lt.ty.clone()) } else { None };
            let rt = try_lit(self, r, h.as_ref())?;
            Ok((lt, rt))
        } else {
            let h = hint.filter(|t| t.is_num());
            let lt = try_lit(self, l, h)?;
            let rt = try_lit(self, r, h)?;
            Ok((lt, rt))
        }
    }

    fn binary(&mut self, op: BinOp, l: &Expr, r: &Expr, span: Span, expected: Option<&Type>) -> CResult<TExpr> {
        match op {
            BinOp::And | BinOp::Or => {
                let what = if op == BinOp::And { "'ve'" } else { "'veya'" };
                let lt = self.expr(l, Some(&Type::Bool))?;
                let lt = self.bool_cond(lt, l.span, what)?;
                let rt = self.expr(r, Some(&Type::Bool))?;
                let rt = self.bool_cond(rt, r.span, what)?;
                let kind = if op == BinOp::And { TK::And(Box::new(lt), Box::new(rt)) } else { TK::Or(Box::new(lt), Box::new(rt)) };
                Ok(self.mk(kind, Type::Bool, span))
            }
            _ if op.is_arith() => {
                let (lt, rt) = self.num_operands(l, r, expected, true)?;
                if lt.ty == Type::Str || rt.ty == Type::Str {
                    if op == BinOp::Add && lt.ty == Type::Str && rt.ty == Type::Str {
                        return Ok(self.mk(TK::Concat(Box::new(lt), Box::new(rt)), Type::Str, span));
                    }
                    let other = if lt.ty == Type::Str { &rt.ty } else { &lt.ty };
                    if op == BinOp::Add {
                        return err(span, format!("Yazı ile {} toplanamaz. f-yazı kullanın: f\"{{a}}{{b}}\"", self.type_name(other)));
                    }
                    return err(span, format!("'{}' işlemi yazılarla yapılamaz.", op.symbol()));
                }
                for (t, e) in [(&lt, l), (&rt, r)] {
                    if !t.ty.is_num() {
                        if t.ty == Type::Hic && !matches!(t.kind, TK::Hic) {
                            return Err(self.mismatch(t, &TAM64, e.span));
                        }
                        return err(
                            span,
                            format!("'{}' işlemi {} ve {} arasında yapılamaz.", op.symbol(), self.type_name(&lt.ty), self.type_name(&rt.ty)),
                        );
                    }
                }
                let ty = self.num_join(&lt.ty, &rt.ty);
                let lt = self.coerce(lt, &ty, l.span)?;
                let rt = self.coerce(rt, &ty, r.span)?;
                Ok(self.mk(TK::Arith(op, Box::new(lt), Box::new(rt)), ty, span))
            }
            _ => {
                let (lt, rt) = self.num_operands(l, r, None, true)?;
                if lt.ty.is_num() && rt.ty.is_num() {
                    let ty = self.num_join(&lt.ty, &rt.ty);
                    let lt = self.coerce(lt, &ty, l.span)?;
                    let rt = self.coerce(rt, &ty, r.span)?;
                    return Ok(self.mk(TK::Cmp(op, Box::new(lt), Box::new(rt)), Type::Bool, span));
                }
                if matches!(op, BinOp::Eq | BinOp::Ne) {
                    let (lt, rt) = if lt.ty == rt.ty {
                        (lt, rt)
                    } else if self.assignable(&rt.ty, &lt.ty) {
                        let ty = lt.ty.clone();
                        (lt, self.coerce(rt, &ty, r.span)?)
                    } else if self.assignable(&lt.ty, &rt.ty) {
                        let ty = rt.ty.clone();
                        (self.coerce(lt, &ty, l.span)?, rt)
                    } else {
                        return err(span, format!("{} ile {} karşılaştırılamaz.", self.type_name(&lt.ty), self.type_name(&rt.ty)));
                    };
                    return Ok(self.mk(TK::Cmp(op, Box::new(lt), Box::new(rt)), Type::Bool, span));
                }
                if lt.ty == Type::Str && rt.ty == Type::Str {
                    return Ok(self.mk(TK::Cmp(op, Box::new(lt), Box::new(rt)), Type::Bool, span));
                }
                err(
                    span,
                    format!(
                        "'{}' sadece sayılar ve yazılar için kullanılabilir, ama {} ve {} verildi.",
                        op.symbol(),
                        self.type_name(&lt.ty),
                        self.type_name(&rt.ty)
                    ),
                )
            }
        }
    }

    // ------------------------------------------------------------ çağrılar

    pub(crate) fn static_path(&self, e: &Expr) -> Option<PathRes> {
        match &e.kind {
            ExprKind::Ident(n) => {
                if self.lookup_var(n).is_some() || self.owner_field(n).is_some() {
                    return None;
                }
                let m = &self.mods[self.cx.module];
                if let Some(&mi) = m.namespaces.get(n) {
                    return Some(PathRes::Module(mi));
                }
                m.types.get(n).map(|&t| PathRes::Type(t))
            }
            ExprKind::Field(b, n, _) => match self.static_path(b)? {
                PathRes::Module(mi) => {
                    let m = &self.mods[mi];
                    if let Some(&x) = m.namespaces.get(n) {
                        return Some(PathRes::Module(x));
                    }
                    m.types.get(n).map(|&t| PathRes::Type(t))
                }
                PathRes::Type(_) => None,
            },
            _ => None,
        }
    }

    pub(crate) fn bind_args(&mut self, params: &[(String, Type)], args: &[Arg], span: Span, what: &str) -> CResult<Vec<TExpr>> {
        let mut slots: Vec<Option<TExpr>> = vec![None; params.len()];
        let mut pos = 0;
        let mut seen_named = false;
        for a in args {
            let idx = match &a.name {
                None => {
                    if seen_named {
                        return err(a.value.span, "Adı yazılmış değerlerden sonra adsız değer yazılamaz.");
                    }
                    if pos >= params.len() {
                        return err(span, format!("'{what}' {} değer alır, ama {} değer verildi.", params.len(), args.len()));
                    }
                    pos += 1;
                    pos - 1
                }
                Some((n, s)) => {
                    seen_named = true;
                    match params.iter().position(|(p, _)| p == n) {
                        Some(i) => {
                            if slots[i].is_some() {
                                return err(*s, format!("'{n}' iki kez verildi."));
                            }
                            i
                        }
                        None => {
                            let hint = closest(n, params.iter().map(|p| &p.0)).map(|c| format!(" '{c}' mı demek istediniz?")).unwrap_or_default();
                            return err(*s, format!("'{what}' içinde '{n}' diye bir alan ya da parametre yok.{hint}"));
                        }
                    }
                }
            };
            let pty = params[idx].1.clone();
            let t = self.expr(&a.value, Some(&pty))?;
            slots[idx] = Some(self.coerce(t, &pty, a.value.span)?);
        }
        let mut out = Vec::new();
        for (i, s) in slots.into_iter().enumerate() {
            match s {
                Some(t) => out.push(t),
                None => return err(span, format!("'{what}' için '{}' değeri eksik.", params[i].0)),
            }
        }
        Ok(out)
    }

    fn call_expr(&mut self, callee: &Expr, args: &[Arg], span: Span) -> CResult<TExpr> {
        match &callee.kind {
            ExprKind::Ident(name) => {
                if self.lookup_var(name).is_some() || self.owner_field(name).is_some() {
                    return err(callee.span, format!("'{name}' bir değişken, fonksiyon değil."));
                }
                if let Some(o) = self.cx.owner {
                    if self.types[o].methods.contains_key(name) {
                        let bu = Expr { kind: ExprKind::Bu, span: callee.span };
                        return self.method_call(&bu, name, callee.span, args, span);
                    }
                }
                let m = self.cx.module;
                if let Some(&fid) = self.mods[m].funcs.get(name) {
                    return self.call_func(fid, args, span);
                }
                if let Some(&tid) = self.mods[m].types.get(name) {
                    return self.construct(tid, args, span);
                }
                match name.as_str() {
                    "yazdır" => {
                        let mut vals = Vec::new();
                        for a in args {
                            if let Some((n, s)) = &a.name {
                                return err(*s, format!("'{}' adlı değer almaz ('{n}:' yazmayın).", self.shown("yazdır")));
                            }
                            let t = self.expr(&a.value, None)?;
                            if t.ty == Type::Hic && !matches!(t.kind, TK::Hic) {
                                return Err(self.mismatch(&t, &Type::Str, a.value.span));
                            }
                            vals.push(t);
                        }
                        Ok(self.mk(TK::Builtin(Builtin::Yazdir, vals), Type::Hic, span))
                    }
                    "oku" => {
                        if args.len() > 1 {
                            return err(span, "'oku' en fazla bir değer alır: oku(\"Adınız: \")");
                        }
                        let mut vals = Vec::new();
                        if let Some(a) = args.first() {
                            let t = self.expr(&a.value, Some(&Type::Str))?;
                            vals.push(self.coerce(t, &Type::Str, a.value.span)?);
                        }
                        Ok(self.mk(TK::Builtin(Builtin::Oku, vals), Type::Str, span))
                    }
                    _ => Err(self.unknown_name(name, callee.span)),
                }
            }
            ExprKind::Field(base, name, nspan) => {
                match self.static_path(base) {
                    Some(PathRes::Module(mi)) => {
                        if let Some(&fid) = self.mods[mi].funcs.get(name) {
                            return self.call_func(fid, args, span);
                        }
                        if let Some(&tid) = self.mods[mi].types.get(name) {
                            return self.construct(tid, args, span);
                        }
                        return err(*nspan, format!("'{}' dosyasında '{name}' diye bir fonksiyon ya da tür yok.", self.mods[mi].name));
                    }
                    Some(PathRes::Type(tid)) => {
                        if matches!(self.types[tid].kind, TypeDefKind::Secenek { .. }) {
                            return self.variant(tid, name, *nspan, Some(args), span);
                        }
                        let tname = self.types[tid].name.clone();
                        let is_parent = self.cx.owner.is_some_and(|o| o != tid && self.is_subtype(o, tid));
                        if !is_parent || !matches!(self.types[tid].kind, TypeDefKind::Tur { .. }) {
                            return err(
                                span,
                                format!("'{tname}.{name}()' sadece {tname} türünden kalıtım alan bir türün içinde kullanılabilir."),
                            );
                        }
                        return self.parent_call(tid, name, *nspan, args, span);
                    }
                    None => {}
                }
                self.method_call(base, name, *nspan, args, span)
            }
            _ => err(callee.span, "Bu ifade bir fonksiyon değil, çağrılamaz."),
        }
    }

    fn call_func(&mut self, fid: FuncId, args: &[Arg], span: Span) -> CResult<TExpr> {
        let params = self.funcs[fid].params.clone();
        let name = self.funcs[fid].name.clone();
        let vals = self.bind_args(&params, args, span, &name)?;
        let ret = self.funcs[fid].ret.clone();
        Ok(self.mk(TK::Call(fid, vals), ret, span))
    }

    fn construct(&mut self, tid: TypeId, args: &[Arg], span: Span) -> CResult<TExpr> {
        let tname = self.types[tid].name.clone();
        match &self.types[tid].kind {
            TypeDefKind::Tur { fields, .. } => {
                let params: Vec<(String, Type)> = fields.iter().map(|f| (f.name.to_string(), f.ty.clone())).collect();
                let vals = self.bind_args(&params, args, span, &tname)?;
                Ok(self.mk(TK::New(tid, vals), Type::Named(tid), span))
            }
            TypeDefKind::Arayuz { .. } => err(span, format!("'{tname}' bir arayüz; arayüzden nesne oluşturulamaz.")),
            TypeDefKind::Secenek { variants } => {
                err(span, format!("'{tname}' bir seçenek; {tname}.{} gibi yazın.", variants[0].name))
            }
        }
    }

    pub(crate) fn variant(&mut self, tid: TypeId, name: &str, nspan: Span, args: Option<&[Arg]>, span: Span) -> CResult<TExpr> {
        let TypeDefKind::Secenek { variants } = self.types[tid].kind.clone() else { unreachable!() };
        let tname = self.types[tid].name.clone();
        let Some(idx) = variants.iter().position(|v| &*v.name == name) else {
            let names: Vec<String> = variants.iter().map(|v| v.name.to_string()).collect();
            let hint = closest(name, names.iter()).map(|c| format!(" '{c}' mı demek istediniz?")).unwrap_or_default();
            return err(nspan, format!("'{tname}' içinde '{name}' diye bir seçenek yok.{hint}"));
        };
        let v = &variants[idx];
        let vals = match args {
            None if v.fields.is_empty() => vec![],
            None => {
                let fs: Vec<String> = v.fields.iter().map(|f| format!("{}: ...", f.0)).collect();
                return err(span, format!("'{name}' seçeneği değer alır: {tname}.{name}({})", fs.join(", ")));
            }
            Some(a) if v.fields.is_empty() => {
                if !a.is_empty() {
                    return err(span, format!("'{name}' seçeneği değer almaz; {tname}.{name} yazın."));
                }
                vec![]
            }
            Some(a) => {
                let params: Vec<(String, Type)> = v.fields.iter().map(|(n, t)| (n.to_string(), t.clone())).collect();
                self.bind_args(&params, a, span, &format!("{tname}.{name}"))?
            }
        };
        Ok(self.mk(TK::Variant(tid, idx as u32, vals), Type::Named(tid), span))
    }

    fn parent_call(&mut self, ptid: TypeId, name: &str, nspan: Span, args: &[Arg], span: Span) -> CResult<TExpr> {
        let pname = self.types[ptid].name.clone();
        let f = match self.types[ptid].methods.get(name) {
            Some(MethodRes::Impl(f)) => *f,
            Some(MethodRes::Ambiguous(_)) => return err(nspan, format!("'{pname}.{name}' belirsiz.")),
            None => return err(nspan, format!("'{pname}' içinde '{name}' diye bir fonksiyon yok.")),
        };
        let params = self.funcs[f].params.clone();
        let vals = self.bind_args(&params, args, span, name)?;
        if let Some(cur) = self.cx.func {
            self.mut_direct.push((cur, f));
        }
        let ret = self.funcs[f].ret.clone();
        Ok(self.mk(TK::Direct { func: f, recv: Recv::Place(TPlace::Bu), args: vals }, ret, span))
    }

    pub(crate) fn place_of(&mut self, e: &Expr) -> CResult<Option<PlaceInfo>> {
        Ok(match &e.kind {
            ExprKind::Ident(n) => {
                if let Some(v) = self.lookup_var(n) {
                    self.check_readable(n, &v, e.span)?;
                    Some(PlaceInfo {
                        place: TPlace::Var(v.slot),
                        expr: self.mk(TK::Var(v.slot), v.ty.clone(), e.span),
                        ty: v.ty,
                        immut: if v.sabit { Some(format!("'{n}' sabit")) } else { None },
                        rooted_bu: false,
                    })
                } else if let Some(f) = self.owner_field(n) {
                    let fid = self.field_id(&f.name);
                    let bu = self.mk(TK::Bu, Type::Named(self.cx.owner.unwrap()), e.span);
                    Some(PlaceInfo {
                        place: TPlace::Field(Box::new(TPlace::Bu), fid),
                        expr: self.mk(TK::Field(Box::new(bu), fid), f.ty.clone(), e.span),
                        ty: f.ty,
                        immut: if f.sabit { Some(format!("'{n}' alanı sabit")) } else { None },
                        rooted_bu: true,
                    })
                } else {
                    None
                }
            }
            ExprKind::Bu => {
                let Some(o) = self.cx.owner else {
                    return err(e.span, "'bu' sadece bir tür ya da seçenek içindeki fonksiyonlarda kullanılabilir.");
                };
                Some(PlaceInfo {
                    place: TPlace::Bu,
                    expr: self.mk(TK::Bu, Type::Named(o), e.span),
                    ty: Type::Named(o),
                    immut: None,
                    rooted_bu: true,
                })
            }
            ExprKind::Field(b, n, _) => {
                if let Some(PathRes::Module(mi)) = self.static_path(b) {
                    return Ok(self.mods[mi].globals.get(n).cloned().map(|v| PlaceInfo {
                        place: TPlace::Var(v.slot),
                        expr: self.mk(TK::Var(v.slot), v.ty.clone(), e.span),
                        ty: v.ty.clone(),
                        immut: if v.sabit { Some(format!("'{n}' sabit")) } else { None },
                        rooted_bu: false,
                    }));
                }
                let Some(pb) = self.place_of(b)? else { return Ok(None) };
                let Type::Named(tid) = pb.ty else { return Ok(None) };
                let Some(f) = self.fields_of(tid).iter().find(|f| &*f.name == n).cloned() else { return Ok(None) };
                let fid = self.field_id(&f.name);
                Some(PlaceInfo {
                    place: TPlace::Field(Box::new(pb.place), fid),
                    expr: self.mk(TK::Field(Box::new(pb.expr), fid), f.ty.clone(), e.span),
                    ty: f.ty,
                    immut: pb.immut.or(if f.sabit { Some(format!("'{n}' alanı sabit")) } else { None }),
                    rooted_bu: pb.rooted_bu,
                })
            }
            ExprKind::Index(b, i) => {
                let Some(pb) = self.place_of(b)? else { return Ok(None) };
                match pb.ty.clone() {
                    Type::List(elem, _) => {
                        if matches!(i.kind, ExprKind::Range { .. }) {
                            return Ok(None);
                        }
                        let it = self.index_value(i)?;
                        Some(PlaceInfo {
                            place: TPlace::ListIndex(Box::new(pb.place), Box::new(it.clone())),
                            expr: self.mk(TK::Index(Box::new(pb.expr), Box::new(it)), (*elem).clone(), e.span),
                            ty: *elem,
                            immut: pb.immut,
                            rooted_bu: pb.rooted_bu,
                        })
                    }
                    Type::Dict(k, v) => {
                        let kt = self.expr(i, Some(&k))?;
                        let kt = self.coerce(kt, &k, i.span)?;
                        Some(PlaceInfo {
                            place: TPlace::DictKey(Box::new(pb.place), Box::new(kt.clone())),
                            expr: self.mk(TK::Index(Box::new(pb.expr), Box::new(kt)), (*v).clone(), e.span),
                            ty: *v,
                            immut: pb.immut,
                            rooted_bu: pb.rooted_bu,
                        })
                    }
                    _ => None,
                }
            }
            _ => None,
        })
    }

    fn index_value(&mut self, i: &Expr) -> CResult<TExpr> {
        let t = self.expr(i, Some(&TAM64))?;
        if !t.ty.is_int() {
            return err(i.span, format!("Liste indeksi tam sayı olmalı, ama {} verildi.", self.type_name(&t.ty)));
        }
        if self.assignable(&t.ty, &TAM64) { self.coerce(t, &TAM64, i.span) } else { Ok(t) }
    }

    fn method_call(&mut self, base: &Expr, name: &str, nspan: Span, args: &[Arg], span: Span) -> CResult<TExpr> {
        // Alıcı ifade sadece bir kez denetlenir (zincirleme çağrılarda süre üstel büyümesin).
        let mut pi = self.place_of(base)?;
        let mut value: Option<TExpr> = None;
        let recv_ty = match &pi {
            Some(p) => p.ty.clone(),
            None => {
                let t = self.expr(base, None)?;
                let ty = t.ty.clone();
                value = Some(t);
                ty
            }
        };
        if let Type::Named(tid) = recv_ty {
            let user = match &self.types[tid].kind {
                TypeDefKind::Arayuz { sigs } => sigs.iter().find(|s| s.name == name).map(|s| (s.params.clone(), s.ret.clone())),
                _ => match self.types[tid].methods.get(name) {
                    Some(MethodRes::Impl(f)) => Some((self.funcs[*f].params.clone(), self.funcs[*f].ret.clone())),
                    Some(MethodRes::Ambiguous(ps)) => {
                        let opts: Vec<String> = ps.iter().map(|p| format!("{}.{name}", self.types[*p].name)).collect();
                        return err(
                            nspan,
                            format!("'{name}' belirsiz: {} mı? Türün içine ortak bir 'yeniden {name}()' yazın.", opts.join(" mı, ")),
                        );
                    }
                    None => None,
                },
            };
            if let Some((params, ret)) = user {
                let vals = self.bind_args(&params, args, span, name)?;
                let recv = match pi {
                    Some(p) => {
                        if p.rooted_bu {
                            if let Some(cur) = self.cx.func {
                                self.mut_edges.push((cur, tid, name.to_string()));
                            }
                        }
                        if let Some(reason) = p.immut {
                            self.pending_mut.push(PendingMut {
                                view: Some((tid, name.to_string())),
                                direct: None,
                                reason: format!("{reason}; '{name}()' nesneyi değiştiriyor."),
                                span,
                            });
                        }
                        Recv::Place(p.place)
                    }
                    None => Recv::Value(Box::new(value.take().expect("alıcı"))),
                };
                let mid = self.method_id(name);
                return Ok(self.mk(TK::Method { recv, view: tid, name: name.into(), mid, args: vals }, ret, span));
            }
        }
        let Some(bi) = self.builtin_method(&recv_ty, name) else {
            let tn = self.type_name(&recv_ty);
            if let Type::Union(ms) = &recv_ty {
                let rest = make_union(ms.iter().filter(|m| **m != Type::Hic).cloned().collect());
                if ms.contains(&Type::Hic) && (self.builtin_method(&rest, name).is_some() || matches!(rest, Type::Named(_))) {
                    return err(nspan, format!("Bu değer Hiç olabilir ({tn}). Kullanmadan önce eşleştir ile kontrol edin."));
                }
                return err(nspan, format!("{tn} tipindeki değerde '{name}' kullanılamaz. Önce eşleştir ile hangi tip olduğunu kontrol edin."));
            }
            if let Type::Named(tid) = recv_ty {
                if self.fields_of(tid).iter().any(|f| &*f.name == name) {
                    return err(nspan, format!("'{name}' bir alan, fonksiyon değil; parantez olmadan yazın."));
                }
                let names: Vec<String> = self.types[tid].methods.keys().cloned().collect();
                let hint = closest(name, names.iter()).map(|c| format!(" '{c}' mı demek istediniz?")).unwrap_or_default();
                return err(nspan, format!("'{tn}' içinde '{name}' diye bir fonksiyon yok.{hint}"));
            }
            let hint = self
                .builtin_method_names(&recv_ty)
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            let hint = closest(name, hint.iter()).map(|c| format!(" '{}' mı demek istediniz?", self.shown(&c))).unwrap_or_default();
            return err(nspan, format!("{tn} tipinde '{name}' diye bir fonksiyon yok.{hint}"));
        };
        let shown = self.shown(name);
        if bi.mutating {
            if let Some(p) = &pi {
                if let Some(reason) = &p.immut {
                    return err(span, format!("{reason}; '{shown}()' içeriği değiştirir."));
                }
                if p.rooted_bu {
                    if let Some(cur) = self.cx.func {
                        self.mutates[cur] = true;
                    }
                }
            }
            if bi.resizes {
                if let Type::List(_, Some(_)) = recv_ty {
                    return err(span, format!("Sabit boyutlu listeye ({}) eleman eklenemez ya da çıkarılamaz.", self.type_name(&recv_ty)));
                }
            }
        }
        if bi.bm == BM::Sirala {
            if let Type::List(elem, _) = &recv_ty {
                if !(elem.is_num() || **elem == Type::Str) {
                    return err(span, format!("Sadece sayı ve yazı listeleri sıralanabilir, ama {} verildi.", self.type_name(&recv_ty)));
                }
            }
        }
        let vals = self.bind_args(&bi.params, args, span, &shown)?;
        let recv = match pi.take() {
            Some(p) if bi.mutating => Recv::Place(p.place),
            Some(p) => Recv::Value(Box::new(p.expr)),
            None => Recv::Value(Box::new(value.take().expect("alıcı"))),
        };
        Ok(self.mk(TK::BMethod { recv, m: bi.bm, args: vals }, bi.ret, span))
    }

    fn builtin_method_names(&self, t: &Type) -> Vec<&'static str> {
        match t {
            Type::List(..) => vec!["ekle", "araya_ekle", "sil", "çıkar", "uzunluk", "içerir", "bul", "sırala", "ters", "temizle", "yazıya"],
            Type::Dict(..) => vec!["uzunluk", "içerir", "sil", "anahtarlar", "değerler", "temizle", "yazıya"],
            Type::Str => vec!["uzunluk", "büyük_harf", "küçük_harf", "karakter", "içerir", "böl", "kırp", "tama", "ondalığa", "yazıya"],
            Type::Float(_) => vec!["tama", "yazıya"],
            Type::Range => vec!["adım"],
            _ => vec!["yazıya"],
        }
    }

    pub(crate) fn builtin_method(&self, t: &Type, name: &str) -> Option<BInfo> {
        let p = |n: &str, t: Type| (n.to_string(), t);
        let mk = |bm, params, ret, mutating, resizes| Some(BInfo { bm, params, ret, mutating, resizes });
        if name == "yazıya" && *t != Type::Range {
            return mk(BM::Yaziya, vec![], Type::Str, false, false);
        }
        match t {
            Type::List(elem, _) => {
                let e = (**elem).clone();
                match name {
                    "ekle" => mk(BM::Ekle, vec![p("değer", e)], Type::Hic, true, true),
                    "araya_ekle" => mk(BM::ArayaEkle, vec![p("konum", TAM64), p("değer", e)], Type::Hic, true, true),
                    "sil" => mk(BM::Sil, vec![p("konum", TAM64)], Type::Hic, true, true),
                    "çıkar" => mk(BM::Cikar, vec![], e, true, true),
                    "uzunluk" => mk(BM::Uzunluk, vec![], TAM64, false, false),
                    "içerir" => mk(BM::Icerir, vec![p("değer", e)], Type::Bool, false, false),
                    "bul" => mk(BM::Bul, vec![p("değer", e)], Type::Union(vec![TAM64, Type::Hic]), false, false),
                    "sırala" => mk(BM::Sirala, vec![], Type::Hic, true, false),
                    "ters" => mk(BM::Ters, vec![], Type::Hic, true, false),
                    "temizle" => mk(BM::Temizle, vec![], Type::Hic, true, true),
                    _ => None,
                }
            }
            Type::Dict(k, v) => {
                let (k, v) = ((**k).clone(), (**v).clone());
                match name {
                    "uzunluk" => mk(BM::DUzunluk, vec![], TAM64, false, false),
                    "içerir" => mk(BM::DIcerir, vec![p("anahtar", k)], Type::Bool, false, false),
                    "sil" => mk(BM::DSil, vec![p("anahtar", k)], Type::Hic, true, true),
                    "anahtarlar" => mk(BM::Anahtarlar, vec![], Type::list(k), false, false),
                    "değerler" => mk(BM::Degerler, vec![], Type::list(v), false, false),
                    "temizle" => mk(BM::DTemizle, vec![], Type::Hic, true, true),
                    _ => None,
                }
            }
            Type::Str => match name {
                "uzunluk" => mk(BM::SUzunluk, vec![], TAM64, false, false),
                "büyük_harf" => mk(BM::BuyukHarf, vec![], Type::Str, false, false),
                "küçük_harf" => mk(BM::KucukHarf, vec![], Type::Str, false, false),
                "karakter" => mk(BM::Karakter, vec![p("konum", TAM64)], Type::Str, false, false),
                "içerir" => mk(BM::SIcerir, vec![p("parça", Type::Str)], Type::Bool, false, false),
                "böl" => mk(BM::Bol, vec![p("ayraç", Type::Str)], Type::list(Type::Str), false, false),
                "kırp" => mk(BM::Kirp, vec![], Type::Str, false, false),
                "tama" => mk(BM::Tama, vec![], TAM64, false, false),
                "ondalığa" => mk(BM::Ondaliga, vec![], Type::Float(64), false, false),
                _ => None,
            },
            Type::Float(_) => match name {
                "tama" => mk(BM::FTama, vec![], TAM64, false, false),
                _ => None,
            },
            Type::Range => match name {
                "adım" => mk(BM::Adim, vec![p("adım", TAM64)], Type::Range, false, false),
                _ => None,
            },
            _ => None,
        }
    }

    fn field_expr(&mut self, base: &Expr, name: &str, nspan: Span, span: Span) -> CResult<TExpr> {
        match self.static_path(base) {
            Some(PathRes::Module(mi)) => {
                if let Some(v) = self.mods[mi].globals.get(name).cloned() {
                    return Ok(self.mk(TK::Var(v.slot), v.ty, span));
                }
                if self.mods[mi].funcs.contains_key(name) {
                    return err(nspan, format!("'{name}' bir fonksiyon; çağırmak için {name}(...) yazın."));
                }
                return err(nspan, format!("'{}' dosyasında '{name}' diye bir değişken yok.", self.mods[mi].name));
            }
            Some(PathRes::Type(tid)) => {
                if matches!(self.types[tid].kind, TypeDefKind::Secenek { .. }) {
                    return self.variant(tid, name, nspan, None, span);
                }
                let tn = self.types[tid].name.clone();
                return err(span, format!("'{tn}.{name}' şeklinde kullanılamaz. Ebeveyn fonksiyonunu çağırmak için {tn}.{name}() yazın."));
            }
            None => {}
        }
        let b = self.expr(base, None)?;
        if let Type::Named(tid) = &b.ty {
            let tid = *tid;
            if let Some(f) = self.fields_of(tid).iter().find(|f| &*f.name == name).cloned() {
                let fid = self.field_id(&f.name);
                return Ok(self.mk(TK::Field(Box::new(b), fid), f.ty, span));
            }
            if self.types[tid].methods.contains_key(name) {
                return err(nspan, format!("'{name}' bir fonksiyon; çağırmak için {name}() yazın."));
            }
            let tn = self.types[tid].name.clone();
            return err(nspan, match &self.types[tid].kind {
                TypeDefKind::Secenek { .. } => format!("'{tn}' bir seçenek; içindeki değerlere eşleştir ile erişilir."),
                TypeDefKind::Arayuz { .. } => format!("'{tn}' bir arayüz; arayüzlerin alanı olmaz, sadece fonksiyonları vardır."),
                TypeDefKind::Tur { fields, .. } => {
                    let names: Vec<String> = fields.iter().map(|f| f.name.to_string()).collect();
                    let hint = closest(name, names.iter()).map(|c| format!(" '{c}' mı demek istediniz?")).unwrap_or_default();
                    format!("'{tn}' içinde '{name}' diye bir alan yok.{hint}")
                }
            });
        }
        if self.builtin_method(&b.ty, name).is_some() {
            let s = self.shown(name);
            return err(nspan, format!("'{s}' bir fonksiyon; çağırmak için {s}() yazın."));
        }
        err(nspan, format!("{} tipinde '{name}' diye bir alan yok.", self.type_name(&b.ty)))
    }

    fn index_expr(&mut self, base: &Expr, idx: &Expr, span: Span) -> CResult<TExpr> {
        let b = self.expr(base, None)?;
        match b.ty.clone() {
            Type::List(elem, _) => {
                if matches!(idx.kind, ExprKind::Range { .. }) {
                    let r = self.expr(idx, None)?;
                    return Ok(self.mk(TK::Slice(Box::new(b), Box::new(r)), Type::List(elem, None), span));
                }
                let i = self.index_value(idx)?;
                Ok(self.mk(TK::Index(Box::new(b), Box::new(i)), *elem, span))
            }
            Type::Dict(k, v) => {
                let kt = self.expr(idx, Some(&k))?;
                let kt = self.coerce(kt, &k, idx.span)?;
                Ok(self.mk(TK::Index(Box::new(b), Box::new(kt)), *v, span))
            }
            Type::Str => err(span, format!("Yazıda karakter erişimi için {} kullanın: yazı.karakter(0)", self.shown("karakter"))),
            t => err(span, format!("{} tipinde köşeli parantezle erişim yapılamaz.", self.type_name(&t))),
        }
    }

    fn olarak(&mut self, x: &Expr, te: &TypeExpr, span: Span) -> CResult<TExpr> {
        let target = self.resolve_type(te, self.cx.module)?;
        let v = self.expr(x, None)?;
        let ok = match (&v.ty, &target) {
            (a, b) if a.is_num() && b.is_num() => true,
            (Type::Union(ms), t) => ms.contains(t) || ms.iter().any(|m| self.assignable(m, t) || self.assignable(t, m)),
            (Type::Named(p), Type::Named(c)) => self.is_subtype(*c, *p) || self.is_subtype(*p, *c),
            (a, b) => self.assignable(a, b),
        };
        if !ok {
            return err(span, format!("{} değeri {} tipine dönüştürülemez.", self.type_name(&v.ty), self.type_name(&target)));
        }
        if self.assignable(&v.ty, &target) && !v.ty.is_num() {
            return self.coerce(v, &target, span);
        }
        Ok(self.mk(TK::Olarak(Box::new(v)), target, span))
    }

    // varsayılan değeri olan tipler (sabit boyutlu listelerin elemanları için)
    pub(crate) fn has_default(&self, t: &Type) -> bool {
        match t {
            Type::Int { .. } | Type::Float(_) | Type::Str | Type::Bool | Type::Dict(..) => true,
            Type::List(e, Some(_)) => self.has_default(e),
            Type::List(..) => true,
            Type::Union(ms) => ms.contains(&Type::Hic) || ms.iter().any(|m| self.has_default(m)),
            Type::Named(id) => match &self.types[*id].kind {
                TypeDefKind::Secenek { variants } => variants[0].fields.is_empty(),
                _ => false,
            },
            _ => false,
        }
    }
}
