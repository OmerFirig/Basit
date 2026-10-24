# Kolay

Kolay (Türkçe programlama dili) için VS Code eklentisi.

- **Tamamlama:** anahtar kelimeler, hazır tipler ve fonksiyonlar; dosyadaki değişkenler, fonksiyonlar,
  türler, seçenekler. Noktadan sonra değişkenin tipine göre metotlar ve alanlar
  (`liste.` → `ekle`, `uzunluk`...; `ali.` → türün alanları ve metotları; `bu.`; `Renk.` → seçenekler).
  `klavye("us")` dosyalarında Türkçe harfsiz önerir.
- **Hata gösterme:** dosya açılınca ve kaydedilince `kolay denetle` çalışır, hata kırmızı altı çizili gösterilir.
- **Çalıştırma:** F5 ya da sağ üstteki ▷ düğmesi (`Kolay: Çalıştır`), komut paletinde `Kolay: Derle`.
- Generic: `fonksiyon en_büyük<T>(...)` ve `tür Kutu<T>(...)` tanınır; tip parametreleri (`T`) ve `Kutu<Tam>` tamamlamada çıkar.
- Renklendirme ve hazır kod parçaları (`eğer`, `kere`, `her`, `tür`, `fonksiyong` / `türg` (generic), `dene`, `fırlat`, `dış`, `olarak`...).

Ayar: `kolay.yol` (boşsa PATH'teki `kolay` kullanılır).
