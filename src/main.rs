// Kolay: Türkçe programlama dili. Komut satırı arayüzü.

mod ast;
mod cexpr;
mod check;
mod codegen;
mod cstmt;
mod diag;
mod dis;
mod firlat;
mod generic;
mod interp;
mod lexer;
mod loader;
mod parser;
mod repl;
mod tir;
mod types;
mod values;
mod varsayilan;
mod vm;

use diag::Sources;
use lexer::Mode;
use std::path::{Path, PathBuf};

const YARDIM: &str = "\
Kolay - Türkçe programlama dili

Kullanım:
  kolay                               Etkileşimli kip (REPL)
  kolay <dosya.kolay>                 Programı çalıştırır
  kolay çalıştır <dosya.kolay>        Programı çalıştırır
  kolay derle <dosya.kolay> [-o ad]   Programı makine koduna derler (LLVM)
                                      --ir ile üretilen LLVM IR de kaydedilir
  kolay denetle <dosya.kolay>         Sadece hataları kontrol eder
  kolay çevir <dosya.kolay> --klavye us [-o yeni.kolay]
                                      Anahtar kelimeleri diğer klavye moduna çevirir
  kolay yardım                        Bu yardımı gösterir

Seçenekler:
  --klavye tr|us   Dosyada klavye(...) satırı yoksa kullanılacak mod (varsayılan: tr)
  -o <ad>          Çıktı dosyasının adı
";

fn main() {
    // Kolay'ın kendi hatası (panik) olursa Rust mesajı yerine anlaşılır bir Türkçe mesaj.
    std::panic::set_hook(Box::new(|info| {
        let yer = info.location().map(|l| format!(" ({}:{})", l.file(), l.line())).unwrap_or_default();
        let ne = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        eprintln!("İÇ HATA: Kolay'ın kendisinde bir hata oluştu; bu sizin programınızın hatası değil.");
        eprintln!("Lütfen bu mesajla birlikte bildirin: {ne}{yer}");
    }));
    let child = std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(real_main)
        .expect("iş parçacığı başlatılamadı");
    let code = child.join().unwrap_or(101);
    std::process::exit(code);
}

struct Opts {
    cmd: String,
    file: Option<String>,
    mode: Mode,
    out: Option<String>,
    mode_given: bool,
    ir: bool,
}

fn parse_args() -> Result<Opts, String> {
    let mut args = std::env::args().skip(1).peekable();
    let mut o = Opts { cmd: String::new(), file: None, mode: Mode::Tr, out: None, mode_given: false, ir: false };
    while let Some(a) = args.next() {
        match a.as_str() {
            "--klavye" | "--keyboard" => {
                let v = args.next().ok_or("--klavye sonrasında 'tr' ya da 'us' yazılmalı.")?;
                o.mode = match v.as_str() {
                    "tr" => Mode::Tr,
                    "us" => Mode::Us,
                    _ => return Err(format!("Geçersiz klavye modu: '{v}'. 'tr' ya da 'us' olmalı.")),
                };
                o.mode_given = true;
            }
            "--ir" => o.ir = true,
            "-o" => o.out = Some(args.next().ok_or("-o sonrasında bir dosya adı yazılmalı.")?),
            "-h" | "--help" | "--yardım" | "yardım" | "yardim" => o.cmd = "yardım".into(),
            "çalıştır" | "calistir" | "derle" | "denetle" | "çevir" | "cevir" if o.cmd.is_empty() && o.file.is_none() => {
                o.cmd = match a.as_str() {
                    "calistir" => "çalıştır".into(),
                    "cevir" => "çevir".into(),
                    x => x.into(),
                };
            }
            _ if a.starts_with('-') => return Err(format!("Tanınmayan seçenek: '{a}'. Yardım için: kolay yardım")),
            _ => {
                if o.file.is_some() {
                    return Err(format!("Fazladan değer: '{a}'. Yardım için: kolay yardım"));
                }
                o.file = Some(a)
            }
        }
    }
    if o.cmd.is_empty() {
        o.cmd = "çalıştır".into();
    }
    Ok(o)
}

fn front(path: &Path, mode: Mode, sources: &mut Sources) -> Option<tir::Program> {
    let mods = match loader::load(path, mode, sources) {
        Ok(m) => m,
        Err(d) => {
            eprint!("{}", sources.render("HATA", &d));
            return None;
        }
    };
    match check::check(mods, sources) {
        Ok((prog, warnings)) => {
            for w in &warnings {
                eprint!("{}", sources.render("Uyarı", w));
            }
            Some(prog)
        }
        Err(d) => {
            eprint!("{}", sources.render("HATA", &d));
            None
        }
    }
}

/// Ctrl+C: program Türkçe bir mesajla durur (derlenmiş programdaki runtime/cekirdek.c ile aynı).
fn ctrl_c_yakala() {
    use std::ffi::c_int;
    unsafe extern "C" {
        fn signal(sig: c_int, handler: extern "C" fn(c_int)) -> usize;
        fn write(fd: c_int, buf: *const std::ffi::c_void, n: usize) -> isize;
        fn _exit(code: c_int) -> !;
    }
    extern "C" fn durdur(_: c_int) {
        let m = DURDURULDU.as_bytes();
        unsafe {
            write(2, m.as_ptr().cast(), m.len());
            _exit(130);
        }
    }
    unsafe {
        signal(2, durdur);
    }
}

const DURDURULDU: &str = "\nProgram durduruldu (Ctrl+C).\n";

fn real_main() -> i32 {
    let o = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("HATA: {e}");
            return 2;
        }
    };
    if o.cmd == "yardım" {
        print!("{YARDIM}");
        return 0;
    }
    let Some(file) = o.file.clone() else {
        if o.cmd == "çalıştır" {
            // dosya adı yoksa etkileşimli kip
            return repl::calistir(o.mode);
        }
        print!("{YARDIM}");
        return 2;
    };
    let path = PathBuf::from(&file);
    if !path.exists() {
        eprintln!("HATA: '{file}' dosyası bulunamadı.");
        return 1;
    }
    let mut sources = Sources::default();
    match o.cmd.as_str() {
        "çalıştır" => {
            let Some(prog) = front(&path, o.mode, &mut sources) else { return 1 };
            ctrl_c_yakala();
            let dis = match dis::resolve(&prog) {
                Ok(d) => d,
                Err(d) => {
                    eprint!("{}", sources.render("HATA", &d));
                    return 1;
                }
            };
            let mut it = interp::Interp::new(&prog);
            it.dis = dis;
            if let Err(e) = it.run() {
                let d = diag::Diag::new(diag::Span { file: e.loc.file, line: e.loc.line, col: 0, start: 0, end: 0 }, e.msg);
                eprint!("{}", render_runtime(&sources, &d));
                return 1;
            }
            0
        }
        "denetle" => {
            let Some(prog) = front(&path, o.mode, &mut sources) else { return 1 };
            if let Err(d) = dis::resolve(&prog) {
                eprint!("{}", sources.render("HATA", &d));
                return 1;
            }
            println!("Hata bulunamadı.");
            0
        }
        "derle" => {
            let Some(prog) = front(&path, o.mode, &mut sources) else { return 1 };
            let out = o.out.clone().map(PathBuf::from).unwrap_or_else(|| path.with_extension(""));
            match codegen::compile(&prog, &sources, &out, o.ir) {
                Ok(()) => {
                    println!("Derlendi: {}", out.display());
                    0
                }
                Err(d) => {
                    eprint!("{}", sources.render("HATA", &d));
                    1
                }
            }
        }
        "çevir" => {
            if !o.mode_given {
                eprintln!("HATA: Hangi moda çevrileceğini yazın: kolay çevir {file} --klavye us");
                return 2;
            }
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("HATA: '{file}' okunamadı: {e}");
                    return 1;
                }
            };
            sources.add(path.clone(), text.clone());
            let from = match lexer::find_klavye(&text, 0) {
                Ok(Some((m, _))) => m,
                Ok(None) => if o.mode == Mode::Us { Mode::Tr } else { Mode::Us },
                Err(d) => {
                    eprint!("{}", sources.render("HATA", &d));
                    return 1;
                }
            };
            match lexer::convert(&text, from, o.mode) {
                Ok(s) => {
                    match &o.out {
                        Some(p) => {
                            if let Err(e) = std::fs::write(p, s) {
                                eprintln!("HATA: '{p}' yazılamadı: {e}");
                                return 1;
                            }
                            println!("Çevrildi: {p}");
                        }
                        None => print!("{s}"),
                    }
                    0
                }
                Err(d) => {
                    eprint!("{}", sources.render("HATA", &d));
                    1
                }
            }
        }
        _ => unreachable!(),
    }
}

fn render_runtime(sources: &Sources, d: &diag::Diag) -> String {
    let name = sources.name(d.span.file);
    let mut out = format!("ÇALIŞMA HATASI ({name}, satır {}): {}\n", d.span.line, d.msg);
    if let Some(f) = sources.files.get(d.span.file as usize) {
        if let Some(line) = f.text.lines().nth(d.span.line.saturating_sub(1) as usize) {
            out.push_str(&format!("  {} | {}\n", d.span.line, line.trim_end()));
        }
    }
    out
}
