// Generic fonksiyonlar: `fonksiyon en_büyük<T>(l: Liste<T>) -> T { ... }`
//
// Kalıp tarzı: kalıbın gövdesi tek başına denetlenmez. Her çağrıda tip parametreleri argümanlardan
// çıkarılır (T = Tam(64)) ve bu tiplerle fonksiyonun ayrı bir kopyası oluşturulur; kopya normal bir
// fonksiyon gibi denetlenir ve derlenir. Aynı tiplerle yapılan çağrılar aynı kopyayı kullanır.
// Böylece yorumlayıcı ve derleyici generic'ten habersizdir, çalışma anında maliyet yoktur.

use crate::ast::*;
use crate::check::*;
use crate::diag::{err, CResult, Diag, Span};
use crate::tir::*;
use crate::types::*;
use std::collections::HashMap;

/// Bir fonksiyonun kopya sayısı sınırı (kendini her seferinde başka bir tiple çağıran fonksiyon için)
const KOPYA_SINIRI: usize = 64;

/// Generic çağrıdaki bir parametrenin tipi
#[derive(Clone)]
pub(crate) enum PTip {
    /// Kaynakta yazılan, tip parametresi içerebilir
    Yazilan(TypeExpr),
    /// Ebeveynden gelen alan: tipi belli
    Belli(Type),
    /// `Fonksiyon<T>(...)` parametresi (kaç değer aldığı): değeri generic bir fonksiyon olmalı
    GenelFonk(usize),
}

/// Tip ifadesinde tip parametrelerinden biri geçiyor mu?
pub(crate) fn has_param(te: &TypeExpr, gs: &[String]) -> bool {
    match &te.kind {
        TypeKind::Path { path, generics, .. } => {
            (path.len() == 1 && gs.contains(&path[0])) || generics.iter().any(|g| has_param(g, gs))
        }
        TypeKind::Union(ps) => ps.iter().any(|p| has_param(p, gs)),
        TypeKind::Func { params, ret, .. } => params.iter().any(|p| has_param(p, gs)) || ret.as_deref().is_some_and(|r| has_param(r, gs)),
    }
}

/// Sayı sabiti değilse doğru; sabitse `t` tipine sığıyor mu?
fn lit_fits(e: &Expr, t: &Type) -> bool {
    let (n, neg) = match &e.kind {
        ExprKind::Neg(x) => (&**x, true),
        _ => (e, false),
    };
    match (&n.kind, t) {
        (ExprKind::Int(v), Type::Int { bits, sign }) => fits(&if neg { -v.clone() } else { v.clone() }, *bits, *sign),
        (ExprKind::Int(_), Type::Float(_)) | (ExprKind::Float(_), Type::Float(_)) => true,
        (ExprKind::Int(_) | ExprKind::Float(_), _) => false,
        _ => !neg || !matches!(n.kind, ExprKind::Int(_) | ExprKind::Float(_)),
    }
}

/// Tip ifadesindeki tip parametrelerinin hepsinin karşılığı bulundu mu?
fn all_bound(te: &TypeExpr, gs: &[String], binds: &HashMap<String, Type>) -> bool {
    let free: Vec<String> = gs.iter().filter(|g| !binds.contains_key(*g)).cloned().collect();
    !has_param(te, &free)
}

impl Checker {
    /// `f` çalışırken tip parametreleri `b` olur (kalıbın imzasındaki tipleri çözmek için)
    fn with_subst<R>(&mut self, b: &HashMap<String, Type>, f: impl FnOnce(&mut Self) -> R) -> R {
        let eski = std::mem::replace(&mut self.tsubst, b.clone());
        let r = f(self);
        self.tsubst = eski;
        r
    }

    /// Generic fonksiyon çağrısı: tipleri çıkar, kopyayı bul ya da oluştur, kopyayı çağır.
    pub(crate) fn call_generic(&mut self, fid: FuncId, args: &[Arg], span: Span) -> CResult<TExpr> {
        let name = self.funcs[fid].name.clone();
        let gs = self.funcs[fid].generics.clone();
        let mi = self.funcs[fid].module;
        let decl = self.funcs[fid].decl.clone().expect("generic kalıp");
        let params: Vec<(String, PTip, bool)> = decl
            .params
            .iter()
            .map(|p| {
                let tip = match crate::genel_deger::genel_fonk_tipi(&p.ty) {
                    Some(n) => PTip::GenelFonk(n),
                    None => PTip::Yazilan(p.ty.clone()),
                };
                (p.name.clone(), tip, p.varsayilan.is_some())
            })
            .collect();
        let (tipler, vals, refs) = self.infer_call(&name, &gs, mi, &params, args, span)?;
        // Fonksiyon<T> parametrelerine verilenler kopyanın parçasıdır, çalışma anında verilmez
        let refs: Vec<GenelRef> = refs.into_iter().flatten().collect();
        let vals: Vec<Option<(TExpr, Span)>> =
            vals.into_iter().zip(&params).filter(|(_, p)| !matches!(p.1, PTip::GenelFonk(_))).map(|(v, _)| v).collect();
        let inst = match self.instances.get(&(fid, tipler.clone(), refs.clone())) {
            Some(&f) => f,
            None => self.new_instance(fid, tipler, refs, span)?,
        };
        let ptys: Vec<Type> = self.funcs[inst].params.iter().map(|p| p.1.clone()).collect();
        let mut out = Vec::new();
        for (i, v) in self.coerce_all(vals, &ptys)?.into_iter().enumerate() {
            out.push(match v {
                Some(t) => t,
                None => self.fonk_varsayilan(inst, i)?,
            });
        }
        let ret = self.funcs[inst].ret.clone();
        Ok(self.mk(TK::Call(inst, out), ret, span))
    }

    /// Generic tür kalıbından nesne: `Kutu(değer: 5)` -> Kutu<Tam(64)>. Beklenen tip bu kalıbın bir
    /// kopyasıysa (`değişken k: Kutu<Tam(8)> = Kutu(değer: 5)`) o kullanılır.
    pub(crate) fn construct_generic(&mut self, tid: TypeId, args: &[Arg], expected: Option<&Type>, span: Span) -> CResult<TExpr> {
        let hedef = expected.and_then(|e| {
            e.members().into_iter().find_map(|m| match m {
                Type::Named(i) if self.tur_inst.get(&i).is_some_and(|x| x.0 == tid) => Some(i),
                _ => None,
            })
        });
        if let Some(inst) = hedef {
            let tname = self.types[inst].name.clone();
            return self.kur(inst, args, span, &tname);
        }
        let (gs, decl) = self.tur_kalip[&tid].clone();
        let TypeDecl::Tur(d) = decl else { unreachable!("generic seçenek yok") };
        let mi = self.type_decls[tid].0;
        // ebeveynlerin alanları önce (tipleri belli), sonra türün kendi alanları
        let mut params: Vec<(String, PTip, bool)> = Vec::new();
        for (path, pspan) in &d.parents {
            let te = TypeExpr { kind: TypeKind::Path { path: path.clone(), generics: vec![], args: None, dims: vec![] }, span: *pspan };
            if let Type::Named(p) = self.with_subst(&HashMap::new(), |c| c.resolve_type(&te, mi))? {
                for f in self.fields_of(p).to_vec() {
                    if f.kurucuda && !params.iter().any(|x| *x.0 == *f.name) {
                        let v = self.alan_varsayilani_var(&f);
                        params.push((f.name.to_string(), PTip::Belli(f.ty), v));
                    }
                }
            }
        }
        params.extend(d.fields.iter().map(|p| (p.name.clone(), PTip::Yazilan(p.ty.clone()), p.varsayilan.is_some())));
        let (tipler, vals, _) = self.infer_call(&d.name, &gs, mi, &params, args, span)?;
        let inst = self.tur_instance(tid, tipler, span)?;
        let ftys: Vec<Type> = self.fields_of(inst).iter().filter(|f| f.kurucuda).map(|f| f.ty.clone()).collect();
        let verilen = self.coerce_all(vals, &ftys)?;
        self.nesne(inst, verilen, span)
    }

    pub(crate) fn coerce_all(&mut self, vals: Vec<Option<(TExpr, Span)>>, tys: &[Type]) -> CResult<Vec<Option<TExpr>>> {
        let mut out = Vec::new();
        for (v, ty) in vals.into_iter().zip(tys) {
            out.push(match v {
                Some((t, sp)) => Some(self.coerce(t, ty, sp)?),
                None => None,
            });
        }
        Ok(out)
    }

    /// Argümanları denetler ve tip parametrelerini onlardan çıkarır. Dönüş: tipler (`gs` sırasıyla),
    /// parametre sırasıyla değerler (henüz parametre tipine çevrilmemiş) ve yerleri, `Fonksiyon<T>`
    /// parametrelerine verilen generic fonksiyonlar.
    #[allow(clippy::type_complexity)]
    pub(crate) fn infer_call(
        &mut self,
        name: &str,
        gs: &[String],
        mi: usize,
        params: &[(String, PTip, bool)],
        args: &[Arg],
        span: Span,
    ) -> CResult<(Vec<Type>, Vec<Option<(TExpr, Span)>>, Vec<Option<GenelRef>>)> {
        let names: Vec<String> = params.iter().map(|p| p.0.clone()).collect();
        let idx = self.arg_slots(&names, args, span, name)?;
        // verilmeyen parametrenin varsayılanı yoksa eksik (varsayılan değerden tip çıkarılmaz)
        if let Some(i) = (0..names.len()).find(|i| !idx.contains(i) && !params[*i].2) {
            return err(span, format!("'{name}' için '{}' değeri eksik.", names[i]));
        }
        let mut binds: HashMap<String, Type> = HashMap::new();
        let mut vals: Vec<Option<(TExpr, Span)>> = vec![None; names.len()];
        let mut refs: Vec<Option<GenelRef>> = vec![None; names.len()];
        // Önce tipi kendinden belli değerler, sonra sayı sabitleri (T belliyse ona uysun diye: Tam(8)
        // ile 5 verilince T = Tam(8)), en son lambdalar (parametre tipleri diğerlerinden gelir).
        // generic fonksiyon adları (kare, iki_kat) de lambdalar gibi sona kalır
        let rank = |me: &Self, e: &Expr| match &e.kind {
            ExprKind::Lambda { .. } => 2,
            _ if me.genel_ref(e).is_some() => 2,
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Neg(_) => 1,
            _ => 0,
        };
        let mut order: Vec<usize> = (0..args.len()).collect();
        order.sort_by_key(|&a| rank(self, &args[a].value));
        for a in order {
            let arg = &args[a];
            let sp = arg.value.span;
            let te = match &params[idx[a]].1 {
                PTip::Belli(t) => {
                    let t = t.clone();
                    vals[idx[a]] = Some((self.expr(&arg.value, Some(&t))?, sp));
                    continue;
                }
                PTip::GenelFonk(n) => {
                    let pn = &params[idx[a]].0;
                    let Some(r) = self.genel_arguman(&arg.value)? else {
                        // hiç tanımlı olmayan ad: önce onu söyle
                        if let ExprKind::Ident(n) = &arg.value.kind {
                            if !self.var_exists(n) && !self.mods[self.cx.module].funcs.contains_key(n) {
                                return Err(self.unknown_name(n, sp));
                            }
                        }
                        return err(
                            sp,
                            format!("'{pn}' her tiple çalışan (generic) bir fonksiyon bekliyor. Örnek: fonksiyon iki_kat<T>(x: T) -> T {{ ... }} ya da her<T> x: T için ..."),
                        );
                    };
                    let m = self.genel_parametre_sayisi(r);
                    if m != *n {
                        return err(sp, format!("'{pn}' {n} değer alan bir fonksiyon bekliyor, ama verilen fonksiyon {m} değer alıyor."));
                    }
                    refs[idx[a]] = Some(r);
                    continue;
                }
                PTip::Yazilan(te) => te,
            };
            if all_bound(te, gs, &binds) {
                let pty = self.with_subst(&binds, |c| c.resolve_type(te, mi))?;
                // Sayı sabiti T'ye sığmıyorsa (T = Tam(8) iken 1000) ya da T sayı değilse, T genişler
                // ya da çakışma hatası verilir; diğer değerler T'ye uymak zorunda.
                if !has_param(te, gs) || lit_fits(&arg.value, &pty) {
                    vals[idx[a]] = Some((self.expr(&arg.value, Some(&pty))?, sp));
                    continue;
                }
            }
            // generic fonksiyon değeri: parametre tipleri diğer değerlerden belli olunca somutlaşır
            if let TypeKind::Func { params: fps, ret, .. } = &te.kind {
                if let Some(g) = self.genel_arguman(&arg.value)? {
                    let mut ptys = Vec::new();
                    for fp in fps {
                        if !all_bound(fp, gs, &binds) {
                            return err(sp, format!("'{}' burada hangi tiple kullanılacağı belli değil; diğer değerlerden çıkarılamıyor.", self.kaynak(sp)));
                        }
                        ptys.push(self.with_subst(&binds, |c| c.resolve_type(fp, mi))?);
                    }
                    let rt = match ret {
                        Some(r) if all_bound(r, gs, &binds) => Some(self.with_subst(&binds, |c| c.resolve_type(r, mi))?),
                        Some(_) => None,
                        None => Some(Type::Hic),
                    };
                    let t = self.genel_deger(g, &ptys, rt.as_ref(), sp)?;
                    self.infer(te, &t.ty, gs, &mut binds, mi, sp)?;
                    vals[idx[a]] = Some((t, sp));
                    continue;
                }
            }
            let t = match (&arg.value.kind, &te.kind) {
                (ExprKind::Lambda { params: lp, body, .. }, TypeKind::Func { params: fps, ret, .. }) => {
                    let mut ptys = Vec::new();
                    for fp in fps {
                        if !all_bound(fp, gs, &binds) {
                            return err(sp, "Lambdanın parametre tipi çıkarılamıyor. Tipini yazın: her (x: Tam(64)) için ...");
                        }
                        ptys.push(self.with_subst(&binds, |c| c.resolve_type(fp, mi))?);
                    }
                    let rt = match ret {
                        Some(r) if all_bound(r, gs, &binds) => Some(self.with_subst(&binds, |c| c.resolve_type(r, mi))?),
                        Some(_) => None,
                        None => Some(Type::Hic),
                    };
                    self.lambda(lp, body, Some(&ptys), rt.as_ref(), sp)?
                }
                _ => self.expr(&arg.value, None)?,
            };
            self.infer(te, &t.ty, gs, &mut binds, mi, sp)?;
            vals[idx[a]] = Some((t, sp));
        }
        for g in gs {
            if !binds.contains_key(g) {
                return err(
                    span,
                    format!(
                        "'{name}' için '{g}' tipi verilen değerlerden çıkarılamıyor. '{g}' bir parametrenin tipinde geçmeli; \
                         boş liste gibi tipi belli olmayan değerlere tip verin: [] olarak Liste<Tam>"
                    ),
                );
            }
        }
        let tipler = gs.iter().map(|g| binds[g].clone()).collect();
        Ok((tipler, vals, refs))
    }

    /// Generic türün kopyası: `Kutu<Tam(64)>`. İlk kez istenince oluşturulur ve hemen çözülür (alanlar,
    /// metot imzaları; kalıtım tabloları kurulduysa onlar da).
    pub(crate) fn tur_instance(&mut self, tid: TypeId, ts: Vec<Type>, span: Span) -> CResult<TypeId> {
        if let Some(&i) = self.tur_instances.get(&(tid, ts.clone())) {
            return Ok(i);
        }
        let tname = self.types[tid].name.clone();
        if self.tur_inst.values().filter(|x| x.0 == tid).count() >= KOPYA_SINIRI {
            return err(span, format!("'{tname}' {KOPYA_SINIRI}'ten fazla farklı tiple kullanılıyor. Tür kendini her seferinde başka bir tiple kullanıyor olabilir."));
        }
        let adlar: Vec<String> = ts.iter().map(|t| self.type_name(t)).collect();
        let (_, decl) = self.tur_kalip[&tid].clone();
        let (mi, tspan, _) = self.type_decls[tid];
        let new = self.types.len();
        self.types.push(TypeDef {
            name: format!("{tname}<{}>", adlar.join(", ")),
            kind: self.types[tid].kind.clone(),
            interfaces: vec![],
            methods: HashMap::new(),
        });
        self.type_decls.push((mi, tspan, Some(decl)));
        self.own_fields.push(Vec::new());
        self.type_methods.push(Vec::new());
        self.tur_inst.insert(new, (tid, ts.clone(), span));
        self.tur_instances.insert((tid, ts), new);
        let binds: Vec<(String, Type)> = self.tur_kalip[&tid].0.iter().cloned().zip(self.tur_inst[&new].1.iter().cloned()).collect();
        if let Err(d) = self.resolve_type_decl(new) {
            return Err(self.kopya_hatasi(d, &tname, "ilk kullanım", span, &binds, &[]));
        }
        if self.miras_hazir {
            self.inherit_one(new)?;
            self.mutates.resize(self.funcs.len(), false);
        }
        Ok(new)
    }

    /// Generic fonksiyonun kopyası. `refs`: `Fonksiyon<T>` parametrelerine (sırasıyla) verilen generic
    /// fonksiyonlar; bu parametreler kopyada yoktur, gövdede derleme anındaki değişkenlerdir.
    pub(crate) fn new_instance(&mut self, fid: FuncId, tipler: Vec<Type>, refs: Vec<GenelRef>, span: Span) -> CResult<FuncId> {
        let name = self.funcs[fid].name.clone();
        if self.instances.keys().filter(|k| k.0 == fid).count() >= KOPYA_SINIRI {
            return err(
                span,
                format!(
                    "'{name}' {KOPYA_SINIRI}'ten fazla farklı tiple çağrılıyor. Fonksiyon kendini her seferinde başka bir tiple \
                     (örneğin [x] ile) çağırıyor olabilir."
                ),
            );
        }
        let gs = self.funcs[fid].generics.clone();
        let mi = self.funcs[fid].module;
        let mut decl = self.funcs[fid].decl.clone().expect("generic kalıp");
        let genel_adlar: Vec<String> =
            decl.params.iter().filter(|p| crate::genel_deger::genel_fonk_tipi(&p.ty).is_some()).map(|p| p.name.clone()).collect();
        decl.params.retain(|p| crate::genel_deger::genel_fonk_tipi(&p.ty).is_none());
        let genel_baglar: Vec<(String, GenelRef)> = genel_adlar.into_iter().zip(refs.iter().copied()).collect();
        let ekler = self.genel_bag_yazilari(&genel_baglar);
        let binds: Vec<(String, Type)> = gs.into_iter().zip(tipler.iter().cloned()).collect();
        let map: HashMap<String, Type> = binds.iter().cloned().collect();
        let (params, param_sabit, ret) = match self.with_subst(&map, |c| c.sig_of(&decl, mi)) {
            Ok(x) => x,
            Err(d) => return Err(self.kopya_hatasi(d, &name, "çağrı", span, &binds, &ekler)),
        };
        let inst = self.funcs.len();
        self.funcs.push(FuncInfo {
            name,
            module: mi,
            owner: None,
            params,
            param_sabit,
            ret,
            span: decl.span,
            yeniden: Yeniden::Yok,
            varsayilan: decl.params.iter().map(|p| p.varsayilan.clone()).collect(),
            decl: Some(decl),
            snapshot: None,
            body: None,
            dis: None,
            generics: vec![],
            inst: Some(Inst { kalip: Kalip::Fonksiyon(fid), binds, call: span, genel_baglar }),
            ortak_of: None,
        });
        self.mutates.push(false);
        self.instances.insert((fid, tipler, refs), inst);
        Ok(inst)
    }

    /// Generic kopyada çıkan hataya hangi çağrının ve hangi tiplerin yol açtığını ekler:
    /// "topla içinde: ... \n(çağrı: satır 5, T = Yazı)"
    /// `ekler`: tiplerin yanında yazılacak başka bağlamalar ("f = iki_kat" gibi)
    pub(crate) fn kopya_hatasi(&self, mut d: Diag, ad: &str, nerede: &str, cagri: Span, binds: &[(String, Type)], ekler: &[String]) -> Diag {
        if d.msg.contains(&format!("\n({nerede}: satır")) {
            return d; // iç içe kopyalarda bilgi bir kez eklenir (en içteki)
        }
        let mut tipler: Vec<String> = binds
            .iter()
            .map(|(n, t)| {
                let tn = self.type_name(t);
                match tn.char_indices().nth(60) {
                    Some((i, _)) => format!("{n} = {}...", &tn[..i]),
                    None => format!("{n} = {tn}"),
                }
            })
            .collect();
        tipler.extend(ekler.iter().cloned());
        // çağrı başka bir dosyadaysa adı da yazılır (hata kütüphanede, çağrı ana dosyada olabilir)
        let dosya = match self.dosya_adlari.get(cagri.file as usize) {
            Some(n) if cagri.file != d.span.file => format!("{n}, "),
            _ => String::new(),
        };
        d.msg = format!("{ad} içinde: {}\n({nerede}: {dosya}satır {}, {})", d.msg, cagri.line, tipler.join(", "));
        d
    }

    /// Hata mesajı için: "f = iki_kat"
    pub(crate) fn genel_bag_yazilari(&self, baglar: &[(String, GenelRef)]) -> Vec<String> {
        baglar
            .iter()
            .map(|(n, r)| match r {
                GenelRef::Fonksiyon(f) => format!("{n} = {}", self.funcs[*f].name),
                GenelRef::Lambda(i) => format!("{n} = {}", self.genel_lambdalar[*i].ad),
            })
            .collect()
    }

    /// Yazılan yol (`Kutu`, `m.Kutu`) bir generic tür kalıbıysa onun numarası
    fn kalip_of(&self, path: &[String], mi: usize) -> Option<TypeId> {
        let mut module = mi;
        for part in &path[..path.len() - 1] {
            module = *self.mods[module].namespaces.get(part)?;
        }
        let tid = *self.mods[module].types.get(path.last()?)?;
        self.tur_kalip.contains_key(&tid).then_some(tid)
    }

    /// Parametre tipini (`te`) verilen değerin tipiyle (`actual`) karşılaştırıp tip parametrelerini bulur:
    /// `Liste<T>` ile `Liste<Yazı>` -> T = Yazı. Uymayan yerler burada değil, değer parametreye
    /// verilirken (coerce) hata verir.
    pub(crate) fn infer(&mut self, te: &TypeExpr, actual: &Type, gs: &[String], binds: &mut HashMap<String, Type>, mi: usize, span: Span) -> CResult<()> {
        match &te.kind {
            TypeKind::Path { path, generics, dims, .. } => {
                if path.len() == 1 && gs.contains(&path[0]) {
                    let g = &path[0];
                    if matches!(actual, Type::Hic | Type::Never) {
                        return err(span, format!("Bu bir değer değil; '{g}' tipindeki parametreye verilemez."));
                    }
                    let yeni = match binds.get(g) {
                        None => actual.clone(),
                        Some(old) if old == actual => return Ok(()),
                        Some(old) => match self.unify(&[old.clone(), actual.clone()], span, "") {
                            Ok(t) => t,
                            Err(_) => {
                                return err(
                                    span,
                                    format!(
                                        "'{g}' hem {} hem {} olamaz: aynı tip parametresine farklı tipte değerler verildi.",
                                        self.type_name(old),
                                        self.type_name(actual)
                                    ),
                                );
                            }
                        },
                    };
                    binds.insert(g.clone(), yeni);
                    return Ok(());
                }
                if let Type::Named(i) = actual {
                    if let Some((kalip, ts, _)) = self.tur_inst.get(i).cloned() {
                        if self.kalip_of(path, mi) == Some(kalip) {
                            for (g, t) in generics.iter().zip(&ts) {
                                self.infer(g, t, gs, binds, mi, span)?;
                            }
                        }
                        return Ok(());
                    }
                }
                match (path.last().map(String::as_str), actual) {
                    (Some("Liste"), _) if path.len() == 1 && generics.len() == 1 => {
                        let mut cur = actual;
                        for _ in 0..dims.len().max(1) {
                            match cur {
                                Type::List(e, _) => cur = e,
                                _ => return Ok(()),
                            }
                        }
                        self.infer(&generics[0], cur, gs, binds, mi, span)
                    }
                    (Some("Sözlük"), Type::Dict(k, v)) if path.len() == 1 && generics.len() == 2 => {
                        self.infer(&generics[0], k, gs, binds, mi, span)?;
                        self.infer(&generics[1], v, gs, binds, mi, span)
                    }
                    _ => Ok(()),
                }
            }
            TypeKind::Func { params, ret, .. } => {
                let Type::Func(ps, r) = actual else { return Ok(()) };
                for (p, a) in params.iter().zip(ps) {
                    self.infer(p, a, gs, binds, mi, span)?;
                }
                match ret {
                    Some(rt) => self.infer(rt, r, gs, binds, mi, span),
                    None => Ok(()),
                }
            }
            TypeKind::Union(parts) => {
                // `T veya Hiç` ile `Yazı veya Hiç` -> T = Yazı
                let (genel, conc): (Vec<&TypeExpr>, Vec<&TypeExpr>) = parts.iter().partition(|p| has_param(p, gs));
                if genel.len() != 1 {
                    return Ok(());
                }
                let mut known = Vec::new();
                for c in conc {
                    known.push(self.with_subst(&HashMap::new(), |ch| ch.resolve_type(c, mi))?);
                }
                let rest: Vec<Type> = actual.members().into_iter().filter(|m| !known.contains(m)).collect();
                if rest.is_empty() {
                    return Ok(());
                }
                self.infer(genel[0], &make_union(rest), gs, binds, mi, span)
            }
        }
    }
}
