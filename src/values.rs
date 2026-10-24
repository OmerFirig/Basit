// Çalışma anındaki değerler, yazıya çevirme, karşılaştırma ve Türkçe harf kuralları.

use crate::tir::{Program, TypeDefKind};
use crate::types::{Sign, Type, TypeId};
use indexmap::IndexMap;
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};
use std::cmp::Ordering;
use std::rc::Rc;

/// Tam sayı: 64 bite sığan değerler doğrudan, sığmayanlar büyük sayı olarak tutulur.
#[derive(Clone, Debug)]
pub enum Int {
    S(i64),
    B(Rc<BigInt>),
}

impl Int {
    pub fn from_big(b: BigInt) -> Int {
        match b.to_i64() {
            Some(v) => Int::S(v),
            None => Int::B(Rc::new(b)),
        }
    }
    pub fn from_i128(v: i128) -> Int {
        match i64::try_from(v) {
            Ok(x) => Int::S(x),
            Err(_) => Int::B(Rc::new(BigInt::from(v))),
        }
    }
    pub fn to_big(&self) -> BigInt {
        match self {
            Int::S(v) => BigInt::from(*v),
            Int::B(b) => (**b).clone(),
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Int::S(v) => Some(*v),
            Int::B(b) => b.to_i64(),
        }
    }
    pub fn to_f64(&self) -> f64 {
        match self {
            Int::S(v) => *v as f64,
            Int::B(b) => b.to_f64().unwrap_or(f64::INFINITY),
        }
    }
    pub fn is_zero(&self) -> bool {
        match self {
            Int::S(v) => *v == 0,
            Int::B(b) => b.is_zero(),
        }
    }
    pub fn cmp_int(&self, o: &Int) -> Ordering {
        match (self, o) {
            (Int::S(a), Int::S(b)) => a.cmp(b),
            _ => self.to_big().cmp(&o.to_big()),
        }
    }
    /// Büyük sayı ayırmadan karşılaştırma (eşleştir desenleri için).
    pub fn eq_big(&self, n: &BigInt) -> bool {
        match self {
            Int::S(v) => n.to_i64() == Some(*v),
            Int::B(b) => **b == *n,
        }
    }
    pub fn cmp_big(&self, n: &BigInt) -> Ordering {
        match (self, n.to_i64()) {
            (Int::S(v), Some(m)) => v.cmp(&m),
            _ => self.to_big().cmp(n),
        }
    }
}

impl std::fmt::Display for Int {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Int::S(v) => write!(f, "{v}"),
            Int::B(b) => write!(f, "{b}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ListV {
    pub items: Vec<Value>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Key {
    Int(i64),
    Big(BigInt),
    Str(Rc<String>),
    Bool(bool),
    Var(TypeId, u32, Vec<Key>),
}

#[derive(Clone, Debug, Default)]
pub struct DictV {
    pub map: IndexMap<Key, (Value, Value)>,
}

#[derive(Clone, Debug)]
pub struct ObjV {
    pub ty: TypeId,
    /// Alanlar, türün alan sırasıyla (Program::types[ty] içindeki `fields`).
    pub fields: Vec<Value>,
}

#[derive(Clone, Debug)]
pub struct VarV {
    pub ty: TypeId,
    pub idx: u32,
    pub fields: Vec<Value>,
}

/// `a..b` / `a..=b` / `(a..b).adım(n)` listesinin elemanları: ilk, ilk + adım, ... (`adet` tane).
/// Liste oluşturmadan saymak için (döngü) ve listeyi oluşturmak için aynı hesap kullanılır;
/// derleyicideki (codegen/komut.rs range_plan) ile aynı kural.
#[derive(Clone, Copy, Debug)]
pub struct RangePlan {
    pub first: i64,
    pub step: i64,
    pub count: u64,
}

/// `n`: `.adım(n)` (yoksa 1). Başlangıç bitişten büyükse liste geri sayar (5..0 -> 5, 4, 3, 2, 1).
/// Eksi adım listenin sonundan başlar (liste.adım ile aynı). Adım 0 ise None.
pub fn range_plan(s: i64, e: i64, inclusive: bool, n: i64) -> Option<RangePlan> {
    if n == 0 {
        return None;
    }
    let up = s <= e;
    let dir: i64 = if up { 1 } else { -1 };
    let dist = if up { (e as u64).wrapping_sub(s as u64) } else { (s as u64).wrapping_sub(e as u64) };
    let len = if inclusive { dist.saturating_add(1) } else { dist };
    let k = n.unsigned_abs();
    let count = if len == 0 { 0 } else { (len - 1) / k + 1 };
    // elemanların gerçek değeri her zaman Tam(64)'e sığar; ara hesap sarmalı (wrapping) yapılabilir
    let first = if n > 0 { s } else { s.wrapping_add(((len.wrapping_sub(1)) as i64).wrapping_mul(dir)) };
    Some(RangePlan { first, step: dir.wrapping_mul(n), count })
}

impl RangePlan {
    pub fn get(&self, i: u64) -> i64 {
        self.first.wrapping_add((i as i64).wrapping_mul(self.step))
    }
}

/// Liste ya da yazı diliminin indeksleri (runtime/koleksiyon.c kolay_liste_dilim ile aynı kural).
/// Eksi indeksler önce sondan sayılır; iki sınır da yazılmışsa ve başlangıç bitişten büyükse dilim
/// terstir (l[4..1] -> 4, 3, 2). Sınırlar kırpılır. Dönüş: (ilk, son, ters mi); düzde ilk..son,
/// tersde ilk'ten son'a doğru (ikisi de dahil).
pub fn slice_bounds(len: usize, start: Option<i64>, end: Option<i64>, inclusive: bool) -> Option<(usize, usize, bool)> {
    let n = len as i64;
    let norm = |i: i64| if i < 0 { i.saturating_add(n) } else { i };
    let s = norm(start.unwrap_or(0));
    let e = end.map(norm);
    match e {
        Some(e) if start.is_some() && s > e => {
            let hi = s.min(n - 1);
            let lo = (if inclusive { e } else { e.saturating_add(1) }).max(0);
            (hi >= lo).then_some((hi as usize, lo as usize, true))
        }
        _ => {
            let a = s.clamp(0, n);
            let b = match e {
                Some(e) => (if inclusive { e.saturating_add(1) } else { e }).clamp(0, n),
                None => n,
            };
            (a < b).then_some((a as usize, b as usize, false))
        }
    }
}

/// Dilimi uygular (liste elemanları ya da yazının karakterleri).
pub fn apply_slice<T: Clone>(items: &[T], b: Option<(usize, usize, bool)>) -> Vec<T> {
    match b {
        None => vec![],
        Some((a, b, false)) => items[a..b].to_vec(),
        Some((hi, lo, true)) => items[lo..=hi].iter().rev().cloned().collect(),
    }
}

/// Fonksiyon değeri: lambda (yakaladığı değerlerle) ya da isimli fonksiyon
#[derive(Debug, Clone)]
pub struct FuncV {
    pub fid: usize,
    pub caps: Vec<Value>,
    /// metot içinde oluşturulmuş ve nesneyi kullanan lambda: nesnenin kopyası
    pub bu: Option<Value>,
}

#[derive(Clone, Debug)]
pub enum Value {
    Int(Int),
    Float(f64),
    Bool(bool),
    Str(Rc<String>),
    Hic,
    List(Rc<ListV>),
    Dict(Rc<DictV>),
    Obj(Rc<ObjV>),
    Var(Rc<VarV>),
    Func(Rc<FuncV>),
    Uninit,
}

impl Value {
    pub fn int(v: i64) -> Value {
        Value::Int(Int::S(v))
    }
    pub fn str(s: &str) -> Value {
        Value::Str(Rc::new(s.to_string()))
    }
    pub fn string(s: String) -> Value {
        Value::Str(Rc::new(s))
    }
    pub fn key(&self) -> Option<Key> {
        Some(match self {
            Value::Int(Int::S(i)) => Key::Int(*i),
            Value::Int(Int::B(b)) => Key::Big((**b).clone()),
            Value::Str(s) => Key::Str(s.clone()),
            Value::Bool(b) => Key::Bool(*b),
            Value::Var(v) => {
                let mut fs = Vec::with_capacity(v.fields.len());
                for f in &v.fields {
                    fs.push(f.key()?);
                }
                Key::Var(v.ty, v.idx, fs)
            }
            _ => return None,
        })
    }
    pub fn as_f64(&self) -> f64 {
        match self {
            Value::Int(i) => i.to_f64(),
            Value::Float(f) => *f,
            _ => 0.0,
        }
    }
}

pub fn round_float(f: f64, bits: u32) -> f64 {
    match bits {
        32 => f as f32 as f64,
        16 => half::f16::from_f64(f).to_f64(),
        _ => f,
    }
}

pub fn fits_int(v: &Int, bits: Option<u32>, sign: Sign) -> bool {
    match v {
        Int::S(x) => {
            let x = *x;
            match (bits, sign) {
                (None, Sign::Signed) => true,
                (None, Sign::Pos) => x >= 0,
                (None, Sign::Neg) => x <= 0,
                (Some(b), Sign::Signed) => {
                    if b >= 64 {
                        true
                    } else {
                        let half = 1i64 << (b - 1);
                        x >= -half && x < half
                    }
                }
                (Some(b), Sign::Pos) => x >= 0 && (b >= 63 || x < (1i64 << b)),
                (Some(b), Sign::Neg) => x <= 0 && (b >= 63 || x > -(1i64 << b)),
            }
        }
        Int::B(b) => crate::types::fits(b, bits, sign),
    }
}

// ------------------------------------------------------------ Türkçe harfler

pub fn tr_lower_char(c: char) -> char {
    match c {
        'I' => 'ı',
        'İ' => 'i',
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

pub fn tr_upper(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'i' => out.push('İ'),
            'ı' => out.push('I'),
            _ => out.extend(c.to_uppercase()),
        }
    }
    out
}

pub fn tr_lower(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'I' => out.push('ı'),
            'İ' => out.push('i'),
            _ => out.extend(c.to_lowercase()),
        }
    }
    out
}

/// Türk alfabesindeki sırası (a=0 ... z=31); alfabede olmayan harfler için None.
fn alphabet_pos(c: char) -> Option<u32> {
    Some(match c {
        'a' => 0,
        'b' => 1,
        'c' => 2,
        'ç' => 3,
        'd' => 4,
        'e' => 5,
        'f' => 6,
        'g' => 7,
        'ğ' => 8,
        'h' => 9,
        'ı' => 10,
        'i' => 11,
        'j' => 12,
        'k' => 13,
        'l' => 14,
        'm' => 15,
        'n' => 16,
        'o' => 17,
        'ö' => 18,
        'p' => 19,
        'q' => 20,
        'r' => 21,
        's' => 22,
        'ş' => 23,
        't' => 24,
        'u' => 25,
        'ü' => 26,
        'v' => 27,
        'w' => 28,
        'x' => 29,
        'y' => 30,
        'z' => 31,
        _ => return None,
    })
}

/// (birincil, ikincil): birincil harfin alfabedeki yeri, ikincil büyük/küçük farkı.
fn tr_key(c: char) -> (u32, u32) {
    let low = tr_lower_char(c);
    match alphabet_pos(low) {
        Some(i) => (1000 + i, if c == low { 1 } else { 0 }),
        None => (c as u32, 0),
    }
}

/// Sıralama anahtarı: önce harfler (büyük/küçük ayrımı olmadan), eşitse büyük/küçük farkı.
pub fn tr_sort_key(s: &str) -> (Vec<u32>, Vec<u32>) {
    let mut p = Vec::with_capacity(s.len());
    let mut q = Vec::with_capacity(s.len());
    for c in s.chars() {
        let (a, b) = tr_key(c);
        p.push(a);
        q.push(b);
    }
    (p, q)
}

/// Türk alfabesine göre karşılaştırma (c < ç < d, ı < i, o < ö ...).
pub fn tr_cmp(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().map(tr_key);
    let mut bi = b.chars().map(tr_key);
    let mut second = Ordering::Equal;
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return second,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                if x.0 != y.0 {
                    return x.0.cmp(&y.0);
                }
                if second == Ordering::Equal {
                    second = x.1.cmp(&y.1);
                }
            }
        }
    }
}

// ------------------------------------------------------------ eşitlik ve sıralama

pub fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp_int(y) == Ordering::Equal,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Int(x), Value::Float(y)) | (Value::Float(y), Value::Int(x)) => x.to_f64() == *y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Hic, Value::Hic) => true,
        (Value::List(x), Value::List(y)) => {
            x.items.len() == y.items.len() && x.items.iter().zip(&y.items).all(|(a, b)| values_eq(a, b))
        }
        (Value::Dict(x), Value::Dict(y)) => {
            x.map.len() == y.map.len()
                && x.map.iter().all(|(k, (_, v))| y.map.get(k).is_some_and(|(_, w)| values_eq(v, w)))
        }
        (Value::Obj(x), Value::Obj(y)) => {
            x.ty == y.ty && x.fields.len() == y.fields.len() && x.fields.iter().zip(&y.fields).all(|(a, b)| values_eq(a, b))
        }
        (Value::Func(x), Value::Func(y)) => Rc::ptr_eq(x, y),
        (Value::Var(x), Value::Var(y)) => {
            x.ty == y.ty && x.idx == y.idx && x.fields.iter().zip(&y.fields).all(|(a, b)| values_eq(a, b))
        }
        _ => false,
    }
}

/// Sıralama için tam düzen: TanımsızSayı (NaN) her sayıdan büyük sayılır ve sona gider.
pub fn cmp_values(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp_int(y),
        (Value::Str(x), Value::Str(y)) => tr_cmp(x, y),
        _ => {
            let (x, y) = (a.as_f64(), b.as_f64());
            x.partial_cmp(&y).unwrap_or_else(|| x.is_nan().cmp(&y.is_nan()))
        }
    }
}

// ------------------------------------------------------------ yazıya çevirme

/// Ondalık sayıyı yazıya çevirir (Python'daki gibi): en kısa geri dönüşen basamaklar; üs -4 ile 15
/// arasındaysa düz (0.0001, 123.5, 1e15 -> 1000000000000000.0), dışındaysa bilimsel (1e-05, 6.022e+23).
/// Derlenmiş programda: runtime/sayilar.c ondalik_yaz (aynı sonucu vermeli).
pub fn fmt_float(f: f64, bits: u32) -> String {
    if f.is_nan() {
        return "TanımsızSayı".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "sonsuz".into() } else { "-sonsuz".into() };
    }
    if bits == 16 {
        let h = half::f16::from_f64(f).to_f64();
        if h != 0.0 && h.abs() < 1e-4 {
            // küçük Ondalık(16): en kısa bilimsel gösterim
            let sci = (1..=8)
                .map(|p| format!("{:.*e}", p - 1, h))
                .find(|t| t.parse::<f64>().map(half::f16::from_f64) == Ok(half::f16::from_f64(h)))
                .unwrap_or_else(|| format!("{h:e}"));
            return ondalik_bicim(&sci);
        }
        // Ondalık(16) düz yazımda en kısa geri dönüşen sabit basamaklı gösterim (65504.0 gibi)
        return (1..=8)
            .map(|p| format!("{:.*}", p, h))
            .find(|t| t.parse::<f64>().map(half::f16::from_f64) == Ok(half::f16::from_f64(h)))
            .map(|t| {
                let t = t.trim_end_matches('0');
                if t.ends_with('.') { format!("{t}0") } else { t.to_string() }
            })
            .unwrap_or_else(|| format!("{h}"));
    }
    let sci = if bits == 32 { format!("{:e}", f as f32) } else { format!("{f:e}") };
    ondalik_bicim(&sci)
}

/// Rust'ın en kısa bilimsel gösteriminden ("-1.5e-7") Python biçimi
fn ondalik_bicim(sci: &str) -> String {
    let (mant, us) = sci.split_once('e').expect("bilimsel gösterim");
    let us: i32 = us.parse().expect("üs");
    let (isaret, mant) = match mant.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mant),
    };
    let mut basamak: String = mant.chars().filter(|c| *c != '.').collect();
    while basamak.len() > 1 && basamak.ends_with('0') {
        basamak.pop();
    }
    if !(-4..16).contains(&us) {
        let kesir = if basamak.len() > 1 { format!(".{}", &basamak[1..]) } else { String::new() };
        return format!("{isaret}{}{kesir}e{}{:02}", &basamak[..1], if us < 0 { '-' } else { '+' }, us.abs());
    }
    if us >= 0 {
        let u = us as usize + 1;
        let tam: String = (0..u).map(|i| basamak.as_bytes().get(i).map_or('0', |b| *b as char)).collect();
        let kesir = if basamak.len() > u { &basamak[u..] } else { "0" };
        format!("{isaret}{tam}.{kesir}")
    } else {
        format!("{isaret}0.{}{basamak}", "0".repeat((-us - 1) as usize))
    }
}

pub fn type_display(t: &Type, prog: &Program) -> String {
    match t {
        Type::Int { bits, sign } => {
            let s = match sign {
                Sign::Signed => None,
                Sign::Pos => Some("+"),
                Sign::Neg => Some("-"),
            };
            match (bits, s) {
                (None, None) => "Tam".into(),
                (None, Some(s)) => format!("Tam({s})"),
                (Some(b), None) => format!("Tam({b})"),
                (Some(b), Some(s)) => format!("Tam({b}, {s})"),
            }
        }
        Type::Float(64) => "Ondalık".into(),
        Type::Float(b) => format!("Ondalık({b})"),
        Type::Str => "Yazı".into(),
        Type::Bool => "Mantık".into(),
        Type::Hic | Type::Never => "Hiç".into(),
        Type::List(e, Some(n)) => format!("Liste<{}>[{n}]", type_display(e, prog)),
        Type::List(e, None) => format!("Liste<{}>", type_display(e, prog)),
        Type::Dict(k, v) => format!("Sözlük<{}, {}>", type_display(k, prog), type_display(v, prog)),
        Type::Union(v) => v.iter().map(|t| type_display(t, prog)).collect::<Vec<_>>().join(" veya "),
        Type::Named(id) => prog.types[*id].name.clone(),
        Type::Func(ps, r) => {
            let ps: Vec<String> = ps.iter().map(|t| type_display(t, prog)).collect();
            if **r == Type::Hic {
                format!("Fonksiyon({})", ps.join(", "))
            } else {
                format!("Fonksiyon({}) -> {}", ps.join(", "), type_display(r, prog))
            }
        }
    }
}

pub fn kind_name(v: &Value, prog: &Program) -> String {
    match v {
        Value::Int(_) => "Tam".into(),
        Value::Float(_) => "Ondalık".into(),
        Value::Bool(_) => "Mantık".into(),
        Value::Str(_) => "Yazı".into(),
        Value::Hic => "Hiç".into(),
        Value::List(_) => "Liste".into(),
        Value::Dict(_) => "Sözlük".into(),
        Value::Obj(o) => prog.types[o.ty].name.clone(),
        Value::Var(v) => prog.types[v.ty].name.clone(),
        Value::Func(_) => "Fonksiyon".into(),
        Value::Uninit => "belirsiz".into(),
    }
}

fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
}

pub fn fmt_value(v: &Value, ty: Option<&Type>, prog: &Program, top: bool) -> String {
    let mut out = String::new();
    write_value(&mut out, v, ty, prog, top);
    out
}

fn join_into<T>(out: &mut String, items: impl IntoIterator<Item = T>, mut f: impl FnMut(&mut String, T)) {
    for (i, x) in items.into_iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        f(out, x);
    }
}

pub fn write_value(out: &mut String, v: &Value, ty: Option<&Type>, prog: &Program, top: bool) {
    use std::fmt::Write as _;
    // 'veya' tipinde değerin kendi üyesinin tipi kullanılır (Ondalık(32) üye 0.1 diye yazılsın).
    // Bir 'veya' tipinde en fazla bir ondalık üye olabilir; iki liste üyesi varsa (Liste<Tam> veya
    // Liste<Yazı>) ilki kullanılır, bu da sadece ondalık elemanların yazılışını etkiler.
    if let Some(Type::Union(ms)) = ty {
        let m = ms.iter().find(|m| match (v, m) {
            (Value::Float(_), Type::Float(_)) | (Value::List(_), Type::List(..)) | (Value::Dict(_), Type::Dict(..)) | (Value::Func(_), Type::Func(..)) => true,
            _ => false,
        });
        if let Some(m) = m {
            return write_value(out, v, Some(m), prog, top);
        }
    }
    match v {
        Value::Int(i) => {
            let _ = write!(out, "{i}");
        }
        Value::Float(f) => {
            let bits = match ty {
                Some(Type::Float(b)) => *b,
                _ => 64,
            };
            out.push_str(&fmt_float(*f, bits));
        }
        Value::Bool(b) => out.push_str(if *b { "doğru" } else { "yanlış" }),
        Value::Str(s) => {
            if top {
                out.push_str(s)
            } else {
                quote(s, out)
            }
        }
        Value::Hic => out.push_str("Hiç"),
        Value::List(l) => {
            let h = match ty {
                Some(Type::List(e, _)) => Some(&**e),
                _ => None,
            };
            out.push('[');
            join_into(out, &l.items, |o, x| write_value(o, x, h, prog, false));
            out.push(']');
        }
        Value::Dict(d) => {
            let (kh, vh) = match ty {
                Some(Type::Dict(k, v)) => (Some(&**k), Some(&**v)),
                _ => (None, None),
            };
            out.push('{');
            join_into(out, d.map.values(), |o, (k, v)| {
                write_value(o, k, kh, prog, false);
                o.push_str(": ");
                write_value(o, v, vh, prog, false);
            });
            out.push('}');
        }
        Value::Obj(o) => {
            let td = &prog.types[o.ty];
            out.push_str(&td.name);
            out.push('(');
            if let TypeDefKind::Tur { fields, .. } = &td.kind {
                join_into(out, fields.iter().zip(&o.fields), |s, (fi, v)| {
                    s.push_str(&fi.name);
                    s.push_str(": ");
                    write_value(s, v, Some(&fi.ty), prog, false);
                });
            }
            out.push(')');
        }
        Value::Var(x) => {
            let td = &prog.types[x.ty];
            let TypeDefKind::Secenek { variants } = &td.kind else {
                out.push_str(&td.name);
                return;
            };
            let vi = &variants[x.idx as usize];
            out.push_str(&td.name);
            out.push('.');
            out.push_str(&vi.name);
            if !vi.fields.is_empty() {
                out.push('(');
                join_into(out, vi.fields.iter().zip(&x.fields), |s, ((n, t), v)| {
                    s.push_str(n);
                    s.push_str(": ");
                    write_value(s, v, Some(t), prog, false);
                });
                out.push(')');
            }
        }
        Value::Func(_) => match ty {
            Some(t @ Type::Func(..)) => out.push_str(&type_display(t, prog)),
            _ => out.push_str("Fonksiyon"),
        },
        Value::Uninit => out.push_str("<belirsiz>"),
    }
}
