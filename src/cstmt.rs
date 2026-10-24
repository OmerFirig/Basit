// Tip denetleyici: komutlar, eşleştir, akış analizi, dosya üst düzeyi ve fonksiyon gövdeleri.

use crate::ast::*;
use crate::check::*;
use crate::diag::{err, CResult, Span};
use crate::tir::*;
use crate::types::*;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Default)]
struct Coverage {
    variants: HashSet<(TypeId, u32)>,
    types: Vec<Type>,
    bools: [bool; 2],
    hic: bool,
    wild: bool,
}

impl Coverage {
    fn add(&mut self, p: &TPat) {
        match p {
            TPat::Variant { ty, idx, .. } => {
                self.variants.insert((*ty, *idx));
            }
            TPat::TypeTest { ty, .. } => self.types.push(ty.clone()),
            TPat::Bool(b) => self.bools[*b as usize] = true,
            TPat::Hic => self.hic = true,
            TPat::Wild => self.wild = true,
            _ => {}
        }
    }

    fn missing(&self, sty: &Type, c: &Checker) -> Option<Vec<String>> {
        if self.wild {
            return Some(vec![]);
        }
        let mut out = Vec::new();
        let enum_missing = |tid: TypeId, out: &mut Vec<String>| {
            if let TypeDefKind::Secenek { variants } = &c.types[tid].kind {
                for (i, v) in variants.iter().enumerate() {
                    if !self.variants.contains(&(tid, i as u32)) {
                        out.push(v.name.to_string());
                    }
                }
            }
        };
        match sty {
            Type::Named(tid) if matches!(c.types[*tid].kind, TypeDefKind::Secenek { .. }) => {
                if !self.types.contains(sty) {
                    enum_missing(*tid, &mut out);
                }
            }
            Type::Bool => {
                if !self.bools[1] {
                    out.push(c.kw_name(crate::lexer::Kw::Dogru));
                }
                if !self.bools[0] {
                    out.push(c.kw_name(crate::lexer::Kw::Yanlis));
                }
            }
            Type::Hic => {
                if !self.hic {
                    out.push("Hiç".into());
                }
            }
            Type::Union(ms) => {
                for m in ms {
                    match m {
                        Type::Hic => {
                            if !self.hic {
                                out.push(c.kw_name(crate::lexer::Kw::Hic));
                            }
                        }
                        Type::Named(tid) if matches!(c.types[*tid].kind, TypeDefKind::Secenek { .. }) => {
                            if !self.types.contains(m) {
                                enum_missing(*tid, &mut out);
                            }
                        }
                        _ => {
                            if !self.types.contains(m) {
                                out.push(c.type_name(m));
                            }
                        }
                    }
                }
            }
            _ => return None,
        }
        Some(out)
    }
}

impl Checker {
    pub(crate) fn kw_name(&self, k: crate::lexer::Kw) -> String {
        crate::lexer::kw_text(k, self.mode()).to_string()
    }

    pub(crate) fn push_scope(&mut self) {
        self.cx.scopes.push(HashMap::new());
    }
    pub(crate) fn pop_scope(&mut self) {
        self.cx.scopes.pop();
    }

    fn merge(&mut self, outs: Vec<Vec<FlowSt>>, pre_len: usize, pre: &[FlowSt]) {
        if outs.is_empty() {
            let mut f = pre.to_vec();
            f.truncate(pre_len);
            self.cx.flow = f;
            return;
        }
        let mut res = Vec::with_capacity(pre_len);
        for k in 0..pre_len {
            let first = outs[0][k];
            let st = if outs.iter().all(|o| o[k] == first) { first } else { FlowSt::Maybe };
            res.push(st);
        }
        self.cx.flow = res;
    }

    pub(crate) fn block(&mut self, stmts: &[Stmt]) -> CResult<(Vec<TS>, bool)> {
        self.push_scope();
        let r = self.stmts(stmts);
        self.pop_scope();
        r
    }

    fn stmts(&mut self, stmts: &[Stmt]) -> CResult<(Vec<TS>, bool)> {
        let mut out = Vec::new();
        let mut div = false;
        for s in stmts {
            let (ts, d) = self.stmt(s)?;
            out.extend(ts);
            div |= d;
        }
        Ok((out, div))
    }

    /// Döngü gövdesi. İkinci değer: gövdede bu döngüden çıkan bir `dur` var mı?
    fn loop_body(&mut self, body: &[Stmt]) -> CResult<(Vec<TS>, bool)> {
        self.cx.loop_depth += 1;
        let saved = std::mem::replace(&mut self.breaks_flag, false);
        let r = self.block(body);
        self.cx.loop_depth -= 1;
        let broke = std::mem::replace(&mut self.breaks_flag, saved);
        let (ts, _) = r?;
        Ok((ts, broke))
    }

    fn loop_flow<T>(&mut self, f: impl FnOnce(&mut Self) -> CResult<T>) -> CResult<T> {
        let pre = self.cx.flow.clone();
        let pre_len = pre.len();
        let r = f(self)?;
        let after = self.cx.flow.clone();
        self.merge(vec![pre.clone(), after], pre_len, &pre);
        Ok(r)
    }

    pub(crate) fn stmt(&mut self, s: &Stmt) -> CResult<(Vec<TS>, bool)> {
        let span = s.span;
        Ok(match &s.kind {
            StmtKind::Ata { sabit, explicit_ata, name, name_span, ty, value } => {
                (self.ata(*sabit, *explicit_ata, name, *name_span, ty.as_ref(), value.as_ref())?, false)
            }
            StmtKind::Assign { target, op, value } => (vec![self.assign(target, *op, value, span)?], false),
            StmtKind::Expr(Expr { kind: ExprKind::Lambda { .. }, span }) => {
                return err(*span, "Bu bir isimsiz fonksiyon (lambda), tek başına bir şey yapmaz. Bir liste üzerinde dönmek için: liste içindeki her x için yap { }");
            }
            StmtKind::Expr(e) => {
                let t = self.expr(e, None)?;
                let div = t.ty == Type::Never;
                (vec![TS::Expr(t)], div)
            }
            StmtKind::Eger { branches, else_ } => {
                let pre = self.cx.flow.clone();
                let pre_len = pre.len();
                let mut outs = Vec::new();
                let mut all_div = true;
                let mut tb = Vec::new();
                for (cond, body) in branches {
                    self.cx.flow = pre.clone();
                    let c = self.expr(cond, Some(&Type::Bool))?;
                    let c = self.bool_cond(c, cond.span, "'eğer'")?;
                    let (b, d) = self.block(body)?;
                    if !d {
                        outs.push(self.cx.flow.clone());
                    }
                    all_div &= d;
                    tb.push((c, b));
                }
                let te = match else_ {
                    Some(b) => {
                        self.cx.flow = pre.clone();
                        let (b, d) = self.block(b)?;
                        if !d {
                            outs.push(self.cx.flow.clone());
                        }
                        all_div &= d;
                        Some(b)
                    }
                    None => {
                        outs.push(pre.clone());
                        all_div = false;
                        None
                    }
                };
                self.merge(outs, pre_len, &pre);
                (vec![TS::If(tb, te)], all_div)
            }
            StmtKind::Kere { count, body } => {
                let c = self.expr(count, Some(&TAM64))?;
                if !c.ty.is_int() {
                    return err(count.span, format!("'kere' bir tam sayı bekler, ama {} verildi.", self.type_name(&c.ty)));
                }
                let c = if self.assignable(&c.ty, &TAM64) { self.coerce(c, &TAM64, count.span)? } else { c };
                let (b, _) = self.loop_flow(|me| me.loop_body(body))?;
                (vec![TS::Kere(c, b)], false)
            }
            StmtKind::Her { iter, pat, body } => {
                let it = self.expr(iter, None)?;
                let bind_tys: Vec<Type> = match &it.ty {
                    Type::List(elem, _) => {
                        if pat.len() != 1 {
                            return err(pat[1].1, "Liste üzerinde dönerken tek değişken yazılır: liste içindeki her x için yap");
                        }
                        vec![(**elem).clone()]
                    }
                    Type::Str => {
                        if pat.len() != 1 {
                            return err(pat[1].1, "Yazı üzerinde dönerken tek değişken yazılır.");
                        }
                        vec![Type::Str]
                    }
                    Type::Dict(k, v) => match pat.len() {
                        1 => vec![(**k).clone()],
                        2 => vec![(**k).clone(), (**v).clone()],
                        _ => return err(pat[2].1, "Sözlük üzerinde dönerken en fazla iki değişken yazılır: (anahtar, değer)"),
                    },
                    t => {
                        return err(
                            iter.span,
                            format!("{} üzerinde dönülemez. Liste, Sözlük, Yazı ya da aralık (0..10) olmalı.", self.type_name(t)),
                        );
                    }
                };
                let r = self.loop_flow(|me| {
                    me.push_scope();
                    let mut binds = Vec::new();
                    for ((n, sp), t) in pat.iter().zip(bind_tys) {
                        me.check_shadow_sabit(n, *sp)?;
                        binds.push(me.declare_var(n, t, false, true).slot);
                    }
                    let r = me.loop_body(body);
                    me.pop_scope();
                    let (b, _) = r?;
                    Ok((binds, b))
                })?;
                (vec![TS::ForEach { iter: it, binds: r.0, body: r.1 }], false)
            }
            StmtKind::Surece { cond, body } => {
                let c = self.expr(cond, Some(&Type::Bool))?;
                let c = self.bool_cond(c, cond.span, "'sürece'")?;
                let infinite = matches!(c.kind, TK::Bool(true));
                let (b, broke) = self.loop_flow(|me| me.loop_body(body))?;
                (vec![TS::While(c, b)], infinite && !broke)
            }
            StmtKind::CFor { init, cond, step, body } => {
                self.push_scope();
                let r = (|| -> CResult<TS> {
                    let (mut it, _) = self.stmt(init)?;
                    let c = self.expr(cond, Some(&Type::Bool))?;
                    let c = self.bool_cond(c, cond.span, "C tarzı döngünün koşulu")?;
                    let (b, st) = self.loop_flow(|me| {
                        let (b, _) = me.loop_body(body)?;
                        me.cx.loop_depth += 1;
                        let st = me.stmt(step);
                        me.cx.loop_depth -= 1;
                        Ok((b, st?.0))
                    })?;
                    let init = if it.len() == 1 { it.remove(0) } else { TS::If(vec![], Some(it)) };
                    let step = if st.len() == 1 { st.into_iter().next().unwrap() } else { TS::If(vec![], Some(st)) };
                    Ok(TS::CFor(Box::new(init), c, Box::new(step), b))
                })();
                self.pop_scope();
                (vec![r?], false)
            }
            StmtKind::Dur | StmtKind::Devam => {
                let is_dur = matches!(s.kind, StmtKind::Dur);
                if self.cx.loop_depth == 0 {
                    let k = if is_dur { "dur" } else { "devam" };
                    return err(span, format!("'{k}' sadece bir döngü içinde kullanılabilir."));
                }
                if is_dur {
                    self.breaks_flag = true;
                    (vec![TS::Break], true)
                } else {
                    (vec![TS::Continue], true)
                }
            }
            StmtKind::Dondur(e) if self.cx.ret_infer.is_some() => {
                // bloklu lambdanın dönüş tipi çıkarılıyor: döndürülen değerin tipi toplanır
                let t = match e {
                    Some(x) => Some(self.expr(x, None)?),
                    None => None,
                };
                let ty = t.as_ref().map(|t| t.ty.clone()).unwrap_or(Type::Hic);
                self.cx.ret_infer.as_mut().unwrap().push((ty, span));
                (vec![TS::Return(t)], true)
            }
            StmtKind::Dondur(e) => {
                let Some(ret) = self.cx.ret.clone() else {
                    return err(span, "'döndür' sadece bir fonksiyon içinde kullanılabilir.");
                };
                let fname = self.cx.fname.clone();
                match (e, ret == Type::Hic) {
                    (Some(x), true) => {
                        return err(
                            x.span,
                            format!("'{fname}' bir değer döndürmüyor ('->' yazılmamış). Değer döndürmek için fonksiyona '-> Tip' ekleyin."),
                        );
                    }
                    (None, false) => {
                        return err(span, format!("'{fname}' {} döndürmeli. 'döndür <değer>' yazın.", self.type_name(&ret)));
                    }
                    (Some(x), false) => {
                        let t = self.expr(x, Some(&ret))?;
                        let t = self.coerce(t, &ret, x.span)?;
                        (vec![TS::Return(Some(t))], true)
                    }
                    (None, true) => (vec![TS::Return(None)], true),
                }
            }
            StmtKind::Match(m) => {
                let (tm, _, div) = self.match_(m, false, None)?;
                (vec![TS::Match(tm)], div)
            }
            StmtKind::Fonksiyon(_) | StmtKind::Tur(_) | StmtKind::Arayuz(_) | StmtKind::Secenek(_) => {
                return err(span, "Fonksiyon, tür, arayüz ve seçenekler sadece dosyanın en üst düzeyinde tanımlanabilir.");
            }
            StmtKind::Dahil { .. } => {
                return err(span, "'dahil et' sadece dosyanın en üst düzeyinde kullanılabilir.");
            }
            StmtKind::Dis { .. } => {
                return err(span, "Dış kütüphaneler sadece dosyanın en üst düzeyinde tanımlanabilir.");
            }
        })
    }

    fn check_shadow_sabit(&self, name: &str, span: Span) -> CResult<()> {
        if let Some(v) = self.local_var(name) {
            if v.sabit {
                return err(span, format!("'{name}' sabit; aynı isimle yeni bir değişken tanımlanamaz."));
            }
        }
        Ok(())
    }

    fn ata(
        &mut self,
        sabit: bool,
        explicit_ata: bool,
        name: &str,
        name_span: Span,
        ty: Option<&TypeExpr>,
        value: Option<&Expr>,
    ) -> CResult<Vec<TS>> {
        let declared = match ty {
            Some(t) => {
                let r = self.resolve_type(t, self.cx.module)?;
                self.no_hic_value(&r, t.span)?;
                Some(r)
            }
            None => None,
        };
        let existing = self.local_var(name);
        if !explicit_ata && existing.is_none() {
            let tn = declared.as_ref().map(|t| self.type_name(t)).unwrap_or_default();
            return err(name_span, format!("'{name}' tanımlı değil. Yeni değişken için 'ata {name}: {tn} = ...' yazın."));
        }
        self.check_shadow_sabit(name, name_span)?;
        if existing.is_none() && self.owner_field(name).is_some() {
            // alanla aynı isimde yerel değişken: izin ver, alan 'bu.' ile erişilebilir kalır
        }
        let mut out = Vec::new();
        match value {
            Some(v) => {
                let t = self.expr(v, declared.as_ref())?;
                let t = match &declared {
                    Some(d) => self.coerce(t, d, v.span)?,
                    None => {
                        if t.ty == Type::Hic {
                            if matches!(t.kind, TK::Hic) {
                                return err(v.span, format!("Hiç tek başına atanamaz. Tipini yazın: ata {name}: Yazı veya Hiç = Hiç"));
                            }
                            return Err(self.mismatch(&t, &Type::Str, v.span));
                        }
                        if t.ty == Type::Never {
                            return err(v.span, "Bu ifade bir değer üretmiyor.");
                        }
                        t
                    }
                };
                let vty = t.ty.clone();
                let info = self.declare_var(name, vty, sabit, true);
                out.push(TS::Let(info.slot, t));
            }
            None => {
                let d = declared.unwrap();
                if let Type::List(..) = &d {
                    let mut cur = &d;
                    while let Type::List(e, size) = cur {
                        if size.is_some() && !self.has_default(e) {
                            return err(
                                name_span,
                                format!("{} için varsayılan değer yok; listeyi değerlerle oluşturun.", self.type_name(e)),
                            );
                        }
                        cur = e;
                    }
                }
                let defaultable = matches!(d, Type::List(..) | Type::Dict(..)) && !sabit;
                let info = self.declare_var(name, d.clone(), sabit, defaultable);
                if defaultable {
                    let span = name_span;
                    out.push(TS::Let(info.slot, self.mk(TK::Default, d, span)));
                } else {
                    out.push(TS::Declare(info.slot, d));
                }
            }
        }
        Ok(out)
    }

    fn assign(&mut self, target: &Expr, op: Option<BinOp>, value: &Expr, span: Span) -> CResult<TS> {
        // sabit ve henüz atanmamış değişkene ilk atama
        if let (ExprKind::Ident(n), None) = (&target.kind, op) {
            if let Some(v) = self.lookup_var(n, target.span)? {
                let st = self.flow_get(v.key);
                if st != FlowSt::Assigned || v.sabit {
                    if v.sabit {
                        if st != FlowSt::Unassigned {
                            return err(span, format!("'{n}' sabit ve zaten bir değer atandı; değeri değiştirilemez."));
                        }
                        if self.cx.loop_depth > v.loop_depth {
                            return err(span, format!("'{n}' sabit; döngü içinde değer atanamaz (birden fazla kez atanabilir)."));
                        }
                    }
                    let t = self.expr(value, Some(&v.ty))?;
                    let t = self.coerce_assign(t, &v.ty, n, value.span)?;
                    self.flow_set(v.key, FlowSt::Assigned);
                    return Ok(TS::Assign(TPlace::Var(v.slot), t, self.loc(span)));
                }
            }
        }
        let Some(pi) = self.place_of(target)? else {
            if let ExprKind::Ident(n) = &target.kind {
                if self.static_path(target).is_none() && !self.mods[self.cx.module].funcs.contains_key(n) {
                    let mut d = self.unknown_name(n, target.span);
                    if op.is_none() {
                        d.msg.push_str(&format!(" Yeni değişken için: ata {n} = ..."));
                    }
                    return Err(d);
                }
            }
            return err(target.span, "Bu ifadeye değer atanamaz.");
        };
        if let Some(reason) = &pi.immut {
            return err(span, format!("{reason}, değeri değiştirilemez."));
        }
        if pi.rooted_bu {
            if let Some(cur) = self.cx.func {
                self.mutates[cur] = true;
            }
        }
        let name = match &target.kind {
            ExprKind::Ident(n) => n.clone(),
            _ => String::new(),
        };
        match op {
            None => {
                let t = self.expr(value, Some(&pi.ty))?;
                let t = self.coerce_assign(t, &pi.ty, &name, value.span)?;
                Ok(TS::Assign(pi.place, t, self.loc(span)))
            }
            Some(op) => {
                if pi.ty == Type::Str && op == BinOp::Add {
                    let t = self.expr(value, Some(&Type::Str))?;
                    let t = self.coerce(t, &Type::Str, value.span)?;
                    return Ok(TS::Compound(pi.place, op, t, Type::Str, self.loc(span)));
                }
                if !pi.ty.is_num() {
                    return err(span, format!("'{}=' sadece sayılarla kullanılabilir, ama {} verildi.", op.symbol(), self.type_name(&pi.ty)));
                }
                if op.is_bit() && !pi.ty.is_int() {
                    return err(span, format!("'{}=' bir bit işlemidir, sadece tam sayılarla yapılır; ama {} verildi.", op.symbol(), self.type_name(&pi.ty)));
                }
                // Tam(N, -) ile pozitif sabit (x -= 1): 'x - 1' yazımındaki gibi x'in tipinde hesaplanır,
                // sonuç çalışma anında kontrol edilir.
                let neg_lit = match (&pi.ty, &value.kind) {
                    (Type::Int { bits: Some(b), sign: Sign::Neg }, ExprKind::Int(n)) if fits(n, Some(*b), Sign::Pos) => {
                        Some(self.mk(TK::Int(n.clone(), num_traits::ToPrimitive::to_i64(n)), pi.ty.clone(), value.span))
                    }
                    _ => None,
                };
                let t = match neg_lit {
                    Some(t) => t,
                    None => self.expr(value, Some(&pi.ty))?,
                };
                if pi.ty.is_int() && t.ty.is_float() {
                    return err(
                        value.span,
                        format!("{} tipindeki değişkene ondalık sayı eklenemez. Önce çevirin: (x olarak {})", self.type_name(&pi.ty), self.type_name(&pi.ty)),
                    );
                }
                let t = self.coerce(t, &pi.ty, value.span)?;
                Ok(TS::Compound(pi.place, op, t, pi.ty.clone(), self.loc(span)))
            }
        }
    }

    fn coerce_assign(&self, t: TExpr, to: &Type, name: &str, span: Span) -> CResult<TExpr> {
        if !name.is_empty() && !self.assignable(&t.ty, to) && !(t.ty.is_num() && to.is_num()) {
            let (f, tt) = (self.type_name(&t.ty), self.type_name(to));
            if !matches!(t.ty, Type::Hic | Type::Union(_)) {
                return err(span, format!("'{name}' {tt} tipinde, {f} atanamaz. Tipini değiştirmek için '{name}: {f} = ...' yazın."));
            }
        }
        self.coerce(t, to, span)
    }

    // ------------------------------------------------------------ eşleştir

    pub(crate) fn match_(&mut self, m: &Match, as_expr: bool, expected: Option<&Type>) -> CResult<(TMatch, Type, bool)> {
        let scrut = self.expr(&m.scrutinee, None)?;
        let sty = scrut.ty.clone();
        let pre = self.cx.flow.clone();
        let pre_len = pre.len();
        let mut arms = Vec::new();
        let mut outs = Vec::new();
        let mut arm_tys: Vec<(usize, Type)> = Vec::new();
        let mut all_div = true;
        let mut cov = Coverage::default();
        let mut has_default = false;
        for (ai, arm) in m.arms.iter().enumerate() {
            self.cx.flow = pre.clone();
            self.push_scope();
            let r = (|| -> CResult<(TArm, Type, bool)> {
                let pats = match &arm.pats {
                    None => {
                        if arm.guard.is_none() {
                            has_default = true;
                        }
                        None
                    }
                    Some(ps) => {
                        let mut v = Vec::new();
                        for p in ps {
                            v.push(self.pattern(p, &sty, ps.len() > 1)?);
                        }
                        if arm.guard.is_none() {
                            for tp in &v {
                                cov.add(tp);
                            }
                        }
                        Some(v)
                    }
                };
                let guard = match &arm.guard {
                    Some(g) => {
                        let t = self.expr(g, Some(&Type::Bool))?;
                        Some(self.bool_cond(t, g.span, "Ek koşul ('eğer')")?)
                    }
                    None => None,
                };
                let (body, ty, div) = match &arm.body {
                    ArmBody::Expr(e) => {
                        if as_expr {
                            let t = self.expr(e, expected)?;
                            let ty = t.ty.clone();
                            (TArmBody::Expr(t), ty, false)
                        } else {
                            let t = self.expr(e, None)?;
                            let d = t.ty == Type::Never;
                            (TArmBody::Block(vec![TS::Expr(t)]), Type::Hic, d)
                        }
                    }
                    ArmBody::Block(b) => {
                        if as_expr {
                            return err(arm.span, "Değer döndüren eşleştir'de dallar bir değer olmalı; '{ }' bloğu ya da atama kullanılamaz.");
                        }
                        let (ts, d) = self.block(b)?;
                        (TArmBody::Block(ts), Type::Hic, d)
                    }
                    ArmBody::Stmt(s) => {
                        let (ts, d) = self.stmt(s)?;
                        (TArmBody::Block(ts), Type::Never, d)
                    }
                };
                Ok((TArm { pats, guard, body }, ty, div))
            })();
            self.pop_scope();
            let (arm_t, ty, div) = r?;
            if !div {
                outs.push(self.cx.flow.clone());
            }
            all_div &= div;
            if as_expr && ty != Type::Never {
                arm_tys.push((ai, ty));
            }
            arms.push(arm_t);
        }
        let missing = cov.missing(&sty, self);
        let exhaustive = has_default || matches!(&missing, Some(v) if v.is_empty());
        if !exhaustive {
            let names = missing.clone().unwrap_or_default();
            if as_expr {
                let extra = if names.is_empty() { String::new() } else { format!(" Eksik: {}.", names.join(", ")) };
                return err(
                    m.span,
                    format!("Değer döndüren eşleştir'de tüm seçenekler yazılmalı ya da 'değilse' dalı eklenmeli.{extra}"),
                );
            }
            if !names.is_empty() {
                let quoted: Vec<String> = names.iter().map(|n| format!("'{n}'")).collect();
                let msg = if quoted.len() == 1 {
                    format!("{} seçeneği ele alınmamış.", quoted[0])
                } else {
                    format!("{} seçenekleri ele alınmamış.", quoted.join(", "))
                };
                self.warn(m.span, msg);
            }
            outs.push(pre.clone());
            all_div = false;
        }
        self.merge(outs, pre_len, &pre);

        let ty = if !as_expr {
            Type::Hic
        } else if let Some(e) = expected {
            e.clone()
        } else if arm_tys.is_empty() {
            Type::Never
        } else if arm_tys.iter().all(|(_, t)| t.is_num()) {
            let mut t = arm_tys[0].1.clone();
            for (_, x) in &arm_tys[1..] {
                t = match (&t, x) {
                    (Type::Int { bits: a, sign: sa }, Type::Int { bits: b, sign: sb }) => {
                        let (bits, sign) = int_join((*a, *sa), (*b, *sb));
                        Type::Int { bits, sign }
                    }
                    (Type::Float(a), Type::Float(b)) => Type::Float(*a.max(b)),
                    (Type::Float(a), _) | (_, Type::Float(a)) => Type::Float(*a),
                    _ => t.clone(),
                };
            }
            t
        } else {
            make_union(arm_tys.iter().map(|(_, t)| t.clone()).collect())
        };
        if as_expr {
            for (ai, arm) in arms.iter_mut().enumerate() {
                if let TArmBody::Expr(e) = &mut arm.body {
                    let span = m.arms[ai].span;
                    let taken = std::mem::replace(e, self.mk(TK::Hic, Type::Hic, span));
                    *e = self.coerce(taken, &ty, span)?;
                }
            }
        }
        let loc = self.loc(m.span);
        Ok((TMatch { scrut, arms, loc }, ty, exhaustive && all_div))
    }

    fn contains_kind(&self, sty: &Type, pred: impl Fn(&Type) -> bool) -> Option<Type> {
        sty.members().into_iter().find(|m| pred(m))
    }

    fn pattern(&mut self, p: &Pattern, sty: &Type, alt: bool) -> CResult<TPat> {
        let sname = self.type_name(sty);
        let need = |_: &Self, ok: bool, what: &str, span: Span| -> CResult<()> {
            if ok { Ok(()) } else { err(span, format!("Bu desen {what}, ama eşleştirilen değer {sname}.")) }
        };
        Ok(match p {
            Pattern::Int(n, s) => {
                need(self, self.contains_kind(sty, Type::is_int).is_some(), "bir tam sayı", *s)?;
                TPat::Int(n.clone())
            }
            Pattern::Float(f, s) => {
                need(self, self.contains_kind(sty, Type::is_float).is_some(), "bir ondalık sayı", *s)?;
                TPat::Float(*f)
            }
            Pattern::Str(x, s) => {
                need(self, self.contains_kind(sty, |t| *t == Type::Str).is_some(), "bir yazı", *s)?;
                TPat::Str(x.as_str().into())
            }
            Pattern::Bool(b, s) => {
                need(self, self.contains_kind(sty, |t| *t == Type::Bool).is_some(), "bir Mantık değeri", *s)?;
                TPat::Bool(*b)
            }
            Pattern::Hic(s) => {
                need(self, sty.contains_hic(), "Hiç", *s)?;
                TPat::Hic
            }
            Pattern::Wild => TPat::Wild,
            Pattern::Range(a, b, incl, s) => {
                need(self, self.contains_kind(sty, Type::is_int).is_some(), "bir sayı aralığı", *s)?;
                // Desen, aralık listesindeki değerlerden birine uyar; sıra önemli değil:
                // 5..=1 -> 1..=5, 5..1 (5, 4, 3, 2) -> 2..=5. Hep 'dahil' biçimine çevrilir.
                let one = num_bigint::BigInt::from(1);
                let (lo, hi) = match (a <= b, *incl) {
                    (true, true) => (a.clone(), b.clone()),
                    (true, false) => (a.clone(), b - &one),
                    (false, true) => (b.clone(), a.clone()),
                    (false, false) => (b + &one, a.clone()),
                };
                TPat::Range(lo, hi, true)
            }
            Pattern::Path { path, binds, span } => {
                // 1) seçenek deseni
                let enums: Vec<TypeId> = sty
                    .members()
                    .iter()
                    .filter_map(|m| match m {
                        Type::Named(t) if matches!(self.types[*t].kind, TypeDefKind::Secenek { .. }) => Some(*t),
                        _ => None,
                    })
                    .collect();
                let vname = path.last().unwrap();
                let mut found: Option<(TypeId, usize)> = None;
                if path.len() == 1 {
                    for &e in &enums {
                        if let TypeDefKind::Secenek { variants } = &self.types[e].kind {
                            if let Some(i) = variants.iter().position(|v| &*v.name == vname) {
                                found = Some((e, i));
                                break;
                            }
                        }
                    }
                } else {
                    let te = TypeExpr {
                        kind: TypeKind::Path { path: path[..path.len() - 1].to_vec(), generics: vec![], args: None, dims: vec![] },
                        span: *span,
                    };
                    if let Ok(Type::Named(e)) = self.resolve_type(&te, self.cx.module) {
                        if let TypeDefKind::Secenek { variants } = &self.types[e].kind {
                            match variants.iter().position(|v| &*v.name == vname) {
                                Some(i) => found = Some((e, i)),
                                None => {
                                    return err(*span, format!("'{}' içinde '{vname}' diye bir seçenek yok.", self.types[e].name));
                                }
                            }
                            if !enums.contains(&e) {
                                return err(*span, format!("Bu desen {} seçeneği, ama eşleştirilen değer {sname}.", self.types[e].name));
                            }
                        }
                    }
                }
                if let Some((e, i)) = found {
                    let TypeDefKind::Secenek { variants } = self.types[e].kind.clone() else { unreachable!() };
                    let v = &variants[i];
                    let mut slots = Vec::new();
                    if let Some(bs) = binds {
                        if bs.len() != v.fields.len() {
                            return err(
                                *span,
                                format!("'{}' {} değer taşır, ama {} isim yazıldı.", v.name, v.fields.len(), bs.len()),
                            );
                        }
                        if alt && bs.iter().any(Option::is_some) {
                            return err(*span, "'veya' ile birleştirilen desenlerde değişken tanımlanamaz.");
                        }
                        for (b, (_, fty)) in bs.iter().zip(&v.fields) {
                            slots.push(match b {
                                Some((n, s)) => {
                                    self.check_shadow_sabit(n, *s)?;
                                    Some(self.declare_var(n, fty.clone(), false, true).slot)
                                }
                                None => None,
                            });
                        }
                    }
                    return Ok(TPat::Variant { ty: e, idx: i as u32, binds: slots });
                }
                // 2) tip testi: Tam(n), Yazı(y), Kişi(k) ...
                let te = TypeExpr {
                    kind: TypeKind::Path { path: path.clone(), generics: vec![], args: None, dims: vec![] },
                    span: *span,
                };
                let target = match self.resolve_type(&te, self.cx.module) {
                    Ok(t) => t,
                    Err(_) => {
                        let mut names = Vec::new();
                        for &e in &enums {
                            if let TypeDefKind::Secenek { variants } = &self.types[e].kind {
                                names.extend(variants.iter().map(|v| v.name.to_string()));
                            }
                        }
                        let hint = closest(vname, names.iter()).map(|c| format!(" '{c}' mı demek istediniz?")).unwrap_or_default();
                        return err(*span, format!("'{}' diye bir seçenek ya da tip yok.{hint}", path.join(".")));
                    }
                };
                let member = sty.members().into_iter().find(|m| {
                    *m == target
                        || (m.is_int() && target.is_int())
                        || (m.is_float() && target.is_float())
                        || matches!((m, &target), (Type::List(..), Type::List(..)) | (Type::Dict(..), Type::Dict(..)))
                });
                let bind_ty = match (member, &target, sty) {
                    (Some(m), _, _) => m,
                    (None, Type::Named(c), Type::Named(p)) if self.is_subtype(*c, *p) => target.clone(),
                    _ => {
                        return err(*span, format!("'{}' deseni bu değerle eşleşemez; eşleştirilen değer {sname}.", self.type_name(&target)));
                    }
                };
                let mut bind = None;
                if let Some(bs) = binds {
                    if bs.len() > 1 {
                        return err(*span, "Tip deseninde en fazla bir isim yazılır: Tam(n)");
                    }
                    if alt && bs.iter().any(Option::is_some) {
                        return err(*span, "'veya' ile birleştirilen desenlerde değişken tanımlanamaz.");
                    }
                    if let Some(Some((n, s))) = bs.first() {
                        self.check_shadow_sabit(n, *s)?;
                        bind = Some(self.declare_var(n, bind_ty.clone(), false, true).slot);
                    }
                }
                TPat::TypeTest { ty: bind_ty, bind }
            }
        })
    }

    // ------------------------------------------------------------ üst düzey ve gövdeler

    fn global_snapshot(&self) -> Rc<HashMap<String, VarInfo>> {
        let mut m = HashMap::new();
        for (k, v) in &self.cx.scopes[0] {
            m.insert(k.clone(), VarInfo { key: ALWAYS, ..v.clone() });
        }
        Rc::new(m)
    }

    pub(crate) fn check_top(&mut self, mi: usize, stmts: &[Stmt]) -> CResult<Vec<TS>> {
        self.cx = Ctx { module: mi, scopes: vec![HashMap::new()], ..Default::default() };
        let mut out = Vec::new();
        for (si, s) in stmts.iter().enumerate() {
            match &s.kind {
                StmtKind::Dahil { alias, .. } => {
                    let dep = self.mods[mi].deps.iter().find(|d| d.0 == si).unwrap().1;
                    out.push(TS::RunModule(dep));
                    if alias.is_none() {
                        let dep_name = self.mods[dep].name.clone();
                        for (name, v) in self.mods[dep].globals.clone() {
                            if let Some(old) = self.cx.scopes[0].get(&name) {
                                if old.slot != v.slot {
                                    return err(s.span, self.conflict_msg(&name, mi, &dep_name));
                                }
                            }
                            self.cx.scopes[0].insert(name, VarInfo { key: ALWAYS, ..v });
                        }
                    }
                }
                StmtKind::Fonksiyon(d) => {
                    let fid = self.mods[mi].funcs[&d.name];
                    self.funcs[fid].snapshot = Some(self.global_snapshot());
                }
                StmtKind::Tur(TurDecl { name, .. }) | StmtKind::Secenek(SecenekDecl { name, .. }) => {
                    let tid = self.mods[mi].types[name];
                    let snap = self.global_snapshot();
                    for fid in self.type_methods[tid].clone() {
                        self.funcs[fid].snapshot = Some(snap.clone());
                    }
                }
                StmtKind::Arayuz(_) | StmtKind::Dis { .. } => {}
                _ => {
                    let (ts, _) = self.stmt(s)?;
                    out.extend(ts);
                }
            }
        }
        let mut g = HashMap::new();
        for (k, v) in &self.cx.scopes[0] {
            g.insert(k.clone(), VarInfo { key: ALWAYS, ..v.clone() });
        }
        self.mods[mi].globals = g;
        Ok(out)
    }

    pub(crate) fn check_body(&mut self, fid: FuncId) -> CResult<()> {
        if self.funcs[fid].body.is_some() {
            return Ok(()); // lambda: yazıldığı yerde denetlendi
        }
        if let Some(dis) = self.funcs[fid].dis.clone() {
            let f = &self.funcs[fid];
            let key = (f.span.file, f.span.start);
            self.funcs[fid].body = Some(TFunc {
                key,
                caps: None,
                dis: Some(dis),
                params: f.params.iter().map(|p| p.1.clone()).collect(),
                ret: f.ret.clone(),
                body: vec![],
                nslots: f.params.len() as u32,
                owner: None,
                loc: self.loc(f.span),
            });
            return Ok(());
        }
        let f = &mut self.funcs[fid];
        let decl = f.decl.take().unwrap();
        let snapshot = f.snapshot.clone().unwrap_or_default();
        let (module, owner, ret, params, sabits) = (f.module, f.owner, f.ret.clone(), f.params.clone(), f.param_sabit.clone());
        self.cx = Ctx {
            module,
            scopes: vec![HashMap::new()],
            snapshot: Some(snapshot),
            owner,
            ret: Some(ret.clone()),
            fname: decl.name.clone(),
            func: Some(fid),
            ..Default::default()
        };
        for (i, (n, t)) in params.iter().enumerate() {
            self.check_shadow_sabit(n, decl.params[i].span)?;
            self.declare_var(n, t.clone(), sabits[i], true);
        }
        let (body, div) = self.block(&decl.body)?;
        if ret != Type::Hic && !div {
            return err(
                decl.span,
                format!("'{}' {} döndürmeli, ama fonksiyonun sonuna bir değer döndürmeden ulaşılabiliyor.", decl.name, self.type_name(&ret)),
            );
        }
        let loc = self.loc(decl.span);
        self.funcs[fid].body = Some(TFunc {
            key: (decl.span.file, decl.span.start),
            caps: None,
            dis: None,
            params: params.into_iter().map(|p| p.1).collect(),
            ret,
            body,
            nslots: self.cx.nslots,
            owner,
            loc,
        });
        Ok(())
    }
}
