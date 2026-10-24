// Değerlerin bellek düzeni: yazmaç tipleri, depolama biçimi, 'veya' paketleme, nesne ofsetleri ve
// çalışma kütüphanesinin kullandığı tip tanımları (runtime/kolay.h içindeki Tip ile aynı düzen).

use super::{Gen, BASLIK, ELEMAN};
use crate::tir::*;
use crate::types::{Sign, Type, TypeId};
use crate::values::type_display;
use std::fmt::Write as _;

// runtime/kolay.h içindeki tip türleri
const T_TAM: i64 = 1;
const T_ESNEK: i64 = 2;
const T_ONDALIK: i64 = 3;
const T_MANTIK: i64 = 4;
const T_HIC: i64 = 6;
const T_LISTE: i64 = 7;
const T_SOZLUK: i64 = 8;
const T_TUR: i64 = 9;
const T_SECENEK: i64 = 10;
const T_ARAYUZ: i64 = 11;
pub(super) const T_FONK: i64 = 15;
const T_VEYA: i64 = 12;

/// Depolama biçimi: LLVM tipi, boyut, hizalama
pub(super) struct Stor {
    pub ll: String,
    pub size: i64,
    pub align: i64,
}

pub(super) fn int_parts(t: &Type) -> (u32, Sign) {
    match t {
        Type::Int { bits: Some(b), sign } => (*b, *sign),
        _ => unreachable!("boyutlu Tam bekleniyordu"),
    }
}

/// LLVM'deki bit genişliği. Tam(N, -) değerleri işaretli olarak N+1 bitte tutulur.
pub(super) fn width(t: &Type) -> u32 {
    let (b, s) = int_parts(t);
    if s == Sign::Neg { b + 1 } else { b }
}

pub(super) fn is_esnek(t: &Type) -> bool {
    matches!(t, Type::Int { bits: None, .. })
}

pub(super) fn is_fixed_int(t: &Type) -> bool {
    matches!(t, Type::Int { bits: Some(_), .. })
}

pub(super) fn is_ptr_type(t: &Type) -> bool {
    matches!(t, Type::Str | Type::List(..) | Type::Dict(..) | Type::Named(_) | Type::Func(..))
}

fn int_storage_bits(w: u32) -> u32 {
    match w {
        0..=8 => 8,
        9..=16 => 16,
        17..=32 => 32,
        33..=64 => 64,
        _ => 128,
    }
}

impl<'p> Gen<'p> {
    /// Yazmaçtaki LLVM tipi
    pub(super) fn llty(&self, t: &Type) -> String {
        match t {
            Type::Int { bits: Some(_), .. } => format!("i{}", width(t)),
            Type::Int { bits: None, .. } => "i64".into(),
            Type::Float(16) => "half".into(),
            Type::Float(32) => "float".into(),
            Type::Float(_) => "double".into(),
            Type::Bool => "i1".into(),
            Type::Str | Type::List(..) | Type::Dict(..) | Type::Named(_) | Type::Func(..) => "ptr".into(),
            Type::Union(_) => "%veya".into(),
            Type::Hic | Type::Never => "void".into(),
        }
    }

    /// Depolama biçimi (liste elemanı, nesne alanı, sözlük girdisi)
    pub(super) fn stor(&self, t: &Type) -> Stor {
        let s = |ll: &str, size: i64, align: i64| Stor { ll: ll.into(), size, align };
        match t {
            Type::Int { bits: Some(_), .. } => {
                let b = int_storage_bits(width(t));
                Stor { ll: format!("i{b}"), size: (b / 8) as i64, align: (b / 8) as i64 }
            }
            Type::Int { bits: None, .. } => s("i64", 8, 8),
            Type::Float(16) => s("half", 2, 2),
            Type::Float(32) => s("float", 4, 4),
            Type::Float(_) => s("double", 8, 8),
            Type::Bool => s("i8", 1, 1),
            Type::Str | Type::List(..) | Type::Dict(..) | Type::Named(_) | Type::Func(..) => s("ptr", 8, 8),
            Type::Union(_) => s("%veya", 24, 8),
            Type::Hic | Type::Never => s("i8", 1, 1),
        }
    }

    pub(super) fn zero_value(&self, t: &Type) -> String {
        match t {
            Type::Float(_) => "0.0".into(),
            Type::Bool => "false".into(),
            Type::Str | Type::List(..) | Type::Dict(..) | Type::Named(_) | Type::Func(..) => "null".into(),
            Type::Union(_) => "zeroinitializer".into(),
            _ => "0".into(),
        }
    }

    // ------------------------------------------------------------ yazmaç <-> depolama

    /// Boyutlu Tam'ı depolama biçimindeki tam sayıya çevirir: (değer, LLVM tipi)
    fn int_to_storage(&mut self, v: &str, t: &Type) -> (String, String) {
        let (_, s) = int_parts(t);
        let w = width(t);
        let sb = int_storage_bits(w);
        if w > 128 {
            // Tam(128, -): eksiyle çarpılıp işaretsiz tutulur
            let (n, m) = (self.tmp(), self.tmp());
            self.emit(&format!("{n} = sub i{w} 0, {v}"));
            self.emit(&format!("{m} = trunc i{w} {n} to i128"));
            return (m, "i128".into());
        }
        if w == sb {
            return (v.to_string(), format!("i{sb}"));
        }
        let t2 = self.tmp();
        let op = if s == Sign::Pos { "zext" } else { "sext" };
        self.emit(&format!("{t2} = {op} i{w} {v} to i{sb}"));
        (t2, format!("i{sb}"))
    }

    fn int_from_storage(&mut self, v: &str, t: &Type) -> String {
        let w = width(t);
        let sb = int_storage_bits(w);
        if w > 128 {
            let (z, n) = (self.tmp(), self.tmp());
            self.emit(&format!("{z} = zext i128 {v} to i{w}"));
            self.emit(&format!("{n} = sub i{w} 0, {z}"));
            return n;
        }
        if w == sb {
            return v.to_string();
        }
        let t2 = self.tmp();
        self.emit(&format!("{t2} = trunc i{sb} {v} to i{w}"));
        t2
    }

    /// Bellekteki (depolama biçimindeki) değeri yazmaca okur. Referans tutulmaz.
    pub(super) fn load_mem(&mut self, ptr: &str, t: &Type) -> String {
        let st = self.stor(t);
        let v = self.tmp();
        self.emit(&format!("{v} = load {}, ptr {ptr}{ELEMAN}", st.ll));
        match t {
            Type::Int { bits: Some(_), .. } => self.int_from_storage(&v, t),
            Type::Bool => {
                let b = self.tmp();
                self.emit(&format!("{b} = icmp ne i8 {v}, 0"));
                b
            }
            _ => v,
        }
    }

    /// Değeri belleğe (depolama biçiminde) yazar. Referans işlemi yapılmaz.
    pub(super) fn store_mem(&mut self, ptr: &str, v: &str, t: &Type) {
        match t {
            Type::Int { bits: Some(_), .. } => {
                let (s, ll) = self.int_to_storage(v, t);
                self.emit(&format!("store {ll} {s}, ptr {ptr}{ELEMAN}"));
            }
            Type::Bool => {
                let b = self.tmp();
                self.emit(&format!("{b} = zext i1 {v} to i8"));
                self.emit(&format!("store i8 {b}, ptr {ptr}{ELEMAN}"));
            }
            Type::Hic | Type::Never => {}
            _ => {
                let st = self.stor(t);
                self.emit(&format!("store {} {v}, ptr {ptr}{ELEMAN}", st.ll));
            }
        }
    }

    /// Değeri geçici bir bellek alanına yazıp adresini verir (çalışma kütüphanesine vermek için).
    pub(super) fn spill(&mut self, v: &str, t: &Type) -> String {
        let st = self.stor(t);
        let a = self.alloca(&st.ll);
        self.store_mem(&a, v, t);
        a
    }

    // ------------------------------------------------------------ 'veya' paketleme

    /// Üye değerini 'veya' kelimelerine çevirir: (birinci, ikinci). Kelimeler, üyenin depolama
    /// biçimindeki baytlarıdır (çalışma kütüphanesi değeri etiketten sonraki 16 baytta bu biçimde bulur).
    fn union_words(&mut self, v: &str, m: &Type) -> (String, String) {
        match m {
            Type::Int { bits: Some(_), .. } => {
                let (s, ll) = self.int_to_storage(v, m);
                if ll == "i64" {
                    return (s, "0".into());
                }
                if ll == "i128" {
                    let (lo, sh, hi) = (self.tmp(), self.tmp(), self.tmp());
                    self.emit(&format!("{lo} = trunc i128 {s} to i64"));
                    self.emit(&format!("{sh} = lshr i128 {s}, 64"));
                    self.emit(&format!("{hi} = trunc i128 {sh} to i64"));
                    return (lo, hi);
                }
                let z = self.tmp();
                self.emit(&format!("{z} = zext {ll} {s} to i64"));
                (z, "0".into())
            }
            Type::Int { bits: None, .. } => (v.to_string(), "0".into()),
            Type::Float(b) => {
                let (fl, il) = match b {
                    16 => ("half", "i16"),
                    32 => ("float", "i32"),
                    _ => ("double", "i64"),
                };
                let c = self.tmp();
                self.emit(&format!("{c} = bitcast {fl} {v} to {il}"));
                if il == "i64" {
                    return (c, "0".into());
                }
                let z = self.tmp();
                self.emit(&format!("{z} = zext {il} {c} to i64"));
                (z, "0".into())
            }
            Type::Bool => {
                let z = self.tmp();
                self.emit(&format!("{z} = zext i1 {v} to i64"));
                (z, "0".into())
            }
            Type::Str | Type::List(..) | Type::Dict(..) | Type::Named(_) | Type::Func(..) => {
                let z = self.tmp();
                self.emit(&format!("{z} = ptrtoint ptr {v} to i64"));
                (z, "0".into())
            }
            _ => ("0".into(), "0".into()),
        }
    }

    /// Üye değerinden 'veya' değeri (sahiplik aynen geçer)
    pub(super) fn union_pack(&mut self, v: &str, m: &Type, tag: usize) -> String {
        let (w0, w1) = self.union_words(v, m);
        let (a, b, c) = (self.tmp(), self.tmp(), self.tmp());
        self.emit(&format!("{a} = insertvalue %veya undef, i64 {tag}, 0"));
        self.emit(&format!("{b} = insertvalue %veya {a}, i64 {w0}, 1"));
        self.emit(&format!("{c} = insertvalue %veya {b}, i64 {w1}, 2"));
        c
    }

    pub(super) fn union_tag(&mut self, u: &str) -> String {
        let t = self.tmp();
        self.emit(&format!("{t} = extractvalue %veya {u}, 0"));
        t
    }

    /// 'veya' değerinden üye değeri (etiketin doğru olduğu bilinmeli; referans tutulmaz)
    pub(super) fn union_unpack(&mut self, u: &str, m: &Type) -> String {
        let (w0, w1) = (self.tmp(), self.tmp());
        self.emit(&format!("{w0} = extractvalue %veya {u}, 1"));
        self.emit(&format!("{w1} = extractvalue %veya {u}, 2"));
        match m {
            Type::Int { bits: Some(_), .. } => {
                let sb = int_storage_bits(width(m));
                let s = if sb == 64 {
                    w0
                } else if sb == 128 {
                    let (lo, hi, sh, o) = (self.tmp(), self.tmp(), self.tmp(), self.tmp());
                    self.emit(&format!("{lo} = zext i64 {w0} to i128"));
                    self.emit(&format!("{hi} = zext i64 {w1} to i128"));
                    self.emit(&format!("{sh} = shl i128 {hi}, 64"));
                    self.emit(&format!("{o} = or i128 {sh}, {lo}"));
                    o
                } else {
                    let t = self.tmp();
                    self.emit(&format!("{t} = trunc i64 {w0} to i{sb}"));
                    t
                };
                self.int_from_storage(&s, m)
            }
            Type::Int { bits: None, .. } => w0,
            Type::Float(b) => {
                let (fl, il) = match b {
                    16 => ("half", "i16"),
                    32 => ("float", "i32"),
                    _ => ("double", "i64"),
                };
                let s = if il == "i64" {
                    w0
                } else {
                    let t = self.tmp();
                    self.emit(&format!("{t} = trunc i64 {w0} to {il}"));
                    t
                };
                let c = self.tmp();
                self.emit(&format!("{c} = bitcast {il} {s} to {fl}"));
                c
            }
            Type::Bool => {
                let t = self.tmp();
                self.emit(&format!("{t} = icmp ne i64 {w0}, 0"));
                t
            }
            Type::Str | Type::List(..) | Type::Dict(..) | Type::Named(_) | Type::Func(..) => {
                let t = self.tmp();
                self.emit(&format!("{t} = inttoptr i64 {w0} to ptr"));
                t
            }
            _ => String::new(),
        }
    }

    // ------------------------------------------------------------ nesne düzeni

    pub(super) fn compute_layouts(&mut self) {
        let prog = self.prog;
        for td in &prog.types {
            let mut offs = Vec::new();
            let mut size = 16;
            let mut voffs = Vec::new();
            let mut vsizes = Vec::new();
            match &td.kind {
                TypeDefKind::Tur { fields, .. } => {
                    let mut off = 16;
                    for f in fields {
                        let st = self.stor(&f.ty);
                        off = align(off, st.align);
                        offs.push(off);
                        off += st.size;
                    }
                    size = align(off.max(16), 16);
                }
                TypeDefKind::Secenek { variants } => {
                    for v in variants {
                        let mut off = 24;
                        let mut o = Vec::new();
                        for (_, ft) in &v.fields {
                            let st = self.stor(ft);
                            off = align(off, st.align);
                            o.push(off);
                            off += st.size;
                        }
                        voffs.push(o);
                        vsizes.push(align(off, 16));
                    }
                }
                TypeDefKind::Arayuz { .. } => {}
            }
            self.field_offsets.push(offs);
            self.object_sizes.push(size);
            self.variant_offsets.push(voffs);
            self.variant_sizes.push(vsizes);
        }
    }

    /// `S` tipindeki (statik) nesnenin `fid` alanının adresi. Alan, S'nin bütün alt türlerinde aynı
    /// yerdeyse sabit ofset; değilse (çoklu kalıtım) nesnenin gerçek türüne göre tablodan.
    pub(super) fn field_ptr(&mut self, obj: &str, s: TypeId, fid: u32) -> String {
        let prog = self.prog;
        let mut offs: Vec<(TypeId, i64)> = Vec::new();
        for (t, td) in prog.types.iter().enumerate() {
            if !matches!(td.kind, TypeDefKind::Tur { .. }) || !prog.is_subtype(t, s) {
                continue;
            }
            let idx = prog.field_pos[t].get(fid as usize).copied().unwrap_or(u32::MAX);
            if idx != u32::MAX {
                offs.push((t, self.field_offsets[t][idx as usize]));
            }
        }
        let p = self.tmp();
        if offs.iter().all(|o| o.1 == offs[0].1) {
            self.emit(&format!("{p} = getelementptr i8, ptr {obj}, i64 {}", offs[0].1));
            return p;
        }
        let name = format!("@ofs.{s}.{fid}");
        if !self.tip_defs.contains(&format!("{name} =")) {
            let mut row = vec![0i64; prog.types.len()];
            for (t, o) in &offs {
                row[*t] = *o;
            }
            let items: Vec<String> = row.iter().map(|o| format!("i64 {o}")).collect();
            let _ = writeln!(self.tip_defs, "{name} = internal constant [{} x i64] [{}]", row.len(), items.join(", "));
        }
        let no = self.dyn_type_no(obj);
        let (op, o) = (self.tmp(), self.tmp());
        self.emit(&format!("{op} = getelementptr i64, ptr {name}, i64 {no}"));
        self.emit(&format!("{o} = load i64, ptr {op}"));
        self.emit(&format!("{p} = getelementptr i8, ptr {obj}, i64 {o}"));
        p
    }

    /// Nesnenin gerçek türünün numarası (başlıktaki tip tanımından)
    pub(super) fn dyn_type_no(&mut self, obj: &str) -> String {
        let (tp, t, np, no) = (self.tmp(), self.tmp(), self.tmp(), self.tmp());
        self.emit(&format!("{tp} = getelementptr i8, ptr {obj}, i64 8"));
        self.emit(&format!("{t} = load ptr, ptr {tp}{BASLIK}"));
        self.emit(&format!("{np} = getelementptr i8, ptr {t}, i64 8"));
        self.emit(&format!("{no} = load i64, ptr {np}{BASLIK}"));
        no
    }

    /// Seçenek nesnesinin etiketi (hangi seçenek olduğu)
    pub(super) fn variant_tag(&mut self, obj: &str) -> String {
        let (p, t) = (self.tmp(), self.tmp());
        self.emit(&format!("{p} = getelementptr i8, ptr {obj}, i64 16"));
        self.emit(&format!("{t} = load i64, ptr {p}{BASLIK}"));
        t
    }

    /// Alt tür tablosu: @kolay_altmi[gerçek * n + hedef] = 1
    pub(super) fn subtype_table(&mut self, out: &mut String) {
        let prog = self.prog;
        let n = prog.types.len();
        let mut items = Vec::with_capacity(n * n);
        for c in 0..n {
            for p in 0..n {
                items.push(format!("i8 {}", prog.is_subtype(c, p) as i32));
            }
        }
        let _ = writeln!(out, "@kolay_altmi = constant [{} x i8] [{}]", n * n, items.join(", "));
    }

    /// Nesnenin gerçek türü `hedef`in alt türü mü? (i1)
    pub(super) fn dyn_is_subtype(&mut self, obj: &str, hedef: TypeId) -> String {
        let n = self.prog.types.len();
        let no = self.dyn_type_no(obj);
        let (i, j, p, b, r) = (self.tmp(), self.tmp(), self.tmp(), self.tmp(), self.tmp());
        self.emit(&format!("{i} = mul i64 {no}, {n}"));
        self.emit(&format!("{j} = add i64 {i}, {hedef}"));
        self.emit(&format!("{p} = getelementptr i8, ptr @kolay_altmi, i64 {j}"));
        self.emit(&format!("{b} = load i8, ptr {p}"));
        self.emit(&format!("{r} = icmp ne i8 {b}, 0"));
        r
    }

    // ------------------------------------------------------------ tip tanımları

    fn ptr_array(&mut self, base: &str, items: &[String]) -> String {
        if items.is_empty() {
            return "null".into();
        }
        let name = format!("{base}.d{}", self.tip_defs.len());
        let v: Vec<String> = items.iter().map(|i| format!("ptr {i}")).collect();
        let _ = writeln!(self.tip_defs, "{name} = internal constant [{} x ptr] [{}]", items.len(), v.join(", "));
        name
    }

    fn i64_array(&mut self, base: &str, items: &[i64]) -> String {
        if items.is_empty() {
            return "null".into();
        }
        let name = format!("{base}.o{}", self.tip_defs.len());
        let v: Vec<String> = items.iter().map(|i| format!("i64 {i}")).collect();
        let _ = writeln!(self.tip_defs, "{name} = internal constant [{} x i64] [{}]", items.len(), v.join(", "));
        name
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_tip(
        &mut self,
        name: &str,
        tur: i64,
        no: i64,
        st: &Stor,
        bit: i64,
        isaret: i64,
        sabit: i64,
        e1: &str,
        e2: &str,
        ad: &str,
        n: i64,
        tipler: &str,
        adlar: &str,
        ofsetler: &str,
        varyantlar: &str,
        nesne_boyut: i64,
    ) {
        let ad = self.cstr(ad);
        let _ = writeln!(
            self.tip_defs,
            "{name} = internal constant %Tip {{ i64 {tur}, i64 {no}, i64 {}, i64 {}, i64 {bit}, i64 {isaret}, i64 {sabit}, ptr {e1}, ptr {e2}, ptr {ad}, i64 {n}, ptr {tipler}, ptr {adlar}, ptr {ofsetler}, ptr {varyantlar}, i64 {nesne_boyut} }}",
            st.size, st.align
        );
    }

    /// Tipin çalışma kütüphanesindeki tanımının (Tip) adı
    pub(super) fn tip(&mut self, t: &Type) -> String {
        if let Type::Named(id) = t {
            return format!("@tur.{id}");
        }
        if *t == Type::Str {
            return "@kolay_yazi_tipi".into();
        }
        if let Some(n) = self.tips.get(t) {
            return n.clone();
        }
        let name = format!("@tip.{}", self.tips.len());
        self.tips.insert(t.clone(), name.clone());
        let st = self.stor(t);
        let ad = type_display(t, self.prog);
        match t {
            Type::Int { bits, sign } => {
                let isaret = match sign {
                    Sign::Signed => 0,
                    Sign::Pos => 1,
                    Sign::Neg => 2,
                };
                let tur = if bits.is_some() { T_TAM } else { T_ESNEK };
                self.emit_tip(&name, tur, 0, &st, bits.unwrap_or(0) as i64, isaret, -1, "null", "null", &ad, 0, "null", "null", "null", "null", 0);
            }
            Type::Float(b) => self.emit_tip(&name, T_ONDALIK, 0, &st, *b as i64, 0, -1, "null", "null", &ad, 0, "null", "null", "null", "null", 0),
            Type::Bool => self.emit_tip(&name, T_MANTIK, 0, &st, 0, 0, -1, "null", "null", &ad, 0, "null", "null", "null", "null", 0),
            Type::Hic | Type::Never => self.emit_tip(&name, T_HIC, 0, &st, 0, 0, -1, "null", "null", &ad, 0, "null", "null", "null", "null", 0),
            Type::List(e, size) => {
                let e1 = self.tip(e);
                let sabit = size.map(|n| n as i64).unwrap_or(-1);
                self.emit_tip(&name, T_LISTE, 0, &st, 0, 0, sabit, &e1, "null", &ad, 0, "null", "null", "null", "null", 0);
            }
            Type::Dict(k, v) => {
                let (e1, e2) = (self.tip(k), self.tip(v));
                self.emit_tip(&name, T_SOZLUK, 0, &st, 0, 0, -1, &e1, &e2, &ad, 0, "null", "null", "null", "null", 0);
            }
            Type::Union(ms) => {
                let items: Vec<String> = ms.iter().map(|m| self.tip(m)).collect();
                let arr = self.ptr_array(&name, &items);
                self.emit_tip(&name, T_VEYA, 0, &st, 0, 0, -1, "null", "null", &ad, ms.len() as i64, &arr, "null", "null", "null", 0);
            }
            Type::Func(..) => self.emit_tip(&name, T_FONK, 0, &st, 0, 0, -1, "null", "null", &ad, 0, "null", "null", "null", "null", 0),
            Type::Str | Type::Named(_) => unreachable!(),
        }
        name
    }

    /// Fonksiyon değeri nesnesinin başlık tipi: tür T_FONK, adı değerin tipi, yakalananlar (tipler, ofsetler)
    pub(super) fn emit_fonk_tip(&mut self, name: &str, ty: &Type, tips: &[String], offs: &[i64]) {
        let st = Stor { ll: "ptr".into(), size: 8, align: 8 };
        let ad = type_display(ty, self.prog);
        let tarr = self.ptr_array(name, tips);
        let oarr = self.i64_array(name, offs);
        self.emit_tip(name, T_FONK, 0, &st, 0, 0, -1, "null", "null", &ad, tips.len() as i64, &tarr, "null", &oarr, "null", 0);
    }

    /// Tür / seçenek / arayüz tanımı (@tur.N) ve değer taşımayan seçenekler için sabit nesneler.
    pub(super) fn named_tip(&mut self, tid: TypeId) {
        let prog = self.prog;
        let td = &prog.types[tid];
        let name = format!("@tur.{tid}");
        let st = Stor { ll: "ptr".into(), size: 8, align: 8 };
        match &td.kind {
            TypeDefKind::Tur { fields, .. } => {
                let tips: Vec<String> = fields.iter().map(|f| self.tip(&f.ty)).collect();
                let adlar: Vec<String> = fields.iter().map(|f| self.cstr(&f.name)).collect();
                let offs = self.field_offsets[tid].clone();
                let (tarr, aarr, oarr) = (self.ptr_array(&name, &tips), self.ptr_array(&name, &adlar), self.i64_array(&name, &offs));
                let size = self.object_sizes[tid];
                self.emit_tip(&name, T_TUR, tid as i64, &st, 0, 0, -1, "null", "null", &td.name.clone(), fields.len() as i64, &tarr, &aarr, &oarr, "null", size);
            }
            TypeDefKind::Secenek { variants } => {
                let mut vs = Vec::new();
                for (i, v) in variants.iter().enumerate() {
                    let tips: Vec<String> = v.fields.iter().map(|f| self.tip(&f.1)).collect();
                    let adlar: Vec<String> = v.fields.iter().map(|f| self.cstr(&f.0)).collect();
                    let offs = self.variant_offsets[tid][i].clone();
                    let (tarr, aarr, oarr) = (self.ptr_array(&name, &tips), self.ptr_array(&name, &adlar), self.i64_array(&name, &offs));
                    let sabit = if v.fields.is_empty() {
                        let c = format!("@sabit.{tid}.{i}");
                        let _ = writeln!(self.tip_defs, "{c} = internal global {{ i64, ptr, i64 }} {{ i64 -1, ptr {name}, i64 {i} }}, align 16");
                        c
                    } else {
                        "null".into()
                    };
                    let vad = self.cstr(&v.name);
                    vs.push(format!(
                        "%Varyant {{ ptr {vad}, i64 {}, ptr {aarr}, ptr {tarr}, ptr {oarr}, ptr {sabit}, i64 {} }}",
                        v.fields.len(),
                        self.variant_sizes[tid][i]
                    ));
                }
                let varr = format!("{name}.v");
                let _ = writeln!(self.tip_defs, "{varr} = internal constant [{} x %Varyant] [{}]", vs.len(), vs.join(", "));
                self.emit_tip(&name, T_SECENEK, tid as i64, &st, 0, 0, -1, "null", "null", &td.name.clone(), variants.len() as i64, "null", "null", "null", &varr, 0);
            }
            TypeDefKind::Arayuz { .. } => {
                self.emit_tip(&name, T_ARAYUZ, tid as i64, &st, 0, 0, -1, "null", "null", &td.name.clone(), 0, "null", "null", "null", "null", 0);
            }
        }
    }
}

pub(super) fn align(x: i64, a: i64) -> i64 {
    (x + a - 1) / a * a
}
