// REPL (etkileşimli kip): `kolay` dosya adı olmadan çalışınca açılır. Girdi/Çıktı hücreleri, değerin
// tipi soluk renkte, hatalar tek satır (kararlar: Notes_Claude.md 4.8).
//
// Oturum, hatasız girdilerin art arda eklendiği tek bir sanal dosyadır. Her yeni girdide bütün metin
// yeniden denetlenir (tipler, değişken yuvaları ve türler önceki girdilerde aynı kaldığı için), ama
// sadece yeni girdinin komutları çalıştırılır; dosya değişkenlerinin değerleri bir hücreden ötekine
// taşınır. Hatalı girdi oturuma eklenmez.

use crate::diag::{Diag, Sources};
use crate::interp::{Interp, KESME};
use crate::lexer::{Mode, BUILTIN_NAMES, KEYWORDS};
use crate::tir::TS;
use crate::types::Type;
use crate::values::{fmt_value, type_display, Value};
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::{FileHistory, History};
use rustyline::validate::Validator;
use rustyline::{Cmd, ConditionalEventHandler, Context, Editor, Event, EventContext, EventHandler, Helper, KeyEvent, Movement, RepeatCount};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;
use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::Ordering;

const YARDIM: &str = "\
Kolay etkileşimli kip. Bir komut ya da ifade yazıp Enter'a basın; ifadelerin değeri gösterilir.
'{' açık kaldıkça alt satırdan devam edilir.

Komutlar:
  :yardım              Bu yardım
  :tip <ifade>         İfadenin tipini gösterir (çalıştırmadan)
  :geçmiş              Oturumdaki girdileri listeler
  :kaydet <dosya>      Hatasız girdileri bir dosyaya yazar
  :yükle <dosya>       Bir dosyayı oturuma çalıştırır
  :temizle             Oturumu sıfırlar (değişkenler, fonksiyonlar, türler)
  :çık                 Çıkar (Ctrl+D de olur)

Çalışan bir girdiyi durdurmak için Ctrl+C.";

const KOMUTLAR: &[&str] = &[":yardım", ":tip", ":geçmiş", ":kaydet", ":yükle", ":temizle", ":çık"];

struct Renk(bool);

impl Renk {
    fn r(&self, kod: &str, s: &str) -> String {
        if self.0 { format!("\x1b[{kod}m{s}\x1b[0m") } else { s.to_string() }
    }
    fn girdi(&self, s: &str) -> String {
        self.r("32", s)
    }
    fn cikti(&self, s: &str) -> String {
        self.r("34", s)
    }
    fn hata(&self, s: &str) -> String {
        self.r("31", s)
    }
    fn uyari(&self, s: &str) -> String {
        self.r("33", s)
    }
    fn soluk(&self, s: &str) -> String {
        self.r("90", s)
    }
}

struct Oturum {
    mode: Mode,
    /// Hatasız girdiler: (hücre numarası, metin)
    girdiler: Vec<(u32, String)>,
    /// Dosya değişkenlerinin değerleri (hücreden hücreye taşınır)
    globals: Vec<Value>,
    /// Üst düzey kodu çalışmış, dahil edilen dosyalar
    calisan: HashSet<PathBuf>,
    /// Ana dosyanın (oturumun) şimdiye kadar çalışmış komut sayısı
    ts_done: usize,
    sayac: u32,
    renk: Renk,
    dir: PathBuf,
    /// Tamamlama için oturumdaki adlar
    adlar: Rc<RefCell<Vec<String>>>,
    /// Önceki programdaki fonksiyonların kaynak anahtarları (fonksiyon numarası sırasıyla)
    fkeys: Vec<(u32, u32)>,
}

impl Oturum {
    fn new(mode: Mode, renk: bool, adlar: Rc<RefCell<Vec<String>>>) -> Self {
        Oturum {
            mode,
            girdiler: Vec::new(),
            globals: Vec::new(),
            calisan: HashSet::new(),
            ts_done: 0,
            sayac: 0,
            renk: Renk(renk),
            dir: std::env::current_dir().unwrap_or_default(),
            adlar,
            fkeys: Vec::new(),
        }
    }

    fn temizle(&mut self) {
        let renk = self.renk.0;
        let adlar = self.adlar.clone();
        adlar.borrow_mut().clear();
        *self = Oturum::new(self.mode, renk, adlar);
    }

    /// Oturum metni + yeni girdi; ikinci değer yeni girdinin ilk satırı.
    fn metin(&self, girdi: &str) -> (String, u32) {
        let onceki: Vec<&str> = self.girdiler.iter().map(|g| g.1.as_str()).collect();
        let onceki = onceki.join("\n");
        if onceki.is_empty() {
            return (girdi.to_string(), 1);
        }
        let satir = onceki.lines().count() as u32 + 1;
        (format!("{onceki}\n{girdi}"), satir)
    }

    fn etiket(&self, ne: &str, n: u32) -> String {
        format!("{ne} [{n}]:")
    }

    /// Tek satırlık hata; hata başka bir dosyadaysa (dahil et) dosya ve satır da yazılır.
    fn hata_yaz(&self, n: u32, sources: &Sources, file: u32, line: u32, ana: u32, msg: &str) {
        if self.renk.0 && msg == crate::interp::DURDURULDU {
            println!(); // terminalin yazdığı ^C'nin ardından yeni satır
        }
        let yer = if file != ana && (file as usize) < sources.files.len() {
            format!(" ({}, satır {line})", sources.name(file))
        } else {
            String::new()
        };
        println!("{} {msg}", self.renk.hata(&format!("Hata [{n}]{yer}:")));
    }

    fn diag_yaz(&self, n: u32, sources: &Sources, d: &Diag, ana: u32) {
        self.hata_yaz(n, sources, d.span.file, d.span.line, ana, &d.msg);
    }

    /// Bir hücreyi denetler ve çalıştırır. `sadece_tip`: :tip komutu (çalıştırmaz).
    fn hucre(&mut self, girdi: &str, sadece_tip: bool) {
        self.sayac += 1;
        let n = self.sayac;
        let (text, start) = self.metin(girdi);
        let mut sources = Sources::default();
        let path = self.dir.join("<oturum>");
        let mods = match crate::loader::load_text(&path, text, self.mode, &mut sources) {
            Ok(m) => m,
            Err(d) => return self.diag_yaz(n, &sources, &d, 0),
        };
        let ana = mods.last().map(|m| m.file).unwrap_or(0);
        let paths: Vec<PathBuf> = mods.iter().map(|m| sources.files[m.file as usize].path.clone()).collect();
        let (prog, warnings) = match crate::check::check(mods, &sources) {
            Ok(r) => r,
            Err(d) => return self.diag_yaz(n, &sources, &d, ana),
        };
        for w in &warnings {
            if w.span.file != ana || w.span.line >= start {
                println!("{} {}", self.renk.uyari(&format!("Uyarı [{n}]:")), w.msg);
            }
        }
        let main = prog.main_module;
        let all = &prog.modules[main];
        let done = self.ts_done.min(all.len());
        let new = &all[done..];
        let (stmts, value) = match new.last() {
            Some(TS::Expr(e)) if !matches!(e.ty, Type::Hic | Type::Never) => (&new[..new.len() - 1], Some(e)),
            _ => (new, None),
        };
        if sadece_tip {
            match value {
                Some(e) if stmts.is_empty() => println!("{} {}", self.renk.cikti(&self.etiket("Çıktı", n)), type_display(&e.ty, &prog)),
                _ => println!("{} ':tip' bir ifade bekler. Örnek: :tip [1, 2].bul(2)", self.renk.hata(&format!("Hata [{n}]:"))),
            }
            return;
        }
        let dis = match crate::dis::resolve(&prog) {
            Ok(d) => d,
            Err(d) => return self.diag_yaz(n, &sources, &d, ana),
        };
        let mut it = Interp::new(&prog);
        it.dis = dis;
        let mut g = std::mem::take(&mut self.globals);
        // fonksiyon numaraları her denetimde yeniden dağıtılır (yeni bir fonksiyon araya girebilir):
        // saklanan fonksiyon değerleri kaynaktaki yerlerine göre yeni numaralarına eşlenir
        let yeni: std::collections::HashMap<(u32, u32), usize> = prog.funcs.iter().enumerate().map(|(i, f)| (f.key, i)).collect();
        let esle: Vec<usize> = self.fkeys.iter().enumerate().map(|(i, k)| yeni.get(k).copied().unwrap_or(i)).collect();
        if esle.iter().enumerate().any(|(i, j)| i != *j) {
            for v in g.iter_mut() {
                fonk_esle(v, &esle);
            }
        }
        if g.len() < prog.nglobals as usize {
            g.resize(prog.nglobals as usize, Value::Uninit);
        }
        it.globals = g;
        for (i, p) in paths.iter().enumerate() {
            if self.calisan.contains(p) {
                it.ran[i] = true;
            }
        }
        it.ran[main] = true;
        KESME.store(false, Ordering::Relaxed);
        let r = it.run_cell(stmts, value);
        for (i, p) in paths.iter().enumerate() {
            if i != main && it.ran[i] {
                self.calisan.insert(p.clone());
            }
        }
        self.globals = std::mem::take(&mut it.globals);
        self.fkeys = prog.funcs.iter().map(|f| f.key).collect();
        match r {
            Ok(v) => {
                self.girdiler.push((n, girdi.to_string()));
                self.ts_done = all.len();
                self.adlari_guncelle(girdi);
                if let (Some(v), Some(e)) = (v, value) {
                    let deger = fmt_value(&v, Some(&e.ty), &prog, false);
                    let tip = type_display(&e.ty, &prog);
                    let etiket = self.etiket("Çıktı", n);
                    let uz = etiket.chars().count() + 1 + deger.chars().count();
                    let bosluk = " ".repeat(38usize.saturating_sub(uz).max(3));
                    println!("{} {deger}{bosluk}{}", self.renk.cikti(&etiket), self.renk.soluk(&tip));
                }
            }
            Err(e) => self.hata_yaz(n, &sources, e.loc.file, e.loc.line, ana, &e.msg),
        }
    }

    /// Tamamlama için girdideki adlar
    fn adlari_guncelle(&self, girdi: &str) {
        let mut adlar = self.adlar.borrow_mut();
        let mut w = String::new();
        for c in girdi.chars().chain(std::iter::once(' ')) {
            if c.is_alphanumeric() || c == '_' {
                w.push(c);
            } else if !w.is_empty() {
                if !w.chars().next().unwrap().is_ascii_digit() && !adlar.contains(&w) {
                    adlar.push(std::mem::take(&mut w));
                }
                w.clear();
            }
        }
    }

    /// `:` ile başlayan komut. Dönüş: false ise çık.
    fn komut(&mut self, satir: &str) -> bool {
        let satir = satir.trim();
        let (ad, arg) = match satir.find(char::is_whitespace) {
            Some(i) => (&satir[..i], satir[i..].trim()),
            None => (satir, ""),
        };
        match ad {
            ":çık" | ":cik" => return false,
            ":yardım" | ":yardim" => println!("{YARDIM}"),
            ":tip" => {
                if arg.is_empty() {
                    println!("{} ':tip' sonrasında bir ifade yazın: :tip 7 / 2", self.renk.hata("Hata:"));
                } else {
                    self.hucre(arg, true);
                }
            }
            ":geçmiş" | ":gecmis" => {
                for (n, g) in &self.girdiler {
                    println!("{} {}", self.renk.girdi(&format!("[{n}]")), g.replace('\n', "\n    "));
                }
            }
            ":kaydet" => {
                if arg.is_empty() {
                    println!("{} Dosya adı yazın: :kaydet oturum.kolay", self.renk.hata("Hata:"));
                } else {
                    let metin: Vec<&str> = self.girdiler.iter().map(|g| g.1.as_str()).collect();
                    match std::fs::write(arg, metin.join("\n") + "\n") {
                        Ok(()) => println!("Kaydedildi: {arg} ({} girdi)", self.girdiler.len()),
                        Err(e) => println!("{} '{arg}' yazılamadı: {e}", self.renk.hata("Hata:")),
                    }
                }
            }
            ":yükle" | ":yukle" => match std::fs::read_to_string(arg) {
                Ok(t) => self.hucre(t.trim_end(), false),
                Err(e) => println!("{} '{arg}' okunamadı: {e}", self.renk.hata("Hata:")),
            },
            ":temizle" => {
                self.temizle();
                println!("Oturum temizlendi.");
            }
            _ => println!("{} '{ad}' diye bir komut yok. Komutlar için :yardım", self.renk.hata("Hata:")),
        }
        true
    }
}

/// Değerin içindeki fonksiyon değerlerinin numaralarını yenileriyle değiştirir.
fn fonk_esle(v: &mut Value, esle: &[usize]) {
    match v {
        Value::Func(f) => {
            let f = Rc::make_mut(f);
            if let Some(&n) = esle.get(f.fid) {
                f.fid = n;
            }
            for c in f.caps.iter_mut().chain(f.bu.iter_mut()) {
                fonk_esle(c, esle);
            }
        }
        Value::List(l) => {
            for x in Rc::make_mut(l).items.iter_mut() {
                fonk_esle(x, esle);
            }
        }
        Value::Dict(d) => {
            for (_, x) in Rc::make_mut(d).map.values_mut() {
                fonk_esle(x, esle);
            }
        }
        Value::Obj(o) => {
            for x in Rc::make_mut(o).fields.iter_mut() {
                fonk_esle(x, esle);
            }
        }
        Value::Var(o) => {
            for x in Rc::make_mut(o).fields.iter_mut() {
                fonk_esle(x, esle);
            }
        }
        _ => {}
    }
}

/// Girdi henüz bitmedi mi? (açık parantez, yazı ya da çok satırlı yorum); ikinci değer açık parantez sayısı
fn acik(s: &str) -> (bool, usize) {
    let c: Vec<char> = s.chars().collect();
    let (mut i, mut derinlik, mut yorum) = (0usize, 0i64, 0i64);
    let mut yazi = false;
    while i < c.len() {
        let (a, b) = (c[i], c.get(i + 1).copied());
        if yorum > 0 {
            if a == '/' && b == Some('<') {
                yorum += 1;
                i += 1;
            } else if a == '>' && b == Some('/') {
                yorum -= 1;
                i += 1;
            }
        } else if yazi {
            if a == '\\' {
                i += 1;
            } else if a == '"' {
                yazi = false;
            }
        } else if a == '/' && b == Some('/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
        } else if a == '/' && b == Some('<') {
            yorum += 1;
            i += 1;
        } else if a == '"' {
            yazi = true;
        } else if "([{".contains(a) {
            derinlik += 1;
        } else if ")]}".contains(a) {
            derinlik -= 1;
        }
        i += 1;
    }
    (derinlik > 0 || yazi || yorum > 0, derinlik.max(0) as usize)
}

// ------------------------------------------------------------ satır düzenleyici (rustyline)

struct Yardimci {
    mode: Mode,
    adlar: Rc<RefCell<Vec<String>>>,
    renk: bool,
}

impl Completer for Yardimci {
    type Candidate = String;
    fn complete(&self, line: &str, pos: usize, _: &Context<'_>) -> rustyline::Result<(usize, Vec<String>)> {
        let once = &line[..pos];
        let bas = once.char_indices().rev().find(|(_, c)| !(c.is_alphanumeric() || *c == '_' || *c == ':')).map(|(i, c)| i + c.len_utf8()).unwrap_or(0);
        let kelime = &once[bas..];
        if kelime.is_empty() {
            return Ok((pos, vec![]));
        }
        let us = self.mode == Mode::Us;
        let mut aday: Vec<String> = Vec::new();
        if kelime.starts_with(':') {
            aday.extend(KOMUTLAR.iter().map(|k| k.to_string()));
        } else {
            aday.extend(KEYWORDS.iter().map(|(_, tr, u)| if us { u.to_string() } else { tr.to_string() }));
            aday.extend(BUILTIN_NAMES.iter().map(|(tr, u)| if us { u.to_string() } else { tr.to_string() }));
            aday.extend(["Tam", "Liste", "oku", "uzunluk", "ekle", "sil", "bul", "temizle", "tersi", "adım", "dönüşüm", "filtre"].iter().map(|s| s.to_string()));
            aday.extend(self.adlar.borrow().iter().cloned());
        }
        let mut v: Vec<String> = aday.into_iter().filter(|a| a.starts_with(kelime) && a != kelime).collect();
        v.sort();
        v.dedup();
        Ok((bas, v))
    }
}

impl Hinter for Yardimci {
    type Hint = String;
}

impl Highlighter for Yardimci {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(&'s self, prompt: &'p str, _default: bool) -> Cow<'b, str> {
        if self.renk { Cow::Owned(format!("\x1b[32m{prompt}\x1b[0m")) } else { Cow::Borrowed(prompt) }
    }
}

impl Validator for Yardimci {}
impl Helper for Yardimci {}

/// Satırın başında (sadece boşluk varken) '}' yazılınca bir girinti geri alınır.
struct Kapat;

impl ConditionalEventHandler for Kapat {
    fn handle(&self, _: &Event, _: RepeatCount, _: bool, ctx: &EventContext) -> Option<Cmd> {
        let once = &ctx.line()[..ctx.pos()];
        if !once.is_empty() && once.chars().all(|c| c == ' ') && ctx.pos() == ctx.line().len() {
            let yeni = " ".repeat(once.len().saturating_sub(4)) + "}";
            return Some(Cmd::Replace(Movement::BeginningOfLine, Some(yeni)));
        }
        None
    }
}

fn gecmis_dosyasi() -> Option<PathBuf> {
    let d = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?
        .join("kolay");
    let _ = std::fs::create_dir_all(&d);
    Some(d.join("repl_gecmis.txt"))
}

/// Ctrl+C: çalışan hücre döngü turlarında durur (Interp::kesildi), REPL açık kalır.
fn ctrl_c_bayrak() {
    use std::ffi::c_int;
    unsafe extern "C" {
        fn signal(sig: c_int, handler: extern "C" fn(c_int)) -> usize;
    }
    extern "C" fn bayrak(_: c_int) {
        KESME.store(true, Ordering::Relaxed);
    }
    unsafe {
        signal(2, bayrak);
    }
}

pub fn calistir(mode: Mode) -> i32 {
    let ekran = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let adlar = Rc::new(RefCell::new(Vec::new()));
    let mut o = Oturum::new(mode, ekran, adlar.clone());
    ctrl_c_bayrak();
    if !ekran {
        return borudan(&mut o);
    }
    let mut rl: Editor<Yardimci, FileHistory> = match Editor::new() {
        Ok(r) => r,
        Err(_) => return borudan(&mut o),
    };
    rl.set_helper(Some(Yardimci { mode, adlar, renk: true }));
    rl.bind_sequence(KeyEvent::from('}'), EventHandler::Conditional(Box::new(Kapat)));
    let gecmis = gecmis_dosyasi();
    if let Some(g) = &gecmis {
        let _ = rl.load_history(g);
    }
    println!("Kolay {} — yardım için :yardım, çıkmak için :çık\n", env!("CARGO_PKG_VERSION"));
    'dis: loop {
        let istem = format!("Girdi [{}]: ", o.sayac + 1);
        let devam = format!("{:>w$}", "...: ", w = istem.chars().count());
        let mut buf = String::new();
        loop {
            let (ilk, derinlik) = (buf.is_empty(), acik(&buf).1);
            let girinti = "    ".repeat(derinlik);
            let p = if ilk { istem.as_str() } else { devam.as_str() };
            match rl.readline_with_initial(p, (&girinti, "")) {
                Ok(satir) => {
                    if ilk && satir.trim_start().starts_with(':') {
                        let _ = rl.add_history_entry(satir.trim());
                        if !o.komut(&satir) {
                            break 'dis;
                        }
                        println!();
                        continue 'dis;
                    }
                    buf.push_str(&satir);
                    if acik(&buf).0 {
                        buf.push('\n');
                        continue;
                    }
                    break;
                }
                Err(ReadlineError::Interrupted) => {
                    println!();
                    continue 'dis;
                }
                Err(_) => break 'dis,
            }
        }
        if buf.trim().is_empty() {
            continue;
        }
        let _ = rl.add_history_entry(buf.trim_end());
        o.hucre(buf.trim_end(), false);
        let _ = std::io::stdout().flush();
        println!();
    }
    if let Some(g) = &gecmis {
        let _ = rl.history_mut().save(g);
    }
    0
}

/// Ekran değilse (boru, dosya): istem ve renk olmadan, satır satır okur.
fn borudan(o: &mut Oturum) -> i32 {
    let mut buf = String::new();
    for satir in std::io::stdin().lock().lines() {
        let Ok(satir) = satir else { break };
        if buf.is_empty() && satir.trim_start().starts_with(':') {
            if !o.komut(&satir) {
                return 0;
            }
            continue;
        }
        buf.push_str(&satir);
        if acik(&buf).0 {
            buf.push('\n');
            continue;
        }
        if !buf.trim().is_empty() {
            o.hucre(buf.trim_end(), false);
        }
        buf.clear();
    }
    if !buf.trim().is_empty() {
        o.hucre(buf.trim_end(), false);
    }
    0
}

