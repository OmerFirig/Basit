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
            StmtKind::Assign { target, op, value } => match self.yaziya_ekleme(target, *op, value) {
                Some(ek) => (vec![self.assign(target, Some(BinOp::Add), &ek, span)?], false),
                None => (vec![self.assign(target, *op, value, span)?], false),
            },
            StmtKind::Expr(Expr { kind: ExprKind::Lambda { .. }, span }) => {
                return err(*span, "Bu bir isimsiz fonksiyon (lambda), tek başına bir şey yapmaz. Bir liste üzerinde dönmek için: liste içindeki her x için yap { }");
            }
            StmtKind::Expr(Expr { kind: ExprKind::Call { callee, args }, span })
                if matches!(&callee.kind, ExprKind::Ident(n) if n == "doğrula")
                    && !self.var_exists("doğrula") =>
            {
                (vec![self.dogrula(args, *span)?], false)
            }
            StmtKind::Expr(e) => {
                let t = self.expr(e, None)?;
                self.kullanilmayan_yazi(e, &t);
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
            StmtKind::Firlat(e) => {
                let t = self.expr(e, None)?;
                if t.ty == Type::Hic {
                    if matches!(t.kind, TK::Hic) {
                        return err(e.span, "Hiç fırlatılamaz. Bir değer ya da hata fırlatın: fırlat Hata(mesaj: \"...\")");
                    }
                    return Err(self.mismatch(&t, &Type::Str, e.span));
                }
                (vec![TS::Throw(t)], true)
            }
            StmtKind::Dene { body, catches } => self.dene(body, catches, span)?,
        })
    }

    /// Yazılar değişmez: `s.değiştir(...)` tek başına yazılırsa hiçbir şey olmaz, uyarı verilir.
    fn kullanilmayan_yazi(&mut self, e: &Expr, t: &TExpr) {
        let ExprKind::Call { callee, args } = &e.kind else { return };
        let ExprKind::Field(base, name, nspan) = &callee.kind else { return };
        let yeni_yazi = match &t.kind {
            TK::BMethod { m, .. } => matches!(m, BM::Degistir | BM::BuyukHarf | BM::KucukHarf | BM::Kirp | BM::Tekrar),
            TK::Slice { .. } => name == "tersi",
            _ => false,
        };
        if !yeni_yazi || t.ty != Type::Str {
            return;
        }
        let sp = base.span;
        let alici = self.texts.get(sp.file as usize).and_then(|x| x.get(sp.start as usize..sp.end as usize)).unwrap_or("yazı").to_string();
        let ad = self.shown(name);
        self.warn(
            *nspan,
            format!("'{ad}' yazıyı değiştirmez, yeni bir yazı verir; burada sonuç kullanılmıyor. Şöyle yazın: {alici} = {alici}.{ad}({})", if args.is_empty() { "" } else { "..." }),
        );
    }

    /// `doğrula(koşul)` ya da `doğrula(koşul, "mesaj")`: koşul yanlışsa `Hata` fırlatılır
    /// (`eğer değil koşul { fırlat Hata(mesaj: ...) }` ile aynı). Mesaj verilmezse koşulun yazılışı kullanılır.
    fn dogrula(&mut self, args: &[Arg], span: Span) -> CResult<TS> {
        let ad = self.shown("doğrula");
        if args.is_empty() || args.len() > 2 {
            return err(span, format!("'{ad}' bir koşul ve isteğe bağlı bir mesaj alır: {ad}(x > 0) ya da {ad}(x > 0, \"x pozitif olmalı\")"));
        }
        if let Some((n, s)) = args.iter().find_map(|a| a.name.as_ref()) {
            return err(*s, format!("'{ad}' adlı değer almaz ('{n}:' yazmayın)."));
        }
        let c = self.expr(&args[0].value, Some(&Type::Bool))?;
        let c = self.bool_cond(c, args[0].value.span, &format!("'{ad}'"))?;
        let on = self.mk(TK::Str(Rc::new("Doğrulama başarısız: ".to_string())), Type::Str, span);
        let mesaj = match args.get(1) {
            Some(a) => {
                let t = self.expr(&a.value, Some(&Type::Str))?;
                let t = self.coerce(t, &Type::Str, a.value.span)?;
                self.mk(TK::Concat(Box::new(on), Box::new(t)), Type::Str, span)
            }
            None => {
                let sp = args[0].value.span;
                let yazilan = self.texts.get(sp.file as usize).and_then(|t| t.get(sp.start as usize..sp.end as usize)).unwrap_or("koşul");
                let tam = format!("Doğrulama başarısız: {yazilan}");
                self.mk(TK::Str(Rc::new(tam)), Type::Str, span)
            }
        };
        let hata = self.hata_turleri[0];
        let nesne = self.mk(TK::New(hata, vec![mesaj]), Type::Named(hata), span);
        let degil = self.mk(TK::Not(Box::new(c)), Type::Bool, span);
        Ok(TS::If(vec![(degil, vec![TS::Throw(nesne)])], None))
    }

    /// `dene { } yakala ...`. Hata gövdenin herhangi bir yerinde olabileceği için yakala dalları,
    /// gövdede atanan değişkenleri "belki atanmış" görür.
    fn dene(&mut self, body: &[Stmt], catches: &[Catch], span: Span) -> CResult<(Vec<TS>, bool)> {
        let pre = self.cx.flow.clone();
        let pre_len = pre.len();
        self.cx.dene_depth += 1;
        let r = self.block(body);
        self.cx.dene_depth -= 1;
        let (tb, bdiv) = r?;
        let after_body = self.cx.flow.clone();
        self.merge(vec![pre.clone(), after_body.clone()], pre_len, &pre);
        let at_catch = self.cx.flow.clone();
        let mut outs = Vec::new();
        if !bdiv {
            outs.push(after_body);
        }
        if catches.is_empty() {
            let yer = match ilk_cagri(body) {
                Some((sp, ad)) => format!(" (satır {}: {ad} hata verebilir)", sp.line),
                None => String::new(),
            };
            let d = self.kw_name(crate::lexer::Kw::Dene);
            self.warn(span, format!("'{d}' bloğundaki hatalar yok sayılıyor{yer}. Hatayı ele almak için: {d} {{ }} {} h {{ }}", self.kw_name(crate::lexer::Kw::Yakala)));
            outs.push(at_catch.clone());
        }
        let yk = self.kw_name(crate::lexer::Kw::Yakala);
        let mut tcs: Vec<TCatch> = Vec::new();
        let mut onceki: Vec<(String, Option<Type>)> = Vec::new();
        for c in catches {
            self.cx.flow = at_catch.clone();
            let cty = match &c.ty {
                Some(te) => {
                    let t = self.resolve_type(te, self.cx.module)?;
                    self.no_hic_value(&t, te.span)?;
                    Some(t)
                }
                None => None,
            };
            // önceki bir dal bunu her zaman yakalıyorsa bu dal hiç çalışmaz
            for (on, ot) in &onceki {
                let kapsar = match (ot, &cty) {
                    (None, _) => true,
                    (Some(o), Some(t)) => self.assignable(t, o),
                    (Some(_), None) => false,
                };
                if kapsar {
                    let once = match ot {
                        Some(o) => format!("{yk} {on}: {}", self.type_name(o)),
                        None => format!("{yk} {on}"),
                    };
                    let bu = match &cty {
                        Some(t) => self.type_name(t),
                        None => format!("{yk} {}", c.name),
                    };
                    self.warn(c.span, format!("'{once}' önce geldiği için '{bu}' dalı hiç çalışmaz. Dalları özelden genele sıralayın."));
                    break;
                }
            }
            let bind_ty = match &cty {
                Some(t) => t.clone(),
                None => {
                    self.tipsiz_var = true;
                    self.tahmin.get(&(c.span.file, c.span.start)).cloned().unwrap_or(Type::Named(self.hata_turleri[0]))
                }
            };
            self.push_scope();
            let bind = if c.name == "_" {
                None
            } else {
                if let Err(e) = self.check_shadow_sabit(&c.name, c.name_span) {
                    self.pop_scope();
                    return Err(e);
                }
                Some(self.declare_var(&c.name, bind_ty.clone(), false, true).slot)
            };
            let geri = self.geri_nokta();
            let r = self.block(&c.body);
            self.pop_scope();
            let (b, d) = match r {
                Ok(x) => x,
                // keşif geçişi: tipsiz dalın değişken tipi henüz kesin değil, hata sonra yeniden denetlenir
                Err(_) if self.kesif && cty.is_none() => {
                    self.geri_al(geri);
                    self.yutulan = true;
                    (vec![], true)
                }
                Err(e) => return Err(e),
            };
            if !d {
                outs.push(self.cx.flow.clone());
            }
            onceki.push((c.name.clone(), cty.clone()));
            tcs.push(TCatch { ty: cty, bind, bind_ty, body: b, kaynak: vec![], span: c.span });
        }
        let div = outs.is_empty();
        self.merge(outs, pre_len, &pre);
        Ok((vec![TS::Dene { body: tb, catches: tcs }], div))
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
        // generic lambda: değeri olmayan, her çağrıda kendi tipiyle kopyalanan fonksiyon
        if let Some(Expr { kind: ExprKind::Lambda { generics, params, body }, span }) = value {
            if !generics.is_empty() {
                if let Some(t) = ty {
                    return err(t.span, "Generic lambdaya tip yazılmaz; tipi her çağrıda belli olur.");
                }
                self.check_shadow_sabit(name, name_span)?;
                self.genel_lambda_ata(name, generics, params, body, *span)?;
                return Ok(vec![]);
            }
        }
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

    /// Değişkenin tipi (ifade bir değişken adıysa)
    fn local_var_tipi(&self, e: &Expr) -> Option<Type> {
        let ExprKind::Ident(n) = &e.kind else { return None };
        self.local_var(n).map(|v| v.ty)
    }

    /// `s = s + a + b` (s yazı) -> `s += a + b`. Sonuç aynıdır, ama `+=` yazıyı yerinde büyütür; diğer
    /// yazımda her turda bütün yazı kopyalanır ve döngüde program karesel yavaşlar. Değiştirilecek kısım
    /// (`a + b`) döner; uymuyorsa None.
    fn yaziya_ekleme(&self, target: &Expr, op: Option<BinOp>, value: &Expr) -> Option<Expr> {
        let (ExprKind::Ident(n), None) = (&target.kind, op) else { return None };
        let v = self.local_var(n)?;
        if v.ty != Type::Str || v.sabit || v.yakalanan || self.flow_get(v.key) != FlowSt::Assigned {
            return None;
        }
        // en soldaki terim 's' olan bir + zinciri: ((s + a) + b)
        let mut terimler: Vec<&Expr> = Vec::new();
        let mut cur = value;
        while let ExprKind::Binary(BinOp::Add, l, r) = &cur.kind {
            terimler.push(r);
            cur = l;
        }
        if terimler.is_empty() || !matches!(&cur.kind, ExprKind::Ident(m) if m == n) {
            return None;
        }
        terimler.reverse();
        let mut ek = terimler[0].clone();
        for t in &terimler[1..] {
            let span = ek.span.to(t.span);
            ek = Expr { kind: ExprKind::Binary(BinOp::Add, Box::new(ek), Box::new((*t).clone())), span };
        }
        Some(ek)
    }

    fn assign(&mut self, target: &Expr, op: Option<BinOp>, value: &Expr, span: Span) -> CResult<TS> {
        if let ExprKind::Ident(n) = &target.kind {
            if self.genel_var(n).is_some() {
                return err(span, format!("'{n}' her tiple çalışan (generic) bir fonksiyon; değeri değiştirilemez. Yeni bir ad verin: ata başka = her<T> ..."));
            }
        }
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
            // yazının bir karakteri ya da parçası: yazılar değişmez
            if let ExprKind::Index(b, _) = &target.kind {
                if self.local_var_tipi(b) == Some(Type::Str) {
                    return err(
                        target.span,
                        "Yazılar değişmez; bir karakteri değiştirilemez. Yeni bir yazı oluşturun: s = \"x\" + s[1..] ya da s = s.değiştir(\"a\", \"b\")",
                    );
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
        if m.arms.is_empty() && !as_expr {
            let ad = self.shown("eşleştir");
            self.warn(m.span, format!("Bu '{ad}' hiç dal içermiyor, hiçbir şey yapmaz. Dal ekleyin: {ad} x {{ 1 -> yazdır(\"bir\") }}"));
        }
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
                if let Some(bs) = binds {
                    if bs.len() > 1 {
                        return err(*span, "Tip deseninde en fazla bir isim yazılır: Tam(n)");
                    }
                }
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
                let name = binds.as_ref().and_then(|bs| bs.first().cloned().flatten());
                self.type_test(target, name, *span, sty, alt)?
            }
            Pattern::Typed { name, ty, span } => {
                let target = self.resolve_type(ty, self.cx.module)?;
                self.type_test(target, name.clone(), *span, sty, alt)?
            }
        })
    }

    /// Tip deseni: `Tam(n) ->`, `n: Tam ->`, `Kişi(k) ->`
    fn type_test(&mut self, target: Type, name: Option<(String, Span)>, span: Span, sty: &Type, alt: bool) -> CResult<TPat> {
        let sname = self.type_name(sty);
        let alt_tur = |m: &Type, t: &Type| matches!((m, t), (Type::Named(a), Type::Named(b)) if self.is_subtype(*b, *a));
        let member = sty.members().into_iter().find(|m| {
            *m == target
                || (m.is_int() && target.is_int())
                || (m.is_float() && target.is_float())
                || matches!((m, &target), (Type::List(..), Type::List(..)) | (Type::Dict(..), Type::Dict(..)))
                || alt_tur(m, &target)
        });
        let bind_ty = match (member, &target, sty) {
            // üyenin alt türü (Hata -> Hata.Dönüşüm): değişken alt türün tipinde
            (Some(m), Type::Named(_), _) if alt_tur(&m, &target) => target.clone(),
            (Some(m), _, _) => m,
            (None, Type::Named(c), Type::Named(p)) if self.is_subtype(*c, *p) => target.clone(),
            _ => {
                return err(span, format!("'{}' deseni bu değerle eşleşemez; eşleştirilen değer {sname}.", self.type_name(&target)));
            }
        };
        let mut bind = None;
        if let Some((n, s)) = name {
            if alt {
                return err(span, "'veya' ile birleştirilen desenlerde değişken tanımlanamaz.");
            }
            self.check_shadow_sabit(&n, s)?;
            bind = Some(self.declare_var(&n, bind_ty.clone(), false, true).slot);
        }
        Ok(TPat::TypeTest { ty: bind_ty, bind })
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
                    // ortak değerler tanımın yerinde, `Tür.ad` adlı dosya değişkeni olarak oluşur
                    if let StmtKind::Tur(d) = &s.kind {
                        self.yasak_tip = d.generics.iter().map(|g| g.0.clone()).collect();
                        for o in &d.ortak {
                            let mut o = o.clone();
                            if let StmtKind::Ata { name: n, .. } = &mut o.kind {
                                *n = format!("{name}.{n}");
                            }
                            let (ts, _) = self.stmt(&o)?;
                            out.extend(ts);
                        }
                        self.yasak_tip.clear();
                    }
                    let snap = self.global_snapshot();
                    for (&(t, _), &fid) in &self.ortak_fonk {
                        if t == tid {
                            self.funcs[fid].snapshot = Some(snap.clone());
                        }
                    }
                    if self.tur_kalip.contains_key(&tid) {
                        self.tur_snapshot.insert(tid, snap.clone());
                    }
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
        if self.kalip_mi(fid) {
            // generic kalıp: gövdesi her kopyada ayrıca denetlenir; kendisi hiç çağrılmaz
            let f = &self.funcs[fid];
            let (key, loc) = ((f.span.file, f.span.start), self.loc(f.span));
            self.funcs[fid].body = Some(TFunc { key, caps: None, dis: None, params: vec![], ret: Type::Hic, body: vec![], nslots: 0, owner: None, loc });
            return Ok(());
        }
        if let Some(inst) = self.funcs[fid].inst.clone() {
            self.funcs[fid].snapshot = match inst.kalip {
                Kalip::Fonksiyon(t) => self.funcs[t].snapshot.clone(),
                Kalip::Tur(t) => self.tur_snapshot.get(&t).cloned(),
            };
            let eski = std::mem::replace(&mut self.tsubst, inst.binds.iter().cloned().collect());
            let r = self.check_body_inner(fid);
            self.tsubst = eski;
            return r.map_err(|d| {
                let (name, nerede) = match inst.kalip {
                    Kalip::Fonksiyon(t) => (self.funcs[t].name.clone(), "çağrı"),
                    Kalip::Tur(t) => (format!("{}.{}", self.types[t].name, self.funcs[fid].name), "ilk kullanım"),
                };
                let ekler = self.genel_bag_yazilari(&inst.genel_baglar);
                self.kopya_hatasi(d, &name, nerede, inst.call, &inst.binds, &ekler)
            });
        }
        let r = self.check_body_inner(fid);
        self.yasak_tip.clear();
        r
    }

    /// Fonksiyonun kaynaktaki yeri; REPL fonksiyon değerlerini hücreden hücreye bununla eşler. Generic
    /// kopyalarda (ve içlerindeki lambdalarda) tipler de katılır, yoksa kopyalar karışırdı.
    pub(crate) fn func_key(&self, span: Span) -> (u32, u32) {
        if self.tsubst.is_empty() {
            return (span.file, span.start);
        }
        use std::hash::{Hash, Hasher};
        let mut binds: Vec<(&String, &Type)> = self.tsubst.iter().collect();
        binds.sort_by(|a, b| a.0.cmp(b.0));
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (span.start, binds).hash(&mut h);
        (span.file, 0x8000_0000 | (h.finish() as u32 & 0x7fff_ffff))
    }

    fn check_body_inner(&mut self, fid: FuncId) -> CResult<()> {
        let f = &mut self.funcs[fid];
        let decl = f.decl.take().unwrap();
        let snapshot = f.snapshot.clone().unwrap_or_default();
        let (module, owner, ret, params, sabits) = (f.module, f.owner, f.ret.clone(), f.params.clone(), f.param_sabit.clone());
        let ortak_tur = f.ortak_of;
        self.yasak_tip = self.ortak_yasak(ortak_tur);
        self.cx = Ctx {
            module,
            scopes: vec![HashMap::new()],
            snapshot: Some(snapshot),
            owner,
            ret: Some(ret.clone()),
            fname: decl.name.clone(),
            func: Some(fid),
            ortak_tur,
            ..Default::default()
        };
        for (i, (n, t)) in params.iter().enumerate() {
            self.check_shadow_sabit(n, decl.params[i].span)?;
            self.declare_var(n, t.clone(), sabits[i], true);
        }
        // Fonksiyon<T> parametrelerine verilen generic fonksiyonlar: gövdede derleme anındaki değişkenler
        if let Some(inst) = self.funcs[fid].inst.clone() {
            for (n, r) in inst.genel_baglar {
                self.declare_var(&n, Type::Func(vec![], Box::new(Type::Hic)), true, true);
                if let Some(v) = self.cx.scopes.last_mut().and_then(|s| s.get_mut(&n)) {
                    v.genel = Some(r);
                }
            }
        }
        let (body, div) = self.block(&decl.body)?;
        if ret != Type::Hic && !div {
            return err(
                decl.span,
                format!("'{}' {} döndürmeli, ama fonksiyonun sonuna bir değer döndürmeden ulaşılabiliyor.", decl.name, self.type_name(&ret)),
            );
        }
        let loc = self.loc(decl.span);
        let key = self.func_key(decl.span);
        self.funcs[fid].body = Some(TFunc {
            key,
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

/// Bloktaki ilk fonksiyon çağrısı (uyarı mesajı için): yeri ve adı (`böl()`)
fn ilk_cagri(stmts: &[Stmt]) -> Option<(Span, String)> {
    fn e(x: &Expr) -> Option<(Span, String)> {
        match &x.kind {
            ExprKind::Call { callee, args } => {
                for a in args {
                    if let Some(r) = e(&a.value) {
                        return Some(r);
                    }
                }
                if let ExprKind::Field(b, _, _) = &callee.kind {
                    if let Some(r) = e(b) {
                        return Some(r);
                    }
                }
                let ad = match &callee.kind {
                    ExprKind::Ident(n) | ExprKind::Field(_, n, _) => n.clone(),
                    _ => "fonksiyon".into(),
                };
                Some((x.span, format!("{ad}()")))
            }
            ExprKind::Neg(a) | ExprKind::Not(a) | ExprKind::BitNot(a) | ExprKind::Field(a, _, _) | ExprKind::Olarak(a, _) => e(a),
            ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => e(a).or_else(|| e(b)),
            ExprKind::List(v) => v.iter().find_map(e),
            ExprKind::Dict(v) => v.iter().find_map(|(a, b)| e(a).or_else(|| e(b))),
            ExprKind::FStr(ps) => ps.iter().find_map(|p| match p {
                FPartE::Expr(x) | FPartE::Bicimli(x, _, _) => e(x),
                FPartE::Lit(_) => None,
            }),
            ExprKind::Range { start, end, .. } => start.as_deref().and_then(e).or_else(|| end.as_deref().and_then(e)),
            ExprKind::Match(m) => e(&m.scrutinee),
            _ => None,
        }
    }
    for st in stmts {
        let r = match &st.kind {
            StmtKind::Ata { value: Some(v), .. } => e(v),
            StmtKind::Assign { target, value, .. } => e(target).or_else(|| e(value)),
            StmtKind::Expr(x) | StmtKind::Firlat(x) | StmtKind::Dondur(Some(x)) => e(x),
            StmtKind::Eger { branches, else_ } => {
                branches.iter().find_map(|(c, b)| e(c).or_else(|| ilk_cagri(b))).or_else(|| else_.as_deref().and_then(ilk_cagri))
            }
            StmtKind::Kere { count: c, body } | StmtKind::Her { iter: c, body, .. } | StmtKind::Surece { cond: c, body } => {
                e(c).or_else(|| ilk_cagri(body))
            }
            StmtKind::CFor { cond, body, .. } => e(cond).or_else(|| ilk_cagri(body)),
            StmtKind::Match(m) => e(&m.scrutinee),
            StmtKind::Dene { body, .. } => ilk_cagri(body),
            _ => None,
        };
        if r.is_some() {
            return r;
        }
    }
    None
}
