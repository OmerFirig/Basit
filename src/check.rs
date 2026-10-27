// Tip denetleyici: isimleri çözer, tipleri kontrol eder ve TIR üretir.
// Bu dosya: genel yapı, tanımlar, tip çözümleme, kalıtım ve arayüzler.
// İfadeler `cexpr.rs`, komutlar ve eşleştir `cstmt.rs` içinde.

use crate::ast::*;
use crate::diag::{err, CResult, Diag, Sources, Span};
pub(crate) use crate::genel_deger::{GenelLambda, GenelRef};
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
    /// Lambdanın dışarıdan kopya olarak yakaladığı değişken (lambda içinde değiştirilemez)
    pub yakalanan: bool,
    /// Generic fonksiyon değeri (generic lambda ya da `Fonksiyon<T>` parametresine verilen fonksiyon):
    /// çalışma anında yoktur, her kullanımda tipleri belli bir kopyası oluşturulur (bkz. genel_deger.rs)
    pub genel: Option<GenelRef>,
}

/// Denetlenmekte olan bir lambdanın dışarıdaki bağlamı ve yakaladıkları.
pub(crate) struct LambdaFrame {
    /// Lambdanın yazıldığı yerin bağlamı
    pub outer: Ctx,
    /// (dışarıdaki yuva, içerideki yuva, tip)
    pub caps: Vec<(Slot, Slot, Type)>,
    /// Lambda nesneyi (`bu`, alanlar, kendi metotları) kullanıyor mu
    pub bu: bool,
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
    /// Tip parametreleri (generic kalıp ise boş değil). Kalıbın gövdesi denetlenmez; her çağrıldığı
    /// tiplerle ayrı bir kopyası (`inst`) oluşturulup o denetlenir.
    pub generics: Vec<String>,
    pub inst: Option<Inst>,
    /// `ortak fonksiyon` ise ait olduğu tür (nesnesi yok, türün ortak değerlerine adıyla erişir)
    pub ortak_of: Option<TypeId>,
    /// Parametrelerin varsayılan değerleri (gövde denetlenince `decl` alındığı için ayrıca saklanır)
    pub varsayilan: Vec<Option<Expr>>,
}

/// Generic bir fonksiyonun somut tiplerle oluşturulmuş kopyası
#[derive(Clone)]
pub(crate) struct Inst {
    pub kalip: Kalip,
    /// (T, Tam(64)) ...
    pub binds: Vec<(String, Type)>,
    /// Kopyayı ilk oluşturan çağrının yeri (hata mesajı için)
    pub call: Span,
    /// `Fonksiyon<T>(...)` parametrelerine verilen generic fonksiyonlar: (parametre adı, fonksiyon)
    pub genel_baglar: Vec<(String, GenelRef)>,
}

#[derive(Clone, Copy)]
pub(crate) enum Kalip {
    Fonksiyon(FuncId),
    /// Generic türün kopyasındaki metot: türün kalıbı
    Tur(TypeId),
}

/// Lambdanın iki geçişli denetiminde ilk geçişten önceki durum
#[derive(Clone, Copy)]
pub(crate) struct GeriNokta {
    pub funcs: usize,
    pub warnings: usize,
    pub mut_edges: usize,
    pub mut_direct: usize,
    pub pending_mut: usize,
    pub types: usize,
    pub genel_lambdalar: usize,
}

#[derive(Clone)]
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
    /// Gövdesi blok olan lambdanın dönüş tipi çıkarılırken: `döndür` edilen değerlerin tipleri
    pub ret_infer: Option<Vec<(Type, Span)>>,
    /// İç içe `dene` blokları (yakala dalları sayılmaz): `T veya Hata` döndüren çağrılar açılır
    pub dene_depth: u32,
    /// `ortak fonksiyon` içindeyse türü (metotlarda `owner` yeterli)
    pub ortak_tur: Option<TypeId>,
    /// Varsayılan değer denetleniyor (değişken görünmez)
    pub varsayilan: bool,
    /// Generic lambdanın kopyası denetleniyor (dışarıdaki değişkenler görünmez)
    pub genel_lambda: bool,
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
    /// Kaynak dosyaların metni (hata mesajlarında kullanıcının yazdığı ifadeyi göstermek için) ve adları
    pub(crate) texts: Vec<String>,
    pub(crate) dosya_adlari: Vec<String>,
    /// İç içe denetlenen lambdalar (en içteki sonda)
    pub(crate) lambda_stack: Vec<LambdaFrame>,
    /// Hazır hata türleri (tir::HATA_TURLERI sırasıyla)
    pub(crate) hata_turleri: [TypeId; 6],
    /// Tipsiz `yakala` dallarının değişken tipleri (önceki geçişin fırlatma analizinden; anahtar: dalın yeri)
    pub(crate) tahmin: HashMap<(u32, u32), Type>,
    /// Keşif geçişi: tipsiz yakala dalındaki hatalar yok sayılır (tipi henüz kesin değil)
    pub(crate) kesif: bool,
    pub(crate) tipsiz_var: bool,
    pub(crate) yutulan: bool,
    /// Denetlenmekte olan generic kopyadaki tip parametrelerinin karşılıkları (T -> Tam(64))
    pub(crate) tsubst: HashMap<String, Type>,
    /// (kalıp, tipler, Fonksiyon<T> parametrelerine verilenler) -> kopya
    pub(crate) instances: HashMap<(FuncId, Vec<Type>, Vec<GenelRef>), FuncId>,
    /// `değişken f = her<T> ...` ile tanımlanan generic lambdalar
    pub(crate) genel_lambdalar: Vec<GenelLambda>,
    /// Generic tür kalıpları: tip parametreleri ve tanımı
    pub(crate) tur_kalip: HashMap<TypeId, (Vec<String>, TypeDecl)>,
    /// Generic türün kopyası -> (kalıp, tipler, ilk kullanıldığı yer)
    pub(crate) tur_inst: HashMap<TypeId, (TypeId, Vec<Type>, Span)>,
    pub(crate) tur_instances: HashMap<(TypeId, Vec<Type>), TypeId>,
    /// Generic tür kalıbının tanımlandığı yerde görünen dosya değişkenleri (kopyaların metotları için)
    pub(crate) tur_snapshot: HashMap<TypeId, Rc<HashMap<String, VarInfo>>>,
    /// Kalıtım tabloları kuruldu mu (sonra oluşan tür kopyaları kendi tablolarını hemen kurar)
    pub(crate) miras_hazir: bool,
    /// Türe ait ortak değerlerin adları ve ortak fonksiyonlar. Ortak değer, `Tür.ad` adlı gizli bir
    /// dosya değişkenidir; `.` kullanıcı adlarında olmadığı için çakışmaz.
    pub(crate) ortak_adlar: HashMap<TypeId, Vec<String>>,
    pub(crate) ortak_fonk: HashMap<(TypeId, String), FuncId>,
    /// Generic türün ortak değerleri ve fonksiyonları denetlenirken kullanılamayan tip parametreleri
    pub(crate) yasak_tip: Vec<String>,
    /// Alanların başlangıç değerleri: (alanın tanımlandığı tür, ad) -> ifade
    pub(crate) alan_varsayilan: HashMap<(TypeId, String), Expr>,
    /// Şu an denetlenen varsayılan değerler (kendini kullanan varsayılanı yakalamak için)
    pub(crate) varsayilan_yigini: Vec<String>,
}

/// Tipsiz `yakala` dalının değişken tipi, fırlatılabilen değerlere bağlıdır; bunlar ise bütün program
/// denetlenince belli olur. Bu yüzden böyle bir dal varsa program birkaç kez denetlenir: önce keşif
/// geçişleri (tipler oturana kadar), sonra asıl geçiş.
pub fn check(parsed: Vec<ParsedModule>, src: &Sources) -> CResult<(Program, Vec<Diag>)> {
    let mut tahmin: HashMap<(u32, u32), Type> = HashMap::new();
    let mut kesif = true;
    for _ in 0..8 {
        let (prog, mut warnings, yeni, bilgi) = check_pass(parsed.clone(), src, &tahmin, kesif)?;
        if !bilgi.0 || (yeni == tahmin && !bilgi.1) {
            sort_warnings(&mut warnings);
            // tipsiz dal yok ya da tipler oturdu ve hiçbir dal yok sayılmadı: sonuç kesin
            return Ok((prog, warnings));
        }
        if yeni == tahmin {
            kesif = false;
        }
        tahmin = yeni;
    }
    let (prog, mut warnings, _, _) = check_pass(parsed, src, &tahmin, false)?;
    sort_warnings(&mut warnings);
    Ok((prog, warnings))
}

/// Generic bir fonksiyonun her kopyası aynı uyarıyı verebilir: bir kez gösterilir.
fn sort_warnings(w: &mut Vec<Diag>) {
    w.sort_by_key(|w| (w.span.file, w.span.line));
    let mut seen = HashSet::new();
    w.retain(|d| seen.insert((d.span.file, d.span.start, d.msg.clone())));
}

/// Dönüş: program, uyarılar, tipsiz yakala dallarının analizden çıkan tipleri, (tipsiz dal var mı, keşifte
/// yok sayılan dal oldu mu)
#[allow(clippy::type_complexity)]
fn check_pass(
    parsed: Vec<ParsedModule>,
    src: &Sources,
    tahmin: &HashMap<(u32, u32), Type>,
    kesif: bool,
) -> CResult<(Program, Vec<Diag>, HashMap<(u32, u32), Type>, (bool, bool))> {

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
        dosya_adlari: src.files.iter().map(|f| f.name.clone()).collect(),
        lambda_stack: Vec::new(),
        hata_turleri: [0; 6],
        tahmin: tahmin.clone(),
        kesif,
        tipsiz_var: false,
        yutulan: false,
        tsubst: HashMap::new(),
        instances: HashMap::new(),
        genel_lambdalar: Vec::new(),
        tur_kalip: HashMap::new(),
        tur_inst: HashMap::new(),
        tur_instances: HashMap::new(),
        tur_snapshot: HashMap::new(),
        miras_hazir: false,
        ortak_adlar: HashMap::new(),
        ortak_fonk: HashMap::new(),
        yasak_tip: Vec::new(),
        alan_varsayilan: HashMap::new(),
        varsayilan_yigini: Vec::new(),
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
    c.declare_hata_types();

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
                        let d = FnDecl { name: f.name.clone(), span: f.span, generics: vec![], params: f.params.clone(), ret: f.ret.clone(), body: vec![], yeniden: Yeniden::Yok };
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

    // 3. Türlerin içeriği (alanlar, seçenekler, metot imzaları); generic tür kopyaları bu sırada eklenebilir
    let mut tid = 0;
    while tid < c.type_decls.len() {
        c.resolve_type_decl(tid)?;
        tid += 1;
    }
    let pending = c.pending_keys.borrow_mut().take().unwrap_or_default();
    for (k, span) in pending {
        if let Err(why) = c.key_type_check(&k) {
            return err(span, why);
        }
    }
    // 4. Fonksiyon imzaları
    for fid in 0..c.funcs.len() {
        if c.funcs[fid].owner.is_none() && !c.kalip_mi(fid) {
            c.yasak_tip = c.ortak_yasak(c.funcs[fid].ortak_of);
            c.resolve_func_sig(fid)?;
            c.yasak_tip.clear();
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
    // 7a. Varsayılan değerler (hiç kullanılmasalar da hataları görünsün)
    for fid in 0..c.funcs.len() {
        if c.funcs[fid].generics.is_empty() {
            for i in 0..c.funcs[fid].varsayilan.len() {
                if c.funcs[fid].varsayilan[i].is_some() {
                    c.fonk_varsayilan(fid, i)?;
                }
            }
        }
    }
    for t in 0..c.types.len() {
        for f in c.own_fields[t].clone() {
            if c.alan_varsayilani_var(&f) {
                c.alan_varsayilan(&f)?;
            }
        }
    }
    // 7. Fonksiyon gövdeleri (generic kopyalar denetlenirken yenileri eklenebilir)
    let mut fid = 0;
    while fid < c.funcs.len() {
        c.check_body(fid)?;
        fid += 1;
    }
    // 8. Sabit nesneler üzerinde değiştiren metot çağrısı var mı?
    c.resolve_mutation()?;
    // 9. Fırlatma analizi: yakala dallarına hangi değerlerin gelebileceği
    let tipsiz = c.firlat_analizi(&mut modules_code);

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
            hata_turleri: c.hata_turleri,
        },
        c.warnings,
        tipsiz,
        (c.tipsiz_var, c.yutulan),
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

    /// Hazır hata türleri: `tür Hata(mesaj: Yazı)` ve alt türleri (Hata.SıfıraBölme ...). Her dosyada görünür.
    fn declare_hata_types(&mut self) {
        let sp = Span { file: u32::MAX, ..Default::default() };
        let yazi = TypeExpr { kind: TypeKind::Path { path: vec!["Yazı".into()], generics: vec![], args: None, dims: vec![] }, span: sp };
        for (i, name) in HATA_TURLERI.iter().enumerate() {
            let (parents, fields) = if i == 0 {
                (vec![], vec![Param { name: "mesaj".into(), ty: yazi.clone(), sabit: false, span: sp, varsayilan: None }])
            } else {
                (vec![(vec!["Hata".to_string()], sp)], vec![])
            };
            let d = TurDecl { name: name.to_string(), span: sp, generics: vec![], parents, fields, methods: vec![], alanlar: vec![], ortak: vec![], ortak_fonk: vec![] };
            let tid = self.types.len();
            self.types.push(TypeDef {
                name: name.to_string(),
                kind: TypeDefKind::Tur { fields: vec![], parents: vec![], ancestors: vec![] },
                interfaces: vec![],
                methods: HashMap::new(),
            });
            self.type_decls.push((0, sp, Some(TypeDecl::Tur(d))));
            self.own_fields.push(Vec::new());
            self.type_methods.push(Vec::new());
            for m in &mut self.mods {
                m.types.insert(name.to_string(), tid);
            }
            self.hata_turleri[i] = tid;
        }
    }

    /// Hata ya da onun alt türü mü?
    pub(crate) fn is_hata(&self, t: &Type) -> bool {
        matches!(t, Type::Named(id) if self.is_subtype(*id, self.hata_turleri[0]))
    }

    fn declare_type(&mut self, mi: usize, name: &str, span: Span, decl: TypeDecl) -> CResult<()> {
        if is_builtin_type_name(name) {
            return err(span, format!("'{name}' dilin hazır tiplerinden biri, başka bir ad seçin."));
        }
        if self.mods[mi].types.contains_key(name) || self.mods[mi].funcs.contains_key(name) {
            return err(span, format!("'{name}' bu dosyada zaten tanımlı."));
        }
        let tid = self.types.len();
        let mut decl = Some(decl);
        if let Some(TypeDecl::Tur(d)) = &decl {
            if !d.generics.is_empty() {
                for (g, sp) in &d.generics {
                    if is_builtin_type_name(g) || self.mods[mi].types.contains_key(g) || g == name {
                        return err(*sp, format!("'{g}' bir tipin adı; tip parametresine başka bir ad verin (T, Eleman gibi)."));
                    }
                }
                let gs = d.generics.iter().map(|g| g.0.clone()).collect();
                self.tur_kalip.insert(tid, (gs, decl.take().unwrap()));
            }
        }
        let kind = match decl.as_ref().unwrap_or_else(|| &self.tur_kalip[&tid].1) {
            TypeDecl::Tur(_) => TypeDefKind::Tur { fields: vec![], parents: vec![], ancestors: vec![] },
            TypeDecl::Arayuz(_) => TypeDefKind::Arayuz { sigs: vec![] },
            TypeDecl::Secenek(_) => TypeDefKind::Secenek { variants: vec![] },
        };
        self.types.push(TypeDef { name: name.to_string(), kind, interfaces: vec![], methods: HashMap::new() });
        let tur = match decl.as_ref().unwrap_or_else(|| &self.tur_kalip[&tid].1) {
            TypeDecl::Tur(d) => Some(d.clone()),
            _ => None,
        };
        self.type_decls.push((mi, span, decl));
        self.own_fields.push(Vec::new());
        self.type_methods.push(Vec::new());
        self.mods[mi].types.insert(name.to_string(), tid);
        if let Some(d) = tur {
            self.declare_ortak(mi, tid, &d)?;
        }
        Ok(())
    }

    /// Ortak fonksiyonun türü generic ise kullanılamayan tip parametreleri
    pub(crate) fn ortak_yasak(&self, ortak_of: Option<TypeId>) -> Vec<String> {
        ortak_of.and_then(|t| self.tur_kalip.get(&t)).map(|k| k.0.clone()).unwrap_or_default()
    }

    /// Generic türün kopyası ise kalıbı (ortak değerler kalıba aittir)
    pub(crate) fn ortak_kok(&self, t: TypeId) -> TypeId {
        self.tur_inst.get(&t).map(|x| x.0).unwrap_or(t)
    }

    /// `ortak değişken` adları ve `ortak fonksiyon`lar
    fn declare_ortak(&mut self, mi: usize, tid: TypeId, d: &TurDecl) -> CResult<()> {
        if d.ortak.is_empty() && d.ortak_fonk.is_empty() {
            return Ok(());
        }
        let mut adlar: Vec<String> = Vec::new();
        for s in &d.ortak {
            let StmtKind::Ata { name, name_span, value, .. } = &s.kind else { unreachable!("ortak değişken") };
            // generic türün ortak değeri bütün kopyalar için tektir: değeri baştan belli olmalı
            if !d.generics.is_empty() && value.is_none() {
                let d = self.kw_name(crate::lexer::Kw::Degisken);
                return err(*name_span, format!("Generic türün ortak değerine başlangıç değeri verilmeli: ortak {d} {name} = ..."));
            }
            if adlar.contains(name) {
                return err(*name_span, format!("'{name}' bu türde iki kez tanımlanmış."));
            }
            adlar.push(name.clone());
        }
        for f in &d.ortak_fonk {
            if adlar.contains(&f.name) || self.ortak_fonk.contains_key(&(tid, f.name.clone())) {
                return err(f.span, format!("'{}' bu türde iki kez tanımlanmış.", f.name));
            }
            let fid = self.funcs.len();
            self.funcs.push(FuncInfo {
                name: f.name.clone(),
                module: mi,
                owner: None,
                params: vec![],
                param_sabit: vec![],
                ret: Type::Hic,
                span: f.span,
                yeniden: Yeniden::Yok,
                generics: vec![],
                varsayilan: f.params.iter().map(|p| p.varsayilan.clone()).collect(),
                decl: Some(f.clone()),
                snapshot: None,
                body: None,
                dis: None,
                inst: None,
                ortak_of: Some(tid),
            });
            self.ortak_fonk.insert((tid, f.name.clone()), fid);
        }
        self.ortak_adlar.insert(tid, adlar);
        Ok(())
    }

    fn declare_func(&mut self, mi: usize, d: FnDecl) -> CResult<()> {
        if self.mods[mi].funcs.contains_key(&d.name) || self.mods[mi].types.contains_key(&d.name) {
            return err(d.span, format!("'{}' bu dosyada zaten tanımlı.", d.name));
        }
        for (g, sp) in &d.generics {
            if is_builtin_type_name(g) || self.mods[mi].types.contains_key(g) {
                return err(*sp, format!("'{g}' bir tipin adı; tip parametresine başka bir ad verin (T, Eleman gibi)."));
            }
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
            generics: d.generics.iter().map(|g| g.0.clone()).collect(),
            varsayilan: d.params.iter().map(|p| p.varsayilan.clone()).collect(),
            decl: Some(d),
            snapshot: None,
            body: None,
            dis: None,
            inst: None,
            ortak_of: None,
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
            Type::Func(ps, r) => {
                let ps: Vec<String> = ps.iter().map(|t| self.type_name(t)).collect();
                if **r == Type::Hic {
                    format!("Fonksiyon({})", ps.join(", "))
                } else {
                    format!("Fonksiyon({}) -> {}", ps.join(", "), self.type_name(r))
                }
            }
        }
    }

    pub(crate) fn resolve_type(&mut self, te: &TypeExpr, mi: usize) -> CResult<Type> {
        match &te.kind {
            TypeKind::Func { generics, params, ret } => {
                if !generics.is_empty() {
                    return err(
                        te.span,
                        "Her tiple çalışan fonksiyon tipi (Fonksiyon<T>(...)) sadece bir fonksiyonun parametresinin tipi olarak yazılabilir: fonksiyon ikisine(f: Fonksiyon<T>(T) -> T)",
                    );
                }
                let mut ps = Vec::new();
                for p in params {
                    let t = self.resolve_type(p, mi)?;
                    self.no_hic_value(&t, p.span)?;
                    ps.push(t);
                }
                let r = match ret {
                    Some(r) => self.resolve_type(r, mi)?,
                    None => Type::Hic,
                };
                Ok(Type::Func(ps, Box::new(r)))
            }
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
                // hiçbir bilgisayarın belleğine sığmayacak boyut: program başlamadan söyle. Daha küçükleri
                // çalışırken denenir, bellek yetmezse "Bellek yetmedi." hatası verilir.
                let eleman = dims.iter().flatten().try_fold(1u64, |a, &d| a.checked_mul(d as u64)).unwrap_or(u64::MAX);
                if eleman > SABIT_LISTE_SINIRI {
                    return err(
                        te.span,
                        format!(
                            "Sabit boyutlu listede çok fazla değer var ({} eleman); bu kadarı hiçbir bilgisayarın belleğine sığmaz.",
                            group_digits(&eleman.to_string())
                        ),
                    );
                }
                for d in dims.iter().rev() {
                    t = Type::List(Box::new(t), *d);
                }
                Ok(t)
            }
        }
    }

    fn resolve_base_type(
        &mut self,
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
            if let Some(t) = self.tsubst.get(name) {
                if !generics.is_empty() || args.is_some() {
                    return err(span, format!("'{name}' bir tip parametresi; '<...>' ya da parantez almaz."));
                }
                return Ok(t.clone());
            }
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
        // hazır hata türleri: Hata.SıfıraBölme ...
        if path.len() == 2 && path[0] == "Hata" && !self.mods[mi].namespaces.contains_key("Hata") {
            return match self.mods[mi].types.get(&format!("Hata.{}", path[1])) {
                Some(&tid) => Ok(Type::Named(tid)),
                None => err(span, hata_turu_yok(&path[1])),
            };
        }
        // kullanıcı tipleri
        let mut module = mi;
        for (i, part) in path[..path.len() - 1].iter().enumerate() {
            match self.mods[module].namespaces.get(part) {
                Some(&m) => module = m,
                None => return err(span, format!("'{}' diye bir dosya adı yok.", path[..=i].join("."))),
            }
        }
        if path.len() == 1 && self.yasak_tip.iter().any(|g| g == name) {
            return err(
                span,
                format!("'{name}' burada kullanılamaz: generic türün ortak değerleri ve fonksiyonları bütün kopyalar için tektir, tip parametresine bağlı olamaz."),
            );
        }
        if let Some(&tid) = self.mods[module].types.get(name) {
            if let Some((gs, _)) = self.tur_kalip.get(&tid) {
                let gs = gs.clone();
                if args.is_some() || generics.is_empty() {
                    let ornek = vec!["Tam"; gs.len()].join(", ");
                    return err(span, format!("'{name}' generic bir tür; içindeki tipleri yazın: {name}<{ornek}>"));
                }
                if generics.len() != gs.len() {
                    return err(span, format!("'{name}' {} tip alır ({name}<{}>), ama {} yazılmış.", gs.len(), gs.join(", "), generics.len()));
                }
                let mut ts = Vec::new();
                for g in generics {
                    let t = self.resolve_type(g, mi)?;
                    self.no_hic_value(&t, g.span)?;
                    ts.push(t);
                }
                return Ok(Type::Named(self.tur_instance(tid, ts, span)?));
            }
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
        let mut names: Vec<String> = ["Tam", "Ondalık", "Yazı", "Mantık", "Liste", "Sözlük", "Hiç", "Hata"].iter().map(|s| s.to_string()).collect();
        names.extend(self.mods[mi].types.keys().filter(|k| !k.contains('.')).cloned());
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

    /// Generic tür kopyasıysa tip parametreleri kopyanın tipleriyle çözülür; metotları da kopyaya göre
    /// denetlenecek şekilde işaretlenir.
    pub(crate) fn resolve_type_decl(&mut self, tid: TypeId) -> CResult<()> {
        let Some((kalip, ts, call)) = self.tur_inst.get(&tid).cloned() else {
            return self.resolve_type_decl_inner(tid);
        };
        let binds: Vec<(String, Type)> = self.tur_kalip[&kalip].0.iter().cloned().zip(ts).collect();
        let eski = std::mem::replace(&mut self.tsubst, binds.iter().cloned().collect());
        let r = self.resolve_type_decl_inner(tid);
        self.tsubst = eski;
        r?;
        for fid in self.type_methods[tid].clone() {
            self.funcs[fid].inst = Some(Inst { kalip: Kalip::Tur(kalip), binds: binds.clone(), call, genel_baglar: vec![] });
        }
        Ok(())
    }

    fn resolve_type_decl_inner(&mut self, tid: TypeId) -> CResult<()> {
        let (mi, _span, decl) = &mut self.type_decls[tid];
        let mi = *mi;
        // generic tür kalıbı ya da zaten çözülmüş kopya
        let Some(decl) = decl.take() else { return Ok(()) };
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
                self.varsayilan_sirasi(&d.fields)?;
                for p in &d.fields {
                    if fields.iter().any(|f| *f.name == *p.name) {
                        return err(p.span, format!("'{}' alanı iki kez tanımlanmış.", p.name));
                    }
                    let ty = self.resolve_type(&p.ty, mi)?;
                    self.no_hic_value(&ty, p.ty.span)?;
                    if let Some(v) = &p.varsayilan {
                        self.alan_varsayilan.insert((tid, p.name.clone()), v.clone());
                    }
                    fields.push(FieldInfo { name: p.name.as_str().into(), ty, sabit: p.sabit, origin: tid, kurucuda: true });
                }
                // gövdedeki alanlar: değişken değer = 0
                for s in &d.alanlar {
                    let StmtKind::Ata { sabit, name, name_span, ty, value, .. } = &s.kind else { unreachable!("tür alanı") };
                    if fields.iter().any(|f| *f.name == **name) || d.methods.iter().any(|m| m.name == *name) {
                        return err(*name_span, format!("'{name}' bu türde zaten tanımlı."));
                    }
                    let Some(v) = value else {
                        let d = self.kw_name(if *sabit { crate::lexer::Kw::Sabit } else { crate::lexer::Kw::Degisken });
                        return err(*name_span, format!("Türün içindeki alana başlangıç değeri verilmeli: {d} {name} = 0"));
                    };
                    let t = self.govde_alan_tipi(mi, ty.as_ref(), v, name)?;
                    self.no_hic_value(&t, *name_span)?;
                    self.alan_varsayilan.insert((tid, name.clone()), v.clone());
                    fields.push(FieldInfo { name: name.as_str().into(), ty: t, sabit: *sabit, origin: tid, kurucuda: false });
                }
                if let TypeDefKind::Tur { parents: ps, .. } = &mut self.types[tid].kind {
                    *ps = parents;
                }
                self.types[tid].interfaces = interfaces;
                for o in self.ortak_adlar.get(&self.ortak_kok(tid)).into_iter().flatten().chain(d.ortak_fonk.iter().map(|f| &f.name)) {
                    if fields.iter().any(|f| *f.name == **o) || d.methods.iter().any(|m| m.name == *o) {
                        return err(d.span, format!("'{o}' hem ortak hem de alan ya da fonksiyon adı olarak kullanılmış."));
                    }
                }
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
                    if let Some(p) = v.fields.iter().find(|p| p.varsayilan.is_some()) {
                        return err(p.span, "Seçeneklerin değerlerine varsayılan değer yazılamaz.");
                    }
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
                    if let Some(p) = s.params.iter().find(|p| p.varsayilan.is_some()) {
                        return err(p.span, "Arayüzdeki fonksiyon imzalarına varsayılan değer yazılamaz; değeri türdeki fonksiyona yazın.");
                    }
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
        // imza çözülürken generic tür kopyaları (ve metotları) eklenebilir: numara sonra alınır
        let (params, sabits, ret) = self.sig_of(&d, mi)?;
        let fid = self.funcs.len();
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
            varsayilan: d.params.iter().map(|p| p.varsayilan.clone()).collect(),
            decl: Some(d),
            snapshot: None,
            body: None,
            dis: None,
            generics: vec![],
            inst: None,
            ortak_of: None,
        });
        Ok(fid)
    }

    pub(crate) fn sig_of(&mut self, d: &FnDecl, mi: usize) -> CResult<(Vec<(String, Type)>, Vec<bool>, Type)> {
        let mut params: Vec<(String, Type)> = Vec::new();
        let mut sabits = Vec::new();
        self.varsayilan_sirasi(&d.params)?;
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
        for &tid in &order {
            self.inherit_one(tid)?;
        }
        self.miras_hazir = true;
        Ok(())
    }

    /// Bir türün alanları (ebeveynlerinkiyle), metot ve dağıtım tabloları; ebeveynleri önceden kurulmuş olmalı.
    pub(crate) fn inherit_one(&mut self, tid: TypeId) -> CResult<()> {
        {
            let span = self.type_decls[tid].1;
            match self.types[tid].kind.clone() {
                TypeDefKind::Tur { parents, .. } => {
                    // alanlar
                    let mut fields: Vec<FieldInfo> = Vec::new();
                    let mut ancestors: Vec<TypeId> = Vec::new();
                    let mut interfaces = self.types[tid].interfaces.clone();
                    for &p in &parents {
                        for f in &self.fields_of(p).to_vec() {
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

/// Sabit boyutlu bir listenin en fazla eleman sayısı: 2^37 eleman (Tam(64) ile 1 TB). Bunun altında
/// kalan ama belleğe sığmayan listeler çalışırken "Bellek yetmedi." hatası verir.
const SABIT_LISTE_SINIRI: u64 = 1 << 37;

const DIS_TIPLER: &str = "Kullanılabilen tipler: Tam(8/16/32/64), Tam(N, +), Ondalık, Ondalık(32), Mantık, Yazı.";

/// C'ye verilebilen tipler
fn dis_type_ok(t: &Type) -> bool {
    matches!(t, Type::Int { bits: Some(8 | 16 | 32 | 64), sign: Sign::Signed | Sign::Pos } | Type::Float(32 | 64) | Type::Bool | Type::Str)
}

pub(crate) fn starts_upper(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_uppercase())
}

pub(crate) fn is_builtin_type_name(s: &str) -> bool {
    matches!(s, "Tam" | "Ondalık" | "Yazı" | "Mantık" | "Liste" | "Sözlük" | "Hiç" | "Fonksiyon" | "Hata")
}

pub(crate) fn hata_turu_yok(name: &str) -> String {
    let adlar: Vec<&str> = HATA_TURLERI[1..].iter().map(|n| &n[5..]).collect();
    format!("'Hata' içinde '{name}' diye bir hata türü yok. Hazır hata türleri: {}.", adlar.join(", "))
}

pub(crate) fn to_us(s: &str) -> String {
    for (tr, us) in BUILTIN_NAMES {
        if s == *tr {
            return us.to_string();
        }
    }
    s.to_string()
}
