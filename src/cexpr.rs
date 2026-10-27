// Tip denetleyici: ifadeler.

use crate::ast::*;
use crate::check::*;
use crate::diag::{err, CResult, Diag, Span};
use crate::lexer::{Mode, BUILTIN_NAMES};
use crate::tir::*;
use crate::types::*;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::HashMap;

fn slot_no(s: Slot) -> u32 {
    match s {
        Slot::Local(i) | Slot::Global(i) => i,
    }
}

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

const BUILTIN_FUNCS: &[&str] = &["yazdır", "oku", "doğrula"];

const LAMBDA_BU: &str = "Lambda içinde nesne (bu) bir kopyadır";

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

    /// Sadece şu anki bağlamda (lambda içindeyse lambdanın kendisinde) arar.
    pub(crate) fn local_var(&self, name: &str) -> Option<VarInfo> {
        for s in self.cx.scopes.iter().rev() {
            if let Some(v) = s.get(name) {
                return Some(v.clone());
            }
        }
        // türün metodunda ya da ortak fonksiyonunda: türün (ve ebeveynlerinin) ortak değerleri
        for t in self.ortak_turleri() {
            let gizli = format!("{}.{name}", self.types[t].name);
            let v = self.cx.scopes.first().and_then(|s| s.get(&gizli)).or_else(|| self.cx.snapshot.as_ref().and_then(|s| s.get(&gizli)));
            if let Some(v) = v {
                return Some(v.clone());
            }
        }
        self.cx.snapshot.as_ref().and_then(|s| s.get(name).cloned())
    }

    /// Ortak değerlerine adıyla erişilebilen türler: içinde bulunulan tür ve ebeveynleri
    pub(crate) fn ortak_turleri(&self) -> Vec<TypeId> {
        let Some(t) = self.cx.ortak_tur.or(self.cx.owner) else { return vec![] };
        let mut v = vec![self.ortak_kok(t)];
        if let TypeDefKind::Tur { ancestors, .. } = &self.types[t].kind {
            v.extend(ancestors.iter().map(|a| self.ortak_kok(*a)));
        }
        v
    }

    /// `Tür.ad` ortak değeri (türün ya da ebeveynlerinin). Tanımlı ama henüz oluşmamışsa hata.
    pub(crate) fn ortak_var(&mut self, tid: TypeId, name: &str, span: Span) -> CResult<Option<VarInfo>> {
        let mut turler = vec![self.ortak_kok(tid)];
        if let TypeDefKind::Tur { ancestors, .. } = &self.types[tid].kind {
            turler.extend(ancestors.iter().map(|a| self.ortak_kok(*a)));
        }
        for t in turler {
            if !self.ortak_adlar.get(&t).is_some_and(|a| a.iter().any(|x| x == name)) {
                continue;
            }
            let gizli = format!("{}.{name}", self.types[t].name);
            if let Some(v) = self.lookup_var(&gizli, span)? {
                self.check_readable(&gizli, &v, span)?;
                return Ok(Some(v));
            }
            if let Some(v) = self.mods[self.type_decls[t].0].globals.get(&gizli) {
                return Ok(Some(v.clone()));
            }
            return err(span, format!("'{gizli}' burada henüz tanımlı değil; türün tanımından sonra kullanılabilir."));
        }
        Ok(None)
    }

    /// `ad` türün (ya da bir ebeveyninin) ortak değeri ya da fonksiyonu mu? Öyleyse ait olduğu tür.
    pub(crate) fn ortak_sahibi(&self, tid: TypeId, name: &str) -> Option<TypeId> {
        let mut turler = vec![self.ortak_kok(tid)];
        if let TypeDefKind::Tur { ancestors, .. } = &self.types[tid].kind {
            turler.extend(ancestors.iter().map(|a| self.ortak_kok(*a)));
        }
        turler.into_iter().find(|t| {
            self.ortak_adlar.get(t).is_some_and(|a| a.iter().any(|x| x == name)) || self.ortak_fonk.contains_key(&(*t, name.to_string()))
        })
    }

    /// `Tür.f()` ortak fonksiyonu (türün ya da ebeveynlerinin)
    pub(crate) fn ortak_fonk_bul(&self, tid: TypeId, name: &str) -> Option<FuncId> {
        let mut turler = vec![self.ortak_kok(tid)];
        if let TypeDefKind::Tur { ancestors, .. } = &self.types[tid].kind {
            turler.extend(ancestors.iter().map(|a| self.ortak_kok(*a)));
        }
        turler.into_iter().find_map(|t| self.ortak_fonk.get(&(t, name.to_string())).copied())
    }

    /// Bu ad bir değişken mi? (lambdanın dışındakiler dahil; yakalamadan)
    pub(crate) fn var_exists(&self, name: &str) -> bool {
        let in_ctx = |c: &Ctx| c.scopes.iter().any(|s| s.contains_key(name)) || c.snapshot.as_ref().is_some_and(|s| s.contains_key(name));
        in_ctx(&self.cx) || self.lambda_stack.iter().any(|f| in_ctx(&f.outer)) || self.local_var(name).is_some()
    }

    /// Değişkeni bulur. Lambda içindeyse ve değişken dışarıdaysa onu kopya olarak yakalar.
    pub(crate) fn lookup_var(&mut self, name: &str, span: Span) -> CResult<Option<VarInfo>> {
        if let Some(v) = self.local_var(name) {
            return Ok(Some(v));
        }
        let Some(mut frame) = self.lambda_stack.pop() else { return Ok(None) };
        // dışarıdaki bağlamda ara (o da bir lambdaysa, oradan da yakalanabilir)
        std::mem::swap(&mut self.cx, &mut frame.outer);
        let found = match self.lookup_var(name, span) {
            Ok(Some(v)) => self.check_readable(name, &v, span).map(|_| Some(v)),
            r => r,
        };
        std::mem::swap(&mut self.cx, &mut frame.outer);
        let found = match found {
            Ok(f) => f,
            Err(d) => {
                self.lambda_stack.push(frame);
                return Err(d);
            }
        };
        let r = found.map(|v| {
            let slot = self.new_slot();
            let info = VarInfo { slot, ty: v.ty.clone(), sabit: false, key: ALWAYS, loop_depth: 0, yakalanan: true, genel: None };
            self.cx.scopes[0].insert(name.to_string(), info.clone());
            frame.caps.push((v.slot, slot, v.ty));
            info
        });
        self.lambda_stack.push(frame);
        Ok(r)
    }

    /// Lambda içinde nesneye (`bu`) erişildi: lambda nesnenin kopyasını yakalar.
    pub(crate) fn use_bu(&mut self) {
        if let Some(f) = self.lambda_stack.last_mut() {
            f.bu = true;
        }
    }

    pub(crate) fn in_lambda(&self) -> bool {
        !self.lambda_stack.is_empty()
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
        let info = VarInfo { slot, ty, sabit, key, loop_depth: self.cx.loop_depth, yakalanan: false, genel: None };
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
        v.extend(m.types.keys().filter(|k| !k.contains('.')).cloned());
        v.extend(BUILTIN_FUNCS.iter().map(|s| s.to_string()));
        if let Some(o) = self.cx.owner {
            v.extend(self.fields_of(o).iter().map(|f| f.name.to_string()));
            v.extend(self.types[o].methods.keys().cloned());
        }
        v
    }

    pub(crate) fn unknown_name(&self, name: &str, span: Span) -> Diag {
        if self.cx.genel_lambda {
            return Diag::new(
                span,
                format!("'{name}' burada kullanılamaz: generic lambda dışarıdaki değişkenleri kullanamaz. Değeri bir parametre olarak verin."),
            );
        }
        if self.cx.varsayilan {
            return Diag::new(
                span,
                format!("'{name}' burada kullanılamaz: varsayılan değerde değişkenler kullanılamaz. Sayı, yazı, liste ya da fonksiyon çağrısı yazılabilir."),
            );
        }
        if let Some(t) = self.cx.ortak_tur {
            if self.fields_of(t).iter().any(|f| &*f.name == name) || self.types[t].methods.contains_key(name) {
                let tn = &self.types[t].name;
                return Diag::new(
                    span,
                    format!("'{name}' {tn} nesnelerine ait; ortak fonksiyonda nesne olmadığı için kullanılamaz. Nesneyi parametre olarak alın: ortak fonksiyon f(n: {tn})"),
                );
            }
        }
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
                format!("'{name}' bu fonksiyondan sonra tanımlanıyor. Fonksiyonlar sadece kendilerinden önce tanımlanan değişkenleri görebilir; '{name}' tanımını fonksiyonun üstüne taşıyın."),
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

    /// Üs sanılabilecek küçük bir sabit mi? (`x ^ 2` uyarısı için: 0x olmadan yazılmış 2..10)
    fn is_decimal_lit(&self, e: &Expr) -> bool {
        matches!(&e.kind, ExprKind::Int(n) if *n >= BigInt::from(2) && *n <= BigInt::from(10))
            && self
                .texts
                .get(e.span.file as usize)
                .and_then(|t| t.get(e.span.start as usize..e.span.end as usize))
                .is_some_and(|t| !t.trim_start_matches('-').starts_with("0x") && !t.trim_start_matches('-').starts_with("0X"))
    }

    /// Kullanıcının yazdığı ifade (kısa ve tek satırsa), örnek önerilerinde kullanılır.
    fn written(&self, span: Span) -> String {
        let text = self.texts.get(span.file as usize).and_then(|t| t.get(span.start as usize..span.end as usize));
        match text {
            Some(t) if !t.is_empty() && t.len() <= 40 && !t.contains('\n') => {
                if t.contains(' ') { format!("({t})") } else { t.to_string() }
            }
            _ => "x".to_string(),
        }
    }

    /// Tipin üyelerinden biri Hata (ya da alt türü) mü?
    pub(crate) fn hata_kabul(&self, t: &Type) -> bool {
        t.members().iter().any(|m| self.is_hata(m))
    }

    /// 'veya' tipinden Hata üyeleri çıkarılmış hali
    pub(crate) fn hatasiz(&self, t: &Type) -> Type {
        make_union(t.members().into_iter().filter(|m| !self.is_hata(m)).collect())
    }

    /// `T veya Hata` değeri T gibi kullanıldıysa yol gösterici mesaj
    pub(crate) fn hata_olabilir(&self, ty: &Type, span: Span) -> Option<Diag> {
        if !matches!(ty, Type::Union(_)) || !self.hata_kabul(ty) || self.hatasiz(ty) == Type::Never {
            return None;
        }
        let f = self.type_name(ty);
        let text = self.texts.get(span.file as usize).and_then(|t| t.get(span.start as usize..span.end as usize)).unwrap_or("").trim_end();
        Some(Diag::new(
            span,
            match cagri_adi(text) {
                Some(ad) => format!("'{ad}' hata verebilir ({f}). Bir 'dene {{ }} yakala' bloğu içinde kullanın ya da eşleştir ile kontrol edin."),
                None => format!("Bu değer bir hata olabilir ({f}). Kullanmadan önce eşleştir ile kontrol edin."),
            },
        ))
    }

    pub(crate) fn mismatch(&self, e: &TExpr, to: &Type, span: Span) -> Diag {
        let from = &e.ty;
        if let Some(d) = self.hata_olabilir(from, span) {
            if self.assignable(&self.hatasiz(from), to) {
                return d;
            }
        }
        let x = self.written(span);
        let (f, t) = (self.type_name(from), self.type_name(to));
        let msg = match (from, to) {
            (Type::Hic, _) if matches!(e.kind, TK::Call(..) | TK::Method { .. } | TK::Direct { .. } | TK::Builtin(..)) => {
                "Bu fonksiyon bir değer döndürmüyor, sonucu kullanılamaz.".to_string()
            }
            (Type::Int { .. }, Type::Int { .. }) => {
                format!("{f} değeri {t} tipine otomatik çevrilemez (veri kaybı olabilir). Bilerek çevirmek için: {x} olarak {t}")
            }
            (Type::Float(_), Type::Int { .. }) => {
                format!("Ondalık değer {t} tipine otomatik çevrilemez. Bilerek çevirmek için: {x} olarak {t}")
            }
            (Type::Float(_), Type::Float(_)) => {
                format!("{f} değeri {t} tipine otomatik çevrilemez (hassasiyet kaybı olabilir). Bilerek çevirmek için: {x} olarak {t}")
            }
            (Type::Int { .. }, Type::Float(_)) => {
                format!("{f} değeri {t} tipine otomatik çevrilemez ({t} küçük bir tip, değer sığmayabilir). Bilerek çevirmek için: {x} olarak {t}")
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
        // generic fonksiyon değeri (adı ya da generic lambda): tipi belli fonksiyon bekleniyorsa somutlaşır
        if let Some(Type::Func(ps, r)) = expected {
            if let Some(g) = self.genel_arguman(e)? {
                return self.genel_deger(g, ps, Some(r), span);
            }
        }
        if let ExprKind::Lambda { generics, .. } = &e.kind {
            if !generics.is_empty() {
                let s = self.kw_name(crate::lexer::Kw::Sabit);
                return err(span, format!("Generic lambda bir değişkene atanmalı ya da tipi belli bir fonksiyon bekleyen yere verilmeli: {s} iki_kat = her<T> x: T için x * 2"));
            }
        }
        if let ExprKind::Ident(n) = &e.kind {
            if let Some(g) = self.genel_var(n) {
                return Err(self.genel_deger_hatasi(g, span));
            }
        }
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
                        FPartE::Bicimli(x, b, bsp) => {
                            let t = self.expr(x, None)?;
                            if t.ty == Type::Hic && !matches!(t.kind, TK::Hic) {
                                return Err(self.mismatch(&t, &Type::Str, x.span));
                            }
                            out.push(TFPart::Expr(self.bicimle(t, b, *bsp)?));
                        }
                    }
                }
                self.mk(TK::FStr(out), Type::Str, span)
            }
            ExprKind::Bool(b) => self.mk(TK::Bool(*b), Type::Bool, span),
            ExprKind::Hic => self.mk(TK::Hic, Type::Hic, span),
            ExprKind::Bu => {
                let Some(o) = self.cx.owner else {
                    return err(span, if self.cx.ortak_tur.is_some() {
                        "Ortak fonksiyonlarda 'bu' yoktur: ortak fonksiyon tek bir nesneye değil, türe aittir."
                    } else {
                        "'bu' sadece bir tür ya da seçenek içindeki fonksiyonlarda kullanılabilir."
                    });
                };
                self.use_bu();
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
            ExprKind::BitNot(x) => {
                let t = self.expr(x, expected.filter(|t| t.is_int()))?;
                let Type::Int { bits, sign } = t.ty.clone() else {
                    return err(span, format!("'~' bir bit işlemidir, sadece tam sayılarla yapılır; ama {} verildi.", self.type_name(&t.ty)));
                };
                // ~x = -1 - x (işaretli; hiç taşmaz), sadece pozitif tiplerde (2^N - 1) - x
                let k: num_bigint::BigInt = match (bits, sign) {
                    (Some(b), Sign::Pos) => (num_bigint::BigInt::from(1) << b) - 1,
                    _ => num_bigint::BigInt::from(-1),
                };
                let ty = t.ty.clone();
                let c = self.mk(TK::Int(k.clone(), num_traits::ToPrimitive::to_i64(&k)), ty.clone(), span);
                self.mk(TK::Arith(BinOp::Sub, Box::new(c), Box::new(t)), ty, span)
            }
            ExprKind::Not(x) => {
                let t = self.expr(x, Some(&Type::Bool))?;
                let t = self.bool_cond(t, x.span, "'değil'")?;
                self.mk(TK::Not(Box::new(t)), Type::Bool, span)
            }
            ExprKind::Binary(op, l, r) => self.binary(*op, l, r, span, expected)?,
            ExprKind::Range { start, end, inclusive } => {
                let (Some(st), Some(en)) = (start, end) else {
                    let op = if *inclusive { "..=" } else { ".." };
                    return err(
                        span,
                        format!("Aralık listesinin başlangıcı ve bitişi yazılmalı: 0{op}10. Sonu açık yazım (2.., ..5) sadece dilimde kullanılır: liste[2..]"),
                    );
                };
                let s = self.range_bound(st)?;
                let e = self.range_bound(en)?;
                self.mk(TK::Range { start: Box::new(s), end: Box::new(e), inclusive: *inclusive, step: None }, Type::list(TAM64), span)
            }
            ExprKind::Call { callee, args } => {
                let t = self.call_expr(callee, args, span, expected)?;
                // `dene` içinde `T veya Hata` döndüren çağrı: Hata dönerse fırlatılır, sonuç T olur
                // (beklenen tip Hata'yı da kabul ediyorsa değer olduğu gibi kalır)
                if self.cx.dene_depth > 0
                    && matches!(t.ty, Type::Union(_))
                    && self.hata_kabul(&t.ty)
                    && !expected.is_some_and(|e| self.hata_kabul(e))
                {
                    let rest = self.hatasiz(&t.ty);
                    self.mk(TK::Unwrap(Box::new(t)), rest, span)
                } else {
                    t
                }
            }
            ExprKind::Field(base, name, nspan) => self.field_expr(base, name, *nspan, span)?,
            ExprKind::Index(base, idx) => self.index_expr(base, idx, span)?,
            ExprKind::Olarak(x, te) => self.olarak(x, te, span)?,
            ExprKind::Match(m) => {
                let (tm, ty, _) = self.match_(m, true, expected)?;
                self.mk(TK::Match(Box::new(tm)), ty, span)
            }
            ExprKind::Lambda { params, body, .. } => match expected {
                Some(Type::Func(ps, r)) => self.lambda(params, body, Some(ps), Some(r), span)?,
                _ => self.lambda(params, body, None, None, span)?,
            },
        })
    }

    // ------------------------------------------------------------ lambda

    /// `her x için ...`: parametre tipleri yazılmamışsa kullanıldığı yerden (`hint`) gelir.
    pub(crate) fn lambda(
        &mut self,
        params: &[LParam],
        body: &LambdaBody,
        hint: Option<&[Type]>,
        hint_ret: Option<&Type>,
        span: Span,
    ) -> CResult<TExpr> {
        if let Some(h) = hint {
            if h.len() != params.len() {
                return err(span, format!("Burada {} değer alan bir fonksiyon bekleniyordu, ama lambda {} parametreli.", h.len(), params.len()));
            }
        }
        let mut ptys = Vec::new();
        for (i, p) in params.iter().enumerate() {
            let t = match (&p.ty, hint) {
                (Some(te), _) => {
                    let t = self.resolve_type(te, self.cx.module)?;
                    self.no_hic_value(&t, te.span)?;
                    if let Some(h) = hint {
                        if h[i] != t {
                            return err(te.span, format!("'{}' burada {} olmalı, ama {} yazılmış.", p.name, self.type_name(&h[i]), self.type_name(&t)));
                        }
                    }
                    t
                }
                (None, Some(h)) => h[i].clone(),
                (None, None) => {
                    return err(p.span, format!("'{}' parametresinin tipi belli değil. Tipini yazın: her ({}: Tam(64)) için ...", p.name, p.name));
                }
            };
            ptys.push(t);
        }
        let ret = match (body, hint_ret) {
            (_, Some(r)) => Some(r.clone()),
            (LambdaBody::Expr(_), None) => None,
            (LambdaBody::Block(_), None) => {
                // dönüş tipi `döndür` edilen değerlerden çıkarılır; sonra o tiple yeniden denetlenir
                let geri = self.geri_nokta();
                let r = self.lambda_once(params, &ptys, body, None, true, span);
                self.geri_al(geri);
                let Type::Func(_, r) = r?.ty else { unreachable!() };
                Some(*r)
            }
        };
        self.lambda_once(params, &ptys, body, ret.as_ref(), false, span)
    }

    /// İki geçişli denetim için: lambdanın ilk denetiminde eklenenler geri alınır.
    pub(crate) fn geri_nokta(&self) -> GeriNokta {
        GeriNokta {
            funcs: self.funcs.len(),
            warnings: self.warnings.len(),
            mut_edges: self.mut_edges.len(),
            mut_direct: self.mut_direct.len(),
            pending_mut: self.pending_mut.len(),
            types: self.types.len(),
            genel_lambdalar: self.genel_lambdalar.len(),
        }
    }

    pub(crate) fn geri_al(&mut self, g: GeriNokta) {
        self.funcs.truncate(g.funcs);
        self.mutates.truncate(g.funcs);
        self.instances.retain(|_, f| *f < g.funcs);
        self.warnings.truncate(g.warnings);
        self.mut_edges.truncate(g.mut_edges);
        self.mut_direct.truncate(g.mut_direct);
        self.pending_mut.truncate(g.pending_mut);
        // ilk geçişte oluşan generic tür kopyaları
        self.types.truncate(g.types);
        self.type_decls.truncate(g.types);
        self.own_fields.truncate(g.types);
        self.type_methods.truncate(g.types);
        self.tur_inst.retain(|t, _| *t < g.types);
        self.tur_instances.retain(|_, t| *t < g.types);
        self.dispatch.retain(|k, _| k.0 < g.types && k.1 < g.types);
        self.alan_varsayilan.retain(|k, _| k.0 < g.types);
        self.genel_lambdalar.truncate(g.genel_lambdalar);
        for gl in &mut self.genel_lambdalar {
            gl.ornekler.retain(|_, f| *f < g.funcs);
        }
    }

    /// `infer`: dönüş tipini bulmak için (gövde blok, `döndür` değerleri toplanır); dönen ifadenin tipi
    /// çıkarılan dönüş tipidir.
    fn lambda_once(&mut self, params: &[LParam], ptys: &[Type], body: &LambdaBody, ret: Option<&Type>, infer: bool, span: Span) -> CResult<TExpr> {
        let fid = self.funcs.len();
        self.funcs.push(FuncInfo {
            name: "isimsiz fonksiyon".into(),
            module: self.cx.module,
            owner: self.cx.owner,
            params: params.iter().zip(ptys).map(|(p, t)| (p.name.clone(), t.clone())).collect(),
            param_sabit: vec![false; params.len()],
            ret: ret.cloned().unwrap_or(Type::Hic),
            varsayilan: vec![],
            decl: None,
            snapshot: None,
            yeniden: Yeniden::Yok,
            span,
            body: None,
            dis: None,
            generics: vec![],
            inst: None,
            ortak_of: None,
        });
        self.mutates.push(false);
        let inner = Ctx {
            module: self.cx.module,
            scopes: vec![HashMap::new()],
            owner: self.cx.owner,
            ret: if infer { None } else { Some(ret.cloned().unwrap_or(Type::Hic)) },
            ret_infer: if infer { Some(Vec::new()) } else { None },
            fname: "isimsiz fonksiyon".into(),
            func: Some(fid),
            ortak_tur: self.cx.ortak_tur,
            ..Default::default()
        };
        let outer = std::mem::replace(&mut self.cx, inner);
        self.lambda_stack.push(LambdaFrame { outer, caps: Vec::new(), bu: false });
        let r = self.lambda_body(params, ptys, body, ret, span);
        let frame = self.lambda_stack.pop().expect("lambda");
        let inner = std::mem::replace(&mut self.cx, frame.outer);
        let (tbody, rty) = r?;
        let rty = if infer {
            let rs = inner.ret_infer.clone().unwrap_or_default();
            self.infer_ret(&rs, span)?
        } else {
            rty
        };
        let caps: Vec<TExpr> = frame.caps.iter().map(|(o, _, t)| self.mk(TK::Var(*o), t.clone(), span)).collect();
        let bu = if frame.bu {
            self.use_bu();
            Some(Box::new(self.mk(TK::Bu, Type::Named(self.cx.owner.expect("tür")), span)))
        } else {
            None
        };
        let loc = self.loc(span);
        self.funcs[fid].ret = rty.clone();
        self.funcs[fid].body = Some(TFunc {
            key: self.func_key(span),
            dis: None,
            caps: Some(frame.caps.iter().map(|(_, i, t)| (slot_no(*i), t.clone())).collect()),
            params: ptys.to_vec(),
            ret: rty.clone(),
            body: tbody,
            nslots: inner.nslots,
            owner: if frame.bu { self.cx.owner } else { None },
            loc,
        });
        Ok(self.mk(TK::Lambda { func: fid, caps, bu }, Type::Func(ptys.to_vec(), Box::new(rty)), span))
    }

    fn lambda_body(&mut self, params: &[LParam], ptys: &[Type], body: &LambdaBody, ret: Option<&Type>, span: Span) -> CResult<(Vec<TS>, Type)> {
        for (p, t) in params.iter().zip(ptys) {
            self.declare_var(&p.name, t.clone(), false, true);
        }
        match body {
            LambdaBody::Expr(e) => {
                let t = self.expr(e, ret)?;
                let t = match ret {
                    Some(r) => self.coerce(t, r, e.span)?,
                    None => t,
                };
                let ty = t.ty.clone();
                if ty == Type::Hic || ty == Type::Never {
                    Ok((vec![TS::Expr(t), TS::Return(None)], Type::Hic))
                } else {
                    Ok((vec![TS::Return(Some(t))], ty))
                }
            }
            LambdaBody::Block(b) => {
                let (ts, div) = self.block(b)?;
                let r = ret.cloned().unwrap_or(Type::Hic);
                if self.cx.ret_infer.is_none() && r != Type::Hic && !div {
                    return err(span, format!("Bu lambda {} döndürmeli, ama sonuna bir değer döndürmeden ulaşılabiliyor.", self.type_name(&r)));
                }
                Ok((ts, r))
            }
        }
    }

    /// Bloklu lambdanın `döndür`lerinden dönüş tipi
    fn infer_ret(&self, rs: &[(Type, Span)], span: Span) -> CResult<Type> {
        let vals: Vec<&(Type, Span)> = rs.iter().filter(|(t, _)| *t != Type::Never).collect();
        if vals.is_empty() {
            return Ok(Type::Hic);
        }
        let has_hic = vals.iter().any(|(t, _)| *t == Type::Hic);
        if has_hic && vals.iter().any(|(t, _)| *t != Type::Hic) {
            return err(span, "Bu lambda bazı yollarda değer döndürüyor, bazılarında döndürmüyor.");
        }
        let ts: Vec<Type> = vals.iter().map(|(t, _)| t.clone()).collect();
        self.unify(&ts, span, "lambdanın döndürdüğü")
    }

    /// Fonksiyon değerini çağırma: `f(3)`
    pub(crate) fn call_value(&mut self, f: TExpr, args: &[Arg], fspan: Span, span: Span) -> CResult<TExpr> {
        let Type::Func(ps, r) = f.ty.clone() else {
            return err(fspan, format!("Bu bir {} değeri, fonksiyon değil; çağrılamaz.", self.type_name(&f.ty)));
        };
        if args.len() != ps.len() {
            return err(span, format!("Bu fonksiyon {} değer alır, ama {} değer verildi.", ps.len(), args.len()));
        }
        let mut vals = Vec::new();
        for (a, p) in args.iter().zip(&ps) {
            if let Some((n, s)) = &a.name {
                return err(*s, format!("Fonksiyon değerine adlı değer verilemez ('{n}:' yazmayın)."));
            }
            let t = self.expr(&a.value, Some(p))?;
            vals.push(self.coerce(t, p, a.value.span)?);
        }
        Ok(self.mk(TK::CallValue(Box::new(f), vals), *r, span))
    }

    /// Fonksiyon bekleyen argüman (dönüşüm, filtre...): lambda ise parametre tipleri buradan gelir.
    fn func_arg(&mut self, a: &Arg, params: &[Type], ret: Option<&Type>, what: &str) -> CResult<TExpr> {
        if let Some(g) = self.genel_arguman(&a.value)? {
            return self.genel_deger(g, params, ret, a.value.span);
        }
        if let ExprKind::Lambda { params: lp, body, .. } = &a.value.kind {
            return self.lambda(lp, body, Some(params), ret, a.value.span);
        }
        let t = self.expr(&a.value, None)?;
        let ok = matches!(&t.ty, Type::Func(ps, r) if ps.as_slice() == params && ret.is_none_or(|x| **r == *x));
        if !ok {
            let beklenen = match ret {
                Some(r) => self.type_name(&Type::Func(params.to_vec(), Box::new(r.clone()))),
                None => format!("Fonksiyon({}) -> ...", params.iter().map(|p| self.type_name(p)).collect::<Vec<_>>().join(", ")),
            };
            return err(a.value.span, format!("'{what}' bir fonksiyon bekler ({beklenen}), ama {} verildi. Örnek: her x için x * 2", self.type_name(&t.ty)));
        }
        Ok(t)
    }

    /// Liste metotları: fonksiyon alanlar ve sayı listeleri (dönüşüm, filtre, toplam...)
    fn list_method(&mut self, pi: &mut Option<PlaceInfo>, value: &mut Option<TExpr>, elem: &Type, name: &str, args: &[Arg], span: Span) -> CResult<Option<TExpr>> {
        let shown = self.shown(name);
        let tek = |_: &Self| -> CResult<&Arg> {
            match args {
                [a] => Ok(a),
                _ => err(span, format!("'{shown}' tek bir fonksiyon alır: liste.{shown}(her x için ...)")),
            }
        };
        let siralanabilir = |t: &Type| t.is_num() || *t == Type::Str;
        let (bm, f, ret) = match name {
            "dönüşüm" => {
                let a = tek(self)?;
                let f = self.func_arg(a, std::slice::from_ref(elem), None, &shown)?;
                let Type::Func(_, r) = &f.ty else { unreachable!() };
                if matches!(**r, Type::Hic | Type::Never) {
                    return err(a.value.span, format!("'{shown}' için fonksiyon bir değer döndürmeli."));
                }
                let rt = Type::List(r.clone(), None);
                (BM::Donusum, Some(f), rt)
            }
            "filtre" | "hepsi" | "herhangi" => {
                let a = tek(self)?;
                let f = self.func_arg(a, std::slice::from_ref(elem), Some(&Type::Bool), &shown)?;
                match name {
                    "filtre" => (BM::Filtre, Some(f), Type::List(Box::new(elem.clone()), None)),
                    "hepsi" => (BM::Hepsi, Some(f), Type::Bool),
                    _ => (BM::Herhangi, Some(f), Type::Bool),
                }
            }
            "sırala" | "sıralı" if !args.is_empty() => {
                let a = match args {
                    [a] if a.name.as_ref().is_none_or(|(n, _)| n == "göre") => a,
                    _ => return err(span, format!("Örnek: liste.{shown}(göre: her x için x.yaş)")),
                };
                let f = self.func_arg(a, std::slice::from_ref(elem), None, &shown)?;
                let Type::Func(_, k) = &f.ty else { unreachable!() };
                if !siralanabilir(k) {
                    return err(a.value.span, format!("Sıralama anahtarı sayı ya da yazı olmalı, ama {} verildi.", self.type_name(k)));
                }
                if name == "sırala" {
                    (BM::SiralaGore, Some(f), Type::Hic)
                } else {
                    (BM::SiraliGore, Some(f), Type::List(Box::new(elem.clone()), None))
                }
            }
            "toplam" | "en_büyük" | "en_küçük" => {
                if !args.is_empty() {
                    return err(span, format!("'{shown}' değer almaz: liste.{shown}()"));
                }
                if name == "toplam" && !elem.is_num() {
                    return err(span, format!("'{shown}' sadece sayı listelerinde kullanılır, ama liste {} elemanlı.", self.type_name(elem)));
                }
                if !siralanabilir(elem) {
                    return err(span, format!("'{shown}' sadece sayı ve yazı listelerinde kullanılır, ama liste {} elemanlı.", self.type_name(elem)));
                }
                let bm = match name {
                    "toplam" => BM::Toplam,
                    "en_büyük" => BM::EnBuyuk,
                    _ => BM::EnKucuk,
                };
                (bm, None, elem.clone())
            }
            _ => return Ok(None),
        };
        let recv = if bm == BM::SiralaGore {
            let Some(p) = pi.take() else {
                // geçici liste: sıralanır ve atılır
                let v = value.take().expect("alıcı");
                let vals = f.into_iter().collect();
                return Ok(Some(self.mk(TK::BMethod { recv: Recv::Value(Box::new(v)), m: bm, args: vals }, ret, span)));
            };
            if let Some(reason) = &p.immut {
                return err(span, format!("{reason}; '{shown}()' içeriği değiştirir."));
            }
            if p.rooted_bu {
                if let Some(cur) = self.cx.func {
                    self.mutates[cur] = true;
                }
            }
            Recv::Place(p.place)
        } else {
            match pi.take() {
                Some(p) => Recv::Value(Box::new(p.expr)),
                None => Recv::Value(Box::new(value.take().expect("alıcı"))),
            }
        };
        let vals = f.into_iter().collect();
        Ok(Some(self.mk(TK::BMethod { recv, m: bm, args: vals }, ret, span)))
    }

    /// Aralık ya da dilim sınırı: Tam(64)'e sığan tam sayı
    fn range_bound(&mut self, x: &Expr) -> CResult<TExpr> {
        let t = self.expr(x, Some(&TAM64))?;
        if !t.ty.is_int() {
            return err(x.span, format!("Aralık sınırları tam sayı olmalı, ama {} verildi.", self.type_name(&t.ty)));
        }
        if !self.assignable(&t.ty, &TAM64) {
            return err(x.span, "Aralık sınırları Tam(64)'e sığmalı.");
        }
        self.coerce(t, &TAM64, x.span)
    }

    pub(crate) fn bool_cond(&self, t: TExpr, span: Span, what: &str) -> CResult<TExpr> {
        if t.ty == Type::Bool || t.ty == Type::Never {
            return Ok(t);
        }
        let extra = if t.ty.is_int() && (what.contains("sürece") || what.contains("surece")) { " Bir işi N kez tekrarlamak için '5 kere yap' yazın." } else { "" };
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
        if let Some(v) = self.lookup_var(name, span)? {
            self.check_readable(name, &v, span)?;
            return Ok(self.mk(TK::Var(v.slot), v.ty, span));
        }
        if let Some(f) = self.owner_field(name) {
            self.use_bu();
            let bu = self.mk(TK::Bu, Type::Named(self.cx.owner.unwrap()), span);
            let fid = self.field_id(&f.name);
            return Ok(self.mk(TK::Field(Box::new(bu), fid), f.ty, span));
        }
        if let Some(&fid) = self.mods[self.cx.module].funcs.get(name) {
            if self.kalip_param_var(fid) {
                return err(span, format!("'{name}' her tiple çalışan bir fonksiyon parametresi alıyor; değer olarak verilemez, çağırın: {name}(...)"));
            }
            if !self.funcs[fid].generics.is_empty() {
                return Err(self.genel_deger_hatasi(GenelRef::Fonksiyon(fid), span));
            }
            return Ok(self.func_ref(fid, span));
        }
        let m = &self.mods[self.cx.module];
        if BUILTIN_FUNCS.contains(&name) {
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

    /// Beklenen tip 'veya' ise içindeki tek liste (ya da sözlük) üyesi beklenir: `Liste<Ondalık(32)> veya Yazı = [0.1]`
    fn union_member(expected: Option<&Type>, pred: fn(&Type) -> bool) -> Option<Type> {
        let Some(Type::Union(ms)) = expected else { return None };
        let mut it = ms.iter().filter(|m| pred(m));
        match (it.next(), it.next()) {
            (Some(m), None) => Some(m.clone()),
            _ => None,
        }
    }

    /// İsimli fonksiyon değer olarak: `uygula(kare, 3)`
    pub(crate) fn func_ref(&self, fid: FuncId, span: Span) -> TExpr {
        let f = &self.funcs[fid];
        let ty = Type::Func(f.params.iter().map(|p| p.1.clone()).collect(), Box::new(f.ret.clone()));
        self.mk(TK::FuncRef(fid), ty, span)
    }

    fn list_lit(&mut self, items: &[Expr], expected: Option<&Type>, span: Span) -> CResult<TExpr> {
        if let Some(m) = Self::union_member(expected, |t| matches!(t, Type::List(..))) {
            return self.list_lit(items, Some(&m), span);
        }
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
            let d = self.kw_name(crate::lexer::Kw::Degisken);
            return err(span, format!("Boş listenin tipi belli değil. Tipini yazın: {d} liste: Liste<Tam> = []"));
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

    pub(crate) fn unify(&self, ts: &[Type], span: Span, what: &str) -> CResult<Type> {
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
        // farklı türler: ortak ebeveyn ya da arayüz ([Kedi(..), Köpek(..)] -> Liste<Hayvan>)
        if let Type::Named(f) = first {
            let mut cands: Vec<TypeId> = Vec::new();
            if let TypeDefKind::Tur { ancestors, .. } = &self.types[f].kind {
                cands.extend(ancestors);
            }
            cands.extend(&self.types[f].interfaces);
            if let Some(c) = cands.into_iter().find(|&c| ts.iter().all(|x| matches!(x, Type::Named(t) if self.is_subtype(*t, c)))) {
                return Ok(Type::Named(c));
            }
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
        if let Some(m) = Self::union_member(expected, |t| matches!(t, Type::Dict(..))) {
            return self.dict_lit(items, Some(&m), span);
        }
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
            let d = self.kw_name(crate::lexer::Kw::Degisken);
            return err(span, format!("Boş sözlüğün tipi belli değil. Tipini yazın: {d} s: Sözlük<Yazı, Tam> = {{}}"));
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
                        if let Some(d) = self.hata_olabilir(&t.ty, e.span) {
                            return Err(d);
                        }
                        if t.ty == Type::Hic && !matches!(t.kind, TK::Hic) {
                            return Err(self.mismatch(t, &TAM64, e.span));
                        }
                        return err(
                            span,
                            format!("'{}' işlemi {} ve {} arasında yapılamaz.", op.symbol(), self.type_name(&lt.ty), self.type_name(&rt.ty)),
                        );
                    }
                }
                if op.is_bit() && !(lt.ty.is_int() && rt.ty.is_int()) {
                    let bad = if lt.ty.is_int() { &rt.ty } else { &lt.ty };
                    return err(span, format!("'{}' bir bit işlemidir, sadece tam sayılarla yapılır; ama {} verildi.", op.symbol(), self.type_name(bad)));
                }
                if op == BinOp::BitXor && self.is_decimal_lit(r) {
                    self.warn(span, "'^' özel veya (xor) işlemidir; üs için '**' yazın.");
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
                    if matches!(lt.ty, Type::Func(..)) || matches!(rt.ty, Type::Func(..)) {
                        return err(span, "Fonksiyonlar karşılaştırılamaz.");
                    }
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
                for (t, e) in [(&lt, l), (&rt, r)] {
                    if let Some(d) = self.hata_olabilir(&t.ty, e.span) {
                        return Err(d);
                    }
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
                if self.var_exists(n) || self.owner_field(n).is_some() {
                    return None;
                }
                let m = &self.mods[self.cx.module];
                if let Some(&mi) = m.namespaces.get(n) {
                    return Some(PathRes::Module(mi));
                }
                m.types.get(n).map(|&t| PathRes::Type(t))
            }
            ExprKind::Field(b, n, _) => match self.static_path(b)? {
                PathRes::Type(t) if t == self.hata_turleri[0] => {
                    self.mods[self.cx.module].types.get(&format!("Hata.{n}")).map(|&x| PathRes::Type(x))
                }
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

    /// Her argümanın gittiği parametrenin sırası (adsızlar sırayla, adlılar adıyla)
    pub(crate) fn arg_slots(&self, names: &[String], args: &[Arg], span: Span, what: &str) -> CResult<Vec<usize>> {
        let mut out: Vec<usize> = Vec::new();
        let mut pos = 0;
        let mut seen_named = false;
        for a in args {
            let idx = match &a.name {
                None => {
                    if seen_named {
                        return err(a.value.span, "Adı yazılmış değerlerden sonra adsız değer yazılamaz.");
                    }
                    if pos >= names.len() {
                        return err(span, format!("'{what}' {} değer alır, ama {} değer verildi.", names.len(), args.len()));
                    }
                    pos += 1;
                    pos - 1
                }
                Some((n, s)) => {
                    seen_named = true;
                    match names.iter().position(|p| p == n) {
                        Some(i) => {
                            if out.contains(&i) {
                                return err(*s, format!("'{n}' iki kez verildi."));
                            }
                            i
                        }
                        None => {
                            let hint = closest(n, names.iter()).map(|c| format!(" '{c}' mı demek istediniz?")).unwrap_or_default();
                            return err(*s, format!("'{what}' içinde '{n}' diye bir alan ya da parametre yok.{hint}"));
                        }
                    }
                }
            };
            out.push(idx);
        }
        Ok(out)
    }

    pub(crate) fn bind_args(&mut self, params: &[(String, Type)], args: &[Arg], span: Span, what: &str) -> CResult<Vec<TExpr>> {
        let names: Vec<String> = params.iter().map(|p| p.0.clone()).collect();
        let idx = self.arg_slots(&names, args, span, what)?;
        let mut slots: Vec<Option<TExpr>> = vec![None; params.len()];
        for (a, &i) in args.iter().zip(&idx) {
            let pty = params[i].1.clone();
            let t = self.expr(&a.value, Some(&pty))?;
            slots[i] = Some(self.coerce(t, &pty, a.value.span)?);
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

    fn call_expr(&mut self, callee: &Expr, args: &[Arg], span: Span, expected: Option<&Type>) -> CResult<TExpr> {
        match &callee.kind {
            ExprKind::Ident(name) => {
                if let Some(g) = self.genel_var(name) {
                    return self.genel_cagri(g, args, span);
                }
                if self.var_exists(name) || self.owner_field(name).is_some() {
                    // fonksiyon tipindeki değişken: f(3)
                    let f = self.expr(callee, None)?;
                    return self.call_value(f, args, callee.span, span);
                }
                if let Some(o) = self.cx.owner {
                    if self.types[o].methods.contains_key(name) {
                        let bu = Expr { kind: ExprKind::Bu, span: callee.span };
                        return self.method_call(&bu, name, callee.span, args, span);
                    }
                }
                if let Some(t) = self.cx.ortak_tur.or(self.cx.owner) {
                    if let Some(fid) = self.ortak_fonk_bul(t, name) {
                        return self.call_func(fid, args, span);
                    }
                }
                let m = self.cx.module;
                if let Some(&fid) = self.mods[m].funcs.get(name) {
                    return self.call_func(fid, args, span);
                }
                if let Some(&tid) = self.mods[m].types.get(name) {
                    return self.construct(tid, args, span, expected);
                }
                match name.as_str() {
                    "yazdır" => self.yazdir(args, span),
                    "doğrula" => err(
                        span,
                        format!("'{}' bir değer vermez, tek başına bir satır olarak yazılır: {}(koşul)", self.shown("doğrula"), self.shown("doğrula")),
                    ),
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
                // Hata.SıfıraBölme(mesaj: "...")
                if let Some(PathRes::Type(t)) = self.static_path(callee) {
                    return self.construct(t, args, span, expected);
                }
                match self.static_path(base) {
                    Some(PathRes::Type(t)) if t == self.hata_turleri[0] => return err(*nspan, hata_turu_yok(name)),
                    Some(PathRes::Module(mi)) => {
                        if let Some(g) = self.mods[mi].globals.get(name).and_then(|v| v.genel) {
                            return self.genel_cagri(g, args, span);
                        }
                        if let Some(&fid) = self.mods[mi].funcs.get(name) {
                            return self.call_func(fid, args, span);
                        }
                        if let Some(&tid) = self.mods[mi].types.get(name) {
                            return self.construct(tid, args, span, expected);
                        }
                        return err(*nspan, format!("'{}' dosyasında '{name}' diye bir fonksiyon ya da tür yok.", self.mods[mi].name));
                    }
                    Some(PathRes::Type(tid)) => {
                        if let Some(fid) = self.ortak_fonk_bul(tid, name) {
                            return self.call_func(fid, args, span);
                        }
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
            _ => {
                let f = self.expr(callee, None)?;
                self.call_value(f, args, callee.span, span)
            }
        }
    }

    fn call_func(&mut self, fid: FuncId, args: &[Arg], span: Span) -> CResult<TExpr> {
        if self.kalip_mi(fid) {
            return self.call_generic(fid, args, span);
        }
        let name = self.funcs[fid].name.clone();
        let vals = self.fonk_argumanlari(fid, args, span, &name)?;
        let ret = self.funcs[fid].ret.clone();
        Ok(self.mk(TK::Call(fid, vals), ret, span))
    }

    fn construct(&mut self, tid: TypeId, args: &[Arg], span: Span, expected: Option<&Type>) -> CResult<TExpr> {
        if self.tur_kalip.contains_key(&tid) {
            return self.construct_generic(tid, args, expected, span);
        }
        let tname = self.types[tid].name.clone();
        match &self.types[tid].kind {
            TypeDefKind::Tur { .. } => self.kur(tid, args, span, &tname),
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
        let vals = self.fonk_argumanlari(f, args, span, name)?;
        if let Some(cur) = self.cx.func {
            self.mut_direct.push((cur, f));
        }
        let ret = self.funcs[f].ret.clone();
        Ok(self.mk(TK::Direct { func: f, recv: Recv::Place(TPlace::Bu), args: vals }, ret, span))
    }

    pub(crate) fn place_of(&mut self, e: &Expr) -> CResult<Option<PlaceInfo>> {
        Ok(match &e.kind {
            ExprKind::Ident(n) => {
                if let Some(v) = self.lookup_var(n, e.span)? {
                    self.check_readable(n, &v, e.span)?;
                    Some(PlaceInfo {
                        place: TPlace::Var(v.slot),
                        expr: self.mk(TK::Var(v.slot), v.ty.clone(), e.span),
                        ty: v.ty,
                        immut: if v.yakalanan {
                            Some(format!("'{n}' lambdanın dışında tanımlı (lambda onun kopyasını kullanır)"))
                        } else if v.sabit {
                            Some(format!("'{n}' sabit"))
                        } else {
                            None
                        },
                        rooted_bu: false,
                    })
                } else if let Some(f) = self.owner_field(n) {
                    self.use_bu();
                    let fid = self.field_id(&f.name);
                    let bu = self.mk(TK::Bu, Type::Named(self.cx.owner.unwrap()), e.span);
                    Some(PlaceInfo {
                        place: TPlace::Field(Box::new(TPlace::Bu), fid),
                        expr: self.mk(TK::Field(Box::new(bu), fid), f.ty.clone(), e.span),
                        ty: f.ty,
                        immut: if self.in_lambda() {
                            Some(LAMBDA_BU.to_string())
                        } else if f.sabit {
                            Some(format!("'{n}' alanı sabit"))
                        } else {
                            None
                        },
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
                self.use_bu();
                Some(PlaceInfo {
                    place: TPlace::Bu,
                    expr: self.mk(TK::Bu, Type::Named(o), e.span),
                    ty: Type::Named(o),
                    immut: if self.in_lambda() { Some(LAMBDA_BU.to_string()) } else { None },
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
                if let Some(PathRes::Type(tid)) = self.static_path(b) {
                    let Some(v) = self.ortak_var(tid, n, e.span)? else { return Ok(None) };
                    let tn = self.types[tid].name.clone();
                    return Ok(Some(PlaceInfo {
                        place: TPlace::Var(v.slot),
                        expr: self.mk(TK::Var(v.slot), v.ty.clone(), e.span),
                        ty: v.ty.clone(),
                        immut: if v.yakalanan {
                            Some(format!("'{tn}.{n}' lambdanın dışında tanımlı (lambda onun kopyasını kullanır)"))
                        } else if v.sabit {
                            Some(format!("'{tn}.{n}' sabit"))
                        } else {
                            None
                        },
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
        // (a..b).adım(n): sonuç `(a..b)` listesinin adım(n)'i ile aynı; döngüde liste oluşturulmadan sayılsın
        // diye aralığın kendisine yazılır.
        if let (ExprKind::Range { start: Some(_), end: Some(_), .. }, "adım", [a]) = (&base.kind, name, args) {
            if a.name.as_ref().is_none_or(|(n, _)| n == "adım") {
                let r = self.expr(base, None)?;
                let n = self.expr(&a.value, Some(&TAM64))?;
                let n = self.coerce(n, &TAM64, a.value.span)?;
                let TK::Range { start, end, inclusive, step: None } = r.kind else { unreachable!() };
                return Ok(self.mk(TK::Range { start, end, inclusive, step: Some(Box::new(n)) }, r.ty, span));
            }
        }
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
                TypeDefKind::Arayuz { sigs } => sigs.iter().find(|s| s.name == name).map(|s| (s.params.clone(), s.ret.clone(), None)),
                _ => match self.types[tid].methods.get(name) {
                    Some(MethodRes::Impl(f)) => Some((self.funcs[*f].params.clone(), self.funcs[*f].ret.clone(), Some(*f))),
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
            if let Some((params, ret, f)) = user {
                let vals = match f {
                    Some(f) => self.fonk_argumanlari(f, args, span, name)?,
                    None => self.bind_args(&params, args, span, name)?,
                };
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
        if let Type::List(elem, _) = &recv_ty {
            let elem = (**elem).clone();
            if let Some(r) = self.list_method(&mut pi, &mut value, &elem, name, args, span)? {
                return Ok(r);
            }
        }
        if recv_ty == Type::Str && name == "tersi" {
            if !args.is_empty() {
                return err(span, "'tersi' değer almaz: yazı.tersi()");
            }
            // yazı.tersi() = yazı[-1..=0]
            let b = match pi.take() {
                Some(p) => p.expr,
                None => value.take().expect("alıcı"),
            };
            let k = |me: &Self, v: i64| Box::new(me.mk(TK::Int(v.into(), Some(v)), TAM64, span));
            let (s, e) = (k(self, -1), k(self, 0));
            return Ok(self.mk(TK::Slice { base: Box::new(b), start: Some(s), end: Some(e), inclusive: true }, Type::Str, span));
        }
        let Some(bi) = self.builtin_method(&recv_ty, name) else {
            if name == "birleşik" && matches!(recv_ty, Type::List(..)) {
                return err(
                    nspan,
                    format!(
                        "'{}' sadece yazı listelerinde çalışır ({} verildi). Önce yazıya çevirin: liste.{}(her x için x.{}()).{}(\", \")",
                        self.shown("birleşik"),
                        self.type_name(&recv_ty),
                        self.shown("dönüşüm"),
                        self.shown("yazıya"),
                        self.shown("birleşik")
                    ),
                );
            }
            if name == "ters" && matches!(recv_ty, Type::List(..)) {
                return err(
                    nspan,
                    format!("'ters' yerine '{}()' (listeyi değiştirir) ya da 'tersi()' (yeni liste verir) yazın.", self.shown("ters_çevir")),
                );
            }
            let tn = self.type_name(&recv_ty);
            if let Some(d) = self.hata_olabilir(&recv_ty, base.span) {
                return Err(d);
            }
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
                if let Some(t) = self.ortak_sahibi(tid, name) {
                    let ot = self.types[t].name.clone();
                    return err(nspan, format!("'{name}' {ot} türünün ortak fonksiyonu, nesne üzerinden çağrılmaz. Şöyle yazın: {ot}.{name}()"));
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
            if recv_ty == Type::Bool && self.builtin_method(&Type::Str, name).is_some() {
                let ad = self.shown(name);
                return err(
                    nspan,
                    format!(
                        "Bu değer bir Mantık (doğru/yanlış), yazı değil; '{ad}' yazılarda kullanılır. İki koşulu birlikte sormak için '{}' kullanın: s.{}(\"x\") {} s.{}(\"y\")",
                        self.shown("ve"),
                        self.shown("başlar"),
                        self.shown("ve"),
                        self.shown("biter")
                    ),
                );
            }
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
        if matches!(bi.bm, BM::Sirala | BM::Sirali) {
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

    /// `yazdır(a, b)`; `ayraç:` (değerlerin arası, varsayılan " ") ve `son:` (en sona, varsayılan satır sonu)
    /// verilirse değerler tek bir yazıda birleştirilip `Yaz` ile yazılır: yazdır(a, b, ayraç: ", ", son: "")
    fn yazdir(&mut self, args: &[Arg], span: Span) -> CResult<TExpr> {
        let ad = self.shown("yazdır");
        let mut vals = Vec::new();
        let (mut ayrac, mut son): (Option<TExpr>, Option<TExpr>) = (None, None);
        for a in args {
            if let Some((n, s)) = &a.name {
                let yer = match n.as_str() {
                    "ayraç" => &mut ayrac,
                    "son" => &mut son,
                    _ => {
                        return err(*s, format!("'{ad}' sadece '{}:' ve 'son:' adlı değerlerini alır: {ad}(a, b, {}: \", \", son: \"\")", self.shown("ayraç"), self.shown("ayraç")));
                    }
                };
                if yer.is_some() {
                    return err(*s, format!("'{n}' iki kez verildi."));
                }
                let t = self.expr(&a.value, Some(&Type::Str))?;
                *yer = Some(self.coerce(t, &Type::Str, a.value.span)?);
                continue;
            }
            if ayrac.is_some() || son.is_some() {
                return err(a.value.span, "Adı yazılmış değerlerden sonra adsız değer yazılamaz.");
            }
            let t = self.expr(&a.value, None)?;
            if t.ty == Type::Hic && !matches!(t.kind, TK::Hic) {
                return Err(self.mismatch(&t, &Type::Str, a.value.span));
            }
            vals.push(t);
        }
        if ayrac.is_none() && son.is_none() {
            return Ok(self.mk(TK::Builtin(Builtin::Yazdir, vals), Type::Hic, span));
        }
        let yazi = |me: &Self, s: &str| me.mk(TK::Str(std::rc::Rc::new(s.to_string())), Type::Str, span);
        let ayrac = ayrac.unwrap_or_else(|| yazi(self, " "));
        let son = son.unwrap_or_else(|| yazi(self, "\n"));
        // a + ayraç + b + ayraç + ... + son (yazdır ile aynı görünüm: yazıya())
        let mut sonuc: Option<TExpr> = None;
        for v in vals {
            let v = if v.ty == Type::Str { v } else { self.mk(TK::BMethod { recv: Recv::Value(Box::new(v)), m: BM::Yaziya, args: vec![] }, Type::Str, span) };
            sonuc = Some(match sonuc {
                None => v,
                Some(s) => {
                    let s = self.mk(TK::Concat(Box::new(s), Box::new(ayrac.clone())), Type::Str, span);
                    self.mk(TK::Concat(Box::new(s), Box::new(v)), Type::Str, span)
                }
            });
        }
        let tum = match sonuc {
            None => son,
            Some(s) => self.mk(TK::Concat(Box::new(s), Box::new(son)), Type::Str, span),
        };
        Ok(self.mk(TK::Builtin(Builtin::Yaz, vec![tum]), Type::Hic, span))
    }

    /// f-yazıdaki biçimli değer (`{x:.2}`) yazıya çevrilir: basamak -> gruplama -> hizalama
    fn bicimle(&mut self, t: TExpr, b: &Bicim, sp: Span) -> CResult<TExpr> {
        let sayi = t.ty.is_num();
        if !sayi {
            let ne = if b.basamak.is_some() {
                Some("Basamak ('.2')")
            } else if b.grup {
                Some("Gruplama (',')")
            } else if b.sifir {
                Some("Sıfırla doldurma ('0')")
            } else {
                None
            };
            if let Some(ne) = ne {
                return err(sp, format!("{ne} sadece sayılarda kullanılabilir, ama {} verildi.", self.type_name(&t.ty)));
            }
        }
        if b.hiza.is_some() && b.genislik.is_none() {
            return err(sp, "Hizalama için genişlik de yazılmalı: {ad:<10}");
        }
        let int = |me: &Self, v: i64| me.mk(TK::Int(v.into(), Some(v)), TAM64, sp);
        let bm = |me: &Self, recv: TExpr, m: BM, args: Vec<TExpr>| me.mk(TK::BMethod { recv: Recv::Value(Box::new(recv)), m, args }, Type::Str, sp);
        let mut s = if let Some(n) = b.basamak {
            let f = if t.ty == Type::Float(64) { t } else { self.mk(TK::Olarak(Box::new(t)), Type::Float(64), sp) };
            let n = int(self, n as i64);
            bm(self, f, BM::Hassas, vec![n])
        } else if t.ty == Type::Str {
            t
        } else {
            bm(self, t, BM::Yaziya, vec![])
        };
        if b.grup {
            s = bm(self, s, BM::Grupla, vec![]);
        }
        if let Some(w) = b.genislik {
            let kip = if b.sifir {
                3
            } else {
                match b.hiza {
                    Some('<') => 0,
                    Some('>') => 1,
                    Some('^') => 2,
                    _ => if sayi { 1 } else { 0 },
                }
            };
            let args = vec![int(self, w as i64), self.mk(TK::Str(std::rc::Rc::new(b.dolgu.to_string())), Type::Str, sp), int(self, kip)];
            s = bm(self, s, BM::Hizala, args);
        }
        Ok(s)
    }

    /// Hazır metotların adları ("'sya' mı demek istediniz?" önerileri için). Yeni metot eklenince adı
    /// buraya da yazılır (bkz. belgeler/GELISTIRICI.md).
    fn builtin_method_names(&self, t: &Type) -> Vec<&'static str> {
        match t {
            Type::List(..) => vec!["ekle", "araya_ekle", "sil", "çıkar", "uzunluk", "içerir", "bul", "sırala", "sıralı", "ters_çevir", "tersi", "temizle", "adım", "dönüşüm", "filtre", "hepsi", "herhangi", "toplam", "en_büyük", "en_küçük", "birleşik", "yazıya"],
            Type::Dict(..) => vec!["uzunluk", "içerir", "sil", "anahtarlar", "değerler", "temizle", "yazıya"],
            Type::Str => vec!["uzunluk", "büyük_harf", "küçük_harf", "karakter", "içerir", "böl", "kırp", "değiştir", "başlar", "biter", "bul", "say", "tekrar", "tama", "ondalığa", "tersi", "yazıya"],
            Type::Float(_) => vec!["tama", "yazıya"],
            _ => vec!["yazıya"],
        }
    }

    /// Hazır metotların tablosu: alıcının tipine göre ad -> (metot, parametreler, dönüş tipi, değiştirir mi,
    /// boyutu değiştirir mi). Fonksiyon alan liste metotları (dönüşüm, filtre...) `list_method` içindedir.
    pub(crate) fn builtin_method(&self, t: &Type, name: &str) -> Option<BInfo> {
        let p = |n: &str, t: Type| (n.to_string(), t);
        let mk = |bm, params, ret, mutating, resizes| Some(BInfo { bm, params, ret, mutating, resizes });
        if name == "yazıya" {
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
                    "sıralı" => mk(BM::Sirali, vec![], Type::List(Box::new(e.clone()), None), false, false),
                    "ters_çevir" => mk(BM::TersCevir, vec![], Type::Hic, true, false),
                    "tersi" => mk(BM::Tersi, vec![], Type::List(Box::new(e.clone()), None), false, false),
                    "temizle" => mk(BM::Temizle, vec![], Type::Hic, true, true),
                    "adım" => mk(BM::LAdim, vec![p("adım", TAM64)], Type::List(Box::new(e.clone()), None), false, false),
                    "birleşik" if e == Type::Str => mk(BM::Birlesik, vec![p("ayraç", Type::Str)], Type::Str, false, false),
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
                "değiştir" => mk(BM::Degistir, vec![p("eski", Type::Str), p("yeni", Type::Str)], Type::Str, false, false),
                "başlar" => mk(BM::Baslar, vec![p("parça", Type::Str)], Type::Bool, false, false),
                "biter" => mk(BM::Biter, vec![p("parça", Type::Str)], Type::Bool, false, false),
                "bul" => mk(BM::SBul, vec![p("parça", Type::Str)], Type::Union(vec![TAM64, Type::Hic]), false, false),
                "say" => mk(BM::Say, vec![p("parça", Type::Str)], TAM64, false, false),
                "tekrar" => mk(BM::Tekrar, vec![p("sayı", TAM64)], Type::Str, false, false),
                // yazıdan sayıya çevirme başarısız olabilir: değer ya da Hata.Dönüşüm
                "tama" => mk(BM::Tama, vec![], Type::Union(vec![TAM64, Type::Named(self.hata_turleri[hata_turu::DONUSUM as usize])]), false, false),
                "ondalığa" => mk(BM::Ondaliga, vec![], Type::Union(vec![Type::Float(64), Type::Named(self.hata_turleri[hata_turu::DONUSUM as usize])]), false, false),
                _ => None,
            },
            Type::Float(_) => match name {
                "tama" => mk(BM::FTama, vec![], TAM64, false, false),
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
                if let Some(&fid) = self.mods[mi].funcs.get(name) {
                    return Ok(self.func_ref(fid, span));
                }
                return err(nspan, format!("'{}' dosyasında '{name}' diye bir değişken yok.", self.mods[mi].name));
            }
            Some(PathRes::Type(tid)) => {
                if let Some(v) = self.ortak_var(tid, name, nspan)? {
                    return Ok(self.mk(TK::Var(v.slot), v.ty, span));
                }
                // ortak fonksiyon değer olarak: uygula(Hesap.iki, 6)
                if let Some(f) = self.ortak_fonk_bul(tid, name) {
                    return Ok(self.func_ref(f, span));
                }
                if matches!(self.types[tid].kind, TypeDefKind::Secenek { .. }) {
                    return self.variant(tid, name, nspan, None, span);
                }
                if tid == self.hata_turleri[0] {
                    if self.mods[self.cx.module].types.contains_key(&format!("Hata.{name}")) {
                        return err(span, format!("'Hata.{name}' bir tür; nesne oluşturmak için Hata.{name}(mesaj: \"...\") yazın."));
                    }
                    return err(nspan, hata_turu_yok(name));
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
            if let Some(t) = self.ortak_sahibi(tid, name) {
                let ot = self.types[t].name.clone();
                return err(nspan, format!("'{name}' {ot} türünün ortak değeri, tek bir nesnenin değil. Şöyle yazın: {ot}.{name}"));
            }
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
                if let ExprKind::Range { .. } = &idx.kind {
                    return self.slice(b, idx, Type::List(elem, None), span);
                }
                let i = self.index_value(idx)?;
                Ok(self.mk(TK::Index(Box::new(b), Box::new(i)), *elem, span))
            }
            Type::Dict(k, v) => {
                let kt = self.expr(idx, Some(&k))?;
                let kt = self.coerce(kt, &k, idx.span)?;
                Ok(self.mk(TK::Index(Box::new(b), Box::new(kt)), *v, span))
            }
            Type::Str if matches!(idx.kind, ExprKind::Range { .. }) => self.slice(b, idx, Type::Str, span),
            // yazı[i] = yazı.karakter(i): Türkçe harfler tek karakter sayılır
            Type::Str => {
                let i = self.index_value(idx)?;
                Ok(self.mk(TK::BMethod { recv: Recv::Value(Box::new(b)), m: BM::Karakter, args: vec![i] }, Type::Str, span))
            }
            t => err(span, format!("{} tipinde köşeli parantezle erişim yapılamaz.", self.type_name(&t))),
        }
    }

    /// `l[a..b]`, `yazı[a..b]`: köşeli parantez içindeki aralık liste değil, dilimdir.
    fn slice(&mut self, b: TExpr, idx: &Expr, ty: Type, span: Span) -> CResult<TExpr> {
        let ExprKind::Range { start, end, inclusive } = &idx.kind else { unreachable!() };
        let s = match start {
            Some(x) => Some(Box::new(self.range_bound(x)?)),
            None => None,
        };
        let e = match end {
            Some(x) => Some(Box::new(self.range_bound(x)?)),
            None => None,
        };
        Ok(self.mk(TK::Slice { base: Box::new(b), start: s, end: e, inclusive: *inclusive }, ty, span))
    }

    fn olarak(&mut self, x: &Expr, te: &TypeExpr, span: Span) -> CResult<TExpr> {
        let target = self.resolve_type(te, self.cx.module)?;
        // `[] olarak Liste<Tam>`: liste ve sözlük sabitleri hedef tipte oluşturulur
        let hint = if matches!(x.kind, ExprKind::List(_) | ExprKind::Dict(_)) { Some(&target) } else { None };
        let v = self.expr(x, hint)?;
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

/// Çağrı yazısından fonksiyonun adı: `böl(100, x)` -> `böl()`, `"42".tama()` -> `tama()`. Çağrı değilse None.
fn cagri_adi(text: &str) -> Option<String> {
    if !text.ends_with(')') {
        return None;
    }
    let b = text.as_bytes();
    let mut depth = 0i32;
    let mut open = None;
    for i in (0..b.len()).rev() {
        match b[i] {
            b')' => depth += 1,
            b'(' => {
                depth -= 1;
                if depth == 0 {
                    open = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let head = &text[..open?];
    let name = head.rsplit('.').next()?.trim();
    (!name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')).then(|| format!("{name}()"))
}
