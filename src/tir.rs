// Tip denetiminden geçmiş ara gösterim (TIR). Yorumlayıcı ve LLVM derleyicisi bunu kullanır.
// Burada her isim çözülmüş, her ifadenin tipi bellidir.

use crate::ast::BinOp;
use crate::types::{Type, TypeId};
use num_bigint::BigInt;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;

/// Küçük tam sayı anahtarları için hızlı hash (FxHash).
#[derive(Default, Clone, Copy)]
pub struct FxHasher(u64);

impl Hasher for FxHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        // 8'er bayt (uzun yazı anahtarlarında bayt bayt işlemekten çok daha hızlı)
        let mut parcalar = bytes.chunks_exact(8);
        for p in &mut parcalar {
            self.write_u64(u64::from_le_bytes(p.try_into().expect("8 bayt")));
        }
        for &b in parcalar.remainder() {
            self.write_u64(b as u64);
        }
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_u32(&mut self, i: u32) {
        self.write_u64(i as u64);
    }
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
}

pub type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

pub type FuncId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Local(u32),
    Global(u32),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Loc {
    pub file: u32,
    pub line: u32,
}

#[derive(Clone, Debug)]
pub struct TExpr {
    pub kind: TK,
    pub ty: Type,
    pub loc: Loc,
}

#[derive(Clone, Debug)]
pub enum TFPart {
    Lit(Rc<str>),
    Expr(TExpr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    /// Değerleri boşlukla ayırıp yazar, satır sonu ekler
    Yazdir,
    /// Tek bir yazıyı olduğu gibi yazar (satır sonu yok): `yazdır(..., ayraç: ..., son: ...)` buna çevrilir
    Yaz,
    Oku,
}

/// Hazır metotlar (liste.ekle, yazı.say...). Yeni metot eklemek için adımlar:
/// belgeler/GELISTIRICI.md, "Dile hazır metot eklemek".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BM {
    // Liste
    Ekle,
    ArayaEkle,
    Sil,
    Cikar,
    Uzunluk,
    Icerir,
    Bul,
    Sirala,
    TersCevir,
    Temizle,
    /// liste.adım(n): her n. eleman (yeni liste)
    LAdim,
    /// yeni liste döndürenler: sıralı(), tersi()
    Sirali,
    Tersi,
    // fonksiyon alanlar (lambda): ilk argüman fonksiyon değeri
    Donusum,
    Filtre,
    Hepsi,
    Herhangi,
    /// sırala(göre: f): listeyi değiştirir
    SiralaGore,
    SiraliGore,
    // sayı listeleri
    Toplam,
    EnBuyuk,
    EnKucuk,
    // Sözlük
    DUzunluk,
    DIcerir,
    DSil,
    Anahtarlar,
    Degerler,
    DTemizle,
    // Yazı
    SUzunluk,
    BuyukHarf,
    KucukHarf,
    Karakter,
    SIcerir,
    Bol,
    Kirp,
    Tama,
    Ondaliga,
    /// değiştir(eski, yeni), başlar(p), biter(p), bul(p) -> Tam veya Hiç, say(p), tekrar(n)
    Degistir,
    Baslar,
    Biter,
    SBul,
    Say,
    Tekrar,
    /// Liste<Yazı>.birleşik(ayraç)
    Birlesik,
    /// f-yazı biçimleri: {x:.2} (Ondalık -> Yazı), {n:,} (sayı yazısını grupla), {ad:<10} (genişlik, dolgu, kip)
    Hassas,
    Grupla,
    Hizala,
    // Ondalık
    FTama,
    // Her tip
    Yaziya,
}

#[derive(Clone, Debug)]
pub enum Recv {
    Place(TPlace),
    Value(Box<TExpr>),
}

#[derive(Clone, Debug)]
pub enum TPlace {
    Var(Slot),
    Bu,
    /// Alan, alan numarasıyla (Program::field_names)
    Field(Box<TPlace>, u32),
    ListIndex(Box<TPlace>, Box<TExpr>),
    DictKey(Box<TPlace>, Box<TExpr>),
}

#[derive(Clone, Debug)]
pub enum TK {
    /// Sayı sabiti; 64 bite sığıyorsa ikinci değer hazır olarak tutulur.
    Int(BigInt, Option<i64>),
    Float(f64),
    Str(Rc<String>),
    Bool(bool),
    Hic,
    Var(Slot),
    Bu,
    FStr(Vec<TFPart>),
    List(Vec<TExpr>),
    Dict(Vec<(TExpr, TExpr)>),
    /// Tipin varsayılan değeri (sabit boyutlu listeler, boş liste/sözlük)
    Default,
    /// `a..b`, `a..=b` ve `(a..b).adım(n)`: Liste<Tam(64)> değeri. Döngüde liste oluşturulmadan sayılır.
    Range { start: Box<TExpr>, end: Box<TExpr>, inclusive: bool, step: Option<Box<TExpr>> },
    Neg(Box<TExpr>),
    Not(Box<TExpr>),
    /// İki taraf da ifadenin kendi tipine çevrilmiştir.
    Arith(BinOp, Box<TExpr>, Box<TExpr>),
    Concat(Box<TExpr>, Box<TExpr>),
    /// İki taraf aynı tipe çevrilmiştir (sayılarda).
    Cmp(BinOp, Box<TExpr>, Box<TExpr>),
    And(Box<TExpr>, Box<TExpr>),
    Or(Box<TExpr>, Box<TExpr>),
    /// Otomatik genişletme / üst tipe çevirme (veri kaybı yok).
    Convert(Box<TExpr>),
    /// `olarak` ile açık dönüştürme (çalışma anında kontrol edilir).
    Olarak(Box<TExpr>),
    Call(FuncId, Vec<TExpr>),
    /// Dinamik çağrı: gerçek fonksiyon nesnenin türüne göre seçilir.
    Method { recv: Recv, view: TypeId, name: Rc<str>, mid: u32, args: Vec<TExpr> },
    /// Ebeveyn metodunu doğrudan çağırma: `Kişi.tanıt()`
    Direct { func: FuncId, recv: Recv, args: Vec<TExpr> },
    Builtin(Builtin, Vec<TExpr>),
    BMethod { recv: Recv, m: BM, args: Vec<TExpr> },
    New(TypeId, Vec<TExpr>),
    Variant(TypeId, u32, Vec<TExpr>),
    Field(Box<TExpr>, u32),
    Index(Box<TExpr>, Box<TExpr>),
    /// Liste ya da yazı dilimi: `l[a..b]` (sınırlar boş olabilir: `l[2..]`, `l[..-1]`)
    Slice { base: Box<TExpr>, start: Option<Box<TExpr>>, end: Option<Box<TExpr>>, inclusive: bool },
    Match(Box<TMatch>),
    /// İsimsiz fonksiyon değeri: fonksiyonu ve yakaladığı değerler (oluşturulduğu andaki kopyaları).
    /// `bu`: metot içinde oluşturulduysa ve nesneyi kullanıyorsa nesnenin kopyası.
    Lambda { func: FuncId, caps: Vec<TExpr>, bu: Option<Box<TExpr>> },
    /// İsimli fonksiyonun değer olarak kullanılması: `uygula(kare, 3)`
    FuncRef(FuncId),
    /// Fonksiyon değerini çağırma: `f(3)`
    CallValue(Box<TExpr>, Vec<TExpr>),
    /// `dene` içinde `T veya Hata` döndüren çağrı: Hata ise fırlatılır, değilse T değeri
    Unwrap(Box<TExpr>),
}

#[derive(Clone, Debug)]
pub enum TPat {
    Int(BigInt),
    Float(f64),
    Str(Rc<str>),
    Bool(bool),
    Hic,
    Wild,
    Range(BigInt, BigInt, bool),
    Variant { ty: TypeId, idx: u32, binds: Vec<Option<Slot>> },
    TypeTest { ty: Type, bind: Option<Slot> },
}

#[derive(Clone, Debug)]
pub enum TArmBody {
    Expr(TExpr),
    Block(Vec<TS>),
}

#[derive(Clone, Debug)]
pub struct TArm {
    pub pats: Option<Vec<TPat>>,
    pub guard: Option<TExpr>,
    pub body: TArmBody,
}

#[derive(Clone, Debug)]
pub struct TMatch {
    pub scrut: TExpr,
    pub arms: Vec<TArm>,
    pub loc: Loc,
}

/// `yakala` dalı
#[derive(Clone, Debug)]
pub struct TCatch {
    /// None: tipsiz `yakala diğer` (geri kalan her şey)
    pub ty: Option<Type>,
    pub bind: Option<Slot>,
    /// Değişkenin tipi (tipsizde: blokta fırlatılabilenlerin 'veya'sı)
    pub bind_ty: Type,
    pub body: Vec<TS>,
    /// Bu dala uyabilen fırlatılmış değer tipleri (statik tipleri); ikinci değer: nesnenin gerçek türü
    /// ayrıca kontrol edilmeli mi (Hata tipinde fırlatılan değer Hata.Dönüşüm dalına uyabilir).
    /// Tip denetiminden sonra fırlatma analizi (firlat.rs) doldurur.
    pub kaynak: Vec<(Type, bool)>,
    pub span: crate::diag::Span,
}

/// Çalışma anı hatalarının türü: yakalanınca hangi `Hata.X` nesnesi olacağı (Program::hata_turleri sırası).
pub mod hata_turu {
    pub const GENEL: u8 = 0;
    pub const SIFIRA_BOLME: u8 = 1;
    pub const INDEKS: u8 = 2;
    pub const ANAHTAR: u8 = 3;
    pub const TASMA: u8 = 4;
    pub const DONUSUM: u8 = 5;
    /// Yakalanamayan hatalar: çok derin özyineleme, Ctrl+C, bellek yetmedi, belirsiz değişken
    pub const YAKALANAMAZ: u8 = 255;
}

/// Hazır hata türlerinin adları (hata_turu sırasıyla)
pub const HATA_TURLERI: [&str; 6] = ["Hata", "Hata.SıfıraBölme", "Hata.İndeks", "Hata.Anahtar", "Hata.Taşma", "Hata.Dönüşüm"];

#[derive(Clone, Debug)]
pub enum TS {
    Let(Slot, TExpr),
    /// Değeri sonra atanacak değişken (sadece tipini bildirir)
    Declare(Slot, Type),
    Assign(TPlace, TExpr, Loc),
    Compound(TPlace, BinOp, TExpr, Type, Loc),
    Expr(TExpr),
    If(Vec<(TExpr, Vec<TS>)>, Option<Vec<TS>>),
    Kere(TExpr, Vec<TS>),
    ForEach { iter: TExpr, binds: Vec<Slot>, body: Vec<TS> },
    While(TExpr, Vec<TS>),
    CFor(Box<TS>, TExpr, Box<TS>, Vec<TS>),
    Break,
    Continue,
    Return(Option<TExpr>),
    Match(TMatch),
    RunModule(usize),
    /// `dene { } yakala ...` (dal yoksa hatalar yok sayılır)
    Dene { body: Vec<TS>, catches: Vec<TCatch> },
    /// `fırlat değer`
    Throw(TExpr),
}

/// Dış (C) fonksiyon: hangi kütüphanede, C'deki adı
#[derive(Clone, Debug)]
pub struct DisInfo {
    pub lib: String,
    pub symbol: String,
    /// kütüphane adının yazıldığı yer (hata mesajı için)
    pub lib_loc: Loc,
}

#[derive(Clone, Debug)]
pub struct TFunc {
    pub dis: Option<DisInfo>,
    /// Lambda ise yakalanan değerlerin (yerel yuva numarası, tip); çağrıda bu yuvalara yazılır.
    /// None: isimli fonksiyon.
    pub caps: Option<Vec<(u32, Type)>>,
    pub params: Vec<Type>,
    pub ret: Type,
    pub body: Vec<TS>,
    pub nslots: u32,
    pub owner: Option<TypeId>,
    pub loc: Loc,
    /// Kaynaktaki yeri (dosya, bayt): REPL'de fonksiyon değerlerini hücreden hücreye eşlemek için
    pub key: (u32, u32),
}

#[derive(Clone, Debug)]
pub struct FieldInfo {
    pub name: Rc<str>,
    pub ty: Type,
    pub sabit: bool,
    pub origin: TypeId,
    /// Oluştururken verilir mi (başlıktaki alanlar); gövdedeki `değişken`/`sabit` alanları verilmez
    pub kurucuda: bool,
}

#[derive(Clone, Debug)]
pub struct VariantInfo {
    pub name: Rc<str>,
    pub fields: Vec<(Rc<str>, Type)>,
}

#[derive(Clone, Debug)]
pub struct Sig {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub ret: Type,
}

#[derive(Clone, Debug)]
pub enum MethodRes {
    Impl(FuncId),
    Ambiguous(Vec<TypeId>),
}

#[derive(Clone, Debug)]
pub enum TypeDefKind {
    Tur { fields: Vec<FieldInfo>, parents: Vec<TypeId>, ancestors: Vec<TypeId> },
    Secenek { variants: Vec<VariantInfo> },
    Arayuz { sigs: Vec<Sig> },
}

#[derive(Clone, Debug)]
pub struct TypeDef {
    pub name: String,
    pub kind: TypeDefKind,
    pub interfaces: Vec<TypeId>,
    /// Türün kendi gözünden metotları
    pub methods: HashMap<String, MethodRes>,
}

pub struct Program {
    pub funcs: Vec<TFunc>,
    pub types: Vec<TypeDef>,
    pub nglobals: u32,
    pub modules: Vec<Vec<TS>>,
    pub main_module: usize,
    /// (gerçek tür, bakış türü, metot numarası) -> fonksiyon
    pub dispatch: FxMap<(TypeId, TypeId, u32), FuncId>,
    /// Alan adları; TK::Field ve TPlace::Field bu listedeki sırayı kullanır.
    pub field_names: Vec<Rc<str>>,
    /// field_pos[tür][alan numarası] = nesnedeki yeri (yoksa u32::MAX)
    pub field_pos: Vec<Vec<u32>>,
    /// Hazır hata türleri (hata_turu sırasıyla): Hata, Hata.SıfıraBölme, ...
    pub hata_turleri: [TypeId; 6],
}

impl Program {
    /// Hata ya da onun alt türü mü?
    pub fn is_hata(&self, t: &Type) -> bool {
        matches!(t, Type::Named(id) if self.is_subtype(*id, self.hata_turleri[0]))
    }

    pub fn is_subtype(&self, child: TypeId, parent: TypeId) -> bool {
        if child == parent {
            return true;
        }
        let t = &self.types[child];
        if t.interfaces.contains(&parent) {
            return true;
        }
        matches!(&t.kind, TypeDefKind::Tur { ancestors, .. } if ancestors.contains(&parent))
    }
}

/// Fonksiyon değeri alan hazır metot mu?
pub fn is_hof(m: BM) -> bool {
    matches!(m, BM::Donusum | BM::Filtre | BM::Hepsi | BM::Herhangi | BM::SiralaGore | BM::SiraliGore)
}

/// İfade, bir değişkeni değiştirebilecek kod (çağrı, okuma, eşleştir...) içermiyor mu?
pub fn is_pure(e: &TExpr) -> bool {
    match &e.kind {
        TK::Int(..) | TK::Float(_) | TK::Str(_) | TK::Bool(_) | TK::Hic | TK::Var(_) | TK::Bu | TK::Default => true,
        TK::Neg(x) | TK::Not(x) | TK::Convert(x) | TK::Olarak(x) | TK::Field(x, _) => is_pure(x),
        TK::Arith(_, a, b) | TK::Concat(a, b) | TK::Cmp(_, a, b) | TK::And(a, b) | TK::Or(a, b) | TK::Index(a, b) => {
            is_pure(a) && is_pure(b)
        }
        TK::Slice { base, start, end, .. } => is_pure(base) && start.as_deref().is_none_or(is_pure) && end.as_deref().is_none_or(is_pure),
        TK::FStr(ps) => ps.iter().all(|p| match p {
            TFPart::Lit(_) => true,
            TFPart::Expr(x) => is_pure(x),
        }),
        TK::List(v) | TK::New(_, v) | TK::Variant(_, _, v) => v.iter().all(is_pure),
        TK::Lambda { caps, bu, .. } => caps.iter().all(is_pure) && bu.as_deref().is_none_or(is_pure),
        TK::FuncRef(_) => true,
        TK::Dict(v) => v.iter().all(|(a, b)| is_pure(a) && is_pure(b)),
        TK::Range { start, end, step, .. } => is_pure(start) && is_pure(end) && step.as_deref().is_none_or(is_pure),
        TK::BMethod { recv: Recv::Value(x), m, args } => {
            // fonksiyon alan metotlar kullanıcının kodunu çalıştırır
            !matches!(m, BM::Ekle | BM::ArayaEkle | BM::Sil | BM::Cikar | BM::Sirala | BM::TersCevir | BM::Temizle | BM::DSil | BM::DTemizle)
                && !is_hof(*m)
                && is_pure(x)
                && args.iter().all(is_pure)
        }
        _ => false,
    }
}

