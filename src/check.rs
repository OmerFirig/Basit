// Tip denetleyici: isimleri çözer, tipleri kontrol eder ve TIR üretir.
// Bu dosya: genel yapı, tanımlar, tip çözümleme, kalıtım ve arayüzler.
// İfadeler `cexpr.rs`, komutlar ve eşleştir `cstmt.rs` içinde.

use crate::ast::*;
use crate::diag::{err, CResult, Diag, Sources, Span};
use crate::lexer::{Mode, BUILTIN_NAMES};
use crate::loader::ParsedModule;
use crate::tir::*;
use crate::types::*;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FlowSt {
    Assigned,
    Unassigned,
    Maybe,
}

/// Akış takibi yapılmayan (her zaman atanmış sayılan) değişkenler.
pub(crate) const ALWAYS: usize = usize::MAX;

#[derive(Clone, Debug)]
pub(crate) struct VarInfo {
    pub slot: Slot,
    pub ty: Type,
    pub sabit: bool,
    pub key: usize,
    pub loop_depth: u32,
}

pub(crate) struct ModInfo {
    pub mode: Mode,
    pub name: String,
    pub funcs: HashMap<String, FuncId>,
    pub types: HashMap<String, TypeId>,
    pub namespaces: HashMap<String, usize>,
    pub globals: HashMap<String, VarInfo>,
    pub deps: Vec<(usize, usize)>,
}

pub(crate) struct FuncInfo {
    pub name: String,
    pub module: usize,
    pub owner: Option<TypeId>,
    pub params: Vec<(String, Type)>,
    pub param_sabit: Vec<bool>,
    pub ret: Type,
    pub decl: Option<FnDecl>,
    pub snapshot: Option<Rc<HashMap<String, VarInfo>>>,
    pub yeniden: Yeniden,
    pub span: Span,
    pub body: Option<TFunc>,
    /// dış (C) fonksiyonsa: kütüphane, C'deki ad
    pub dis: Option<DisInfo>,
}

pub(crate) enum TypeDecl {
    Tur(TurDecl),
    Arayuz(ArayuzDecl),
    Secenek(SecenekDecl),
}

#[derive(Default)]
pub(crate) struct Ctx {
    pub module: usize,
    pub scopes: Vec<HashMap<String, VarInfo>>,
    pub snapshot: Option<Rc<HashMap<String, VarInfo>>>,
    pub nslots: u32,
    pub owner: Option<TypeId>,
    /// None: dosyanın üst düzeyi
    pub ret: Option<Type>,
    pub fname: String,
    pub loop_depth: u32,
    pub flow: Vec<FlowSt>,
    pub func: Option<FuncId>,
}

pub(crate) struct PendingMut {
    pub view: Option<(TypeId, String)>,
    pub direct: Option<FuncId>,
    pub reason: String,
    pub span: Span,
}

pub struct Checker {
    pub(crate) mods: Vec<ModInfo>,
    pub(crate) types: Vec<TypeDef>,
    pub(crate) type_decls: Vec<(usize, Span, Option<TypeDecl>)>,
    pub(crate) own_fields: Vec<Vec<FieldInfo>>,
    pub(crate) type_methods: Vec<Vec<FuncId>>,
    pub(crate) funcs: Vec<FuncInfo>,
    pub(crate) dispatch: HashMap<(TypeId, TypeId, String), FuncId>,
    pub(crate) nglobals: u32,
    pub(crate) warnings: Vec<Diag>,
    pub(crate) cx: Ctx,
    /// (çağıran, bakış türü, metot) — `bu` üzerinde çağrılan metotlar
    pub(crate) mut_edges: Vec<(FuncId, TypeId, String)>,
    pub(crate) mut_direct: Vec<(FuncId, FuncId)>,
    pub(crate) mutates: Vec<bool>,
    pub(crate) pending_mut: Vec<PendingMut>,
    pub(crate) breaks_flag: bool,
    /// Alan ve metot adlarının numaraları (çalışma anında isimle arama yapılmasın diye)
    pub(crate) field_ids: HashMap<Rc<str>, u32>,
    pub(crate) field_names: Vec<Rc<str>>,
    pub(crate) method_ids: HashMap<String, u32>,
    /// Türlerin içeriği çözülürken yazılan sözlük anahtarları; seçenekler henüz çözülmemiş
    /// olabileceği için kontrolleri tüm türler çözüldükten sonra yapılır.
    pub(crate) pending_keys: std::cell::RefCell<Option<Vec<(Type, Span)>>>,
    /// Kaynak dosyaların metni (hata mesajlarında kullanıcının yazdığı ifadeyi göstermek için)
    pub(crate) texts: Vec<String>,
}

pub fn check(parsed: Vec<ParsedModule>, src: &Sources) -> CResult<(Program, Vec<Diag>)> {
    let mut c = Checker {
        mods: Vec::new(),
        types: Vec::new(),
        type_decls: Vec::new(),
        own_fields: Vec::new(),
        type_methods: Vec::new(),
        funcs: Vec::new(),
        dispatch: HashMap::new(),
        nglobals: 0,
        warnings: Vec::new(),
        cx: Ctx::default(),
        mut_edges: Vec::new(),
        mut_direct: Vec::new(),
        mutates: Vec::new(),
        pending_mut: Vec::new(),
        breaks_flag: false,
        field_ids: HashMap::new(),
        field_names: Vec::new(),
        method_ids: HashMap::new(),
        pending_keys: std::cell::RefCell::new(Some(Vec::new())),
        texts: src.files.iter().map(|f| f.text.clone()).collect(),
    };
    let mut all_stmts = Vec::new();
    for m in parsed {
        c.mods.push(ModInfo {
            mode: m.mode,
            name: src.name(m.file).to_string(),
            funcs: HashMap::new(),
            types: HashMap::new(),
            namespaces: HashMap::new(),
            globals: HashMap::new(),
            deps: m.deps,
        });
        all_stmts.push(m.stmts);
    }
    let n = c.mods.len();

    // 1. Tür ve fonksiyon isimlerini kaydet
    for (mi, stmts) in all_stmts.iter().enumerate() {
        for s in stmts {
            match &s.kind {
                StmtKind::Tur(d) => c.declare_type(mi, &d.name, d.span, TypeDecl::Tur(d.clone()))?,
                StmtKind::Arayuz(d) => c.declare_type(mi, &d.name, d.span, TypeDecl::Arayuz(d.clone()))?,
                StmtKind::Secenek(d) => c.declare_type(mi, &d.name, d.span, TypeDecl::Secenek(d.clone()))?,
                StmtKind::Fonksiyon(d) => c.declare_func(mi, d.clone())?,
                StmtKind::Dis { alias, lib, funcs, lib_span } => {
                    // isimli (B): ayrı bir isim alanı (dahil et ile eklenen dosya gibi)
                    let target = match alias {
                        Some((a, sp)) => {
                            if c.mods[mi].funcs.contains_key(a) || c.mods[mi].types.contains_key(a) || c.mods[mi].namespaces.contains_key(a) {
                                return err(*sp, format!("'{a}' adı bu dosyada zaten kullanılıyor, başka bir ad seçin."));
                            }
                            let id = c.mods.len();
                            c.mods.push(ModInfo {
                                mode: c.mods[mi].mode,
                                name: format!("\"{lib}\" kütüphanesi"),
                                funcs: HashMap::new(),
                                types: HashMap::new(),
                                namespaces: HashMap::new(),
                                globals: HashMap::new(),
                                deps: Vec::new(),
                            });
                            c.mods[mi].namespaces.insert(a.clone(), id);
                            id
                        }
                        None => mi,
                    };
                    for f in funcs {
                        let d = FnDecl { name: f.name.clone(), span: f.span, params: f.params.clone(), ret: f.ret.clone(), body: vec![], yeniden: Yeniden::Yok };
                        let sym = f.symbol.clone().unwrap_or_else(|| f.name.clone());
                        if sym.starts_with("kolay_") || sym == "main" || sym.is_empty() {
                            return err(f.span, format!("'{sym}' C fonksiyonu adı olarak kullanılamaz."));
                        }
                        c.declare_func(target, d)?;
                        let fid = c.funcs.len() - 1;
                        c.funcs[fid].dis = Some(DisInfo { lib: lib.clone(), symbol: sym, lib_loc: Loc { file: lib_span.file, line: lib_span.line } });
                    }
                }
                _ => {}
            }
        }
    }

    // 2. dahil et: isimleri içe aktar
    for mi in 0..n {
        for (si, dep) in c.mods[mi].deps.clone() {
            let s = &all_stmts[mi][si];
            let StmtKind::Dahil { alias, .. } = &s.kind else { continue };
            match alias {
                Some((a, sp)) => {
                    if c.mods[mi].funcs.contains_key(a) || c.mods[mi].types.contains_key(a) || c.mods[mi].namespaces.contains_key(a) {
                        return err(*sp, format!("'{a}' adı bu dosyada zaten kullanılıyor, başka bir ad seçin."));
                    }
                    c.mods[mi].namespaces.insert(a.clone(), dep);
                }
                None => c.merge_names(mi, dep, s.span)?,
            }
        }
    }

    // 3. Türlerin içeriği (alanlar, seçenekler, metot imzaları)
    for tid in 0..c.type_decls.len() {
        c.resolve_type_decl(tid)?;
    }
    let pending = c.pending_keys.borrow_mut().take().unwrap_or_default();
    for (k, span) in pending {
        if let Err(why) = c.key_type_check(&k) {
            return err(span, why);
        }
    }
    // 4. Fonksiyon imzaları
    for fid in 0..c.funcs.len() {
        if c.funcs[fid].owner.is_none() {
            c.resolve_func_sig(fid)?;
        }
    }
    // 5. Kalıtım, metot tabloları, arayüz kontrolü
    c.build_inheritance()?;

    // 6. Dosyaların üst düzey kodu
    c.mutates = vec![false; c.funcs.len()];
    let mut modules_code = Vec::new();
    for (mi, stmts) in all_stmts.iter().enumerate() {
        modules_code.push(c.check_top(mi, stmts)?);
    }
    // 7. Fonksiyon gövdeleri
    for fid in 0..c.funcs.len() {
        c.check_body(fid)?;
    }
    // 8. Sabit nesneler üzerinde değiştiren metot çağrısı var mı?
    c.resolve_mutation()?;

    let funcs = c.funcs.iter_mut().map(|f| f.body.take().expect("gövde")).collect();
    // metot ve alan numaraları
    let mut dispatch = FxMap::default();
    for ((t, v, name), f) in std::mem::take(&mut c.dispatch) {
        let mid = c.method_id(&name);
        dispatch.insert((t, v, mid), f);
    }
    for t in 0..c.types.len() {
        let names: Vec<Rc<str>> = c.fields_of(t).iter().map(|f| f.name.clone()).collect();
        for n in &names {
            c.field_id(n);
        }
    }
    let mut field_pos = vec![vec![u32::MAX; c.field_names.len()]; c.types.len()];
    for (t, row) in field_pos.iter_mut().enumerate() {
        for (i, f) in c.fields_of(t).iter().enumerate() {
            row[c.field_ids[&f.name] as usize] = i as u32;
        }
    }
    Ok((
        Program {
            funcs,
            types: c.types,
            nglobals: c.nglobals,
            modules: modules_code,
            main_module: n - 1,
            dispatch,
            field_names: c.field_names,
            field_pos,
        },
        c.warnings,
    ))
}

pub(crate) fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

pub(crate) fn closest<'a>(name: &str, candidates: impl Iterator<Item = &'a String>) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    let limit = if name.chars().count() <= 3 { 1 } else { 2 };
    for c in candidates {
        let d = levenshtein(&name.to_lowercase(), &c.to_lowercase());
        if d <= limit && best.as_ref().is_none_or(|b| d < b.0) {
            best = Some((d, c.clone()));
        }
    }
    best.map(|b| b.1)
}

impl Checker {
    pub(crate) fn mode(&self) -> Mode {
        self.mods[self.cx.module].mode
    }

    pub(crate) fn field_id(&mut self, name: &Rc<str>) -> u32 {
        if let Some(&i) = self.field_ids.get(name) {
            return i;
        }
        let i = self.field_names.len() as u32;
        self.field_names.push(name.clone());
        self.field_ids.insert(name.clone(), i);
        i
    }

    pub(crate) fn method_id(&mut self, name: &str) -> u32 {
        if let Some(&i) = self.method_ids.get(name) {
            return i;
        }
        let i = self.method_ids.len() as u32;
        self.method_ids.insert(name.to_string(), i);
        i
    }

    pub(crate) fn warn(&mut self, span: Span, msg: impl Into<String>) {
        self.warnings.push(Diag::new(span, msg));
    }

    // ------------------------------------------------------------ tanımlar

    fn declare_type(&mut self, mi: usize, name: &str, span: Span, decl: TypeDecl) -> CResult<()> {
        if self.mods[mi].types.contains_key(name) || self.mods[mi].funcs.contains_key(name) {
            return err(span, format!("'{name}' bu dosyada zaten tanımlı."));
        }
        if is_builtin_type_name(name) {
            return err(span, format!("'{name}' dilin hazır tiplerinden biri, başka bir ad seçin."));
        }
        let tid = self.types.len();
        let kind = match &decl {
            TypeDecl::Tur(_) => TypeDefKind::Tur { fields: vec![], parents: vec![], ancestors: vec![] },
            TypeDecl::Arayuz(_) => TypeDefKind::Arayuz { sigs: vec![] },
            TypeDecl::Secenek(_) => TypeDefKind::Secenek { variants: vec![] },
        };
        self.types.push(TypeDef { name: name.to_string(), kind, interfaces: vec![], methods: HashMap::new() });
        self.type_decls.push((mi, span, Some(decl)));
        self.own_fields.push(Vec::new());
        self.type_methods.push(Vec::new());
        self.mods[mi].types.insert(name.to_string(), tid);
        Ok(())
    }

    fn declare_func(&mut self, mi: usize, d: FnDecl) -> CResult<()> {
        if self.mods[mi].funcs.contains_key(&d.name) || self.mods[mi].types.contains_key(&d.name) {
            return err(d.span, format!("'{}' bu dosyada zaten tanımlı.", d.name));
        }
        let fid = self.funcs.len();
        self.mods[mi].funcs.insert(d.name.clone(), fid);
        self.funcs.push(FuncInfo {
            name: d.name.clone(),
            module: mi,
            owner: None,
            params: vec![],
            param_sabit: vec![],
            ret: Type::Hic,
            span: d.span,
            yeniden: Yeniden::Yok,
            decl: Some(d),
            snapshot: None,
            body: None,
            dis: None,
        });
        Ok(())
    }

    fn merge_names(&mut self, mi: usize, dep: usize, span: Span) -> CResult<()> {
        let dep_name = self.mods[dep].name.clone();
        let funcs: Vec<(String, FuncId)> = self.mods[dep].funcs.iter().map(|(k, v)| (k.clone(), *v)).collect();
        let types: Vec<(String, TypeId)> = self.mods[dep].types.iter().map(|(k, v)| (k.clone(), *v)).collect();
        for (name, id) in funcs {
            match self.mods[mi].funcs.get(&name) {
                Some(&old) if old != id => return err(span, self.conflict_msg(&name, mi, &dep_name)),
                _ => {}
            }
            if self.mods[mi].types.contains_key(&name) {
                return err(span, self.conflict_msg(&name, mi, &dep_name));
            }
            self.mods[mi].funcs.insert(name, id);
        }
        for (name, id) in types {
            match self.mods[mi].types.get(&name) {
                Some(&old) if old != id => return err(span, self.conflict_msg(&name, mi, &dep_name)),
                _ => {}
            }
            if self.mods[mi].funcs.contains_key(&name) {
                return err(span, self.conflict_msg(&name, mi, &dep_name));
            }
            self.mods[mi].types.insert(name, id);
        }
        Ok(())
    }

    pub(crate) fn conflict_msg(&self, name: &str, mi: usize, dep_name: &str) -> String {
        format!(
            "'{name}' hem {} hem de {dep_name} içinde tanımlı. Dosyalardan birini isimle dahil edin: 'k adıyla dahil et(...)'.",
            self.mods[mi].name
        )
    }

    // ------------------------------------------------------------ tipler

    pub(crate) fn type_name(&self, t: &Type) -> String {
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
            Type::List(..) => {
                let mut dims = Vec::new();
                let mut cur = t;
                while let Type::List(inner, d) = cur {
                    dims.push(*d);
                    cur = inner;
                }
                let base = format!("Liste<{}>", self.type_name(cur));
                if dims.iter().all(Option::is_none) && dims.len() == 1 {
                    return base;
                }
                let mut s = base;
                for d in dims {
                    match d {
                        Some(n) => s.push_str(&format!("[{n}]")),
                        None => s.push_str("[]"),
                    }
                }
                s
            }
            Type::Dict(k, v) => format!("Sözlük<{}, {}>", self.type_name(k), self.type_name(v)),
            Type::Union(v) => v.iter().map(|t| self.type_name(t)).collect::<Vec<_>>().join(" veya "),
            Type::Named(id) => self.types[*id].name.clone(),
        }
    }

    pub(crate) fn resolve_type(&self, te: &TypeExpr, mi: usize) -> CResult<Type> {
        match &te.kind {
            TypeKind::Union(parts) => {
                let mut ts = Vec::new();
                for p in parts {
                    ts.push(self.resolve_type(p, mi)?);
                }
                let u = make_union(ts);
                if let Type::Union(ms) = &u {
                    if ms.iter().filter(|t| t.is_int()).count() > 1 {
                        return err(te.span, "Bir 'veya' tipinde iki farklı tam sayı tipi olamaz.");
                    }
                    if ms.iter().filter(|t| t.is_float()).count() > 1 {
                        return err(te.span, "Bir 'veya' tipinde iki farklı ondalık tipi olamaz.");
                    }
                }
                Ok(u)
            }
            TypeKind::Path { path, generics, args, dims } => {
                let base = self.resolve_base_type(te.span, path, generics, args, mi)?;
                let is_list = matches!(base, Type::List(..)) && path.len() == 1 && path[0] == "Liste";
                if !dims.is_empty() && !is_list {
                    return err(te.span, "Köşeli parantezle boyut sadece Liste için yazılabilir: Liste<Tam>[4]");
                }
                if !is_list {
                    return Ok(base);
                }
                let Type::List(elem, _) = base else { unreachable!() };
                let mut t = *elem;
                if dims.is_empty() {
                    return Ok(Type::List(Box::new(t), None));
                }
                for d in dims.iter().rev() {
                    t = Type::List(Box::new(t), *d);
                }
                Ok(t)
            }
        }
    }

    fn resolve_base_type(
        &self,
        span: Span,
        path: &[String],
        generics: &[TypeExpr],
        args: &Option<Vec<TypeArg>>,
        mi: usize,
    ) -> CResult<Type> {
        let name = path.last().unwrap().as_str();
        let no_generics = |what: &str| -> CResult<()> {
            if generics.is_empty() { Ok(()) } else { err(span, format!("{what} tipi '<...>' almaz.")) }
        };
        let no_args = |what: &str| -> CResult<()> {
            if args.is_none() { Ok(()) } else { err(span, format!("{what} tipi parantez almaz.")) }
        };
        if path.len() == 1 {
            match name {
                "Tam" => {
                    no_generics("Tam")?;
                    let (bits, sign) = match args.as_deref() {
                        None => (None, Sign::Signed),
                        Some([TypeArg::Num(n)]) => (Some(*n), Sign::Signed),
                        Some([TypeArg::Num(n), TypeArg::Plus]) => (Some(*n), Sign::Pos),
                        Some([TypeArg::Num(n), TypeArg::Minus]) => (Some(*n), Sign::Neg),
                        Some([TypeArg::Plus]) => (None, Sign::Pos),
                        Some([TypeArg::Minus]) => (None, Sign::Neg),
                        Some(_) => {
                            return err(span, "Tam tipi şöyle yazılır: Tam, Tam(8), Tam(8, +), Tam(8, -), Tam(+), Tam(-)");
                        }
                    };
                    if let Some(b) = bits {
                        if ![8, 16, 32, 64, 128].contains(&b) {
                            return err(span, format!("Tam({b}) geçersiz. Geçerli boyutlar: 8, 16, 32, 64, 128."));
                        }
                    }
                    return Ok(Type::Int { bits, sign });
                }
                "Ondalık" => {
                    no_generics("Ondalık")?;
                    return match args.as_deref() {
                        None => Ok(Type::Float(64)),
                        Some([TypeArg::Num(b)]) if [16, 32, 64].contains(b) => Ok(Type::Float(*b)),
                        Some(_) => err(span, "Ondalık tipi şöyle yazılır: Ondalık, Ondalık(16), Ondalık(32), Ondalık(64)"),
                    };
                }
                "Yazı" => {
                    no_generics("Yazı")?;
                    no_args("Yazı")?;
                    return Ok(Type::Str);
                }
                "Mantık" => {
                    no_generics("Mantık")?;
                    no_args("Mantık")?;
                    return Ok(Type::Bool);
                }
                "Hiç" => {
                    no_generics("Hiç")?;
                    no_args("Hiç")?;
                    return Ok(Type::Hic);
                }
                "Liste" => {
                    no_args("Liste")?;
                    if generics.len() != 1 {
                        return err(span, "Liste tipi tek bir eleman tipi alır: Liste<Tam>");
                    }
                    let elem = self.resolve_type(&generics[0], mi)?;
                    return Ok(Type::List(Box::new(elem), None));
                }
                "Sözlük" => {
                    no_args("Sözlük")?;
                    if generics.len() != 2 {
                        return err(span, "Sözlük tipi anahtar ve değer tipi alır: Sözlük<Yazı, Tam>");
                    }
                    let k = self.resolve_type(&generics[0], mi)?;
                    let v = self.resolve_type(&generics[1], mi)?;
                    if let Some(p) = self.pending_keys.borrow_mut().as_mut() {
                        p.push((k.clone(), generics[0].span));
                    } else if let Err(why) = self.key_type_check(&k) {
                        return err(generics[0].span, why);
                    }
                    return Ok(Type::Dict(Box::new(k), Box::new(v)));
                }
                _ => {}
            }
        }
        // kullanıcı tipleri
        let mut module = mi;
        for (i, part) in path[..path.len() - 1].iter().enumerate() {
            match self.mods[module].namespaces.get(part) {
                Some(&m) => module = m,
                None => return err(span, format!("'{}' diye bir dosya adı yok.", path[..=i].join("."))),
            }
        }
        if let Some(&tid) = self.mods[module].types.get(name) {
            if !generics.is_empty() || args.is_some() {
                return err(span, format!("'{name}' tipi '<...>' ya da parantez almaz."));
            }
            return Ok(Type::Named(tid));
        }
        Err(Diag::new(span, self.unknown_type_msg(name, module)))
    }

    pub(crate) fn unknown_type_msg(&self, name: &str, mi: usize) -> String {
        let mode = self.mods[mi].mode;
        for (tr, us) in BUILTIN_NAMES {
            if name == *us && mode == Mode::Tr && starts_upper(tr) {
                return format!("'{name}' diye bir tip yok. '{tr}' mı demek istediniz? (Türkçe karakter olmadan yazmak için dosyanın başına klavye(\"us\") ekleyin.)");
            }
        }
        let mut names: Vec<String> = ["Tam", "Ondalık", "Yazı", "Mantık", "Liste", "Sözlük", "Hiç"].iter().map(|s| s.to_string()).collect();
        names.extend(self.mods[mi].types.keys().cloned());
        if let Some(s) = closest(name, names.iter()) {
            let shown = if mode == Mode::Us { to_us(&s) } else { s };
            return format!("'{name}' diye bir tip yok. '{shown}' mı demek istediniz?");
        }
        format!("'{name}' diye bir tip yok.")
    }

    /// Sözlük anahtarı olabilir mi? Olamıyorsa nedenini anlatan mesaj döner.
    /// Seçenekler, taşıdıkları değerler de anahtar olabiliyorsa anahtar olabilir.
    pub(crate) fn key_type_check(&self, t: &Type) -> Result<(), String> {
        let mut seen = Vec::new();
        self.key_type_inner(t, &mut seen).map_err(|bad| {
            let tn = self.type_name(t);
            let base = "Anahtar Tam, Yazı, Mantık ya da bir seçenek olmalı";
            if bad == tn {
                format!("{tn} sözlük anahtarı olamaz. {base}.")
            } else {
                format!("{tn} sözlük anahtarı olamaz: içinde {bad} değeri taşıyor. {base}; seçenekler de sadece bunları taşıyabilir.")
            }
        })
    }

    fn key_type_inner(&self, t: &Type, seen: &mut Vec<TypeId>) -> Result<(), String> {
        match t {
            Type::Int { .. } | Type::Str | Type::Bool => Ok(()),
            Type::Named(id) => {
                let TypeDefKind::Secenek { variants } = &self.types[*id].kind else {
                    return Err(self.type_name(t));
                };
                if seen.contains(id) {
                    return Ok(());
                }
                seen.push(*id);
                for v in variants {
                    for (_, ft) in &v.fields {
                        self.key_type_inner(ft, seen)?;
                    }
                }
                Ok(())
            }
            _ => Err(self.type_name(t)),
        }
    }

    fn resolve_type_decl(&mut self, tid: TypeId) -> CResult<()> {
        let (mi, _span, decl) = &mut self.type_decls[tid];
        let mi = *mi;
        let decl = decl.take().unwrap();
        match &decl {
            TypeDecl::Tur(d) => {
                let mut parents = Vec::new();
                let mut interfaces = Vec::new();
                for (path, pspan) in &d.parents {
                    let t = self.resolve_type(
                        &TypeExpr {
                            kind: TypeKind::Path { path: path.clone(), generics: vec![], args: None, dims: vec![] },
                            span: *pspan,
                        },
                        mi,
                    )?;
                    let Type::Named(pid) = t else {
                        return err(*pspan, format!("'{}' bir ebeveyn olamaz; sadece tür ya da arayüz yazılabilir.", self.type_name(&t)));
                    };
                    match self.types[pid].kind {
                        TypeDefKind::Tur { .. } => {
                            if parents.contains(&pid) {
                                return err(*pspan, "Aynı ebeveyn iki kez yazılmış.");
                            }
                            parents.push(pid)
                        }
                        TypeDefKind::Arayuz { .. } => interfaces.push(pid),
                        TypeDefKind::Secenek { .. } => {
                            return err(*pspan, format!("'{}' bir seçenek; seçeneklerden kalıtım alınamaz.", self.types[pid].name));
                        }
                    }
                }
                let mut fields: Vec<FieldInfo> = Vec::new();
                for p in &d.fields {
                    if fields.iter().any(|f| *f.name == *p.name) {
                        return err(p.span, format!("'{}' alanı iki kez tanımlanmış.", p.name));
                    }
                    let ty = self.resolve_type(&p.ty, mi)?;
                    self.no_hic_value(&ty, p.ty.span)?;
                    fields.push(FieldInfo { name: p.name.as_str().into(), ty, sabit: p.sabit, origin: tid });
                }
                if let TypeDefKind::Tur { parents: ps, .. } = &mut self.types[tid].kind {
                    *ps = parents;
                }
                self.types[tid].interfaces = interfaces;
                self.own_fields[tid] = fields;
                for m in &d.methods {
                    let fid = self.add_method(tid, mi, m.clone())?;
                    self.type_methods[tid].push(fid);
                }
            }
            TypeDecl::Secenek(d) => {
                let mut interfaces = Vec::new();
                for (path, pspan) in &d.interfaces {
                    let t = self.resolve_type(
                        &TypeExpr {
                            kind: TypeKind::Path { path: path.clone(), generics: vec![], args: None, dims: vec![] },
                            span: *pspan,
                        },
                        mi,
                    )?;
                    match t {
                        Type::Named(pid) if matches!(self.types[pid].kind, TypeDefKind::Arayuz { .. }) => interfaces.push(pid),
                        _ => return err(*pspan, "Seçenekler sadece arayüz uygulayabilir, kalıtım alamaz."),
                    }
                }
                let mut variants: Vec<VariantInfo> = Vec::new();
                for v in &d.variants {
                    if variants.iter().any(|x| *x.name == *v.name) {
                        return err(v.span, format!("'{}' seçeneği iki kez yazılmış.", v.name));
                    }
                    let mut fs = Vec::new();
                    for p in &v.fields {
                        let ty = self.resolve_type(&p.ty, mi)?;
                        self.no_hic_value(&ty, p.ty.span)?;
                        fs.push((Rc::from(p.name.as_str()), ty));
                    }
                    variants.push(VariantInfo { name: v.name.as_str().into(), fields: fs });
                }
                self.types[tid].kind = TypeDefKind::Secenek { variants };
                self.types[tid].interfaces = interfaces;
                for m in &d.methods {
                    if !matches!(m.yeniden, Yeniden::Yok) {
                        return err(m.span, "Seçeneklerde kalıtım olmadığı için 'yeniden' kullanılamaz.");
                    }
                    let fid = self.add_method(tid, mi, m.clone())?;
                    self.type_methods[tid].push(fid);
                }
            }
            TypeDecl::Arayuz(d) => {
                let mut sigs: Vec<Sig> = Vec::new();
                for s in &d.sigs {
                    if sigs.iter().any(|x| x.name == s.name) {
                        return err(s.span, format!("'{}' arayüzde iki kez yazılmış.", s.name));
                    }
                    let mut params = Vec::new();
                    for p in &s.params {
                        params.push((p.name.clone(), self.resolve_type(&p.ty, mi)?));
                    }
                    let ret = match &s.ret {
                        Some(t) => self.resolve_type(t, mi)?,
                        None => Type::Hic,
                    };
                    sigs.push(Sig { name: s.name.clone(), params, ret });
                }
                self.types[tid].kind = TypeDefKind::Arayuz { sigs };
            }
        }
        Ok(())
    }

    pub(crate) fn no_hic_value(&self, t: &Type, span: Span) -> CResult<()> {
        if *t == Type::Hic {
            return err(span, "Hiç tek başına bir değer tipi olamaz. 'Yazı veya Hiç' gibi yazın.");
        }
        Ok(())
    }

    fn add_method(&mut self, tid: TypeId, mi: usize, d: FnDecl) -> CResult<FuncId> {
        let fid = self.funcs.len();
        let (params, sabits, ret) = self.sig_of(&d, mi)?;
        if self.type_methods[tid].iter().any(|&f| {
            let o = &self.funcs[f];
            o.name == d.name
                && match (&o.yeniden, &d.yeniden) {
                    (Yeniden::Ebeveyn(a, _), Yeniden::Ebeveyn(b, _)) => a == b,
                    (Yeniden::Ebeveyn(..), _) | (_, Yeniden::Ebeveyn(..)) => false,
                    _ => true,
                }
        }) {
            return err(d.span, format!("'{}' bu tür içinde iki kez tanımlanmış.", d.name));
        }
        if self.own_fields[tid].iter().any(|f| *f.name == *d.name) {
            return err(d.span, format!("'{}' hem alan hem fonksiyon adı olamaz.", d.name));
        }
        self.funcs.push(FuncInfo {
            name: d.name.clone(),
            module: mi,
            owner: Some(tid),
            params,
            param_sabit: sabits,
            ret,
            span: d.span,
            yeniden: d.yeniden.clone(),
            decl: Some(d),
            snapshot: None,
            body: None,
            dis: None,
        });
        Ok(fid)
    }

    fn sig_of(&self, d: &FnDecl, mi: usize) -> CResult<(Vec<(String, Type)>, Vec<bool>, Type)> {
        let mut params: Vec<(String, Type)> = Vec::new();
        let mut sabits = Vec::new();
        for p in &d.params {
            if params.iter().any(|(n, _)| *n == p.name) {
                return err(p.span, format!("'{}' parametresi iki kez yazılmış.", p.name));
            }
            let ty = self.resolve_type(&p.ty, mi)?;
            self.no_hic_value(&ty, p.ty.span)?;
            params.push((p.name.clone(), ty));
            sabits.push(p.sabit);
        }
        let ret = match &d.ret {
            Some(t) => self.resolve_type(t, mi)?,
            None => Type::Hic,
        };
        Ok((params, sabits, ret))
    }

    fn resolve_func_sig(&mut self, fid: FuncId) -> CResult<()> {
        let mi = self.funcs[fid].module;
        let d = self.funcs[fid].decl.clone().unwrap();
        let (params, sabits, ret) = self.sig_of(&d, mi)?;
        if self.funcs[fid].dis.is_some() {
            for (p, (_, t)) in d.params.iter().zip(&params) {
                if !dis_type_ok(t) {
                    return err(p.ty.span, format!("{} tipi C fonksiyonuna verilemez. {DIS_TIPLER}", self.type_name(t)));
                }
            }
            if ret != Type::Hic && !dis_type_ok(&ret) {
                let sp = d.ret.as_ref().map(|r| r.span).unwrap_or(d.span);
                return err(sp, format!("{} tipi C fonksiyonundan dönemez. {DIS_TIPLER}", self.type_name(&ret)));
            }
        }
        let f = &mut self.funcs[fid];
        f.params = params;
        f.param_sabit = sabits;
        f.ret = ret;
        Ok(())
    }

    // ------------------------------------------------------------ kalıtım

    fn same_sig(&self, a: FuncId, params: &[(String, Type)], ret: &Type) -> bool {
        let f = &self.funcs[a];
        f.ret == *ret && f.params.len() == params.len() && f.params.iter().zip(params).all(|(x, y)| x.1 == y.1)
    }

    fn sig_text(&self, params: &[(String, Type)], ret: &Type) -> String {
        let ps: Vec<String> = params.iter().map(|(n, t)| format!("{n}: {}", self.type_name(t))).collect();
        if *ret == Type::Hic {
            format!("({})", ps.join(", "))
        } else {
            format!("({}) -> {}", ps.join(", "), self.type_name(ret))
        }
    }

    fn build_inheritance(&mut self) -> CResult<()> {
        let n = self.types.len();
        // topolojik sıra (ebeveynler önce)
        let mut order = Vec::new();
        let mut state = vec![0u8; n];
        for t in 0..n {
            self.topo(t, &mut state, &mut order, &mut Vec::new())?;
        }
        let mut flat_fields: Vec<Vec<FieldInfo>> = vec![Vec::new(); n];
        for &tid in &order {
            let span = self.type_decls[tid].1;
            match self.types[tid].kind.clone() {
                TypeDefKind::Tur { parents, .. } => {
                    // alanlar
                    let mut fields: Vec<FieldInfo> = Vec::new();
                    let mut ancestors: Vec<TypeId> = Vec::new();
                    let mut interfaces = self.types[tid].interfaces.clone();
                    for &p in &parents {
                        for f in &flat_fields[p] {
                            if let Some(old) = fields.iter().find(|x| x.name == f.name) {
                                if old.origin != f.origin {
                                    return err(
                                        span,
                                        format!(
                                            "'{}' alanı hem {} hem de {} içinde var.",
                                            f.name, self.types[old.origin].name, self.types[f.origin].name
                                        ),
                                    );
                                }
                                continue;
                            }
                            fields.push(f.clone());
                        }
                        if !ancestors.contains(&p) {
                            ancestors.push(p);
                        }
                        if let TypeDefKind::Tur { ancestors: pa, .. } = &self.types[p].kind {
                            for a in pa {
                                if !ancestors.contains(a) {
                                    ancestors.push(*a);
                                }
                            }
                        }
                        for i in self.types[p].interfaces.clone() {
                            if !interfaces.contains(&i) {
                                interfaces.push(i);
                            }
                        }
                    }
                    for f in &self.own_fields[tid] {
                        if let Some(old) = fields.iter().find(|x| x.name == f.name) {
                            return err(
                                span,
                                format!("'{}' alanı ebeveyn {} içinde zaten var.", f.name, self.types[old.origin].name),
                            );
                        }
                        fields.push(f.clone());
                    }
                    flat_fields[tid] = fields.clone();
                    self.types[tid].kind = TypeDefKind::Tur { fields, parents: parents.clone(), ancestors: ancestors.clone() };
                    self.types[tid].interfaces = interfaces;
                    self.resolve_methods(tid, &parents, &ancestors)?;
                }
                TypeDefKind::Secenek { .. } => {
                    for fid in self.type_methods[tid].clone() {
                        let name = self.funcs[fid].name.clone();
                        self.types[tid].methods.insert(name.clone(), MethodRes::Impl(fid));
                        self.dispatch.insert((tid, tid, name), fid);
                    }
                }
                TypeDefKind::Arayuz { .. } => {}
            }
            self.check_interfaces(tid, span)?;
        }
        Ok(())
    }

    fn topo(&self, t: TypeId, state: &mut [u8], order: &mut Vec<TypeId>, path: &mut Vec<TypeId>) -> CResult<()> {
        if state[t] == 2 {
            return Ok(());
        }
        if state[t] == 1 {
            let mut names: Vec<String> = path.iter().map(|&x| self.types[x].name.clone()).collect();
            names.push(self.types[t].name.clone());
            return err(self.type_decls[t].1, format!("Kalıtım döngüsü: {}", names.join(" → ")));
        }
        state[t] = 1;
        path.push(t);
        if let TypeDefKind::Tur { parents, .. } = &self.types[t].kind {
            for &p in parents {
                self.topo(p, state, order, path)?;
            }
        }
        path.pop();
        state[t] = 2;
        order.push(t);
        Ok(())
    }

    fn resolve_methods(&mut self, tid: TypeId, parents: &[TypeId], ancestors: &[TypeId]) -> CResult<()> {
        let tname = self.types[tid].name.clone();
        // ebeveynlerden gelen adaylar
        let mut candidates: HashMap<String, Vec<(TypeId, MethodRes)>> = HashMap::new();
        for &p in parents {
            for (m, r) in &self.types[p].methods {
                candidates.entry(m.clone()).or_default().push((p, r.clone()));
            }
        }
        let mut own: HashMap<String, MethodRes> = HashMap::new();
        let mut defined_here: HashSet<String> = HashSet::new();
        let mut per_parent: HashMap<(TypeId, String), FuncId> = HashMap::new();
        for fid in self.type_methods[tid].clone() {
            let f = &self.funcs[fid];
            let (name, span, yeniden) = (f.name.clone(), f.span, f.yeniden.clone());
            let (params, ret) = (f.params.clone(), f.ret.clone());
            match yeniden {
                Yeniden::Yok => {
                    if let Some(c) = candidates.get(&name) {
                        let pname = &self.types[c[0].0].name;
                        return err(
                            span,
                            format!("'{name}' ebeveyn {pname} içinde zaten var. Değiştirmek için 'yeniden {name}()' yazın."),
                        );
                    }
                    own.insert(name.clone(), MethodRes::Impl(fid));
                    defined_here.insert(name);
                }
                Yeniden::Genel => {
                    let Some(c) = candidates.get(&name) else {
                        return err(span, format!("'yeniden {name}': ebeveynlerde '{name}' diye bir fonksiyon yok."));
                    };
                    for (p, r) in c {
                        if let MethodRes::Impl(pf) = r {
                            if !self.same_sig(*pf, &params, &ret) {
                                let pfi = &self.funcs[*pf];
                                return err(
                                    span,
                                    format!(
                                        "'yeniden {name}' imzası {} içindekiyle aynı olmalı: {}",
                                        self.types[*p].name,
                                        self.sig_text(&pfi.params, &pfi.ret)
                                    ),
                                );
                            }
                        }
                    }
                    own.insert(name.clone(), MethodRes::Impl(fid));
                    defined_here.insert(name);
                }
                Yeniden::Ebeveyn(pn, pspan) => {
                    let Some(&pid) = parents.iter().find(|&&p| self.types[p].name == pn) else {
                        return err(pspan, format!("'{pn}', {tname} türünün doğrudan ebeveyni değil."));
                    };
                    let Some(MethodRes::Impl(pf)) = self.types[pid].methods.get(&name).cloned() else {
                        return err(span, format!("'{pn}' içinde '{name}' diye bir fonksiyon yok."));
                    };
                    if !self.same_sig(pf, &params, &ret) {
                        let pfi = &self.funcs[pf];
                        return err(span, format!("'yeniden {pn}.{name}' imzası {pn} içindekiyle aynı olmalı: {}", self.sig_text(&pfi.params, &pfi.ret)));
                    }
                    per_parent.insert((pid, name), fid);
                }
            }
        }
        // miras alınanlar
        for (name, cands) in &candidates {
            if own.contains_key(name) {
                continue;
            }
            let mut impls: Vec<FuncId> = Vec::new();
            let mut ambiguous = false;
            for (_, r) in cands {
                match r {
                    MethodRes::Impl(f) => {
                        if !impls.contains(f) {
                            impls.push(*f);
                        }
                    }
                    MethodRes::Ambiguous(_) => ambiguous = true,
                }
            }
            if impls.len() == 1 && !ambiguous {
                own.insert(name.clone(), MethodRes::Impl(impls[0]));
            } else {
                own.insert(name.clone(), MethodRes::Ambiguous(cands.iter().map(|c| c.0).collect()));
            }
        }
        // dağıtım tabloları
        for (name, r) in &own {
            if let MethodRes::Impl(f) = r {
                self.dispatch.insert((tid, tid, name.clone()), *f);
            }
        }
        for &a in ancestors {
            let names: Vec<(String, MethodRes)> = self.types[a].methods.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            for (m, r) in names {
                if !matches!(r, MethodRes::Impl(_)) {
                    continue;
                }
                let f = if let Some(&f) = per_parent.get(&(a, m.clone())) {
                    Some(f)
                } else if defined_here.contains(&m) {
                    match own.get(&m) {
                        Some(MethodRes::Impl(f)) => Some(*f),
                        _ => None,
                    }
                } else if let Some(MethodRes::Impl(f)) = own.get(&m) {
                    Some(*f)
                } else {
                    let mut found = None;
                    for &p in parents {
                        let reaches = p == a
                            || matches!(&self.types[p].kind, TypeDefKind::Tur { ancestors, .. } if ancestors.contains(&a));
                        if reaches {
                            found = self.dispatch.get(&(p, a, m.clone())).copied();
                            if found.is_some() {
                                break;
                            }
                        }
                    }
                    found
                };
                if let Some(f) = f {
                    self.dispatch.insert((tid, a, m), f);
                }
            }
        }
        self.types[tid].methods = own;
        Ok(())
    }

    fn check_interfaces(&mut self, tid: TypeId, span: Span) -> CResult<()> {
        let tname = self.types[tid].name.clone();
        for iid in self.types[tid].interfaces.clone() {
            let TypeDefKind::Arayuz { sigs } = self.types[iid].kind.clone() else { continue };
            let iname = self.types[iid].name.clone();
            for s in sigs {
                match self.types[tid].methods.get(&s.name).cloned() {
                    Some(MethodRes::Impl(f)) => {
                        if !self.same_sig(f, &s.params, &s.ret) {
                            return err(
                                self.funcs[f].span,
                                format!("'{}' imzası {iname} arayüzündekiyle aynı olmalı: {}", s.name, self.sig_text(&s.params, &s.ret)),
                            );
                        }
                        self.dispatch.insert((tid, iid, s.name.clone()), f);
                    }
                    Some(MethodRes::Ambiguous(_)) => {
                        return err(span, format!("'{tname}' içinde '{}' belirsiz. Ortak bir 'yeniden {}()' yazın.", s.name, s.name));
                    }
                    None => {
                        return err(span, format!("'{tname}', {iname} arayüzünü uyguluyor ama '{}()' fonksiyonu yok.", s.name));
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn is_subtype(&self, child: TypeId, parent: TypeId) -> bool {
        if child == parent {
            return true;
        }
        let t = &self.types[child];
        if t.interfaces.contains(&parent) {
            return true;
        }
        matches!(&t.kind, TypeDefKind::Tur { ancestors, .. } if ancestors.contains(&parent))
    }

    pub(crate) fn fields_of(&self, tid: TypeId) -> &[FieldInfo] {
        match &self.types[tid].kind {
            TypeDefKind::Tur { fields, .. } => fields,
            _ => &[],
        }
    }

    // ------------------------------------------------------------ değiştirme analizi

    fn resolve_mutation(&mut self) -> CResult<()> {
        loop {
            let mut changed = false;
            for (caller, view, name) in self.mut_edges.clone() {
                if self.mutates[caller] {
                    continue;
                }
                let any = self.dispatch.iter().any(|((_, v, m), f)| *v == view && *m == name && self.mutates[*f]);
                if any {
                    self.mutates[caller] = true;
                    changed = true;
                }
            }
            for (caller, callee) in self.mut_direct.clone() {
                if !self.mutates[caller] && self.mutates[callee] {
                    self.mutates[caller] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for p in &self.pending_mut {
            let mutating = match (&p.view, p.direct) {
                (_, Some(f)) => self.mutates[f],
                (Some((view, name)), None) => {
                    self.dispatch.iter().any(|((_, v, m), f)| v == view && m == name && self.mutates[*f])
                }
                _ => false,
            };
            if mutating {
                return err(p.span, p.reason.clone());
            }
        }
        Ok(())
    }
}

const DIS_TIPLER: &str = "Kullanılabilen tipler: Tam(8/16/32/64), Tam(N, +), Ondalık, Ondalık(32), Mantık, Yazı.";

/// C'ye verilebilen tipler
fn dis_type_ok(t: &Type) -> bool {
    matches!(t, Type::Int { bits: Some(8 | 16 | 32 | 64), sign: Sign::Signed | Sign::Pos } | Type::Float(32 | 64) | Type::Bool | Type::Str)
}

pub(crate) fn starts_upper(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_uppercase())
}

pub(crate) fn is_builtin_type_name(s: &str) -> bool {
    matches!(s, "Tam" | "Ondalık" | "Yazı" | "Mantık" | "Liste" | "Sözlük" | "Hiç")
}

pub(crate) fn to_us(s: &str) -> String {
    for (tr, us) in BUILTIN_NAMES {
        if s == *tr {
            return us.to_string();
        }
    }
    s.to_string()
}
