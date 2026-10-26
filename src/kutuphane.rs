// Kütüphaneler: `dahil et("matematik")` ile adıyla dahil edilen .kolay dosyaları ve `kolay kütüphane` komutları.
//
// Yolu yazılmayan (içinde '/' ve '.kolay' olmayan) bir dahil etme kütüphane adıdır ve sırasıyla şu
// klasörlerde aranır:
//   1. dahil eden dosyanın klasörü (yanındaki ad.kolay)
//   2. KOLAY_KUTUPHANE ortam değişkenindeki klasörler (':' ile ayrılmış)
//   3. kişisel kütüphane klasörü: ~/.local/share/kolay/kutuphaneler (kolay kütüphane kur buraya koyar)
//   4. Kolay'ın kendi kutuphaneler/ klasörü (hazır kütüphaneler: matematik, rastgele, istatistik...)

use std::path::{Path, PathBuf};

/// `dahil et("...")` içindeki yazı bir yol değil de kütüphane adı mı?
pub fn ad_mi(p: &str) -> bool {
    !p.is_empty() && !p.contains('/') && !p.contains('\\') && !p.ends_with(".kolay")
}

/// Kişisel kütüphane klasörü (yoksa da yolu döner)
pub fn kisisel_klasor() -> Option<PathBuf> {
    let veri = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| if cfg!(windows) { std::env::var_os("APPDATA").map(PathBuf::from) } else { None })
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(veri.join("kolay").join("kutuphaneler"))
}

/// Kolay'ın kendi kutuphaneler/ klasörü: çalışan programın (bağlantı çözülmüş) yolunun üstlerinde aranır.
/// Kaynaktan derlenmişse (target/release/kolay) deponun kökündeki kutuphaneler/ bulunur.
pub fn hazir_klasor() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    exe.ancestors().skip(1).map(|a| a.join("kutuphaneler")).find(|k| k.is_dir())
}

/// Aranacak klasörler, sırasıyla (dahil eden dosyanın klasörü hariç)
pub fn klasorler() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(e) = std::env::var_os("KOLAY_KUTUPHANE") {
        v.extend(std::env::split_paths(&e).filter(|p| !p.as_os_str().is_empty()));
    }
    v.extend(kisisel_klasor());
    v.extend(hazir_klasor());
    v
}

/// Kütüphaneyi bulur; bulamazsa aranan yerleri anlatan Türkçe mesaj döner.
pub fn bul(ad: &str, dahil_eden_klasor: &Path) -> Result<PathBuf, String> {
    let dosya = format!("{ad}.kolay");
    let mut aranan = vec![dahil_eden_klasor.to_path_buf()];
    aranan.extend(klasorler());
    for k in &aranan {
        let p = k.join(&dosya);
        if p.is_file() {
            return Ok(p);
        }
    }
    let yerler: Vec<String> = aranan.iter().map(|k| k.display().to_string()).collect();
    Err(format!(
        "'{ad}' kütüphanesi bulunamadı. Arandığı yerler: {}. Bir dosyayı dahil etmek için yolunu yazın: dahil et(\"./{ad}.kolay\"). Hazır kütüphaneler: kolay kütüphane liste",
        yerler.join(", ")
    ))
}

// ------------------------------------------------------------ kolay kütüphane ...

const KOMUT_YARDIM: &str = "\
Kullanım:
  kolay kütüphane liste            Kullanılabilen kütüphaneleri ve yerlerini gösterir
  kolay kütüphane yeni <ad>        <ad>.kolay adında, testleriyle hazır bir kütüphane şablonu oluşturur
  kolay kütüphane kur <dosya.kolay>
                                   Kütüphaneyi kişisel klasöre kopyalar; sonra her yerden
                                   dahil et(\"ad\") ile kullanılabilir
";

pub fn komut(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("liste") => liste(),
        // editörler için (VS Code eklentisi): her satırda "ad<TAB>yol"; klasör verilirse önce orası
        Some("yollar") => {
            yollar(args.get(1).map(Path::new));
            0
        }
        Some("yeni") => match args.get(1) {
            Some(ad) => yeni(ad),
            None => {
                eprintln!("HATA: Kütüphanenin adını yazın: kolay kütüphane yeni geometri");
                2
            }
        },
        Some("kur") => match args.get(1) {
            Some(f) => kur(Path::new(f)),
            None => {
                eprintln!("HATA: Kurulacak dosyayı yazın: kolay kütüphane kur geometri.kolay");
                2
            }
        },
        _ => {
            print!("{KOMUT_YARDIM}");
            2
        }
    }
}

fn yollar(dahil_eden: Option<&Path>) {
    let mut gorulen = std::collections::HashSet::new();
    let klasorler: Vec<PathBuf> = dahil_eden.map(Path::to_path_buf).into_iter().chain(klasorler()).collect();
    for k in klasorler {
        let Ok(oku) = std::fs::read_dir(&k) else { continue };
        let mut adlar: Vec<String> =
            oku.filter_map(|e| e.ok()).filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".kolay")).map(str::to_string)).collect();
        adlar.sort();
        for a in adlar {
            if gorulen.insert(a.clone()) {
                println!("{a}\t{}", k.join(format!("{a}.kolay")).display());
            }
        }
    }
}

fn liste() -> i32 {
    let mut hic = true;
    for k in klasorler() {
        let Ok(oku) = std::fs::read_dir(&k) else { continue };
        let mut adlar: Vec<String> = oku
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".kolay")).map(str::to_string))
            .collect();
        if adlar.is_empty() {
            continue;
        }
        adlar.sort();
        hic = false;
        println!("{}:", k.display());
        for a in adlar {
            let ozet = std::fs::read_to_string(k.join(format!("{a}.kolay")))
                .ok()
                .and_then(|t| t.lines().next().map(|l| l.trim_start_matches('/').trim().to_string()))
                .unwrap_or_default();
            println!("  {a:<14} {ozet}");
        }
    }
    if hic {
        println!("Hiç kütüphane bulunamadı.");
    }
    println!("\nKullanmak için: m adıyla dahil et(\"matematik\")");
    0
}

fn yeni(ad: &str) -> i32 {
    let ad = ad.strip_suffix(".kolay").unwrap_or(ad);
    if !ad.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_') || !ad.chars().all(|c| c.is_alphanumeric() || c == '_') {
        eprintln!("HATA: '{ad}' kütüphane adı olamaz; harfle başlamalı, harf, rakam ve _ içerebilir: geometri, yazı_araçları");
        return 2;
    }
    let dosya = PathBuf::from(format!("{ad}.kolay"));
    if dosya.exists() {
        eprintln!("HATA: '{}' zaten var; üzerine yazılmadı.", dosya.display());
        return 1;
    }
    let sablon = format!(
        "// {ad}: (kütüphanenin ne işe yaradığını tek satırda yazın; `kolay kütüphane liste` bunu gösterir)
//
// Kullanmak için:  {k} adıyla dahil et(\"{ad}\")
//                  yazdır({k}.örnek(2, 3))
//
// - Dışarıya açık olan her şey (fonksiyon, tür, değişken) dosyanın üst düzeyinde tanımlanır.
// - Üst düzeydeki komutlar dahil edilince bir kez çalışır: burada yazdır kullanmayın, sadece
//   doğrula ile kütüphanenin kendi testlerini yazın (hata yoksa bir şey yazmazlar).
// - Hazır olduğunda: kolay kütüphane kur {ad}.kolay  (sonra her yerden dahil et(\"{ad}\"))

// Örnek fonksiyon: kendi fonksiyonlarınızla değiştirin.
fonksiyon örnek(a: Tam(64), b: Tam(64) = 1) -> Tam(64) {{
    doğrula(b != 0, \"b sıfır olamaz.\")
    döndür a * b
}}

// Her tiple çalışan (generic) bir fonksiyon örneği
fonksiyon ilk<T>(l: Liste<T>) -> T {{
    doğrula(l.uzunluk() > 0, \"Boş listenin ilk elemanı yok.\")
    döndür l[0]
}}

// ------------------------------------------------------------ testler
doğrula(örnek(2, 3) == 6)
doğrula(örnek(5) == 5)
doğrula(ilk([\"a\", \"b\"]) == \"a\")
",
        k = ad.chars().next().map(|c| c.to_lowercase().to_string()).unwrap_or_default()
    );
    if let Err(e) = std::fs::write(&dosya, sablon) {
        eprintln!("HATA: '{}' yazılamadı: {e}", dosya.display());
        return 1;
    }
    println!("Oluşturuldu: {}", dosya.display());
    println!("Denemek için:   kolay denetle {}", dosya.display());
    println!("Kurmak için:    kolay kütüphane kur {}", dosya.display());
    0
}

fn kur(dosya: &Path) -> i32 {
    let Some(ad) = dosya.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix(".kolay")) else {
        eprintln!("HATA: Kütüphane bir .kolay dosyası olmalı: kolay kütüphane kur geometri.kolay");
        return 2;
    };
    if !dosya.is_file() {
        eprintln!("HATA: '{}' dosyası bulunamadı.", dosya.display());
        return 1;
    }
    let Some(hedef_klasor) = kisisel_klasor() else {
        eprintln!("HATA: Kişisel klasör bulunamadı (HOME ya da APPDATA tanımlı değil).");
        return 1;
    };
    if let Err(e) = std::fs::create_dir_all(&hedef_klasor) {
        eprintln!("HATA: '{}' oluşturulamadı: {e}", hedef_klasor.display());
        return 1;
    }
    let hedef = hedef_klasor.join(format!("{ad}.kolay"));
    if let Err(e) = std::fs::copy(dosya, &hedef) {
        eprintln!("HATA: '{}' kopyalanamadı: {e}", hedef.display());
        return 1;
    }
    println!("Kuruldu: {}", hedef.display());
    println!("Kullanmak için: {} adıyla dahil et(\"{ad}\")", ad.chars().next().map(|c| c.to_lowercase().to_string()).unwrap_or_default());
    0
}
