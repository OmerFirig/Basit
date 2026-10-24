// İfadeler: sabitler, aritmetik, karşılaştırma, dönüştürmeler, çağrılar, nesneler.

use super::duzen::{int_parts, is_esnek, width};
use super::Gen;
use crate::ast::BinOp;
use crate::diag::CResult;
use crate::tir::*;
use crate::types::{bounds_text, group_digits, int_bounds, int_subset, Sign, Type, TypeId};
use crate::values::type_display;
use num_bigint::BigInt;
use num_traits::{One, Signed, ToPrimitive};

/// N bitlik tam sayının LLVM gösterimi (ikiye tümleyen).
pub(super) fn int_const(n: &BigInt, width: u32) -> String {
    let m = BigInt::one() << width;
    let mut v = n % &m;
    if v.is_negative() {
        v += &m;
    }
    if v >= (BigInt::one() << (width - 1)) {
        v -= &m;
    }
    v.to_string()
}

pub(super) fn float_const(x: f64, bits: u32) -> String {
    match bits {
        16 => format!("0xH{:04X}", half::f16::from_f64(x).to_bits()),
        32 => format!("0x{:016X}", ((x as f32) as f64).to_bits()),
        _ => format!("0x{:016X}", x.to_bits()),
    }
}

const ESNEK_UST: i64 = (1 << 62) - 1;
const ESNEK_ALT: i64 = -(1 << 62);

/// Değerin hangi 'veya' üyesine dönüşebileceği: her zaman, çalışma anında bakılarak, hiç
#[derive(PartialEq)]
enum Uyum {
    Her,
    Bak,
    Yok,
}

impl<'p> Gen<'p> {
    pub(super) fn expr(&mut self, e: &TExpr) -> CResult<String> {
        let loc = e.loc;
        Ok(match &e.kind {
            TK::Int(n, small) => {
                if is_esnek(&e.ty) {
                    self.esnek_const(n)
                } else {
                    let _ = small;
                    int_const(n, width(&e.ty))
                }
            }
            TK::Float(x) => {
                let Type::Float(b) = e.ty else { unreachable!() };
                float_const(*x, b)
            }
            TK::Bool(b) => b.to_string(),
            TK::Str(s) => self.str_const(s),
            TK::Hic => self.str_const("Hiç"),
            TK::Var(slot) => {
                let v = self.load_slot(*slot, &e.ty, loc);
                if self.is_rc(&e.ty) && matches!(slot, Slot::Global(_)) {
                    // aynı ifadedeki bir fonksiyon dosya değişkenini değiştirebilir: tut
                    self.retain(&v, &e.ty);
                    self.own(v, &e.ty)
                } else {
                    v
                }
            }
            TK::Bu => {
                let bu = self.f.bu.clone().expect("bu");
                let v = self.tmp();
                self.emit(&format!("{v} = load ptr, ptr {bu}"));
                self.retain(&v, &e.ty);
                self.own(v, &e.ty)
            }
            TK::FStr(parts) => {
                let acc0 = self.tmp();
                self.emit(&format!("{acc0} = call ptr @kolay_bos()"));
                let mut acc = acc0;
                for p in parts {
                    let (piece, src) = match p {
                        TFPart::Lit(s) => (self.str_const(s), None),
                        TFPart::Expr(x) => {
                            let v = self.expr(x)?;
                            (self.to_str(&v, &x.ty)?, Some(v))
                        }
                    };
                    let t = self.tmp();
                    self.emit(&format!("{t} = call ptr @kolay_ekle(ptr {acc}, ptr {piece})"));
                    self.drop_temp(&piece);
                    if let Some(v) = src {
                        self.drop_temp(&v);
                    }
                    acc = t;
                }
                self.own(acc, &Type::Str)
            }
            TK::List(items) => {
                let Type::List(et, _) = &e.ty else { unreachable!() };
                let et = (**et).clone();
                let tip = self.tip(&e.ty);
                let l = self.tmp();
                self.emit(&format!("{l} = call ptr @kolay_liste_yeni(ptr {tip}, i64 {})", items.len()));
                if !items.is_empty() {
                    let st = self.stor(&et);
                    let dp = self.tmp();
                    self.emit(&format!("{dp} = getelementptr i8, ptr {l}, i64 32"));
                    for (i, it) in items.iter().enumerate() {
                        let v = self.expr(it)?;
                        self.keep_ref(&v, &et);
                        let ep = self.tmp();
                        self.emit(&format!("{ep} = getelementptr {}, ptr {dp}, i64 {i}", st.ll));
                        self.store_mem(&ep, &v, &et);
                    }
                }
                self.own(l, &e.ty)
            }
            TK::Dict(items) => {
                let Type::Dict(kt, vt) = &e.ty else { unreachable!() };
                let (kt, vt) = ((**kt).clone(), (**vt).clone());
                let tip = self.tip(&e.ty);
                let d = self.tmp();
                self.emit(&format!("{d} = call ptr @kolay_sozluk_yeni(ptr {tip})"));
                for (k, v) in items {
                    let kv = self.expr(k)?;
                    let vv = self.expr(v)?;
                    let kp = self.spill(&kv, &kt);
                    let vp = self.tmp();
                    self.emit(&format!("{vp} = call ptr @kolay_sozluk_yer(ptr {d}, ptr {kp})"));
                    self.store_at(&vp, &vv, &vt, false);
                    self.drop_temp(&kv);
                }
                self.own(d, &e.ty)
            }
            TK::Default => {
                let tip = self.tip(&e.ty);
                let st = self.stor(&e.ty);
                let a = self.alloca(&st.ll);
                self.emit(&format!("call void @kolay_varsayilan(ptr {a}, ptr {tip})"));
                let v = self.load_mem(&a, &e.ty);
                self.own(v, &e.ty)
            }
            TK::Range { start, end, inclusive, step } => {
                // değer olarak kullanılan aralık listesi (döngüde liste oluşturulmaz, bkz. komut.rs)
                let s = self.expr(start)?;
                let en = self.expr(end)?;
                let k = match step {
                    Some(x) => self.expr(x)?,
                    None => "1".into(),
                };
                let tip = self.tip(&e.ty);
                let (kn, kd) = self.where_(loc);
                let t = self.tmp();
                self.emit(&format!("{t} = call ptr @kolay_aralik_liste(i64 {s}, i64 {en}, i1 zeroext {inclusive}, i64 {k}, ptr {tip}, ptr {kn}, ptr {kd})"));
                self.own(t, &e.ty)
            }
            TK::Neg(x) => {
                let v = self.expr(x)?;
                match &e.ty {
                    Type::Float(_) => {
                        let lt = self.llty(&e.ty);
                        let t = self.tmp();
                        self.emit(&format!("{t} = fneg {lt} {v}"));
                        t
                    }
                    t if is_esnek(t) => {
                        let isaret = sign_code(t);
                        let tn = self.cstr(&type_display(t, self.prog));
                        let (k, kd) = self.where_(loc);
                        let r = self.tmp();
                        self.emit(&format!("{r} = call i64 @kolay_esnek_neg(i64 {v}, i64 {isaret}, ptr {tn}, ptr {k}, ptr {kd})"));
                        self.drop_temp(&v);
                        self.own(r, t)
                    }
                    _ => self.arith_neg(&v, &e.ty, loc)?,
                }
            }
            TK::Not(x) => {
                let v = self.expr(x)?;
                let t = self.tmp();
                self.emit(&format!("{t} = xor i1 {v}, true"));
                t
            }
            TK::Arith(op, l, r) => {
                let a = self.expr(l)?;
                let b = self.expr(r)?;
                let res = self.arith(*op, &a, &b, &e.ty, loc)?;
                self.drop_temp(&a);
                self.drop_temp(&b);
                res
            }
            TK::Concat(l, r) => {
                let a = self.expr(l)?;
                let b = self.expr(r)?;
                let t = self.tmp();
                if self.disown(&a) {
                    self.emit(&format!("{t} = call ptr @kolay_ekle(ptr {a}, ptr {b})"));
                } else {
                    self.emit(&format!("{t} = call ptr @kolay_birlestir(ptr {a}, ptr {b})"));
                }
                self.drop_temp(&b);
                self.own(t, &Type::Str)
            }
            TK::Cmp(op, l, r) => {
                let a = self.expr(l)?;
                let b = self.expr(r)?;
                let t = self.compare(*op, &a, &b, &l.ty)?;
                self.drop_temp(&a);
                self.drop_temp(&b);
                t
            }
            TK::And(l, r) | TK::Or(l, r) => {
                let is_and = matches!(e.kind, TK::And(..));
                let res = self.alloca("i1");
                let a = self.expr(l)?;
                self.emit(&format!("store i1 {a}, ptr {res}"));
                let (rhs, end) = (self.new_label(), self.new_label());
                if is_and {
                    self.term(&format!("br i1 {a}, label %{rhs}, label %{end}"));
                } else {
                    self.term(&format!("br i1 {a}, label %{end}, label %{rhs}"));
                }
                self.block(&rhs);
                let b = self.expr(r)?;
                self.emit(&format!("store i1 {b}, ptr {res}"));
                self.block(&end);
                let t = self.tmp();
                self.emit(&format!("{t} = load i1, ptr {res}"));
                t
            }
            TK::Convert(x) => {
                let v = self.expr(x)?;
                self.convert(&v, &x.ty, &e.ty, loc)?
            }
            TK::Olarak(x) => {
                let v = self.expr(x)?;
                self.olarak(&v, &x.ty, &e.ty, loc, false)?
            }
            TK::Call(fid, args) => {
                let func = &self.prog.funcs[*fid];
                let (params, ret) = (func.params.clone(), func.ret.clone());
                let (vals, temps) = self.call_args(args, &params)?;
                let r = self.emit_call(*fid, None, &vals, &ret);
                for v in temps {
                    self.drop_temp(&v);
                }
                if self.is_rc(&ret) { self.own(r, &ret) } else { r }
            }
            TK::Method { recv, view, mid, args, .. } => {
                let cands = self.dispatch.get(&(*view, *mid)).cloned().unwrap_or_default();
                let (params, ret) = {
                    let f = &self.prog.funcs[cands[0].1];
                    (f.params.clone(), f.ret.clone())
                };
                let (vals, temps) = self.call_args(args, &params)?;
                let (bu, after) = self.receiver(recv, loc)?;
                let r = if cands.iter().all(|c| c.1 == cands[0].1) {
                    self.emit_call(cands[0].1, Some(&bu), &vals, &ret)
                } else {
                    self.dispatch_call(&cands, &bu, &vals, &ret)
                };
                self.after_receiver(after, &bu, loc)?;
                for v in temps {
                    self.drop_temp(&v);
                }
                if self.is_rc(&ret) { self.own(r, &ret) } else { r }
            }
            TK::Direct { func, recv, args } => {
                let f = &self.prog.funcs[*func];
                let (params, ret) = (f.params.clone(), f.ret.clone());
                let (vals, temps) = self.call_args(args, &params)?;
                let (bu, after) = self.receiver(recv, loc)?;
                let r = self.emit_call(*func, Some(&bu), &vals, &ret);
                self.after_receiver(after, &bu, loc)?;
                for v in temps {
                    self.drop_temp(&v);
                }
                if self.is_rc(&ret) { self.own(r, &ret) } else { r }
            }
            TK::Builtin(Builtin::Yazdir, args) => {
                // önce bütün değerler hesaplanır (biri hata verirse satırın yarısı yazılmasın)
                let mut parts = Vec::new();
                for a in args {
                    let v = self.expr(a)?;
                    let s = self.to_str(&v, &a.ty)?;
                    parts.push((v, s));
                }
                for (i, (v, s)) in parts.iter().enumerate() {
                    if i > 0 {
                        self.emit("call void @kolay_yaz_bosluk()");
                    }
                    self.emit(&format!("call void @kolay_yaz(ptr {s})"));
                    self.drop_temp(s);
                    self.drop_temp(v);
                }
                self.emit("call void @kolay_satir()");
                String::new()
            }
            TK::Builtin(Builtin::Oku, args) => {
                let p = match args.first() {
                    Some(a) => self.expr(a)?,
                    None => "null".into(),
                };
                let t = self.tmp();
                self.emit(&format!("{t} = call ptr @kolay_oku(ptr {p})"));
                self.drop_temp(&p);
                self.own(t, &Type::Str)
            }
            TK::BMethod { recv, m, args } => self.bmethod(*m, recv, args, &e.ty, loc)?,
            TK::New(tid, args) => {
                let size = self.object_sizes[*tid];
                let TypeDefKind::Tur { fields, .. } = &self.prog.types[*tid].kind else { unreachable!() };
                let ftys: Vec<Type> = fields.iter().map(|f| f.ty.clone()).collect();
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.expr(a)?);
                }
                let o = self.tmp();
                self.emit(&format!("{o} = call ptr @kolay_nesne_yeni(ptr @tur.{tid}, i64 {size})"));
                for (i, v) in vals.iter().enumerate() {
                    let off = self.field_offsets[*tid][i];
                    let fp = self.tmp();
                    self.emit(&format!("{fp} = getelementptr i8, ptr {o}, i64 {off}"));
                    self.keep_ref(v, &ftys[i]);
                    self.store_mem(&fp, v, &ftys[i]);
                }
                self.own(o, &e.ty)
            }
            TK::Variant(tid, idx, args) => {
                if args.is_empty() {
                    // değer taşımayan seçenek: hazır sabit nesne
                    format!("@sabit.{tid}.{idx}")
                } else {
                    let TypeDefKind::Secenek { variants } = &self.prog.types[*tid].kind else { unreachable!() };
                    let ftys: Vec<Type> = variants[*idx as usize].fields.iter().map(|f| f.1.clone()).collect();
                    let mut vals = Vec::new();
                    for a in args {
                        vals.push(self.expr(a)?);
                    }
                    let size = self.variant_sizes[*tid][*idx as usize];
                    let o = self.tmp();
                    self.emit(&format!("{o} = call ptr @kolay_nesne_yeni(ptr @tur.{tid}, i64 {size})"));
                    let tp = self.tmp();
                    self.emit(&format!("{tp} = getelementptr i8, ptr {o}, i64 16"));
                    self.emit(&format!("store i64 {idx}, ptr {tp}"));
                    for (i, v) in vals.iter().enumerate() {
                        let off = self.variant_offsets[*tid][*idx as usize][i];
                        let fp = self.tmp();
                        self.emit(&format!("{fp} = getelementptr i8, ptr {o}, i64 {off}"));
                        self.keep_ref(v, &ftys[i]);
                        self.store_mem(&fp, v, &ftys[i]);
                    }
                    self.own(o, &e.ty)
                }
            }
            TK::Field(x, fid) => {
                let Type::Named(s) = x.ty else { unreachable!() };
                // nesne, alan okunana kadar başka kod çalışmadığı için ödünç alınabilir
                let (obj, temp) = match &x.kind {
                    TK::Bu => {
                        let bu = self.f.bu.clone().expect("bu");
                        let o = self.tmp();
                        self.emit(&format!("{o} = load ptr, ptr {bu}"));
                        (o, None)
                    }
                    TK::Var(Slot::Local(_)) => (self.expr(x)?, None),
                    _ => {
                        let o = self.expr(x)?;
                        (o.clone(), Some(o))
                    }
                };
                let fp = self.field_ptr(&obj, s, *fid);
                let ft = self.field_type(s, *fid);
                let v = self.load_mem(&fp, &ft);
                if self.is_rc(&ft) {
                    self.retain(&v, &ft);
                    self.own(v.clone(), &ft);
                }
                if let Some(t) = temp {
                    self.drop_temp(&t);
                }
                v
            }
            TK::Index(x, i) => {
                // Dosya değişkenindeki liste/sözlük: indeks yan etkisizse önce indeks hesaplanır, sonra
                // liste ödünç okunur (tutup bırakmaya gerek kalmaz).
                let c = match &x.kind {
                    TK::Var(Slot::Global(g)) if is_pure(i) && matches!(x.ty, Type::List(..)) => {
                        let et = match &x.ty {
                            Type::List(et, _) => (**et).clone(),
                            _ => unreachable!(),
                        };
                        let iv0 = self.expr(i)?;
                        let iv = self.to_index(&iv0, &i.ty, i.loc);
                        self.drop_temp(&iv0);
                        let l = self.load_slot(Slot::Global(*g), &x.ty, loc);
                        let ep = self.elem_ptr(&l, &iv, &et, loc);
                        let v = self.load_mem(&ep, &et);
                        if self.is_rc(&et) {
                            self.retain(&v, &et);
                            self.own(v.clone(), &et);
                        }
                        return Ok(v);
                    }
                    _ => self.expr(x)?,
                };
                match &x.ty {
                    Type::List(et, _) => {
                        let et = (**et).clone();
                        let iv0 = self.expr(i)?;
                        let iv = self.to_index(&iv0, &i.ty, i.loc);
                        self.drop_temp(&iv0);
                        let ep = self.elem_ptr(&c, &iv, &et, loc);
                        let v = self.load_mem(&ep, &et);
                        if self.is_rc(&et) {
                            self.retain(&v, &et);
                            self.own(v.clone(), &et);
                        }
                        self.drop_temp(&c);
                        v
                    }
                    Type::Dict(kt, vt) => {
                        let (kt, vt) = ((**kt).clone(), (**vt).clone());
                        let kv = self.expr(i)?;
                        let kp = self.spill(&kv, &kt);
                        let (k, kd) = self.where_(loc);
                        let vp = self.tmp();
                        self.emit(&format!("{vp} = call ptr @kolay_sozluk_al(ptr {c}, ptr {kp}, ptr {k}, ptr {kd})"));
                        let v = self.load_mem(&vp, &vt);
                        if self.is_rc(&vt) {
                            self.retain(&v, &vt);
                            self.own(v.clone(), &vt);
                        }
                        self.drop_temp(&kv);
                        self.drop_temp(&c);
                        v
                    }
                    _ => unreachable!(),
                }
            }
            TK::Slice { base, start, end, inclusive } => {
                let l = self.expr(base)?;
                let s = match start {
                    Some(x) => self.expr(x)?,
                    None => "0".into(),
                };
                let en = match end {
                    Some(x) => self.expr(x)?,
                    None => "0".into(),
                };
                // bayrak: 1 başlangıç var, 2 bitiş var, 4 bitiş dahil
                let fl = start.is_some() as i64 | (end.is_some() as i64) << 1 | (*inclusive as i64) << 2;
                let t = self.tmp();
                if e.ty == Type::Str {
                    self.emit(&format!("{t} = call ptr @kolay_yazi_dilim(ptr {l}, i64 {s}, i64 {en}, i64 {fl})"));
                } else {
                    let tip = self.tip(&e.ty);
                    self.emit(&format!("{t} = call ptr @kolay_liste_dilim(ptr {l}, ptr {tip}, i64 {s}, i64 {en}, i64 {fl})"));
                }
                self.drop_temp(&l);
                self.own(t, &e.ty)
            }
            TK::Lambda { func, caps, bu } => {
                // nesne: başlık, fonksiyon adresi, yakalanan değerlerin kopyaları
                let (offs, bu_off, size) = self.lambda_layout(*func);
                let tip = self.lambda_tip(*func, &e.ty);
                let mut vals = Vec::new();
                for c in caps {
                    vals.push((self.expr(c)?, c.ty.clone()));
                }
                let buv = match bu {
                    Some(b) => Some((self.expr(b)?, b.ty.clone())),
                    None => None,
                };
                let o = self.tmp();
                self.emit(&format!("{o} = call ptr @kolay_nesne_yeni(ptr {tip}, i64 {size})"));
                let fp = self.tmp();
                self.emit(&format!("{fp} = getelementptr i8, ptr {o}, i64 16"));
                self.emit(&format!("store ptr @f{func}, ptr {fp}"));
                let all: Vec<(i64, (String, Type))> = offs.into_iter().zip(vals).chain(bu_off.zip(buv)).collect();
                for (off, (v, t)) in all {
                    let p = self.tmp();
                    self.emit(&format!("{p} = getelementptr i8, ptr {o}, i64 {off}"));
                    self.keep_ref(&v, &t);
                    self.store_mem(&p, &v, &t);
                }
                self.own(o, &e.ty)
            }
            TK::FuncRef(fid) => self.func_ref(*fid, &e.ty),
            TK::Unwrap(x) => self.unwrap(x, &e.ty, loc)?,
            TK::CallValue(fe, args) => {
                let Type::Func(ps, ret) = &fe.ty else { unreachable!() };
                let (ps, ret) = (ps.clone(), (**ret).clone());
                let fv = self.expr(fe)?;
                let (vals, temps) = self.call_args(args, &ps)?;
                let (fpp, fp) = (self.tmp(), self.tmp());
                self.emit(&format!("{fpp} = getelementptr i8, ptr {fv}, i64 16"));
                self.emit(&format!("{fp} = load ptr, ptr {fpp}"));
                let mut all = vec![format!("ptr {fv}")];
                all.extend(vals);
                let r = if ret == Type::Hic {
                    self.emit(&format!("call void {fp}({})", all.join(", ")));
                    String::new()
                } else {
                    let lt = self.llty(&ret);
                    let t = self.tmp();
                    self.emit(&format!("{t} = call {lt} {fp}({})", all.join(", ")));
                    t
                };
                for v in temps {
                    self.drop_temp(&v);
                }
                self.drop_temp(&fv);
                if self.is_rc(&ret) { self.own(r, &ret) } else { r }
            }
            TK::Match(m) => {
                if e.ty == Type::Never || e.ty == Type::Hic {
                    self.match_code(m, None)?;
                    return Ok(String::new());
                }
                let lt = self.llty(&e.ty);
                let res = self.alloca(&lt);
                self.match_code(m, Some((&res, &e.ty)))?;
                let t = self.tmp();
                self.emit(&format!("{t} = load {lt}, ptr {res}"));
                self.own(t, &e.ty)
            }
        })
    }

    /// `dene` içinde `T veya Hata` değeri: Hata üyesiyse fırlatılır, değilse T'ye çevrilir. Sonuç sahiplidir.
    fn unwrap(&mut self, x: &TExpr, rest: &Type, loc: Loc) -> CResult<String> {
        let v = self.expr(x)?;
        let Type::Union(ms) = &x.ty else { unreachable!("'veya' bekleniyordu") };
        let sahipli = self.f.owned.contains(&v);
        let lt = self.llty(rest);
        let res = if lt == "void" { None } else { Some(self.alloca(&lt)) };
        let tag = self.union_tag(&v);
        let (end, def) = (self.new_label(), self.new_label());
        let labels: Vec<String> = ms.iter().map(|_| self.new_label()).collect();
        let cases: Vec<String> = labels.iter().enumerate().map(|(i, l)| format!("i64 {i}, label %{l}")).collect();
        self.term(&format!("switch i64 {tag}, label %{def} [ {} ]", cases.join(" ")));
        for (m, l) in ms.iter().zip(&labels) {
            self.block(l);
            let mv = self.union_unpack(&v, m);
            if self.prog.is_hata(m) {
                // değerin sahipliği fırlatmaya geçer
                if !sahipli {
                    self.retain(&mv, m);
                }
                let p = self.spill(&mv, m);
                let t = self.tip(m);
                let (k, kd) = self.where_(loc);
                self.emit(&format!("call void @kolay_firlat(ptr {t}, ptr {p}, ptr {k}, ptr {kd})"));
                self.term("unreachable");
                continue;
            }
            let c = self.convert(&mv, m, rest, loc)?;
            self.ensure_owned(&c, rest);
            self.disown(&c);
            if let Some(r) = &res {
                self.emit(&format!("store {lt} {c}, ptr {r}"));
            }
            self.term(&format!("br label %{end}"));
        }
        self.block(&def);
        self.term("unreachable");
        self.block(&end);
        self.drop_temp(&v);
        let Some(r) = res else { return Ok(String::new()) };
        let t = self.tmp();
        self.emit(&format!("{t} = load {lt}, ptr {r}"));
        Ok(self.own(t, rest))
    }

    // ------------------------------------------------------------ çağrılar

    /// Argümanları hesaplar: (LLVM argümanları, sonra bırakılacak geçiciler)
    fn call_args(&mut self, args: &[TExpr], params: &[Type]) -> CResult<(Vec<String>, Vec<String>)> {
        let mut vals = Vec::new();
        let mut temps = Vec::new();
        for (a, t) in args.iter().zip(params) {
            let v = self.expr(a)?;
            vals.push(format!("{} {v}", self.llty(t)));
            temps.push(v);
        }
        Ok((vals, temps))
    }

    fn emit_call(&mut self, fid: FuncId, bu: Option<&str>, vals: &[String], ret: &Type) -> String {
        let mut all = Vec::new();
        if let Some(b) = bu {
            all.push(format!("ptr {b}"));
        }
        all.extend(vals.iter().cloned());
        // notail: kendini çağıran fonksiyonlar döngüye çevrilmesin; sonsuz özyineleme
        // yorumlayıcıdaki gibi "çok derin özyineleme" hatasıyla dursun.
        if *ret == Type::Hic {
            self.emit(&format!("notail call void @f{fid}({})", all.join(", ")));
            String::new()
        } else {
            let lt = self.llty(ret);
            let t = self.tmp();
            self.emit(&format!("{t} = notail call {lt} @f{fid}({})", all.join(", ")));
            t
        }
    }

    /// Birden fazla türün metodu olabilir: nesnenin gerçek türüne göre seçilir (switch).
    fn dispatch_call(&mut self, cands: &[(TypeId, FuncId)], bu: &str, vals: &[String], ret: &Type) -> String {
        let obj = self.tmp();
        self.emit(&format!("{obj} = load ptr, ptr {bu}"));
        let no = self.dyn_type_no(&obj);
        let lt = self.llty(ret);
        let res = if *ret == Type::Hic { None } else { Some(self.alloca(&lt)) };
        let end = self.new_label();
        let def = self.new_label();
        let mut labels = Vec::new();
        for _ in cands {
            labels.push(self.new_label());
        }
        let cases: Vec<String> = cands.iter().zip(&labels).map(|((t, _), l)| format!("i64 {t}, label %{l}")).collect();
        self.term(&format!("switch i64 {no}, label %{def} [ {} ]", cases.join(" ")));
        for ((_, f), l) in cands.iter().zip(&labels) {
            self.block(l);
            let r = self.emit_call(*f, Some(bu), vals, ret);
            if let Some(res) = &res {
                self.emit(&format!("store {lt} {r}, ptr {res}"));
            }
            self.term(&format!("br label %{end}"));
        }
        self.block(&def);
        self.term("unreachable");
        self.block(&end);
        match res {
            Some(res) => {
                let t = self.tmp();
                self.emit(&format!("{t} = load {lt}, ptr {res}"));
                t
            }
            None => String::new(),
        }
    }

    /// Metodun çalışacağı nesnenin yeri. Nesne her zaman yerinde değiştirilir; metot sürerken
    /// başkaları güncel hali görür (yorumlayıcıdaki gibi). Liste elemanı / alan gibi yerlerde yolun
    /// ilk nesnesi çağrı boyunca tutulur: biri o listeyi değiştirirse kopyalanır (yazınca kopyala),
    /// böylece elemanın adresi geçerli kalır.
    fn receiver(&mut self, recv: &Recv, loc: Loc) -> CResult<(String, Option<Option<String>>)> {
        Ok(match recv {
            Recv::Place(TPlace::Var(s)) => {
                if let Slot::Global(g) = s {
                    self.global_check(*g, loc);
                }
                (self.slot_ptr(*s), None)
            }
            Recv::Place(TPlace::Bu) => (self.f.bu.clone().expect("bu"), None),
            Recv::Place(p) => {
                let (addr, _, _, first) = self.place_mut_addr_pin(p, false, loc)?;
                let pin = first.expect("yol");
                self.emit(&format!("call void @kolay_tut(ptr {pin})"));
                (addr, Some(Some(pin)))
            }
            Recv::Value(x) => {
                let v = self.expr(x)?;
                self.ensure_owned(&v, &x.ty);
                self.disown(&v);
                let a = self.alloca("ptr");
                self.emit(&format!("store ptr {v}, ptr {a}"));
                (a, Some(None))
            }
        })
    }

    fn after_receiver(&mut self, after: Option<Option<String>>, bu: &str, _loc: Loc) -> CResult<()> {
        match after {
            None => {}
            Some(Some(pin)) => {
                self.emit(&format!("call void @kolay_birak(ptr {pin})"));
            }
            Some(None) => {
                let v = self.tmp();
                self.emit(&format!("{v} = load ptr, ptr {bu}"));
                self.emit(&format!("call void @kolay_birak(ptr {v})"));
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------ sayılar

    /// Esnek Tam sabiti: küçükse etiketli kelime, değilse (sahipli) büyük sayı nesnesi.
    pub(super) fn esnek_const(&mut self, n: &BigInt) -> String {
        if let Some(v) = n.to_i64() {
            if (ESNEK_ALT..=ESNEK_UST).contains(&v) {
                return ((v << 1) | 1).to_string();
            }
        }
        let c = self.cstr(&n.to_string());
        let t = self.tmp();
        self.emit(&format!("{t} = call i64 @kolay_esnek_metinden(ptr {c})"));
        self.own(t, &crate::types::ESNEK)
    }

    /// Tam sayıyı çalışma kütüphanesine 128 bit olarak verir: (değer, biçim).
    /// biçim 0: işaretli, 1: işaretsiz, 2: büyüklük (gerçek değer = -değer; sadece Tam(128, -))
    pub(super) fn int128_form(&mut self, v: &str, ty: &Type) -> (String, String) {
        let (_, s) = int_parts(ty);
        let w = width(ty);
        if w == 128 {
            return (v.to_string(), if s == Sign::Pos { "1" } else { "0" }.into());
        }
        let t = self.tmp();
        if w > 128 {
            let n = self.tmp();
            self.emit(&format!("{n} = sub i{w} 0, {v}"));
            self.emit(&format!("{t} = trunc i{w} {n} to i128"));
            return (t, "2".into());
        }
        let (op, mode) = if s == Sign::Pos { ("zext", "1") } else { ("sext", "0") };
        self.emit(&format!("{t} = {op} i{w} {v} to i128"));
        (t, mode.into())
    }

    pub(super) fn to_double(&mut self, v: &str, bits: u32) -> String {
        if bits == 64 {
            return v.to_string();
        }
        let src = if bits == 32 { "float" } else { "half" };
        let t = self.tmp();
        self.emit(&format!("{t} = fpext {src} {v} to double"));
        t
    }

    fn tasma(&mut self, op: &str, a: &str, b: &str, ty: &Type, loc: Loc) {
        let (bits, s) = int_parts(ty);
        let (am, mode) = self.int128_form(a, ty);
        let (bm, _) = self.int128_form(b, ty);
        let tn = self.cstr(&type_display(ty, self.prog));
        let ops = self.cstr(op);
        let bounds = self.cstr(&bounds_text(Some(bits), s));
        let (lo, hi) = int_bounds(Some(bits), s);
        let lo = self.cstr(&group_digits(&lo.unwrap().to_string()));
        let hi = self.cstr(&group_digits(&hi.unwrap().to_string()));
        let (k, c) = self.where_(loc);
        let pos = (s == Sign::Pos) as i32;
        self.emit(&format!(
            "call void @kolay_tasma(ptr {tn}, ptr {ops}, i128 {am}, i128 {bm}, i32 {mode}, ptr {bounds}, ptr {lo}, ptr {hi}, i32 {pos}, ptr {k}, ptr {c})"
        ));
        self.term("unreachable");
    }

    /// Neg (sadece negatif) tipler için ek sınır kontrolü: -(2^N - 1) <= v <= 0
    fn neg_range_check(&mut self, v: &str, ty: &Type) -> String {
        let (bits, _) = int_parts(ty);
        let w = bits + 1;
        let m: BigInt = (BigInt::one() << bits) - 1;
        let lo = -m;
        let (t1, t2, t3) = (self.tmp(), self.tmp(), self.tmp());
        self.emit(&format!("{t1} = icmp sgt i{w} {v}, 0"));
        self.emit(&format!("{t2} = icmp slt i{w} {v}, {}", int_const(&lo, w)));
        self.emit(&format!("{t3} = or i1 {t1}, {t2}"));
        t3
    }

    fn overflow_branch(&mut self, bad: &str, op: &str, a: &str, b: &str, ty: &Type, loc: Loc) {
        let (badl, ok) = (self.new_label(), self.new_label());
        self.br_cold(bad, &badl, &ok);
        self.block(&badl);
        self.tasma(op, a, b, ty, loc);
        self.block(&ok);
    }

    /// Aritmetik işlem. Esnek Tam sonucu sahiplidir.
    pub(super) fn arith(&mut self, op: BinOp, a: &str, b: &str, ty: &Type, loc: Loc) -> CResult<String> {
        if let Type::Float(bits) = ty {
            let lt = self.llty(ty);
            if op == BinOp::Pow {
                // yorumlayıcıdaki gibi double'da hesaplanıp tipe yuvarlanır
                let (x, y) = (self.to_double(a, *bits), self.to_double(b, *bits));
                self.intrinsic("llvm.pow.f64", "declare double @llvm.pow.f64(double, double)".into());
                let r = self.tmp();
                self.emit(&format!("{r} = call double @llvm.pow.f64(double {x}, double {y})"));
                if *bits == 64 {
                    return Ok(r);
                }
                let t = self.tmp();
                self.emit(&format!("{t} = fptrunc double {r} to {lt}"));
                return Ok(t);
            }
            let ins = match op {
                BinOp::Add => "fadd",
                BinOp::Sub => "fsub",
                BinOp::Mul => "fmul",
                BinOp::Div => "fdiv",
                BinOp::Rem => "frem",
                _ => unreachable!(),
            };
            let t = self.tmp();
            self.emit(&format!("{t} = {ins} {lt} {a}, {b}"));
            if op == BinOp::Div {
                // Sıfıra bölme tanımsızdır: sonuç TanımsızSayı (sonsuz değil).
                let (z, r) = (self.tmp(), self.tmp());
                self.emit(&format!("{z} = fcmp oeq {lt} {b}, 0.0"));
                self.emit(&format!("{r} = select i1 {z}, {lt} {}, {lt} {t}", float_const(f64::NAN, *bits)));
                return Ok(r);
            }
            return Ok(t);
        }
        if is_esnek(ty) {
            return Ok(self.esnek_arith(op, a, b, ty, loc));
        }
        let (_, s) = int_parts(ty);
        let w = width(ty);
        let lt = format!("i{w}");
        let sym = op.symbol();
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul => {
                let base = match op {
                    BinOp::Add => "add",
                    BinOp::Sub => "sub",
                    _ => "mul",
                };
                let pre = if s == Sign::Pos { "u" } else { "s" };
                let name = format!("llvm.{pre}{base}.with.overflow.{lt}");
                self.intrinsic(&name, format!("declare {{{lt}, i1}} @{name}({lt}, {lt})"));
                let r = self.tmp();
                self.emit(&format!("{r} = call {{{lt}, i1}} @{name}({lt} {a}, {lt} {b})"));
                let (v, of) = (self.tmp(), self.tmp());
                self.emit(&format!("{v} = extractvalue {{{lt}, i1}} {r}, 0"));
                self.emit(&format!("{of} = extractvalue {{{lt}, i1}} {r}, 1"));
                let bad = if s == Sign::Neg {
                    let rc = self.neg_range_check(&v, ty);
                    let t = self.tmp();
                    self.emit(&format!("{t} = or i1 {of}, {rc}"));
                    t
                } else {
                    of
                };
                self.overflow_branch(&bad, sym, a, b, ty, loc);
                Ok(v)
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                let ins = match op {
                    BinOp::BitAnd => "and",
                    BinOp::BitOr => "or",
                    _ => "xor",
                };
                let t = self.tmp();
                self.emit(&format!("{t} = {ins} {lt} {a}, {b}"));
                if s == Sign::Neg {
                    // sadece eksi tipte sonuç sınır dışına çıkabilir (-5 ^ -3 = 6)
                    let rc = self.neg_range_check(&t, ty);
                    self.overflow_branch(&rc, sym, a, b, ty, loc);
                }
                Ok(t)
            }
            BinOp::Shl | BinOp::Shr | BinOp::Pow => Ok(self.shift_pow(op, a, b, ty, loc)),
            BinOp::Div | BinOp::Rem => {
                let z = self.tmp();
                self.emit(&format!("{z} = icmp eq {lt} {b}, 0"));
                let (zl, ok) = (self.new_label(), self.new_label());
                self.br_cold(&z, &zl, &ok);
                self.block(&zl);
                self.error_call_tur(hata_turu::SIFIRA_BOLME, "Sıfıra bölme hatası.", loc);
                self.block(&ok);
                let mut divisor = b.to_string();
                if s != Sign::Pos {
                    // en küçük sayı / -1 donanımda taşar: bölmede taşma hatası, kalanda sonuç 0
                    let min = int_const(&-(BigInt::one() << (w - 1)), w);
                    let (c1, c2, c3) = (self.tmp(), self.tmp(), self.tmp());
                    self.emit(&format!("{c1} = icmp eq {lt} {a}, {min}"));
                    self.emit(&format!("{c2} = icmp eq {lt} {b}, -1"));
                    self.emit(&format!("{c3} = and i1 {c1}, {c2}"));
                    if op == BinOp::Div {
                        self.overflow_branch(&c3, sym, a, b, ty, loc);
                    } else {
                        let d = self.tmp();
                        self.emit(&format!("{d} = select i1 {c3}, {lt} 1, {lt} {b}"));
                        divisor = d;
                    }
                }
                let ins = match (op, s) {
                    (BinOp::Div, Sign::Pos) => "udiv",
                    (BinOp::Div, _) => "sdiv",
                    (_, Sign::Pos) => "urem",
                    _ => "srem",
                };
                let t = self.tmp();
                self.emit(&format!("{t} = {ins} {lt} {a}, {divisor}"));
                if s == Sign::Neg {
                    let rc = self.neg_range_check(&t, ty);
                    self.overflow_branch(&rc, sym, a, b, ty, loc);
                }
                Ok(t)
            }
            _ => unreachable!(),
        }
    }

    /// Boyutlu Tam'da `<<`, `>>`, `**` (yorumlayıcıdaki Interp::shift_pow ile aynı kurallar ve mesajlar).
    fn shift_pow(&mut self, op: BinOp, a: &str, b: &str, ty: &Type, loc: Loc) -> String {
        let (_, s) = int_parts(ty);
        let w = width(ty);
        let lt = format!("i{w}");
        if s != Sign::Pos {
            let neg = self.tmp();
            self.emit(&format!("{neg} = icmp slt {lt} {b}, 0"));
            let (bad, ok) = (self.new_label(), self.new_label());
            self.br_cold(&neg, &bad, &ok);
            self.block(&bad);
            let (once, sonra) = match op {
                BinOp::Pow => ("Tam sayının üssü negatif olamaz (", "). Ondalık sonuç için: 2.0 ** -1"),
                _ => ("Kaydırma miktarı negatif olamaz (", ")."),
            };
            let (once, sonra) = (self.cstr(once), self.cstr(sonra));
            let (v, mode) = self.int128_form(b, ty);
            let (k, kd) = self.where_(loc);
            self.emit(&format!("call void @kolay_sayi_hata(i128 {v}, i32 {mode}, ptr {once}, ptr {sonra}, ptr {k}, ptr {kd})"));
            self.term("unreachable");
            self.block(&ok);
        }
        let t = |me: &mut Self| me.tmp();
        match op {
            BinOp::Shr => {
                let (big, sh, r) = (t(self), t(self), t(self));
                self.emit(&format!("{big} = icmp uge {lt} {b}, {w}"));
                if s == Sign::Pos {
                    self.emit(&format!("{sh} = select i1 {big}, {lt} 0, {lt} {b}"));
                    let r0 = t(self);
                    self.emit(&format!("{r0} = lshr {lt} {a}, {sh}"));
                    self.emit(&format!("{r} = select i1 {big}, {lt} 0, {lt} {r0}"));
                } else {
                    // eksi sayılar aşağı yuvarlanır; büyük kaydırmada sonuç 0 ya da -1
                    self.emit(&format!("{sh} = select i1 {big}, {lt} {}, {lt} {b}", w - 1));
                    self.emit(&format!("{r} = ashr {lt} {a}, {sh}"));
                }
                r
            }
            BinOp::Shl => {
                let (big, sh, r, back, lost, nz, bnz, of) = (t(self), t(self), t(self), t(self), t(self), t(self), t(self), t(self));
                self.emit(&format!("{big} = icmp uge {lt} {b}, {w}"));
                self.emit(&format!("{sh} = select i1 {big}, {lt} 0, {lt} {b}"));
                self.emit(&format!("{r} = shl {lt} {a}, {sh}"));
                let back_op = if s == Sign::Pos { "lshr" } else { "ashr" };
                self.emit(&format!("{back} = {back_op} {lt} {r}, {sh}"));
                self.emit(&format!("{lost} = icmp ne {lt} {back}, {a}"));
                self.emit(&format!("{nz} = icmp ne {lt} {a}, 0"));
                self.emit(&format!("{bnz} = and i1 {big}, {nz}"));
                self.emit(&format!("{of} = or i1 {lost}, {bnz}"));
                let bad = if s == Sign::Neg {
                    let rc = self.neg_range_check(&r, ty);
                    let o2 = t(self);
                    self.emit(&format!("{o2} = or i1 {of}, {rc}"));
                    o2
                } else {
                    of
                };
                self.overflow_branch(&bad, "<<", a, b, ty, loc);
                let res = t(self);
                self.emit(&format!("{res} = select i1 {big}, {lt} 0, {lt} {r}"));
                res
            }
            _ => {
                // kare al ve çarp; ara çarpımlardan biri taşarsa sonuç da gerçekten taşar
                let pre = if s == Sign::Pos { "u" } else { "s" };
                let name = format!("llvm.{pre}mul.with.overflow.{lt}");
                self.intrinsic(&name, format!("declare {{{lt}, i1}} @{name}({lt}, {lt})"));
                let (res, base, e) = (self.alloca(&lt), self.alloca(&lt), self.alloca(&lt));
                self.emit(&format!("store {lt} 1, ptr {res}"));
                self.emit(&format!("store {lt} {a}, ptr {base}"));
                self.emit(&format!("store {lt} {b}, ptr {e}"));
                let (cond, body, odd, even, sq, done) =
                    (self.new_label(), self.new_label(), self.new_label(), self.new_label(), self.new_label(), self.new_label());
                self.block(&cond);
                let ev = t(self);
                self.emit(&format!("{ev} = load {lt}, ptr {e}"));
                let z = t(self);
                self.emit(&format!("{z} = icmp eq {lt} {ev}, 0"));
                self.term(&format!("br i1 {z}, label %{done}, label %{body}"));
                self.block(&body);
                let (bit, isodd) = (t(self), t(self));
                self.emit(&format!("{bit} = and {lt} {ev}, 1"));
                self.emit(&format!("{isodd} = icmp ne {lt} {bit}, 0"));
                self.term(&format!("br i1 {isodd}, label %{odd}, label %{even}"));
                self.block(&odd);
                let (rv, bv, m, mv, mo) = (t(self), t(self), t(self), t(self), t(self));
                self.emit(&format!("{rv} = load {lt}, ptr {res}"));
                self.emit(&format!("{bv} = load {lt}, ptr {base}"));
                self.emit(&format!("{m} = call {{{lt}, i1}} @{name}({lt} {rv}, {lt} {bv})"));
                self.emit(&format!("{mv} = extractvalue {{{lt}, i1}} {m}, 0"));
                self.emit(&format!("{mo} = extractvalue {{{lt}, i1}} {m}, 1"));
                self.overflow_branch(&mo, "**", a, b, ty, loc);
                self.emit(&format!("store {lt} {mv}, ptr {res}"));
                self.term(&format!("br label %{even}"));
                self.block(&even);
                let (ev2, ne, z2) = (t(self), t(self), t(self));
                self.emit(&format!("{ev2} = load {lt}, ptr {e}"));
                self.emit(&format!("{ne} = lshr {lt} {ev2}, 1"));
                self.emit(&format!("store {lt} {ne}, ptr {e}"));
                self.emit(&format!("{z2} = icmp eq {lt} {ne}, 0"));
                self.term(&format!("br i1 {z2}, label %{done}, label %{sq}"));
                self.block(&sq);
                let (bv2, q, qv, qo) = (t(self), t(self), t(self), t(self));
                self.emit(&format!("{bv2} = load {lt}, ptr {base}"));
                self.emit(&format!("{q} = call {{{lt}, i1}} @{name}({lt} {bv2}, {lt} {bv2})"));
                self.emit(&format!("{qv} = extractvalue {{{lt}, i1}} {q}, 0"));
                self.emit(&format!("{qo} = extractvalue {{{lt}, i1}} {q}, 1"));
                self.overflow_branch(&qo, "**", a, b, ty, loc);
                self.emit(&format!("store {lt} {qv}, ptr {base}"));
                self.term(&format!("br label %{cond}"));
                self.block(&done);
                let r = t(self);
                self.emit(&format!("{r} = load {lt}, ptr {res}"));
                if s == Sign::Neg {
                    let rc = self.neg_range_check(&r, ty);
                    self.overflow_branch(&rc, "**", a, b, ty, loc);
                }
                r
            }
        }
    }

    /// Esnek Tam işlemi: iki değer de küçükse (en sık durum) yerinde, değilse çalışma kütüphanesinde.
    fn esnek_arith(&mut self, op: BinOp, a: &str, b: &str, ty: &Type, loc: Loc) -> String {
        let isaret = sign_code(ty);
        let tn = self.cstr(&type_display(ty, self.prog));
        let (k, kd) = self.where_(loc);
        let opc = match op {
            BinOp::Add => '+',
            BinOp::Sub => '-',
            BinOp::Mul => '*',
            BinOp::Div => '/',
            BinOp::BitAnd => '&',
            BinOp::BitOr => '|',
            BinOp::BitXor => '^',
            BinOp::Shl => 'l',
            BinOp::Shr => 'r',
            BinOp::Pow => 'p',
            _ => '%',
        } as u32;
        let slow_call = |me: &mut Self| -> String {
            let r = me.tmp();
            me.emit(&format!(
                "{r} = call i64 @kolay_esnek_islem(i64 {opc}, i64 {a}, i64 {b}, i64 {isaret}, ptr {tn}, ptr {k}, ptr {kd})"
            ));
            r
        };
        if !matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul) {
            let r = slow_call(self);
            return self.own(r, ty);
        }
        let (both, small) = (self.tmp(), self.tmp());
        self.emit(&format!("{both} = and i64 {a}, {b}"));
        let lowbit = self.tmp();
        self.emit(&format!("{lowbit} = and i64 {both}, 1"));
        self.emit(&format!("{small} = icmp ne i64 {lowbit}, 0"));
        let (fast, slow, end) = (self.new_label(), self.new_label(), self.new_label());
        self.term(&format!("br i1 {small}, label %{fast}, label %{slow}, !prof !0"));
        self.block(&fast);
        let (x, y) = match op {
            BinOp::Mul => {
                let (x, y) = (self.tmp(), self.tmp());
                self.emit(&format!("{x} = ashr i64 {a}, 1"));
                self.emit(&format!("{y} = sub i64 {b}, 1"));
                (x, y)
            }
            _ => {
                let y = self.tmp();
                self.emit(&format!("{y} = sub i64 {b}, 1"));
                (a.to_string(), y)
            }
        };
        let base = match op {
            BinOp::Add => "sadd",
            BinOp::Sub => "ssub",
            _ => "smul",
        };
        let name = format!("llvm.{base}.with.overflow.i64");
        self.intrinsic(&name, format!("declare {{i64, i1}} @{name}(i64, i64)"));
        let (r0, v0, of) = (self.tmp(), self.tmp(), self.tmp());
        self.emit(&format!("{r0} = call {{i64, i1}} @{name}(i64 {x}, i64 {y})"));
        self.emit(&format!("{v0} = extractvalue {{i64, i1}} {r0}, 0"));
        self.emit(&format!("{of} = extractvalue {{i64, i1}} {r0}, 1"));
        let v = if op == BinOp::Mul {
            let t = self.tmp();
            self.emit(&format!("{t} = or i64 {v0}, 1"));
            t
        } else {
            v0
        };
        // işaret kısıtı (Tam(+): değer >= 0 yani kelime >= 1; Tam(-): kelime <= 1)
        let bad = match isaret {
            1 => {
                let (c, o) = (self.tmp(), self.tmp());
                self.emit(&format!("{c} = icmp slt i64 {v}, 1"));
                self.emit(&format!("{o} = or i1 {of}, {c}"));
                o
            }
            2 => {
                let (c, o) = (self.tmp(), self.tmp());
                self.emit(&format!("{c} = icmp sgt i64 {v}, 1"));
                self.emit(&format!("{o} = or i1 {of}, {c}"));
                o
            }
            _ => of,
        };
        let fast_end = self.f.cur.clone();
        self.term(&format!("br i1 {bad}, label %{slow}, label %{end}, !prof !1"));
        self.block(&slow);
        let r = slow_call(self);
        let slow_end = self.f.cur.clone();
        self.term(&format!("br label %{end}"));
        self.block(&end);
        let p = self.tmp();
        self.emit(&format!("{p} = phi i64 [ {v}, %{fast_end} ], [ {r}, %{slow_end} ]"));
        self.own(p, ty)
    }

    fn arith_neg(&mut self, v: &str, ty: &Type, loc: Loc) -> CResult<String> {
        let (_, s) = int_parts(ty);
        let w = width(ty);
        let lt = format!("i{w}");
        let pre = if s == Sign::Pos { "u" } else { "s" };
        let name = format!("llvm.{pre}sub.with.overflow.{lt}");
        self.intrinsic(&name, format!("declare {{{lt}, i1}} @{name}({lt}, {lt})"));
        let r = self.tmp();
        self.emit(&format!("{r} = call {{{lt}, i1}} @{name}({lt} 0, {lt} {v})"));
        let (nv, of) = (self.tmp(), self.tmp());
        self.emit(&format!("{nv} = extractvalue {{{lt}, i1}} {r}, 0"));
        self.emit(&format!("{of} = extractvalue {{{lt}, i1}} {r}, 1"));
        let bad = if s == Sign::Neg {
            let rc = self.neg_range_check(&nv, ty);
            let t = self.tmp();
            self.emit(&format!("{t} = or i1 {of}, {rc}"));
            t
        } else {
            of
        };
        self.overflow_branch(&bad, "", "0", v, ty, loc);
        Ok(nv)
    }

    pub(super) fn compare(&mut self, op: BinOp, a: &str, b: &str, ty: &Type) -> CResult<String> {
        let t = self.tmp();
        match ty {
            Type::Int { bits: None, .. } => {
                // ikisi de küçükse etiketli kelimeler doğrudan karşılaştırılabilir
                let (both, lowbit, small) = (self.tmp(), self.tmp(), self.tmp());
                self.emit(&format!("{both} = and i64 {a}, {b}"));
                self.emit(&format!("{lowbit} = and i64 {both}, 1"));
                self.emit(&format!("{small} = icmp ne i64 {lowbit}, 0"));
                let (fast, slow, end) = (self.new_label(), self.new_label(), self.new_label());
                self.term(&format!("br i1 {small}, label %{fast}, label %{slow}, !prof !0"));
                self.block(&fast);
                let c = icmp_op(op, false);
                let f1 = self.tmp();
                self.emit(&format!("{f1} = icmp {c} i64 {a}, {b}"));
                let fe = self.f.cur.clone();
                self.term(&format!("br label %{end}"));
                self.block(&slow);
                let (r, s1) = (self.tmp(), self.tmp());
                self.emit(&format!("{r} = call i32 @kolay_esnek_kars(i64 {a}, i64 {b})"));
                self.emit(&format!("{s1} = icmp {c} i32 {r}, 0"));
                let se = self.f.cur.clone();
                self.term(&format!("br label %{end}"));
                self.block(&end);
                self.emit(&format!("{t} = phi i1 [ {f1}, %{fe} ], [ {s1}, %{se} ]"));
            }
            Type::Int { .. } => {
                let (_, s) = int_parts(ty);
                let w = width(ty);
                let c = icmp_op(op, s == Sign::Pos);
                self.emit(&format!("{t} = icmp {c} i{w} {a}, {b}"));
            }
            Type::Float(_) => {
                let lt = self.llty(ty);
                let c = match op {
                    BinOp::Eq => "oeq",
                    BinOp::Ne => "une",
                    BinOp::Lt => "olt",
                    BinOp::Le => "ole",
                    BinOp::Gt => "ogt",
                    BinOp::Ge => "oge",
                    _ => unreachable!(),
                };
                self.emit(&format!("{t} = fcmp {c} {lt} {a}, {b}"));
            }
            Type::Bool => {
                let c = if op == BinOp::Eq { "eq" } else { "ne" };
                self.emit(&format!("{t} = icmp {c} i1 {a}, {b}"));
            }
            Type::Str => match op {
                BinOp::Eq | BinOp::Ne => {
                    self.emit(&format!("{t} = call zeroext i1 @kolay_yazi_esit(ptr {a}, ptr {b})"));
                    if op == BinOp::Ne {
                        let n = self.tmp();
                        self.emit(&format!("{n} = xor i1 {t}, true"));
                        return Ok(n);
                    }
                }
                _ => {
                    let r = self.tmp();
                    self.emit(&format!("{r} = call i32 @kolay_yazi_kars(ptr {a}, ptr {b})"));
                    let c = icmp_op(op, false);
                    self.emit(&format!("{t} = icmp {c} i32 {r}, 0"));
                }
            },
            Type::Hic | Type::Never => {
                self.emit(&format!("{t} = icmp {} i1 true, true", if op == BinOp::Eq { "eq" } else { "ne" }));
            }
            _ => {
                // liste, sözlük, nesne, seçenek, 'veya', aralık: genel eşitlik
                let (pa, pb) = (self.spill(a, ty), self.spill(b, ty));
                let tip = self.tip(ty);
                self.emit(&format!("{t} = call zeroext i1 @kolay_esit(ptr {pa}, ptr {pb}, ptr {tip})"));
                if op == BinOp::Ne {
                    let n = self.tmp();
                    self.emit(&format!("{n} = xor i1 {t}, true"));
                    return Ok(n);
                }
            }
        }
        Ok(t)
    }

    // ------------------------------------------------------------ dönüştürmeler

    /// Sonucun sahipliği girdiden gelir (sahipli girdi -> sahipli sonuç).
    fn inherit(&mut self, from: &str, to: String, ty: &Type) -> String {
        if from != to && self.disown(from) {
            self.own(to, ty)
        } else {
            to
        }
    }

    fn assignable_cg(&self, f: &Type, t: &Type) -> bool {
        match (f, t) {
            _ if f == t => true,
            (Type::Int { bits: a, sign: sa }, Type::Int { bits: b, sign: sb }) => int_subset((*a, *sa), (*b, *sb)),
            (Type::Int { .. }, Type::Float(b)) => *b == 64,
            (Type::Float(a), Type::Float(b)) => a <= b,
            (Type::Named(c), Type::Named(p)) => self.prog.is_subtype(*c, *p),
            (Type::List(a, _), Type::List(b, _)) => a == b,
            (Type::Never, _) => true,
            _ => false,
        }
    }

    fn pick_member(&self, f: &Type, ms: &[Type]) -> usize {
        ms.iter().position(|m| m == f).or_else(|| ms.iter().position(|m| self.assignable_cg(f, m))).expect("uygun 'veya' üyesi")
    }

    /// Otomatik (veri kaybı olmayan) dönüştürme.
    pub(super) fn convert(&mut self, v: &str, from: &Type, to: &Type, loc: Loc) -> CResult<String> {
        if from == to || *from == Type::Never {
            return Ok(v.to_string());
        }
        Ok(match (from, to) {
            (_, Type::Union(ms)) if !matches!(from, Type::Union(_)) => {
                let idx = self.pick_member(from, ms);
                let m = ms[idx].clone();
                let inner = self.convert(v, from, &m, loc)?;
                let r = self.union_pack(&inner, &m, idx);
                self.inherit(&inner, r, to)
            }
            (Type::Union(a), Type::Union(b)) => self.union_remap(v, a, b, to, loc)?,
            (Type::Int { bits: Some(_), .. }, Type::Int { bits: Some(_), .. }) => {
                let (fw, tw) = (width(from), width(to));
                if fw == tw {
                    return Ok(v.to_string());
                }
                let (_, fs) = int_parts(from);
                let op = if tw < fw { "trunc" } else if fs == Sign::Pos { "zext" } else { "sext" };
                let t = self.tmp();
                self.emit(&format!("{t} = {op} i{fw} {v} to i{tw}"));
                t
            }
            (Type::Int { bits: Some(_), .. }, Type::Int { bits: None, .. }) => self.esnek_from_fixed(v, from),
            (Type::Int { bits: None, .. }, Type::Int { bits: None, .. }) => v.to_string(),
            (Type::Int { bits: Some(_), .. }, Type::Float(_)) => {
                let (_, fs) = int_parts(from);
                let op = if fs == Sign::Pos { "uitofp" } else { "sitofp" };
                let (fl, tl) = (self.llty(from), self.llty(to));
                let t = self.tmp();
                self.emit(&format!("{t} = {op} {fl} {v} to {tl}"));
                t
            }
            (Type::Int { bits: None, .. }, Type::Float(b)) => {
                let d = self.tmp();
                self.emit(&format!("{d} = call double @kolay_esnek_ondalik(i64 {v})"));
                self.drop_temp(v);
                if *b == 64 {
                    d
                } else {
                    let tl = self.llty(to);
                    let t = self.tmp();
                    self.emit(&format!("{t} = fptrunc double {d} to {tl}"));
                    t
                }
            }
            (Type::Float(a), Type::Float(b)) => {
                let op = if b > a { "fpext" } else { "fptrunc" };
                let (fl, tl) = (self.llty(from), self.llty(to));
                let t = self.tmp();
                self.emit(&format!("{t} = {op} {fl} {v} to {tl}"));
                t
            }
            (Type::Named(_), Type::Named(_)) => v.to_string(),
            (Type::List(_, s1), Type::List(_, Some(n))) => {
                if *s1 != Some(*n) {
                    // esnek liste sabit boyutlu listeye: eleman sayısı kontrol edilir
                    let (lp, len, ok) = (self.tmp(), self.tmp(), self.tmp());
                    self.emit(&format!("{lp} = getelementptr i8, ptr {v}, i64 16"));
                    self.emit(&format!("{len} = load i64, ptr {lp}{}", super::BASLIK));
                    self.emit(&format!("{ok} = icmp eq i64 {len}, {n}"));
                    let (bad, good) = (self.new_label(), self.new_label());
                    self.term(&format!("br i1 {ok}, label %{good}, label %{bad}, !prof !0"));
                    self.block(&bad);
                    let tn = self.cstr(&type_display(to, self.prog));
                    let (k, kd) = self.where_(loc);
                    self.emit(&format!("call void @kolay_liste_boyut_hatasi(ptr {tn}, i64 {n}, i64 {len}, ptr {k}, ptr {kd})"));
                    self.term("unreachable");
                    self.block(&good);
                }
                v.to_string()
            }
            (Type::List(..), Type::List(..)) | (Type::Dict(..), Type::Dict(..)) => v.to_string(),
            _ => return self.unsupported(loc, &format!("bu tip dönüşümünü ({} -> {})", type_display(from, self.prog), type_display(to, self.prog))),
        })
    }

    fn esnek_from_fixed(&mut self, v: &str, from: &Type) -> String {
        let w = width(from);
        let (_, s) = int_parts(from);
        let inline = if s == Sign::Pos { w <= 62 } else { w <= 63 };
        if inline {
            let (x, sh, o) = (self.tmp(), self.tmp(), self.tmp());
            if w == 64 {
                self.emit(&format!("{x} = add i64 {v}, 0"));
            } else {
                let op = if s == Sign::Pos { "zext" } else { "sext" };
                self.emit(&format!("{x} = {op} i{w} {v} to i64"));
            }
            self.emit(&format!("{sh} = shl i64 {x}, 1"));
            self.emit(&format!("{o} = or i64 {sh}, 1"));
            return o;
        }
        let (bits, mode) = self.int128_form(v, from);
        let t = self.tmp();
        self.emit(&format!("{t} = call i64 @kolay_esnek_tamdan(i128 {bits}, i32 {mode})"));
        self.own(t, &crate::types::ESNEK)
    }

    /// 'veya' -> daha geniş 'veya': etiket yeni tipteki üyeye çevrilir. Sonuç sahiplidir.
    fn union_remap(&mut self, v: &str, a: &[Type], b: &[Type], to: &Type, loc: Loc) -> CResult<String> {
        let tag = self.union_tag(v);
        let res = self.alloca("%veya");
        let end = self.new_label();
        let labels: Vec<String> = a.iter().map(|_| self.new_label()).collect();
        let def = self.new_label();
        let cases: Vec<String> = labels.iter().enumerate().map(|(i, l)| format!("i64 {i}, label %{l}")).collect();
        self.term(&format!("switch i64 {tag}, label %{def} [ {} ]", cases.join(" ")));
        for (i, l) in labels.iter().enumerate() {
            self.block(l);
            let m = a[i].clone();
            let j = self.pick_member(&m, b);
            let x = self.union_unpack(v, &m);
            let y = self.convert(&x, &m, &b[j], loc)?;
            let bj = b[j].clone();
            self.ensure_owned(&y, &bj);
            self.disown(&y);
            let p = self.union_pack(&y, &bj, j);
            self.emit(&format!("store %veya {p}, ptr {res}"));
            self.term(&format!("br label %{end}"));
        }
        self.block(&def);
        self.term("unreachable");
        self.block(&end);
        let r = self.tmp();
        self.emit(&format!("{r} = load %veya, ptr {res}"));
        self.drop_temp(v);
        Ok(self.own(r, to))
    }

    fn uyum(&self, m: &Type, t: &Type) -> Uyum {
        match (m, t) {
            (_, Type::Union(ts)) => {
                let us: Vec<Uyum> = ts.iter().map(|x| self.uyum(m, x)).collect();
                if us.contains(&Uyum::Her) {
                    Uyum::Her
                } else if us.contains(&Uyum::Bak) {
                    Uyum::Bak
                } else {
                    Uyum::Yok
                }
            }
            (Type::Named(a), Type::Named(b)) => {
                if self.prog.is_subtype(*a, *b) {
                    Uyum::Her
                } else {
                    Uyum::Bak
                }
            }
            (Type::Int { .. }, Type::Int { .. })
            | (Type::Float(_), Type::Float(_))
            | (Type::Str, Type::Str)
            | (Type::Bool, Type::Bool)
            | (Type::Hic, Type::Hic)
            | (Type::List(..), Type::List(..))
            | (Type::Dict(..), Type::Dict(..))
            => Uyum::Her,
            _ => Uyum::Yok,
        }
    }

    fn kind_name(&self, t: &Type) -> &'static str {
        match t {
            Type::Int { .. } => "Tam",
            Type::Float(_) => "Ondalık",
            Type::Bool => "Mantık",
            Type::Str => "Yazı",
            Type::Hic | Type::Never => "Hiç",
            Type::List(..) => "Liste",
            Type::Dict(..) => "Sözlük",
            _ => "?",
        }
    }

    fn type_error(&mut self, to: &Type, m: &Type, val: &str, loc: Loc) {
        let tn = self.cstr(&type_display(to, self.prog));
        let (k, kd) = self.where_(loc);
        if let Type::Named(_) = m {
            self.emit(&format!("call void @kolay_tur_degil_nesne(ptr {tn}, ptr {val}, ptr {k}, ptr {kd})"));
        } else {
            let kn = self.cstr(self.kind_name(m));
            self.emit(&format!("call void @kolay_tur_degil(ptr {tn}, ptr {kn}, ptr {k}, ptr {kd})"));
        }
        self.term("unreachable");
    }

    /// `olarak` ile açık dönüştürme (çalışma anında kontrol edilir).
    pub(super) fn olarak(&mut self, v: &str, from: &Type, to: &Type, loc: Loc, ftama: bool) -> CResult<String> {
        if from == to {
            return Ok(v.to_string());
        }
        match (from, to) {
            (Type::Union(ms), _) => self.union_olarak(v, ms, to, loc),
            (Type::Named(a), Type::Named(b)) => {
                if !self.prog.is_subtype(*a, *b) {
                    let ok = self.dyn_is_subtype(v, *b);
                    let (bad, good) = (self.new_label(), self.new_label());
                    self.term(&format!("br i1 {ok}, label %{good}, label %{bad}, !prof !0"));
                    self.block(&bad);
                    self.type_error(to, from, v, loc);
                    self.block(&good);
                }
                Ok(v.to_string())
            }
            (Type::Int { bits: Some(_), .. }, Type::Int { bits: Some(_), .. }) => self.olarak_fixed(v, from, to, loc),
            (Type::Int { bits: Some(_), .. }, Type::Int { bits: None, sign }) => {
                let e = self.esnek_from_fixed(v, from);
                Ok(self.esnek_restrict(&e, *sign, to, loc))
            }
            (Type::Int { bits: None, .. }, Type::Int { bits: None, sign }) => {
                let r = self.esnek_restrict(v, *sign, to, loc);
                Ok(r)
            }
            (Type::Int { bits: None, .. }, Type::Int { bits: Some(tb), sign }) => {
                let tn = self.cstr(&type_display(to, self.prog));
                let bounds = self.cstr(&bounds_text(Some(*tb), *sign));
                let (k, kd) = self.where_(loc);
                let r = self.tmp();
                self.emit(&format!(
                    "{r} = call i128 @kolay_esnek_boyutluya(i64 {v}, i64 {tb}, i64 {}, ptr {tn}, ptr {bounds}, ptr {k}, ptr {kd})",
                    sign_code(to)
                ));
                self.drop_temp(v);
                let w = width(to);
                Ok(if w == 128 {
                    r
                } else if w > 128 {
                    let (z, n) = (self.tmp(), self.tmp());
                    self.emit(&format!("{z} = zext i128 {r} to i{w}"));
                    self.emit(&format!("{n} = sub i{w} 0, {z}"));
                    n
                } else {
                    let t = self.tmp();
                    self.emit(&format!("{t} = trunc i128 {r} to i{w}"));
                    t
                })
            }
            (Type::Float(fb), Type::Int { bits: None, sign }) => {
                let d = self.to_double(v, *fb);
                let (k, kd) = self.where_(loc);
                let r = self.tmp();
                self.emit(&format!("{r} = call i64 @kolay_esnek_ondaliktan(double {d}, ptr {k}, ptr {kd})"));
                let r = self.own(r, to);
                Ok(self.esnek_restrict(&r, *sign, to, loc))
            }
            (Type::Float(fb), Type::Int { .. }) => self.olarak_float_int(v, *fb, to, loc, ftama),
            (Type::Int { bits: None, .. }, Type::Float(tb)) if *tb < 64 => {
                // esnek Tam: değer, kontrol bitene kadar bırakılmaz (hata mesajında yazılır)
                let d = self.tmp();
                self.emit(&format!("{d} = call double @kolay_esnek_ondalik(i64 {v})"));
                let tl = self.llty(to);
                let r = self.tmp();
                self.emit(&format!("{r} = fptrunc double {d} to {tl}"));
                self.float_fit_check(v, from, &r, *tb, to, loc);
                self.drop_temp(v);
                Ok(r)
            }
            (Type::Int { .. } | Type::Float(_), Type::Float(tb)) => {
                let r = self.convert(v, from, to, loc)?;
                if *tb < 64 {
                    self.float_fit_check(v, from, &r, *tb, to, loc);
                }
                Ok(r)
            }
            _ => self.convert(v, from, to, loc),
        }
    }

    /// `olarak Ondalık(16/32)`: sonuç sonsuza döndüyse (ve kaynak sonlu idiyse) hata.
    fn float_fit_check(&mut self, src: &str, from: &Type, r: &str, tb: u32, to: &Type, loc: Loc) {
        let lt = self.llty(to);
        let inf = float_const(f64::INFINITY, tb);
        let (a, ri) = (self.tmp(), self.tmp());
        let fabs = format!("llvm.fabs.{}", if tb == 16 { "f16" } else { "f32" });
        self.intrinsic(&fabs, format!("declare {lt} @{fabs}({lt})"));
        self.emit(&format!("{a} = call {lt} @{fabs}({lt} {r})"));
        self.emit(&format!("{ri} = fcmp oeq {lt} {a}, {inf}"));
        let bad = match from {
            Type::Float(fb) => {
                // kaynak zaten sonsuzsa hata değil
                let d = self.to_double(src, *fb);
                self.intrinsic("fabs.f64", "declare double @llvm.fabs.f64(double)".into());
                let (sa, fin, b) = (self.tmp(), self.tmp(), self.tmp());
                self.emit(&format!("{sa} = call double @llvm.fabs.f64(double {d})"));
                self.emit(&format!("{fin} = fcmp one double {sa}, 0x7FF0000000000000"));
                self.emit(&format!("{b} = and i1 {ri}, {fin}"));
                b
            }
            _ => ri,
        };
        let (badl, ok) = (self.new_label(), self.new_label());
        self.br_cold(&bad, &badl, &ok);
        self.block(&badl);
        let tn = self.cstr(&type_display(to, self.prog));
        let lim = self.cstr(crate::types::float_limit_text(tb));
        let (k, kd) = self.where_(loc);
        match from {
            Type::Float(fb) => {
                let d = self.to_double(src, *fb);
                self.emit(&format!("call void @kolay_sigmiyor_f(double {d}, ptr {tn}, ptr {lim}, ptr {k}, ptr {kd})"));
            }
            Type::Int { bits: None, .. } => {
                self.emit(&format!("call void @kolay_sigmiyor_esnek(i64 {src}, ptr {tn}, ptr {lim}, ptr {k}, ptr {kd})"));
            }
            _ => {
                let (bits, mode) = self.int128_form(src, from);
                self.emit(&format!("call void @kolay_sigmiyor(i128 {bits}, i32 {mode}, ptr {tn}, ptr {lim}, ptr {k}, ptr {kd})"));
            }
        }
        self.term("unreachable");
        self.block(&ok);
    }

    fn esnek_restrict(&mut self, v: &str, sign: Sign, to: &Type, loc: Loc) -> String {
        if sign == Sign::Signed {
            return v.to_string();
        }
        let tn = self.cstr(&type_display(to, self.prog));
        let bounds = self.cstr(&bounds_text(None, sign));
        let (k, kd) = self.where_(loc);
        let r = self.tmp();
        self.emit(&format!(
            "{r} = call i64 @kolay_esnek_kisitla(i64 {v}, i64 {}, ptr {tn}, ptr {bounds}, ptr {k}, ptr {kd})",
            sign_code(to)
        ));
        self.inherit(v, r, to)
    }

    fn olarak_fixed(&mut self, v: &str, from: &Type, to: &Type, loc: Loc) -> CResult<String> {
        let (fw, tw) = (width(from), width(to));
        let (tb, ts) = int_parts(to);
        let (_, fs) = int_parts(from);
        let big = fw.max(tw) + 2;
        let ext = self.tmp();
        let op = if fs == Sign::Pos { "zext" } else { "sext" };
        self.emit(&format!("{ext} = {op} i{fw} {v} to i{big}"));
        let (lo, hi) = int_bounds(Some(tb), ts);
        let (c1, c2, c3) = (self.tmp(), self.tmp(), self.tmp());
        self.emit(&format!("{c1} = icmp slt i{big} {ext}, {}", int_const(&lo.unwrap(), big)));
        self.emit(&format!("{c2} = icmp sgt i{big} {ext}, {}", int_const(&hi.unwrap(), big)));
        self.emit(&format!("{c3} = or i1 {c1}, {c2}"));
        let (bad, ok) = (self.new_label(), self.new_label());
        self.br_cold(&c3, &bad, &ok);
        self.block(&bad);
        let (mag, neg) = self.int128_form(v, from);
        let tn = self.cstr(&type_display(to, self.prog));
        let bounds = self.cstr(&bounds_text(Some(tb), ts));
        let (k, c) = self.where_(loc);
        self.emit(&format!("call void @kolay_sigmiyor(i128 {mag}, i32 {neg}, ptr {tn}, ptr {bounds}, ptr {k}, ptr {c})"));
        self.term("unreachable");
        self.block(&ok);
        let t = self.tmp();
        self.emit(&format!("{t} = trunc i{big} {ext} to i{tw}"));
        Ok(t)
    }

    fn olarak_float_int(&mut self, v: &str, fb: u32, to: &Type, loc: Loc, ftama: bool) -> CResult<String> {
        let (tb, ts) = int_parts(to);
        let tl = self.llty(to);
        let d = self.to_double(v, fb);
        let tn = self.cstr(&type_display(to, self.prog));
        let bounds = self.cstr(&bounds_text(Some(tb), ts));
        let (k, c) = self.where_(loc);
        // Sonsuz ve TanımsızSayı tam sayıya çevrilemez.
        self.intrinsic("fabs.f64", "declare double @llvm.fabs.f64(double)".into());
        let abs = self.tmp();
        self.emit(&format!("{abs} = call double @llvm.fabs.f64(double {d})"));
        let nonfinite = self.tmp();
        self.emit(&format!("{nonfinite} = fcmp ueq double {abs}, 0x7FF0000000000000"));
        let (nf, fin) = (self.new_label(), self.new_label());
        self.br_cold(&nonfinite, &nf, &fin);
        self.block(&nf);
        if ftama {
            self.emit(&format!("call void @kolay_sigmiyor_f(double {d}, ptr {tn}, ptr {bounds}, ptr {k}, ptr {c})"));
        } else {
            self.emit(&format!("call void @kolay_cevrilemez_f(double {d}, ptr {k}, ptr {c})"));
        }
        self.term("unreachable");
        self.block(&fin);
        let tr = self.tmp();
        self.intrinsic("trunc.f64", "declare double @llvm.trunc.f64(double)".into());
        self.emit(&format!("{tr} = call double @llvm.trunc.f64(double {d})"));
        let (lo, hi) = int_bounds(Some(tb), ts);
        let lo: f64 = lo.unwrap().to_string().parse().unwrap();
        let hi: f64 = hi.unwrap().to_string().parse().unwrap();
        let (c1, c2, c3) = (self.tmp(), self.tmp(), self.tmp());
        self.emit(&format!("{c1} = fcmp ult double {tr}, {}", float_const(lo, 64)));
        self.emit(&format!("{c2} = fcmp uge double {tr}, {}", float_const(hi + 1.0, 64)));
        self.emit(&format!("{c3} = or i1 {c1}, {c2}"));
        let (bad, ok) = (self.new_label(), self.new_label());
        self.br_cold(&c3, &bad, &ok);
        self.block(&bad);
        self.emit(&format!("call void @kolay_sigmiyor_f(double {d}, ptr {tn}, ptr {bounds}, ptr {k}, ptr {c})"));
        self.term("unreachable");
        self.block(&ok);
        let op = if ts == Sign::Pos { "fptoui" } else { "fptosi" };
        let t = self.tmp();
        self.emit(&format!("{t} = {op} double {tr} to {tl}"));
        Ok(t)
    }

    /// 'veya' değerinden `olarak` ile çıkarma: etikete göre uygun üye seçilir, uymazsa hata.
    fn union_olarak(&mut self, v: &str, ms: &[Type], to: &Type, loc: Loc) -> CResult<String> {
        let tag = self.union_tag(v);
        let lt = self.llty(to);
        let res = self.alloca(&lt);
        let end = self.new_label();
        let labels: Vec<String> = ms.iter().map(|_| self.new_label()).collect();
        let def = self.new_label();
        let cases: Vec<String> = labels.iter().enumerate().map(|(i, l)| format!("i64 {i}, label %{l}")).collect();
        self.term(&format!("switch i64 {tag}, label %{def} [ {} ]", cases.join(" ")));
        for (i, l) in labels.iter().enumerate() {
            self.block(l);
            let m = ms[i].clone();
            let x = self.union_unpack(v, &m);
            let u = self.uyum(&m, to);
            if u == Uyum::Yok {
                self.type_error(to, &m, &x, loc);
                continue;
            }
            let y = if let Type::Union(ts) = to {
                let j = self.pick_member(&m, ts);
                if u == Uyum::Bak {
                    if let Type::Named(t) = &ts[j] {
                        let ok = self.dyn_is_subtype(&x, *t);
                        let (bad, good) = (self.new_label(), self.new_label());
                        self.term(&format!("br i1 {ok}, label %{good}, label %{bad}, !prof !0"));
                        self.block(&bad);
                        self.type_error(to, &m, &x, loc);
                        self.block(&good);
                    }
                }
                let tj = ts[j].clone();
                let c = self.convert(&x, &m, &tj, loc)?;
                self.ensure_owned(&c, &tj);
                self.disown(&c);
                self.union_pack(&c, &tj, j)
            } else {
                let c = match (&m, to) {
                    (Type::Named(_), Type::Named(t)) => {
                        if u == Uyum::Bak {
                            let ok = self.dyn_is_subtype(&x, *t);
                            let (bad, good) = (self.new_label(), self.new_label());
                            self.term(&format!("br i1 {ok}, label %{good}, label %{bad}, !prof !0"));
                            self.block(&bad);
                            self.type_error(to, &m, &x, loc);
                            self.block(&good);
                        }
                        x.clone()
                    }
                    (Type::Int { .. } | Type::Float(_), Type::Int { .. } | Type::Float(_)) => self.olarak(&x, &m, to, loc, false)?,
                    _ => self.convert(&x, &m, to, loc)?,
                };
                self.ensure_owned(&c, to);
                self.disown(&c);
                c
            };
            if lt != "void" {
                self.emit(&format!("store {lt} {y}, ptr {res}"));
            }
            self.term(&format!("br label %{end}"));
        }
        self.block(&def);
        self.term("unreachable");
        self.block(&end);
        if lt == "void" {
            return Ok(String::new());
        }
        let r = self.tmp();
        self.emit(&format!("{r} = load {lt}, ptr {res}"));
        self.drop_temp(v);
        Ok(self.own(r, to))
    }
}

/// Tam tipinin işaret kodu (çalışma kütüphanesi için): 0 işaretli, 1 sadece pozitif, 2 sadece negatif
pub(super) fn sign_code(t: &Type) -> i64 {
    match t {
        Type::Int { sign: Sign::Pos, .. } => 1,
        Type::Int { sign: Sign::Neg, .. } => 2,
        _ => 0,
    }
}

fn icmp_op(op: BinOp, unsigned: bool) -> &'static str {
    match op {
        BinOp::Eq => "eq",
        BinOp::Ne => "ne",
        BinOp::Lt => if unsigned { "ult" } else { "slt" },
        BinOp::Le => if unsigned { "ule" } else { "sle" },
        BinOp::Gt => if unsigned { "ugt" } else { "sgt" },
        BinOp::Ge => if unsigned { "uge" } else { "sge" },
        _ => unreachable!(),
    }
}

