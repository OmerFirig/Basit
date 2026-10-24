// Varsayılan değerler: `fonksiyon selam(ad: Yazı = "dünya")`, `tür Kişi(ad: Yazı, yaş: Tam = 0)` ve
// türün gövdesindeki alanlar (`tür Sayaç(ad: Yazı) { ata değer = 0 }`).
//
// Değer her kullanımda (değer verilmeyen her çağrıda, her yeni nesnede) yeniden hesaplanır. Tanımın olduğu
// dosyada ve hiçbir değişken görünmeden denetlenir: çağıranın değişkenleri karışmasın, aynı varsayılan
// her yerde aynı anlamı taşısın. Sayılar, yazılar, listeler, fonksiyon çağrıları ve seçenekler yazılabilir.

use crate::ast::*;
use crate::check::*;
use crate::diag::{err, CResult, Span};
use crate::tir::*;
use crate::types::*;
use std::collections::HashMap;

impl Checker {
    /// Varsayılan değeri olan parametreden sonra varsayılanı olmayan gelemez (sıralı verilen değerler karışmasın).
    pub(crate) fn varsayilan_sirasi(&self, params: &[Param]) -> CResult<()> {
        let mut onceki: Option<&Param> = None;
        for p in params {
            if p.varsayilan.is_some() {
                onceki = Some(p);
            } else if let Some(o) = onceki {
                return err(
                    p.span,
                    format!("'{}' varsayılan değerli '{}' parametresinden sonra geliyor; varsayılan değerli parametreler sona yazılır.", p.name, o.name),
                );
            }
        }
        Ok(())
    }

    /// Varsayılan değeri tanımın olduğu dosyada, değişkenler görünmeden denetler. `ty` yoksa tipi çıkarılır.
    /// `binds`: generic kopyada tip parametrelerinin karşılıkları.
    pub(crate) fn varsayilan_te(&mut self, mi: usize, e: &Expr, ty: Option<&Type>, binds: Option<&[(String, Type)]>) -> CResult<TExpr> {
        let eski_cx = std::mem::replace(&mut self.cx, Ctx { module: mi, scopes: vec![HashMap::new()], varsayilan: true, ..Default::default() });
        let eski_lambda = std::mem::take(&mut self.lambda_stack);
        let eski_tip = binds.map(|b| std::mem::replace(&mut self.tsubst, b.iter().cloned().collect()));
        let r = match ty {
            Some(t) => self.expr(e, Some(t)).and_then(|x| self.coerce(x, t, e.span)),
            None => self.expr(e, None),
        };
        self.cx = eski_cx;
        self.lambda_stack = eski_lambda;
        if let Some(t) = eski_tip {
            self.tsubst = t;
        }
        r
    }

    /// Kopyanın tip parametreleri (generic fonksiyon ya da generic türün metodu)
    fn kopya_tipleri(&self, fid: FuncId) -> Option<Vec<(String, Type)>> {
        self.funcs[fid].inst.as_ref().map(|i| i.binds.clone())
    }

    /// Fonksiyonun i. parametresinin varsayılan değeri
    pub(crate) fn fonk_varsayilan(&mut self, fid: FuncId, i: usize) -> CResult<TExpr> {
        let e = self.funcs[fid].varsayilan[i].clone().expect("varsayılan");
        let ty = self.funcs[fid].params[i].1.clone();
        let mi = self.funcs[fid].module;
        let b = self.kopya_tipleri(fid);
        self.varsayilan_te(mi, &e, Some(&ty), b.as_deref())
    }

    /// Alanın başlangıç değeri (başlıkta `= ...` ya da gövdede `ata`)
    pub(crate) fn alan_varsayilan(&mut self, f: &FieldInfo) -> CResult<TExpr> {
        let e = self.alan_varsayilan.get(&(f.origin, f.name.to_string())).cloned().expect("alan varsayılanı");
        let mi = self.type_decls[f.origin].0;
        let b: Option<Vec<(String, Type)>> = self.tur_inst.get(&f.origin).map(|(k, ts, _)| self.tur_kalip[k].0.iter().cloned().zip(ts.iter().cloned()).collect());
        self.varsayilan_te(mi, &e, Some(&f.ty), b.as_deref())
    }

    pub(crate) fn alan_varsayilani_var(&self, f: &FieldInfo) -> bool {
        self.alan_varsayilan.contains_key(&(f.origin, f.name.to_string()))
    }

    /// Argümanları bağlar; verilmeyen ve varsayılanı olan parametre None kalır.
    pub(crate) fn bind_args_vars(
        &mut self,
        params: &[(String, Type)],
        varsayilanli: &[bool],
        args: &[Arg],
        span: Span,
        what: &str,
    ) -> CResult<Vec<Option<TExpr>>> {
        let names: Vec<String> = params.iter().map(|p| p.0.clone()).collect();
        let idx = self.arg_slots(&names, args, span, what)?;
        let mut slots: Vec<Option<TExpr>> = vec![None; params.len()];
        for (a, &i) in args.iter().zip(&idx) {
            let pty = params[i].1.clone();
            let t = self.expr(&a.value, Some(&pty))?;
            slots[i] = Some(self.coerce(t, &pty, a.value.span)?);
        }
        for (i, s) in slots.iter().enumerate() {
            if s.is_none() && !varsayilanli[i] {
                return err(span, format!("'{what}' için '{}' değeri eksik.", params[i].0));
            }
        }
        Ok(slots)
    }

    /// Fonksiyon çağrısının argümanları; verilmeyenlere varsayılan değer konur.
    pub(crate) fn fonk_argumanlari(&mut self, fid: FuncId, args: &[Arg], span: Span, what: &str) -> CResult<Vec<TExpr>> {
        let params = self.funcs[fid].params.clone();
        let vars: Vec<bool> = (0..params.len()).map(|i| self.funcs[fid].varsayilan.get(i).is_some_and(Option::is_some)).collect();
        let slots = self.bind_args_vars(&params, &vars, args, span, what)?;
        let mut out = Vec::new();
        for (i, s) in slots.into_iter().enumerate() {
            out.push(match s {
                Some(t) => t,
                None => self.fonk_varsayilan(fid, i)?,
            });
        }
        Ok(out)
    }

    /// Nesne oluşturma: başlıktaki alanlar verilir (ya da varsayılanları kullanılır), gövdedeki alanlar
    /// başlangıç değerleriyle dolar. `verilen`: oluştururken verilebilen alanlar sırasıyla.
    pub(crate) fn nesne(&mut self, tid: TypeId, verilen: Vec<Option<TExpr>>, span: Span) -> CResult<TExpr> {
        let fields = self.fields_of(tid).to_vec();
        let mut verilen = verilen.into_iter();
        let mut vals = Vec::new();
        for f in &fields {
            let v = if f.kurucuda { verilen.next().expect("alan") } else { None };
            vals.push(match v {
                Some(t) => t,
                None => self.alan_varsayilan(f)?,
            });
        }
        Ok(self.mk(TK::New(tid, vals), Type::Named(tid), span))
    }

    /// Generic olmayan (ya da tipleri belli) türden nesne: `Kişi("Ali")`
    pub(crate) fn kur(&mut self, tid: TypeId, args: &[Arg], span: Span, tname: &str) -> CResult<TExpr> {
        let fields = self.fields_of(tid).to_vec();
        for a in args {
            if let Some((n, s)) = &a.name {
                if fields.iter().any(|f| !f.kurucuda && *f.name == **n) {
                    return err(*s, format!("'{n}' türün içinde başlangıç değeriyle tanımlı bir alan; oluştururken verilemez. Nesne oluştuktan sonra değiştirin: nesne.{n} = ..."));
                }
            }
        }
        let kurucu: Vec<&FieldInfo> = fields.iter().filter(|f| f.kurucuda).collect();
        let params: Vec<(String, Type)> = kurucu.iter().map(|f| (f.name.to_string(), f.ty.clone())).collect();
        let vars: Vec<bool> = kurucu.iter().map(|f| self.alan_varsayilani_var(f)).collect();
        let verilen = self.bind_args_vars(&params, &vars, args, span, tname)?;
        self.nesne(tid, verilen, span)
    }

    /// Gövdedeki `ata` alanının tipi: yazılmışsa o, yazılmamışsa basit bir değerden (sayı, yazı, liste...) çıkarılır.
    pub(crate) fn govde_alan_tipi(&mut self, mi: usize, ty: Option<&TypeExpr>, deger: &Expr, ad: &str) -> CResult<Type> {
        if let Some(te) = ty {
            return self.resolve_type(te, mi);
        }
        if !basit_deger(deger) {
            return err(deger.span, format!("Bu alanın tipi değerden çıkarılamıyor; tipini yazın: ata {ad}: Tam = ..."));
        }
        let b: Vec<(String, Type)> = self.tsubst.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        Ok(self.varsayilan_te(mi, deger, None, Some(&b))?.ty)
    }
}

/// Tipi kendinden belli değer: sayı, yazı, mantık ve bunlardan oluşan liste/sözlük
fn basit_deger(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Str(_) | ExprKind::FStr(_) | ExprKind::Bool(_) => true,
        ExprKind::Neg(x) => basit_deger(x),
        ExprKind::List(v) => v.iter().all(basit_deger),
        ExprKind::Dict(v) => v.iter().all(|(a, b)| basit_deger(a) && basit_deger(b)),
        ExprKind::Olarak(..) => true,
        _ => false,
    }
}
