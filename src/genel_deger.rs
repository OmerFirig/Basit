// Generic fonksiyon değerleri:
//   ata iki_kat = her<T> x: T için x * 2              generic lambda: her çağrıda kendi tipiyle çalışır
//   uygula(iki_kat, 5)                                 somut tip beklenen yere verilince orada somutlaşır
//   fonksiyon ikisine(f: Fonksiyon<T>(T) -> T) { }     parametre her tiple çalışan bir fonksiyon olabilir
//
// Generic bir fonksiyon değeri çalışma anında var olmaz: tip denetleyici onu bilir ve her kullanıldığı yerde
// (çağrı ya da somut fonksiyon tipi beklenen yer) tipleri belli bir kopyasını oluşturur. Generic lambda
// dışarıdaki değişkenleri kullanamaz (kopyası başka bir yerde oluşturulabilir; neyi yakalayacağı belli olmazdı).

use crate::ast::*;
use crate::cexpr::PathRes;
use crate::check::*;
use crate::diag::{err, CResult, Span};
use crate::generic::PTip;
use crate::tir::*;
use crate::types::*;
use std::collections::HashMap;

/// Generic fonksiyon değeri: adlı generic fonksiyon ya da generic lambda
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum GenelRef {
    Fonksiyon(FuncId),
    Lambda(usize),
}

pub(crate) struct GenelLambda {
    /// Mesajlarda görünen ad (atandığı değişken)
    pub ad: String,
    pub generics: Vec<String>,
    pub params: Vec<LParam>,
    pub body: LambdaBody,
    pub module: usize,
    pub span: Span,
    /// tipler -> kopyanın fonksiyonu
    pub ornekler: HashMap<Vec<Type>, FuncId>,
    /// Tanımlandığı yerde görünen generic değerler (başka bir generic lambda kullanılabilsin diye)
    pub gorunen: Vec<(String, GenelRef)>,
}

/// `Fonksiyon<T>(...)` tipiyse parametre sayısı
pub(crate) fn genel_fonk_tipi(te: &TypeExpr) -> Option<usize> {
    match &te.kind {
        TypeKind::Func { generics, params, .. } if !generics.is_empty() => Some(params.len()),
        _ => None,
    }
}

impl Checker {
    /// Generic kalıp mı: `<T>` alıyor ya da bir parametresi `Fonksiyon<T>(...)`. Kalıbın gövdesi tek başına
    /// denetlenmez, her çağrıda kopyası oluşturulur.
    pub(crate) fn kalip_mi(&self, fid: FuncId) -> bool {
        !self.funcs[fid].generics.is_empty()
            || self.funcs[fid].decl.as_ref().is_some_and(|d| d.params.iter().any(|p| genel_fonk_tipi(&p.ty).is_some()))
    }

    /// `ata f = her<T> ...`: değeri olmayan (derleme anındaki) bir generic fonksiyon tanımlar
    pub(crate) fn genel_lambda_ata(&mut self, ad: &str, generics: &[(String, Span)], params: &[LParam], body: &LambdaBody, span: Span) -> CResult<()> {
        let idx = self.genel_lambda_kaydet(ad, generics, params, body, span)?;
        self.declare_var(ad, Type::Func(vec![], Box::new(Type::Hic)), true, true);
        if let Some(v) = self.cx.scopes.last_mut().and_then(|s| s.get_mut(ad)) {
            v.genel = Some(GenelRef::Lambda(idx));
        }
        Ok(())
    }

    fn genel_lambda_kaydet(&mut self, ad: &str, generics: &[(String, Span)], params: &[LParam], body: &LambdaBody, span: Span) -> CResult<usize> {
        let gs: Vec<String> = generics.iter().map(|g| g.0.clone()).collect();
        for p in params {
            if p.ty.is_none() {
                return err(p.span, format!("Generic lambdanın parametrelerinin tipi yazılmalı: her<T> {}: T için ...", p.name));
            }
        }
        for (g, sp) in generics {
            if !params.iter().any(|p| p.ty.as_ref().is_some_and(|t| crate::generic::has_param(t, std::slice::from_ref(g)))) {
                return err(*sp, format!("'{g}' hiçbir parametrenin tipinde geçmiyor; çağrıda tipi çıkarılamaz."));
            }
        }
        // dışarıdaki değişkenler kullanılamaz: kopyası başka yerde oluşturulabildiği için neyi yakalayacağı belli olmazdı
        let mut adlar: Vec<(String, Span)> = Vec::new();
        let mut yerel: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
        match body {
            LambdaBody::Expr(e) => adlari_topla(e, &mut adlar, &mut yerel),
            LambdaBody::Block(b) => b.iter().for_each(|s| komut_adlari(s, &mut adlar, &mut yerel)),
        }
        let mut gorunen = Vec::new();
        for (n, sp) in adlar {
            if yerel.contains(&n) {
                continue;
            }
            match self.genel_var(&n) {
                Some(r) => gorunen.push((n, r)),
                None if self.var_exists(&n) => {
                    return err(sp, format!("'{n}' dışarıdaki bir değişken; generic lambda dışarıdaki değişkenleri kullanamaz. Değeri bir parametre olarak verin."));
                }
                None => {}
            }
        }
        self.genel_lambdalar.push(GenelLambda {
            ad: ad.to_string(),
            generics: gs,
            params: params.to_vec(),
            body: body.clone(),
            module: self.cx.module,
            span,
            ornekler: HashMap::new(),
            gorunen,
        });
        Ok(self.genel_lambdalar.len() - 1)
    }

    /// Değişkeni lambdaların dışına da bakarak bulur (yakalamadan); generic işaretliyse döndürür
    pub(crate) fn genel_var(&self, n: &str) -> Option<GenelRef> {
        let ara = |c: &Ctx| c.scopes.iter().rev().find_map(|s| s.get(n)).or_else(|| c.snapshot.as_ref().and_then(|s| s.get(n))).cloned();
        if let Some(v) = ara(&self.cx) {
            return v.genel;
        }
        for f in self.lambda_stack.iter().rev() {
            if let Some(v) = ara(&f.outer) {
                return v.genel;
            }
        }
        None
    }

    /// İfade generic bir fonksiyonun adı mı: generic değişken, `<T>` alan fonksiyon ya da `dosya.fonksiyon`
    pub(crate) fn genel_ref(&self, e: &Expr) -> Option<GenelRef> {
        let fonk = |me: &Self, mi: usize, n: &str| {
            me.mods[mi].funcs.get(n).copied().filter(|&f| !me.funcs[f].generics.is_empty() && !me.kalip_param_var(f)).map(GenelRef::Fonksiyon)
        };
        match &e.kind {
            ExprKind::Ident(n) => {
                if let Some(r) = self.genel_var(n) {
                    return Some(r);
                }
                if self.var_exists(n) {
                    return None;
                }
                fonk(self, self.cx.module, n)
            }
            ExprKind::Field(b, n, _) => match self.static_path(b) {
                Some(PathRes::Module(mi)) => self.mods[mi].globals.get(n).and_then(|v| v.genel).or_else(|| fonk(self, mi, n)),
                _ => None,
            },
            _ => None,
        }
    }

    /// Fonksiyonun `Fonksiyon<T>(...)` parametresi var mı (böyle fonksiyonlar değer olarak verilemez)
    pub(crate) fn kalip_param_var(&self, f: FuncId) -> bool {
        self.funcs[f].decl.as_ref().is_some_and(|d| d.params.iter().any(|p| genel_fonk_tipi(&p.ty).is_some()))
    }

    /// Generic değer gereken yerde yazılan ifade: adı ya da doğrudan generic lambda
    pub(crate) fn genel_arguman(&mut self, e: &Expr) -> CResult<Option<GenelRef>> {
        if let ExprKind::Lambda { generics, params, body } = &e.kind {
            if !generics.is_empty() {
                let i = self.genel_lambda_kaydet("isimsiz fonksiyon", generics, params, body, e.span)?;
                return Ok(Some(GenelRef::Lambda(i)));
            }
        }
        Ok(self.genel_ref(e))
    }

    fn genel_ad(&self, r: GenelRef) -> String {
        match r {
            GenelRef::Fonksiyon(f) => self.funcs[f].name.clone(),
            GenelRef::Lambda(i) => self.genel_lambdalar[i].ad.clone(),
        }
    }

    /// Tip parametreleri, parametre tipleri ve tanımın dosyası
    fn genel_imza(&self, r: GenelRef) -> (Vec<String>, Vec<TypeExpr>, usize) {
        match r {
            GenelRef::Fonksiyon(f) => {
                let d = self.funcs[f].decl.as_ref().expect("generic kalıp");
                (self.funcs[f].generics.clone(), d.params.iter().map(|p| p.ty.clone()).collect(), self.funcs[f].module)
            }
            GenelRef::Lambda(i) => {
                let g = &self.genel_lambdalar[i];
                (g.generics.clone(), g.params.iter().map(|p| p.ty.clone().expect("tipli parametre")).collect(), g.module)
            }
        }
    }

    pub(crate) fn genel_parametre_sayisi(&self, r: GenelRef) -> usize {
        self.genel_imza(r).1.len()
    }

    /// Mesajlar için imza: Fonksiyon<T>(T) -> T (lambdanın dönüş tipi çağrıda belli olur)
    pub(crate) fn genel_imza_yazisi(&self, r: GenelRef) -> String {
        let (gs, ps, _) = self.genel_imza(r);
        let ps: Vec<String> = ps.iter().map(|t| self.kaynak_yazisi(t.span)).collect();
        let don = match r {
            GenelRef::Fonksiyon(f) => match &self.funcs[f].decl.as_ref().expect("kalıp").ret {
                Some(t) => format!(" -> {}", self.kaynak_yazisi(t.span)),
                None => String::new(),
            },
            GenelRef::Lambda(_) => " -> (çağrıda belli olur)".to_string(),
        };
        format!("Fonksiyon<{}>({}){don}", gs.join(", "), ps.join(", "))
    }

    pub(crate) fn kaynak(&self, sp: Span) -> String {
        self.kaynak_yazisi(sp)
    }

    fn kaynak_yazisi(&self, sp: Span) -> String {
        self.texts.get(sp.file as usize).and_then(|t| t.get(sp.start as usize..sp.end as usize)).unwrap_or("?").to_string()
    }

    /// Generic değer değer olarak kullanıldı ama tipi belli değil
    pub(crate) fn genel_deger_hatasi(&self, r: GenelRef, span: Span) -> crate::diag::Diag {
        let ad = self.genel_ad(r);
        crate::diag::Diag::new(
            span,
            format!(
                "'{ad}' her tiple çalışan (generic) bir fonksiyon: {}. Çağırın: {ad}(...), ya da tipi belli bir fonksiyon bekleyen yere verin: uygula({ad}, 5)",
                self.genel_imza_yazisi(r)
            ),
        )
    }

    /// Generic değeri bu argümanlarla çağırır: tipler argümanlardan çıkarılır
    pub(crate) fn genel_cagri(&mut self, r: GenelRef, args: &[Arg], span: Span) -> CResult<TExpr> {
        let i = match r {
            GenelRef::Fonksiyon(f) => return self.call_generic(f, args, span),
            GenelRef::Lambda(i) => i,
        };
        let (gs, pts, mi) = self.genel_imza(r);
        let ad = self.genel_ad(r);
        let params: Vec<(String, PTip, bool)> = self.genel_lambdalar[i].params.iter().zip(pts).map(|(p, t)| (p.name.clone(), PTip::Yazilan(t), false)).collect();
        let (tipler, vals, _) = self.infer_call(&ad, &gs, mi, &params, args, span)?;
        let fid = self.lambda_ornegi(i, tipler, span)?;
        let ptys: Vec<Type> = self.funcs[fid].params.iter().map(|p| p.1.clone()).collect();
        let vals: Vec<TExpr> = self.coerce_all(vals, &ptys)?.into_iter().map(|v| v.expect("değer")).collect();
        let f = self.lambda_degeri(fid, span);
        let ret = self.funcs[fid].ret.clone();
        Ok(self.mk(TK::CallValue(Box::new(f), vals), ret, span))
    }

    /// Somut fonksiyon tipi beklenen yerde generic değer: `uygula(iki_kat, 5)`. Tipler beklenen parametre
    /// tiplerinden çıkarılır; dönüş tipi beklenen varsa uymalı.
    pub(crate) fn genel_deger(&mut self, r: GenelRef, ps: &[Type], ret: Option<&Type>, span: Span) -> CResult<TExpr> {
        let (gs, pts, mi) = self.genel_imza(r);
        let ad = self.genel_ad(r);
        if pts.len() != ps.len() {
            return err(span, format!("'{ad}' {} değer alır, ama burada {} değer alan bir fonksiyon bekleniyor.", pts.len(), ps.len()));
        }
        let mut binds: HashMap<String, Type> = HashMap::new();
        for (te, a) in pts.iter().zip(ps) {
            self.infer(te, a, &gs, &mut binds, mi, span)?;
        }
        if let Some(g) = gs.iter().find(|g| !binds.contains_key(*g)) {
            return err(span, format!("'{ad}' burada hangi tiple kullanılacağı belli değil ('{g}' çıkarılamıyor)."));
        }
        let tipler: Vec<Type> = gs.iter().map(|g| binds[g].clone()).collect();
        let deger = match r {
            GenelRef::Fonksiyon(f) => {
                let inst = match self.instances.get(&(f, tipler.clone(), vec![])) {
                    Some(&i) => i,
                    None => self.new_instance(f, tipler, vec![], span)?,
                };
                self.func_ref(inst, span)
            }
            GenelRef::Lambda(i) => {
                let fid = self.lambda_ornegi(i, tipler, span)?;
                self.lambda_degeri(fid, span)
            }
        };
        let Type::Func(gp, gr) = deger.ty.clone() else { unreachable!("fonksiyon") };
        if gp.as_slice() != ps || ret.is_some_and(|r| *gr != *r) {
            let beklenen = Type::Func(ps.to_vec(), Box::new(ret.cloned().unwrap_or((*gr).clone())));
            return err(
                span,
                format!("'{ad}' burada {} olarak kullanılamaz; bu tiplerle {} oluyor.", self.type_name(&beklenen), self.type_name(&deger.ty)),
            );
        }
        Ok(deger)
    }

    fn lambda_degeri(&self, fid: FuncId, span: Span) -> TExpr {
        let ty = Type::Func(self.funcs[fid].params.iter().map(|p| p.1.clone()).collect(), Box::new(self.funcs[fid].ret.clone()));
        self.mk(TK::Lambda { func: fid, caps: vec![], bu: None }, ty, span)
    }

    /// Generic lambdanın bu tiplerle kopyası. Tanımlandığı dosyada, hiçbir değişken görünmeden denetlenir.
    fn lambda_ornegi(&mut self, i: usize, tipler: Vec<Type>, span: Span) -> CResult<FuncId> {
        if let Some(&f) = self.genel_lambdalar[i].ornekler.get(&tipler) {
            return Ok(f);
        }
        let g = &self.genel_lambdalar[i];
        let (params, body, mi, tanim, ad) = (g.params.clone(), g.body.clone(), g.module, g.span, g.ad.clone());
        let gorunen = g.gorunen.clone();
        let binds: Vec<(String, Type)> = g.generics.iter().cloned().zip(tipler.iter().cloned()).collect();
        let eski_cx = std::mem::replace(&mut self.cx, Ctx { module: mi, scopes: vec![HashMap::new()], genel_lambda: true, ..Default::default() });
        let eski_lambda = std::mem::take(&mut self.lambda_stack);
        let eski_tip = std::mem::replace(&mut self.tsubst, binds.iter().cloned().collect());
        for (n, r) in gorunen {
            self.declare_var(&n, Type::Func(vec![], Box::new(Type::Hic)), true, true);
            if let Some(v) = self.cx.scopes.last_mut().and_then(|s| s.get_mut(&n)) {
                v.genel = Some(r);
            }
        }
        let r = self.lambda(&params, &body, None, None, tanim);
        self.cx = eski_cx;
        self.lambda_stack = eski_lambda;
        self.tsubst = eski_tip;
        let t = r.map_err(|d| self.kopya_hatasi(d, &ad, "çağrı", span, &binds, &[]))?;
        let TK::Lambda { func, .. } = t.kind else { unreachable!("lambda") };
        self.genel_lambdalar[i].ornekler.insert(tipler, func);
        Ok(func)
    }
}

/// İfadede geçen adlar (değişken olabilecekler). `yerel`: gövdede tanımlananlar (ata, döngü değişkeni).
fn adlari_topla(e: &Expr, out: &mut Vec<(String, Span)>, yerel: &mut Vec<String>) {
    let mut t = |x: &Expr| adlari_topla(x, out, yerel);
    match &e.kind {
        ExprKind::Ident(n) => out.push((n.clone(), e.span)),
        ExprKind::List(v) => v.iter().for_each(t),
        ExprKind::Dict(v) => v.iter().for_each(|(a, b)| {
            adlari_topla(a, out, yerel);
            adlari_topla(b, out, yerel);
        }),
        ExprKind::Neg(x) | ExprKind::Not(x) | ExprKind::BitNot(x) | ExprKind::Field(x, _, _) | ExprKind::Olarak(x, _) => t(x),
        ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
            t(a);
            t(b);
        }
        ExprKind::Range { start, end, .. } => {
            start.iter().for_each(|x| adlari_topla(x, out, yerel));
            end.iter().for_each(|x| adlari_topla(x, out, yerel));
        }
        ExprKind::Call { callee, args } => {
            t(callee);
            args.iter().for_each(|a| adlari_topla(&a.value, out, yerel));
        }
        ExprKind::FStr(ps) => ps.iter().for_each(|p| match p {
            FPartE::Expr(x) | FPartE::Bicimli(x, _, _) => adlari_topla(x, out, yerel),
            FPartE::Lit(_) => {}
        }),
        ExprKind::Match(m) => eslestir_adlari(m, out, yerel),
        ExprKind::Lambda { params, body, .. } => {
            yerel.extend(params.iter().map(|p| p.name.clone()));
            match body {
                LambdaBody::Expr(x) => adlari_topla(x, out, yerel),
                LambdaBody::Block(b) => b.iter().for_each(|s| komut_adlari(s, out, yerel)),
            }
        }
        _ => {}
    }
}

fn eslestir_adlari(m: &Match, out: &mut Vec<(String, Span)>, yerel: &mut Vec<String>) {
    adlari_topla(&m.scrutinee, out, yerel);
    for a in &m.arms {
        for p in a.pats.iter().flatten() {
            match p {
                Pattern::Path { binds: Some(bs), .. } => yerel.extend(bs.iter().flatten().map(|b| b.0.clone())),
                Pattern::Typed { name: Some((n, _)), .. } => yerel.push(n.clone()),
                _ => {}
            }
        }
        if let Some(g) = &a.guard {
            adlari_topla(g, out, yerel);
        }
        match &a.body {
            ArmBody::Expr(x) => adlari_topla(x, out, yerel),
            ArmBody::Block(b) => b.iter().for_each(|s| komut_adlari(s, out, yerel)),
            ArmBody::Stmt(s) => komut_adlari(s, out, yerel),
        }
    }
}

fn komut_adlari(s: &Stmt, out: &mut Vec<(String, Span)>, yerel: &mut Vec<String>) {
    let blok = |b: &Block, out: &mut Vec<(String, Span)>, yerel: &mut Vec<String>| b.iter().for_each(|s| komut_adlari(s, out, yerel));
    match &s.kind {
        StmtKind::Ata { name, value, .. } => {
            if let Some(v) = value {
                adlari_topla(v, out, yerel);
            }
            yerel.push(name.clone());
        }
        StmtKind::Assign { target, value, .. } => {
            adlari_topla(target, out, yerel);
            adlari_topla(value, out, yerel);
        }
        StmtKind::Expr(e) | StmtKind::Firlat(e) | StmtKind::Dondur(Some(e)) => adlari_topla(e, out, yerel),
        StmtKind::Eger { branches, else_ } => {
            for (c, b) in branches {
                adlari_topla(c, out, yerel);
                blok(b, out, yerel);
            }
            if let Some(b) = else_ {
                blok(b, out, yerel);
            }
        }
        StmtKind::Kere { count: c, body } | StmtKind::Surece { cond: c, body } => {
            adlari_topla(c, out, yerel);
            blok(body, out, yerel);
        }
        StmtKind::Her { iter, pat, body } => {
            adlari_topla(iter, out, yerel);
            yerel.extend(pat.iter().map(|p| p.0.clone()));
            blok(body, out, yerel);
        }
        StmtKind::CFor { init, cond, step, body } => {
            komut_adlari(init, out, yerel);
            adlari_topla(cond, out, yerel);
            komut_adlari(step, out, yerel);
            blok(body, out, yerel);
        }
        StmtKind::Match(m) => eslestir_adlari(m, out, yerel),
        StmtKind::Dene { body, catches } => {
            blok(body, out, yerel);
            for c in catches {
                yerel.push(c.name.clone());
                blok(&c.body, out, yerel);
            }
        }
        _ => {}
    }
}
