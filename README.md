# Kolay

Türkçe, tip güvenli ve hızlı bir programlama dili. Programlamanın temel mantığını İngilizce bilmeden
öğrenmek ve sonra Python'a kolayca geçmek için tasarlandı. Hem yorumlayıcısı (hemen çalıştırır) hem de
LLVM ile makine koduna derleyicisi (C'ye yakın hız) var.

- **Öğrenmek için:** [Kolay Kursu](belgeler/Kolay.md): sıfırdan başlayan, alıştırmalı kurs
- **Kütüphane yazmak ya da dile katkı için:** [Geliştirici rehberi](belgeler/GELISTIRICI.md), [Katkı](KATKI.md)

```
fonksiyon fib(n: Tam(64)) -> Tam(64) {
    eğer n < 2 { döndür n }
    döndür fib(n - 1) + fib(n - 2)
}

ata sayılar = [1, 2, 3, 4, 5]
sayılar içindeki her s için yap {
    yazdır(f"fib({s}) = {fib(s)}")
}
```

## Kurulum

Linux ve Windows'ta çalışır. Gerekenler: Rust (1.85 ya da yenisi), derleyici için clang ve llvm.

### Linux

```
# Ubuntu / Debian
sudo apt install clang llvm libffi-dev
# Arch
sudo pacman -S clang llvm libffi
# Rust: https://rustup.rs

git clone https://github.com/OmerFirig/Kolay.git && cd Kolay
./kur.sh
```

`kur.sh` projeyi derler ve `kolay` komutunu `~/.local/bin` içine bağlar. Kaldırmak için: `rm ~/.local/bin/kolay`

### Windows

1. Rust: https://rustup.rs adresinden `rustup-init.exe` (kurulum sırasında istenen Visual Studio Build Tools'u da kurun).
2. Git ve clang (LLVM), PowerShell'de:
   ```
   winget install Git.Git LLVM.LLVM
   ```
3. Yeni bir PowerShell penceresinde:
   ```
   git clone https://github.com/OmerFirig/Kolay.git
   cd Kolay
   powershell -ExecutionPolicy Bypass -File kur.ps1
   ```

`kur.ps1` projeyi derler ve `kolay` komutunu kullanıcının PATH'ine ekler; yeni açılan terminallerde `kolay`
yazmak yeter. `kolay derle kod.kolay` Windows'ta `kod.exe` üretir. libffi ayrıca kurulmaz (Rust ile birlikte derlenir).

## Kullanım

| Komut | Ne yapar |
|---|---|
| `kolay` | Etkileşimli kip (REPL): komutları tek tek dener, ifadelerin değerini ve tipini gösterir |
| `kolay kod.kolay` | Programı çalıştırır (yorumlayıcı, hemen başlar) |
| `kolay derle kod.kolay` | Makine koduna derler, `./kod` dosyasını üretir (LLVM, C'ye yakın hız) |
| `kolay derle kod.kolay -o ad --ir` | Çıktıya ad verir, üretilen LLVM IR'yi de `ad.ll` olarak kaydeder |
| `kolay denetle kod.kolay` | Sadece hataları kontrol eder |
| `kolay çevir kod.kolay --klavye us` | Anahtar kelimeleri Türkçe karakter olmayan yazıma çevirir |
| `kolay kütüphane liste` | Kullanılabilen kütüphaneler (`yeni <ad>`: şablon oluşturur, `kur <dosya>`: kişisel klasöre kurar) |
| `kolay yardım` | Yardım |

`--klavye tr|us`: Dosyada `klavye("...")` satırı yoksa kullanılacak mod (varsayılan `tr`).

Türkçe klavyesi olmayanlar dosyanın ilk satırına `klavye("us")` yazar: dilin kelimeleri Türkçe harfsiz yazılır
(`eger`, `dondur`, `yazdir`). Dahil edilen `tr` dosyalarındaki Türkçe harfli adlar da harfsiz yazılabilir:
`m.karekok(16.0)` → `karekök`.

## Etkileşimli kip (REPL)

```
$ kolay
Kolay 0.7.0 — yardım için :yardım, çıkmak için :çık

Girdi [1]: ata x = 5

Girdi [2]: x * 2
Çıktı [2]: 10                         Tam(64)
```

Komutlar: `:yardım`, `:tip ifade`, `:geçmiş`, `:kaydet dosya`, `:yükle dosya`, `:temizle`, `:çık`.
Çalışan bir girdi Ctrl+C ile durdurulur. Ok tuşlarıyla geçmiş (`~/.cache/kolay/repl_gecmis.txt`, Windows'ta `%LOCALAPPDATA%\kolay`), Tab ile tamamlama.

## VS Code eklentisi

`editor/vscode/`: `.kolay` dosyaları için renklendirme ve hazır kod parçaları (`eğer`, `kere`, `her`, `tür`, `eşleştir`... yazıp Tab'a basın). Kurmak için:

```
code --install-extension editor/vscode/kolay-0.7.0.vsix
```

## Örnekler

`ornekler/` klasöründe her özelliği gösteren açıklamalı programlar var:

| Dosya | Konu |
|---|---|
| `01_merhaba.kolay` | yazdır, oku, f-yazı, dene/yakala |
| `02_sayilar.kolay` | Tam(N), işaret, esnek Tam, Ondalık, olarak, taşma |
| `03_asal_sayilar.kolay` | fonksiyonlar ve tüm döngü türleri |
| `04_hiz_testi.kolay` | fib(35) hız testi |
| `05_okul.kolay` | tür, kalıtım, arayüz, Liste, Sözlük, sıralama |
| `06_sekiller.kolay` | seçenek, eşleştir, veya tipleri, Hiç |
| `07_us_klavye.kolay` | `klavye("us")` modu |
| `08_kutuphane/` | Kolay ile kütüphane yazmak ve kullanmak (`ana.kolay`) |

Hepsi hem yorumlayıcıyla hem derleyiciyle çalışır.

## Dil özeti

Dili sıfırdan öğreten kurs (alıştırmalar ve çözümleriyle): [`belgeler/Kolay.md`](belgeler/Kolay.md)

```
// tek satırlık yorum
/< çok satırlı yorum, /< iç içe >/ olabilir >/

ata x = 42                       // Tam(64)
ata y: Tam(8, +) = 200           // 8 bit, sadece pozitif
ata z: Tam = 1                   // esnek: hiç taşmaz
ata o: Ondalık(32) = 3.14
ata renk = 0xFF8800              // onaltılık sayı
ata m = renk & 0xFF | 1 << 8     // bit işlemleri: & | ^ ~ << >>
ata ü = 2 ** 10                  // üs

sayılar.dönüşüm(her x için x * 2)            // lambda: isimsiz fonksiyon
sayılar.filtre(her x için x > 10).toplam()
öğrenciler.sırala(göre: her ö için ö.yaş)
fonksiyon uygula(f: Fonksiyon(Tam(64)) -> Tam(64), x: Tam(64)) -> Tam(64) { döndür f(x) }
fonksiyon en_büyük<T>(l: Liste<T>) -> T { ... }    // generic: T her çağrıda çıkarılır
fonksiyon selam(ad: Yazı = "dünya") { }            // varsayılan değer: selam() ya da selam("Ali")
ata iki_kat = her<T> x: T için x * 2               // generic lambda: iki_kat(5), iki_kat(2.5)
uygula(iki_kat, 5)                                  // generic fonksiyon değer olarak
fonksiyon ikisine(f: Fonksiyon<T>(T) -> T) { }     // her tiple çalışan fonksiyon parametresi
tür Kutu<T>(değer: T) { fonksiyon al() -> T { döndür değer } }
ata k = Kutu(değer: 5)                              // Kutu<Tam(64)>
ata t = 1.0 / 0.0                // TanımsızSayı (sıfıra bölme tanımsız)
yazdır(f"{pi:.2} {ad:<10} {n:06} {büyük:,}")   // f-yazı biçimleri: 3.14  Ali  000042  1.234.567
yazdır(1, 2, 3, ayraç: ", ", son: "")         // 1, 2, 3 (satır sonu olmadan)
ata avogadro = 6.022e23                       // bilimsel yazım (Ondalık)
"elma".değiştir("a", "e")  s.başlar("x")  s.bul("ma")  s.say("a")  "-".tekrar(5)  l.birleşik(", ")
doğrula(topla(2, 3) == 5)        // yanlışsa program durur (kendi kodunu test etmek için)
ata sabit pi = 3.14159           // değiştirilemez
ata sabit mesaj: Yazı            // değeri sonra, bir kez atanır
x: Yazı = "artık yazı"           // tip değiştirme (gölgeleme)
ata k = 300 olarak Tam(16)       // açık dönüştürme

eğer x > 5 ve değil bitti { } değilse eğer x == 0 { } değilse { }

5 kere yap { }
liste içindeki her e için yap { }
0..10 içindeki her i için yap { }
(0..100).adım(5) içindeki her i için yap { }
10..0 içindeki her i için yap { }          // geri sayar: 10 .. 1
(10..0).adım(2) içindeki her i için yap { }          // 10 8 6 4 2
ata r = 1..=5                    // aralık bir listedir: [1, 2, 3, 4, 5]
koşul sürece yap { }
(ata i = 0; i < 10; i += 1) sürece yap { }
// dur, devam

tür Öğrenci[Kişi, Sporcu](sabit numara: Tam(32, +)) {
    ata puan = 0                                 // gövdede alan: oluştururken verilmez
    ortak ata sayı = 0                           // türe ait: Öğrenci.sayı
    ortak fonksiyon özet() -> Yazı { döndür f"{sayı} öğrenci" }
    yeniden tanıt() { Kişi.tanıt() }
}
arayüz Konuşabilir { fonksiyon konuş() -> Yazı }
seçenek Renk { Kırmızı, Yeşil, Mavi }

ata ad = eşleştir renk {
    Kırmızı -> "kırmızı"
    değilse -> Hiç               // ad: Yazı veya Hiç
}

ata tablo: Liste<Tam veya Yazı>[4][]
ata notlar: Sözlük<Yazı, Tam> = {"Ali": 90}
liste[4..1]  liste[-1..=0]  liste.adım(2)   // ters dilim, her 2. eleman
liste.sırala()  liste.sıralı()              // emir kipi değiştirir, sıfat yeni liste verir
"merhaba"[0..3]                             // yazı dilimi: "mer"

dene {                                      // hata yakalama
    ata yaş = oku("Yaş: ").tama()           // tama(): Tam(64) veya Hata.Dönüşüm
} yakala h: Hata.Dönüşüm {
    yazdır(h.mesaj)
} yakala diğer { yazdır(diğer) }            // tipsiz: geri kalan her şey
fırlat Hata.SıfıraBölme(mesaj: "...")       // her değer fırlatılabilir: fırlat 42
fonksiyon böl(a: Tam, b: Tam) -> Tam veya Hata { ... }   // dene içinde dönen Hata fırlatılır
tür GeçersizYaş[Hata](yaş: Tam)             // kendi hata türü

dahil et("./yardımcı.kolay")
m adıyla dahil et("./matematik.kolay")

#Güvensiz
mat adıyla dış kütüphane("m") {                   // C kütüphanesi
    fonksiyon karekök(x: Ondalık) -> Ondalık = "sqrt"
}
```

## Kütüphaneler

Hazır kütüphaneler (`kutuphaneler/`) adıyla dahil edilir; `#Güvensiz` gerekmez:

```
m adıyla dahil et("matematik")     // karekök, üs, yuvarla, taban, tavan, mutlak, sinüs, ln, ebob, asal_mı, PI, E
r adıyla dahil et("rastgele")      // tam_sayı(1, 6), zar(), ondalık(), şans(0.3), seç(liste), karıştır(liste)
ist adıyla dahil et("istatistik")  // ortalama, ortanca, en_sık, varyans, standart_sapma, Rapor
yazdır(m.karekök(2.0), r.zar(), ist.ortalama([1.0, 2.0]))
```

Kendi kütüphanenizi yazmak için `kolay kütüphane yeni geometri` (testleriyle hazır şablon), bitince
`kolay kütüphane kur geometri.kolay`; sonra her yerden `dahil et("geometri")`. Ayrıntılar:
[Geliştirici rehberi](belgeler/GELISTIRICI.md). Yeni bir hazır kütüphaneyi `kutuphaneler/` klasörüne
eklemek için bkz. [Katkı](KATKI.md).

## Geliştirmek

Kolay ile kütüphane yazmak, bir C kütüphanesini sarmak ya da dile hazır metot eklemek için: [`belgeler/GELISTIRICI.md`](belgeler/GELISTIRICI.md)

## Mimari

```
kaynak (.kolay)
  → lexer.rs      token'lar (klavye modu, yorumlar, f-yazılar, onaltılık sayılar)
  → parser.rs     sözdizimi ağacı (ast.rs)
  → loader.rs     dahil et ile eklenen dosyalar
    kutuphane.rs  kütüphaneleri adıyla bulma, kolay kütüphane komutları
    us_adlar.rs   us modunda tr dosyalarının Türkçe harfli adlarını bulma
  → check.rs      tip denetleyici: tanımlar, kalıtım, arayüzler
    cexpr.rs      ifadeler, tip çıkarımı, Türkçe hata mesajları
    cstmt.rs      komutlar, eşleştir, dene/yakala, akış analizi (belirsiz/sabit kontrolü)
    firlat.rs     fırlatma analizi: yakala dallarına gelebilecek değerler, uyarılar
    generic.rs    generic fonksiyon ve türler: tipleri çıkarır, her tip ataması için kopya oluşturur
    varsayilan.rs varsayılan değerler ve türün gövdesindeki alanlar
    genel_deger.rs generic lambda ve generic fonksiyonların değer olarak verilmesi
  → tir.rs        tipleri çözülmüş ara gösterim (TIR)
      → repl.rs     etkileşimli kip (oturum, hücreler, rustyline)
      → vm.rs       bytecode yorumlayıcı (kolay kod.kolay)
        interp.rs   ağaç yorumlayıcı: değerler, nadir yapılar (vm.rs buna devreder)
        values.rs   çalışma anı değerleri, yazıya çevirme, Türkçe sıralama
        dis.rs      dış (C) kütüphaneler: bulma, libffi ile çağırma
      → codegen/    LLVM IR üretici + clang (kolay derle)
          mod.rs      program ve fonksiyonlar      duzen.rs   bellek düzeni, tip tanımları
          ifade.rs    ifadeler, dönüştürmeler      komut.rs   komutlar, döngüler, eşleştir
          sahip.rs    referans sayımı              yerlesik.rs hazır metotlar
        runtime/    derlenmiş programların C kütüphanesi (yazı, liste, sözlük, büyük sayı...)
  diag.rs: Türkçe hata ve uyarı mesajlarını kaynak satırıyla yazar (her aşama kullanır)
```

## Hız (bu bilgisayarda, saniye)

| Program | `kolay derle` | C (clang -O2) | Python 3 | `kolay` (yorumlayıcı) |
|---|---|---|---|---|
| fib(35) | 0,04 | 0,02 | 1,07 | 1,9 |
| Eratosten eleği (20 milyon) | 0,40 | 0,28 | 3,0 | 4,6 |
| 1000 nesne × 3000 adım | 0,02 | 0,004 | 0,54 | 1,8 |
| Sözlükte 2 milyon kelime sayma | 0,20 | – | 0,62 | 0,87 |
| Esnek Tam (büyük sayılar) | 0,02 | – | 0,60 | 0,45 |

Yorumlayıcı programı hemen başlatır (derleme beklemez); hız gerekiyorsa `kolay derle` kullanılmalı.

## Derleyici

Dilin tamamı derlenir: tüm sayı tipleri (esnek Tam dahil), yazılar, listeler, sözlükler, türler
(kalıtım, arayüz, `yeniden`, generic), seçenekler, `veya` tipleri, eşleştir, `olarak`, `dahil et`, `dene`/`yakala`/`fırlat`.

Derlenmiş programlarda:
- Değerler yazmaçlarda durur; yazı, liste, sözlük ve nesneler referans sayımıyla tutulur ve
  kullanılmayınca bellekleri hemen geri verilir (çöp toplayıcı yok). Değer semantiği "yazınca
  kopyala" ile sağlanır: paylaşılan bir liste ancak değiştirilirken kopyalanır.
- `dene` bloğu girişte setjmp yapar; hata (çalışma hatası ya da `fırlat`) longjmp ile yakala dalına döner. Dene içinden çağrılan fonksiyonların temizlik kaydı tutan bir kopyası kullanılır; atlanan fonksiyonların değişkenleri böylece bırakılır (dene dışındaki koda maliyeti yok).
- Metotlar mümkünse doğrudan, birden fazla aday varsa türe göre `switch` ile çağrılır.
- Hata mesajları yorumlayıcıdakiyle aynıdır (dosya, satır ve kaynak satırı dahil).
- Program 1 GB'lık yığında çalışır (yorumlayıcıyla aynı). Sonsuz özyineleme Türkçe "Çok derin özyineleme" hatasıyla durur. Fark: yorumlayıcı 100.000 iç içe çağrıda durur ve hatalı çağrının satırını gösterir; derlenmiş program yığın dolunca durur ve fonksiyonun tanımlandığı satırı gösterir.
- Çalışma kütüphanesi (`runtime/`) bir kez LLVM bitcode'a derlenip `~/.cache/kolay/` (Windows'ta `%LOCALAPPDATA%\kolay`) içinde saklanır ve programla birleştirilir (küçük fonksiyonlar programın içine yerleşir). `llvm-link` yoksa nesne dosyası olarak bağlanır.

## Testler

```
./testler/calistir.sh              # tüm testler
./testler/calistir.sh --guncelle   # beklenen çıktıları yeniden yazar (bilerek değişiklik yaptıysanız)
```

Windows'ta aynı betik Git Bash ile çalışır (`bash testler/calistir.sh`). GitHub'a her gönderimde testler
Linux ve Windows'ta otomatik çalışır (`.github/workflows/testler.yml`).

- `testler/calisma/`: çalışması gereken programlar
- `testler/hata/`: hata vermesi gereken programlar ve beklenen Türkçe mesajlar
- `testler/moduller/`: dahil et
- `testler/derleme/`: yorumlayıcı ile derleyicinin aynı çıktıyı (hata mesajları dahil) verdiği programlar
- `testler/repl/`: etkileşimli kip (`.girdi` borudan verilir)

## Lisans

Copyright 2026 Ömer Faruk Yılmaz ([OmerFirig](https://github.com/OmerFirig)).

- **Kod** (dil, derleyici, yorumlayıcı, VS Code eklentisi): [PolyForm Noncommercial 1.0.0](LICENSE). Ticari olmayan
  kullanım, değiştirme ve paylaşım ücretsizdir; ticari kullanım için izin gerekir.
- **Kurs** ([belgeler/Kolay.md](belgeler/Kolay.md)): [CC BY-NC-SA 4.0](belgeler/LICENSE).
- **Eğitim:** Okullarda, üniversitelerde ve kişisel öğrenmede eğitim amaçlı kullanım ve yayın ücretsizdir. Ücretli
  kurs, eğitim ya da ürün satışı ticari kullanımdır ve ayrı izin gerektirir (bkz. [NOTICE](NOTICE)).

Ticari izin ve sorular için: ofy.omer.yilmaz@gmail.com
