# Kolay'a katkı

Kolay'a katkı vermek istediğiniz için teşekkürler. Bu belge nasıl başlayacağınızı anlatır.

## Başlamak

```
git clone <bu depo> && cd <depo>
cargo build --release
./testler/calistir.sh          # bütün testler geçmeli
```

Gerekenler: Rust (1.85+), clang, llvm, libffi (bkz. [README](README.md#kurulum)).

## Ne katkı verebilirim?

**1. Hazır kütüphane** (en kolayı; derleyiciye dokunulmaz)
- Kolay ile yazılır, `kutuphaneler/` klasörüne konur: `kutuphaneler/geometri.kolay`.
- Şablon için: `kolay kütüphane yeni geometri`. İlk satır kütüphanenin tek satırlık özetidir
  (`kolay kütüphane liste` bunu gösterir).
- Dosyanın sonunda `doğrula(...)` satırlarıyla kendi testleri bulunmalı.
- Kütüphaneyi kullanan bir test ekleyin: `testler/derleme/kut_geometri.kolay` (bkz. `kut_hazir.kolay`).
- Örnekler: `kutuphaneler/matematik.kolay` (bir C kütüphanesini sarar), `kutuphaneler/istatistik.kolay`
  (saf Kolay).

**2. Hata düzeltmesi:** Önce hatayı gösteren bir test ekleyin (`testler/`), sonra düzeltin.

**3. Dile yeni hazır metot ya da sözdizimi:** Adım adım: [Geliştirici rehberi](belgeler/GELISTIRICI.md).
Dilin tasarım kararlarını proje sahibi verir: büyük bir değişiklikten önce bir öneri (issue) açın.

## Kurallar

- **İki yol aynı sonucu vermeli:** Yorumlayıcı (`kolay kod.kolay`) ve derleyici (`kolay derle`) aynı
  çıktıyı ve aynı hata mesajlarını vermeli. `testler/derleme/` altındaki testler bunu kendiliğinden
  kontrol eder.
- **Hata mesajları Türkçe ve yol gösterici:** Ne yanlış, nasıl düzeltilir, mümkünse bir örnekle.
- **Testler:** Her yeni özellik ya da düzeltme için test. Beklenen çıktılar `./testler/calistir.sh --guncelle`
  ile yazılır; farkı mutlaka gözden geçirin.
- **Öğretici kalmalı:** Kolay bir öğretim dili. Var olan özelliklerle yapılabilen bir şey için yeni özellik
  eklemek yerine var olanı kullanın; gizli davranış yerine açık yazımı tercih edin.
- **Kod:** Değişken ve fonksiyon adları ile açıklamalar Türkçe ya da çevresindeki kodla uyumlu olsun.
  `cargo build --release` uyarısız bitmeli.

## Katkıların lisansı

Kolay [PolyForm Noncommercial 1.0.0](LICENSE) ile lisanslıdır (kurs: CC BY-NC-SA 4.0). Katkı göndererek
(pull request), katkınızın bu lisansla yayımlanmasını ve proje sahibinin (Ömer Faruk Yılmaz) katkınızı da
içeren projeyi ticari kullanım için ayrıca lisanslayabilmesini kabul etmiş olursunuz. Katkınızda adınız
korunur.

Sorular için: ofy.omer.yilmaz@gmail.com
