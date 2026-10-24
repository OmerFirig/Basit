# Kolay

Türkçe, tip güvenli ve hızlı bir programlama dili. Bitirme projesi.

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

Gerekenler: Rust (cargo) ve derleme için clang (LLVM). İkisi de bu sistemde kurulu.

```
./kur.sh
```

Bu betik projeyi derler ve `kolay` komutunu `~/.local/bin` içine bağlar. Kaldırmak için: `rm ~/.local/bin/kolay`

## Kullanım

| Komut | Ne yapar |
|---|---|
| `kolay` | Etkileşimli kip (REPL): komutları tek tek dener, ifadelerin değerini ve tipini gösterir |
| `kolay kod.kolay` | Programı çalıştırır (yorumlayıcı, hemen başlar) |
| `kolay derle kod.kolay` | Makine koduna derler, `./kod` dosyasını üretir (LLVM, C'ye yakın hız) |
| `kolay derle kod.kolay -o ad --ir` | Çıktıya ad verir, üretilen LLVM IR'yi de `ad.ll` olarak kaydeder |
| `kolay denetle kod.kolay` | Sadece hataları kontrol eder |
| `kolay çevir kod.kolay --klavye us` | Anahtar kelimeleri Türkçe karakter olmayan yazıma çevirir |
| `kolay yardım` | Yardım |

`--klavye tr|us`: Dosyada `klavye("...")` satırı yoksa kullanılacak mod (varsayılan `tr`).

## Etkileşimli kip (REPL)

```
$ kolay
Kolay 0.5.0 — yardım için :yardım, çıkmak için :çık

Girdi [1]: ata x = 5

Girdi [2]: x * 2
Çıktı [2]: 10                         Tam(64)
```

Komutlar: `:yardım`, `:tip ifade`, `:geçmiş`, `:kaydet dosya`, `:yükle dosya`, `:temizle`, `:çık`.
Çalışan bir girdi Ctrl+C ile durdurulur. Ok tuşlarıyla geçmiş (`~/.cache/kolay/repl_gecmis.txt`), Tab ile tamamlama.

## VS Code eklentisi

`editor/vscode/`: `.kolay` dosyaları için renklendirme ve hazır kod parçaları (`eğer`, `kere`, `her`, `tür`, `eşleştir`... yazıp Tab'a basın). Kurmak için:

```
code --install-extension editor/vscode/kolay-0.4.1.vsix
```

## Örnekler

`ornekler/` klasöründe her özelliği gösteren açıklamalı programlar var:

| Dosya | Konu |
|---|---|
| `01_merhaba.kolay` | yazdır, oku, f-yazı |
| `02_sayilar.kolay` | Tam(N), işaret, esnek Tam, Ondalık, olarak, taşma |
| `03_asal_sayilar.kolay` | fonksiyonlar ve tüm döngü türleri |
| `04_hiz_testi.kolay` | fib(35) hız testi |
| `05_okul.kolay` | tür, kalıtım, arayüz, Liste, Sözlük, sıralama |
| `06_sekiller.kolay` | seçenek, eşleştir, veya tipleri, Hiç |
| `07_us_klavye.kolay` | `klavye("us")` modu |

Hepsi hem yorumlayıcıyla hem derleyiciyle çalışır.

## Dil özeti

Tasarım kararlarının tamamı ve gerekçeleri: `../Notes_Claude.md`

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
ata t = 1.0 / 0.0                // TanımsızSayı (sıfıra bölme tanımsız)
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

dahil et("./yardımcı.kolay")
m adıyla dahil et("./matematik.kolay")

#Güvensiz
mat adıyla dış kütüphane("m") {                   // C kütüphanesi
    fonksiyon karekök(x: Ondalık) -> Ondalık = "sqrt"
}
```

## Mimari

```
kaynak (.kolay)
  → lexer.rs      token'lar (klavye modu, yorumlar, f-yazılar, onaltılık sayılar)
  → parser.rs     sözdizimi ağacı (ast.rs)
  → loader.rs     dahil et ile eklenen dosyalar
  → check.rs      tip denetleyici: tanımlar, kalıtım, arayüzler
    cexpr.rs      ifadeler, tip çıkarımı, Türkçe hata mesajları
    cstmt.rs      komutlar, eşleştir, akış analizi (belirsiz/sabit kontrolü)
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
(kalıtım, arayüz, `yeniden`), seçenekler, `veya` tipleri, eşleştir, `olarak`, `dahil et`.

Derlenmiş programlarda:
- Değerler yazmaçlarda durur; yazı, liste, sözlük ve nesneler referans sayımıyla tutulur ve
  kullanılmayınca bellekleri hemen geri verilir (çöp toplayıcı yok). Değer semantiği "yazınca
  kopyala" ile sağlanır: paylaşılan bir liste ancak değiştirilirken kopyalanır.
- Metotlar mümkünse doğrudan, birden fazla aday varsa türe göre `switch` ile çağrılır.
- Hata mesajları yorumlayıcıdakiyle aynıdır (dosya, satır ve kaynak satırı dahil).
- Program 1 GB'lık yığında çalışır (yorumlayıcıyla aynı). Sonsuz özyineleme Türkçe "Çok derin özyineleme" hatasıyla durur. Fark: yorumlayıcı 100.000 iç içe çağrıda durur ve hatalı çağrının satırını gösterir; derlenmiş program yığın dolunca durur ve fonksiyonun tanımlandığı satırı gösterir.
- Çalışma kütüphanesi (`runtime/`) bir kez LLVM bitcode'a derlenip `~/.cache/kolay/` içinde saklanır ve programla birleştirilir (küçük fonksiyonlar programın içine yerleşir). `llvm-link` yoksa nesne dosyası olarak bağlanır.

## Testler

```
./testler/calistir.sh              # tüm testler
./testler/calistir.sh --guncelle   # beklenen çıktıları yeniden yazar (bilerek değişiklik yaptıysanız)
```

- `testler/calisma/`: çalışması gereken programlar
- `testler/hata/`: hata vermesi gereken programlar ve beklenen Türkçe mesajlar
- `testler/moduller/`: dahil et
- `testler/derleme/`: yorumlayıcı ile derleyicinin aynı çıktıyı (hata mesajları dahil) verdiği programlar
- `testler/repl/`: etkileşimli kip (`.girdi` borudan verilir)
