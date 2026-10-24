# Kolay Geliştirici Rehberi

Bu rehber Kolay'a bir şey eklemek isteyenler içindir. Üç yol vardır; en kolayından en zoruna:

1. [Kolay ile kütüphane yazmak](#1-kolay-ile-kütüphane-yazmak): Yeni fonksiyonlar ve türler Kolay'ın kendisiyle yazılır. Derleyiciye dokunulmaz. **Çoğu iş için doğru yol budur.**
2. [Bir C kütüphanesini sarmak](#2-bir-c-kütüphanesini-sarmak): Hazır bir C (ya da C arayüzlü Rust, C++, Go) kütüphanesi Türkçe adlarla kullanılabilir hale getirilir.
3. [Dile hazır metot ya da fonksiyon eklemek](#4-dile-hazır-metot-eklemek): `"yazı".say("a")` gibi her programda hazır olan bir şey eklenir. Derleyicinin birkaç dosyasına dokunulur.

Önce derleyicinin genel yapısını anlamak isterseniz [3. bölüme](#3-derleyicinin-yapısı) bakın.

---

## 1. Kolay ile kütüphane yazmak

Bir kütüphane sıradan bir `.kolay` dosyasıdır. Kullanan dosya onu `dahil et` ile ekler.

Çalışan bir örnek: [`ornekler/08_kutuphane/`](../ornekler/08_kutuphane/)

```kolay
// istatistik.kolay
fonksiyon ortalama(l: Liste<Ondalık>) -> Ondalık {
    doğrula(l.uzunluk() > 0, "Boş listenin ortalaması alınamaz.")
    döndür l.toplam() / l.uzunluk()
}

fonksiyon en_sık<T>(l: Liste<T>) -> T { ... }        // generic: her tiple çalışır

tür Rapor(başlık: Yazı, basamak: Tam(64) = 2) {      // varsayılan değerli alan
    ortak ata yazılan = 0                            // bütün raporların ortak sayacı
    fonksiyon yaz(ad: Yazı, değer: Ondalık) { ... }
}

// kütüphanenin kendi testleri: dahil edilince bir kez çalışır, hata yoksa bir şey yazmaz
doğrula(ortalama([1.0, 2.0, 3.0]) == 2.0)
```

```kolay
// ana.kolay
ist adıyla dahil et("./istatistik.kolay")
yazdır(ist.ortalama([70.0, 85.5]))
ata r = ist.Rapor("Sınıf notları")
yazdır(ist.Rapor.yazılan)
```

Kurallar ve öneriler:

- **İsimle dahil etmeyi önerin:** `ist adıyla dahil et(...)`. Kütüphanedeki her şeye `ist.` ile erişilir; başka dosyalardaki adlarla çakışmaz. İsimsiz `dahil et(...)` küçük projeler içindir.
- **Dışarıya açık olan her şey üst düzeydedir:** Dosyanın üst düzeyindeki fonksiyonlar, türler, seçenekler, arayüzler ve değişkenler kullanan dosyadan görünür. Gizli yardımcı fonksiyon kavramı şimdilik yok; yardımcıların adını `_` ile başlatmak gelenek olabilir.
- **Üst düzey komutlar bir kez çalışır:** Dosya dahil edildiği anda üst düzeydeki komutlar çalışır. Burada `yazdır` kullanmayın; sadece hazırlık ve `doğrula` ile testler yazın.
- **Testleri kütüphanenin içine yazın:** `doğrula(...)` satırları kütüphane her yüklendiğinde kontrol edilir. Yanlışsa hangi satırın tutmadığı Türkçe yazılır.
- **Hataları açık verin:** Yanlış kullanım için `doğrula(koşul, "mesaj")` ya da `fırlat Hata(mesaj: "...")`. Başarısız olabilen işlemlerde `-> T veya Hata` döndürmek kullanıcıya seçim bırakır (bkz. kurs, Hata yönetimi).
- **Generic yazın:** Bir fonksiyon her tiple aynı şeyi yapıyorsa `<T>` ile bir kez yazın. Hata olursa mesaj hem kütüphanedeki satırı hem kullanıcının çağrısını (dosya adıyla) gösterir.
- **Yollar dahil eden dosyaya göredir:** `dahil et("./alt/kutuphane.kolay")`. Her dosya bir kez yüklenir; döngüsel dahil etme hata verir.
- **Klavye modu dosya başınadır:** Kütüphane `tr` modunda yazılsa da `klavye("us")` kullanan bir dosya onu dahil edebilir.

---

## 2. Bir C kütüphanesini sarmak

C'deki bir fonksiyon `#Güvensiz dış kütüphane` ile tanımlanır. En iyi yöntem, bunu bir Kolay kütüphanesinin içine koyup Türkçe ve güvenli fonksiyonlarla sarmaktır. Böylece kullanıcı `#Güvensiz` görmez:

```kolay
// matematik.kolay
#Güvensiz
c adıyla dış kütüphane("m") {                         // libm
    fonksiyon karekök(x: Ondalık) -> Ondalık = "sqrt"  // Kolay'daki adı = C'deki adı
    fonksiyon üs(a: Ondalık, b: Ondalık) -> Ondalık = "pow"
}

ata sabit PI = 3.141592653589793

fonksiyon hipotenüs(a: Ondalık, b: Ondalık) -> Ondalık {
    döndür c.karekök(a * a + b * b)
}
```

```kolay
mat adıyla dahil et("./matematik.kolay")
yazdır(mat.hipotenüs(6.0, 8.0))      // 10.0
```

- C'ye verilebilen tipler: `Tam(8/16/32/64)`, `Tam(N, +)`, `Ondalık`, `Ondalık(32)`, `Mantık`, `Yazı` (C'de `const char*`). Liste, sözlük ve nesne verilemez.
- İmza yanlışsa program çöker; `#Güvensiz` zorunluluğu bu yüzden var. İmzayı C başlık dosyasından dikkatle kopyalayın.
- Kütüphane adı: `"m"` → `libm.so`. Kendi derlediğiniz kütüphane için yol: `"./libbenim.so"`. Rust (`#[no_mangle] pub extern "C"`), C++ (`extern "C"`) ve Go (`//export`) kütüphaneleri de aynı şekilde kullanılır.

---

## 3. Derleyicinin yapısı

```
kaynak (.kolay)
  → lexer.rs      token'lar; klavye modu (tr/us), f-yazılar, sayılar
  → parser.rs     sözdizimi ağacı (ast.rs)
  → loader.rs     dahil et ile eklenen dosyalar
  → check.rs      tip denetleyici: adlar, türler, kalıtım, arayüzler, ortak değerler
    cexpr.rs      ifadeler, çağrılar, hazır metotların tablosu (builtin_method)
    cstmt.rs      komutlar, eşleştir, akış analizi (atanmış mı, sabit mi), fonksiyon gövdeleri
    generic.rs    generic fonksiyon ve türler: tip çıkarımı, her tip için kopya
    varsayilan.rs varsayılan değerler ve türün gövdesindeki alanlar
    firlat.rs     hangi değerin nereye fırlatılabileceği (yakala dalları, uyarılar)
  → tir.rs        tipleri çözülmüş ara gösterim (TIR): iki arka uç da bunu kullanır
      → vm.rs + interp.rs   yorumlayıcı (kolay kod.kolay); values.rs çalışma anı değerleri
      → codegen/            LLVM IR üretici (kolay derle); runtime/*.c çalışma kütüphanesi
```

Bilmeniz gereken üç ilke:

1. **Tip denetleyici TIR üretir, arka uçlar sadece TIR'ı çalıştırır.** Generic, varsayılan değer, `ortak`, `doğrula` gibi özellikler tip denetleyicide var olan TIR yapılarına çevrilir. Yorumlayıcı ve derleyici bunlardan habersizdir. Yeni bir özellik eklerken önce "bunu var olan TIR ile yazabilir miyim?" diye bakın; çoğu zaman yazılabilir ve arka uçlara dokunmak gerekmez.
2. **İki arka uç aynı sonucu vermeli:** Çıktılar ve hata mesajları dahil. `testler/derleme/` altındaki her program iki yolla da çalıştırılıp karşılaştırılır.
3. **Hata mesajları Türkçe ve yol göstericidir:** Ne yanlış, nasıl düzeltilir, mümkünse bir örnekle. Mesajın yeri (`Span`) kullanıcının yazdığı yeri göstermeli.

| Dosya | Ne zaman açılır |
|---|---|
| `src/tir.rs` | `BM` (hazır metotlar), `Builtin` (hazır fonksiyonlar), `TK`/`TS` (ifade/komut) |
| `src/cexpr.rs` | `builtin_method` (metot tablosu), `call_expr` (hazır fonksiyonlar), `method_call` |
| `src/interp.rs` | `bm_val` (değer üzerindeki hazır metotlar), `eval_cold` (hazır fonksiyonlar) |
| `src/codegen/yerlesik.rs` | `value_bmethod` (değer üzerindeki hazır metotlar), `bmethod` (listeyi değiştirenler) |
| `src/codegen/mod.rs` | Dosyanın başı: kullanılan C fonksiyonlarının `declare` satırları ve bellek kuralları |
| `runtime/*.c` | Derlenmiş programın C fonksiyonları (`yazilar.c`, `koleksiyon.c`, `sayilar.c`...) |
| `src/lexer.rs` | `KEYWORDS` (anahtar kelimeler), `BUILTIN_NAMES` (Türkçe karaktersiz yazımlar) |

---

## 4. Dile hazır metot eklemek

Örnek olarak `"ananas".say("a")` → `3` metodunun nasıl eklendiğini adım adım gösteriyoruz. Her adım ayrı bir dosyadadır.

**1. Metodun adı: `src/tir.rs`, `enum BM`:**
```rust
Say,
```

**2. Tipi: `src/cexpr.rs`, `builtin_method`, alıcının tipine göre kol (burada `Type::Str`):**
```rust
"say" => mk(BM::Say, vec![p("parça", Type::Str)], TAM64, false, false),
```
`mk(metot, parametreler, dönüş tipi, değiştirir_mi, boyutu_değiştirir_mi)`. Yazı metotları yeni değer verdiği için ikisi de `false`. Listeyi değiştiren metotlarda (`ekle`) `true`'dur; `sabit` listeler bunu kullanarak korunur. Aynı dosyada `builtin_method_names` listesine de adı ekleyin ("'sya' mı demek istediniz?" önerileri için).

**3. Yorumlayıcı: `src/interp.rs`, `bm_val`:**
```rust
(BM::Say, Value::Str(s)) => {
    let Value::Str(p) = &args[0] else { unreachable!() };
    if p.is_empty() {
        return rt(loc, "Aranan parça boş olamaz.");      // genel Hata; türlü hata için rtk(hata_turu::X, ...)
    }
    Value::int(s.matches(p.as_str()).count() as i64)
}
```

**4. Derlenmiş program için C fonksiyonu: `runtime/yazilar.c`:**
```c
int64_t kolay_say(Yazi *s, Yazi *p, const char *konum, const char *kod) {
    if (p->uzunluk == 0) kolay_hata_tur(HT_GENEL, "Aranan parça boş olamaz.", konum, kod);
    ...
}
```
Mesaj yorumlayıcıdakiyle **harfi harfine aynı** olmalı. Hata verebilen fonksiyonlar `konum` ve `kod` alır; hatanın satırını göstermek için.

**5. C fonksiyonunun bildirimi: `src/codegen/mod.rs`, dosyanın başındaki `declare` listesi:**
```
declare i64 @kolay_say(ptr, ptr, ptr, ptr)
```

**6. Çağrının üretilmesi: `src/codegen/yerlesik.rs`, `value_bmethod`:**
```rust
BM::Say => {
    self.emit(&format!("{t} = call i64 @kolay_say(ptr {v}, ptr {}, ptr {k}, ptr {kd})", a[0]));
    t
}
```
`v` alıcı, `a[i]` argümanlar, `k`/`kd` hatanın yeri. Sonuç bir yazı, liste ya da nesneyse `self.own(t, &tip)` ile döndürün (referans sayımı; kurallar `codegen/mod.rs`'nin başında).

**7. Türkçe karaktersiz yazım: `src/lexer.rs`, `BUILTIN_NAMES`.** Sadece adda Türkçe harf varsa gerekir: `("değiştir", "degistir")`.

**8. Testler:** `testler/derleme/` altına metodu kullanan bir program ekleyin, hata durumları için `testler/hata/` altına numaralı dosyalar. Sonra:
```
./testler/calistir.sh --guncelle    # beklenen çıktıları yazar; farkı mutlaka gözden geçirin
./testler/calistir.sh               # hepsi geçmeli
```

**9. Editör ve belgeler:** `editor/vscode/extension.js` (`METHODS` listesi: tamamlama), `editor/vscode/syntaxes/kolay.tmLanguage.json` (metot renklendirme), kurs (`belgeler/Kolay.md`, Yazılar bölümü ve başvuru tablosu).

> **Kısa yol:** Yeni metot var olan metotlarla yazılabiliyorsa, 3–6. adımlar yerine tip denetleyicide (2. adım) var olan TIR'a çevirin. Örneğin `yazı.tersi()` aslında `yazı[-1..=0]` diliminin kendisidir (`cexpr.rs`, `method_call`).

### Listeyi değiştiren metotlar
`ekle` gibi alıcıyı değiştiren metotlar alıcıyı bir **yer** (`Recv::Place`) olarak alır. Derleyicide `yerlesik.rs`'deki `bmethod` fonksiyonunda işlenir. Yazınca kopyala kuralı yüzünden listenin önce tek sahipli olması sağlanır. Var olan `BM::Ekle` örneğini izleyin.

---

## 5. Dile hazır fonksiyon eklemek

`yazdır`, `oku` ve `doğrula` gibi her yerde hazır olan fonksiyonlar `src/cexpr.rs`'deki `call_expr` içinde adıyla tanınır (`match name.as_str()`). Adı `BUILTIN_FUNCS` listesine de ekleyin.

İki yol vardır:

- **Var olan TIR'a çevirmek (önerilen):** `doğrula(koşul, "mesaj")` aslında `eğer değil koşul { fırlat Hata(mesaj: ...) }` komutudur (`cstmt.rs`, `dogrula`). `yazdır(..., ayraç: ", ")` ise tek bir yazıyı yazan `Builtin::Yaz`'a çevrilir (`cexpr.rs`, `yazdir`). Arka uçlara dokunmak gerekmez.
- **Yeni bir `Builtin`:** `src/tir.rs`, `enum Builtin`'e ekleyin. Yorumlayıcıda `interp.rs`, `eval_cold` içinde işleyin; bytecode yorumlayıcı (`vm.rs`) tanımadığı ifadeleri kendiliğinden `interp.rs`'ye devreder. Derleyicide `codegen/ifade.rs`, `TK::Builtin` kollarına ekleyin.

---

## 6. Yeni sözdizimi eklemek

1. Yeni anahtar kelime: `src/lexer.rs`, `enum Kw` ve `KEYWORDS` (tr ve us yazımı).
2. Ağaç: `src/ast.rs`'ye yeni düğüm, `src/parser.rs`'de okuma. Hatalı yazımlar için yol gösterici mesaj yazın (örnek: `'ortak' sonrasında 'ata' ya da 'fonksiyon' yazılır`).
3. Tip denetimi: `check.rs` / `cexpr.rs` / `cstmt.rs`. Mümkünse var olan TIR'a çevirin (bkz. 3. bölümdeki ilke).
4. Testler, editör (`kolay.tmLanguage.json` anahtar kelime listesi, `extension.js` `KEYWORDS`), kurs.

---

## 7. Kontroller

- **Testler:** `./testler/calistir.sh`. Her değişiklikten sonra hepsi geçmeli.
- **İki arka uç aynı mı:** `testler/derleme/` altındaki programlar bunu kendiliğinden kontrol eder.
- **Bellek hatası:** Derlenmiş programı AddressSanitizer ile çalıştırın:
  ```
  kolay derle x.kolay -o x --ir
  cat runtime/kolay.h runtime/cekirdek.c runtime/sayilar.c runtime/yazilar.c runtime/genel.c runtime/koleksiyon.c > rt.c
  clang -O1 -g -fsanitize=address -w x.ll rt.c -lm -lpthread -o x_asan && ./x_asan
  ```
- **Derleme uyarısı:** `cargo build --release` uyarısız bitmeli.
