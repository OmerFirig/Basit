// LLVM derleyicisi: TIR'dan LLVM IR metni üretir, clang ile makine koduna çevirir (`kolay derle`).
//
// Dosyalar:
//   mod.rs      genel yapı: program, fonksiyonlar, IR yazma yardımcıları
//   duzen.rs    değerlerin bellek düzeni (yazmaç ve depolama biçimi), 'veya' paketleme, tip tanımları
//   sahip.rs    referans sayımı: geçici değerlerin sahipliği, tutma/bırakma
//   komut.rs    komutlar, atama yerleri, döngüler, eşleştir
//   ifade.rs    ifadeler, aritmetik, dönüştürmeler, çağrılar
//   yerlesik.rs hazır metotlar (liste, sözlük, yazı...) ve yazıya çevirme
//
// Değer biçimleri: Tam(N) -> iN (Tam(N, -) -> iN+1), esnek Tam -> etiketli i64 (bkz. runtime/sayilar.c),
// Ondalık -> half/float/double, Mantık -> i1, Yazı/Liste/Sözlük/tür/seçenek/arayüz -> ptr (referans
// sayımlı yığın nesnesi), 'veya' -> %veya {etiket, kelime, kelime}. Aralık (a..b) bir listedir; döngüde
// liste oluşturulmadan sayılır (komut.rs for_range).
//
// Sahiplik kuralları (sahip.rs):
// - Bir ifadenin ürettiği yeni değer "sahipli" geçicidir; kullanıldıktan sonra bırakılır ya da bir
//   değişkene/alana/dönüş değerine devredilir.
// - Yerel değişkenden okunan değer ödünçtür (ifade bitmeden değişkeni kimse değiştiremez).
//   Dosya değişkeni, nesne alanı, liste elemanı ve sözlük değerinden okunan değer tutulur, çünkü aynı
//   ifadedeki bir çağrı onu değiştirebilir.
// - Fonksiyonlar parametrelerini girişte tutar, yerel değişkenlerini her dönüşte bırakır.
// - Değer semantiği: liste/sözlük/nesne değiştirilmeden önce tek sahibi olunması sağlanır
//   (kolay_benzersiz: paylaşılıyorsa kopyalanır).

mod duzen;
mod ifade;
mod komut;
mod sahip;
mod yerlesik;

use crate::diag::{CResult, Diag, Sources, Span};
use crate::tir::*;
use crate::types::{Type, TypeId};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

const RUNTIME_PARCALAR: [&str; 6] = [
    include_str!("../../runtime/kolay.h"),
    include_str!("../../runtime/cekirdek.c"),
    include_str!("../../runtime/sayilar.c"),
    include_str!("../../runtime/yazilar.c"),
    include_str!("../../runtime/genel.c"),
    include_str!("../../runtime/koleksiyon.c"),
];

const DECLS: &str = "\
%Tip = type { i64, i64, i64, i64, i64, i64, i64, ptr, ptr, ptr, i64, ptr, ptr, ptr, ptr, i64 }
%Varyant = type { ptr, i64, ptr, ptr, ptr, ptr, i64 }
%veya = type { i64, i64, i64 }
@kolay_yazi_tipi = external constant %Tip
declare void @kolay_calistir(ptr)
declare void @kolay_hata(ptr, ptr, ptr) noreturn
declare void @kolay_hata_tur(i32, ptr, ptr, ptr) noreturn
declare void @kolay_firlat(ptr, ptr, ptr, ptr) noreturn
declare void @kolay_dene_gir(ptr)
declare void @kolay_dene_cik(ptr)
declare ptr @kolay_dene_tip(ptr)
declare ptr @kolay_dene_deger(ptr)
declare void @kolay_dene_birak(ptr)
declare void @kolay_dene_bitti(ptr)
declare void @kolay_dene_yeniden(ptr) noreturn
declare void @kolay_kacir(ptr)
@kolay_temizlik_ust = external global ptr
declare void @kolay_hata_sayi(ptr, i64, ptr, ptr, ptr) noreturn
declare void @kolay_tasma(ptr, ptr, i128, i128, i32, ptr, ptr, ptr, i32, ptr, ptr) noreturn
declare void @kolay_sigmiyor(i128, i32, ptr, ptr, ptr, ptr) noreturn
declare void @kolay_sigmiyor_f(double, ptr, ptr, ptr, ptr) noreturn
declare void @kolay_sigmiyor_esnek(i64, ptr, ptr, ptr, ptr) noreturn
declare void @kolay_cevrilemez_f(double, ptr, ptr) noreturn
declare void @kolay_kere_negatif(i128, i32, ptr, ptr) noreturn
declare void @kolay_sayi_hata(i128, i32, ptr, ptr, ptr, ptr) noreturn
declare void @kolay_tur_degil(ptr, ptr, ptr, ptr) noreturn
declare void @kolay_tur_degil_nesne(ptr, ptr, ptr, ptr) noreturn
declare void @kolay_indeks_hatasi(i64, i64, ptr, ptr) noreturn
declare void @kolay_liste_boyut_hatasi(ptr, i64, i64, ptr, ptr) noreturn
declare void @kolay_tut(ptr)
declare void @kolay_birak(ptr)
declare void @kolay_esnek_tut(i64)
declare void @kolay_esnek_birak(i64)
declare void @kolay_veya_tut(ptr, i64, i64)
declare void @kolay_veya_birak(ptr, i64, i64)
declare ptr @kolay_benzersiz(ptr)
declare void @kolay_bellek_yeri(ptr, ptr)
declare ptr @kolay_nesne_yeni(ptr, i64)
declare ptr @kolay_bos()
declare ptr @kolay_c_yazi(ptr)
declare void @kolay_tampon_bosalt()
declare ptr @kolay_ekle(ptr, ptr)
declare ptr @kolay_birlestir(ptr, ptr)
declare ptr @kolay_tam_yazi(i128, i32)
declare ptr @kolay_ondalik_yazi(double, i32)
declare ptr @kolay_yaziya(ptr, ptr)
declare void @kolay_yaz(ptr)
declare void @kolay_yaz_bosluk()
declare void @kolay_satir()
declare ptr @kolay_oku(ptr)
declare zeroext i1 @kolay_yazi_esit(ptr, ptr)
declare i32 @kolay_yazi_kars(ptr, ptr)
declare i64 @kolay_yazi_uzunluk(ptr)
declare ptr @kolay_buyuk_harf(ptr)
declare ptr @kolay_kucuk_harf(ptr)
declare ptr @kolay_kirp(ptr)
declare zeroext i1 @kolay_icerir(ptr, ptr)
declare ptr @kolay_degistir(ptr, ptr, ptr, ptr, ptr)
declare zeroext i1 @kolay_baslar(ptr, ptr)
declare zeroext i1 @kolay_biter(ptr, ptr)
declare i64 @kolay_yazi_bul(ptr, ptr)
declare i64 @kolay_say(ptr, ptr, ptr, ptr)
declare ptr @kolay_tekrar(ptr, i64, ptr, ptr)
declare ptr @kolay_birlesik(ptr, ptr)
declare ptr @kolay_hassas(double, i64)
declare ptr @kolay_grupla(ptr)
declare ptr @kolay_hizala(ptr, i64, ptr, i64)
declare ptr @kolay_karakter(ptr, i64, ptr, ptr)
declare void @kolay_yazi_tama(ptr, ptr, ptr)
declare void @kolay_yazi_ondaliga(ptr, ptr, ptr)
declare ptr @kolay_yazi_bol(ptr, ptr, ptr)
declare ptr @kolay_yazi_sonraki(ptr, ptr)
declare zeroext i1 @kolay_esit(ptr, ptr, ptr)
declare void @kolay_varsayilan(ptr, ptr)
declare ptr @kolay_liste_yeni(ptr, i64)
declare ptr @kolay_liste_ekle(ptr, ptr)
declare ptr @kolay_liste_araya(ptr, i64, ptr, ptr, ptr)
declare void @kolay_liste_sil(ptr, i64, ptr, ptr)
declare void @kolay_liste_cikar(ptr, ptr, ptr, ptr)
declare void @kolay_liste_temizle(ptr)
declare void @kolay_liste_ters(ptr)
declare void @kolay_liste_sirala(ptr)
declare zeroext i1 @kolay_liste_icerir(ptr, ptr)
declare i64 @kolay_liste_bul(ptr, ptr)
declare ptr @kolay_liste_dilim(ptr, ptr, i64, i64, i64)
declare ptr @kolay_liste_adim(ptr, i64, ptr, ptr, ptr)
declare ptr @kolay_liste_anahtarla(ptr, ptr)
declare ptr @kolay_aralik_liste(i64, i64, i1 zeroext, i64, ptr, ptr, ptr)
declare ptr @kolay_liste_sirali(ptr)
declare ptr @kolay_liste_tersi(ptr)
declare ptr @kolay_yazi_dilim(ptr, i64, i64, i64)
declare ptr @kolay_sozluk_yeni(ptr)
declare ptr @kolay_sozluk_al(ptr, ptr, ptr, ptr)
declare ptr @kolay_sozluk_yer(ptr, ptr)
declare zeroext i1 @kolay_sozluk_icerir(ptr, ptr)
declare void @kolay_sozluk_sil(ptr, ptr, ptr, ptr)
declare void @kolay_sozluk_temizle(ptr)
declare i64 @kolay_sozluk_uzunluk(ptr)
declare ptr @kolay_sozluk_anahtarlar(ptr, ptr)
declare ptr @kolay_sozluk_degerler(ptr, ptr)
declare i64 @kolay_sozluk_girdi_sayisi(ptr)
declare ptr @kolay_sozluk_anahtar_i(ptr, i64)
declare ptr @kolay_sozluk_deger_i(ptr, i64)
declare i64 @kolay_esnek_islem(i64, i64, i64, i64, ptr, ptr, ptr)
declare i64 @kolay_esnek_neg(i64, i64, ptr, ptr, ptr)
declare i32 @kolay_esnek_kars(i64, i64)
declare i64 @kolay_esnek_tamdan(i128, i32)
declare i64 @kolay_esnek_metinden(ptr)
declare double @kolay_esnek_ondalik(i64)
declare ptr @kolay_esnek_yazi(i64)
declare i128 @kolay_esnek_boyutluya(i64, i64, i64, ptr, ptr, ptr, ptr)
declare i64 @kolay_esnek_kisitla(i64, i64, ptr, ptr, ptr, ptr)
declare i64 @kolay_esnek_ondaliktan(double, ptr, ptr)
declare i64 @kolay_esnek_i64(i64, ptr, ptr, ptr)
declare i64 @kolay_esnek_kere(i64, ptr, ptr)
";

// Windows'ta _setjmp ikinci bir değer alır (çerçeve adresi); boş verilince longjmp yığını
// açmadan (Linux'taki gibi) doğrudan döner.
#[cfg(not(windows))]
const SETJMP_DECL: &str = "declare i32 @_setjmp(ptr) returns_twice\n";
#[cfg(windows)]
const SETJMP_DECL: &str = "declare i32 @_setjmp(ptr, ptr) returns_twice\n";
#[cfg(not(windows))]
pub(super) const SETJMP_ARGS: &str = "";
#[cfg(windows)]
pub(super) const SETJMP_ARGS: &str = ", ptr null";

// Kolay fonksiyonlarının konduğu bölüm. Windows'ta (COFF) '$' sonrası bağlayıcının sıralamasıdır:
// çalışma kütüphanesi bölümün başını ve sonunu kolay_kod$a ve kolay_kod$z ile işaretler.
#[cfg(not(windows))]
const KOD_BOLUMU: &str = "kolay_kod";
#[cfg(windows)]
const KOD_BOLUMU: &str = "kolay_kod$m";

const BELIRSIZ: &str = "Değişken henüz belirsiz (bir fonksiyon, değişken tanımlanmadan önce çağrıldı).";

pub fn compile(prog: &Program, sources: &Sources, out: &Path, keep_ir: bool) -> CResult<()> {
    // dış kütüphaneler: yorumlayıcıdaki gibi önce bulunup kontrol edilir (aynı hata mesajları)
    crate::dis::resolve(prog)?;
    let mut libs: Vec<String> = Vec::new();
    for f in &prog.funcs {
        if let Some(d) = &f.dis {
            let arg = if d.lib.contains('/') || d.lib.contains(".so") { d.lib.clone() } else { format!("-l{}", d.lib) };
            // Windows'ta matematik ve C fonksiyonları C'nin kendi kütüphanesindedir (ayrıca bağlanmaz)
            if cfg!(windows) && (d.lib == "m" || d.lib == "c") {
                continue;
            }
            if !libs.contains(&arg) {
                libs.push(arg);
            }
        }
    }
    let ir = Gen::new(prog, sources).program()?;
    let tool = |msg: &str| Diag::new(Span { file: u32::MAX, ..Default::default() }, msg.to_string());
    if Command::new("clang").arg("--version").output().is_err() {
        return Err(tool(if cfg!(windows) {
            "Derlemek için clang gerekiyor ama bulunamadı. Kurmak için: winget install LLVM.LLVM (sonra terminali yeniden açın)"
        } else {
            "Derlemek için clang gerekiyor ama bulunamadı. Kurmak için: sudo apt install clang llvm (Arch: sudo pacman -S clang llvm)"
        }));
    }
    let dir = std::env::temp_dir().join(format!("kolay-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| tool(&format!("Geçici klasör oluşturulamadı: {e}")))?;
    let ll = dir.join("program.ll");
    std::fs::write(&ll, &ir).map_err(|e| tool(&format!("Dosya yazılamadı: {e}")))?;
    if keep_ir {
        let p = out.with_extension("ll");
        std::fs::write(&p, &ir).map_err(|e| tool(&format!("Dosya yazılamadı: {e}")))?;
    }
    // Çalışma kütüphanesi LLVM bitcode olarak programla birleştirilir: küçük fonksiyonları
    // (referans sayımı, eşitlik...) LLVM programın içine yerleştirebilir. llvm-link yoksa
    // önceden derlenmiş nesne dosyası bağlanır.
    let linked = dir.join("hepsi.bc");
    let input = match runtime_object(&dir, true) {
        Ok(bc)
            if Command::new("llvm-link")
                .arg("-o")
                .arg(&linked)
                .arg(&ll)
                .arg(&bc)
                .output()
                .is_ok_and(|o| o.status.success()) =>
        {
            vec![linked.clone()]
        }
        _ => match runtime_object(&dir, false) {
            Ok(o) => vec![ll.clone(), o],
            Err(e) => {
                let _ = std::fs::remove_dir_all(&dir);
                return Err(tool(&e));
            }
        },
    };
    let res = Command::new("clang")
        .args(["-O2", "-w", "-o"])
        .arg(out)
        .args(&input)
        .args(sistem_kutuphaneleri())
        .args(&libs)
        .output()
        .map_err(|e| tool(&format!("clang çalıştırılamadı: {e}")));
    let _ = std::fs::remove_dir_all(&dir);
    let res = res?;
    if !res.status.success() {
        return Err(tool(&format!(
            "clang derlemeyi tamamlayamadı (bu bir derleyici hatası, lütfen bildirin):\n{}",
            String::from_utf8_lossy(&res.stderr)
        )));
    }
    Ok(())
}

/// Programın bağlandığı sistem kütüphaneleri.
#[cfg(not(windows))]
fn sistem_kutuphaneleri() -> Vec<String> {
    vec!["-lm".into(), "-lpthread".into()]
}

/// Windows: 128 bitlik bölme ve Ondalık(16) dönüştürmeleri için clang'ın kendi yardımcı kütüphanesi
/// (compiler-rt) gerekir; MSVC'nin kütüphanelerinde yoktur.
#[cfg(windows)]
fn sistem_kutuphaneleri() -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(o) = Command::new("clang").args(["--rtlib=compiler-rt", "-print-libgcc-file-name"]).output() {
        let yol = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !yol.is_empty() && Path::new(&yol).exists() {
            v.push(yol);
        }
    }
    v
}

/// Çalışma kütüphanesini bir kez derleyip önbellekte tutar (her derlemede yeniden derlenmesin).
/// `bitcode`: LLVM bitcode (.bc) mı, nesne dosyası (.o) mı
fn runtime_object(tmp: &Path, bitcode: bool) -> Result<std::path::PathBuf, String> {
    use std::hash::{Hash, Hasher};
    let source: String = RUNTIME_PARCALAR.concat();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut h);
    let name = format!("kolay_runtime_{:016x}.{}", h.finish(), if bitcode { "bc" } else { "o" });
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| if cfg!(windows) { std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from) } else { None })
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))
        .map(|d| d.join("kolay"));
    if let Some(dir) = &cache {
        let obj = dir.join(&name);
        if obj.exists() {
            return Ok(obj);
        }
    }
    let src = tmp.join("kolay_runtime.c");
    std::fs::write(&src, &source).map_err(|e| format!("Dosya yazılamadı: {e}"))?;
    let obj = tmp.join(&name);
    let mut cmd = Command::new("clang");
    cmd.args(["-O2", "-w", "-c"]);
    if bitcode {
        cmd.arg("-emit-llvm");
    }
    let res = cmd
        .arg("-o")
        .arg(&obj)
        .arg(&src)
        .output()
        .map_err(|e| format!("clang çalıştırılamadı: {e}"))?;
    if !res.status.success() {
        return Err(format!(
            "Çalışma kütüphanesi derlenemedi (bu bir derleyici hatası, lütfen bildirin):\n{}",
            String::from_utf8_lossy(&res.stderr)
        ));
    }
    if let Some(dir) = cache {
        // Önbelleğe yazılamazsa geçici dosya kullanılır.
        if std::fs::create_dir_all(&dir).is_ok() {
            let tmp_target = dir.join(format!("{name}.{}", std::process::id()));
            if std::fs::copy(&obj, &tmp_target).is_ok() && std::fs::rename(&tmp_target, dir.join(&name)).is_ok() {
                // eski sürümlerin önbellek dosyaları silinir
                let ext = if bitcode { ".bc" } else { ".o" };
                if let Ok(rd) = std::fs::read_dir(&dir) {
                    for e in rd.flatten() {
                        let n = e.file_name().to_string_lossy().into_owned();
                        if n.starts_with("kolay_runtime_") && n.ends_with(ext) && n != name {
                            let _ = std::fs::remove_file(e.path());
                        }
                    }
                }
                return Ok(dir.join(&name));
            }
            let _ = std::fs::remove_file(&tmp_target);
        }
    }
    Ok(obj)
}

pub(super) struct FnState {
    body: String,
    allocas: String,
    tmp: u32,
    terminated: bool,
    /// Şu an yazılan bloğun etiketi (phi için)
    cur: String,
    /// döngüler: (devam etiketi, dur etiketi, döngünün içinde olduğu dene sayısı)
    loops: Vec<(String, String, usize)>,
    /// İçinde bulunulan dene bloklarının çerçeveleri (dıştan içe)
    deneler: Vec<String>,
    /// Fonksiyonda dene bloğu var mı (varsa yerel değişkenler belleğe zorlanır, bkz. finish_fn)
    has_dene: bool,
    /// Yerel değişkenlerin ve alloca'ların adları
    adlar: Vec<String>,
    ret: Type,
    local_tys: HashMap<u32, Type>,
    /// Bırakılması gereken (sahipli) geçici değerler
    owned: HashSet<String>,
    owned_tys: HashMap<String, Type>,
    /// Fonksiyondan dönerken bırakılacak yerel değişkenler: (adres, tip)
    rc_slots: Vec<(String, Type)>,
    /// Fonksiyon mu (değilse bir dosyanın üst düzey kodu)
    is_func: bool,
    /// Fonksiyonun "dene içi" kopyası mı (temizlik kaydı tutar, bkz. temizlik_kaydi)
    dene_modu: bool,
    module: usize,
    /// Metotlarda nesnenin bulunduğu yerin adresi
    bu: Option<String>,
    owner: Option<TypeId>,
}

pub(super) struct Gen<'p> {
    prog: &'p Program,
    sources: &'p Sources,
    consts: String,
    strings: HashMap<String, String>,
    cstrings: HashMap<String, String>,
    global_tys: HashMap<u32, Type>,
    /// Dosya değişkeninin tanımlandığı dosya (modül)
    global_owner: HashMap<u32, usize>,
    intrinsics: HashMap<String, String>,
    /// Fonksiyon tablosu: { adres, konum, kaynak satırı } (derin özyineleme mesajı için)
    fn_table: Vec<String>,
    /// Tip tanımları (çalışma kütüphanesi için): tip -> global adı
    tips: HashMap<Type, String>,
    tip_defs: String,
    /// Tür alanlarının ofsetleri [tür][alan sırası]; seçeneklerde [tür] boş
    field_offsets: Vec<Vec<i64>>,
    object_sizes: Vec<i64>,
    /// Seçenek varyantlarının değer ofsetleri ve boyutları [tür][varyant]
    variant_offsets: Vec<Vec<Vec<i64>>>,
    variant_sizes: Vec<Vec<i64>>,
    /// (bakış türü, metot numarası) -> [(gerçek tür, fonksiyon)]
    dispatch: HashMap<(TypeId, u32), Vec<(TypeId, FuncId)>>,
    /// değer olarak kullanılan isimli fonksiyonlar
    fref_used: HashSet<FuncId>,
    /// "Dene içi" kopyası istenen fonksiyonlar: üretilecekler ve hepsi (bkz. temizlik_kaydi)
    dene_istek: Vec<FuncId>,
    dene_kopya: HashSet<FuncId>,
    /// Programda dene bloğu var mı (yoksa "dene içi" kopyalar hiç üretilmez)
    dene_kullanilir: bool,
    label: u32,
    f: FnState,
}

impl<'p> Gen<'p> {
    fn new(prog: &'p Program, sources: &'p Sources) -> Self {
        let mut dispatch: HashMap<(TypeId, u32), Vec<(TypeId, FuncId)>> = HashMap::new();
        for (&(dyn_t, view, mid), &f) in &prog.dispatch {
            dispatch.entry((view, mid)).or_default().push((dyn_t, f));
        }
        for v in dispatch.values_mut() {
            v.sort();
        }
        Gen {
            prog,
            sources,
            consts: String::new(),
            strings: HashMap::new(),
            cstrings: HashMap::new(),
            global_tys: HashMap::new(),
            global_owner: HashMap::new(),
            intrinsics: HashMap::new(),
            fn_table: Vec::new(),
            tips: HashMap::new(),
            tip_defs: String::new(),
            field_offsets: Vec::new(),
            object_sizes: Vec::new(),
            variant_offsets: Vec::new(),
            variant_sizes: Vec::new(),
            dispatch,
            fref_used: HashSet::new(),
            dene_istek: Vec::new(),
            dene_kopya: HashSet::new(),
            dene_kullanilir: prog.funcs.iter().any(|f| komut::dene_var(&f.body)) || prog.modules.iter().any(|m| komut::dene_var(m)),
            label: 0,
            f: Self::fresh_fn(Type::Hic, false, 0, None),
        }
    }

    fn fresh_fn(ret: Type, is_func: bool, module: usize, owner: Option<TypeId>) -> FnState {
        FnState {
            body: String::new(),
            allocas: String::new(),
            tmp: 0,
            terminated: false,
            cur: "L0".into(),
            loops: Vec::new(),
            deneler: Vec::new(),
            has_dene: false,
            adlar: Vec::new(),
            ret,
            local_tys: HashMap::new(),
            owned: HashSet::new(),
            owned_tys: HashMap::new(),
            rc_slots: Vec::new(),
            is_func,
            dene_modu: false,
            module,
            bu: if owner.is_some() { Some("%bu".into()) } else { None },
            owner,
        }
    }

    // ------------------------------------------------------------ IR yazma yardımcıları

    /// Kolay yazısı olarak sabit (başlıklı, hiç silinmeyen).
    fn str_const(&mut self, s: &str) -> String {
        if let Some(n) = self.strings.get(s) {
            return n.clone();
        }
        let name = format!("@.s{}", self.strings.len());
        let len = s.len() + 1;
        let _ = writeln!(
            self.consts,
            "{name} = private unnamed_addr constant {{ i64, ptr, i64, i64, [{len} x i8] }} {{ i64 -1, ptr @kolay_yazi_tipi, i64 {}, i64 0, [{len} x i8] c\"{}\\00\" }}, align 16",
            s.len(),
            llescape(s)
        );
        self.strings.insert(s.to_string(), name.clone());
        name
    }

    /// Düz C yazısı (hata mesajları ve adlar için).
    fn cstr(&mut self, s: &str) -> String {
        if let Some(n) = self.cstrings.get(s) {
            return n.clone();
        }
        let name = format!("@.c{}", self.cstrings.len());
        let len = s.len() + 1;
        let _ = writeln!(self.consts, "{name} = private unnamed_addr constant [{len} x i8] c\"{}\\00\"", llescape(s));
        self.cstrings.insert(s.to_string(), name.clone());
        name
    }

    /// Hata mesajı için konum ("dosya.kolay, satır 3") ve kaynak satırı (yorumlayıcıdaki biçimde).
    fn where_(&mut self, loc: Loc) -> (String, String) {
        let name = self.sources.name(loc.file).to_string();
        let konum = self.cstr(&format!("{name}, satır {}", loc.line));
        let line_text = self
            .sources
            .files
            .get(loc.file as usize)
            .and_then(|f| f.text.lines().nth(loc.line.saturating_sub(1) as usize))
            .map(|l| format!("  {} | {}\n", loc.line, l.trim_end()))
            .unwrap_or_default();
        let kod = self.cstr(&line_text);
        (konum, kod)
    }

    fn tmp(&mut self) -> String {
        self.f.tmp += 1;
        format!("%t{}", self.f.tmp)
    }

    fn new_label(&mut self) -> String {
        self.label += 1;
        format!("L{}", self.label)
    }

    fn emit(&mut self, s: &str) {
        if self.f.terminated {
            let l = self.new_label();
            let _ = writeln!(self.f.body, "{l}:");
            self.f.cur = l;
            self.f.terminated = false;
        }
        let _ = writeln!(self.f.body, "  {s}");
    }

    fn term(&mut self, s: &str) {
        self.emit(s);
        self.f.terminated = true;
    }

    fn block(&mut self, l: &str) {
        if !self.f.terminated {
            let _ = writeln!(self.f.body, "  br label %{l}");
        }
        let _ = writeln!(self.f.body, "{l}:");
        self.f.cur = l.to_string();
        self.f.terminated = false;
    }

    fn alloca(&mut self, ty: &str) -> String {
        self.f.tmp += 1;
        let name = format!("%a{}", self.f.tmp);
        let _ = writeln!(self.f.allocas, "  {name} = alloca {ty}, align 16");
        self.f.adlar.push(name.clone());
        name
    }

    fn intrinsic(&mut self, name: &str, decl: String) {
        self.intrinsics.entry(name.to_string()).or_insert(decl);
    }

    fn slot_ptr(&self, s: Slot) -> String {
        match s {
            Slot::Local(i) => format!("%s{i}"),
            Slot::Global(i) => format!("@g{i}"),
        }
    }

    fn slot_ty(&self, s: Slot) -> Option<&Type> {
        match s {
            Slot::Local(i) => self.f.local_tys.get(&i),
            Slot::Global(i) => self.global_tys.get(&i),
        }
    }

    fn error_call(&mut self, msg: &str, loc: Loc) {
        self.error_call_tur(hata_turu::GENEL, msg, loc);
    }

    /// Türü belli çalışma hatası (dene içindeyse Hata.X olarak yakalanabilir)
    fn error_call_tur(&mut self, tur: u8, msg: &str, loc: Loc) {
        let m = self.cstr(msg);
        let (k, c) = self.where_(loc);
        self.emit(&format!("call void @kolay_hata_tur(i32 {tur}, ptr {m}, ptr {k}, ptr {c})"));
        self.term("unreachable");
    }

    fn unsupported<T>(&self, loc: Loc, what: &str) -> CResult<T> {
        let name = self.sources.name(0);
        Err(Diag::new(
            Span { file: loc.file, line: loc.line, col: 0, start: 0, end: 0 },
            format!("Derleyici henüz {what} desteklemiyor. Bu programı yorumlayıcıyla çalıştırabilirsiniz: kolay {name}"),
        ))
    }

    /// Koşula göre dallanır; `cold` dal nadiren çalışır (hata yolu).
    fn br_cold(&mut self, cond: &str, cold: &str, hot: &str) {
        self.term(&format!("br i1 {cond}, label %{cold}, label %{hot}, !prof !1"));
    }

    // ------------------------------------------------------------ dosya değişkenleri

    /// Dosya değişkeni bu noktada atanmamış olabilir mi? (bir fonksiyonun içinden ya da başka bir
    /// dosyadan okunuyorsa evet; kendi dosyasının üst düzeyinde akış analizi bunu zaten önler)
    fn global_needs_check(&self, g: u32) -> bool {
        self.f.is_func || self.global_owner.get(&g) != Some(&self.f.module)
    }

    fn global_check(&mut self, g: u32, loc: Loc) {
        if !self.global_needs_check(g) {
            return;
        }
        let h = self.tmp();
        self.emit(&format!("{h} = load i1, ptr @g{g}_h, !tbaa !2"));
        let (bad, ok) = (self.new_label(), self.new_label());
        self.term(&format!("br i1 {h}, label %{ok}, label %{bad}, !prof !0"));
        self.block(&bad);
        self.error_call_tur(hata_turu::YAKALANAMAZ, BELIRSIZ, loc);
        self.block(&ok);
    }

    // ------------------------------------------------------------ program

    fn program(mut self) -> CResult<String> {
        let prog = self.prog;
        self.compute_layouts();
        for tid in 0..prog.types.len() {
            self.named_tip(tid);
        }
        for (mi, m) in prog.modules.iter().enumerate() {
            let mut own = HashMap::new();
            collect_slots_stmts(m, &mut own, true, prog);
            for (g, v) in own {
                self.global_owner.insert(g, mi);
                self.global_tys.insert(g, v);
            }
        }
        let mut funcs_ir = String::new();
        let mut globals_ir = String::new();
        let mut gl: Vec<(u32, Type)> = self.global_tys.iter().map(|(k, v)| (*k, v.clone())).collect();
        gl.sort_by_key(|g| g.0);
        for (i, t) in &gl {
            let ty = self.llty(t);
            let zero = self.zero_value(t);
            let _ = writeln!(globals_ir, "@g{i} = internal global {ty} {zero}, align 16");
            let _ = writeln!(globals_ir, "@g{i}_h = internal global i1 false");
        }
        for (fid, func) in prog.funcs.iter().enumerate() {
            funcs_ir.push_str(&self.function(fid, func, false)?);
        }
        for (mi, code) in prog.modules.iter().enumerate() {
            let _ = writeln!(globals_ir, "@m{mi}_ran = internal global i1 false");
            funcs_ir.push_str(&self.module_fn(mi, code)?);
        }
        // dene bloklarının içinden çağrılan fonksiyonların kopyaları (onlar da başkalarını isteyebilir)
        while let Some(fid) = self.dene_istek.pop() {
            funcs_ir.push_str(&self.function(fid, &prog.funcs[fid], true)?);
        }
        funcs_ir.push_str(&self.fref_wrappers());
        let n = self.fn_table.len();
        let _ = writeln!(
            globals_ir,
            "@kolay_fonk_sayi = constant i64 {n}\n@kolay_fonk_tablo = constant [{n} x {{ ptr, ptr, ptr }}] [{}]",
            self.fn_table.join(", ")
        );
        self.subtype_table(&mut globals_ir);
        let ht: Vec<String> = prog.hata_turleri.iter().map(|t| format!("ptr @tur.{t}")).collect();
        let _ = writeln!(globals_ir, "@kolay_hata_tipleri = constant [6 x ptr] [{}]", ht.join(", "));
        let _ = writeln!(globals_ir, "@kolay_tur_sayisi = constant i64 {}", prog.types.len());
        let mut out = String::new();
        let _ = writeln!(out, "; Kolay derleyicisinin ürettiği LLVM IR\n");
        out.push_str(DECLS);
        out.push_str(SETJMP_DECL);
        let mut intr: Vec<&String> = self.intrinsics.values().collect();
        intr.sort();
        for d in intr {
            out.push_str(d);
            out.push('\n');
        }
        out.push('\n');
        out.push_str(&self.consts);
        out.push('\n');
        out.push_str(&self.tip_defs);
        out.push('\n');
        out.push_str(&globals_ir);
        out.push('\n');
        out.push_str(&funcs_ir);
        let _ = writeln!(out, "define i32 @main() {{\n  call void @kolay_calistir(ptr @m{})\n  ret i32 0\n}}\n", prog.main_module);
        // dal olasılıkları: !0 ilk dal neredeyse her zaman, !1 ikinci dal neredeyse her zaman
        out.push_str("!0 = !{!\"branch_weights\", i32 2000, i32 1}\n!1 = !{!\"branch_weights\", i32 1, i32 2000}\n");
        // Bellek bölgeleri (TBAA): dosya değişkenleri, nesne başlıkları (sayaç, uzunluk, tip) ve
        // elemanlar/alanlar birbirinin yerine geçmez. LLVM böylece örneğin listeye yazarken döngü
        // değişkenini yazmaçta tutabilir.
        out.push_str("!10 = !{!\"kolay\"}\n!11 = !{!\"genel\", !10, i64 0}\n!12 = !{!\"baslik\", !10, i64 0}\n!13 = !{!\"eleman\", !10, i64 0}\n");
        out.push_str("!2 = !{!11, !11, i64 0}\n!3 = !{!12, !12, i64 0}\n!4 = !{!13, !13, i64 0}\n");
        Ok(out)
    }

    fn finish_fn(&mut self, header: &str) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "{header} {{");
        s.push_str("entry:\n");
        s.push_str(&self.f.allocas);
        if self.f.has_dene {
            for a in &self.f.adlar {
                let _ = writeln!(s, "  call void @kolay_kacir(ptr {a})");
            }
        }
        s.push_str("  br label %L0\nL0:\n");
        s.push_str(&self.f.body);
        s.push_str("}\n\n");
        s
    }

    /// "Dene içi" kopyada (dene_modu), hata atlamasında (longjmp) fonksiyon atlanırsa yerel değişkenlerinin
    /// bırakılması için temizlik kaydı (bkz. runtime/cekirdek.c Temizlik). Dene bloklarının içindeki
    /// doğrudan çağrılar bu kopyaları çağırır (emit_call); böylece dene dışındaki kodun maliyeti yoktur.
    fn temizlik_kaydi(&mut self, fid: FuncId) {
        let slots = self.f.rc_slots.clone();
        let n = slots.len();
        let tips: Vec<String> = slots.iter().map(|(_, t)| format!("ptr {}", self.tip(t))).collect();
        let _ = writeln!(self.tip_defs, "@tz.{fid} = internal constant {{ i64, [{n} x ptr] }} {{ i64 {n}, [{n} x ptr] [{}] }}", tips.join(", "));
        let kt = format!("{{ ptr, ptr, [{n} x ptr] }}");
        let a = &mut self.f.allocas;
        let _ = writeln!(a, "  %tz = alloca {kt}, align 16");
        let _ = writeln!(a, "  %tz.u = load ptr, ptr @kolay_temizlik_ust\n  store ptr %tz.u, ptr %tz");
        let _ = writeln!(a, "  %tz.t = getelementptr inbounds {kt}, ptr %tz, i32 0, i32 1\n  store ptr @tz.{fid}, ptr %tz.t");
        for (i, (p, _)) in slots.iter().enumerate() {
            let _ = writeln!(a, "  %tz.y{i} = getelementptr inbounds {kt}, ptr %tz, i32 0, i32 2, i32 {i}\n  store ptr {p}, ptr %tz.y{i}");
        }
        let _ = writeln!(a, "  store ptr %tz, ptr @kolay_temizlik_ust");
    }

    /// Dış (C) fonksiyon için sarmalayıcı: Kolay değerlerini C'ye çevirip çağırır.
    fn dis_wrapper(&mut self, fid: FuncId, func: &TFunc, d: &DisInfo) -> String {
        let cty = |t: &Type| -> &'static str {
            match t {
                Type::Int { bits: Some(8), .. } => "i8",
                Type::Int { bits: Some(16), .. } => "i16",
                Type::Int { bits: Some(32), .. } => "i32",
                Type::Int { .. } => "i64",
                Type::Float(32) => "float",
                Type::Float(_) => "double",
                Type::Bool => "zeroext i1",
                Type::Str => "ptr",
                _ => "void",
            }
        };
        let decl = format!(
            "declare {} @\"{}\"({})",
            cty(&func.ret),
            d.symbol,
            func.params.iter().map(|t| cty(t).to_string()).collect::<Vec<_>>().join(", ")
        );
        self.intrinsic(&format!("dış {}", d.symbol), decl);
        let mut out = String::new();
        let ret = self.llty(&func.ret);
        let params: Vec<String> = func.params.iter().enumerate().map(|(i, t)| format!("{} %p{i}", self.llty(t))).collect();
        let _ = writeln!(out, "define internal {ret} @f{fid}({}) {{", params.join(", "));
        // C fonksiyonu da ekrana yazabilir: önce Kolay'ın tamponu boşaltılır (sıra korunur)
        let _ = writeln!(out, "  call void @kolay_tampon_bosalt()");
        let mut args = Vec::new();
        for (i, t) in func.params.iter().enumerate() {
            if *t == Type::Str {
                // Kolay yazısının baytları başlıktan sonra (32. bayt), sonda 0 ile
                let _ = writeln!(out, "  %c{i} = getelementptr i8, ptr %p{i}, i64 32");
                args.push(format!("ptr %c{i}"));
            } else {
                args.push(format!("{} %p{i}", cty(t)));
            }
        }
        if func.ret == Type::Hic {
            let _ = writeln!(out, "  call void @\"{}\"({})\n  ret void\n}}\n", d.symbol, args.join(", "));
        } else if func.ret == Type::Str {
            let _ = writeln!(out, "  %r = call ptr @\"{}\"({})", d.symbol, args.join(", "));
            let _ = writeln!(out, "  %y = call ptr @kolay_c_yazi(ptr %r)\n  ret ptr %y\n}}\n");
        } else {
            let _ = writeln!(out, "  %r = call {} @\"{}\"({})\n  ret {ret} %r\n}}\n", cty(&func.ret), d.symbol, args.join(", "));
        }
        out
    }

    /// `dene`: dene bloklarının içinden çağrılan, temizlik kaydı tutan kopya (@fN.d, bkz. temizlik_kaydi)
    fn function(&mut self, fid: FuncId, func: &TFunc, dene: bool) -> CResult<String> {
        if let Some(d) = &func.dis {
            return Ok(self.dis_wrapper(fid, func, d));
        }
        self.f = Self::fresh_fn(func.ret.clone(), true, 0, func.owner);
        self.f.dene_modu = dene;
        let mut slots = HashMap::new();
        for (i, t) in func.params.iter().enumerate() {
            slots.insert(i as u32, t.clone());
        }
        if let Some(caps) = &func.caps {
            for (s, t) in caps {
                slots.insert(*s, t.clone());
            }
        }
        collect_slots_stmts(&func.body, &mut slots, false, self.prog);
        self.f.local_tys = slots;
        let ret = self.llty(&func.ret);
        let mut params = Vec::new();
        if func.caps.is_some() {
            // lambda: ilk parametre kendi nesnesi (fonksiyon adresi ve yakalanan değerler)
            params.push("ptr %env".to_string());
            if func.owner.is_some() {
                let (_, bu_off, _) = self.lambda_layout(fid);
                let _ = writeln!(self.f.allocas, "  %bu = getelementptr i8, ptr %env, i64 {}", bu_off.expect("bu"));
            }
        } else if func.owner.is_some() {
            params.push("ptr %bu".to_string());
        }
        for (i, t) in func.params.iter().enumerate() {
            params.push(format!("{} %p{i}", self.llty(t)));
        }
        let mut locals: Vec<(u32, Type)> = self.f.local_tys.iter().map(|(k, v)| (*k, v.clone())).collect();
        locals.sort_by_key(|l| l.0);
        let nparams = func.params.len() as u32;
        self.f.has_dene = komut::dene_var(&func.body);
        for (i, t) in &locals {
            let ty = self.llty(t);
            let _ = writeln!(self.f.allocas, "  %s{i} = alloca {ty}, align 16");
            self.f.adlar.push(format!("%s{i}"));
            if self.is_rc(t) {
                self.f.rc_slots.push((format!("%s{i}"), t.clone()));
                if *i >= nparams {
                    let z = self.zero_value(t);
                    let _ = writeln!(self.f.allocas, "  store {ty} {z}, ptr %s{i}");
                }
            }
        }
        for (i, t) in func.params.iter().enumerate() {
            let ty = self.llty(t);
            let _ = writeln!(self.f.allocas, "  store {ty} %p{i}, ptr %s{i}");
        }
        self.label = 0;
        // parametreler girişte tutulur (fonksiyon onları değiştirip bırakabilir)
        for (i, t) in func.params.iter().enumerate() {
            if self.is_rc(t) {
                self.retain(&format!("%p{i}"), t);
            }
        }
        // lambda: yakalanan değerler nesneden kendi yuvalarına kopyalanır (ve tutulur)
        if let Some(caps) = &func.caps {
            let (offs, _, _) = self.lambda_layout(fid);
            for ((s, t), off) in caps.iter().zip(offs) {
                let p = self.tmp();
                self.emit(&format!("{p} = getelementptr i8, ptr %env, i64 {off}"));
                let v = self.load_mem(&p, t);
                if self.is_rc(t) {
                    self.retain(&v, t);
                }
                let lt = self.llty(t);
                self.emit(&format!("store {lt} {v}, ptr %s{s}"));
            }
        }
        self.stmts(&func.body)?;
        if !self.f.terminated {
            if func.ret == Type::Hic {
                self.release_rc_slots();
                self.term("ret void");
            } else {
                self.term("unreachable");
            }
        }
        if dene {
            self.temizlik_kaydi(fid);
        }
        // Fonksiyonlar "kolay_kod" bölümüne konur: yığın taşınca çalışma kütüphanesi, hatanın
        // hangi fonksiyonda olduğunu bu bölümdeki adreslerden bulur (çalışma anında maliyeti yok).
        let (k, c) = self.where_(func.loc);
        let ad = if dene { format!("@f{fid}.d") } else { format!("@f{fid}") };
        self.fn_table.push(format!("{{ ptr, ptr, ptr }} {{ ptr {ad}, ptr {k}, ptr {c} }}"));
        Ok(self.finish_fn(&format!("define internal {ret} {ad}({}) section \"{KOD_BOLUMU}\"", params.join(", "))))
    }

    /// Lambda nesnesinin düzeni: yakalananların ofsetleri, nesnenin (bu) ofseti, toplam boyut.
    /// Nesne: başlık (16), fonksiyon adresi (8), dene bloğu içinden çağrılınca kullanılan adres (8,
    /// bkz. dene_adresi), yakalananlar.
    pub(super) fn lambda_layout(&self, fid: FuncId) -> (Vec<i64>, Option<i64>, i64) {
        let f = &self.prog.funcs[fid];
        let mut off = 32;
        let mut offs = Vec::new();
        for (_, t) in f.caps.as_deref().unwrap_or(&[]) {
            let st = self.stor(t);
            off = duzen::align(off, st.align);
            offs.push(off);
            off += st.size;
        }
        let bu = f.owner.map(|_| {
            off = duzen::align(off, 8);
            let o = off;
            off += 8;
            o
        });
        (offs, bu, duzen::align(off.max(32), 16))
    }

    /// Fonksiyon değeri nesnesindeki ikinci adres: programda dene bloğu varsa fonksiyonun "dene içi"
    /// kopyası (bkz. temizlik_kaydi), yoksa kendisi. `ad`: "@fN" ya da "@fN.deger".
    pub(super) fn dene_adresi(&mut self, fid: FuncId, ad: &str) -> String {
        if !self.dene_kullanilir {
            return ad.to_string();
        }
        if self.dene_kopya.insert(fid) {
            self.dene_istek.push(fid);
        }
        format!("{ad}.d")
    }

    /// Fonksiyon değerini çağırırken kullanılacak adresin nesnedeki yeri (bkz. dene_adresi)
    pub(super) fn cagri_adresi_ofset(&self) -> i64 {
        if self.f.dene_modu || !self.f.deneler.is_empty() { 24 } else { 16 }
    }

    /// Lambdanın başlık tipi (@lam.N): yakalanan değerlerin tipleri ve ofsetleri (bırakmak için)
    pub(super) fn lambda_tip(&mut self, fid: FuncId, ty: &Type) -> String {
        let name = format!("@lam.{fid}");
        if self.tip_defs.contains(&format!("{name} =")) {
            return name;
        }
        let f = &self.prog.funcs[fid];
        let mut tys: Vec<Type> = f.caps.as_deref().unwrap_or(&[]).iter().map(|c| c.1.clone()).collect();
        if let Some(o) = f.owner {
            tys.push(Type::Named(o));
        }
        let (mut offs, bu, _) = self.lambda_layout(fid);
        offs.extend(bu);
        let tips: Vec<String> = tys.iter().map(|t| self.tip(t)).collect();
        self.emit_fonk_tip(&name, ty, &tips, &offs);
        name
    }

    /// İsimli fonksiyonun değer hali: hiç silinmeyen sabit nesne ve sarmalayıcı (@fN.deger)
    pub(super) fn func_ref(&mut self, fid: FuncId, ty: &Type) -> String {
        let name = format!("@fref.{fid}");
        if !self.fref_used.contains(&fid) {
            self.fref_used.insert(fid);
            let tip = format!("@fref.{fid}.tip");
            self.emit_fonk_tip(&tip, ty, &[], &[]);
            let d = self.dene_adresi(fid, &format!("@f{fid}.deger"));
            let _ = writeln!(self.tip_defs, "{name} = internal global {{ i64, ptr, ptr, ptr }} {{ i64 -1, ptr {tip}, ptr @f{fid}.deger, ptr {d} }}, align 16");
        }
        name
    }

    /// Kullanılan isimli fonksiyonlar için değer sarmalayıcıları: ilk parametre (nesne) kullanılmaz.
    fn fref_wrappers(&self) -> String {
        let mut out = String::new();
        let mut v: Vec<&FuncId> = self.fref_used.iter().collect();
        v.sort();
        for &fid in v {
            let f = &self.prog.funcs[fid];
            let ret = self.llty(&f.ret);
            let ps: Vec<String> = f.params.iter().enumerate().map(|(i, t)| format!("{} %p{i}", self.llty(t))).collect();
            let args: Vec<String> = f.params.iter().enumerate().map(|(i, t)| format!("{} %p{i}", self.llty(t))).collect();
            for son in if self.dene_kullanilir { &["", ".d"][..] } else { &[""] } {
                let call = format!("call {ret} @f{fid}{son}({})", args.join(", "));
                let body = if f.ret == Type::Hic { format!("  {call}\n  ret void") } else { format!("  %r = {call}\n  ret {ret} %r") };
                let _ = writeln!(out, "define internal {ret} @f{fid}.deger{son}(ptr %env{}{}) {{\n{body}\n}}\n", if ps.is_empty() { "" } else { ", " }, ps.join(", "));
            }
        }
        out
    }

    fn module_fn(&mut self, mi: usize, code: &[TS]) -> CResult<String> {
        self.f = Self::fresh_fn(Type::Hic, false, mi, None);
        self.f.has_dene = komut::dene_var(code);
        self.label = 0;
        let ran = self.tmp();
        self.emit(&format!("{ran} = load i1, ptr @m{mi}_ran"));
        let (run, done) = (self.new_label(), self.new_label());
        self.term(&format!("br i1 {ran}, label %{done}, label %{run}"));
        self.block(&run);
        self.emit(&format!("store i1 true, ptr @m{mi}_ran"));
        self.stmts(code)?;
        // döngülerin gizli değişkenleri (dosya değişkenleri program boyunca yaşar)
        self.release_rc_slots();
        self.block(&done);
        self.term("ret void");
        Ok(self.finish_fn(&format!("define internal void @m{mi}()")))
    }
}

/// Değişken yuvası erişimi için bellek bölgesi etiketi (dosya değişkenleri)
pub(super) fn slot_tag(p: &str) -> &'static str {
    if p.starts_with("@g") { ", !tbaa !2" } else { "" }
}

pub(super) const BASLIK: &str = ", !tbaa !3";
pub(super) const ELEMAN: &str = ", !tbaa !4";

fn llescape(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if (0x20..0x7f).contains(&b) && b != b'"' && b != b'\\' {
            o.push(b as char);
        } else {
            let _ = write!(o, "\\{b:02X}");
        }
    }
    o
}

// ------------------------------------------------------------ değişken yuvalarının tipleri

/// Komutlardaki tüm değişken yuvalarını (desenlerde tanımlananlar dahil) tipleriyle toplar.
pub(crate) fn collect_slots_stmts(stmts: &[TS], out: &mut HashMap<u32, Type>, global: bool, prog: &Program) {
    for s in stmts {
        collect_slots_stmt(s, out, global, prog);
    }
}

fn put_slot(slot: &Slot, t: &Type, out: &mut HashMap<u32, Type>, global: bool) {
    match (slot, global) {
        (Slot::Local(i), false) | (Slot::Global(i), true) => {
            out.insert(*i, t.clone());
        }
        _ => {}
    }
}

fn collect_slots_stmt(s: &TS, out: &mut HashMap<u32, Type>, global: bool, prog: &Program) {
    let ex = |e: &TExpr, out: &mut HashMap<u32, Type>| collect_slots_expr(e, out, global, prog);
    match s {
        TS::Let(slot, e) => {
            put_slot(slot, &e.ty, out, global);
            ex(e, out);
        }
        TS::Declare(slot, t) => put_slot(slot, t, out, global),
        TS::Assign(p, e, _) | TS::Compound(p, _, e, _, _) => {
            collect_slots_place(p, out, global, prog);
            ex(e, out);
        }
        TS::Expr(e) => ex(e, out),
        TS::If(bs, e) => {
            for (c, b) in bs {
                ex(c, out);
                collect_slots_stmts(b, out, global, prog);
            }
            if let Some(b) = e {
                collect_slots_stmts(b, out, global, prog);
            }
        }
        TS::Kere(c, b) | TS::While(c, b) => {
            ex(c, out);
            collect_slots_stmts(b, out, global, prog);
        }
        TS::ForEach { iter, binds, body } => {
            ex(iter, out);
            match &iter.ty {
                Type::List(e, _) => put_slot(&binds[0], e, out, global),
                Type::Str => put_slot(&binds[0], &Type::Str, out, global),
                Type::Dict(k, v) => {
                    put_slot(&binds[0], k, out, global);
                    if binds.len() > 1 {
                        put_slot(&binds[1], v, out, global);
                    }
                }
                _ => put_slot(&binds[0], &crate::types::TAM64, out, global),
            }
            collect_slots_stmts(body, out, global, prog);
        }
        TS::CFor(i, c, st, b) => {
            collect_slots_stmt(i, out, global, prog);
            ex(c, out);
            collect_slots_stmt(st, out, global, prog);
            collect_slots_stmts(b, out, global, prog);
        }
        TS::Return(Some(e)) | TS::Throw(e) => ex(e, out),
        TS::Match(m) => collect_slots_match(m, out, global, prog),
        TS::Dene { body, catches } => {
            collect_slots_stmts(body, out, global, prog);
            for c in catches {
                if let Some(s) = &c.bind {
                    put_slot(s, &c.bind_ty, out, global);
                }
                collect_slots_stmts(&c.body, out, global, prog);
            }
        }
        _ => {}
    }
}

fn collect_slots_place(p: &TPlace, out: &mut HashMap<u32, Type>, global: bool, prog: &Program) {
    match p {
        TPlace::Field(b, _) => collect_slots_place(b, out, global, prog),
        TPlace::ListIndex(b, e) | TPlace::DictKey(b, e) => {
            collect_slots_place(b, out, global, prog);
            collect_slots_expr(e, out, global, prog);
        }
        _ => {}
    }
}

fn collect_slots_match(m: &TMatch, out: &mut HashMap<u32, Type>, global: bool, prog: &Program) {
    collect_slots_expr(&m.scrut, out, global, prog);
    for a in &m.arms {
        if let Some(ps) = &a.pats {
            for p in ps {
                match p {
                    TPat::Variant { ty, idx, binds } => {
                        if let TypeDefKind::Secenek { variants } = &prog.types[*ty].kind {
                            for (b, (_, ft)) in binds.iter().zip(&variants[*idx as usize].fields) {
                                if let Some(s) = b {
                                    put_slot(s, ft, out, global);
                                }
                            }
                        }
                    }
                    TPat::TypeTest { ty, bind: Some(s) } => put_slot(s, ty, out, global),
                    _ => {}
                }
            }
        }
        if let Some(g) = &a.guard {
            collect_slots_expr(g, out, global, prog);
        }
        match &a.body {
            TArmBody::Expr(e) => collect_slots_expr(e, out, global, prog),
            TArmBody::Block(b) => collect_slots_stmts(b, out, global, prog),
        }
    }
}

fn collect_slots_expr(e: &TExpr, out: &mut HashMap<u32, Type>, global: bool, prog: &Program) {
    let mut go = |x: &TExpr| collect_slots_expr(x, out, global, prog);
    match &e.kind {
        TK::FStr(parts) => {
            for p in parts {
                if let TFPart::Expr(x) = p {
                    go(x);
                }
            }
        }
        TK::List(v) | TK::Call(_, v) | TK::Builtin(_, v) | TK::New(_, v) | TK::Variant(_, _, v) => {
            for x in v {
                go(x);
            }
        }
        TK::Dict(v) => {
            for (a, b) in v {
                go(a);
                go(b);
            }
        }
        TK::Range { start, end, step, .. } => {
            go(start);
            go(end);
            if let Some(x) = step {
                go(x);
            }
        }
        TK::Slice { base, start, end, .. } => {
            go(base);
            if let Some(x) = start {
                go(x);
            }
            if let Some(x) = end {
                go(x);
            }
        }
        TK::Neg(x) | TK::Not(x) | TK::Convert(x) | TK::Olarak(x) | TK::Field(x, _) => go(x),
        TK::Arith(_, a, b) | TK::Concat(a, b) | TK::Cmp(_, a, b) | TK::And(a, b) | TK::Or(a, b) | TK::Index(a, b) => {
            go(a);
            go(b);
        }
        TK::Method { recv, args, .. } | TK::Direct { recv, args, .. } | TK::BMethod { recv, args, .. } => {
            match recv {
                Recv::Value(x) => go(x),
                Recv::Place(p) => collect_slots_place(p, out, global, prog),
            }
            for x in args {
                collect_slots_expr(x, out, global, prog);
            }
        }
        TK::Match(m) => collect_slots_match(m, out, global, prog),
        TK::Lambda { caps, bu, .. } => {
            for x in caps {
                go(x);
            }
            if let Some(b) = bu {
                go(b);
            }
        }
        TK::CallValue(fe, args) => {
            go(fe);
            for x in args {
                collect_slots_expr(x, out, global, prog);
            }
        }
        TK::Unwrap(x) => go(x),
        _ => {}
    }
}
