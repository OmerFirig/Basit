// Dış (C) kütüphaneler: kütüphaneyi bulma, fonksiyon adresini alma ve yorumlayıcıda libffi ile
// çağırma. Derleyici (codegen) sadece bulma/kontrol kısmını kullanır; çağrıyı LLVM doğrudan yapar.

use crate::diag::{Diag, Span};
use crate::interp::{rt, R};
use crate::tir::*;
use crate::types::{Sign, Type};
use crate::values::*;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_uint, c_void, CStr, CString};

unsafe extern "C" {
    fn dlopen(name: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(h: *mut c_void, name: *const c_char) -> *mut c_void;
}

const RTLD_NOW: c_int = 2;

/// Kütüphane adından açılabilecek dosya adayları: "m" -> libm.so, libm.so.6 ...
fn candidates(name: &str) -> Vec<String> {
    if name.contains('/') || name.contains(".so") {
        return vec![name.to_string()];
    }
    let mut out = vec![format!("lib{name}.so")];
    let prefix = format!("lib{name}.so.");
    for dir in ["/usr/lib", "/usr/lib64", "/lib", "/lib64", "/usr/local/lib", "/usr/lib/x86_64-linux-gnu", "/usr/lib/aarch64-linux-gnu"] {
        if let Ok(rd) = std::fs::read_dir(dir) {
            let mut v: Vec<String> = rd
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.starts_with(&prefix))
                .map(|n| format!("{dir}/{n}"))
                .collect();
            v.sort();
            out.extend(v);
        }
    }
    out
}

fn open_lib(name: &str) -> Option<*mut c_void> {
    for c in candidates(name) {
        let cs = CString::new(c).ok()?;
        let h = unsafe { dlopen(cs.as_ptr(), RTLD_NOW) };
        if !h.is_null() {
            return Some(h);
        }
    }
    None
}

fn diag(loc: Loc, msg: String) -> Diag {
    Diag::new(Span { file: loc.file, line: loc.line, col: 0, start: 0, end: 0 }, msg)
}

/// Bütün dış fonksiyonların kütüphanesini ve adresini bulur (program çalışmadan / derlenmeden önce).
/// Dönüş: fonksiyon numarası -> adres (dış olmayanlar için 0).
pub fn resolve(prog: &Program) -> Result<Vec<usize>, Diag> {
    let mut libs: HashMap<String, *mut c_void> = HashMap::new();
    let mut seen: HashMap<String, (Vec<Type>, Type)> = HashMap::new();
    let mut out = vec![0usize; prog.funcs.len()];
    for (fid, f) in prog.funcs.iter().enumerate() {
        let Some(d) = &f.dis else { continue };
        let h = match libs.get(&d.lib) {
            Some(h) => *h,
            None => {
                let Some(h) = open_lib(&d.lib) else {
                    return Err(diag(d.lib_loc, format!("\"{}\" kütüphanesi bulunamadı (aranan: lib{}.so).", d.lib, d.lib)));
                };
                libs.insert(d.lib.clone(), h);
                h
            }
        };
        let sym = CString::new(d.symbol.clone()).map_err(|_| diag(f.loc, "C fonksiyon adında geçersiz karakter var.".into()))?;
        let p = unsafe { dlsym(h, sym.as_ptr()) };
        if p.is_null() {
            return Err(diag(f.loc, format!("\"{}\" kütüphanesindeki '{}' fonksiyonu bulunamadı.", d.lib, d.symbol)));
        }
        let sig = (f.params.clone(), f.ret.clone());
        if let Some(old) = seen.get(&d.symbol) {
            if *old != sig {
                return Err(diag(f.loc, format!("'{}' C fonksiyonu iki farklı imzayla tanımlanmış.", d.symbol)));
            }
        }
        seen.insert(d.symbol.clone(), sig);
        out[fid] = p as usize;
    }
    Ok(out)
}

// ------------------------------------------------------------ libffi

#[repr(C)]
struct FfiType {
    size: usize,
    alignment: u16,
    type_: u16,
    elements: *mut *mut FfiType,
}

#[repr(C)]
struct FfiCif {
    abi: c_int,
    nargs: c_uint,
    arg_types: *mut *mut FfiType,
    rtype: *mut FfiType,
    bytes: c_uint,
    flags: c_uint,
    extra: [u64; 4],
}

#[link(name = "ffi")]
unsafe extern "C" {
    static mut ffi_type_void: FfiType;
    static mut ffi_type_uint8: FfiType;
    static mut ffi_type_sint8: FfiType;
    static mut ffi_type_uint16: FfiType;
    static mut ffi_type_sint16: FfiType;
    static mut ffi_type_uint32: FfiType;
    static mut ffi_type_sint32: FfiType;
    static mut ffi_type_uint64: FfiType;
    static mut ffi_type_sint64: FfiType;
    static mut ffi_type_float: FfiType;
    static mut ffi_type_double: FfiType;
    static mut ffi_type_pointer: FfiType;
    fn ffi_prep_cif(cif: *mut FfiCif, abi: c_int, nargs: c_uint, rtype: *mut FfiType, atypes: *mut *mut FfiType) -> c_int;
    fn ffi_call(cif: *mut FfiCif, f: *mut c_void, rvalue: *mut c_void, avalue: *mut *mut c_void);
}

#[cfg(target_arch = "x86_64")]
const FFI_DEFAULT_ABI: c_int = 2;
#[cfg(not(target_arch = "x86_64"))]
const FFI_DEFAULT_ABI: c_int = 1;

fn ffi_type(t: &Type) -> *mut FfiType {
    match t {
        Type::Int { bits: Some(8), sign: Sign::Pos } => &raw mut ffi_type_uint8,
        Type::Int { bits: Some(8), .. } => &raw mut ffi_type_sint8,
        Type::Int { bits: Some(16), sign: Sign::Pos } => &raw mut ffi_type_uint16,
        Type::Int { bits: Some(16), .. } => &raw mut ffi_type_sint16,
        Type::Int { bits: Some(32), sign: Sign::Pos } => &raw mut ffi_type_uint32,
        Type::Int { bits: Some(32), .. } => &raw mut ffi_type_sint32,
        Type::Int { bits: Some(64), sign: Sign::Pos } => &raw mut ffi_type_uint64,
        Type::Int { .. } => &raw mut ffi_type_sint64,
        Type::Float(32) => &raw mut ffi_type_float,
        Type::Float(_) => &raw mut ffi_type_double,
        Type::Bool => &raw mut ffi_type_uint8,
        Type::Str => &raw mut ffi_type_pointer,
        _ => &raw mut ffi_type_void,
    }
}

/// Dış fonksiyonu yorumlayıcıdan çağırır.
pub fn call(addr: usize, params: &[Type], ret: &Type, args: &[Value], loc: Loc) -> R<Value> {
    let mut cstrs: Vec<CString> = Vec::new();
    let mut store: Vec<[u64; 2]> = vec![[0; 2]; args.len()];
    for (i, (v, t)) in args.iter().zip(params).enumerate() {
        let slot = &mut store[i];
        match (v, t) {
            (Value::Int(n), Type::Int { sign, .. }) => {
                let x: u64 = match n {
                    Int::S(x) => *x as u64,
                    Int::B(b) => {
                        use num_traits::ToPrimitive;
                        if *sign == Sign::Pos { b.to_u64().unwrap_or(0) } else { b.to_i64().unwrap_or(0) as u64 }
                    }
                };
                slot[0] = x; // küçük tipler: düşük baytlar (little-endian)
            }
            (Value::Float(x), Type::Float(32)) => slot[0] = (*x as f32).to_bits() as u64,
            (Value::Float(x), _) => slot[0] = x.to_bits(),
            (Value::Bool(b), _) => slot[0] = *b as u64,
            (Value::Str(s), _) => {
                let Ok(c) = CString::new(s.as_bytes()) else {
                    return rt(loc, "C fonksiyonuna verilen yazının içinde boş karakter (\\0) var.");
                };
                slot[0] = c.as_ptr() as u64;
                cstrs.push(c);
            }
            _ => unreachable!("C'ye verilemeyen değer"),
        }
    }
    let mut atypes: Vec<*mut FfiType> = params.iter().map(ffi_type).collect();
    let mut avalues: Vec<*mut c_void> = store.iter_mut().map(|s| s.as_mut_ptr() as *mut c_void).collect();
    let mut cif = FfiCif {
        abi: 0,
        nargs: 0,
        arg_types: std::ptr::null_mut(),
        rtype: std::ptr::null_mut(),
        bytes: 0,
        flags: 0,
        extra: [0; 4],
    };
    let mut rv: [u64; 2] = [0; 2];
    unsafe {
        let st = ffi_prep_cif(&mut cif, FFI_DEFAULT_ABI, params.len() as c_uint, ffi_type(ret), atypes.as_mut_ptr());
        if st != 0 {
            return rt(loc, "C fonksiyonu çağrılamadı (imza hazırlanamadı).");
        }
        ffi_call(&mut cif, addr as *mut c_void, rv.as_mut_ptr() as *mut c_void, avalues.as_mut_ptr());
    }
    drop(cstrs);
    Ok(match ret {
        Type::Hic => Value::Hic,
        Type::Int { bits: Some(b), sign } => {
            let raw = rv[0];
            let v: i128 = match (b, sign) {
                (8, Sign::Pos) => raw as u8 as i128,
                (8, _) => raw as i8 as i128,
                (16, Sign::Pos) => raw as u16 as i128,
                (16, _) => raw as i16 as i128,
                (32, Sign::Pos) => raw as u32 as i128,
                (32, _) => raw as i32 as i128,
                (_, Sign::Pos) => raw as i128,
                _ => raw as i64 as i128,
            };
            Value::Int(Int::from_i128(v))
        }
        Type::Float(32) => Value::Float(f32::from_bits(rv[0] as u32) as f64),
        Type::Float(_) => Value::Float(f64::from_bits(rv[0])),
        Type::Bool => Value::Bool(rv[0] as u8 != 0),
        Type::Str => {
            let p = rv[0] as *const c_char;
            if p.is_null() {
                Value::str("")
            } else {
                Value::string(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
            }
        }
        _ => Value::Hic,
    })
}
