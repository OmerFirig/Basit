// Kolay'ın tipleri ve tipler arası kurallar (genişletme, birleştirme, sınırlar).

use num_bigint::BigInt;
use num_traits::{One, Zero};

pub type TypeId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sign {
    Signed,
    Pos,
    Neg,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    /// bits: None = esnek
    Int { bits: Option<u32>, sign: Sign },
    Float(u32),
    Str,
    Bool,
    Hic,
    List(Box<Type>, Option<usize>),
    Dict(Box<Type>, Box<Type>),
    Union(Vec<Type>),
    Named(TypeId),
    /// Hiçbir zaman değer üretmeyen ifadeler (dur, döndür...)
    Never,
}

pub const TAM64: Type = Type::Int { bits: Some(64), sign: Sign::Signed };
pub const ESNEK: Type = Type::Int { bits: None, sign: Sign::Signed };

impl Type {
    pub fn is_int(&self) -> bool {
        matches!(self, Type::Int { .. })
    }
    pub fn is_float(&self) -> bool {
        matches!(self, Type::Float(_))
    }
    pub fn is_num(&self) -> bool {
        self.is_int() || self.is_float()
    }
    pub fn list(elem: Type) -> Type {
        Type::List(Box::new(elem), None)
    }
    pub fn contains_hic(&self) -> bool {
        match self {
            Type::Hic => true,
            Type::Union(v) => v.contains(&Type::Hic),
            _ => false,
        }
    }
    pub fn members(&self) -> Vec<Type> {
        match self {
            Type::Union(v) => v.clone(),
            t => vec![t.clone()],
        }
    }
}

/// Bir tam sayı tipinin alabileceği en küçük ve en büyük değer (None = sınırsız).
pub fn int_bounds(bits: Option<u32>, sign: Sign) -> (Option<BigInt>, Option<BigInt>) {
    let Some(b) = bits else {
        return match sign {
            Sign::Signed => (None, None),
            Sign::Pos => (Some(BigInt::zero()), None),
            Sign::Neg => (None, Some(BigInt::zero())),
        };
    };
    let one = BigInt::one();
    match sign {
        Sign::Signed => {
            let half = &one << (b - 1);
            (Some(-half.clone()), Some(half - 1))
        }
        Sign::Pos => (Some(BigInt::zero()), Some((&one << b) - 1)),
        Sign::Neg => {
            let m: BigInt = (&one << b) - 1;
            (Some(-m), Some(BigInt::zero()))
        }
    }
}

pub fn fits(n: &BigInt, bits: Option<u32>, sign: Sign) -> bool {
    let (lo, hi) = int_bounds(bits, sign);
    lo.is_none_or(|lo| *n >= lo) && hi.is_none_or(|hi| *n <= hi)
}

/// `from` tipindeki her değer `to` tipine sığar mı?
pub fn int_subset(from: (Option<u32>, Sign), to: (Option<u32>, Sign)) -> bool {
    let (flo, fhi) = int_bounds(from.0, from.1);
    let (tlo, thi) = int_bounds(to.0, to.1);
    let lo_ok = match (flo, tlo) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(f), Some(t)) => f >= t,
    };
    let hi_ok = match (fhi, thi) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(f), Some(t)) => f <= t,
    };
    lo_ok && hi_ok
}

/// İki tam sayı tipi arasındaki işlemin sonuç tipi: ikisini de kapsayan en küçük tip.
pub fn int_join(a: (Option<u32>, Sign), b: (Option<u32>, Sign)) -> (Option<u32>, Sign) {
    if int_subset(a, b) {
        return b;
    }
    if int_subset(b, a) {
        return a;
    }
    if a.0.is_none() || b.0.is_none() {
        return (None, Sign::Signed);
    }
    let mut bits = a.0.unwrap().max(b.0.unwrap());
    for candidate in [8, 16, 32, 64, 128] {
        if candidate >= bits && int_subset(a, (Some(candidate), Sign::Signed)) && int_subset(b, (Some(candidate), Sign::Signed)) {
            return (Some(candidate), Sign::Signed);
        }
        bits = bits.max(candidate);
    }
    (None, Sign::Signed)
}

pub fn bounds_text(bits: Option<u32>, sign: Sign) -> String {
    let (lo, hi) = int_bounds(bits, sign);
    let f = |x: Option<BigInt>, neg: bool| match x {
        Some(v) => group_digits(&v.to_string()),
        None if neg => "-sonsuz".into(),
        None => "sonsuz".into(),
    };
    format!("{} .. {}", f(lo, true), f(hi, false))
}

pub fn group_digits(s: &str) -> String {
    let (neg, digits) = match s.strip_prefix('-') {
        Some(d) => (true, d),
        None => (false, s),
    };
    if digits.len() <= 4 {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    if neg { format!("-{out}") } else { out }
}

/// Union tipini düzleştirir, tekrarları siler.
pub fn make_union(parts: Vec<Type>) -> Type {
    let mut out: Vec<Type> = Vec::new();
    for p in parts {
        for m in p.members() {
            if m == Type::Never {
                continue;
            }
            if !out.contains(&m) {
                out.push(m);
            }
        }
    }
    match out.len() {
        0 => Type::Never,
        1 => out.pop().unwrap(),
        _ => Type::Union(out),
    }
}

/// Ondalık tipinin üst sınırı (hata mesajları için)
pub fn float_limit_text(bits: u32) -> &'static str {
    match bits {
        16 => "en büyük değer 65.504",
        32 => "en büyük değer yaklaşık 3,4 × 10^38",
        _ => "en büyük değer yaklaşık 1,8 × 10^308",
    }
}
