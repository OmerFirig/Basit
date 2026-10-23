// Kolay çalışma kütüphanesi: ortak tanımlar.
//
// Derleyici, runtime/ içindeki dosyaları şu sırayla tek bir C dosyası olarak birleştirip derler:
// kolay.h, cekirdek.c, sayilar.c, yazilar.c, genel.c, koleksiyon.c
// Bu yüzden .c dosyaları bu başlığı ayrıca #include etmez.
//
// Bellek düzeni (derleyici, codegen/duzen.rs, bununla aynı olmalı):
//   Her yığın (heap) nesnesi bir Baslik ile başlar: { sayac, tip }.
//   sayac < 0 olan nesneler sabittir (programın içine gömülü), hiç silinmez.
//   Yazi:   Baslik, uzunluk, kapasite, baytlar           (baytlar 32. baytta başlar)
//   Liste:  Baslik, uzunluk, kapasite, elemanlar         (elemanlar 32. baytta başlar)
//   Sozluk: Baslik, ... (bkz. koleksiyon.c)
//   Nesne (tür):   Baslik, alanlar (ofsetler Tip içinde)
//   Seçenek:       Baslik, etiket (i64), değerler (ofsetler Varyant içinde)
//   Buyuk (büyük tam sayı): Baslik, işaret, basamak sayısı, basamaklar
//
// Depolama biçimi (liste elemanı, nesne alanı, sözlük girdisi): her tipin sabit boyutlu bir biçimi
// vardır. Tam(N) 1/2/4/8/16 bayt (N+1 bitlik Tam(N, -) bir üst boyutta; Tam(128, -) değeri eksiyle
// çarpılıp 16 bayt işaretsiz tutulur), esnek Tam 8 bayt (etiketli: tek sayılar küçük değerdir),
// Ondalık 2/4/8, Mantık 1, işaretçiler 8, 'veya' 24 bayt (etiket + 16 bayt değer), aralık 32 bayt.

#define _GNU_SOURCE
#include <errno.h>
#include <limits.h>
#include <locale.h>
#include <math.h>
#include <pthread.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ucontext.h>
#include <unistd.h>
#include <wctype.h>

typedef __int128 i128;
typedef unsigned __int128 u128;

enum {
    T_TAM = 1,     // boyutlu Tam(N)
    T_ESNEK = 2,   // esnek Tam
    T_ONDALIK = 3,
    T_MANTIK = 4,
    T_YAZI = 5,
    T_HIC = 6,
    T_LISTE = 7,
    T_SOZLUK = 8,
    T_TUR = 9,
    T_SECENEK = 10,
    T_ARAYUZ = 11,
    T_VEYA = 12,
    T_ARALIK = 13,
    T_BUYUK = 14,  // büyük tam sayı nesnesi (sadece yığın nesnelerinin tipi olarak)
};

typedef struct Tip Tip;

typedef struct {
    const char *ad;
    int64_t n;                  // taşıdığı değer sayısı
    const char *const *adlar;
    const Tip *const *tipler;
    const int64_t *ofsetler;
    void *sabit;                // değer taşımayan seçenek için hazır (sabit) nesne
    int64_t boyut;              // nesnenin toplam boyutu
} Varyant;

struct Tip {
    int64_t tur;
    int64_t no;                 // tür/seçenek/arayüz numarası
    int64_t boyut;              // depolama boyutu (bayt)
    int64_t hiza;               // depolama hizalaması
    int64_t bit;                // Tam: bit sayısı (0 = esnek); Ondalık: 16/32/64
    int64_t isaret;             // Tam: 0 işaretli, 1 sadece pozitif (+), 2 sadece negatif (-)
    int64_t sabit_boy;          // Liste: sabit boyut, -1: esnek
    const Tip *e1, *e2;         // Liste: eleman; Sözlük: anahtar, değer
    const char *ad;             // görünen ad: "Tam(8)", "Liste<Yazı>", "Kişi" ...
    int64_t n;                  // tür: alan sayısı; seçenek: varyant sayısı; veya: üye sayısı
    const Tip *const *tipler;   // tür: alan tipleri; veya: üyeler
    const char *const *adlar;   // tür: alan adları
    const int64_t *ofsetler;    // tür: alan ofsetleri
    const Varyant *varyantlar;  // seçenek
    int64_t nesne_boyut;        // tür: nesnenin toplam boyutu
};

typedef struct {
    int64_t sayac;
    const Tip *tip;
} Baslik;

typedef struct {
    Baslik b;
    int64_t uzunluk;  // bayt sayısı (sondaki 0 hariç)
    int64_t kapasite; // ayrılan bayt sayısı (sabitlerde 0)
    char d[];
} Yazi;

typedef struct {
    Baslik b;
    int64_t uzunluk;
    int64_t kapasite;
    _Alignas(16) char veri[];
} Liste;

typedef struct {
    Baslik b;
    int64_t isaret;   // -1, 0, 1
    int64_t n;        // basamak sayısı (2^32 tabanında)
    uint32_t r[];     // küçük basamak önce
} Buyuk;

typedef struct {
    int64_t start, end, step, bayrak; // bayrak: 1 = bitişi var, 2 = bitiş dahil
} Aralik;

extern const Tip kolay_yazi_tipi;
extern const Tip kolay_buyuk_tipi;

// cekirdek.c
_Noreturn void kolay_hata(const char *msg, const char *konum, const char *kod);
void *bellek(size_t n);
void *bellek_sifir(size_t n);
void kolay_tut(void *p);
void kolay_birak(void *p);
void kolay_esnek_tut(int64_t v);
void kolay_esnek_birak(int64_t v);
void kolay_deger_tut(void *p, const Tip *t);
void kolay_deger_birak(void *p, const Tip *t);
void *kolay_benzersiz(void *p);

// yazilar.c
Yazi *yazi_ayir(int64_t uzunluk);
Yazi *yazi_yap(const char *s, size_t n);
Yazi *yazi_c(const char *s);
Yazi *kolay_ekle(Yazi *a, Yazi *b);
void yazi_ekle_c(Yazi **a, const char *s, size_t n);
int32_t kolay_yazi_kars(Yazi *a, Yazi *b);
_Bool kolay_yazi_esit(Yazi *a, Yazi *b);
Yazi *bos_yazi(void);

// sayilar.c
void tam_yazi_bicim(i128 bits, int32_t bicim, char *out);
void ondalik_yaz(double v, int32_t bits, char *out);
void grupla(const char *s, char *out);
void esnek_yaz(int64_t v, Yazi **out);
int esnek_kars(int64_t a, int64_t b);
uint64_t esnek_ozet(int64_t v);
double esnek_ondalik(int64_t v);

// genel.c
_Bool kolay_esit(void *a, void *b, const Tip *t);
uint64_t deger_ozet(const void *p, const Tip *t);
void deger_yaz(Yazi **out, const void *p, const Tip *t, int ust, int ipucu);
int deger_kars(const void *a, const void *b, const Tip *t);
void kolay_varsayilan(void *out, const Tip *t);
void deger_kopyala_tut(void *hedef, const void *kaynak, const Tip *t);

// koleksiyon.c
void liste_icerik_birak(Liste *l);
Liste *liste_kopya(Liste *l);
void sozluk_icerik_birak(void *d);
void *sozluk_kopya(void *d);
_Bool sozluk_esit(void *a, void *b);
void sozluk_yaz(Yazi **out, void *d, int ipucu);
Liste *kolay_liste_yeni(const Tip *t, int64_t n);
Liste *kolay_liste_ekle(Liste *l, void *deger);
void *kolay_sozluk_yeni(const Tip *t);
void *kolay_sozluk_yer(void *d, void *anahtar);

static inline int64_t hizala(int64_t x, int64_t h) { return (x + h - 1) / h * h; }
