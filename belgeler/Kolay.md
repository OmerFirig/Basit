# Kolay Kursu: Sıfırdan Programlamaya

Kolay, Türkçe yazılan, tip güvenli ve hızlı bir programlama dilidir. Bu kurs hiç programlama bilmeyen birini adım adım ilk programından nesnelere, kendi veri tiplerine, hataları yakalamaya ve C kütüphanelerini kullanmaya kadar götürür.

> Bu kurs **Kolay 0.6.0** içindir. Hangi sürümün kurulu olduğunu etkileşimli kipi açarak (`kolay`) görebilirsiniz.

Kursun amacı yalnızca Kolay'ı öğretmek değil, **programlamanın temel mantığını** oturtmaktır: değişken, tip, koşul, döngü, fonksiyon, liste, nesne. Bu kavramlar bütün dillerde aynıdır. Kolay'da öğrendiğiniz her şeyin Python karşılığını kursun sonunda bulacaksınız. Kolay'ı bitiren biri Python'a birkaç günde geçebilir.

> **Nasıl çalışmalı?** Her örneği kendiniz yazın, çalıştırın, sonra değiştirip ne olduğuna bakın. Hata almak öğrenmenin parçasıdır: Kolay'ın hata mesajları size ne yapmanız gerektiğini Türkçe söyler. Her bölümün sonundaki alıştırmaları çözmeden sonraki bölüme geçmeyin; çözümler gizli kutuların içinde.

## İçindekiler

1. [Başlarken](#1-başlarken)
2. [İlk program: yazdır](#2-ilk-program-yazdır)
3. [Etkileşimli kip: denemeler için](#3-etkileşimli-kip-denemeler-için)
4. [Değişkenler](#4-değişkenler)
5. [Sayılar](#5-sayılar)
6. [Yazılar](#6-yazılar)
7. [Mantık ve karşılaştırma](#7-mantık-ve-karşılaştırma)
8. [Koşullar: eğer](#8-koşullar-eğer)
9. [Kullanıcıdan bilgi almak: oku](#9-kullanıcıdan-bilgi-almak-oku)
10. [Döngüler](#10-döngüler)
11. [Fonksiyonlar](#11-fonksiyonlar)
12. [Listeler](#12-listeler)
13. [Sözlükler](#13-sözlükler)
14. [Lambda: fonksiyonları değer olarak kullanmak](#14-lambda-fonksiyonları-değer-olarak-kullanmak)
15. [Türler: kendi veri tipinizi yazmak](#15-türler-kendi-veri-tipinizi-yazmak)
16. [Kalıtım](#16-kalıtım)
17. [Arayüzler](#17-arayüzler)
18. [Seçenekler ve eşleştir](#18-seçenekler-ve-eşleştir)
19. [veya tipleri ve Hiç](#19-veya-tipleri-ve-hiç)
20. [Generic: her tiple çalışan kod](#20-generic-her-tiple-çalışan-kod)
21. [Hata yönetimi: dene, yakala, fırlat](#21-hata-yönetimi-dene-yakala-fırlat)
22. [Birden fazla dosya: dahil et](#22-birden-fazla-dosya-dahil-et)
23. [Hata mesajlarını okumak](#23-hata-mesajlarını-okumak)
24. [Türkçe karakter olmadan yazmak](#24-türkçe-karakter-olmadan-yazmak)
25. [Hız: derlemek](#25-hız-derlemek)
26. [C kütüphanelerini kullanmak](#26-c-kütüphanelerini-kullanmak)
27. [Projeler](#27-projeler)
28. [Kolay'dan Python'a](#28-kolaydan-pythona)
29. [Başvuru tabloları](#29-başvuru-tabloları)

---

## 1. Başlarken

### Program nedir?

Bilgisayar, kendisine verilen komutları **sırayla, harfiyen** yapan bir makinedir. Program, bu komutların yazılı listesidir. Programlama dili de komutları hem insanın okuyabileceği hem bilgisayarın anlayabileceği şekilde yazmamızı sağlar.

Kolay'da bir program, uzantısı `.kolay` olan sıradan bir metin dosyasıdır.

### Kurulum

Kolay kuruluysa terminalde şu komut yardım metnini gösterir:

```
kolay yardım
```

Kurulu değilse Kolay'ın kaynak klasöründe `./kur.sh` betiğini çalıştırın. Betik programı derler ve `kolay` komutunu kullanılabilir hale getirir.

### Program yazmak ve çalıştırmak

1. Bir metin düzenleyici açın (önerilen: VS Code ve Kolay eklentisi; renklendirme, tamamlama ve hata gösterme sağlar).
2. `merhaba.kolay` adında bir dosya oluşturup içine şunu yazın:

```kolay
yazdır("Merhaba dünya!")
```

3. Terminalde dosyanın bulunduğu klasöre gidip çalıştırın:

```
kolay merhaba.kolay
```

Çıktı:

```
Merhaba dünya!
```

VS Code'da dosya açıkken **F5**'e ya da sağ üstteki ▷ düğmesine basmak da aynı işi yapar.

### Kolay komutları

| Komut | Ne yapar |
|---|---|
| `kolay` | **Etkileşimli kipi** açar: satır satır deneme yapılır (bkz. [Etkileşimli kip](#3-etkileşimli-kip-denemeler-için)) |
| `kolay dosya.kolay` | Programı hemen çalıştırır (`kolay çalıştır dosya.kolay` de aynıdır) |
| `kolay derle dosya.kolay` | Programı makine koduna çevirir, `./dosya` adında hızlı bir program üretir |
| `kolay derle dosya.kolay -o oyun` | Üretilen programın adını belirler (`./oyun`) |
| `kolay derle dosya.kolay --ir` | Ayrıca derleyicinin ürettiği ara kodu (LLVM IR) `dosya.ll` olarak kaydeder (ileri seviye) |
| `kolay denetle dosya.kolay` | Çalıştırmadan sadece hataları kontrol eder; hata yoksa `Hata bulunamadı.` yazar |
| `kolay çevir dosya.kolay --klavye us` | Türkçe karakterli yazımı Türkçe karaktersiz yazıma çevirir ve ekrana yazar |
| `kolay çevir dosya.kolay --klavye us -o yeni.kolay` | Çeviriyi bir dosyaya yazar |
| `kolay --klavye us dosya.kolay` | Dosyada `klavye(...)` satırı yoksa hangi yazımın kullanılacağını belirler (varsayılan `tr`) |
| `kolay yardım` | Yardım |

Bir komut yanlış yazılırsa Kolay ne yapılması gerektiğini söyler: `Tanınmayan seçenek: '-x'. Yardım için: kolay yardım`.

### VS Code eklentisi

Kolay'ın VS Code eklentisi yazmayı kolaylaştırır:

- **Renklendirme:** anahtar kelimeler, tipler, yazılar, sayılar ve yorumlar farklı renklerde görünür.
- **Tamamlama:** yazmaya başlayınca anahtar kelimeler, hazır fonksiyonlar ve dosyadaki değişkenler, fonksiyonlar, türler ve tip parametreleri (`T`) önerilir. Bir değişkenden sonra `.` yazınca o değişkenin tipine uygun metotlar ve alanlar listelenir (`liste.` → `ekle`, `uzunluk`, ...; `Hata.` → hata türleri).
- **Hata gösterme:** dosya açılınca ve her kaydedişte `kolay denetle` çalışır; hatalı yerin altı kırmızı çizilir, üzerine gelince Türkçe mesaj görünür.
- **Çalıştırma:** **F5** ya da sağ üstteki ▷ düğmesi programı terminalde çalıştırır. Komut paletindeki (Ctrl+Shift+P) `Kolay: Derle` programı derler.
- **Hazır kod parçaları:** `eğer`, `kere`, `her`, `fonksiyon`, `tür`, `eşleştir`, `dene`, `fırlat`, `lambda` gibi bir kelime yazıp Tab'a basınca kalıbın tamamı gelir; Tab ile boşluklar arasında gezilir. Generic fonksiyon için `fonksiyong`, generic tür için `türg` (bkz. [Generic](#20-generic-her-tiple-çalışan-kod)); `ortak`, `ortakf`, `doğrula`, biçimli f-yazı için `yazdırb`.

Kurmak için Kolay'ın kaynak klasöründe:

```
code --install-extension editor/vscode/kolay-0.6.0.vsix
```

Eklenti `kolay` komutunu kullanır; komut başka bir yerdeyse VS Code ayarlarında `kolay.yol` ile yolunu yazabilirsiniz.

---

## 2. İlk program: yazdır

`yazdır` ekrana bir şey yazar. Yazılar çift tırnak `"` içine alınır:

```kolay
yazdır("Merhaba!")
yazdır("Kolay öğreniyorum.")
yazdır(42)
yazdır(3.14)
```

Çıktı:

```
Merhaba!
Kolay öğreniyorum.
42
3.14
```

Her `yazdır` yeni bir satıra yazar. Program yukarıdan aşağıya, satır satır çalışır.

### Birden fazla değer

`yazdır`'a virgülle ayrılmış birden fazla değer verilebilir. Aralarına boşluk konur:

```kolay
yazdır("Sonuç:", 10, "puan")
```

```
Sonuç: 10 puan
```

### İşlemler

`yazdır`'ın içine işlem de yazılabilir. Önce işlem yapılır, sonra sonuç yazılır:

```kolay
yazdır(2 + 3)
yazdır(10 * 4 - 2)
yazdır((10 + 2) * 3)
```

```
5
38
36
```

Matematikteki gibi çarpma ve bölme toplamadan önce yapılır, parantezler sırayı değiştirir.

### Yorumlar

`//` işaretinden sonra satırın sonuna kadar yazılanlar **yorumdur**: bilgisayar onları okumaz, insanlar için not olarak yazılır. Birden fazla satırlık yorum `/<` ile başlar, `>/` ile biter:

```kolay
// Bu program bir selam yazar
yazdır("Selam")   // satırın sonuna da yorum yazılabilir

/<
  Bu satırların hepsi yorumdur.
  yazdır("bu çalışmaz")
>/
```

İyi bir yorum kodun **ne** yaptığını değil, **neden** öyle yazıldığını anlatır.

### f-yazı: yazının içine değer koymak

Yazının önüne `f` konursa, süslü parantez `{ }` içindeki her şey hesaplanıp yazıya yerleştirilir:

```kolay
yazdır(f"İki kere iki {2 * 2} eder")
yazdır("İki kere iki {2 * 2} eder")
yazdır(f"Süslü parantezin kendisi için iki tane yazın: {{ }}")
```

```
İki kere iki 4 eder
İki kere iki {2 * 2} eder
Süslü parantezin kendisi için iki tane yazın: { }
```

İkinci satırda `f` olmadığı için süslü parantezler olduğu gibi yazıldı.

Değerin nasıl görüneceğini de seçebilirsiniz: `f"{pi:.2}"` → `3.14`. Bunu [Yazılar](#6-yazılar) bölümünde göreceğiz.

### Özel karakterler

Yazı içinde bazı karakterler ters bölü `\` ile yazılır:

| Yazım | Anlamı |
|---|---|
| `\n` | yeni satır |
| `\t` | sekme (tab) |
| `\"` | çift tırnak |
| `\\` | ters bölünün kendisi |

```kolay
yazdır("Birinci satır\nİkinci satır")
yazdır("Ad:\tAli")
yazdır("O \"merhaba\" dedi.")
```

### Alıştırmalar

1. Adınızı, yaşınızı ve şehrinizi üç ayrı satıra yazan bir program yazın.
2. Bir haftada kaç saniye olduğunu hesaplayıp `Bir haftada ... saniye var` şeklinde yazdırın.

<details>
<summary>Çözümler</summary>

```kolay
// 1
yazdır("Ali")
yazdır(20)
yazdır("Ankara")

// 2
yazdır(f"Bir haftada {7 * 24 * 60 * 60} saniye var")
```

</details>

---

## 3. Etkileşimli kip: denemeler için

Bir şeyi denemek için her seferinde dosya yazıp çalıştırmak zorunda değilsiniz. Terminalde dosya adı vermeden yalnızca `kolay` yazarsanız **etkileşimli kip** açılır (İngilizcede REPL denir: oku, çalıştır, yaz, tekrarla). Burada her satır yazılır yazılmaz çalışır:

```
$ kolay
Kolay 0.6.0 — yardım için :yardım, çıkmak için :çık

Girdi [1]:
```

Etkileşimli kip; bir ifadenin sonucunu görmek, bir metodun ne yaptığını denemek ya da hata mesajlarını tanımak için idealdir. Bu kursta "bunu bir deneyin" dediğimiz her şeyi burada deneyebilirsiniz.

### Girdi ve Çıktı hücreleri

Her yazdığınız şey bir **hücredir** ve bir numara alır. Bir **ifade** yazarsanız (değeri olan bir şey: `fiyat * 3`, `"abc".uzunluk()`) değeri `Çıktı [n]:` satırında, tipi de sağda soluk renkte gösterilir:

```
Girdi [1]: ata fiyat = 120

Girdi [2]: fiyat * 3
Çıktı [2]: 360                        Tam(64)

Girdi [3]: f"Toplam: {fiyat * 3} TL"
Çıktı [3]: "Toplam: 360 TL"           Yazı
```

- `ata fiyat = 120` bir komuttur, değeri yoktur; bu yüzden çıktı satırı yazılmaz.
- Çıktıdaki yazılar tırnak içinde gösterilir (`"Toplam: 360 TL"`), böylece yazı mı sayı mı olduğu belli olur. `yazdır` ise tırnaksız yazar.
- Tanımladığınız değişkenler, fonksiyonlar ve türler oturum boyunca kalır: 2. hücre 1. hücredeki `fiyat`'ı kullandı.

### Çok satırlı girdiler

Bir `{` açık kaldığı sürece Enter'a bastığınızda girdi bitmez, `...:` ile alt satırdan devam edilir. Girinti kendiliğinden yapılır; satırın başında `}` yazınca bir girinti geri alınır:

```
Girdi [4]: fonksiyon kdv(x: Tam(64)) -> Tam(64) {
      ...:     döndür x * 20 / 100
      ...: }

Girdi [5]: kdv(fiyat)
Çıktı [5]: 24                         Tam(64)
```

Açık kalan parantez `(`, köşeli parantez `[`, yazı `"` ve çok satırlı yorum `/< >/` da girdiyi devam ettirir.

### Hatalar

Hatalar tek satırda, `Hata [n]:` ile gösterilir. **Hatalı bir girdi oturuma eklenmez**: yanlış yazdığınız bir satır, sonraki girdileri bozmaz. Kolay, çalışma hatalarında da oturumu kapatmaz:

```
Girdi [6]: ata yaş: Tam(8) = 300
Hata [6]: 300, Tam(8) sınırlarını aşıyor (-128 .. 127).

Girdi [7]: fiyt + 1
Hata [7]: 'fiyt' tanımlı değil. 'fiyat' mı demek istediniz?

Girdi [8]: 1 / 0
Hata [8]: Sıfıra bölme hatası.

Girdi [9]: fiyat
Çıktı [9]: 120                        Tam(64)
```

Uyarılar da benzer şekilde `Uyarı [n]:` ile yazılır.

### Komutlar

İki nokta `:` ile başlayan satırlar Kolay kodu değil, etkileşimli kipin kendi komutlarıdır:

| Komut | Ne yapar |
|---|---|
| `:yardım` | Komutların listesini gösterir |
| `:tip ifade` | İfadeyi **çalıştırmadan** tipini gösterir |
| `:geçmiş` | Oturumdaki hatasız girdileri numaralarıyla listeler |
| `:kaydet dosya.kolay` | Hatasız girdileri bir dosyaya yazar (denemelerinizi programa çevirmenin en kolay yolu) |
| `:yükle dosya.kolay` | Bir dosyayı oturuma tek bir girdi olarak çalıştırır; içindeki fonksiyon ve türler sonra da kullanılabilir |
| `:temizle` | Oturumu sıfırlar: bütün değişkenler, fonksiyonlar ve türler silinir |
| `:çık` | Çıkar (Ctrl+D de olur) |

`:tip` özellikle bir metodun ne döndürdüğünü öğrenmek için kullanışlıdır:

```
Girdi [10]: :tip "42".tama()
Çıktı [10]: Tam(64) veya Hata.Dönüşüm

Girdi [11]: :tip kdv
Çıktı [11]: Fonksiyon(Tam(64)) -> Tam(64)
```

(Bu tiplerin ne demek olduğunu ileride göreceğiz.)

`:geçmiş` sadece hatasız girdileri listeler; numaralar girdi numaralarıdır, aradaki eksik numaralar hatalı girdilerdir:

```
Girdi [12]: :geçmiş
[1] ata fiyat = 120
[2] fiyat * 3
[3] f"Toplam: {fiyat * 3} TL"
[4] fonksiyon kdv(x: Tam(64)) -> Tam(64) {
        döndür x * 20 / 100
    }
[5] kdv(fiyat)
[9] fiyat
```

### Klavye kısayolları

| Tuş | Ne yapar |
|---|---|
| ↑ / ↓ | Önceki girdiler arasında gezinir. Geçmiş, kapatıp açınca da kalır (`~/.cache/kolay/repl_gecmis.txt`) |
| Tab | Yarım kalan kelimeyi tamamlar: anahtar kelimeler, hazır fonksiyonlar, oturumdaki isimler, `:` komutları |
| Ctrl+C | Çalışan girdiyi durdurur (`Hata [n]: Durduruldu (Ctrl+C).`); etkileşimli kip açık kalır. Yazılmakta olan satırı da siler |
| Ctrl+D | Çıkar |

Sonsuz döngüye giren bir deneme yaparsanız (`doğru sürece yap { }`) Ctrl+C ile durdurup kaldığınız yerden devam edebilirsiniz; tanımladığınız değişkenler kaybolmaz.

### Bilmekte yarar var

- Etkileşimli kip, programların çalıştırıldığı yorumlayıcının aynısını kullanır: bir girdi burada nasıl çalışıyorsa dosyada da öyle çalışır.
- Her girdi çalıştırılmadan önce, oturumdaki bütün girdilerle birlikte yeniden denetlenir; ama **önceki girdiler tekrar çalıştırılmaz**. `yazdır("merhaba")` bir kez yazılır.
- `dahil et("./dosya.kolay")` yolları, etkileşimli kipi açtığınız klasöre göredir.
- Türkçe karakter olmadan yazmak için `kolay --klavye us` ile açın (bkz. [Türkçe karakter olmadan yazmak](#24-türkçe-karakter-olmadan-yazmak)).

### Alıştırma

Etkileşimli kipi açın ve şunları deneyin: `7 / 2`, `7.0 / 2`, `"istanbul".büyük_harf()`, `:tip 3 < 5`. Sonra `ata x = 10` yazıp `x = "on"` yazmayı deneyin: hata mesajını okuyun. Son olarak `:kaydet denemelerim.kolay` ile girdilerinizi kaydedip `kolay denemelerim.kolay` ile çalıştırın.

---

## 4. Değişkenler

Değişken, bir değere verilen isimdir. Değeri bir kutuya koyup kutunun üstüne etiket yapıştırmak gibi düşünebilirsiniz. Yeni bir değişken `ata` ile tanımlanır:

```kolay
ata ad = "Ayşe"
ata yaş = 21
yazdır(f"{ad} {yaş} yaşında")
```

```
Ayşe 21 yaşında
```

### Değeri değiştirmek

Değişken bir kez tanımlandıktan sonra `ata` yazmadan yeni değer verilebilir:

```kolay
ata puan = 10
yazdır(puan)
puan = 25
yazdır(puan)
puan = puan + 5      // eski değerin 5 fazlası
yazdır(puan)
```

```
10
25
30
```

`puan = puan + 5` matematikteki eşitlik değildir. "Sağdakini hesapla, soldaki kutuya koy" demektir.

Kısaltmalar da vardır:

| Kısa yazım | Uzun hali |
|---|---|
| `x += 5` | `x = x + 5` |
| `x -= 5` | `x = x - 5` |
| `x *= 5` | `x = x * 5` |
| `x /= 5` | `x = x / 5` |
| `x **= 2` | `x = x ** 2` (üs, bkz. [Sayılar](#5-sayılar)) |

Bit işlemlerinin de kısaltmaları vardır (`&=`, `|=`, `^=`, `<<=`, `>>=`); onları da sayılar bölümünde göreceğiz. Kısaltma `%=` yoktur: `x = x % 3` yazılır.

### İsim kuralları

- İsimler harf ya da `_` ile başlar; harf, rakam ve `_` içerebilir: `ad`, `toplam_puan`, `öğrenci2`.
- Türkçe harfler serbesttir: `ağırlık`, `şehir`, `çiçek_sayısı`.
- Boşluk olamaz: `toplam puan` yerine `toplam_puan` yazın.
- `ata`, `eğer`, `yap` gibi dilin kendi kelimeleri isim olarak kullanılamaz.
- Büyük-küçük harf farklıdır: `ad` ile `Ad` ayrı isimlerdir. Değişkenler genelde küçük harfle başlar, tipler büyük harfle (`Yazı`, `Tam`).

### Tipler

Her değerin bir **tipi** vardır. Tip, değerin ne çeşit bir şey olduğunu söyler:

| Tip | Ne tutar | Örnek |
|---|---|---|
| `Tam` | tam sayı | `42`, `-7`, `0` |
| `Ondalık` | ondalıklı sayı | `3.14`, `-0.5`, `2.0` |
| `Yazı` | metin | `"merhaba"` |
| `Mantık` | doğru ya da yanlış | `doğru`, `yanlış` |

Tipi yazmanız gerekmez, Kolay değere bakıp anlar (buna **tip çıkarımı** denir). İsterseniz iki noktadan sonra açıkça yazabilirsiniz:

```kolay
ata ad: Yazı = "Can"
ata boy: Ondalık = 1.78
ata öğrenci_mi: Mantık = doğru
yazdır(ad, boy, öğrenci_mi)
```

Tip, değişkenin sözüdür: bir kez `Yazı` olan değişkene sayı konamaz.

```kolay
ata yaş = 21
yaş = "yirmi bir"     // ✗ HATA
```

```
HATA (deneme.kolay, satır 2): 'yaş' Tam(64) tipinde, Yazı atanamaz. Tipini değiştirmek için 'yaş: Yazı = ...' yazın.
```

Bu kural sayesinde "burada sayı vardı, nasıl yazı oldu?" türü hatalar program daha çalışmadan yakalanır.

### Tipi bilerek değiştirmek

Bir değişkenin tipini gerçekten değiştirmek istiyorsanız yeni tipi açıkça yazarsınız. Bu, aynı isimde yeni bir değişken oluşturur:

```kolay
ata ölçü = "on dört harf!!"
ölçü: Tam(64) = ölçü.uzunluk()   // artık Tam(64)
yazdır(ölçü + 1)
```

```
15
```

`ölçü: Tam(64) = ...` ile `ata ölçü: Tam(64) = ...` aynı anlama gelir. `ata` yazmadan yapılan bu yazım yalnızca **zaten var olan** bir değişken için geçerlidir; böylece yazım hatası yapılmış bir isim sessizce yeni bir değişken olmaz:

```kolay
ata sayaç = 0
saycç: Tam = 5    // ✗ HATA
```

```
HATA (deneme.kolay, satır 2): 'saycç' tanımlı değil. Yeni değişken için 'ata saycç: Tam = ...' yazın.
  2 | saycç: Tam = 5    // ✗ HATA
    | ^^^^^
```

### Sabitler: değişmeyen değerler

`ata sabit` ile tanımlanan bir değer bir daha değiştirilemez:

```kolay
ata sabit pi = 3.14159
ata sabit okul = "Atatürk Lisesi"
yazdır(pi * 2)
```

Sabiti değiştirmeye çalışmak hata verir:

```kolay
ata sabit pi = 3.14159
pi = 3.0      // ✗ HATA: 'pi' sabit ve zaten bir değer atandı; değeri değiştirilemez.
```

Değişmeyecek her şeyi sabit yapmak iyi bir alışkanlıktır: okuyan kişi o değerin hiç değişmediğini hemen anlar.

Sabitin değeri sonradan da verilebilir, ama **yalnızca bir kez**:

```kolay
ata saat = 15
ata sabit mesaj: Yazı
eğer saat < 12 {
    mesaj = "Günaydın"
} değilse {
    mesaj = "İyi günler"
}
yazdır(mesaj)
```

### Değeri olmayan değişken

Değişken değersiz tanımlanabilir, ama değer verilmeden kullanılamaz:

```kolay
ata x: Tam
yazdır(x)     // ✗ HATA: 'x' henüz belirsiz. Kullanmadan önce bir değer atayın.
```

Birçok dilde bu durumda program rastgele bir değerle ya da "undefined" ile çalışmaya devam eder. Kolay bunu daha program çalışmadan yakalar.

### Alıştırmalar

1. `genişlik` ve `yükseklik` değişkenleri tanımlayıp dikdörtgenin alanını ve çevresini yazdırın.
2. `para = 100` ile başlayın. Önce 30 harcayın (`-=`), sonra paranızı ikiye katlayın (`*=`), sonucu yazdırın.
3. Aşağıdaki program neden hata verir? Düzeltin.
   ```
   ata sabit vergi = 18
   vergi = 20
   ```

<details>
<summary>Çözümler</summary>

```kolay
// 1
ata genişlik = 8
ata yükseklik = 5
yazdır(f"Alan: {genişlik * yükseklik}, Çevre: {2 * (genişlik + yükseklik)}")

// 2
ata para = 100
para -= 30
para *= 2
yazdır(para)   // 140
```

3: `vergi` sabit tanımlandığı için değiştirilemez. Değişecekse `ata vergi = 18` yazılmalı.

</details>

---

## 5. Sayılar

### Tam sayılar

Tipi yazmazsanız tam sayılar `Tam(64)` olur: 64 bitlik, yaklaşık ±9 milyon trilyona kadar sayı tutan bir tip. Günlük işlerin neredeyse tamamı için yeterlidir.

Dört işlem ve kalan:

```kolay
ata a = 17
ata b = 5
yazdır(a + b)   // 22
yazdır(a - b)   // 12
yazdır(a * b)   // 85
yazdır(a / b)   // 3   (tam bölme: küsurat atılır)
yazdır(a % b)   // 2   (kalan)
```

İki tam sayının bölümü yine tam sayıdır, küsurat atılır. Kalan işlemi `%` çok kullanışlıdır: bir sayının çift olup olmadığı `sayı % 2 == 0` ile anlaşılır.

Negatif sayılarda bölüm **sıfıra doğru** kesilir, kalanın işareti bölünenin işaretidir:

```kolay
yazdır(-7 / 2, -7 % 2, 7 / -2, 7 % -2)
```

```
-3 -1 -3 1
```

(Python'da `-7 // 2` sonucu `-4`'tür; Kolay, C ve Java gibi `-3` verir.)

Uzun sayılar okunabilsin diye `_` ile ayrılabilir, onaltılık (hex) sayılar `0x` ile yazılır:

```kolay
ata nüfus = 85_000_000
ata renk = 0xFF8800
yazdır(nüfus, renk)
```

```
85000000 16746496
```

### Sıfıra bölme

Bir tam sayıyı sıfıra bölmek matematikte tanımsızdır. Kolay bu durumda programı Türkçe bir mesajla durdurur:

```kolay
ata bölen = 0
yazdır(10 / bölen)    // ✗ ÇALIŞMA HATASI: Sıfıra bölme hatası.
```

### Taşma ve boyutlu tam sayılar

Bilgisayarda her sayı belli bir yer kaplar. Parantez içindeki sayı, değerin kaç **bit** tutacağını söyler:

| Tip | Aralık |
|---|---|
| `Tam(8)` | -128 .. 127 |
| `Tam(16)` | -32.768 .. 32.767 |
| `Tam(32)` | yaklaşık ±2,1 milyar |
| `Tam(64)` | yaklaşık ±9,2 × 10¹⁸ (varsayılan) |
| `Tam(128)` | yaklaşık ±1,7 × 10³⁸ |

Sığmayan bir değer verirseniz program çalışmadan hata alırsınız:

```kolay
ata x: Tam(8) = 300    // ✗ HATA: 300, Tam(8) sınırlarını aşıyor (-128 .. 127).
```

İşlem sırasında sınır aşılırsa (**taşma**) program durur. Birçok dil bu durumda sessizce yanlış bir sayıyla devam eder; Kolay etmez:

```kolay
ata b: Tam(8) = 120
b = b + 10    // ✗ ÇALIŞMA HATASI: Tam(8) taştı. 120 + 10 = 130, ama Tam(8) en fazla 127 tutabilir.
```

Neden küçük tip kullanılsın? Bellek ve hız: bir milyon elemanlı `Tam(8)` listesi, `Tam(64)` listesinin sekizde biri kadar yer kaplar. Ama öğrenirken tipi yazmamak, yani `Tam(64)` kullanmak yeterlidir.

### İşaret: sadece pozitif ya da sadece negatif

Boyutun yanına `+` yazılırsa sayı sadece pozitif olabilir, aralık da iki katına çıkar:

```kolay
ata yaş: Tam(8, +) = 200       // 0 .. 255
ata sıcaklık: Tam(8) = -20     // -128 .. 127
ata borç: Tam(8, -) = -200     // -255 .. 0
yazdır(yaş, sıcaklık, borç)
```

Yaş, adet, sıra numarası gibi negatif olamayacak değerlerde `+` kullanmak hatayı erken yakalar.

### Esnek Tam: hiç taşmayan sayılar

Parantezsiz `Tam` yazarsanız sayı **esnek** olur: gerektikçe büyür, hiç taşmaz:

```kolay
ata büyük: Tam = 1
100 kere yap { büyük *= 2 }
yazdır(f"2 üzeri 100 = {büyük}")

ata faktöriyel: Tam = 1
1..=30 içindeki her i için yap { faktöriyel *= i }
yazdır(f"30! = {faktöriyel}")
```

```
2 üzeri 100 = 1267650600228229401496703205376
30! = 265252859812191058636308480000000
```

Esnek `Tam` biraz daha yavaştır; bu yüzden varsayılan `Tam(64)`'tür.

> **Dikkat: `Tam` ile `Tam(64)` farklı tiplerdir.** Tip yazmazsanız sayı `Tam(64)` olur (`ata toplam = 0`). `Tam(64)` değer esnek `Tam` değişkene otomatik geçer, ama tersi geçmez, çünkü esnek sayı 64 bite sığmayabilir. Esnek `Tam` değerleri toplayacağınız bir değişkeni `ata toplam: Tam = 0` diye tanımlayın. Aralık sınırları (`0..n`) `Tam(64)` olmalıdır.

### Küçükten büyüğe, büyükten küçüğe

Küçük tipten büyük tipe geçiş otomatiktir, çünkü veri kaybolmaz. Tersi açıkça yazılmalıdır:

```kolay
ata küçük: Tam(8) = 5
ata büyük: Tam(64) = küçük          // ✓ otomatik
ata geri: Tam(8) = büyük olarak Tam(8)   // ✓ "bilerek çeviriyorum"
yazdır(geri)
```

`olarak` "bunu şu tipe çevir" demektir. Değer sığmıyorsa program çalışırken hata verir.

### Ondalık sayılar

Noktalı sayılar `Ondalık` tipindedir. Türkçede virgül kullansak da programlamada ondalık ayırıcı **noktadır**:

```kolay
ata pi = 3.14159
ata r = 2.5
yazdır(pi * r * r)
yazdır(7.0 / 2)     // 3.5
yazdır(2 * 1.5)     // 3.0: Tam ile Ondalık birlikte kullanılınca sonuç Ondalık
```

Ondalık sayılar her zaman tam doğru değildir. Bilgisayar sayıları ikilik sistemde tuttuğu için `0.1` gibi sayılar küçük bir hatayla saklanır:

```kolay
yazdır(0.1 + 0.2)
```

```
0.30000000000000004
```

Bu Kolay'a özgü değildir; Python, Java, C ve JavaScript'te de aynı sonuç çıkar. Para gibi kesin hesap gereken yerlerde kuruş cinsinden tam sayı kullanmak daha güvenlidir.

Ondalık tipin de boyutları vardır: `Ondalık(16)`, `Ondalık(32)`, `Ondalık(64)` (varsayılan). Küçük boyutlar daha az yer kaplar ama hassasiyeti düşüktür.

### Tanımsız sayı

Ondalık bir sayıyı sıfıra bölmek programı durdurmaz, sonuç `TanımsızSayı` olur:

```kolay
ata sonuç = 1.0 / 0.0
yazdır(sonuç)
```

```
TanımsızSayı
```

### Çevirmeler

```kolay
yazdır(3.99 olarak Tam(64))   // 3: küsurat atılır
yazdır(3.7.tama())            // 3
yazdır(10 olarak Ondalık)     // 10.0
yazdır(42.yazıya() + "!")     // "42!" (sayıyı yazıya çevirme)
```

### Üs: **

`a ** b`, "a üzeri b" demektir:

```kolay
yazdır(2 ** 10, 3 ** 4, 10 ** 0)
yazdır(2.0 ** 0.5, 9.0 ** 0.5, 2.0 ** -1)
yazdır(-2 ** 2, (-2) ** 2, 2 ** 3 ** 2)
```

```
1024 81 1
1.4142135623730951 3.0 0.5
-4 4 512
```

- Tam sayılarda sonuç tam sayıdır ve taşma kontrol edilir: `2 ** 63`, `Tam(64)`'e sığmaz.
- Tam sayının üssü negatif olamaz (`2 ** -1` hata verir, çünkü sonuç tam sayı değildir). Ondalık sonuç için `2.0 ** -1` yazın.
- Matematikteki gibi `-2 ** 2`, `-(2 ** 2)` yani `-4`'tür; üs sağdan sola hesaplanır: `2 ** 3 ** 2` = `2 ** 9`.
- Esnek `Tam` ile büyük kuvvetler de hesaplanabilir: `ata k: Tam = 3` için `k ** 100`.

```kolay
ata k: Tam = 3
yazdır(k ** 50)
yazdır(2 ** 63)    // ✗ ÇALIŞMA HATASI
```

```
717897987691852588770249
ÇALIŞMA HATASI (deneme.kolay, satır 3): Tam(64) taştı: 2 ** 63 sonucu Tam(64) sınırlarını aşıyor (-9.223.372.036.854.775.808 .. 9.223.372.036.854.775.807).
  3 | yazdır(2 ** 63)    // ✗ ÇALIŞMA HATASI
```

### Bit işlemleri

Bilgisayar sayıları ikilik (0 ve 1) olarak tutar. Bit işlemleri sayının bitleri üzerinde çalışır. Günlük programlarda az kullanılırlar; renkler, bayraklar, şifreleme ve donanımla uğraşırken işe yararlar:

| İşlem | Adı | Örnek | Sonuç |
|---|---|---|---|
| `a & b` | ve (iki bitte de 1 ise 1) | `12 & 10` (ikilik: 1100 ve 1010) | `8` (1000) |
| `a \| b` | veya (bitlerden biri 1 ise 1) | `12 \| 10` | `14` (1110) |
| `a ^ b` | özel veya, xor (bitler farklıysa 1) | `12 ^ 10` | `6` (0110) |
| `~a` | bitleri ters çevir | `~0` | `-1` |
| `a << n` | sola kaydır (2ⁿ ile çarpmak gibi) | `1 << 10` | `1024` |
| `a >> n` | sağa kaydır (2ⁿ'e bölmek gibi, aşağı yuvarlar) | `-5 >> 1` | `-3` |

```kolay
ata renk = 0xFF8800
ata kırmızı = (renk >> 16) & 0xFF
ata yeşil = (renk >> 8) & 0xFF
ata mavi = renk & 0xFF
yazdır(kırmızı, yeşil, mavi)

ata bayraklar = 0
bayraklar |= 1 << 2      // 2. biti aç
bayraklar |= 1 << 0      // 0. biti aç
yazdır(bayraklar, bayraklar & (1 << 2) != 0, bayraklar & (1 << 1) != 0)
```

```
255 136 0
5 doğru yanlış
```

- Bit işlemleri yalnızca tam sayılarla yapılır; ondalık sayıyla yapılırsa derleme hatası verir.
- Eksi sayılar Python'daki gibi davranır (ikiye tümleyen gösterim): `-5 & 3` = `3`.
- Sola kaydırma da taşma kontrol eder: `Tam(8, +)` tipinde `200 << 1` taşar.
- Önceliği: `|` en zayıf, sonra `^`, sonra `&`, sonra `<< >>`, sonra `+ -`. Şüphedeyseniz parantez kullanın.

> **Dikkat:** Birçok kişi üs için `^` yazar. Kolay'da (ve Python, C, Java'da) `^` üs değil, **xor**'dur. Kolay, `x ^ 2` gibi üs sanılabilecek bir yazımda uyarır:

```kolay
ata x = 5
yazdır(x ^ 2)
```

```
Uyarı (deneme.kolay, satır 2): '^' özel veya (xor) işlemidir; üs için '**' yazın.
  2 | yazdır(x ^ 2)
    |        ^^^^^
7
```

### Alıştırmalar

1. 1234 saniyenin kaç dakika ve kaç saniye olduğunu `/` ve `%` ile hesaplayın.
2. Bir sayının tek mi çift mi olduğunu yazdıran bir satır yazın (ipucu: `%`).
3. Esnek `Tam` ile 2'nin 200. kuvvetini hesaplayın (önce döngüyle, sonra `**` ile).
4. Celsius'u Fahrenheit'a çevirin: `F = C * 9 / 5 + 32`. `C = 36.6` için sonuç ne?
5. Bir sayının 2'nin kuvveti olup olmadığını `n & (n - 1) == 0` ile bulun. 64 ve 96 için deneyin.

<details>
<summary>Çözümler</summary>

```kolay
// 1
ata toplam = 1234
yazdır(f"{toplam / 60} dakika {toplam % 60} saniye")   // 20 dakika 34 saniye

// 2
ata sayı = 17
yazdır(sayı % 2 == 0)   // yanlış: tek sayı

// 3
ata k: Tam = 1
200 kere yap { k *= 2 }
yazdır(k)
ata iki: Tam = 2
yazdır(iki ** 200)   // aynı sayı

// 4
ata c = 36.6
yazdır(c * 9 / 5 + 32)   // 97.88

// 5
ata n = 64
yazdır(n & (n - 1) == 0)   // doğru
n = 96
yazdır(n & (n - 1) == 0)   // yanlış
```

</details>

---

## 6. Yazılar

`Yazı` tipi metin tutar. Yazılar `+` ile birleştirilir:

```kolay
ata ad = "Ada"
ata soyad = "Lovelace"
ata tam_ad = ad + " " + soyad
yazdır(tam_ad)
```

Yazıyla sayı `+` ile toplanamaz. Kolay bunun yerine f-yazı önerir:

```kolay
ata yaş = 36
yazdır("Yaş: " + yaş)    // ✗ HATA: Yazı ile Tam(64) toplanamaz. f-yazı kullanın
```

```kolay
ata yaş = 36
yazdır(f"Yaş: {yaş}")    // ✓
```

### Yazı metotları

Metot, bir değere ait fonksiyondur: değerin arkasına nokta konup yazılır.

```kolay
ata s = "Merhaba Dünya"
yazdır(s.uzunluk())          // 13: karakter sayısı
yazdır(s.büyük_harf())       // MERHABA DÜNYA
yazdır(s.küçük_harf())       // merhaba dünya
yazdır(s.karakter(0))        // M: ilk karakter (sayma 0'dan başlar)
yazdır(s.içerir("Dün"))      // doğru
yazdır(s.böl(" "))           // ["Merhaba", "Dünya"]
yazdır("  boşluklu  ".kırp()) // "boşluklu": baştaki ve sondaki boşluklar silinir
yazdır(s.tersi())            // aynüD abahreM: tersten
```

Yazının içinde aramak ve değiştirmek için:

```kolay
ata cümle = "Ali topu at, Ali koş"
yazdır(cümle.değiştir("Ali", "Ayşe"))        // Ayşe topu at, Ayşe koş
yazdır(cümle.başlar("Ali"), cümle.biter("koş")) // doğru doğru
yazdır(cümle.bul("topu"), cümle.bul("kedi")) // 4 Hiç: kaçıncı karakterde başlıyor, yoksa Hiç
yazdır(cümle.say("Ali"))                     // 2: kaç kez geçiyor
yazdır("-".tekrar(10))                       // ----------
```

Yazı metotlarının tamamı:

| Metot | Ne yapar | Örnek | Sonuç |
|---|---|---|---|
| `uzunluk()` | Karakter sayısı (Türkçe harfler tek karakter sayılır) | `"çiçek".uzunluk()` | `5` |
| `büyük_harf()` | Büyük harfe çevirir (Türkçe kurallarla) | `"ılık".büyük_harf()` | `"ILIK"` |
| `küçük_harf()` | Küçük harfe çevirir (Türkçe kurallarla) | `"İZMİR".küçük_harf()` | `"izmir"` |
| `karakter(i)` | i. karakter; eksi sayı sondan sayar | `"Kolay".karakter(-1)` | `"y"` |
| `içerir(parça)` | Parça yazının içinde var mı? | `"Ankara".içerir("kar")` | `doğru` |
| `böl(ayraç)` | Ayraçtan bölüp liste yapar; ayraç `""` ise harflere böler | `"a-b".böl("-")` | `["a", "b"]` |
| `kırp()` | Baştaki ve sondaki boşlukları siler | `"  x ".kırp()` | `"x"` |
| `değiştir(eski, yeni)` | Bütün `eski` parçalarını `yeni` ile değiştirir | `"a-b-c".değiştir("-", "+")` | `"a+b+c"` |
| `başlar(parça)` | Bu parçayla mı başlıyor? | `"Kolay".başlar("Ko")` | `doğru` |
| `biter(parça)` | Bu parçayla mı bitiyor? | `"dosya.kolay".biter(".kolay")` | `doğru` |
| `bul(parça)` | Parçanın başladığı karakter sırası; yoksa `Hiç` (`Tam(64) veya Hiç`) | `"merhaba".bul("ha")` | `3` |
| `say(parça)` | Parça kaç kez geçiyor? | `"ananas".say("a")` | `3` |
| `tekrar(n)` | Yazıyı n kez art arda yazar | `"ab".tekrar(3)` | `"ababab"` |
| `tersi()` | Tersten yazılmış yeni yazı | `"kayak".tersi()` | `"kayak"` |
| `tama()` | Tam sayıya çevirir (aşağıya bakın) | `"42".tama()` | `42` |
| `ondalığa()` | Ondalık sayıya çevirir; virgül de kabul edilir | `"3,5".ondalığa()` | `3.5` |
| `yazıya()` | Her tipte vardır: değeri yazıya çevirir | `42.yazıya()` | `"42"` |

Yazılar **değişmez**: bu metotların hiçbiri yazının kendisini değiştirmez, her biri yeni bir değer verir. `s.büyük_harf()` yazdıktan sonra `s` hâlâ eski halindedir; değiştirmek istiyorsanız `s = s.büyük_harf()` yazın. Sonucu kullanmayı unutursanız Kolay uyarır: `Uyarı: 'değiştir' yazıyı değiştirmez, yeni bir yazı verir; burada sonuç kullanılmıyor. Şöyle yazın: s = s.değiştir(...)`

### Yazının parçaları: dilim

Köşeli parantez içine bir aralık yazılırsa yazının o parçası alınır. Sayılar **karakter** sırasıdır ve 0'dan başlar:

```kolay
ata s = "merhaba"
yazdır(s[0..3])      // 0, 1, 2. karakterler
yazdır(s[3..])       // 3. karakterden sona kadar
yazdır(s[..-1])      // sonuncusu hariç hepsi
yazdır(s[-3..])      // son üç karakter
yazdır(s[-1..=0])    // tersten: sondan başa
yazdır("ağaçlık"[1..=3])
```

```
mer
haba
merhab
aba
abahrem
ğaç
```

Dilimin kuralları listelerdekiyle aynıdır, ayrıntıları [Listeler](#12-listeler) bölümünde. Tek bir karakter için köşeli parantez değil `karakter(i)` kullanılır:

```kolay
yazdır("abc"[1])    // ✗ HATA
```

```
HATA (deneme.kolay, satır 1): Yazıda tek karakter için karakter kullanın: yazı.karakter(0). Bir parça için dilim yazın: yazı[0..3]
  1 | yazdır("abc"[1])    // ✗ HATA
    |        ^^^^^^^^
```

### Yazıyı sayıya çevirmek: tama() ve ondalığa()

`"42".tama()` yazıyı tam sayıya, `"2.5".ondalığa()` ondalık sayıya çevirir. Ama bir yazı her zaman sayıya çevrilemez: `"kırk iki".tama()` ne olmalı? Bu yüzden `tama()`'nın sonucu **ya bir sayı ya da bir hatadır**:

```kolay
yazdır("42".tama())
yazdır("kırk iki".tama())
```

```
42
Hata.Dönüşüm(mesaj: "\"kırk iki\" bir tam sayıya çevrilemez.")
```

Sonuç bu yüzden doğrudan bir sayı gibi kullanılamaz. Kolay, kontrol edilmeden kullanılmasına izin vermez:

```kolay
yazdır("42".tama() + 1)    // ✗ HATA
```

```
HATA (deneme.kolay, satır 1): 'tama()' hata verebilir (Tam(64) veya Hata.Dönüşüm). Bir 'dene { } yakala' bloğu içinde kullanın ya da eşleştir ile kontrol edin.
  1 | yazdır("42".tama() + 1)    // ✗ HATA
    |        ^^^^^^^^^^^
```

Bunu ele almanın en kısa yolu `dene` bloğudur; blok içinde sonuç gerçekten bir sayıdır, çevrilemezse `yakala` dalı çalışır:

```kolay
dene {
    yazdır("42".tama() + 1)
    yazdır("2.5".ondalığa() * 2)
} yakala h {
    yazdır("sayıya çevrilemedi:", h)
}
```

```
43
5.0
```

`dene` ve `yakala`'yı [Hata yönetimi](#21-hata-yönetimi-dene-yakala-fırlat) bölümünde ayrıntılı göreceğiz; o zamana kadar bu kalıbı kullanmanız yeterli. `tama()` baştaki ve sondaki boşlukları yok sayar ve başta `+` ya da `-` kabul eder; `ondalığa()` hem `2.5` hem `2,5` yazımını kabul eder.

Büyük-küçük harf dönüşümü **Türkçe kurallarla** yapılır. Birçok dilde `"istanbul".upper()` yanlış olarak `ISTANBUL` verir:

```kolay
yazdır("istanbul".büyük_harf())   // İSTANBUL
yazdır("IŞIK".küçük_harf())       // ışık
```

### Karakter karakter dolaşmak

```kolay
"Kolay" içindeki her harf için yap {
    yazdır(harf)
}
```

(Döngüleri 10. bölümde ayrıntılı göreceğiz.) Her `harf` bir `Yazı`'dır; `ğ`, `ş`, `ü` gibi Türkçe harfler tek bir karakter olarak gelir.

### Karşılaştırma ve Türkçe sıralama

Yazılar `==`, `!=`, `<`, `>` ile karşılaştırılır. Sıralama Türk alfabesine göredir (c < ç < d, ı < i, o < ö):

```kolay
yazdır("çam" < "dam")     // doğru
ata isimler = ["Zeynep", "Çağla", "Can", "İrem", "Ilgaz", "Ömer", "Oya"]
isimler.sırala()
yazdır(isimler)
```

```
doğru
["Can", "Çağla", "Ilgaz", "İrem", "Oya", "Ömer", "Zeynep"]
```

### f-yazı biçimleri: değerleri düzgün yazdırmak

f-yazıda değerden sonra `:` koyup bir **biçim** yazarak değerin nasıl görüneceğini seçebilirsiniz:

```kolay
ata pi = 3.14159
yazdır(f"{pi:.2}")                // 3.14: virgülden sonra 2 basamak
yazdır(f"[{"Ali":<8}]")            // [Ali     ]: 8 karakterlik yere sola yasla
yazdır(f"[{"Ali":>8}]")            // [     Ali]: sağa yasla
yazdır(f"[{"Ali":^8}]")            // [  Ali   ]: ortala
yazdır(f"[{"Ali":*^8}]")           // [**Ali***]: boşluk yerine * ile doldur
yazdır(f"{7:03}")                 // 007: başını sıfırla doldur
yazdır(f"{1234567:,}")            // 1.234.567: basamakları grupla
yazdır(f"{1234567.891:,.2}")      // 1.234.567,89: Türk usulü
```

| Biçim | Anlamı | Örnek | Sonuç |
|---|---|---|---|
| `.2` | Virgülden sonra 2 basamak (yuvarlar) | `{3.14159:.2}` | `3.14` |
| `<10` `>10` `^10` | 10 karakterlik yere sola / sağa / ortaya yasla | `{"Ali":>6}` | `   Ali` |
| `*^10` | Boşluk yerine `*` ile doldur (dolgu, hizadan önce yazılır) | `{"Ali":-^7}` | `--Ali--` |
| `6` | Sadece genişlik: sayılar sağa, yazılar sola yaslanır | `{42:6}` | `    42` |
| `06` | Başını sıfırla doldur | `{42:06}` | `000042` |
| `,` | Basamakları noktayla grupla; ondalık ayırıcı virgül olur | `{1234.5:,.2}` | `1.234,50` |

Biçimler birlikte yazılabilir: `{fiyat:>10.2}` hem 10 karaktere sağa yaslar hem 2 basamak gösterir. Bu, sütunları hizalı tablolar yazmayı kolaylaştırır:

```kolay
ata ürünler = ["Ekmek", "Süt", "Peynir"]
ata fiyatlar = [12.5, 32.0, 189.75]
yazdır(f"{"Ürün":<10}{"Fiyat":>10}")
yazdır("-".tekrar(20))
0..3 içindeki her i için yap {
    yazdır(f"{ürünler[i]:<10}{fiyatlar[i]:>10.2}")
}
yazdır("-".tekrar(20))
yazdır(f"{"Toplam":<10}{fiyatlar.toplam():>10.2}")
```

```
Ürün           Fiyat
--------------------
Ekmek          12.50
Süt            32.00
Peynir        189.75
--------------------
Toplam        234.25
```

- Basamak (`.2`), gruplama (`,`) ve sıfırla doldurma (`0`) sadece sayılarda kullanılabilir.
- Genişlik karakter sayısıdır; Türkçe harfler tek karakter sayılır. Değer genişlikten uzunsa kesilmez, olduğu gibi yazılır.
- Python'u biliyorsanız: Python'da 2 basamak için `.2f` yazılır, Kolay'da `.2` yeterli (`f` yazılırsa Kolay uyarır).

### Alıştırmalar

1. `"ankara"` yazısını büyük harfe çevirip uzunluğunu yazdırın.
2. `"elma,armut,kiraz"` yazısını virgülden bölüp kaç meyve olduğunu yazdırın.
3. Bir kelimedeki `a` harflerini sayın (ipucu: harf harf dolaşın, bir sayaç tutun; koşullar 8. bölümde, döndükten sonra deneyin).

<details>
<summary>Çözümler</summary>

```kolay
// 1
ata ş = "ankara".büyük_harf()
yazdır(ş, ş.uzunluk())

// 2
ata meyveler = "elma,armut,kiraz".böl(",")
yazdır(meyveler.uzunluk())

// 3
ata sayaç = 0
"Ankara Karabük" içindeki her h için yap {
    eğer h == "a" { sayaç += 1 }
}
yazdır(sayaç)   // 5
```

</details>

---

## 7. Mantık ve karşılaştırma

`Mantık` tipinin yalnızca iki değeri vardır: `doğru` ve `yanlış`. Karşılaştırmaların sonucu her zaman `Mantık`'tır:

| İşaret | Anlamı | Örnek | Sonuç |
|---|---|---|---|
| `==` | eşit mi | `3 == 3` | `doğru` |
| `!=` | eşit değil mi | `3 != 3` | `yanlış` |
| `<` | küçük mü | `2 < 5` | `doğru` |
| `>` | büyük mü | `2 > 5` | `yanlış` |
| `<=` | küçük ya da eşit mi | `5 <= 5` | `doğru` |
| `>=` | büyük ya da eşit mi | `4 >= 5` | `yanlış` |

Dikkat: `=` atamadır ("kutuya koy"), `==` karşılaştırmadır ("eşit mi?").

### ve, veya, değil

| Yazım | Anlamı |
|---|---|
| `a ve b` | ikisi de doğruysa doğru |
| `a veya b` | en az biri doğruysa doğru |
| `değil a` | doğruyu yanlış, yanlışı doğru yapar |

```kolay
ata yaş = 20
ata ehliyet = doğru
yazdır(yaş >= 18 ve ehliyet)       // doğru: araba kullanabilir
yazdır(yaş < 13 veya yaş > 65)     // yanlış: indirim yok
yazdır(değil ehliyet)              // yanlış
```

Diğer dillerden gelenler için `&&`, `||` ve `!` de çalışır.

### Alıştırma

Bir yılın artık yıl olup olmadığını hesaplayan ifadeyi yazın. Kural: 4'e bölünüyorsa artık yıldır; ama 100'e bölünüp 400'e bölünmüyorsa değildir. 2024, 1900 ve 2000 için deneyin.

<details>
<summary>Çözüm</summary>

```kolay
ata yıl = 1900
yazdır(yıl % 4 == 0 ve (yıl % 100 != 0 veya yıl % 400 == 0))   // yanlış
```

</details>

---

## 8. Koşullar: eğer

Program bazen bir şeyi ancak bir koşul sağlanırsa yapmalıdır:

```kolay
ata sıcaklık = 32
eğer sıcaklık > 30 {
    yazdır("Çok sıcak, su için!")
}
```

Süslü parantez `{ }` içindeki satırlara **blok** denir. Blok yalnızca koşul doğruysa çalışır. Blok içini 4 boşluk içeri yazmak okumayı kolaylaştırır (zorunlu değildir, ama herkes böyle yazar).

### değilse

```kolay
ata not = 45
eğer not >= 50 {
    yazdır("Geçti")
} değilse {
    yazdır("Kaldı")
}
```

### değilse eğer: birden fazla durum

```kolay
ata puan = 78
eğer puan >= 85 {
    yazdır("Pekiyi")
} değilse eğer puan >= 70 {
    yazdır("İyi")
} değilse eğer puan >= 50 {
    yazdır("Orta")
} değilse {
    yazdır("Geçmez")
}
```

Koşullar yukarıdan aşağıya denenir; **ilk doğru olanın** bloğu çalışır, gerisine bakılmaz. 78 hem `>= 70` hem `>= 50` koşulunu sağlar, ama yalnızca "İyi" yazılır.

Koşulu paranteze almak serbesttir: `eğer (puan > 50) { }` de geçerlidir. Kısa bloklar tek satırda da yazılabilir:

```kolay
ata x = -3
eğer x < 0 { x = -x }
yazdır(x)
```

### İç içe koşullar

```kolay
ata yaş = 16
ata veli_izni = doğru
eğer yaş >= 18 {
    yazdır("Girebilirsiniz")
} değilse {
    eğer veli_izni {
        yazdır("Veli izniyle girebilirsiniz")
    } değilse {
        yazdır("Giremezsiniz")
    }
}
```

### Alıştırmalar

1. Bir sayının pozitif, negatif ya da sıfır olduğunu yazdırın.
2. Üç sayıdan en büyüğünü bulun.
3. Vücut kitle indeksi: `kilo / (boy * boy)`. 18,5'ten küçükse "zayıf", 25'ten küçükse "normal", 30'dan küçükse "kilolu", değilse "obez" yazdırın.

<details>
<summary>Çözümler</summary>

```kolay
// 1
ata s = -4
eğer s > 0 { yazdır("pozitif") } değilse eğer s < 0 { yazdır("negatif") } değilse { yazdır("sıfır") }

// 2
ata a = 7
ata b = 12
ata c = 9
ata en_büyük = a
eğer b > en_büyük { en_büyük = b }
eğer c > en_büyük { en_büyük = c }
yazdır(en_büyük)

// 3
ata kilo = 70.0
ata boy = 1.75
ata vki = kilo / (boy * boy)
eğer vki < 18.5 {
    yazdır("zayıf")
} değilse eğer vki < 25.0 {
    yazdır("normal")
} değilse eğer vki < 30.0 {
    yazdır("kilolu")
} değilse {
    yazdır("obez")
}
```

</details>

---

## 9. Kullanıcıdan bilgi almak: oku

`oku` klavyeden bir satır okur. Parantez içine yazılan soru önce ekrana yazılır:

```kolay
ata ad = oku("Adınız nedir? ")
yazdır(f"Merhaba {ad}!")
```

```
Adınız nedir? Zeynep
Merhaba Zeynep!
```

`oku` her zaman `Yazı` döndürür: kullanıcı `25` yazsa bile sonuç `"25"` yazısıdır. Sayı istiyorsanız `tama()` ya da `ondalığa()` ile çevirmeniz gerekir. Kullanıcı sayı yerine `yirmi beş` yazabileceği için çevirme başarısız olabilir; bu yüzden [Yazılar](#6-yazılar) bölümünde gördüğümüz `dene` kalıbı kullanılır:

```kolay
dene {
    ata yaş = oku("Kaç yaşındasınız? ").tama()
    yazdır(f"10 yıl sonra {yaş + 10} yaşında olacaksınız.")
} yakala h: Hata.Dönüşüm {
    yazdır("Yaşınızı rakamla yazmalısınız.")
}
```

Kullanıcı sayı yazarsa:

```
Kaç yaşındasınız? 25
10 yıl sonra 35 yaşında olacaksınız.
```

Yazmazsa program çökmez:

```
Kaç yaşındasınız? yirmi beş
Yaşınızı rakamla yazmalısınız.
```

`yakala h: Hata.Dönüşüm`, "çevirme hatası olursa" demektir. Bir sayı yazılana kadar tekrar sormak için bu kalıp bir döngünün içine konur; sayı alınınca `dur` ile döngüden çıkılır:

```kolay
doğru sürece yap {
    dene {
        ata yaş = oku("Kaç yaşındasınız? ").tama()
        yazdır(f"Teşekkürler, {yaş} yaşındasınız.")
        dur
    } yakala h: Hata.Dönüşüm {
        yazdır("Bu bir sayı değil, tekrar deneyin.")
    }
}
```

```
Kaç yaşındasınız? on sekiz
Bu bir sayı değil, tekrar deneyin.
Kaç yaşındasınız? 18
Teşekkürler, 18 yaşındasınız.
```

Girdi bittiğinde (örneğin dosyadan okunurken dosyanın sonuna gelindiğinde) `oku` boş yazı `""` döndürür.

### Alıştırma

Kullanıcıdan iki sayı alıp toplamını, farkını, çarpımını ve bölümünü yazdıran bir hesap makinesi yazın.

<details>
<summary>Çözüm</summary>

```kolay
dene {
    ata a = oku("Birinci sayı: ").ondalığa()
    ata b = oku("İkinci sayı: ").ondalığa()
    yazdır(f"{a} + {b} = {a + b}")
    yazdır(f"{a} - {b} = {a - b}")
    yazdır(f"{a} * {b} = {a * b}")
    eğer b == 0.0 {
        yazdır("Sıfıra bölünemez")
    } değilse {
        yazdır(f"{a} / {b} = {a / b}")
    }
} yakala h: Hata.Dönüşüm {
    yazdır("Lütfen sayı yazın:", h.mesaj)
}
```

</details>

---

## 10. Döngüler

Döngü, bir işi tekrar tekrar yapar. Kolay'da döngüler Türkçe cümle sırasıyla yazılır: önce **ne üzerinde** döneceği, sonra `yap`.

### N kere yap

En basit döngü:

```kolay
3 kere yap {
    yazdır("Merhaba")
}
```

```
Merhaba
Merhaba
Merhaba
```

### Aralık üzerinde: içindeki her ... için

```kolay
1..=5 içindeki her i için yap {
    yazdır(f"{i} x 7 = {i * 7}")
}
```

```
1 x 7 = 7
2 x 7 = 14
3 x 7 = 21
4 x 7 = 28
5 x 7 = 35
```

İki aralık yazımı var:

| Yazım | Sayılar |
|---|---|
| `0..5` | 0, 1, 2, 3, 4 (5 hariç) |
| `0..=5` | 0, 1, 2, 3, 4, 5 (5 dahil) |

Başlangıç bitişten büyükse aralık **geri sayar**. Bitiş yine `..` ile hariç, `..=` ile dahildir:

```kolay
ata t = ""
5..0 içindeki her i için yap { t += f"{i} " }
yazdır(t)
t = ""
5..=0 içindeki her i için yap { t += f"{i} " }
yazdır(t)
```

```
5 4 3 2 1 
5 4 3 2 1 0 
```

Adımlı saymak için `adım`. Adım her n. sayıyı alır; **eksi** adım listenin sonundan başlar:

```kolay
yazdır((0..=20).adım(5))
yazdır((10..0).adım(2))
yazdır((0..10).adım(-3))
```

```
[0, 5, 10, 15, 20]
[10, 8, 6, 4, 2]
[9, 6, 3, 0]
```

### Aralık bir listedir

`1..=5` aslında `[1, 2, 3, 4, 5]` listesidir. Bir değişkene konabilir, yazdırılabilir, listelerin bütün metotları kullanılabilir:

```kolay
ata r = 1..=5
yazdır(r, r.uzunluk(), r.içerir(3), r.toplam())
r.ekle(6)
yazdır(r)
yazdır((1..=10).filtre(her x için x % 3 == 0))
```

```
[1, 2, 3, 4, 5] 5 doğru 15
[1, 2, 3, 4, 5, 6]
[3, 6, 9]
```

(Son satırdaki `filtre`'yi [Lambda](#14-lambda-fonksiyonları-değer-olarak-kullanmak) bölümünde göreceğiz.)

Bir döngüde (`0..1000000 içindeki her i için yap`) liste aslında **oluşturulmaz**, sayılar teker teker sayılır; yani çok büyük aralıklarda bile döngü hızlıdır ve bellek harcamaz. Liste ancak aralık bir değer olarak kullanılınca (değişkene atanınca, yazdırılınca) oluşur. Bu yüzden çok büyük bir aralığı liste yapmak hata verir:

```kolay
ata say = 0
0..9_000_000_000_000_000 içindeki her i için yap {
    say += 1
    eğer say == 3 { dur }
}
yazdır(say)
ata çok = 0..9_000_000_000_000_000   // ✗ ÇALIŞMA HATASI
```

```
3
ÇALIŞMA HATASI (deneme.kolay, satır 7): Aralık çok büyük (9.000.000.000.000.000 eleman), liste olarak oluşturulamaz. Döngüde doğrudan kullanın: a..b içindeki her i için yap { }
  7 | ata çok = 0..9_000_000_000_000_000   // ✗ ÇALIŞMA HATASI
```

Aralığın iki ucu da yazılmalıdır: `5..` ya da `..5` gibi sonu açık yazım sadece listelerde ve yazılarda **dilim** için kullanılır (bkz. [Listeler](#12-listeler)). Sonsuz bir döngü için `doğru sürece yap` yazılır.

Aynı yazım listelerde ve yazılarda da çalışır:

```kolay
ata meyveler = ["elma", "armut", "kiraz"]
meyveler içindeki her meyve için yap {
    yazdır(f"{meyve} yemeyi severim")
}
```

### Koşul sürece yap

Koşul doğru olduğu sürece dönen döngü:

```kolay
ata para = 100
ata gün = 0
para > 0 sürece yap {
    para -= 15
    gün += 1
}
yazdır(f"Para {gün} günde bitti")
```

```
Para 7 günde bitti
```

Koşul hiç yanlış olmazsa döngü sonsuza kadar döner. Bilerek sonsuz döngü yazmak için `doğru sürece yap` kullanılır, döngüden `dur` ile çıkılır (programı durdurmak için terminalde Ctrl+C).

### dur ve devam

- `dur`: döngüden hemen çık.
- `devam`: bu turun geri kalanını atla, sonraki tura geç.

```kolay
1..=10 içindeki her i için yap {
    eğer i % 2 == 0 { devam }    // çiftleri atla
    eğer i > 7 { dur }           // 7'den sonra bitir
    yazdır(i)
}
```

```
1
3
5
7
```

### C tarzı döngü

C, Java ve JavaScript'ten tanıdık üç bölümlü döngü de var: başlangıç; koşul; her turdan sonra yapılacak iş.

```kolay
(ata i = 10; i > 0; i -= 3) sürece yap {
    yazdır(i)      // 10, 7, 4, 1
}
```

- Parantez ve üç bölüm de zorunludur; başlangıç `ata` ile başlar.
- `i` yalnızca döngünün içinde geçerlidir, döngü bitince kaybolur.
- `devam` yazılınca üçüncü bölüm (`i -= 3`) yine çalışır, sonra koşula bakılır.

### Döngü değişkeninin tipi

`kere` yalnızca tam sayı, `sürece` yalnızca `Mantık` alır. Karıştırılırsa Kolay doğrusunu söyler:

```kolay
5 sürece yap { }    // ✗ HATA
```

```
HATA (deneme.kolay, satır 1): 'sürece' bir Mantık (doğru/yanlış) değeri bekler, ama Tam(64) verildi. Bir işi N kez tekrarlamak için '5 kere yap' yazın.
  1 | 5 sürece yap { }    // ✗ HATA
    | ^
```

### İç içe döngüler

```kolay
1..=3 içindeki her satır için yap {
    ata çizgi = ""
    1..=satır içindeki her j için yap {
        çizgi = çizgi + "*"
    }
    yazdır(çizgi)
}
```

```
*
**
***
```

### Hangi döngüyü ne zaman?

| Durum | Döngü |
|---|---|
| Sayaç lazım değil, sadece N kez | `N kere yap` |
| Sayaç lazım | `a..b içindeki her i için yap` |
| Geriye doğru sayaç | `b..a içindeki her i için yap` |
| Atlayarak sayaç | `(a..b).adım(n) içindeki her i için yap` |
| Listenin her elemanı | `liste içindeki her x için yap` |
| Sözlüğün her girdisi | `sözlük içindeki her (anahtar, değer) için yap` |
| Yazının her harfi | `yazı içindeki her h için yap` |
| Ne zaman biteceği belli değil | `koşul sürece yap` |
| Bilerek sonsuz, içeride `dur` | `doğru sürece yap` |

### Alıştırmalar

1. 1'den 100'e kadar sayıların toplamını bulun.
2. 1'den 30'a kadar sayılar için: 3'e bölünüyorsa "Fıs", 5'e bölünüyorsa "Bıs", ikisine de bölünüyorsa "FısBıs", değilse sayının kendisini yazdırın.
3. Bir sayının basamak sayısını `sürece` ile bulun (sayıyı 10'a böle böle).
4. 2'den 50'ye kadar asal sayıları yazdırın.

<details>
<summary>Çözümler</summary>

```kolay
// 1
ata toplam = 0
1..=100 içindeki her i için yap { toplam += i }
yazdır(toplam)   // 5050

// 2
1..=30 içindeki her i için yap {
    eğer i % 15 == 0 {
        yazdır("FısBıs")
    } değilse eğer i % 3 == 0 {
        yazdır("Fıs")
    } değilse eğer i % 5 == 0 {
        yazdır("Bıs")
    } değilse {
        yazdır(i)
    }
}

// 3
ata sayı = 48213
ata basamak = 0
sayı > 0 sürece yap {
    sayı /= 10
    basamak += 1
}
yazdır(basamak)   // 5

// 4
2..=50 içindeki her n için yap {
    ata asal = doğru
    ata d = 2
    d * d <= n sürece yap {
        eğer n % d == 0 {
            asal = yanlış
            dur
        }
        d += 1
    }
    eğer asal { yazdır(n) }
}
```

</details>

---

## 11. Fonksiyonlar

Fonksiyon, bir işe isim vermektir. Bir kez yazılır, istendiği kadar çağrılır:

```kolay
fonksiyon selamla(ad: Yazı) {
    yazdır(f"Merhaba {ad}, hoş geldin!")
}

selamla("Ali")
selamla("Ayşe")
```

```
Merhaba Ali, hoş geldin!
Merhaba Ayşe, hoş geldin!
```

Parantez içindekiler **parametrelerdir**: fonksiyona verilen bilgiler. Parametrelerin tipi her zaman yazılır.

### Değer döndürmek

Bir sonuç hesaplayan fonksiyon, sonucun tipini `->` ile belirtir ve `döndür` ile geri verir:

```kolay
fonksiyon kare(x: Tam) -> Tam {
    döndür x * x
}

fonksiyon ortalama(a: Ondalık, b: Ondalık) -> Ondalık {
    döndür (a + b) / 2
}

yazdır(kare(7))
yazdır(ortalama(80.0, 95.0))
ata alan = kare(4) + kare(3)
yazdır(alan)
```

```
49
87.5
25
```

`döndür` fonksiyonu hemen bitirir. Değer döndürmeyen bir fonksiyonda tek başına `döndür` yazmak erken çıkış demektir:

```kolay
fonksiyon böl_ve_yaz(a: Tam, b: Tam) {
    eğer b == 0 {
        yazdır("Sıfıra bölünemez")
        döndür
    }
    yazdır(a / b)
}
böl_ve_yaz(10, 2)
böl_ve_yaz(10, 0)
```

Bir değer döndürmesi gereken fonksiyonda bir yol `döndür` olmadan biterse Kolay uyarır:

```kolay
fonksiyon işaret(x: Tam) -> Yazı {
    eğer x > 0 { döndür "pozitif" }
    eğer x < 0 { döndür "negatif" }
}   // ✗ HATA: 'işaret' Yazı döndürmeli, ama fonksiyonun sonuna bir değer döndürmeden ulaşılabiliyor.
```

`x == 0` durumunu unuttuk; Kolay bunu bizim yerimize fark etti.

### Fonksiyonlar fonksiyonları çağırabilir

```kolay
fonksiyon asal_mı(n: Tam) -> Mantık {
    eğer n < 2 { döndür yanlış }
    ata d = 2
    d * d <= n sürece yap {
        eğer n % d == 0 { döndür yanlış }
        d += 1
    }
    döndür doğru
}

fonksiyon asalları_say(sınır: Tam(64)) -> Tam {
    ata sayı = 0
    2..=sınır içindeki her n için yap {
        eğer asal_mı(n) { sayı += 1 }
    }
    döndür sayı
}

yazdır(asalları_say(100))   // 25
```

Fonksiyonların hangi sırada yazıldığı önemli değildir; bir fonksiyon kendisinden sonra yazılan bir fonksiyonu çağırabilir.

### Özyineleme: kendini çağıran fonksiyon

```kolay
fonksiyon faktöriyel(n: Tam) -> Tam {
    eğer n <= 1 { döndür 1 }
    döndür n * faktöriyel(n - 1)
}

fonksiyon fib(n: Tam) -> Tam {
    eğer n < 2 { döndür n }
    döndür fib(n - 1) + fib(n - 2)
}

yazdır(faktöriyel(25))
yazdır(fib(20))
```

Özyinelemenin her zaman bir **durma koşulu** olmalıdır (`n <= 1`). Olmazsa fonksiyon kendini sonsuza kadar çağırır ve Kolay "Çok derin özyineleme" hatasıyla durur.

### Dosyadaki değişkenler

Fonksiyonlar, dosyada kendilerinden **önce** tanımlanmış değişkenleri görebilir ve değiştirebilir:

```kolay
ata sayaç = 0

fonksiyon tıkla() {
    sayaç += 1
}

tıkla()
tıkla()
tıkla()
yazdır(sayaç)   // 3
```

Bunu az kullanın: değer döndüren fonksiyonlar daha kolay anlaşılır ve test edilir.

### Değer semantiği: fonksiyonlar kopya alır

Bir fonksiyona liste ya da nesne verdiğinizde fonksiyon onun **kopyasıyla** çalışır. Fonksiyon içinde yapılan değişiklik dışarıdakini etkilemez:

```kolay
fonksiyon ekle_ve_göster(l: Liste<Tam>) {
    l.ekle(99)
    yazdır(l)
}

ata sayılar: Liste<Tam> = [1, 2, 3]
ekle_ve_göster(sayılar)
yazdır(sayılar)
```

```
[1, 2, 3, 99]
[1, 2, 3]
```

Fonksiyonun değiştirdiği bir şeyi dışarıda kullanmak istiyorsanız onu `döndür` ile geri verin. Kopyalar ancak değiştirildiklerinde gerçekten yapılır, o yüzden yavaş değildir.

> Python'da bu örnek ikinci satırda da `[1, 2, 3, 99]` yazardı. Python'a geçerken bu farkı unutmayın.

### Kendi kodunuzu test etmek: doğrula

Bir fonksiyon yazdıktan sonra doğru çalıştığını her seferinde `yazdır` ile gözle kontrol etmek yerine, Kolay'a kontrol ettirebilirsiniz. `doğrula(koşul)`, koşul doğruysa hiçbir şey yapmaz; yanlışsa programı durdurur ve hangi kontrolün tutmadığını söyler:

```kolay
fonksiyon faktöriyel(n: Tam(64)) -> Tam(64) {
    eğer n <= 1 { döndür 1 }
    döndür n * faktöriyel(n - 1)
}

doğrula(faktöriyel(0) == 1)
doğrula(faktöriyel(5) == 120)
doğrula(faktöriyel(3) == 5)       // yanlış bir beklenti
yazdır("Bütün testler geçti")
```

```
ÇALIŞMA HATASI (kod.kolay, satır 8): Doğrulama başarısız: faktöriyel(3) == 5
```

`doğrula` bir fonksiyonun **varsayımını** yazmak için de kullanılır. İkinci değer olarak bir mesaj verilebilir:

```kolay
fonksiyon faktöriyel(n: Tam(64)) -> Tam(64) {
    doğrula(n >= 0, "Eksi sayının faktöriyeli olmaz.")
    ...
}
faktöriyel(-2)   // ÇALIŞMA HATASI: Doğrulama başarısız: Eksi sayının faktöriyeli olmaz.
```

İyi bir alışkanlık: alıştırmaları çözerken çözümünüzün altına birkaç `doğrula` satırı yazın. Kodu sonra değiştirdiğinizde bir şeyi bozarsanız hemen görürsünüz. (Python'daki karşılığı `assert`.)

### Alıştırmalar

1. Bir sayının mutlak değerini döndüren `mutlak(x: Tam) -> Tam` fonksiyonunu yazın.
2. Bir yazının tersini döndüren `ters_çevir(s: Yazı) -> Yazı` fonksiyonunu yazın.
3. Bir kelimenin palindrom (tersten de aynı okunan: "kayak", "ses") olup olmadığını bulan fonksiyon yazın.
4. İki sayının en büyük ortak bölenini (EBOB) Öklid yöntemiyle bulun: `ebob(a, b) = ebob(b, a % b)`, `b == 0` ise sonuç `a`.

<details>
<summary>Çözümler</summary>

```kolay
// 1
fonksiyon mutlak(x: Tam) -> Tam {
    eğer x < 0 { döndür -x }
    döndür x
}
yazdır(mutlak(-8))

// 2
fonksiyon ters_çevir(s: Yazı) -> Yazı {
    ata sonuç = ""
    s içindeki her h için yap { sonuç = h + sonuç }
    döndür sonuç
}
yazdır(ters_çevir("Kolay"))   // yaloK

// 3
fonksiyon palindrom_mu(s: Yazı) -> Mantık {
    ata k = s.küçük_harf()
    döndür k == ters_çevir(k)
}
yazdır(palindrom_mu("Kayak"), palindrom_mu("elma"))

// 4
fonksiyon ebob(a: Tam, b: Tam) -> Tam {
    eğer b == 0 { döndür a }
    döndür ebob(b, a % b)
}
yazdır(ebob(48, 18))   // 6
```

</details>

---

## 12. Listeler

Liste, sıralı değerler topluluğudur:

```kolay
ata notlar = [85, 92, 78, 64]
ata şehirler = ["İzmir", "Van", "Bursa"]
yazdır(notlar)
yazdır(şehirler)
```

### Elemanlara erişmek

Elemanların sıra numarası (**indeks**) **0'dan** başlar:

```kolay
ata harfler = ["a", "b", "c", "d", "e"]
yazdır(harfler[0])       // a: ilk eleman
yazdır(harfler[2])       // c
yazdır(harfler[-1])      // e: sondan birinci
yazdır(harfler[1..3])    // ["b", "c"]: 1'den 3'e (3 hariç)
yazdır(harfler[1..=3])   // ["b", "c", "d"]
yazdır(harfler[2..])     // ["c", "d", "e"]: 2'den sona kadar
harfler[0] = "A"
yazdır(harfler)
```

### Dilimler

`liste[a..b]` listenin bir parçasından **yeni** bir liste verir; aralıklardaki kurallar burada da geçerlidir:

```kolay
ata l = [10, 20, 30, 40, 50, 60]
yazdır(l[1..4])      // 1, 2, 3
yazdır(l[1..=4])     // 1, 2, 3, 4
yazdır(l[..2])       // baştan 2'ye kadar
yazdır(l[4..])       // 4'ten sona kadar
yazdır(l[..-1])      // sonuncusu hariç hepsi
yazdır(l[-2..])      // son iki
yazdır(l[4..1])      // tersten: 4, 3, 2
yazdır(l[-1..=0])    // listenin tersi
yazdır(l[2..2], l[10..20])
```

```
[20, 30, 40]
[20, 30, 40, 50]
[10, 20]
[50, 60]
[10, 20, 30, 40, 50]
[50, 60]
[50, 40, 30]
[60, 50, 40, 30, 20, 10]
[] []
```

- Eksi indeksler önce sondan sayılır (`-1` sonuncu), sonra yön belirlenir.
- Başlangıç bitişten büyükse dilim terstir.
- Sınırların dışına taşan dilim hata vermez, liste kırpılır: `l[10..20]` boş liste verir. (Tek bir elemana `l[10]` ile erişmek ise hata verir.)

Her n. elemanı almak için `adım`, eksi adım sondan başlar:

```kolay
ata l = [10, 20, 30, 40, 50, 60]
yazdır(l.adım(2), l.adım(-2), l[1..5].adım(2))
```

```
[10, 30, 50] [60, 40, 20] [20, 40]
```

Olmayan bir indekse erişmek programı durdurur:

```kolay
ata l = [1, 2, 3]
yazdır(l[5])   // ✗ ÇALIŞMA HATASI: Listenin 5. indeksine erişilmek istendi, ama liste 3 elemanlı (0 .. 2).
```

### Liste metotları

```kolay
ata l = [30, 10, 20]
l.ekle(40)              // sona ekle:             [30, 10, 20, 40]
l.araya_ekle(0, 5)      // 0. sıraya ekle:        [5, 30, 10, 20, 40]
l.sil(1)                // 1. sıradakini sil:     [5, 10, 20, 40]
ata son = l.çıkar()     // son elemanı çıkar:     40
yazdır(l, son)
yazdır(l.uzunluk())     // 3
yazdır(l.içerir(10))    // doğru
l.sırala()              // küçükten büyüğe
yazdır(l)
l.ters_çevir()          // sırayı ters çevir
yazdır(l)
l.temizle()             // hepsini sil
yazdır(l)
```

`bul` bir elemanın indeksini verir; eleman yoksa `Hiç` döner. Bunun nasıl kullanıldığını 19. bölümde göreceğiz.

### Metot adı kuralı: değiştiren mi, yeni veren mi?

Liste metotlarının adlarında bir kural vardır, ezberlemek yerine bu kurala bakabilirsiniz:

- **Emir kipindeki** metotlar (`sırala`, `ters_çevir`, `ekle`, `sil`) listenin **kendisini değiştirir**, bir şey döndürmez.
- **İsim ya da sıfat** olan metotlar (`sıralı`, `tersi`, `adım`, `uzunluk`) listeye dokunmaz, **yeni bir değer** verir.

```kolay
ata l = [3, 1, 2]
ata s = l.sıralı()      // l değişmez, sıralanmış yeni liste
ata t = l.tersi()       // l değişmez, ters yeni liste
yazdır(l, s, t)
l.sırala()              // l'nin kendisi sıralanır
yazdır(l)
```

```
[3, 1, 2] [1, 2, 3] [2, 1, 3]
[1, 2, 3]
```

| Değiştirir (emir) | Yeni değer verir (isim/sıfat) |
|---|---|
| `ekle(x)`, `araya_ekle(i, x)`, `sil(i)`, `çıkar()` | `uzunluk()`, `içerir(x)`, `bul(x)` |
| `sırala()`, `sırala(göre: ...)` | `sıralı()`, `sıralı(göre: ...)` |
| `ters_çevir()` | `tersi()` |
| `temizle()` | `adım(n)`, dilimler `l[a..b]` |
| | `dönüşüm(...)`, `filtre(...)`, `toplam()`, `en_büyük()`, `en_küçük()` |

Eski `ters()` metodu bu kurala uymadığı için kaldırıldı; yazılırsa Kolay hangisini kastettiğinizi sorar:

```kolay
ata l = [1, 2, 3]
l.ters()    // ✗ HATA
```

```
HATA (deneme.kolay, satır 2): 'ters' yerine 'ters_çevir()' (listeyi değiştirir) ya da 'tersi()' (yeni liste verir) yazın.
  2 | l.ters()    // ✗ HATA
    |   ^^^^
```

### Bütün liste metotları

| Metot | Ne yapar |
|---|---|
| `ekle(x)` | Sona ekler |
| `araya_ekle(i, x)` | `i`. sıraya ekler; `i` liste uzunluğu olabilir (sona ekler), eksi sondan sayar |
| `sil(i)` | `i`. elemanı siler |
| `çıkar()` | Son elemanı çıkarıp döndürür; liste boşsa hata |
| `temizle()` | Hepsini siler |
| `uzunluk()` | Eleman sayısı |
| `içerir(x)` | `x` listede var mı (`Mantık`) |
| `bul(x)` | `x`'in sırası; yoksa `Hiç` (`Tam(64) veya Hiç`) |
| `sırala()` / `sıralı()` | Küçükten büyüğe (yazılar Türk alfabesine göre); sadece sayı ve yazı listelerinde |
| `ters_çevir()` / `tersi()` | Ters sıra |
| `adım(n)` | Her n. eleman |
| `dönüşüm`, `filtre`, `hepsi`, `herhangi`, `sırala(göre:)`, `sıralı(göre:)` | Fonksiyon alan metotlar, bkz. [Lambda](#14-lambda-fonksiyonları-değer-olarak-kullanmak) |
| `toplam()`, `en_büyük()`, `en_küçük()` | Sayı listelerinde toplam; en büyük ve en küçük eleman |
| `birleşik(ayraç)` | Yazı listesini tek yazıda birleştirir: `["a", "b"].birleşik(", ")` → `"a, b"` |
| `yazıya()` | Listeyi yazıya çevirir: `"[1, 2]"` |

İki liste `==` ile karşılaştırılabilir; aynı sırada aynı elemanlar varsa eşittir:

```kolay
yazdır([1, 2, 3] == [1, 2, 3], [1, 2] == [2, 1], (1..=3) == [1, 2, 3])
```

```
doğru yanlış doğru
```

### Boş liste

Boş bir listenin tipi yazılmalıdır, çünkü Kolay içine ne konacağını bilemez:

```kolay
ata kareler: Liste<Tam>
1..=5 içindeki her i için yap { kareler.ekle(i * i) }
yazdır(kareler)
```

```
[1, 4, 9, 16, 25]
```

Tipte `< >` içine elemanların tipi yazılır: `Liste<Yazı>`, `Liste<Ondalık>`. Bir listedeki bütün elemanlar aynı tipte olmalıdır:

```kolay
ata karışık = [1, "Ali"]   // ✗ HATA: Bir listedeki tüm elemanlar aynı tipte olmalı.
```

(Gerçekten karışık bir liste gerekiyorsa 19. bölümdeki `veya` tiplerine bakın.)

> **Dikkat: `Tam` ile `Tam(64)` farklı tiplerdir.** `ata l = [1, 2]` listesinin tipi `Liste<Tam(64)>` olur. Parametresi `Liste<Tam>` olan bir fonksiyona bu değişkeni verirseniz Kolay "Tip uyuşmuyor" der. Çözüm: listeyi `ata l: Liste<Tam> = [1, 2]` diye tanımlamak ya da fonksiyonda `Liste<Tam(64)>` yazmak. Tek sayılarda bu sorun olmaz, çünkü `Tam(64)` kendiliğinden `Tam`'a çevrilir.

### Liste üzerinde dolaşmak

```kolay
ata notlar = [85, 92, 78, 64]
ata toplam = 0
notlar içindeki her n için yap {
    toplam += n
}
yazdır(f"Ortalama: {toplam / notlar.uzunluk()}")
```

İndeks de lazımsa aralık üzerinden dönün:

```kolay
ata isimler = ["Ali", "Ayşe", "Can"]
0..isimler.uzunluk() içindeki her i için yap {
    yazdır(f"{i + 1}. {isimler[i]}")
}
```

### Değer semantiği

Bir listeyi başka bir değişkene atamak **kopya** oluşturur:

```kolay
ata a = [3, 1, 2]
ata b = a
b.sırala()
yazdır(a)   // [3, 1, 2]: a değişmedi
yazdır(b)   // [1, 2, 3]
```

### Sabit listeler

Sabit bir listenin içeriği de değiştirilemez:

```kolay
ata sabit günler = ["Pazartesi", "Salı"]
günler.ekle("Çarşamba")   // ✗ HATA: 'günler' sabit; 'ekle()' içeriği değiştirir.
```

### Sabit boyutlu ve çok boyutlu listeler

Tipin arkasına `[N]` yazılırsa liste tam N elemanlı olur ve varsayılan değerlerle (`0`, `""`, `yanlış`, `0.0`, boş liste, `Hiç`) dolu başlar. Bu listelere eleman eklenemez, ama elemanları değiştirilebilir. Daha hızlıdırlar.

```kolay
ata sayaçlar: Liste<Tam>[5]
sayaçlar[2] = 7
yazdır(sayaçlar)          // [0, 0, 7, 0, 0]

ata tahta: Liste<Tam>[3][3]       // 3x3 tablo
tahta[1][1] = 5
yazdır(tahta)             // [[0, 0, 0], [0, 5, 0], [0, 0, 0]]

ata satırlar: Liste<Yazı>[2][]    // 2 satır, her satır esnek
satırlar[0].ekle("merhaba")
yazdır(satırlar)          // [["merhaba"], []]
```

Sabit boyutlu listeye esnek bir liste atanabilir; eleman sayısı tutmuyorsa program çalışırken hata verir. Tersi her zaman serbesttir: sabit boyutlu bir liste esnek bir değişkene kopyalanınca kopyaya eleman eklenebilir:

```kolay
ata üçlü: Liste<Tam(64)>[3] = [1, 2, 3]
ata esnek: Liste<Tam(64)> = üçlü
esnek.ekle(4)
yazdır(üçlü, esnek)
üçlü.ekle(4)    // ✗ HATA
```

```
HATA (deneme.kolay, satır 5): Sabit boyutlu listeye (Liste<Tam(64)>[3]) eleman eklenemez ya da çıkarılamaz.
  5 | üçlü.ekle(4)    // ✗ HATA
    | ^^^^^^^^^^^^
```

### Alıştırmalar

1. Bir listedeki en büyük ve en küçük sayıyı (metot kullanmadan) bulun.
2. Bir listedeki çift sayılardan yeni bir liste oluşturun.
3. `[3, 8, 1, 9, 4]` listesini ters çevirmeden, sondan başa doğru yazdırın.
4. Bir cümledeki en uzun kelimeyi bulun (ipucu: `böl(" ")`).

<details>
<summary>Çözümler</summary>

```kolay
// 1
ata l = [42, 7, 93, 15, 61]
ata en_büyük = l[0]
ata en_küçük = l[0]
l içindeki her x için yap {
    eğer x > en_büyük { en_büyük = x }
    eğer x < en_küçük { en_küçük = x }
}
yazdır(en_büyük, en_küçük)

// 2
ata sayılar = [1, 2, 3, 4, 5, 6, 7, 8]
ata çiftler: Liste<Tam(64)>
sayılar içindeki her s için yap {
    eğer s % 2 == 0 { çiftler.ekle(s) }
}
yazdır(çiftler)

// 3
ata d = [3, 8, 1, 9, 4]
ata i = d.uzunluk() - 1
i >= 0 sürece yap {
    yazdır(d[i])
    i -= 1
}
// ya da kısaca: d[-1..=0] içindeki her x için yap { yazdır(x) }

// 4
ata cümle = "Kolay ile programlama öğrenmek eğlenceli"
ata en_uzun = ""
cümle.böl(" ") içindeki her k için yap {
    eğer k.uzunluk() > en_uzun.uzunluk() { en_uzun = k }
}
yazdır(en_uzun)
```

</details>

---

## 13. Sözlükler

Sözlük, **anahtarları** **değerlere** bağlar. Gerçek bir sözlükte kelimeden anlamına gittiğimiz gibi, anahtardan değere gideriz:

```kolay
ata başkentler: Sözlük<Yazı, Yazı> = {"Türkiye": "Ankara", "Japonya": "Tokyo"}
yazdır(başkentler["Japonya"])     // Tokyo
başkentler["Mısır"] = "Kahire"    // yeni anahtar ekle
başkentler["Türkiye"] = "ANKARA"  // var olanı değiştir
yazdır(başkentler)
```

Tip `Sözlük<AnahtarTipi, DeğerTipi>` şeklinde yazılır.

### Sözlük metotları

```kolay
ata stok: Sözlük<Yazı, Tam> = {"elma": 10, "armut": 4}
yazdır(stok.uzunluk())         // 2
yazdır(stok.içerir("elma"))    // doğru
yazdır(stok.anahtarlar())      // ["elma", "armut"]
yazdır(stok.değerler())        // [10, 4]
stok.sil("armut")
yazdır(stok)                   // {"elma": 10}
```

Olmayan bir anahtarı okumak programı durdurur, bu yüzden emin değilseniz önce `içerir` ile bakın:

```kolay
ata stok: Sözlük<Yazı, Tam> = {"elma": 10}
eğer stok.içerir("kiraz") {
    yazdır(stok["kiraz"])
} değilse {
    yazdır("Kiraz kalmadı")
}
```

### Sözlükte dolaşmak

```kolay
ata nüfus: Sözlük<Yazı, Tam> = {"İstanbul": 15_600_000, "Ankara": 5_800_000, "İzmir": 4_500_000}
nüfus içindeki her (şehir, kişi) için yap {
    yazdır(f"{şehir}: {kişi}")
}
```

Sözlük, elemanları eklendiği sırayla verir.

### Örnek: kelime sayma

Sözlüklerin en klasik kullanımı saymaktır:

```kolay
ata metin = "bir iki bir üç iki bir"
ata sayılar: Sözlük<Yazı, Tam>
metin.böl(" ") içindeki her kelime için yap {
    eğer sayılar.içerir(kelime) {
        sayılar[kelime] += 1
    } değilse {
        sayılar[kelime] = 1
    }
}
yazdır(sayılar)
```

```
{"bir": 3, "iki": 2, "üç": 1}
```

Değer olarak liste de konabilir:

```kolay
ata dersler: Sözlük<Yazı, Liste<Tam>> = {"Ali": [90, 80]}
dersler["Ali"].ekle(70)
dersler["Ayşe"] = [100]
yazdır(dersler)
```

### Alıştırmalar

1. Öğrenci adlarını notlarına bağlayan bir sözlük yazıp sınıf ortalamasını bulun.
2. Bir kelimedeki her harfin kaç kez geçtiğini sayın.
3. Telefon rehberi: kullanıcıdan bir ad alın, rehberde varsa numarasını, yoksa "kayıtlı değil" yazın.

<details>
<summary>Çözümler</summary>

```kolay
// 1
ata notlar: Sözlük<Yazı, Tam> = {"Ali": 70, "Ayşe": 95, "Can": 82}
ata toplam: Tam = 0
notlar içindeki her (ad, n) için yap { toplam += n }
yazdır(toplam / notlar.uzunluk())

// 2
ata harfler: Sözlük<Yazı, Tam>
"merhaba" içindeki her h için yap {
    eğer harfler.içerir(h) { harfler[h] += 1 } değilse { harfler[h] = 1 }
}
yazdır(harfler)

// 3
ata rehber: Sözlük<Yazı, Yazı> = {"Ali": "0532 000 00 00", "Ayşe": "0544 111 11 11"}
ata ad = oku("Kimi arıyorsunuz? ")
eğer rehber.içerir(ad) { yazdır(rehber[ad]) } değilse { yazdır("kayıtlı değil") }
```

</details>

---

## 14. Lambda: fonksiyonları değer olarak kullanmak

Şimdiye kadar fonksiyonlara sayı, yazı, liste gibi **değerler** verdik. Bu bölümde fonksiyonların kendilerinin de değer olabileceğini göreceğiz: bir fonksiyon bir değişkene konabilir, başka bir fonksiyona verilebilir, bir listede durabilir.

Bu fikir ilk başta soyut gelebilir, ama günlük kullanımı çok somuttur: "bu listedeki her sayıyı iki katına çıkar", "50'den büyükleri seç", "öğrencileri yaşa göre sırala". Bunları döngü yazmadan, tek satırda söylemenin yolu **lambda**'dır.

### İsimsiz fonksiyon: her x için ...

Döngülerde `liste içindeki her x için yap { }` yazıyorduk. `içindeki` olmadan yazılan `her x için ...` ise bir **isimsiz fonksiyondur** (lambda): "her x için şunu ver" der. Aşağıdaki `dönüşüm` metodu, listenin her elemanına bu fonksiyonu uygular ve sonuçlardan yeni bir liste yapar:

```kolay
ata sayılar = [5, 12, 3, 20, 8]
yazdır(sayılar.dönüşüm(her x için x * 2))
yazdır(sayılar)
```

```
[10, 24, 6, 40, 16]
[5, 12, 3, 20, 8]
```

Cümle gibi okunur: "sayıların dönüşümü: her x için x çarpı 2". `x`, sırayla listenin her elemanıdır. Asıl liste değişmez; `dönüşüm` yeni bir liste verir.

Aynı işi döngüyle yapmak için dört satır gerekirdi:

```kolay
ata sayılar = [5, 12, 3, 20, 8]
ata iki_katlar: Liste<Tam(64)>
sayılar içindeki her x için yap {
    iki_katlar.ekle(x * 2)
}
yazdır(iki_katlar)
```

```
[10, 24, 6, 40, 16]
```

Lambdanın gövdesi bir ifadedir; içinde başka metotlar, f-yazılar, `eşleştir` kullanılabilir:

```kolay
ata isimler = ["ali", "ayşe", "can"]
yazdır(isimler.dönüşüm(her ad için ad.büyük_harf()))
yazdır(isimler.dönüşüm(her ad için ad.uzunluk()))
yazdır(isimler.dönüşüm(her ad için f"Sayın {ad}"))
```

```
["ALİ", "AYŞE", "CAN"]
[3, 4, 3]
["Sayın ali", "Sayın ayşe", "Sayın can"]
```

Lambdanın döndürdüğü tip, yeni listenin tipini belirler: ikinci satırda yazılardan sayılar listesi çıktı.

### Fonksiyon alan liste metotları

| Metot | Ne yapar | Sonuç |
|---|---|---|
| `dönüşüm(her x için ...)` | Her elemana fonksiyonu uygular | yeni liste |
| `filtre(her x için koşul)` | Koşulu sağlayan elemanları seçer | yeni liste |
| `hepsi(her x için koşul)` | Bütün elemanlar koşulu sağlıyor mu? | `Mantık` |
| `herhangi(her x için koşul)` | En az bir eleman koşulu sağlıyor mu? | `Mantık` |
| `sırala(göre: her x için anahtar)` | Listeyi anahtara göre sıralar (listeyi **değiştirir**) | — |
| `sıralı(göre: her x için anahtar)` | Anahtara göre sıralanmış **yeni** liste verir | yeni liste |

Bunlara ek olarak sayı (ve yazı) listelerinde fonksiyon almayan üç metot daha var:

| Metot | Ne yapar |
|---|---|
| `toplam()` | Elemanların toplamı (sadece sayı listelerinde) |
| `en_büyük()` | En büyük eleman |
| `en_küçük()` | En küçük eleman |

```kolay
ata notlar = [72, 95, 48, 88, 61, 39]
yazdır(notlar.filtre(her n için n >= 50))
yazdır(notlar.hepsi(her n için n > 30), notlar.herhangi(her n için n == 100))
yazdır(notlar.toplam(), notlar.en_büyük(), notlar.en_küçük())
yazdır(notlar.sıralı(göre: her n için -n))
```

```
[72, 95, 88, 61]
doğru yanlış
403 95 39
[95, 88, 72, 61, 48, 39]
```

Son satırda `-n`'ye göre sıralayınca büyükten küçüğe sıralandı. Boş bir listede `en_büyük()` ve `en_küçük()` hata verir, çünkü boş listenin en büyük elemanı yoktur.

### Zincirleme

Her metot yeni bir liste verdiği için arka arkaya yazılabilir. Okurken soldan sağa izleyin:

```kolay
ata notlar = [72, 95, 48, 88, 61, 39]
// geçen notların 10 puan eklenmiş halinin toplamı
yazdır(notlar.filtre(her n için n >= 50).dönüşüm(her n için n + 10).toplam())
```

```
356
```

### Nesneleri anahtara göre sıralamak

`sırala` tek başına sadece sayı ve yazı listelerini sıralar. Nesne listelerinde "neye göre?" sorusunun cevabını lambdayla verirsiniz:

```kolay
tür Öğrenci(ad: Yazı, yaş: Tam(64), not: Tam(64))

ata sınıf = [Öğrenci("Can", 20, 70), Öğrenci("Ayşe", 18, 95), Öğrenci("Ömer", 19, 82), Öğrenci("Bora", 18, 60)]

sınıf.sırala(göre: her ö için ö.yaş)
yazdır(sınıf.dönüşüm(her ö için ö.ad))

yazdır(sınıf.sıralı(göre: her ö için ö.ad).dönüşüm(her ö için ö.ad))
yazdır(sınıf.sıralı(göre: her ö için -ö.not).dönüşüm(her ö için f"{ö.ad}: {ö.not}"))
```

```
["Ayşe", "Bora", "Ömer", "Can"]
["Ayşe", "Bora", "Can", "Ömer"]
["Ayşe: 95", "Ömer: 82", "Can: 70", "Bora: 60"]
```

- Sıralama **kararlıdır**: anahtarı eşit olanlar (Ayşe ve Bora, ikisi de 18) eski sıralarını korur.
- Yazı anahtarlar Türk alfabesine göre sıralanır: `Çağla`, `Can`'dan sonra, `Dilek`'ten önce gelir (bkz. [Yazılar](#6-yazılar)).
- `göre:` yazmak zorunlu değildir, ama okumayı kolaylaştırır.
- Anahtar sayı ya da yazı olmalıdır.

### Birden fazla parametre ve bloklu gövde

Birden fazla parametre parantez içinde yazılır. Gövde tek bir ifadeye sığmıyorsa `yap { }` bloğu kullanılır; blokta değer `döndür` ile verilir:

```kolay
ata sayılar = [3, 14, 8, 25]
yazdır(sayılar.dönüşüm(her x için yap {
    ata kare = x * x
    eğer kare > 100 { döndür "büyük" }
    döndür f"{kare}"
}))
```

```
["9", "büyük", "64", "büyük"]
```

### Fonksiyon değerleri: değişkende saklamak ve çağırmak

Lambda bir değişkene konabilir ve sonra normal bir fonksiyon gibi çağrılabilir. Ama bu durumda Kolay'ın parametrenin tipini bilmesi gerekir, çünkü lambdanın nerede kullanılacağını göremez. Tip parantez içinde yazılır:

```kolay
ata iki_kat = her (x: Tam(64)) için x * 2
ata topla = her (a: Tam(64), b: Tam(64)) için a + b
yazdır(iki_kat(21), topla(2, 3))
yazdır(iki_kat)
```

```
42 5
Fonksiyon(Tam(64)) -> Tam(64)
```

Tip yazılmazsa Kolay ne yapılması gerektiğini söyler:

```kolay
ata iki_kat = her x için x * 2    // ✗ HATA
```

```
HATA (deneme.kolay, satır 1): 'x' parametresinin tipi belli değil. Tipini yazın: her (x: Tam(64)) için ...
  1 | ata iki_kat = her x için x * 2    // ✗ HATA
    |                   ^
```

`dönüşüm(her x için ...)` gibi yerlerde tip yazmak gerekmiyordu, çünkü orada `x`'in listenin elemanı olduğu belliydi.

### Fonksiyon tipi

Fonksiyon değerlerinin de tipi vardır: `Fonksiyon(parametre tipleri) -> dönüş tipi`. Bir fonksiyonu parametre olarak alan fonksiyonlar bu tipi yazar:

```kolay
fonksiyon uygula(f: Fonksiyon(Tam(64)) -> Tam(64), x: Tam(64)) -> Tam(64) {
    döndür f(x)
}

fonksiyon iki_kez(f: Fonksiyon(Tam(64)) -> Tam(64), x: Tam(64)) -> Tam(64) {
    döndür f(f(x))
}

yazdır(uygula(her x için x + 1, 5))
yazdır(iki_kez(her x için x * 3, 5))
```

```
6
45
```

Değer döndürmeyen fonksiyonların tipinde `->` kısmı yazılmaz: `Fonksiyon(Yazı)`.

### İsimli fonksiyonlar da değerdir

`fonksiyon` ile tanımlanmış bir fonksiyonun adı, parantezsiz yazılınca fonksiyonun kendisidir. Lambda beklenen her yere verilebilir:

```kolay
fonksiyon kare(n: Tam(64)) -> Tam(64) { döndür n * n }
fonksiyon çift_mi(n: Tam(64)) -> Mantık { döndür n % 2 == 0 }

ata l = [1, 2, 3, 4, 5]
yazdır(l.dönüşüm(kare), l.filtre(çift_mi))

ata işlemler: Liste<Fonksiyon(Tam(64)) -> Tam(64)> = [kare, her x için x + 100, her x için -x]
yazdır(işlemler.dönüşüm(her f için f(7)))
```

```
[1, 4, 9, 16, 25] [2, 4]
[49, 107, -7]
```

Son satıra dikkat: listenin elemanları fonksiyon, lambda da her fonksiyonu 7 ile çağırıyor.

### Fonksiyon döndüren fonksiyonlar

Bir fonksiyon, başka bir fonksiyon üretip döndürebilir:

```kolay
fonksiyon toplayıcı(n: Tam(64)) -> Fonksiyon(Tam(64)) -> Tam(64) {
    döndür her x için x + n
}

ata beş_ekle = toplayıcı(5)
ata yüz_ekle = toplayıcı(100)
yazdır(beş_ekle(1), yüz_ekle(1), toplayıcı(1000)(1))
```

```
6 101 1001
```

`toplayıcı(5)` çağrısı bittiği halde dönen lambda `n`'nin değerini (5) hatırlıyor. Bunun nasıl olduğu bir sonraki başlıkta.

### Yakalama: lambda dışarıdaki değişkenleri kopyalar

Lambda, dışarıdaki bir değişkeni kullanırsa o değişkenin **oluşturulduğu andaki değerinin kopyasını** saklar. Sonradan değişkeni değiştirmek lambdayı etkilemez:

```kolay
ata zam = 10
ata zamlı = her (fiyat: Tam(64)) için fiyat + zam
zam = 99
yazdır(zamlı(100))
```

```
110
```

Lambda 110 verdi, 199 değil: `zam`'ın lambda oluşturulduğundaki değeri (10) kullanıldı. Bu, Kolay'ın genel kuralıyla aynıdır: değerler kopyalanır, paylaşılmaz (bkz. [Fonksiyonlar](#11-fonksiyonlar)).

Aynı nedenle lambda dışarıdaki bir değişkeni **değiştiremez**:

```kolay
ata sayaç = 0
ata say = her (x: Tam(64)) için yap {
    sayaç = sayaç + x    // ✗ HATA
    döndür sayaç
}
```

```
HATA (deneme.kolay, satır 3): 'sayaç' lambdanın dışında tanımlı (lambda onun kopyasını kullanır), değeri değiştirilemez.
  3 |     sayaç = sayaç + x    // ✗ HATA
    |     ^^^^^^^^^^^^^^^^^
```

Bir şeyi saymak istiyorsanız lambdanın değer döndürmesini sağlayın ve sonucu dışarıda toplayın (`toplam()`, `filtre(...).uzunluk()` gibi).

Bir türün metodu içinde yazılan lambda nesnenin alanlarını kullanabilir; ama o da nesnenin bir kopyasıyla çalışır, alanları değiştiremez:

```kolay
tür Kutu(çarpan: Tam(64)) {
    fonksiyon çarp(l: Liste<Tam(64)>) -> Liste<Tam(64)> {
        döndür l.dönüşüm(her x için x * çarpan)
    }
}
yazdır(Kutu(3).çarp([1, 2, 3]))
```

```
[3, 6, 9]
```

### Lambda hataları

| Mesaj | Sebep |
|---|---|
| `'x' parametresinin tipi belli değil. Tipini yazın: her (x: Tam(64)) için ...` | Bağlamı olmayan lambdada tip yazılmamış |
| `Burada 1 değer alan bir fonksiyon bekleniyordu, ama lambda 2 parametreli.` | Parametre sayısı uymuyor |
| `Tip uyuşmuyor: Mantık bekleniyordu, ama Tam(64) verildi.` | `filtre`'ye koşul yerine sayı veren bir lambda verildi |
| `'n' lambdanın dışında tanımlı (lambda onun kopyasını kullanır), değeri değiştirilemez.` | Lambda dışarıdaki değişkeni değiştirmeye çalıştı |
| `Fonksiyonlar karşılaştırılamaz.` | İki fonksiyon `==` ile karşılaştırıldı |
| `Bu lambda bazı yollarda değer döndürüyor, bazılarında döndürmüyor.` | Bloklu gövdede bir yolda `döndür` unutuldu |

### Ne zaman lambda, ne zaman döngü?

- Bir listeden **yeni bir liste ya da tek bir sonuç** çıkarıyorsanız (dönüştürme, seçme, toplama, sıralama) lambda daha kısa ve okunaklıdır.
- Her elemanla **bir şey yapıyorsanız** (ekrana yazmak, başka bir listeyi değiştirmek, birden fazla değişkeni güncellemek) döngü daha uygundur.

Python'da aynı işler `map`, `filter`, `lambda`, `sum`, `max`, `sorted(key=...)` ve liste üreteçleriyle (`[x * 2 for x in l]`) yapılır (bkz. [Kolay'dan Python'a](#28-kolaydan-pythona)).

### Alıştırmalar

1. `["elma", "kiraz", "üzüm", "armut"]` listesinden 5 harfli olanları seçip büyük harfle yazdırın.
2. `[3, -1, 4, -1, 5, -9]` listesindeki negatif sayıların karelerinin toplamını tek satırda bulun.
3. `tür Ürün(ad: Yazı, fiyat: Ondalık)` listesini fiyata göre ucuzdan pahalıya sıralayıp adlarını yazdırın. En pahalı ürünün fiyatını `dönüşüm` ve `en_büyük` ile bulun.
4. `fonksiyon n_kez(f: Fonksiyon(Tam(64)) -> Tam(64), n: Tam(64), x: Tam(64)) -> Tam(64)` fonksiyonunu yazın: `f`'yi `x`'e `n` kez uygulasın. `n_kez(her x için x * 2, 10, 1)` sonucu 1024 olmalı.

<details>
<summary>Çözümler</summary>

```kolay
// 1
ata meyveler = ["elma", "kiraz", "üzüm", "armut"]
yazdır(meyveler.filtre(her m için m.uzunluk() == 5).dönüşüm(her m için m.büyük_harf()))

// 2
yazdır([3, -1, 4, -1, 5, -9].filtre(her x için x < 0).dönüşüm(her x için x * x).toplam())   // 83

// 3
tür Ürün(ad: Yazı, fiyat: Ondalık)
ata ürünler = [Ürün("Defter", 40.0), Ürün("Kalem", 12.5), Ürün("Çanta", 450.0)]
yazdır(ürünler.sıralı(göre: her ü için ü.fiyat).dönüşüm(her ü için ü.ad))
yazdır(ürünler.dönüşüm(her ü için ü.fiyat).en_büyük())

// 4
fonksiyon n_kez(f: Fonksiyon(Tam(64)) -> Tam(64), n: Tam(64), x: Tam(64)) -> Tam(64) {
    ata sonuç = x
    n kere yap { sonuç = f(sonuç) }
    döndür sonuç
}
yazdır(n_kez(her x için x * 2, 10, 1))
```

</details>

---

## 15. Türler: kendi veri tipinizi yazmak

Şimdiye kadar hazır tipleri kullandık. Gerçek programlar kendi kavramlarıyla çalışır: öğrenci, ürün, hesap, oyuncu... `tür` ile yeni bir tip tanımlanır:

```kolay
tür Öğrenci(ad: Yazı, numara: Tam, not: Tam) {
}

ata ali = Öğrenci(ad: "Ali", numara: 101, not: 85)
ata ayşe = Öğrenci("Ayşe", 102, 92)    // isimsiz de yazılabilir, sıra önemli
yazdır(ali)
yazdır(ayşe.ad, ayşe.not)
ali.not = 90
yazdır(ali.not)
```

```
Öğrenci(ad: "Ali", numara: 101, not: 85)
Ayşe 92
90
```

- Parantez içindekiler türün **alanlarıdır**: her nesnenin taşıdığı bilgiler.
- `Öğrenci(...)` yeni bir **nesne** oluşturur. Tüm alanlar verilmelidir.
- Alanlara nokta ile erişilir: `ali.ad`.
- Tür adları **büyük harfle** başlamak zorundadır.

Tür bir kalıp, nesne o kalıptan çıkan şeydir: `Öğrenci` bir form, `ali` ve `ayşe` doldurulmuş iki form.

### Metotlar

Türün içine yazılan fonksiyonlara **metot** denir. Metotlar nesnenin alanlarına doğrudan isimleriyle erişir:

```kolay
tür Hesap(sahip: Yazı, bakiye: Tam) {
    fonksiyon yatır(miktar: Tam) {
        bakiye += miktar
    }

    fonksiyon çek(miktar: Tam) -> Mantık {
        eğer miktar > bakiye {
            yazdır("Yetersiz bakiye")
            döndür yanlış
        }
        bakiye -= miktar
        döndür doğru
    }

    fonksiyon özet() -> Yazı {
        döndür f"{sahip}: {bakiye} TL"
    }
}

ata h = Hesap(sahip: "Zeynep", bakiye: 100)
h.yatır(50)
h.çek(500)
h.çek(30)
yazdır(h.özet())
```

```
Yetersiz bakiye
Zeynep: 120 TL
```

Metotlar nesnenin kendisini değiştirir: `h.yatır(50)` sonrasında `h`'nin bakiyesi gerçekten artar.

### bu

Bir metodun parametresi alanla aynı ada sahipse, alana `bu` ile erişilir. `bu` "bu nesne" demektir:

```kolay
tür Kişi(ad: Yazı) {
    fonksiyon ad_değiştir(ad: Yazı) {
        bu.ad = ad     // soldaki alan, sağdaki parametre
    }
}

ata k = Kişi("Ali")
k.ad_değiştir("Veli")
yazdır(k.ad)
```

Çakışma yoksa `bu` yazmaya gerek yoktur.

### Sabit alanlar

`sabit` ile işaretlenen alan nesne oluşturulduktan sonra değiştirilemez:

```kolay
tür Vatandaş(sabit tc: Yazı, adres: Yazı) {
}

ata v = Vatandaş(tc: "12345678901", adres: "Ankara")
v.adres = "İzmir"        // ✓
yazdır(v)
```

`v.tc = "..."` yazmak derleme hatası verir.

### Nesne listeleri

```kolay
tür Ürün(ad: Yazı, fiyat: Ondalık, adet: Tam) {
    fonksiyon tutar() -> Ondalık {
        döndür fiyat * adet
    }
}

ata sepet: Liste<Ürün> = [
    Ürün("Kalem", 12.5, 4),
    Ürün("Defter", 40.0, 2),
    Ürün("Silgi", 7.5, 1),
]

ata toplam = 0.0
sepet içindeki her ü için yap {
    yazdır(f"{ü.ad}: {ü.tutar()} TL")
    toplam += ü.tutar()
}
yazdır(f"Toplam: {toplam} TL")
```

### Ortak değerler: türe ait bilgiler

Her nesnenin alanları kendisine aittir: Ali'nin adı ile Ayşe'nin adı ayrıdır. Bazen ise bir bilgi tek bir nesneye değil **türün kendisine** aittir: kaç öğrenci kaydedildiği, okulun adı gibi. Bunlar tür içinde `ortak` ile yazılır:

```kolay
tür Öğrenci(ad: Yazı, numara: Tam(64)) {
    ortak ata sayı = 0
    ortak ata sabit OKUL = "Atatürk Lisesi"

    fonksiyon tanıt() {
        yazdır(f"{numara}. {ad} ({OKUL})")
    }

    ortak fonksiyon yeni(ad: Yazı) -> Öğrenci {
        sayı += 1
        döndür Öğrenci(ad, sayı)
    }
}

ata ali = Öğrenci.yeni("Ali")
ata ayşe = Öğrenci.yeni("Ayşe")
ali.tanıt()
ayşe.tanıt()
yazdır(f"{Öğrenci.OKUL}: {Öğrenci.sayı} öğrenci")
```

```
1. Ali (Atatürk Lisesi)
2. Ayşe (Atatürk Lisesi)
Atatürk Lisesi: 2 öğrenci
```

- `ortak ata` bütün nesnelerin paylaştığı **tek bir** değerdir. Türün içinde adıyla (`sayı`), dışarıda tür adıyla (`Öğrenci.sayı`) kullanılır ve değiştirilebilir: `Öğrenci.sayı = 0`.
- `ortak ata sabit` değiştirilemeyen ortak değerdir.
- `ortak fonksiyon` türe aittir, bir nesneye değil: `Öğrenci.yeni("Ali")` diye çağrılır. İçinde `bu` ve alanlar yoktur, çünkü hangi nesne olduğu belli değildir.
- Ortak değere nesne üzerinden erişilmez: `ali.sayı` yazılırsa `'sayı' Öğrenci türünün ortak değeri, tek bir nesnenin değil. Şöyle yazın: Öğrenci.sayı`
- Ortak değer, tür tanımının olduğu yerde oluşur; tanımdan önce kullanılamaz.

Bir ortak değeri değiştirmek bütün nesneleri etkiler:

```kolay
tür Hesap(sahip: Yazı, bakiye: Ondalık) {
    ortak ata faiz = 0.05
    fonksiyon yıl_sonu() { bakiye += bakiye * faiz }
}
ata h1 = Hesap("Ali", 1000.0)
ata h2 = Hesap("Ayşe", 2000.0)
Hesap.faiz = 0.10              // iki hesabın faizi de değişti
h1.yıl_sonu()
h2.yıl_sonu()
yazdır(f"{h1.sahip}: {h1.bakiye:,.2}  {h2.sahip}: {h2.bakiye:,.2}")
```

```
Ali: 1.100,00  Ayşe: 2.200,00
```

### Alıştırmalar

1. `Dikdörtgen(en: Ondalık, boy: Ondalık)` türü yazın; `alan()` ve `çevre()` metotları olsun.
2. `Sayaç` türü yazın: `artır()`, `azalt()` ve `sıfırla()` metotları olsun, değer hiçbir zaman 0'ın altına inmesin.
3. Bir `Kitaplık` türünde `Liste<Yazı>` tipinde bir `kitaplar` alanı olsun. `ekle(ad)` ve `listele()` metotlarını yazın.

<details>
<summary>Çözümler</summary>

```kolay
// 1
tür Dikdörtgen(en: Ondalık, boy: Ondalık) {
    fonksiyon alan() -> Ondalık { döndür en * boy }
    fonksiyon çevre() -> Ondalık { döndür 2 * (en + boy) }
}
ata d = Dikdörtgen(3.0, 4.5)
yazdır(d.alan(), d.çevre())

// 2
tür Sayaç(değer: Tam) {
    fonksiyon artır() { değer += 1 }
    fonksiyon azalt() { eğer değer > 0 { değer -= 1 } }
    fonksiyon sıfırla() { değer = 0 }
}
ata s = Sayaç(0)
s.artır()
s.artır()
s.azalt()
s.azalt()
s.azalt()
yazdır(s.değer)   // 0

// 3
tür Kitaplık(kitaplar: Liste<Yazı>) {
    fonksiyon ekle(ad: Yazı) { kitaplar.ekle(ad) }
    fonksiyon listele() {
        kitaplar içindeki her k için yap { yazdır(f"- {k}") }
    }
}
ata kt = Kitaplık([])
kt.ekle("Çalıkuşu")
kt.ekle("İnce Memed")
kt.listele()
```

</details>

---

## 16. Kalıtım

Bazı türler başka türlerin özel halidir: her öğrenci bir kişidir, her öğretmen de. Ortak alanları ve metotları her türde tekrar yazmak yerine **kalıtım** kullanılır. Ebeveyn tür, köşeli parantez içinde yazılır:

```kolay
tür Kişi(ad: Yazı, yaş: Tam) {
    fonksiyon tanıt() -> Yazı {
        döndür f"Ben {ad}, {yaş} yaşındayım"
    }
    fonksiyon doğum_günü() {
        yaş += 1
    }
}

tür Öğrenci[Kişi](okul: Yazı) {
    fonksiyon okulum() -> Yazı {
        döndür f"{okul} öğrencisiyim"
    }
}

ata ö = Öğrenci(ad: "Elif", yaş: 15, okul: "Fen Lisesi")
ö.doğum_günü()             // Kişi'den gelen metot
yazdır(ö.tanıt())          // Kişi'den gelen metot
yazdır(ö.okulum())         // Öğrenci'nin kendi metodu
```

```
Ben Elif, 16 yaşındayım
Fen Lisesi öğrencisiyim
```

`Öğrenci`, `Kişi`'nin alanlarını (`ad`, `yaş`) ve metotlarını otomatik alır. Nesne oluştururken bütün alanlar verilir.

### yeniden: ebeveynin metodunu değiştirmek

Alt tür, ebeveynden gelen bir metodu kendine göre yeniden yazabilir. Bunun için `fonksiyon` yerine `yeniden` yazılır. Ebeveynin sürümü `Ebeveyn.metot()` ile çağrılabilir:

```kolay
tür Kişi(ad: Yazı) {
    fonksiyon tanıt() -> Yazı { döndür f"Ben {ad}" }
}

tür Öğretmen[Kişi](branş: Yazı) {
    yeniden tanıt() -> Yazı {
        döndür Kişi.tanıt() + f", {branş} öğretmeniyim"
    }
}

yazdır(Öğretmen("Murat", "Fizik").tanıt())
```

```
Ben Murat, Fizik öğretmeniyim
```

### Farklı türler aynı listede

Bir `Liste<Kişi>` içine `Kişi`'nin bütün alt türleri konabilir. Her nesne kendi `tanıt` sürümünü çalıştırır:

```kolay
tür Kişi(ad: Yazı) {
    fonksiyon tanıt() -> Yazı { döndür f"Ben {ad}" }
}
tür Öğrenci[Kişi](numara: Tam) {
    yeniden tanıt() -> Yazı { döndür f"Öğrenci {ad}, numara {numara}" }
}
tür Öğretmen[Kişi](branş: Yazı) {
    yeniden tanıt() -> Yazı { döndür f"{branş} öğretmeni {ad}" }
}

ata okul: Liste<Kişi> = [Öğrenci("Ali", 7), Öğretmen("Ayşe", "Matematik"), Kişi("Hasan")]
okul içindeki her k için yap {
    yazdır(k.tanıt())
}
```

```
Öğrenci Ali, numara 7
Matematik öğretmeni Ayşe
Ben Hasan
```

Buna **çok biçimlilik** (polimorfizm) denir: aynı çağrı, nesnenin gerçek türüne göre farklı davranır.

Tipi yazmasanız da olur: farklı türlerden nesnelerle kurulan bir liste, onların **ortak ebeveyni** (ya da ortak arayüzü) tipinde olur:

```kolay
tür Kişi(ad: Yazı) {
    fonksiyon tanıt() -> Yazı { döndür f"Ben {ad}" }
}
tür Öğrenci[Kişi](numara: Tam(64)) {
    yeniden tanıt() -> Yazı { döndür f"Öğrenci {ad}" }
}
tür Öğretmen[Kişi](branş: Yazı) { }

ata okul = [Öğrenci("Ali", 7), Öğretmen("Ayşe", "Fizik")]   // Liste<Kişi>
okul.ekle(Kişi("Hasan"))
okul içindeki her k için yap { yazdır(k.tanıt()) }
```

```
Öğrenci Ali
Ben Ayşe
Ben Hasan
```

### Nesnenin gerçek türüne bakmak

`Liste<Kişi>` içindeki bir nesnenin öğrenci olup olmadığını `eşleştir` ile kontrol edebilir, öğrenciye özgü alanlara öyle ulaşabilirsiniz. Eminseniz `olarak` ile de çevirirsiniz (yanılırsanız program durur):

```kolay
tür Kişi(ad: Yazı)
tür Öğrenci[Kişi](numara: Tam(64))
tür Öğretmen[Kişi](branş: Yazı)

fonksiyon açıkla(k: Kişi) -> Yazı {
    döndür eşleştir k {
        ö: Öğrenci -> f"{ö.ad}, numarası {ö.numara}"
        t: Öğretmen -> f"{t.ad}, {t.branş} öğretmeni"
        değilse -> k.ad
    }
}
ata okul: Liste<Kişi> = [Öğrenci("Ali", 7), Öğretmen("Ayşe", "Fizik"), Kişi("Hasan")]
okul içindeki her k için yap { yazdır(açıkla(k)) }
ata ali = okul[0] olarak Öğrenci
yazdır(ali.numara)
ata ayşe = okul[1] olarak Öğrenci    // ✗ ÇALIŞMA HATASI
```

```
Ali, numarası 7
Ayşe, Fizik öğretmeni
Hasan
7
ÇALIŞMA HATASI (deneme.kolay, satır 16): Bu değer Öğrenci değil, Öğretmen.
  16 | ata ayşe = okul[1] olarak Öğrenci    // ✗ ÇALIŞMA HATASI
```

### Birden fazla ebeveyn

Bir tür birden fazla türden kalıtım alabilir. İki ebeveynde aynı isimli metot varsa hangisinin çalışacağını sizin belirtmeniz gerekir, yoksa Kolay derlemez:

```kolay
tür Kişi(ad: Yazı) {
    fonksiyon tanıt() { yazdır(f"Ben {ad}") }
}
tür Sporcu(branş: Yazı) {
    fonksiyon tanıt() { yazdır(f"{branş} sporcusuyum") }
}
tür SporcuÖğrenci[Kişi, Sporcu](numara: Tam) {
    yeniden tanıt() {
        Kişi.tanıt()
        Sporcu.tanıt()
    }
}

SporcuÖğrenci(ad: "Deniz", branş: "Yüzme", numara: 12).tanıt()
```

```
Ben Deniz
Yüzme sporcusuyum
```

### Alıştırma

`Hayvan(ad: Yazı)` türü ve `ses() -> Yazı` metodu yazın. `Kedi` ve `Köpek` alt türleri `ses`'i yeniden yazsın. Karışık bir `Liste<Hayvan>` oluşturup her hayvanın adını ve sesini yazdırın.

<details>
<summary>Çözüm</summary>

```kolay
tür Hayvan(ad: Yazı) {
    fonksiyon ses() -> Yazı { döndür "..." }
}
tür Kedi[Hayvan]() {
    yeniden ses() -> Yazı { döndür "Miyav" }
}
tür Köpek[Hayvan]() {
    yeniden ses() -> Yazı { döndür "Hav hav" }
}

ata hayvanlar: Liste<Hayvan> = [Kedi("Tekir"), Köpek("Karabaş"), Hayvan("Balık")]
hayvanlar içindeki her h için yap {
    yazdır(f"{h.ad}: {h.ses()}")
}
```

</details>

---

## 17. Arayüzler

Arayüz bir **sözleşmedir**: "Bu arayüzü uygulayan her tür şu metotlara sahip olmak zorunda." İçinde kod ya da alan yoktur, sadece metotların imzaları vardır:

```kolay
arayüz Konuşabilir {
    fonksiyon konuş() -> Yazı
}

tür Kedi[Konuşabilir](ad: Yazı) {
    fonksiyon konuş() -> Yazı { döndür "Miyav" }
}

tür Robot[Konuşabilir](model: Tam) {
    fonksiyon konuş() -> Yazı { döndür f"Bip bop, ben {model}" }
}

fonksiyon konuştur(k: Konuşabilir) {
    yazdır(k.konuş())
}

konuştur(Kedi("Tekir"))
konuştur(Robot(42))
```

```
Miyav
Bip bop, ben 42
```

`Kedi` ile `Robot` arasında hiçbir akrabalık yok, ama ikisi de konuşabiliyor. `konuştur` fonksiyonu yalnızca bunu bilmek ister.

Söz tutulmazsa, yani tür arayüzdeki bir metodu yazmazsa, derleme hatası alırsınız: `'Taş', Konuşabilir arayüzünü uyguluyor ama 'konuş()' fonksiyonu yok.`

### Kalıtım mı, arayüz mü?

| | Kalıtım | Arayüz |
|---|---|---|
| Anlamı | "Öğrenci **bir** Kişi'dir" | "Kedi konuşabilir" |
| Ne verir | Alanlar ve hazır kod | Sadece söz |
| Ne zaman | Türler gerçekten akraba ve kodu paylaşıyor | Alakasız türlerin ortak bir yeteneği var |

Arayüzler ve ebeveynler aynı köşeli parantez içine yazılabilir: `tür Kedi[Hayvan, Konuşabilir](...)`.

### Alıştırma

`Şekil` arayüzü yazın: `alan() -> Ondalık` ve `ad() -> Yazı` metotları olsun. `Daire` ve `Kare` türleri bunu uygulasın. Bir `Liste<Şekil>` içindeki şekillerin alanlarının toplamını bulun.

<details>
<summary>Çözüm</summary>

```kolay
arayüz Şekil {
    fonksiyon alan() -> Ondalık
    fonksiyon ad() -> Yazı
}
tür Daire[Şekil](r: Ondalık) {
    fonksiyon alan() -> Ondalık { döndür 3.14159 * r * r }
    fonksiyon ad() -> Yazı { döndür "daire" }
}
tür Kare[Şekil](kenar: Ondalık) {
    fonksiyon alan() -> Ondalık { döndür kenar * kenar }
    fonksiyon ad() -> Yazı { döndür "kare" }
}

ata şekiller: Liste<Şekil> = [Daire(1.0), Kare(2.0), Daire(0.5)]
ata toplam = 0.0
şekiller içindeki her ş için yap {
    yazdır(ş.ad(), ş.alan())
    toplam += ş.alan()
}
yazdır(f"Toplam alan: {toplam}")
```

</details>

---

## 18. Seçenekler ve eşleştir

### Seçenek: değerin alabileceği seçenekler

Bazı değerler yalnızca belli seçeneklerden biri olabilir: trafik ışığı kırmızı, sarı ya da yeşildir; haftanın günleri yedi tanedir. Bunları yazıyla (`"kırmızı"`) tutmak hataya açıktır: `"kirmizi"` yazım hatası fark edilmez. `seçenek` ile bu seçenekler tanımlanır:

```kolay
seçenek Işık {
    Kırmızı
    Sarı
    Yeşil
}

ata ışık = Işık.Kırmızı
yazdır(ışık)
eğer ışık == Işık.Kırmızı {
    yazdır("Dur!")
}
```

`Işık.Kırmzı` gibi bir yazım hatası program çalışmadan yakalanır. Kısa seçenekler tek satırda virgülle de yazılabilir: `seçenek Yön { Kuzey, Güney, Doğu, Batı }`.

### eşleştir

`eşleştir`, bir değeri olası durumlarla karşılaştırır ve uyan dalı çalıştırır. Uzun `eğer ... değilse eğer` zincirlerinin yerine geçer:

```kolay
seçenek Işık { Kırmızı, Sarı, Yeşil }

fonksiyon ne_yapmalı(ı: Işık) {
    eşleştir ı {
        Kırmızı -> yazdır("Dur")
        Sarı    -> yazdır("Hazırlan")
        Yeşil   -> yazdır("Geç")
    }
}

ne_yapmalı(Işık.Sarı)
ne_yapmalı(Işık.Yeşil)
```

Değişkenin tipi bilindiği için dallarda `Işık.` yazmaya gerek yoktur.

- `değilse ->` dalı geri kalan her şeyi yakalar.
- Bir dalda birden fazla satır gerekiyorsa `{ }` kullanılır.

```kolay
seçenek Işık { Kırmızı, Sarı, Yeşil }
ata ı = Işık.Kırmızı
eşleştir ı {
    Kırmızı -> {
        yazdır("Dur")
        yazdır("Yayalar geçebilir")
    }
    değilse -> yazdır("Yayalar beklesin")
}
```

### Sayılarla eşleştir

`eşleştir` sayılar, aralıklar ve `veya` ile de çalışır:

```kolay
fonksiyon dönem(yaş: Tam) -> Yazı {
    döndür eşleştir yaş {
        0..=12        -> "çocuk"
        13..=17       -> "genç"
        65 veya 66    -> "yeni emekli"
        değilse       -> "yetişkin"
    }
}
yazdır(dönem(8), dönem(15), dönem(66), dönem(40))
```

```
çocuk genç yeni emekli yetişkin
```

Bu örnekte `eşleştir` bir **değer döndürüyor**. Değer döndüren `eşleştir`, tüm durumları kapsamıyorsa `değilse` dalı zorunludur. Aksi halde hiçbir dal uymadığında sonuç belirsiz kalırdı.

Aralık desenlerinde sıra önemli değildir: `5..=1` deseni de 1'den 5'e kadar sayılara uyar; `10..7` deseni ise (10, 9, 8) değerlerine uyar. Bir desen "bu sayılardan biri mi?" diye sorar.

Desen olarak yazılar, ondalık sayılar ve `doğru`/`yanlış` da kullanılabilir:

```kolay
fonksiyon selam(dil: Yazı) -> Yazı {
    döndür eşleştir dil.küçük_harf() {
        "tr" veya "türkçe" -> "Merhaba"
        "en" -> "Hello"
        "de" -> "Hallo"
        değilse -> "?"
    }
}
yazdır(selam("TR"), selam("en"), selam("fr"))
```

```
Merhaba Hello ?
```

### Eksik durum uyarısı

Değer döndürmeyen bir `eşleştir` bütün seçenekleri kapsamak zorunda değildir: hiçbir dal uymazsa hiçbir şey yapılmaz. Ama bir seçeneği unutmuş olabileceğiniz için Kolay uyarır, program yine çalışır:

```kolay
seçenek Işık { Kırmızı, Sarı, Yeşil }
ata ı = Işık.Sarı
eşleştir ı {
    Kırmızı -> yazdır("Dur")
}
yazdır("bitti")
```

```
Uyarı (deneme.kolay, satır 3): 'Sarı', 'Yeşil' seçenekleri ele alınmamış.
  3 | eşleştir ı {
    | ^^^^^^^^
bitti
```

### Dalda atama

Bir dalda tek bir atama da yazılabilir; bu durumda `{ }` gerekmez:

```kolay
seçenek Işık { Kırmızı, Sarı, Yeşil }
ata ışıklar = [Işık.Kırmızı, Işık.Yeşil, Işık.Kırmızı, Işık.Sarı]
ata dur_sayısı = 0
ata geç_sayısı = 0
ışıklar içindeki her ı için yap {
    eşleştir ı {
        Kırmızı -> dur_sayısı += 1
        Yeşil -> geç_sayısı += 1
        değilse -> { }
    }
}
yazdır(dur_sayısı, geç_sayısı)
```

```
2 1
```

Değer döndüren bir `eşleştir`'de dallar değer olmalıdır; orada atama ya da `{ }` bloğu yazılamaz.

### Veri taşıyan seçenekler

Seçenekler yanlarında veri de taşıyabilir. Bir şekil daire ya da dikdörtgen olabilir, ama dairenin yarıçapı, dikdörtgenin eni ve boyu vardır:

```kolay
seçenek Şekil {
    Daire(yarıçap: Ondalık)
    Dikdörtgen(en: Ondalık, boy: Ondalık)
    Kare(kenar: Ondalık)

    fonksiyon alan() -> Ondalık {
        döndür eşleştir bu {
            Daire(r)         -> 3.14159 * r * r
            Dikdörtgen(e, b) -> e * b
            Kare(k)          -> k * k
        }
    }
}

ata şekiller = [Şekil.Daire(yarıçap: 1.0), Şekil.Dikdörtgen(2.0, 3.0), Şekil.Kare(4.0)]
şekiller içindeki her ş için yap {
    yazdır(ş, "alan:", ş.alan())
}
```

```
Şekil.Daire(yarıçap: 1.0) alan: 3.14159
Şekil.Dikdörtgen(en: 2.0, boy: 3.0) alan: 6.0
Şekil.Kare(kenar: 4.0) alan: 16.0
```

`Daire(r)` dalı, değer bir daireyse yarıçapını `r` adıyla açar. İstenmeyen değerlerin yerine `_` yazılır: `Dikdörtgen(e, _)`. Tek başına `_` deseni `değilse` gibi her şeye uyar. Seçeneklerin içine de metot yazılabilir (yukarıdaki `alan`).

### Ek koşul

Bir dala `eğer` ile ek koşul eklenebilir:

```kolay
seçenek Şekil {
    Daire(r: Ondalık)
    Kare(k: Ondalık)
}

fonksiyon boyut(ş: Şekil) -> Yazı {
    döndür eşleştir ş {
        Daire(r) eğer r > 10.0 -> "büyük daire"
        Daire(r)               -> "küçük daire"
        değilse                -> "başka şekil"
    }
}
yazdır(boyut(Şekil.Daire(20.0)), boyut(Şekil.Daire(1.0)), boyut(Şekil.Kare(3.0)))
```

### Alıştırmalar

1. `seçenek Gün` ile haftanın günlerini tanımlayın. Bir günün hafta sonu olup olmadığını döndüren fonksiyon yazın.
2. `seçenek İşlem { Topla, Çıkar, Çarp, Böl }` tanımlayın. `hesapla(a, b, işlem)` fonksiyonunu `eşleştir` ile yazın.
3. Bir sınav puanını harf notuna çeviren fonksiyonu aralıklarla yazın: 90–100 AA, 80–89 BA, 70–79 BB, 60–69 CB, 50–59 CC, gerisi FF.

<details>
<summary>Çözümler</summary>

```kolay
// 1
seçenek Gün { Pazartesi, Salı, Çarşamba, Perşembe, Cuma, Cumartesi, Pazar }
fonksiyon hafta_sonu_mu(g: Gün) -> Mantık {
    döndür eşleştir g {
        Cumartesi veya Pazar -> doğru
        değilse -> yanlış
    }
}
yazdır(hafta_sonu_mu(Gün.Pazar), hafta_sonu_mu(Gün.Salı))

// 2
seçenek İşlem { Topla, Çıkar, Çarp, Böl }
fonksiyon hesapla(a: Ondalık, b: Ondalık, i: İşlem) -> Ondalık {
    döndür eşleştir i {
        Topla -> a + b
        Çıkar -> a - b
        Çarp  -> a * b
        Böl   -> a / b
    }
}
yazdır(hesapla(6.0, 3.0, İşlem.Çarp))

// 3
fonksiyon harf_notu(p: Tam) -> Yazı {
    döndür eşleştir p {
        90..=100 -> "AA"
        80..=89  -> "BA"
        70..=79  -> "BB"
        60..=69  -> "CB"
        50..=59  -> "CC"
        değilse  -> "FF"
    }
}
yazdır(harf_notu(95), harf_notu(72), harf_notu(30))
```

</details>

---

## 19. veya tipleri ve Hiç

### veya: birden fazla tipten biri

Bazen bir değer birkaç tipten biri olabilir. Tip yazılırken `veya` kullanılır:

```kolay
ata tablo: Liste<Tam veya Yazı> = [5, "Ali", 12, "Ayşe"]
tablo içindeki her e için yap {
    eşleştir e {
        Tam(n)  -> yazdır(f"sayı: {n * 2}")
        Yazı(y) -> yazdır(f"yazı: {y.büyük_harf()}")
    }
}
```

```
sayı: 10
yazı: ALİ
sayı: 24
yazı: AYŞE
```

`veya` tipli bir değeri kullanmadan önce gerçekte hangi tip olduğunu `eşleştir` ile kontrol etmeniz gerekir. Bu sayede "sayı sandım, yazı çıktı" hatası olamaz. Tipten eminseniz `olarak` ile de çıkarabilirsiniz: `tablo[1] olarak Yazı` (yanılırsanız program durur).

Tip desenleri iki şekilde yazılır, ikisi de aynıdır: `Tam(n) ->` ve `n: Tam ->`. İkincisi, tipin kendisi parantez aldığında (`Tam(64)`, `Ondalık(32)`) daha okunaklıdır:

```kolay
ata değerler: Liste<Tam(64) veya Ondalık veya Yazı> = [3, 2.5, "üç"]
değerler içindeki her d için yap {
    eşleştir d {
        n: Tam(64) -> yazdır("tam:", n + 1)
        o: Ondalık -> yazdır("ondalık:", o * 2)
        y: Yazı -> yazdır("yazı:", y.büyük_harf())
    }
}
```

```
tam: 4
ondalık: 5.0
yazı: ÜÇ
```

Değeri kullanmayacaksanız ad yerine `_` yazılır: `_: Yazı -> yazdır("bir yazı")`.

Bir `veya` değişkenine üye tiplerinden herhangi biri atanabilir, `veya` tipi daha geniş bir `veya` tipine de atanabilir:

```kolay
ata x: Tam(64) veya Yazı = 5
x = "beş"
ata y: Tam(64) veya Yazı veya Mantık = x
yazdır(x, y)
```

```
beş beş
```

Tam sayılar için önemli bir kural: bir `veya` tipinde iki farklı tam sayı tipi (`Tam(8) veya Tam(64)`) ya da iki farklı ondalık tipi bulunamaz.

### Hiç: değer yok

`Hiç`, "burada bir değer yok" demektir. Listelerin `bul` metodu, aranan eleman yoksa `Hiç` döndürür; sonucun tipi `Tam veya Hiç`'tir:

```kolay
ata isimler = ["Ali", "Ayşe", "Can"]
ata sonuç = isimler.bul("Ayşe")
eşleştir sonuç {
    Tam(i) -> yazdır(f"{i}. sırada bulundu")
    Hiç    -> yazdır("bulunamadı")
}
```

```
1. sırada bulundu
```

Bunu kendi fonksiyonlarınızda da kullanabilirsiniz:

```kolay
fonksiyon ilk_negatif(l: Liste<Tam(64)>) -> Tam(64) veya Hiç {
    l içindeki her x için yap {
        eğer x < 0 { döndür x }
    }
    döndür Hiç
}

eşleştir ilk_negatif([4, 7, -2, 9]) {
    Tam(n) -> yazdır(f"İlk negatif sayı: {n}")
    Hiç    -> yazdır("Negatif sayı yok")
}
```

Birçok dilde "değer yok" durumu `null` ile gösterilir ve kontrol edilmeyi unutulan bir `null` programı çökertir. Bu hataya, onu icat eden kişi "milyar dolarlık hata" der. Kolay'da `Hiç` olabilecek bir değer kontrol edilmeden kullanılamaz; derleyici izin vermez:

```kolay
ata ad: Yazı veya Hiç = Hiç
yazdır(ad.uzunluk())    // ✗ HATA
```

```
HATA (deneme.kolay, satır 2): Bu değer Hiç olabilir (Yazı veya Hiç). Kullanmadan önce eşleştir ile kontrol edin.
  2 | yazdır(ad.uzunluk())    // ✗ HATA
    |           ^^^^^^^
```

Değer döndüren bir `eşleştir`'in `değilse` dalı `Hiç` döndürürse sonucun tipi kendiliğinden `... veya Hiç` olur:

```kolay
seçenek Renk { Kırmızı, Yeşil, Mavi }
fonksiyon türkçe(r: Renk) -> Yazı veya Hiç {
    döndür eşleştir r {
        Kırmızı -> "kırmızı"
        değilse -> Hiç
    }
}
yazdır(türkçe(Renk.Kırmızı), türkçe(Renk.Mavi))
```

```
kırmızı Hiç
```

`Hiç` ve `veya` tiplerinin bir uzantısı olan `T veya Hata` tipini, [Hata yönetimi](#21-hata-yönetimi-dene-yakala-fırlat) bölümünde göreceğiz.

---

## 20. Generic: her tiple çalışan kod

### Sorun: aynı kodu iki kez yazmak

Bir listenin en büyük elemanını bulan bir fonksiyon yazalım:

```kolay
fonksiyon en_büyüğü(l: Liste<Tam(64)>) -> Tam(64) {
    ata büyük = l[0]
    l içindeki her x için yap {
        eğer x > büyük { büyük = x }
    }
    döndür büyük
}
```

Bu fonksiyon yalnızca tam sayı listelerinde çalışır. Yazı listesinin en büyüğünü (alfabede en sondakini) bulmak için aynı kodu bir kez daha, bu sefer `Liste<Yazı>` ve `Yazı` yazarak kopyalamamız gerekir. Ondalık için bir kez daha. Kod aynı, sadece tipler farklı.

### Çözüm: tip yerine bir ad

Fonksiyonun adından sonra `< >` içine bir **tip parametresi** yazarız. Bu, "henüz belli olmayan bir tip" demektir. Gelenek olarak `T` harfi kullanılır (İngilizce *type*, tip):

```kolay
fonksiyon en_büyüğü<T>(l: Liste<T>) -> T {
    ata büyük = l[0]
    l içindeki her x için yap {
        eğer x > büyük { büyük = x }
    }
    döndür büyük
}

yazdır(en_büyüğü([3, 9, 2]))
yazdır(en_büyüğü(["elma", "çilek", "armut"]))
yazdır(en_büyüğü([1.5, 0.5]))
```

```
9
elma
1.5
```

Okuyuşu: "Herhangi bir `T` tipi için, `T`'lerden oluşan bir liste alır ve bir `T` döndürür." Böyle, tipi sonradan belli olan fonksiyonlara **generic** fonksiyon denir (İngilizcede "genel" demektir; bu ad bütün dillerde kullanılır, Python'a geçince de karşınıza çıkacak).

`T`'nin ne olduğunu siz yazmazsınız. Kolay, verdiğiniz değerden kendisi bulur:

| Çağrı | Verilen | `T` |
|---|---|---|
| `en_büyüğü([3, 9, 2])` | `Liste<Tam(64)>` | `Tam(64)` |
| `en_büyüğü(["elma", ...])` | `Liste<Yazı>` | `Yazı` |
| `en_büyüğü([1.5, 0.5])` | `Liste<Ondalık>` | `Ondalık` |

`T` dönüş tipinde de, fonksiyonun içinde de kullanılabilir:

```kolay
fonksiyon ikile<T>(x: T) -> Liste<T> {
    döndür [x, x]
}
yazdır(ikile(7))
yazdır(ikile("ha"))
```

```
[7, 7]
["ha", "ha"]
```

- Tip parametreleri tipler gibi **büyük harfle** başlar: `T`, `Eleman`, `Anahtar`. Küçük harfle yazılırsa Kolay düzeltmeyi önerir.
- Bir tipin adı tip parametresi olamaz: `fonksiyon f<Tam>(...)` hata verir.

### Tip bulunamazsa

Kolay `T`'yi ancak verilen değerlerden bulabilir. Boş bir listenin tipi belli değildir:

```kolay
fonksiyon ilk<T>(l: Liste<T>) -> T { döndür l[0] }
yazdır(ilk([]))
```

```
HATA (kod.kolay, satır 2): Boş listenin tipi belli değil. Tipini yazın: ata liste: Liste<Tam> = []
```

Çözüm, değere tip vermektir: `ilk([] olarak Liste<Tam>)`. `T` hiçbir parametrede geçmiyorsa da hata alırsınız, çünkü Kolay onu hiçbir yerden çıkaramaz: `'f' için 'T' tipi verilen değerlerden çıkarılamıyor.`

### Birden fazla tip parametresi

Virgülle ayırarak birden fazla tip parametresi yazılabilir. Her biri ayrı bir tiptir:

```kolay
fonksiyon eşle<A, B>(a: A, b: B) -> Yazı {
    döndür f"{a} ile {b}"
}
yazdır(eşle(1, "bir"))
yazdır(eşle(doğru, 2.5))
```

```
1 ile bir
doğru ile 2.5
```

### Aynı T'ye farklı değerler

Aynı `T`'ye iki değer verilirse ikisi de aynı tipte olmalıdır. Sayılarda Kolay, normal işlemlerdeki gibi ikisini de kapsayan tipi seçer:

```kolay
fonksiyon topla<T>(a: T, b: T) -> T {
    döndür a + b
}
ata küçük: Tam(8) = 100
yazdır(topla(küçük, 5))       // T = Tam(8): 5 sığıyor
yazdır(topla(küçük, 1000))    // 1000 Tam(8)'e sığmıyor: T = Tam(64)
yazdır(topla(2, 2.5))         // T = Ondalık
```

```
105
1100
4.5
```

Ortak bir tip yoksa hata alırsınız: `topla("a", 1)` → `'T' hem Yazı hem Tam(64) olamaz: aynı tip parametresine farklı tipte değerler verildi.`

Dikkat: `T` belli olduktan sonra kurallar o tipin kurallarıdır. `topla(küçük, küçük)` çağrısında `T` = `Tam(8)` olur ve `100 + 100` taşar, program Türkçe bir taşma hatasıyla durur.

### Generic fonksiyon ve lambda

Generic bir fonksiyon, fonksiyon da alabilir. Aşağıdaki `dönüştür`, listenin her elemanına verilen fonksiyonu uygular. `T` listenin eleman tipi, `R` (İngilizce *result*, sonuç) fonksiyonun döndürdüğü tiptir:

```kolay
fonksiyon dönüştür<T, R>(l: Liste<T>, f: Fonksiyon(T) -> R) -> Liste<R> {
    ata sonuç: Liste<R>
    l içindeki her x için yap {
        sonuç.ekle(f(x))
    }
    döndür sonuç
}
yazdır(dönüştür([1, 2, 3], her x için x * 10))
yazdır(dönüştür(["a", "bc"], her s için s.uzunluk()))
```

```
[10, 20, 30]
[1, 2]
```

Lambdanın parametre tipini yazmadık: `T` listeden belli olduğu için Kolay `x`'in tipini kendisi bilir. `R` ise lambdanın döndürdüğü değerden bulunur. (Listelerin hazır `dönüşüm` metodu tam olarak budur.)

### Arka planda ne oluyor?

Kolay, generic bir fonksiyonu bir **kalıp** gibi kullanır. `en_büyüğü([3, 9, 2])` çağrısını görünce `T` yerine `Tam(64)` yazılmış bir kopya oluşturur; `en_büyüğü(["elma"])` için `T` yerine `Yazı` yazılmış ikinci bir kopya. Aynı tiple yapılan diğer çağrılar aynı kopyayı kullanır. Bunun üç sonucu var:

1. **Hız kaybı yok.** Her kopya normal bir fonksiyon gibi derlenir; generic yazmak program hızını değiştirmez.
2. **Hata, kopyada bulunur.** Fonksiyonun içi her tiple ayrıca denetlenir. Bir tiple çalışmayan bir şey yazılmışsa hata mesajı hem fonksiyondaki satırı hem çağrının satırını gösterir:

   ```kolay
   fonksiyon topla<T, Z>(x: T, y: Z) -> T {
       döndür x + y
   }
   yazdır(topla(1, 2))       // T = Tam(64), Z = Tam(64): çalışır
   yazdır(topla("a", 2))     // T = Yazı, Z = Tam(64): yazıya sayı eklenemez
   ```

   ```
   HATA (kod.kolay, satır 2): topla içinde: Yazı ile Tam(64) toplanamaz. f-yazı kullanın: f"{a}{b}"
   (çağrı: satır 5, T = Yazı, Z = Tam(64))
     2 |     döndür x + y
       |            ^^^^^
   ```

   İlk satır hatanın nerede olduğunu, ikinci satır hangi çağrının ve hangi tiplerin buna yol açtığını söyler.
3. **Hiç çağrılmayan generic fonksiyonun içi denetlenmez.** `T`'nin ne olduğu bilinmeden içindeki işlemlerin doğru olup olmadığı anlaşılamaz. Generic bir fonksiyon yazdıktan sonra onu en az bir kez çağırarak deneyin.

### Generic türler

Türler de tip parametresi alabilir. Tür adından sonra `< >` yazılır, alanlarda ve metotlarda `T` kullanılır:

```kolay
tür Kutu<T>(değer: T) {
    fonksiyon al() -> T { döndür değer }
    fonksiyon koy(yeni: T) { değer = yeni }
}

ata k = Kutu(değer: 5)
yazdır(k)
k.koy(7)
yazdır(k.al())

ata y = Kutu("merhaba")
yazdır(y)
```

```
Kutu<Tam(64)>(değer: 5)
7
Kutu<Yazı>(değer: "merhaba")
```

`Kutu(değer: 5)` yazınca `T`, fonksiyonlardaki gibi verilen değerden bulunur ve nesnenin tipi `Kutu<Tam(64)>` olur. `Kutu<Tam(64)>` ile `Kutu<Yazı>` iki ayrı tiptir. Birinin içine diğerinin tipinde değer konamaz: `k.değer = "x"` yazılırsa `Tip uyuşmuyor: Tam(64) bekleniyordu, ama Yazı verildi.`

Tipi açıkça yazmak gerektiğinde (değişken, parametre, liste elemanı) içindeki tip `< >` ile yazılır:

```kolay
ata s: Kutu<Tam(8)> = Kutu(değer: 3)       // T = Tam(8): yazılan tip kullanılır
ata kutular: Liste<Kutu<Yazı>> = [Kutu("a"), Kutu("b")]
fonksiyon aç<T>(k: Kutu<T>) -> T { döndür k.değer }
```

Yalnızca `Kutu` yazmak yetmez, çünkü neyin kutusu olduğu belli değildir: `'Kutu' generic bir tür; içindeki tipleri yazın: Kutu<Tam>`.

#### Örnek: yığın

**Yığın**, üst üste konan tabaklar gibidir: en son konan en önce alınır. Her tiple çalışan bir yığın:

```kolay
tür Yığın<T>(elemanlar: Liste<T>) {
    fonksiyon koy(x: T) { elemanlar.ekle(x) }
    fonksiyon al() -> T { döndür elemanlar.çıkar() }
    fonksiyon boş_mu() -> Mantık { döndür elemanlar.uzunluk() == 0 }
}

ata tabaklar = Yığın([] olarak Liste<Yazı>)
tabaklar.koy("mavi")
tabaklar.koy("kırmızı")
tabaklar.koy("yeşil")
değil tabaklar.boş_mu() sürece yap {
    yazdır(tabaklar.al())
}
```

```
yeşil
kırmızı
mavi
```

#### Örnek: kendini içeren tür

Bir tür, alanında kendi tipini de taşıyabilir. Her düğümün bir değeri ve bir sonraki düğümü olan **bağlı liste**:

```kolay
tür Düğüm<T>(değer: T, sonraki: Düğüm<T> veya Hiç)

ata liste = Düğüm(1, Düğüm(2, Düğüm(3, Hiç)))
yazdır(liste)
```

```
Düğüm<Tam(64)>(değer: 1, sonraki: Düğüm<Tam(64)>(değer: 2, sonraki: Düğüm<Tam(64)>(değer: 3, sonraki: Hiç)))
```

Generic türler kalıtım alabilir ve arayüz uygulayabilir: `tür AdlıKutu<T>[Temel, Adlı](değer: T)`.

### Şimdilik olmayanlar

Bunları yazarsanız Kolay ne yapmanız gerektiğini söyleyen bir hata verir:

- Generic bir türden kalıtım: `tür A[Kutu<Tam>]`.
- Türün içindeki bir metoda ayrıca tip parametresi: `tür Kutu<T> { fonksiyon f<Z>() { } }`. Dışarıda generic bir fonksiyon yazın.
- Generic seçenek: `seçenek S<T> { ... }`. Yerine `T veya Hiç` kullanılabilir.
- Generic bir fonksiyonu değer olarak vermek: `ata f = en_büyüğü`. Çağırarak kullanın: `en_büyüğü(...)`.

### Python'daki karşılığı

Python'da tipler zaten program çalışırken kontrol edildiği için her fonksiyon kendiliğinden her tiple çağrılabilir. Generic yazım yalnızca okuyana (ve kontrol araçlarına) bilgi verir:

```python
from typing import TypeVar, Generic
T = TypeVar("T")

def en_buyugu(l: list[T]) -> T:
    return max(l)

class Kutu(Generic[T]):
    def __init__(self, deger: T):
        self.deger = deger
```

Fark: Kolay `en_büyüğü(["a"])` ile `topla("a", 2)` arasındaki farkı program başlamadan bulur; Python `"a" + 2` hatasını ancak o satır çalışınca verir.

### Alıştırmalar

1. `son<T>(l: Liste<T>) -> T veya Hiç` fonksiyonunu yazın: listenin son elemanını, liste boşsa `Hiç` döndürsün. Bir tam sayı, bir yazı ve boş bir ondalık listesiyle deneyin.
2. `kaç_tane<T>(l: Liste<T>, aranan: T) -> Tam(64)`: `aranan` listede kaç kez geçiyor? `kaç_tane([1, 2, 1, 1], 1)` → `3`.
3. `Çift<A, B>(ilk: A, ikinci: B)` türünü yazın. `ters()` metodu yerleri değişmiş yeni bir çift döndürsün. Dönüş tipi ne olmalı?
4. Yığın örneğine `tepe() -> T veya Hiç` metodu ekleyin: en üstteki elemanı almadan göstersin, yığın boşsa `Hiç` döndürsün.

<details>
<summary>Çözüm</summary>

```kolay
// 1
fonksiyon son<T>(l: Liste<T>) -> T veya Hiç {
    eğer l.uzunluk() == 0 { döndür Hiç }
    döndür l[-1]
}
yazdır(son([1, 2, 3]))                     // 3
yazdır(son(["a"]))                         // a
yazdır(son([] olarak Liste<Ondalık>))      // Hiç

// 2
fonksiyon kaç_tane<T>(l: Liste<T>, aranan: T) -> Tam(64) {
    ata sayı = 0
    l içindeki her x için yap {
        eğer x == aranan { sayı += 1 }
    }
    döndür sayı
}
yazdır(kaç_tane([1, 2, 1, 1], 1))          // 3
yazdır(kaç_tane(["a", "b", "a"], "b"))     // 1

// 3: dönüş tipi Çift<B, A>, çünkü yerler değişiyor
tür Çift<A, B>(ilk: A, ikinci: B) {
    fonksiyon ters() -> Çift<B, A> {
        döndür Çift(ilk: ikinci, ikinci: ilk)
    }
}
ata ç = Çift("Ali", 90)
yazdır(ç)            // Çift<Yazı, Tam(64)>(ilk: "Ali", ikinci: 90)
yazdır(ç.ters())     // Çift<Tam(64), Yazı>(ilk: 90, ikinci: "Ali")

// 4
tür Yığın<T>(elemanlar: Liste<T>) {
    fonksiyon koy(x: T) { elemanlar.ekle(x) }
    fonksiyon tepe() -> T veya Hiç {
        eğer elemanlar.uzunluk() == 0 { döndür Hiç }
        döndür elemanlar[-1]
    }
}
ata y = Yığın([1, 2])
y.koy(3)
yazdır(y.tepe())                           // 3
yazdır(Yığın([] olarak Liste<Yazı>).tepe())  // Hiç
```

</details>

---

## 21. Hata yönetimi: dene, yakala, fırlat

Şimdiye kadar bir çalışma hatası olduğunda (sıfıra bölme, olmayan indeks, sayıya çevrilemeyen yazı) program Türkçe bir mesajla **durdu**. Çoğu zaman doğru davranış budur: bir şey ters gittiyse yanlış sonuçla devam etmektense durmak iyidir.

Ama bazı hatalar **beklenen** hatalardır. Kullanıcıdan yaşını istediniz, o "yirmi" yazdı. Bu durumda programın çökmesi değil, "lütfen sayı yazın" demesi gerekir. Bir dosya bulunamadı, bir sözlükte aranan isim yok... Bu bölüm bu tür hataları **yakalayıp ele almayı** anlatır.

### İlk örnek: dene ve yakala

```kolay
ata girdi = "yirmi"
dene {
    ata yaş = girdi.tama()
    yazdır(f"10 yıl sonra {yaş + 10} yaşında olacaksınız.")
} yakala h: Hata.Dönüşüm {
    yazdır("Yaşınızı sayıyla yazmalısınız.")
    yazdır("Ayrıntı:", h.mesaj)
}
yazdır("Program devam ediyor.")
```

```
Yaşınızı sayıyla yazmalısınız.
Ayrıntı: "yirmi" bir tam sayıya çevrilemez.
Program devam ediyor.
```

Nasıl okunur:

- `dene { ... }`: "Bu bloğu dene." Blok normal çalışır.
- Blokta bir hata olursa blok **hatanın olduğu yerde durur**. `yazdır(f"10 yıl sonra ...")` satırı hiç çalışmadı.
- `yakala h: Hata.Dönüşüm { ... }`: "Bir dönüşüm hatası olursa onu `h` adıyla yakala ve bu bloğu çalıştır."
- Yakala bloğu bitince program `dene`'nin **sonrasından** devam eder. Program çökmedi.

`girdi = "42"` olsaydı hata olmazdı: dene bloğu sonuna kadar çalışır, yakala bloğu hiç çalışmazdı:

```kolay
ata girdi = "42"
dene {
    ata yaş = girdi.tama()
    yazdır(f"10 yıl sonra {yaş + 10} yaşında olacaksınız.")
} yakala h: Hata.Dönüşüm {
    yazdır("Yaşınızı sayıyla yazmalısınız.")
}
```

```
10 yıl sonra 52 yaşında olacaksınız.
```

### Hata nesneleri

Yakalanan hata bir **nesnedir** (bkz. [Türler](#15-türler-kendi-veri-tipinizi-yazmak)). Her hatanın bir `mesaj` alanı vardır; bu, hata yakalanmasaydı ekrana yazılacak olan Türkçe mesajdır. Hatanın kendisini de yazdırabilirsiniz:

```kolay
ata l = [10, 20, 30]
dene {
    yazdır(l[7])
} yakala h: Hata.İndeks {
    yazdır(h.mesaj)
    yazdır(h)
}
```

```
Listenin 7. indeksine erişilmek istendi, ama liste 3 elemanlı (0 .. 2).
Hata.İndeks(mesaj: "Listenin 7. indeksine erişilmek istendi, ama liste 3 elemanlı (0 .. 2).")
```

### Hazır hata türleri

Kolay'ın kendi çalışma hataları şu türlerdendir. Hepsi `Hata` türünün alt türüdür (bkz. [Kalıtım](#16-kalıtım)):

| Tür | Ne zaman | Örnek |
|---|---|---|
| `Hata.SıfıraBölme` | Tam sayıyı sıfıra bölmek ya da sıfıra göre kalan almak | `10 / 0`, `10 % 0` |
| `Hata.İndeks` | Listede ya da yazıda olmayan bir sıra numarası; boş listeden eleman istemek | `l[7]`, `"abc".karakter(9)`, boş listede `çıkar()`, `en_büyük()` |
| `Hata.Anahtar` | Sözlükte olmayan bir anahtar | `notlar["Veli"]`, olmayan anahtarı `sil` |
| `Hata.Taşma` | Sayı, tipinin sınırına sığmadı | `Tam(8)` 127'yi aştı, `Tam(+)` negatif oldu, `2 ** 200` |
| `Hata.Dönüşüm` | Bir değer başka bir tipe çevrilemedi | `"abc".tama()`, `300 olarak Tam(8)`, `TanımsızSayı` tam sayıya çevrildi |
| `Hata` | Diğerleri | `Adım sıfır olamaz.`, `Aralık çok büyük`, negatif `kere` |

`yakala h: Hata` yazılırsa `Hata`'nın **bütün** alt türleri yakalanır:

```kolay
ata sayılar = [8, 0, 4]
sayılar içindeki her s için yap {
    dene {
        yazdır(100 / s)
    } yakala h: Hata {
        yazdır("hata:", h.mesaj)
    }
}
```

```
12
hata: Sıfıra bölme hatası.
25
```

Burada `dene` bir döngünün içinde: her tur kendi `dene` bloğunu çalıştırdı. 0'a bölünen tur hata verdi ama döngü devam etti.

### Birden fazla yakala dalı

Bir `dene`'nin arkasına birden fazla `yakala` yazılabilir. Hata olunca dallar **yukarıdan aşağıya** denenir, uyan **ilk** dal çalışır:

```kolay
fonksiyon işle(l: Liste<Tam(64)>, i: Tam(64)) {
    dene {
        yazdır(100 / l[i])
    } yakala h: Hata.SıfıraBölme {
        yazdır("sıfıra bölünemez")
    } yakala h: Hata.İndeks {
        yazdır(f"{i}. eleman yok")
    } yakala h: Hata {
        yazdır("başka bir hata:", h.mesaj)
    }
}
ata l = [5, 0]
işle(l, 0)
işle(l, 1)
işle(l, 9)
```

```
20
sıfıra bölünemez
9. eleman yok
```

Dalları **özelden genele** sıralayın. `yakala h: Hata` en başta olsaydı her hatayı o yakalardı, alttaki özel dallar hiç çalışmazdı. Kolay bunu fark eder ve uyarır:

```kolay
dene {
    yazdır(1 / 0)
} yakala h: Hata {
    yazdır("genel")
} yakala s: Hata.SıfıraBölme {
    yazdır("özel")
}
```

```
Uyarı (deneme.kolay, satır 5): 'yakala h: Hata' önce geldiği için 'Hata.SıfıraBölme' dalı hiç çalışmaz. Dalları özelden genele sıralayın.
  5 | } yakala s: Hata.SıfıraBölme {
    |   ^^^^^^^^
genel
```

Uyarı, programın çalışmasını engellemez; ama bir şeyi yanlış yazmış olabileceğinizi söyler.

### fırlat: kendiniz hata vermek

Kendi kodunuzda bir şey yanlışsa `fırlat` ile hata **fırlatırsınız**. Bu, Kolay'ın kendi hatalarıyla aynı şekilde çalışır: en yakın uygun `yakala`'ya gider, yoksa programı durdurur:

```kolay
fonksiyon kare_kök_tam(n: Tam(64)) -> Tam(64) {
    eğer n < 0 {
        fırlat Hata(mesaj: "Negatif sayının karekökü alınamaz.")
    }
    ata k = 0
    (k + 1) * (k + 1) <= n sürece yap { k += 1 }
    döndür k
}

dene {
    yazdır(kare_kök_tam(50))
    yazdır(kare_kök_tam(-4))
    yazdır("buraya gelinmez")
} yakala h: Hata {
    yazdır("Yakalandı:", h.mesaj)
}
yazdır(kare_kök_tam(-1))    // yakalanmıyor: program burada durur
```

```
7
Yakalandı: Negatif sayının karekökü alınamaz.
ÇALIŞMA HATASI (deneme.kolay, satır 3): Negatif sayının karekökü alınamaz.
  3 |         fırlat Hata(mesaj: "Negatif sayının karekökü alınamaz.")
```

Son satırdaki hata yakalanmadığı için program, fırlatıldığı satırı göstererek durdu. Yakalanmayan bir `Hata` nesnesi, mesajı yazılarak programı durdurur; tıpkı Kolay'ın kendi hataları gibi.

Hazır alt türler de fırlatılabilir: `fırlat Hata.İndeks(mesaj: "...")`.

Hatanın **nereden** fırlatıldığı önemli değildir. `kare_kök_tam` fonksiyonunun içinde fırlatılan hata, fonksiyonu çağıran `dene` bloğuna kadar çıktı. Fonksiyonlar iç içe çağrılmışsa hata, bir `yakala` bulana kadar hepsinin içinden yukarı doğru çıkar.

### Her şey fırlatılabilir

`fırlat` sadece hata nesneleri için değildir; **her değer** fırlatılabilir ve tipine göre yakalanır:

```kolay
fonksiyon ara(l: Liste<Yazı>, aranan: Yazı) {
    l içindeki her s için yap {
        eğer s == aranan { fırlat s.uzunluk() }
    }
    fırlat "bulunamadı"
}

dene { ara(["elma", "armut"], "armut") } yakala n: Tam(64) { yazdır("bulundu, uzunluk:", n) } yakala y: Yazı { yazdır(y) }
dene { ara(["elma", "armut"], "kiraz") } yakala n: Tam(64) { yazdır("bulundu, uzunluk:", n) } yakala y: Yazı { yazdır(y) }
fırlat 42    // yakalanmıyor: program burada durur
```

```
bulundu, uzunluk: 5
bulunamadı
ÇALIŞMA HATASI (deneme.kolay, satır 10): Yakalanmamış değer fırlatıldı: 42 (Tam(64)).
  10 | fırlat 42    // yakalanmıyor: program burada durur
```

Yakalanmayan, hata olmayan bir değer `Yakalanmamış değer fırlatıldı` mesajıyla, değeri ve tipiyle birlikte yazılır.

Bir değerin bir dala uyması için tipi o dalın tipine **atanabilir** olmalıdır: `Tam(8)` tipinde fırlatılan bir sayıyı `yakala n: Tam(64)` de yakalar, çünkü `Tam(8)` her zaman `Tam(64)`'e sığar.

### Kendi hata türleriniz

Programınıza özgü hatalar için `Hata`'dan kalıtım alan bir tür tanımlarsınız. Bu türün `mesaj` alanı kendiliğinden vardır; isterseniz ek alanlar eklersiniz:

```kolay
tür GeçersizYaş[Hata](yaş: Tam(64))
tür EksikBilgi[Hata](alan: Yazı)

fonksiyon kaydet(ad: Yazı, yaş: Tam(64)) {
    eğer ad == "" { fırlat EksikBilgi(mesaj: "Ad boş olamaz.", alan: "ad") }
    eğer yaş < 0 veya yaş > 150 { fırlat GeçersizYaş(mesaj: f"{yaş} geçerli bir yaş değil.", yaş: yaş) }
    yazdır(f"{ad} ({yaş}) kaydedildi.")
}

fonksiyon dene_kaydet(ad: Yazı, yaş: Tam(64)) {
    dene {
        kaydet(ad, yaş)
    } yakala h: GeçersizYaş {
        yazdır("Yaş hatası:", h.mesaj, f"(girilen: {h.yaş})")
    } yakala h: EksikBilgi {
        yazdır(f"'{h.alan}' alanı eksik:", h.mesaj)
    }
}

dene_kaydet("Ayşe", 21)
dene_kaydet("Can", -3)
dene_kaydet("", 30)
```

```
Ayşe (21) kaydedildi.
Yaş hatası: -3 geçerli bir yaş değil. (girilen: -3)
'ad' alanı eksik: Ad boş olamaz.
```

- `tür GeçersizYaş[Hata](yaş: Tam(64))`: "GeçersizYaş bir Hata'dır, ayrıca bir yaş alanı vardır."
- Nesne oluşturulurken önce ebeveynin alanı (`mesaj`), sonra kendi alanları verilir.
- `yakala h: Hata` bu kendi türlerinizi de yakalar, çünkü onlar da birer `Hata`'dır.

### Tipsiz yakala: geri kalan her şey

`yakala` arkasına tip yazılmazsa **önceki dalların yakalamadığı her şeyi** yakalar:

```kolay
fonksiyon riskli(n: Tam(64)) {
    eğer n == 1 { fırlat "bir yazı" }
    eğer n == 2 { fırlat 3.14 }
    yazdır(10 / (n - 3))
}
1..=3 içindeki her n için yap {
    dene {
        riskli(n)
    } yakala y: Yazı {
        yazdır("yazı yakalandı:", y)
    } yakala diğer {
        yazdır("başka bir şey:", diğer)
    }
}
```

```
yazı yakalandı: bir yazı
başka bir şey: 3.14
başka bir şey: Hata.SıfıraBölme(mesaj: "Sıfıra bölme hatası.")
```

Peki `diğer`'in tipi nedir? Kolay bütün programı gördüğü için, o blokta **fırlatılabilecek bütün değerleri** bilir. `diğer`'in tipi bunların `veya`'sıdır. Bu örnekte `riskli` bir `Yazı`, bir `Ondalık` fırlatabiliyor, ayrıca bölme hatası olabiliyor. `Yazı`'yı önceki dal yakaladığı için `diğer`'in tipi `Ondalık veya Hata` olur. Gerekirse `eşleştir` ile hangisi olduğuna bakılır (bkz. [veya tipleri ve Hiç](#19-veya-tipleri-ve-hiç)):

```kolay
fonksiyon riskli(n: Tam(64)) {
    eğer n == 2 { fırlat 3.14 }
    yazdır(10 / (n - 3))
}
2..=3 içindeki her n için yap {
    dene {
        riskli(n)
    } yakala diğer {
        eşleştir diğer {
            o: Ondalık -> yazdır("ondalık:", o * 2)
            h: Hata -> yazdır("hata:", h.mesaj)
        }
    }
}
```

```
ondalık: 6.28
hata: Sıfıra bölme hatası.
```

Çalışma hataları neredeyse her işlemde olabileceği için tipsiz dalın tipinde hep `Hata` vardır. Hata türlerinin hepsi bu tek `Hata` üyesinde toplanır.

### Tek başına dene: hataları yok saymak

`dene` bloğunun arkasına hiç `yakala` yazılmazsa, bloktaki hatalar **yok sayılır**: blok hatanın olduğu yerde durur, program sonrasından devam eder. Bu, hatayı sessizce yutmak demektir ve çoğu zaman iyi bir fikir değildir; Kolay bu yüzden uyarır:

```kolay
fonksiyon yükle(ad: Yazı) {
    eğer ad == "" { fırlat Hata(mesaj: "boş ad") }
    yazdır(f"{ad} yüklendi")
}
dene { yükle("ayarlar") }
dene { yükle("") }
yazdır("devam")
```

```
Uyarı (deneme.kolay, satır 5): 'dene' bloğundaki hatalar yok sayılıyor (satır 5: yükle() hata verebilir). Hatayı ele almak için: dene { } yakala h { }
  5 | dene { yükle("ayarlar") }
    | ^^^^^^^^^^^^^^^^^^^^^^^^^
Uyarı (deneme.kolay, satır 6): 'dene' bloğundaki hatalar yok sayılıyor (satır 6: yükle() hata verebilir). Hatayı ele almak için: dene { } yakala h { }
  6 | dene { yükle("") }
    | ^^^^^^^^^^^^^^^^^^
ayarlar yüklendi
devam
```

Gerçekten "olursa olsun" diyebileceğiniz yerlerde kullanın (örneğin isteğe bağlı bir ayarın okunması).

### Hata döndüren fonksiyonlar: T veya Hata

Bir fonksiyonun hata verebileceğini, **dönüş tipine** yazarak da söyleyebilirsiniz. `-> Tam(64) veya Hata`, "ya bir sayı ya da bir hata döndürürüm" demektir. Fonksiyon hatayı `fırlat` yerine `döndür` ile verir:

```kolay
fonksiyon böl(a: Tam(64), b: Tam(64)) -> Tam(64) veya Hata {
    eğer b == 0 { döndür Hata.SıfıraBölme(mesaj: "Bölen sıfır olamaz.") }
    döndür a / b
}

dene {
    yazdır(böl(10, 2))
    yazdır(böl(7, 0))
    yazdır("buraya gelinmez")
} yakala h: Hata.SıfıraBölme {
    yazdır("yakalandı:", h.mesaj)
}
```

```
5
yakalandı: Bölen sıfır olamaz.
```

Böyle bir fonksiyon **`dene` içinde** çağrılınca, dönen değer `Hata` ise kendiliğinden fırlatılır; değilse sonuç doğrudan bir `Tam(64)` olarak kullanılır. `yazdır(böl(10, 2))` bu yüzden `5` yazdı.

`dene` **dışında** ise sonuç gerçekten `Tam(64) veya Hata`'dır ve bir sayı gibi kullanılamaz; Kolay bunu program çalışmadan yakalar:

```kolay
fonksiyon böl(a: Tam(64), b: Tam(64)) -> Tam(64) veya Hata {
    eğer b == 0 { döndür Hata.SıfıraBölme(mesaj: "Bölen sıfır olamaz.") }
    döndür a / b
}
ata x = böl(10, 2) + 1    // ✗ HATA
```

```
HATA (deneme.kolay, satır 5): 'böl()' hata verebilir (Tam(64) veya Hata). Bir 'dene { } yakala' bloğu içinde kullanın ya da eşleştir ile kontrol edin.
  5 | ata x = böl(10, 2) + 1    // ✗ HATA
    |         ^^^^^^^^^^
```

Hata verebilecek bir şeyi, onu ele almadan kullanmak mümkün değildir. İki yol var: ya bir `dene` bloğuna almak ya da sonucu `eşleştir` ile kontrol etmek:

```kolay
fonksiyon böl(a: Tam(64), b: Tam(64)) -> Tam(64) veya Hata {
    eğer b == 0 { döndür Hata.SıfıraBölme(mesaj: "Bölen sıfır olamaz.") }
    döndür a / b
}
[2, 0] içindeki her b için yap {
    eşleştir böl(10, b) {
        n: Tam(64) -> yazdır("sonuç:", n)
        h: Hata -> yazdır("hata:", h.mesaj)
    }
}
```

```
sonuç: 5
hata: Bölen sıfır olamaz.
```

`n: Tam(64) ->` dalı "değer bir `Tam(64)` ise onu `n` adıyla al" demektir (bu yazım [Seçenekler ve eşleştir](#18-seçenekler-ve-eşleştir) bölümünde de anlatılıyor). Eski yazım `Tam(64)(n) ->` yerine daha okunaklıdır, ikisi de geçerlidir.

Bir değişkenin tipine açıkça `veya Hata` yazarsanız, `dene` içinde bile hata kendiliğinden fırlatılmaz, değer olarak saklanır:

```kolay
fonksiyon böl(a: Tam(64), b: Tam(64)) -> Tam(64) veya Hata {
    eğer b == 0 { döndür Hata.SıfıraBölme(mesaj: "Bölen sıfır olamaz.") }
    döndür a / b
}
dene {
    ata sonuç: Tam(64) veya Hata = böl(1, 0)
    yazdır("hata değer olarak saklandı:", sonuç)
} yakala h {
    yazdır("buraya gelinmez")
}
```

```
hata değer olarak saklandı: Hata.SıfıraBölme(mesaj: "Bölen sıfır olamaz.")
```

#### fırlat mı, döndür mü?

| | `fırlat` | `-> T veya Hata` ve `döndür Hata(...)` |
|---|---|---|
| Fonksiyonun imzası | Değişmez | Hata verebileceği imzada yazılı |
| Çağıran ne yapmalı | Bir yerde `dene`/`yakala` (yoksa program durur) | `dene` içinde çağırmalı ya da `eşleştir` ile kontrol etmeli; unutursa derleme hatası |
| Ne zaman | Beklenmeyen, nadir durumlar | Çağıranın mutlaka düşünmesi gereken durumlar (kullanıcı girdisi, arama) |

### Hazır metotlar: tama() ve ondalığa()

Yazıyı sayıya çeviren iki metot, başarısız olabildikleri için tam da bu şekilde çalışır. `"42".tama()` sonucunun tipi `Tam(64)` değil, `Tam(64) veya Hata.Dönüşüm`'dür:

```kolay
yazdır("42".tama(), "abc".tama(), "3,5".ondalığa())
```

```
42 Hata.Dönüşüm(mesaj: "\"abc\" bir tam sayıya çevrilemez.") 3.5
```

Yazdırmak serbesttir; ama sayı gibi kullanmak için `dene` ya da `eşleştir` gerekir:

```kolay
["12", "x", " 7 "] içindeki her s için yap {
    dene {
        yazdır(s.tama() * 2)
    } yakala h: Hata.Dönüşüm {
        yazdır(h.mesaj)
    }
}
```

```
24
"x" bir tam sayıya çevrilemez.
14
```

(`tama()` baştaki ve sondaki boşlukları yok sayar; `ondalığa()` ondalık ayırıcı olarak virgülü de kabul eder.)

### Akış: döngüler, döndür, dur, devam

`dene` bloğu, içinde bulunduğu fonksiyonun ve döngünün bir parçasıdır. İçinden `döndür`, `dur` ve `devam` normal şekilde çalışır:

```kolay
fonksiyon ilk_sayı(l: Liste<Yazı>) -> Tam(64) veya Hiç {
    l içindeki her s için yap {
        dene {
            döndür s.tama()      // ilk sayıya çevrilebilen yazıda fonksiyondan çık
        } yakala h {
            devam                // çevrilemezse sonrakine geç
        }
    }
    döndür Hiç
}
yazdır(ilk_sayı(["a", "b", "17", "20"]), ilk_sayı(["x"]))
```

```
17 Hiç
```

Hata olduğunda, `dene` bloğunda **o ana kadar** yapılan değişiklikler kalır, geri alınmaz:

```kolay
ata toplam = 0
ata l: Liste<Tam(64)>
dene {
    1..=10 içindeki her i için yap {
        eğer i == 5 { fırlat i }
        toplam += i
        l.ekle(i)
    }
} yakala k: Tam(64) {
    yazdır(f"{k}. turda durdu; toplam {toplam}, liste {l}")
}
```

```
5. turda durdu; toplam 10, liste [1, 2, 3, 4]
```

### İç içe dene ve yeniden fırlatmak

`dene` blokları iç içe yazılabilir. İçteki bloğun hiçbir dalına uymayan hata dıştaki bloğa gider (Kolay burada içteki dalın hiç çalışmayacağını da fark edip uyarıyor):

```kolay
dene {
    dene {
        fırlat "iç bloktan"
    } yakala n: Tam(64) {
        yazdır("buraya gelinmez: sayı değil")
    }
} yakala y: Yazı {
    yazdır("dıştaki blok yakaladı:", y)
}
```

```
Uyarı (deneme.kolay, satır 4): Bu blokta Tam(64) fırlatılamaz, 'yakala n: Tam(64)' dalı hiç çalışmaz.
  4 |     } yakala n: Tam(64) {
    |       ^^^^^^^^
dıştaki blok yakaladı: iç bloktan
```

Bir yakala dalının içinden de `fırlat` yazılabilir; örneğin bir hatayı yakalayıp daha anlamlı bir hataya çevirmek için:

```kolay
tür AyarHatası[Hata](ayar: Yazı)
fonksiyon ayar_oku(ad: Yazı, değer: Yazı) -> Tam(64) {
    dene {
        döndür değer.tama()
    } yakala h: Hata.Dönüşüm {
        fırlat AyarHatası(mesaj: f"'{ad}' ayarı sayı olmalı, '{değer}' yazılmış.", ayar: ad)
    }
}
dene {
    yazdır(ayar_oku("genişlik", "800"))
    yazdır(ayar_oku("yükseklik", "çok"))
} yakala h: AyarHatası {
    yazdır(h.mesaj)
}
```

```
800
'yükseklik' ayarı sayı olmalı, 'çok' yazılmış.
```

### Değişkensiz yakala

Hatanın kendisiyle işiniz yoksa değişken adı yerine `_` yazabilirsiniz:

```kolay
ata sözlük: Sözlük<Yazı, Tam(64)> = {"a": 1}
dene { yazdır(sözlük["b"]) } yakala _: Hata.Anahtar { yazdır("anahtar yok") }
```

```
anahtar yok
```

### Uyarılar

Kolay, `dene` bloklarıyla ilgili muhtemel yanlışlıkları uyarıyla bildirir:

| Uyarı | Anlamı |
|---|---|
| `'dene' bloğundaki hatalar yok sayılıyor (satır 3: yükle() hata verebilir).` | `yakala`'sız `dene`: hatalar sessizce yutuluyor |
| `'yakala h: Hata' önce geldiği için 'Hata.SıfıraBölme' dalı hiç çalışmaz.` | Dal sırası yanlış: genel dal özel daldan önce |
| `Bu blokta Ondalık fırlatılamaz, 'yakala o: Ondalık' dalı hiç çalışmaz.` | Blokta o tipte bir şey fırlatılmıyor; dal gereksiz ya da yanlış tip yazılmış |

```kolay
dene {
    yazdır(10 / 2)
} yakala o: Ondalık {
    yazdır(o)
}
```

```
Uyarı (deneme.kolay, satır 3): Bu blokta Ondalık fırlatılamaz, 'yakala o: Ondalık' dalı hiç çalışmaz.
  3 | } yakala o: Ondalık {
    |   ^^^^^^^^
5
```

### Yakalanamayan durumlar

Birkaç durum `dene` ile yakalanamaz; bunlar programın kendisindeki ciddi sorunlardır ve program her zaman durur:

- **Çok derin özyineleme**: bir fonksiyon kendini durmadan çağırdı (durma koşulu unutuldu).
- **Ctrl+C**: kullanıcı programı durdurdu.
- **Bellek yetmedi.**
- **Değişken henüz belirsiz**: bir fonksiyon, kullandığı bir dosya değişkeni tanımlanmadan önce çağrıldı.

### Hata yönetimi hataları

| Mesaj | Sebep | Çözüm |
|---|---|---|
| `'böl()' hata verebilir (Tam(64) veya Hata). Bir 'dene { } yakala' bloğu içinde kullanın ya da eşleştir ile kontrol edin.` | `T veya Hata` döndüren bir fonksiyonun sonucu `dene` dışında `T` gibi kullanıldı | `dene` ya da `eşleştir` |
| `'yakala' sonrasında önce bir değişken adı yazılır: yakala h: Hata.Dönüşüm { }` | `yakala Hata.Dönüşüm { }` yazıldı | Ad ekleyin: `yakala h: Hata.Dönüşüm` |
| `'yakala' sadece bir 'dene { }' bloğundan sonra yazılabilir.` | `dene` olmadan `yakala` | Önce `dene { }` |
| `'Hata' içinde 'Bilinmeyen' diye bir hata türü yok. Hazır hata türleri: SıfıraBölme, İndeks, Anahtar, Taşma, Dönüşüm.` | Yanlış hata türü adı | Tablodaki adlardan birini yazın ya da kendi türünüzü tanımlayın |
| `Hiç fırlatılamaz.` | `fırlat Hiç` | Bir değer ya da hata fırlatın |

### Python'daki karşılığı

Python'da aynı yapı `try` / `except` / `raise` ile kurulur:

```python
try:
    yaş = int(girdi)
except ValueError as h:
    print("Sayı yazın:", h)
```

Farkları:

- Python, bir fonksiyonun hangi hataları verebileceğini bilmez; `except` dalının çalışıp çalışmayacağı ancak program çalışırken belli olur. Kolay, hiç çalışmayacak dalları önceden söyler.
- Python'da yalnızca `Exception` türünden nesneler fırlatılabilir; Kolay'da her değer.
- Python'da `int("abc")` hata **fırlatır**; Kolay'da `"abc".tama()` hatayı **döndürür**, `dene` içinde ise fırlatılmış sayılır. Sonucu kontrol etmeyi unutmak Kolay'da derleme hatasıdır.

### Alıştırmalar

1. Kullanıcıdan (bir listeden, `oku` kullanmadan) gelen `["3", "8", "x", "-2", "10"]` yazılarını sayıya çevirip toplayın. Çevrilemeyenleri atlayın ve kaç tanesinin atlandığını yazdırın.
2. `fonksiyon karekök(x: Ondalık) -> Ondalık veya Hata` yazın: negatif sayıda `Hata` döndürsün. (Karekök için Newton yöntemi: `t = x; 20 kere yap { t = (t + x / t) / 2 }`.) Sonucu `eşleştir` ile yazdırın.
3. `tür BakiyeYetersiz[Hata](eksik: Ondalık)` tanımlayın. `Hesap` türünün `çek(m)` metodu bakiye yetmiyorsa bu hatayı fırlatsın. Hatayı yakalayıp ne kadar eksik olduğunu yazdırın.
4. Bir sözlükte aranan anahtarları `dene`/`yakala` ile bulun; olmayanlar için `Hata.Anahtar` yakalayıp "yok" yazdırın. Aynısını `içerir` ile yazın. Hangisi daha açık?

<details>
<summary>Çözümler</summary>

```kolay
// 1
ata yazılar = ["3", "8", "x", "-2", "10"]
ata toplam = 0
ata atlanan = 0
yazılar içindeki her y için yap {
    dene {
        toplam += y.tama()
    } yakala _: Hata.Dönüşüm {
        atlanan += 1
    }
}
yazdır(f"Toplam: {toplam}, atlanan: {atlanan}")   // Toplam: 19, atlanan: 1

// 2
fonksiyon karekök(x: Ondalık) -> Ondalık veya Hata {
    eğer x < 0.0 { döndür Hata(mesaj: f"{x} negatif, karekökü yok.") }
    eğer x == 0.0 { döndür 0.0 }
    ata t = x
    20 kere yap { t = (t + x / t) / 2 }
    döndür t
}
[2.0, -9.0] içindeki her x için yap {
    eşleştir karekök(x) {
        k: Ondalık -> yazdır(k)
        h: Hata -> yazdır(h.mesaj)
    }
}

// 3
tür BakiyeYetersiz[Hata](eksik: Ondalık)
tür Hesap(bakiye: Ondalık) {
    fonksiyon çek(m: Ondalık) {
        eğer m > bakiye { fırlat BakiyeYetersiz(mesaj: "Bakiye yetersiz.", eksik: m - bakiye) }
        bakiye -= m
    }
}
ata h = Hesap(100.0)
dene {
    h.çek(30.0)
    h.çek(200.0)
} yakala e: BakiyeYetersiz {
    yazdır(e.mesaj, f"{e.eksik} TL eksik.")
}
yazdır(h.bakiye)   // 70.0: ilk çekim yapıldı

// 4
ata telefon: Sözlük<Yazı, Yazı> = {"Ali": "0532"}
["Ali", "Veli"] içindeki her ad için yap {
    dene { yazdır(ad, telefon[ad]) } yakala _: Hata.Anahtar { yazdır(ad, "yok") }
}
["Ali", "Veli"] içindeki her ad için yap {
    eğer telefon.içerir(ad) { yazdır(ad, telefon[ad]) } değilse { yazdır(ad, "yok") }
}
// İkincisi daha açık: olmaması beklenen bir durum için eğer yeterli.
```

</details>

---

## 22. Birden fazla dosya: dahil et

Program büyüdükçe kodu dosyalara bölmek gerekir. `dahil et` başka bir `.kolay` dosyasını programa ekler.

`geometri.kolay`:

```
fonksiyon daire_alanı(r: Ondalık) -> Ondalık {
    döndür 3.14159 * r * r
}

tür Nokta(x: Tam, y: Tam) {
    fonksiyon yazıya_dök() -> Yazı { döndür f"({x}, {y})" }
}
```

`ana.kolay`:

```
dahil et("./geometri.kolay")

yazdır(daire_alanı(2.0))
ata n = Nokta(3, 4)
yazdır(n.yazıya_dök())
```

İsim çakışmalarını önlemek için dosyaya bir isim verilebilir. O zaman içindekilere bu isimle erişilir:

```
g adıyla dahil et("./geometri.kolay")

yazdır(g.daire_alanı(2.0))
ata n = g.Nokta(3, 4)
```

Kurallar:

- Dosya yolu, `dahil et` yazan dosyanın bulunduğu klasöre göredir.
- Her dosya yalnızca bir kez eklenir, iki kez dahil etmek sorun çıkarmaz.
- Dahil edilen dosyadaki `yazdır` gibi komutlar, dosya dahil edildiği anda bir kez çalışır.
- İsimsiz dahil etmede iki dosyada aynı isim varsa Kolay hata verir ve isimli dahil etmeyi önerir.

---

## 23. Hata mesajlarını okumak

Hata almak programcılığın olağan bir parçasıdır. Önemli olan hatayı okuyabilmektir. Kolay'da iki çeşit hata vardır.

### Derleme hatası (HATA)

Program **çalışmadan önce** bulunan hatalar: yazım yanlışları, tip uyuşmazlıkları, tanımlanmamış isimler. Program hiç başlamaz:

```
HATA (deneme.kolay, satır 2): Yazı ile Tam(64) toplanamaz. f-yazı kullanın: f"{a}{b}"
  2 | yazdır(ad + 5)
    |        ^^^^^^
```

Okuma sırası:

1. **Nerede?** `deneme.kolay, satır 2`
2. **Ne oldu?** Yazı ile sayı toplanmaya çalışılmış.
3. **Ne yapmalı?** Mesajın sonunda öneri var: f-yazı kullanın.
4. `^^^^^^` işaretleri hatalı parçanın tam yerini gösterir.

### Çalışma hatası (ÇALIŞMA HATASI)

Program **çalışırken** ortaya çıkan hatalar: sıfıra bölme, taşma, olmayan indeks, olmayan sözlük anahtarı. Program o satıra kadar çalışır, sonra durur:

```
ÇALIŞMA HATASI (deneme.kolay, satır 2): "Veli" anahtarı sözlükte yok.
  2 | yazdır(d["Veli"])
```

Çalışma hataları bir `dene` bloğu içinde olurlarsa programı durdurmaz, `yakala` ile ele alınabilirler (bkz. [Hata yönetimi](#21-hata-yönetimi-dene-yakala-fırlat)). Bu durumda yukarıdaki mesaj, yakalanan hata nesnesinin `mesaj` alanı olur.

### Uyarı

Uyarı bir hata değildir: program yine çalışır. Kolay, yanlış yazılmış olabilecek bir şey gördüğünde uyarır:

```
Uyarı (deneme.kolay, satır 2): '^' özel veya (xor) işlemidir; üs için '**' yazın.
  2 | yazdır(x ^ 2)
    |        ^^^^^
```

| Uyarı | Anlamı |
|---|---|
| `'^' özel veya (xor) işlemidir; üs için '**' yazın.` | Üs alındığı sanılabilecek bir xor |
| `'Sarı', 'Yeşil' seçenekleri ele alınmamış.` | `eşleştir`'de bazı seçenekler için dal yok |
| `'dene' bloğundaki hatalar yok sayılıyor ...` | `yakala`'sız `dene` |
| `'yakala h: Hata' önce geldiği için ... dalı hiç çalışmaz.` | `yakala` dallarının sırası yanlış |
| `Bu blokta Ondalık fırlatılamaz, ... dalı hiç çalışmaz.` | Hiç çalışmayacak bir `yakala` dalı |

### Sık görülen hatalar

| Mesaj | Sebep | Çözüm |
|---|---|---|
| `'yazdir' tanımlı değil. 'yazdır' mı demek istediniz?` | Türkçe karakter unutuldu | `yazdır` yazın ya da dosyanın başına `klavye("us")` ekleyin |
| `'ondalık' diye bir tip yok. 'Ondalık' mı demek istediniz?` | Tip adı küçük harfle yazıldı | Tipler büyük harfle başlar |
| `'x' Tam(64) tipinde, Yazı atanamaz.` | Değişkene başka tipte değer verildi | Doğru tipte değer verin ya da `x: Yazı = ...` yazın |
| `'x' henüz belirsiz.` | Değer verilmeden kullanıldı | Önce değer atayın |
| `'sürece' bir Mantık (doğru/yanlış) değeri bekler` | `5 sürece yap` yazıldı | `5 kere yap` yazın |
| `... döndürmeli, ama fonksiyonun sonuna bir değer döndürmeden ulaşılabiliyor.` | Bir yolda `döndür` unutuldu | Her yolda `döndür` yazın |
| `Tam(8) taştı.` | Sayı tipinin sınırı aşıldı | Daha büyük tip ya da esnek `Tam` kullanın |
| `Listenin 5. indeksine erişilmek istendi, ama liste 3 elemanlı` | Olmayan eleman | İndeksin `0 .. uzunluk - 1` aralığında olduğunu kontrol edin |
| `Tam(64) değeri Tam(8) tipine otomatik çevrilemez` | Büyükten küçüğe geçiş | `olarak Tam(8)` ile bilerek çevirin |
| `Tip uyuşmuyor: Liste<Tam> bekleniyordu, ama Liste<Tam(64)> verildi.` | Esnek `Tam` ile `Tam(64)` farklı | Listeyi `Liste<Tam>` olarak tanımlayın ya da parametreyi `Liste<Tam(64)>` yapın |
| `'tama()' hata verebilir (Tam(64) veya Hata.Dönüşüm). Bir 'dene { } yakala' bloğu içinde kullanın ...` | Yazıdan sayıya çevirmenin sonucu kontrol edilmeden kullanıldı | `dene { } yakala` ya da `eşleştir` |
| `Bu değer Hiç olabilir (Yazı veya Hiç). Kullanmadan önce eşleştir ile kontrol edin.` | `Hiç` olabilecek değer doğrudan kullanıldı | `eşleştir` ile kontrol edin |
| `'ters' yerine 'ters_çevir()' (listeyi değiştirir) ya da 'tersi()' (yeni liste verir) yazın.` | Eski metot adı | Metot adı kuralına uygun adı yazın |
| `'x' parametresinin tipi belli değil. Tipini yazın: her (x: Tam(64)) için ...` | Bağlamı olmayan lambda | Parametrenin tipini yazın |
| `Adım sıfır olamaz.` | `adım(0)` | Sıfırdan farklı bir adım verin |
| `Aralık çok büyük (... eleman), liste olarak oluşturulamaz.` | Dev bir aralık değer olarak kullanıldı | Aralığı doğrudan döngüde kullanın |
| `Çok derin özyineleme: bir fonksiyon kendini çok fazla çağırdı.` | Durma koşulu olmayan özyineleme | Durma koşulunu kontrol edin |
| `'dondur' us klavye modunda bir anahtar kelime ('döndür'). Çevirmeden önce bu adı değiştirin.` | `kolay çevir` ile çevrilecek dosyada, diğer modun anahtar kelimesiyle aynı bir isim var | O ismi değiştirin |

### Hata ayıklama ipuçları

- İlk hatayı düzeltin; bazen sonraki hatalar ilkinin sonucudur.
- Programın bir yerinde ne olduğunu görmek için ara değerleri `yazdır` ile yazdırın.
- Büyük bir programı küçük parçalarla deneyin; her parça çalışınca bir sonrakine geçin.
- `kolay denetle dosya.kolay` çalıştırmadan hataları gösterir.

---

## 24. Türkçe karakter olmadan yazmak

Türkçe klavyeniz yoksa, dosyanın **ilk satırına** `klavye("us")` yazarak dilin kelimelerini Türkçe karakter olmadan yazabilirsiniz:

```kolay
klavye("us")

tur Kisi(ad: Yazi, yas: Tam) {
    fonksiyon tanit() -> Yazi {
        dondur f"Ben {ad}, {yas} yasindayim"
    }
}

ata k = Kisi("Ali", 20)
eger k.yas >= 18 {
    yazdir(k.tanit())
} degilse {
    yazdir("cocuk")
}
```

- Bu modda yalnızca dilin kendi kelimeleri değişir: `eğer` → `eger`, `yazdır` → `yazdir`, `Yazı` → `Yazi`.
- Kendi isimleriniz için Türkçe harf kullanmak yine serbesttir.
- Bir dosyada iki yazım karıştırılamaz.
- Her dosya kendi modunu seçer; `us` modunda yazılmış bir dosya `tr` modundaki bir dosyayı dahil edebilir.
- `kolay çevir dosya.kolay --klavye us` bir dosyayı otomatik olarak diğer moda çevirir. `tr` modunda `dondur` gibi bir adı değişken olarak kullandıysanız, bu ad `us` modunda anahtar kelime olduğu için çeviri yapılmaz: `'dondur' us klavye modunda bir anahtar kelime ('döndür'). Çevirmeden önce bu adı değiştirin.`

---

## 25. Hız: derlemek

`kolay dosya.kolay` programı hemen çalıştırır. Hızı Python'a yakındır ve öğrenmek için fazlasıyla yeterlidir.

Hız gerektiğinde program **derlenir**:

```
kolay derle dosya.kolay
./dosya
```

`derle` programı makine koduna çevirir (LLVM ile). Ortaya çıkan program C ile yazılmış bir programa yakın hızda çalışır. Aynı bilgisayarda ölçülen süreler (saniye):

| Program | `kolay derle` | C | Python | `kolay` |
|---|---|---|---|---|
| fib(35) | 0,04 | 0,02 | 1,07 | 1,4 |
| 20 milyona kadar asal sayılar (Eratosten eleği) | 0,40 | 0,28 | 3,0 | 4,6 |
| 1000 nesne × 3000 adım | 0,02 | 0,004 | 0,54 | 1,8 |
| Sözlükte 2 milyon kelime saymak | 0,20 | – | 0,62 | 0,87 |
| Esnek `Tam` ile büyük sayılar | 0,02 | – | 0,60 | 0,45 |

Derlenmiş program ile `kolay dosya.kolay` aynı çıktıyı, aynı hata mesajlarını verir. Fark yalnızca hızdadır. `dene`/`yakala` ile yakalanan hatalar da iki yolda aynı şekilde çalışır.

Derlemenin bir bedeli vardır: derleme birkaç saniye sürebilir (ilk derlemede Kolay'ın çalışma kütüphanesi de bir kez derlenip saklanır). Bu yüzden program yazıp denerken `kolay dosya.kolay`, bitmiş ve hız gereken programlar için `kolay derle` kullanmak en iyisidir.

Derlenmiş program `yazdır` çıktısını hız için biriktirir; çıktı ekrana gidiyorsa her satır hemen görünür, bir dosyaya yönlendirilmişse (`./dosya > sonuç.txt`) toplu yazılır.

Hız ipuçları:

- Sayı tipini yazmamak (`Tam(64)`) en hızlısıdır. Esnek `Tam` biraz daha yavaştır.
- Boyutu belli listelerde `Liste<Tam>[N]` daha hızlıdır.
- `0..n içindeki her i için yap` döngüsü liste oluşturmaz; aralığı bir değişkene atayıp onun üzerinde dönmek bir liste oluşturur.
- Kolay'da çöp toplayıcı yoktur: kullanılmayan bellek hemen geri verilir, program arada bir duraklamaz.
- Listeler ve nesneler kopyalanırken gerçekten kopyalanmaz; kopya ancak biri değiştirildiğinde yapılır ("yazınca kopyala"). Değer semantiği bu yüzden yavaş değildir.
- `dene` bloğuna girmenin küçük bir maliyeti vardır; hata olmadığı sürece başka bir maliyeti yoktur. Çok sık tekrarlanan bir döngünün içindeki `dene`'yi döngünün dışına almak hızlandırabilir.
- Sonsuz özyineleme derlenmiş programda da Türkçe "Çok derin özyineleme" hatasıyla durur; ama hatanın gösterdiği satır, hatalı çağrının değil fonksiyonun tanımlandığı satırdır.

---

## 26. C kütüphanelerini kullanmak

Dünyadaki hazır kütüphanelerin çoğu C ile yazılmıştır ya da C'den çağrılabilir: matematik fonksiyonları, grafik, ses, pencere, bildirim... Kolay bu kütüphanelerdeki fonksiyonları doğrudan çağırabilir.

> **Dikkat:** C fonksiyonlarının imzasını Kolay kontrol edemez. Yanlış yazılan bir imza programı çökertebilir. Bu yüzden tanımın önüne `#Güvensiz` yazmak zorunludur. Bu bölüm ileri seviyedir.

### Matematik kütüphanesi

```kolay
#Güvensiz
dış kütüphane("m") {
    fonksiyon cos(x: Ondalık) -> Ondalık
    fonksiyon karekök(x: Ondalık) -> Ondalık = "sqrt"
}

yazdır(cos(0.0))
yazdır(karekök(2.0))
```

```
1.0
1.4142135623730951
```

- `"m"` kütüphanenin adıdır (`libm.so`).
- `= "sqrt"` C'deki asıl adı belirtir; böylece fonksiyona Kolay içinde Türkçe bir ad verebilirsiniz.

İsimli yazım (`dahil et`'teki gibi):

```kolay
#Güvensiz
mat adıyla dış kütüphane("m") {
    fonksiyon üs(a: Ondalık, b: Ondalık) -> Ondalık = "pow"
    fonksiyon taban(x: Ondalık) -> Ondalık = "floor"
}

yazdır(mat.üs(2.0, 10.0), mat.taban(3.7))
```

Tek bir fonksiyon için kısa yazım:

```kolay
#Güvensiz
"c" kütüphanesindeki fonksiyon uzunluk(s: Yazı) -> Tam(64) = "strlen"

yazdır(uzunluk("merhaba"))
```

### Rastgele sayı

Kolay'da hazır bir rastgele sayı fonksiyonu yok, ama C'ninki kullanılabilir:

```kolay
#Güvensiz
dış kütüphane("c") {
    fonksiyon rastgele() -> Tam(32) = "rand"
    fonksiyon tohum(t: Tam(32, +)) = "srand"
    fonksiyon zaman(p: Tam(64)) -> Tam(64) = "time"
}

tohum(zaman(0) olarak Tam(32, +))   // her çalıştırmada farklı sayılar için
5 kere yap {
    yazdır(rastgele() % 6 + 1)      // zar at
}
```

### Tip eşleşmeleri

| Kolay | C |
|---|---|
| `Tam(8)`, `Tam(16)`, `Tam(32)`, `Tam(64)` | `int8_t` … `int64_t` |
| `Tam(N, +)` | `uint8_t` … `uint64_t` |
| `Ondalık(32)` | `float` |
| `Ondalık` | `double` |
| `Mantık` | `bool` |
| `Yazı` | `const char*` |

Listeler, sözlükler, nesneler ve esnek `Tam` C'ye verilemez. C'deki işaretçiler (pointer) `Tam(64)` olarak tutulabilir; `0` boş işaretçi (`NULL`) demektir.

---

## 27. Projeler

Öğrendiklerinizi birleştirmenin en iyi yolu küçük ama tam programlar yazmaktır. Önce kendiniz deneyin, takılırsanız çözüme bakın.

### Proje 1: Sayı tahmin oyunu

Bilgisayar 1–100 arasında bir sayı tutar; kullanıcı tahmin eder, program "büyük" ya da "küçük" der. Kaç denemede bulunduğunu sayın.

<details>
<summary>Çözüm</summary>

```kolay
#Güvensiz
dış kütüphane("c") {
    fonksiyon rastgele() -> Tam(32) = "rand"
    fonksiyon tohum(t: Tam(32, +)) = "srand"
    fonksiyon zaman(p: Tam(64)) -> Tam(64) = "time"
}

tohum(zaman(0) olarak Tam(32, +))
ata tutulan = rastgele() % 100 + 1
ata deneme = 0

yazdır("1 ile 100 arasında bir sayı tuttum.")
doğru sürece yap {
    ata girdi = oku("Tahmininiz: ").kırp()
    eğer girdi == "" {
        yazdır("Oyun bitti.")
        dur
    }
    dene {
        ata tahmin = girdi.tama()
        deneme += 1
        eğer tahmin < tutulan {
            yazdır("Daha büyük")
        } değilse eğer tahmin > tutulan {
            yazdır("Daha küçük")
        } değilse {
            yazdır(f"Bildiniz! {deneme} denemede buldunuz.")
            dur
        }
    } yakala h: Hata.Dönüşüm {
        yazdır("Lütfen bir sayı yazın.")
    }
}
```

</details>

### Proje 2: Not defteri

Öğrencilerin notlarını tutan bir program: öğrenci ekleme, not ekleme, ortalama ve harf notu hesaplama, sınıf listesi.

<details>
<summary>Çözüm</summary>

```kolay
tür Öğrenci(ad: Yazı, notlar: Liste<Tam>) {
    fonksiyon not_ekle(n: Tam) {
        notlar.ekle(n)
    }

    fonksiyon ortalama() -> Ondalık {
        eğer notlar.uzunluk() == 0 { döndür 0.0 }
        ata toplam: Tam = 0
        notlar içindeki her n için yap { toplam += n }
        döndür toplam olarak Ondalık / notlar.uzunluk()
    }

    fonksiyon harf() -> Yazı {
        ata o = ortalama()
        eğer o >= 85.0 { döndür "AA" }
        eğer o >= 70.0 { döndür "BA" }
        eğer o >= 50.0 { döndür "CC" }
        döndür "FF"
    }
}

tür Sınıf(ad: Yazı, öğrenciler: Sözlük<Yazı, Öğrenci>) {
    fonksiyon kaydet(ad: Yazı) {
        öğrenciler[ad] = Öğrenci(ad, [])
    }

    fonksiyon not_ver(ad: Yazı, n: Tam) {
        eğer değil öğrenciler.içerir(ad) {
            yazdır(f"{ad} bu sınıfta yok")
            döndür
        }
        öğrenciler[ad].not_ekle(n)
    }

    fonksiyon karne() {
        yazdır(f"--- {ad} ---")
        öğrenciler içindeki her (isim, ö) için yap {
            yazdır(f"{isim}: {ö.notlar} ortalama {ö.ortalama()} ({ö.harf()})")
        }
    }
}

ata sınıf = Sınıf("9-A", {})
sınıf.kaydet("Ali")
sınıf.kaydet("Ayşe")
sınıf.not_ver("Ali", 70)
sınıf.not_ver("Ali", 85)
sınıf.not_ver("Ayşe", 95)
sınıf.not_ver("Ayşe", 88)
sınıf.not_ver("Can", 50)
sınıf.karne()
```

</details>

### Proje 3: Kelime istatistikleri

Bir metindeki toplam kelime sayısını, farklı kelime sayısını ve en sık geçen 3 kelimeyi bulun.

<details>
<summary>Çözüm</summary>

```kolay
ata metin = "bir varmış bir yokmuş evvel zaman içinde kalbur saman içinde bir padişah varmış"

ata sayılar: Sözlük<Yazı, Tam>
ata toplam = 0
metin.küçük_harf().böl(" ") içindeki her k için yap {
    eğer k == "" { devam }
    toplam += 1
    eğer sayılar.içerir(k) { sayılar[k] += 1 } değilse { sayılar[k] = 1 }
}

yazdır(f"Toplam kelime: {toplam}")
yazdır(f"Farklı kelime: {sayılar.uzunluk()}")

// en sık 3 kelime: her seferinde en büyüğü bulup çıkar
yazdır("En sık geçenler:")
3 kere yap {
    ata en_kelime = ""
    ata en_sayı: Tam = 0
    sayılar içindeki her (k, s) için yap {
        eğer s > en_sayı {
            en_kelime = k
            en_sayı = s
        }
    }
    eğer en_sayı == 0 { dur }
    yazdır(f"  {en_kelime}: {en_sayı}")
    sayılar.sil(en_kelime)
}
```

</details>

### Proje 4: Banka simülasyonu

Farklı hesap türleri olan bir banka: `Hesap` ebeveyn türü; `VadesizHesap` (eksi bakiyeye belli bir sınıra kadar izin verir) ve `BirikimHesabı` (faiz ekler) alt türleri. Tüm hesapları tek listede tutup ay sonu işlemlerini yapın.

<details>
<summary>Çözüm</summary>

```kolay
tür Hesap(sahip: Yazı, bakiye: Ondalık) {
    fonksiyon yatır(m: Ondalık) { bakiye += m }

    fonksiyon çek(m: Ondalık) -> Mantık {
        eğer m > bakiye {
            yazdır(f"{sahip}: yetersiz bakiye")
            döndür yanlış
        }
        bakiye -= m
        döndür doğru
    }

    fonksiyon ay_sonu() { }

    fonksiyon özet() -> Yazı { döndür f"{sahip}: {bakiye} TL" }
}

tür VadesizHesap[Hesap](ek_hesap: Ondalık) {
    yeniden çek(m: Ondalık) -> Mantık {
        eğer m > bakiye + ek_hesap {
            yazdır(f"{sahip}: ek hesap sınırı aşıldı")
            döndür yanlış
        }
        bakiye -= m
        döndür doğru
    }
}

tür BirikimHesabı[Hesap](faiz: Ondalık) {
    yeniden ay_sonu() {
        bakiye += bakiye * faiz / 100
    }
}

ata hesaplar: Liste<Hesap> = [
    VadesizHesap(sahip: "Ali", bakiye: 100.0, ek_hesap: 500.0),
    BirikimHesabı(sahip: "Ayşe", bakiye: 1000.0, faiz: 4.0),
    Hesap(sahip: "Can", bakiye: 50.0),
]

hesaplar[0].çek(400.0)
hesaplar[2].çek(400.0)
hesaplar[1].yatır(500.0)

hesaplar içindeki her h için yap { h.ay_sonu() }
hesaplar içindeki her h için yap { yazdır(h.özet()) }
```

Not: `hesaplar[0].çek(...)` listedeki nesneyi yerinde değiştirir; ama `ata h = hesaplar[0]` diye bir kopya alıp onu değiştirseydik listedeki hesap değişmezdi (değer semantiği).

</details>

### Proje 5: Tic-tac-toe (XOX)

3x3 tahta, iki oyuncu sırayla satır ve sütun girer. Kazananı ya da beraberliği bulun. (İpucu: `Liste<Yazı>[3][3]`, kazanma kontrolü için 8 çizgi.)

<details>
<summary>Çözüm</summary>

```kolay
ata tahta: Liste<Yazı>[3][3]
0..3 içindeki her i için yap {
    0..3 içindeki her j için yap { tahta[i][j] = "." }
}

fonksiyon göster(t: Liste<Yazı>[3][3]) {
    t içindeki her satır için yap {
        yazdır(satır[0], satır[1], satır[2])
    }
}

fonksiyon kazandı_mı(t: Liste<Yazı>[3][3], o: Yazı) -> Mantık {
    0..3 içindeki her i için yap {
        eğer t[i][0] == o ve t[i][1] == o ve t[i][2] == o { döndür doğru }
        eğer t[0][i] == o ve t[1][i] == o ve t[2][i] == o { döndür doğru }
    }
    eğer t[0][0] == o ve t[1][1] == o ve t[2][2] == o { döndür doğru }
    eğer t[0][2] == o ve t[1][1] == o ve t[2][0] == o { döndür doğru }
    döndür yanlış
}

ata oyuncu = "X"
ata hamle = 0
hamle < 9 sürece yap {
    göster(tahta)
    ata girdi = oku(f"{oyuncu} oynuyor (satır sütun, örn: 1 2): ").kırp()
    eğer girdi == "" { dur }
    ata parçalar = girdi.böl(" ")
    eğer parçalar.uzunluk() != 2 {
        yazdır("İki sayı girin")
        devam
    }
    ata s = 0
    ata k = 0
    dene {
        s = parçalar[0].tama() - 1
        k = parçalar[1].tama() - 1
    } yakala h: Hata.Dönüşüm {
        yazdır("Sayı girin")
        devam
    }
    eğer s < 0 veya s > 2 veya k < 0 veya k > 2 {
        yazdır("1 ile 3 arasında girin")
        devam
    }
    eğer tahta[s][k] != "." {
        yazdır("Orası dolu")
        devam
    }
    tahta[s][k] = oyuncu
    hamle += 1
    eğer kazandı_mı(tahta, oyuncu) {
        göster(tahta)
        yazdır(f"{oyuncu} kazandı!")
        dur
    }
    eğer hamle == 9 {
        göster(tahta)
        yazdır("Berabere")
    }
    eğer oyuncu == "X" { oyuncu = "O" } değilse { oyuncu = "X" }
}
```

</details>

### Proje 6: Sınav sonuçları

Her satırı `"ad,not"` biçiminde olan bir liste var; bazı satırlar bozuk. Bozuk satırları atlayıp hangi satırın neden bozuk olduğunu yazın, sonra geçenleri (50 ve üstü) nota göre büyükten küçüğe sıralayın, sınıf ortalamasını ve en yüksek notu bulun. Lambda ve hata yönetimi kullanın.

<details>
<summary>Çözüm</summary>

```kolay
tür Sonuç(ad: Yazı, not: Tam(64))
tür BozukSatır[Hata](satır: Yazı)

fonksiyon çöz(satır: Yazı) -> Sonuç {
    ata parçalar = satır.böl(",")
    eğer parçalar.uzunluk() != 2 {
        fırlat BozukSatır(mesaj: "virgülle ayrılmış iki parça olmalı", satır: satır)
    }
    ata ad = parçalar[0].kırp()
    dene {
        ata n = parçalar[1].tama()
        eğer n < 0 veya n > 100 {
            fırlat BozukSatır(mesaj: f"not 0 ile 100 arasında olmalı ({n})", satır: satır)
        }
        döndür Sonuç(ad, n)
    } yakala h: Hata.Dönüşüm {
        fırlat BozukSatır(mesaj: "not bir sayı değil", satır: satır)
    }
}

ata satırlar = ["Ali,85", "Ayşe,92", "Can,yetmiş", "Deniz,47", "Ece", "Fatih,61", "Gül,140"]
ata sonuçlar: Liste<Sonuç>
satırlar içindeki her s için yap {
    dene {
        sonuçlar.ekle(çöz(s))
    } yakala h: BozukSatır {
        yazdır(f"atlandı: '{h.satır}': {h.mesaj}")
    }
}

ata geçenler = sonuçlar.filtre(her s için s.not >= 50).sıralı(göre: her s için -s.not)
yazdır("Geçenler:", geçenler.dönüşüm(her s için f"{s.ad} ({s.not})"))
ata notlar = sonuçlar.dönüşüm(her s için s.not)
yazdır(f"Ortalama: {notlar.toplam() olarak Ondalık / notlar.uzunluk()}, en yüksek: {notlar.en_büyük()}")
```

</details>

### Daha fazla proje fikri

- **Birim çevirici:** uzunluk, ağırlık, sıcaklık birimleri arasında çeviri; birimler için `seçenek`.
- **Alışveriş listesi:** ürün ekle, sil, işaretle, listele; menü `doğru sürece yap` ile.
- **Roma rakamları:** sayıyı Roma rakamına ve tersine çeviren fonksiyonlar.
- **Hayat oyunu (Conway):** `Liste<Mantık>[20][20]` üzerinde hücre simülasyonu.
- **Kütüphane otomasyonu:** `Kitap`, `Üye`, `Ödünç` türleri; geciken kitapları bulma.

---

## 28. Kolay'dan Python'a

Kolay'da öğrendiğiniz kavramlar Python'da birebir vardır, yalnızca kelimeler İngilizcedir. Aşağıdaki tablo bir sözlük gibi kullanılabilir.

| Kolay | Python |
|---|---|
| `yazdır("merhaba")` | `print("merhaba")` |
| `ata x = 5` | `x = 5` |
| `ata x: Tam = 5` | `x: int = 5` |
| `ata sabit PI = 3.14` | `PI = 3.14` (Python'da gerçek sabit yok, büyük harf gelenektir) |
| `f"Toplam: {t}"` | `f"Toplam: {t}"` (aynısı) |
| `f"{x:.2}"`, `f"{ad:<10}"`, `f"{n:06}"` | `f"{x:.2f}"`, `f"{ad:<10}"`, `f"{n:06}"` (Python'da basamak için `f` gerekir) |
| `f"{n:,}"` → `1.234.567` | `f"{n:,}"` → `1,234,567` (İngiliz usulü) |
| `// yorum` | `# yorum` |
| `oku("Ad: ")` | `input("Ad: ")` |
| `"42".tama()` | `int("42")` (Python'da başarısızsa `ValueError` fırlatır) |
| `"2.5".ondalığa()` | `float("2.5")` |
| `2 ** 10`, `a & b`, `a \| b`, `a ^ b`, `~a`, `a << n`, `a >> n` | aynısı |
| `42.yazıya()` | `str(42)` |
| `doğru`, `yanlış` | `True`, `False` |
| `ve`, `veya`, `değil` | `and`, `or`, `not` |
| `eğer x > 0 { ... }` | `if x > 0:` + girinti |
| `değilse eğer` | `elif` |
| `değilse` | `else` |
| `5 kere yap { }` | `for _ in range(5):` |
| `0..10 içindeki her i için yap { }` | `for i in range(10):` |
| `1..=10 içindeki her i için yap { }` | `for i in range(1, 11):` |
| `(0..100).adım(5)` | `range(0, 100, 5)` |
| `10..0` (geri sayar) | `range(10, 0, -1)` |
| `ata r = 1..=5` (bir liste) | `r = list(range(1, 6))` |
| `liste içindeki her x için yap { }` | `for x in liste:` |
| `sözlük içindeki her (k, d) için yap { }` | `for k, d in sözlük.items():` |
| `koşul sürece yap { }` | `while koşul:` |
| `dur`, `devam` | `break`, `continue` |
| `fonksiyon f(a: Tam) -> Tam { döndür a }` | `def f(a: int) -> int: return a` |
| `[1, 2, 3]` | `[1, 2, 3]` |
| `l.ekle(4)` | `l.append(4)` |
| `l.uzunluk()` | `len(l)` |
| `l.içerir(3)` | `3 in l` |
| `l.sırala()` | `l.sort()` |
| `l.sıralı()` | `sorted(l)` |
| `l.ters_çevir()` | `l.reverse()` |
| `l.tersi()`, `l[-1..=0]` | `l[::-1]` |
| `l.adım(2)` | `l[::2]` |
| `l[-1]`, `l[1..3]` | `l[-1]`, `l[1:3]` |
| `l[4..1]` | `l[4:1:-1]` |
| `l.bul(x)` (yoksa `Hiç`) | `l.index(x)` (yoksa `ValueError`) |
| `l.toplam()`, `l.en_büyük()`, `l.en_küçük()` | `sum(l)`, `max(l)`, `min(l)` |
| `her x için x * 2` | `lambda x: x * 2` |
| `l.dönüşüm(her x için x * 2)` | `[x * 2 for x in l]` ya da `list(map(lambda x: x * 2, l))` |
| `l.filtre(her x için x > 0)` | `[x for x in l if x > 0]` |
| `l.hepsi(her x için x > 0)`, `l.herhangi(...)` | `all(x > 0 for x in l)`, `any(...)` |
| `l.sırala(göre: her k için k.yaş)` | `l.sort(key=lambda k: k.yaş)` |
| `Fonksiyon(Tam) -> Tam` | `Callable[[int], int]` |
| `fonksiyon ilk<T>(l: Liste<T>) -> T` | `T = TypeVar("T")` + `def ilk(l: list[T]) -> T:` |
| `tür Kutu<T>(değer: T)` | `class Kutu(Generic[T]):` |
| `{"a": 1}` | `{"a": 1}` |
| `s.büyük_harf()` | `s.upper()` (Türkçe `i` için doğru değil!) |
| `s.böl(",")` | `s.split(",")` |
| `s.kırp()` | `s.strip()` |
| `s.değiştir(a, b)` | `s.replace(a, b)` |
| `s.başlar(p)`, `s.biter(p)` | `s.startswith(p)`, `s.endswith(p)` |
| `s.bul(p)` (yoksa `Hiç`) | `s.find(p)` (yoksa `-1`) |
| `s.say(p)` | `s.count(p)` |
| `s.tekrar(3)` | `s * 3` |
| `l.birleşik(", ")` | `", ".join(l)` |
| `doğrula(x > 0, "mesaj")` | `assert x > 0, "mesaj"` |
| `tür Kişi(ad: Yazı) { }` | `class Kişi:` + `def __init__(self, ad): self.ad = ad` |
| `bu.ad` | `self.ad` |
| `ortak ata sayı = 0`, `Öğrenci.sayı` | sınıf değişkeni: `sayi = 0` (sınıf içinde), `Ogrenci.sayi` |
| `ortak fonksiyon yeni()` | `@staticmethod def yeni():` |
| `tür Öğrenci[Kişi]` | `class Öğrenci(Kişi):` |
| `Kişi.tanıt()` (ebeveyn metodu) | `super().tanıt()` |
| `seçenek Renk { Kırmızı }` | `class Renk(Enum): KIRMIZI = 1` |
| `eşleştir x { ... }` | `match x: case ...:` |
| `Hiç` | `None` |
| `dahil et("./m.kolay")` | `from m import *` |
| `m adıyla dahil et("./m.kolay")` | `import m` |
| `dene { } yakala h: Hata.Dönüşüm { }` | `try: ... except ValueError as h: ...` |
| `yakala h: Hata { }` | `except Exception as h:` |
| `yakala diğer { }` | `except:` (tavsiye edilmez) ya da `except BaseException` |
| `fırlat Hata(mesaj: "...")` | `raise Exception("...")` |
| `h.mesaj` | `str(h)` |
| `tür GeçersizYaş[Hata](yaş: Tam)` | `class GeçersizYaş(Exception): ...` |
| `-> Tam veya Hata` | Python'da karşılığı yok (sadece belgelenir) |
| `kolay` (etkileşimli kip) | `python3` (etkileşimli kip) |

### Dikkat edilmesi gereken farklar

1. **Bloklar:** Kolay'da `{ }`, Python'da girinti (boşluk) ve `:`. Python'da girinti zorunludur ve yanlış girinti hata verir.
2. **Tipler:** Python tipleri program çalışırken kontrol eder. `"5" + 3` hatası Kolay'da program başlamadan, Python'da ancak o satır çalışınca görülür. Python'daki tip yazımları (`x: int`) yalnızca bilgi amaçlıdır, kontrol edilmez.
3. **Değer semantiği:** Python'da listeler ve nesneler kopyalanmaz, paylaşılır:
   ```python
   a = [1, 2]
   b = a
   b.append(3)
   print(a)   # [1, 2, 3]: a da değişti!
   ```
   Kolay'da `a` değişmezdi. Python'da kopya için `b = a.copy()` yazılır.
4. **Sayılar:** Python'da `int` her zaman esnektir (Kolay'daki `Tam` gibi), taşmaz. `7 / 2` Python'da `3.5` verir; tam bölme için `7 // 2` yazılır.
5. **Değişken tanımı:** Python'da `ata` yoktur. İlk atama değişkeni oluşturur, bu yüzden yazım hatalı bir isim sessizce yeni bir değişken olur.
6. **Sabitler:** Python'da gerçek sabit yoktur.
7. **Hatalar:** Python'da herhangi bir satır hata fırlatabilir ve bu, fonksiyonun imzasında görünmez. Kolay'da `T veya Hata` döndüren bir fonksiyonu kontrol etmeden kullanmak derleme hatasıdır; Python'da `int("abc")` ancak çalışırken patlar. Python'a geçince `try`/`except` yazmayı kendiniz hatırlamalısınız.
8. **Lambda:** Python'daki `lambda` tek bir ifade olabilir; birden fazla satır gerekiyorsa `def` ile isimli bir fonksiyon yazılır. Kolay'daki `her x için yap { ... }` gibi bloklu lambda Python'da yoktur.
9. **Yakalama:** Python'daki lambda dışarıdaki değişkenin **kendisini** görür, değeri sonradan değişirse lambda da değişmiş değeri kullanır. Kolay'da lambda oluşturulduğu andaki kopyayı kullanır.

---

## 29. Başvuru tabloları

### Anahtar kelimeler

| Kolay | `us` yazımı | Anlamı |
|---|---|---|
| `ata` | `ata` | değişken tanımla |
| `ata sabit` | `ata sabit` | sabit tanımla |
| `fonksiyon` | `fonksiyon` | fonksiyon tanımla |
| `döndür` | `dondur` | fonksiyondan değer döndür |
| `eğer` / `değilse` | `eger` / `degilse` | koşul |
| `N kere yap` | `N kere yap` | N kez tekrarla |
| `... içindeki her x için yap` | `... icindeki her x icin yap` | her eleman için |
| `koşul sürece yap` | `kosul surece yap` | koşul doğru olduğu sürece |
| `dur` / `devam` | `dur` / `devam` | döngüden çık / sonraki tura geç |
| `doğru` / `yanlış` | `dogru` / `yanlis` | Mantık değerleri |
| `ve` / `veya` / `değil` | `ve` / `veya` / `degil` | mantık işlemleri |
| `olarak` | `olarak` | tip çevirme |
| `tür` | `tur` | yeni tip (sınıf) |
| `yeniden` | `yeniden` | ebeveyn metodunu yeniden yaz |
| `bu` | `bu` | nesnenin kendisi |
| `arayüz` | `arayuz` | arayüz |
| `seçenek` | `secenek` | seçenekler (enum) |
| `eşleştir` | `eslestir` | değeri durumlarla karşılaştır |
| `Hiç` | `Hic` | değer yok |
| `dahil et` / `adıyla` | `dahil et` / `adiyla` | başka dosya ekle |
| `#Güvensiz` | `#Guvensiz` | C kütüphanesi kullanımı için zorunlu işaret |
| `dış kütüphane` | `dis kutuphane` | C kütüphanesi |
| `kütüphanesindeki` | `kutuphanesindeki` | tek C fonksiyonu |
| `klavye("us")` | `klavye("us")` | Türkçe karaktersiz yazım (ilk satır) |
| `her x için ...` | `her x icin ...` | isimsiz fonksiyon (lambda) |
| `dene` / `yakala` | `dene` / `yakala` | hata yakalama |
| `fırlat` | `firlat` | hata (ya da herhangi bir değer) fırlat |
| `ortak` | `ortak` | türe ait değer ya da fonksiyon (`ortak ata`, `ortak fonksiyon`) |

### Tipler

| Tip | Açıklama |
|---|---|
| `Tam(64)` | varsayılan tam sayı |
| `Tam(8/16/32/128)` | boyutlu tam sayı |
| `Tam(N, +)` / `Tam(N, -)` | sadece pozitif / sadece negatif |
| `Tam` | esnek, hiç taşmaz |
| `Ondalık`, `Ondalık(16/32)` | ondalıklı sayı |
| `Yazı` | metin |
| `Mantık` | `doğru` / `yanlış` |
| `Liste<T>` | esnek liste |
| `Liste<T>[N]`, `Liste<T>[N][M]` | sabit boyutlu liste, tablo |
| `Sözlük<A, D>` | anahtar → değer |
| `A veya B` | A ya da B tipinde değer |
| `Hiç` | değer yok |
| `T` (tip parametresi) | generic fonksiyon ya da türde, sonradan belli olan tip: `fonksiyon f<T>(x: T)` |
| `Kutu<Tam>` | generic türün bir tiple kullanımı |
| `Fonksiyon(A, B) -> C` | A ve B alıp C döndüren fonksiyon; dönüşü yoksa `Fonksiyon(A, B)` |
| `Hata` | hata nesnesi (`mesaj` alanı var) |
| `Hata.SıfıraBölme`, `Hata.İndeks`, `Hata.Anahtar`, `Hata.Taşma`, `Hata.Dönüşüm` | hazır hata türleri (`us` modunda `SifiraBolme`, `Indeks`, `Tasma`, `Donusum`) |
| `T veya Hata` | ya bir değer ya da bir hata (`dene` içinde hata fırlatılır) |

### Hazır fonksiyonlar ve metotlar

| Ne | Metotlar |
|---|---|
| her yerde | `yazdır(...)`, `oku("soru")`, `doğrula(koşul)`, `doğrula(koşul, "mesaj")` |
| `Yazı` | `uzunluk()`, `büyük_harf()`, `küçük_harf()`, `karakter(i)`, `içerir(parça)`, `böl(ayraç)`, `kırp()`, `değiştir(eski, yeni)`, `başlar(p)`, `biter(p)`, `bul(p)` → `Tam(64) veya Hiç`, `say(p)`, `tekrar(n)`, `tersi()`, `tama()` → `Tam(64) veya Hata.Dönüşüm`, `ondalığa()` → `Ondalık veya Hata.Dönüşüm`, `yazıya()`, dilim `s[a..b]` |
| `Liste` (değiştiren) | `ekle(x)`, `araya_ekle(i, x)`, `sil(i)`, `çıkar()`, `sırala()`, `sırala(göre: f)`, `ters_çevir()`, `temizle()` |
| `Liste` (yeni değer veren) | `uzunluk()`, `içerir(x)`, `bul(x)` → `Tam(64) veya Hiç`, `sıralı()`, `sıralı(göre: f)`, `tersi()`, `adım(n)`, `dönüşüm(f)`, `filtre(f)`, `hepsi(f)`, `herhangi(f)`, `toplam()`, `en_büyük()`, `en_küçük()`, `birleşik(ayraç)`, `yazıya()`, dilim `l[a..b]` |
| `Sözlük` | `uzunluk()`, `içerir(anahtar)`, `sil(anahtar)`, `anahtarlar()`, `değerler()`, `temizle()`, `yazıya()` |
| `Ondalık` | `tama()`, `yazıya()` |
| `Tam` | `yazıya()` |
| aralık (`a..b`) | bir listedir: liste metotlarının hepsi; `(a..b).adım(n)` |
| `Hata` | `mesaj` alanı |

### İşlemler

| İşlem | Anlamı |
|---|---|
| `+ - * /` | dört işlem (tam sayılarda `/` tam bölme) |
| `%` | kalan |
| `== != < > <= >=` | karşılaştırma |
| `ve veya değil` (`&& \|\| !`) | mantık |
| `**` | üs (`2 ** 10`) |
| `& \| ^ ~ << >>` | bit işlemleri (sadece tam sayılar) |
| `+= -= *= /= **= &= \|= ^= <<= >>=` | kısa atama |
| `a..b`, `a..=b` | aralık (b hariç / dahil); `a > b` ise geri sayar |
| `x olarak T` | tip çevirme |
| `f"{x:.2}"`, `{ad:<10}`, `{ad:^10}`, `{n:>6}`, `{n:06}`, `{n:,}`, `{ad:*^9}` | f-yazı biçimleri: basamak, hizalama, sıfırla doldurma, gruplama, dolgu |
| `l[i]`, `l[-1]`, `l[a..b]`, `l[a..]`, `l[..b]`, `l[b..a]` | erişim, sondan erişim, dilim, ters dilim |

İşlem önceliği (yüksekten düşüğe): `**`, tekli `-` ve `~`, `olarak`, `* / %`, `+ -`, `<< >>`, `&`, `^`, `|`, `..`, karşılaştırmalar, `değil`, `ve`, `veya`.

### Etkileşimli kip komutları

| Komut | Ne yapar |
|---|---|
| `:yardım` | yardım |
| `:tip ifade` | ifadenin tipi (çalıştırmadan) |
| `:geçmiş` | hatasız girdiler |
| `:kaydet dosya` | girdileri dosyaya yaz |
| `:yükle dosya` | dosyayı oturuma çalıştır |
| `:temizle` | oturumu sıfırla |
| `:çık` | çık (Ctrl+D) |

### Hazır hata türleri

| Tür | Ne zaman |
|---|---|
| `Hata.SıfıraBölme` | tam sayıda `/ 0`, `% 0` |
| `Hata.İndeks` | olmayan liste/yazı sırası, boş listede `çıkar()`, `en_büyük()`, `en_küçük()` |
| `Hata.Anahtar` | sözlükte olmayan anahtar |
| `Hata.Taşma` | sayı tipine sığmadı |
| `Hata.Dönüşüm` | `tama()`, `ondalığa()`, `olarak` çevrilemedi |
| `Hata` | diğer çalışma hataları; kendi hata türlerinizin ebeveyni |

---

Tebrikler, kursu bitirdiniz! Artık değişkenlerle, koşullarla, döngülerle, fonksiyonlarla, listelerle, sözlüklerle, lambdalarla, kendi türlerinizle, generic kodla ve hata yönetimiyle program yazabiliyorsunuz. Bunlar bütün programlama dillerinin temelidir. Bir sonraki adım için 27. bölümdeki proje fikirlerinden birini seçip baştan sona kendiniz yazmayı deneyin.
