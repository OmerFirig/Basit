// Kolay çalışma kütüphanesi: yazılar (UTF-8), yazdırma ve okuma.

const Tip kolay_yazi_tipi = {.tur = T_YAZI, .boyut = 8, .hiza = 8, .ad = "Yazı"};

static struct {
    Baslik b;
    int64_t uzunluk, kapasite;
    char d[1];
} bos_yazi_veri = {{-1, &kolay_yazi_tipi}, 0, 0, {0}};
#define BOS_YAZI ((Yazi *)&bos_yazi_veri)

Yazi *bos_yazi(void) { return BOS_YAZI; }

Yazi *yazi_ayir(int64_t uzunluk) {
    int64_t kap = uzunluk < 15 ? 15 : uzunluk;
    Yazi *y = bellek(sizeof(Yazi) + (size_t)kap + 1);
    y->b.sayac = 1;
    y->b.tip = &kolay_yazi_tipi;
    y->uzunluk = uzunluk;
    y->kapasite = kap;
    y->d[uzunluk] = 0;
    return y;
}

Yazi *yazi_yap(const char *s, size_t n) {
    Yazi *y = yazi_ayir((int64_t)n);
    memcpy(y->d, s, n);
    return y;
}

Yazi *yazi_c(const char *s) { return yazi_yap(s, strlen(s)); }

Yazi *kolay_bos(void) { return yazi_ayir(0); }

/// C fonksiyonunun döndürdüğü yazıdan Kolay yazısı (NULL ise boş yazı).
Yazi *kolay_c_yazi(const char *s) { return s ? yazi_c(s) : yazi_ayir(0); }

/// Yazı tamponuna bayt ekler (*a'nın tek sahibi çağıran olmalı).
void yazi_ekle_c(Yazi **a, const char *s, size_t n) {
    Yazi *y = *a;
    int64_t gerek = y->uzunluk + (int64_t)n;
    if (gerek > y->kapasite) {
        int64_t kap = y->kapasite * 2;
        if (kap < gerek) kap = gerek;
        y = realloc(y, sizeof(Yazi) + (size_t)kap + 1);
        if (!y) kolay_hata_tur(HT_YAKALANAMAZ, "Bellek yetmedi.", "?", "");
        y->kapasite = kap;
    }
    memcpy(y->d + y->uzunluk, s, n);
    y->uzunluk = gerek;
    y->d[gerek] = 0;
    *a = y;
}

Yazi *kolay_birlestir(Yazi *a, Yazi *b) {
    Yazi *y = yazi_ayir(a->uzunluk + b->uzunluk);
    memcpy(y->d, a->d, (size_t)a->uzunluk);
    memcpy(y->d + a->uzunluk, b->d, (size_t)b->uzunluk);
    return y;
}

/// a'nın sonuna b'yi ekler. a'nın sahipliğini alır: başka kullanıcısı yoksa yerinde büyütülür.
Yazi *kolay_ekle(Yazi *a, Yazi *b) {
    int64_t lb = b->uzunluk;
    if (a->b.sayac != 1) {
        Yazi *y = yazi_ayir(a->uzunluk + lb);
        memcpy(y->d, a->d, (size_t)a->uzunluk);
        memcpy(y->d + a->uzunluk, b->d, (size_t)lb);
        kolay_birak(a);
        return y;
    }
    int64_t gerek = a->uzunluk + lb;
    if (gerek > a->kapasite) {
        int64_t kap = a->kapasite * 2;
        if (kap < gerek) kap = gerek;
        int ayni = (a == b);
        Yazi *n = realloc(a, sizeof(Yazi) + (size_t)kap + 1);
        if (!n) kolay_hata_tur(HT_YAKALANAMAZ, "Bellek yetmedi.", "?", "");
        if (ayni) b = n;
        a = n;
        a->kapasite = kap;
    }
    memmove(a->d + a->uzunluk, b->d, (size_t)lb);
    a->uzunluk += lb;
    a->d[a->uzunluk] = 0;
    return a;
}

_Bool kolay_yazi_esit(Yazi *a, Yazi *b) {
    return a->uzunluk == b->uzunluk && memcmp(a->d, b->d, (size_t)a->uzunluk) == 0;
}

// ------------------------------------------------------------ yazdırma ve okuma

void kolay_yaz(Yazi *s) { fwrite(s->d, 1, (size_t)s->uzunluk, stdout); }
void kolay_yaz_bosluk(void) { fputc(' ', stdout); }
void kolay_satir(void) { fputc('\n', stdout); }
void kolay_tampon_bosalt(void) { fflush(stdout); }

Yazi *kolay_oku(Yazi *soru) {
    if (soru) kolay_yaz(soru);
    fflush(stdout);
    Yazi *s = yazi_ayir(0);
    int c;
    while ((c = getchar()) != EOF && c != '\n') {
        char ch = (char)c;
        yazi_ekle_c(&s, &ch, 1);
    }
    while (s->uzunluk > 0 && s->d[s->uzunluk - 1] == '\r') s->uzunluk--;
    s->d[s->uzunluk] = 0;
    return s;
}

// ------------------------------------------------------------ UTF-8

static uint32_t utf8_oku(const unsigned char **p) {
    const unsigned char *s = *p;
    uint32_t c = s[0];
    int n = 1;
    if (c >= 0xF0) { c &= 0x07; n = 4; }
    else if (c >= 0xE0) { c &= 0x0F; n = 3; }
    else if (c >= 0xC0) { c &= 0x1F; n = 2; }
    for (int i = 1; i < n && s[i]; i++) c = (c << 6) | (s[i] & 0x3F);
    *p = s + n;
    return c;
}

static int utf8_yaz(uint32_t c, char *o) {
    if (c < 0x80) { o[0] = (char)c; return 1; }
    if (c < 0x800) { o[0] = (char)(0xC0 | (c >> 6)); o[1] = (char)(0x80 | (c & 0x3F)); return 2; }
    if (c < 0x10000) {
        o[0] = (char)(0xE0 | (c >> 12)); o[1] = (char)(0x80 | ((c >> 6) & 0x3F)); o[2] = (char)(0x80 | (c & 0x3F));
        return 3;
    }
    o[0] = (char)(0xF0 | (c >> 18)); o[1] = (char)(0x80 | ((c >> 12) & 0x3F));
    o[2] = (char)(0x80 | ((c >> 6) & 0x3F)); o[3] = (char)(0x80 | (c & 0x3F));
    return 4;
}

// Türkçe kurallar (i/İ, ı/I) önce; diğer harfler için Unicode tabloları (towlower/towupper).
static uint32_t tr_kucuk(uint32_t c) {
    if (c == 'I') return 0x131;   // ı
    if (c == 0x130) return 'i';   // İ
    return (uint32_t)towlower((wint_t)c);
}

static uint32_t tr_buyuk(uint32_t c) {
    if (c == 'i') return 0x130;
    if (c == 0x131) return 'I';
    return (uint32_t)towupper((wint_t)c);
}

static Yazi *harf_cevir(Yazi *s, uint32_t (*f)(uint32_t)) {
    Yazi *o = yazi_ayir(s->uzunluk * 2 + 4);
    const unsigned char *p = (const unsigned char *)s->d, *son = p + s->uzunluk;
    int64_t n = 0;
    while (p < son) {
        if (n + 4 > o->kapasite) {
            int64_t kap = o->kapasite * 2;
            Yazi *y = realloc(o, sizeof(Yazi) + (size_t)kap + 1);
            if (!y) kolay_hata_tur(HT_YAKALANAMAZ, "Bellek yetmedi.", "?", "");
            o = y;
            o->kapasite = kap;
        }
        n += utf8_yaz(f(utf8_oku(&p)), o->d + n);
    }
    o->uzunluk = n;
    o->d[n] = 0;
    return o;
}

Yazi *kolay_buyuk_harf(Yazi *s) { return harf_cevir(s, tr_buyuk); }
Yazi *kolay_kucuk_harf(Yazi *s) { return harf_cevir(s, tr_kucuk); }

/// 8 baytlık bloktaki karakter başlangıcı sayısı: devam baytları (10xxxxxx) dışındakiler
static inline int64_t blok_karakter(const unsigned char *p) {
    uint64_t w;
    memcpy(&w, p, 8);
    return 8 - __builtin_popcountll(w & ~(w << 1) & 0x8080808080808080ULL);
}

/// Karakter sayısı (Türkçe harfler tek karakter). Bloklarla sayar: yorumlayıcıdaki chars().count() kadar hızlı.
int64_t kolay_yazi_uzunluk(Yazi *s) {
    int64_t n = 0;
    const unsigned char *p = (const unsigned char *)s->d, *son = p + s->uzunluk;
    for (; son - p >= 8; p += 8) n += blok_karakter(p);
    for (; p < son; p++)
        if ((*p & 0xC0) != 0x80) n++;
    return n;
}

/// Yazı üzerinde dönme: *konum baytından başlayan karakteri döndürür (bittiyse NULL).
Yazi *kolay_yazi_sonraki(Yazi *s, int64_t *konum) {
    if (*konum >= s->uzunluk) return NULL;
    const unsigned char *p = (const unsigned char *)s->d + *konum, *bas = p;
    utf8_oku(&p);
    *konum += p - bas;
    return yazi_yap((const char *)bas, (size_t)(p - bas));
}

// Türk alfabesine göre sıralama anahtarı: c < ç < d, ı < i, o < ö ...
static const uint32_t ALFABE[] = {'a', 'b', 'c', 0xE7, 'd', 'e', 'f', 'g', 0x11F, 'h', 0x131, 'i', 'j', 'k', 'l', 'm',
                                  'n', 'o', 0xF6, 'p', 'q', 'r', 's', 0x15F, 't', 'u', 0xFC, 'v', 'w', 'x', 'y', 'z'};

static void tr_anahtar(uint32_t c, uint32_t *birincil, uint32_t *ikincil) {
    uint32_t k = tr_kucuk(c);
    for (unsigned i = 0; i < sizeof ALFABE / sizeof ALFABE[0]; i++) {
        if (ALFABE[i] == k) {
            *birincil = 1000 + i;
            *ikincil = (k == c) ? 1 : 0;
            return;
        }
    }
    *birincil = c;
    *ikincil = 0;
}

int32_t kolay_yazi_kars(Yazi *a, Yazi *b) {
    for (int tur = 0; tur < 2; tur++) {
        const unsigned char *p = (const unsigned char *)a->d, *q = (const unsigned char *)b->d;
        const unsigned char *ps = p + a->uzunluk, *qs = q + b->uzunluk;
        while (p < ps && q < qs) {
            uint32_t p1, p2, q1, q2;
            tr_anahtar(utf8_oku(&p), &p1, &p2);
            tr_anahtar(utf8_oku(&q), &q1, &q2);
            if (p1 != q1) return p1 < q1 ? -1 : 1;
            if (tur == 1 && p2 != q2) return p2 < q2 ? -1 : 1;
        }
        if (p < ps || q < qs) return p < ps ? 1 : -1;
    }
    return 0;
}

// Unicode boşluk karakterleri (Rust'taki trim ile aynı küme)
static int bosluk_mu(uint32_t c) {
    return (c >= 9 && c <= 13) || c == ' ' || c == 0x85 || c == 0xA0 || c == 0x1680 || (c >= 0x2000 && c <= 0x200A) ||
           c == 0x2028 || c == 0x2029 || c == 0x202F || c == 0x205F || c == 0x3000;
}

static void kirp_sinir(Yazi *s, const char **bas, const char **son) {
    const unsigned char *p = (const unsigned char *)s->d, *e = p + s->uzunluk;
    while (p < e) {
        const unsigned char *q = p;
        if (!bosluk_mu(utf8_oku(&q))) break;
        p = q;
    }
    while (e > p) {
        const unsigned char *k = e - 1;
        while (k > p && (*k & 0xC0) == 0x80) k--;
        const unsigned char *q = k;
        if (!bosluk_mu(utf8_oku(&q))) break;
        e = k;
    }
    *bas = (const char *)p;
    *son = (const char *)e;
}

Yazi *kolay_kirp(Yazi *s) {
    const char *b, *e;
    kirp_sinir(s, &b, &e);
    return yazi_yap(b, (size_t)(e - b));
}

_Bool kolay_icerir(Yazi *s, Yazi *p) {
    if (p->uzunluk == 0) return 1;
    return memmem(s->d, (size_t)s->uzunluk, p->d, (size_t)p->uzunluk) != NULL;
}

/// Bütün `eski` parçalarını `yeni` ile değiştirir (yeni yazı).
Yazi *kolay_degistir(Yazi *s, Yazi *eski, Yazi *yeni, const char *konum, const char *kod) {
    if (eski->uzunluk == 0) kolay_hata_tur(HT_GENEL, "Değiştirilecek parça boş olamaz.", konum, kod);
    Yazi *y = yazi_ayir(0);
    const char *p = s->d, *son = s->d + s->uzunluk;
    for (;;) {
        const char *q = memmem(p, (size_t)(son - p), eski->d, (size_t)eski->uzunluk);
        if (!q) {
            yazi_ekle_c(&y, p, (size_t)(son - p));
            return y;
        }
        yazi_ekle_c(&y, p, (size_t)(q - p));
        yazi_ekle_c(&y, yeni->d, (size_t)yeni->uzunluk);
        p = q + eski->uzunluk;
    }
}

_Bool kolay_baslar(Yazi *s, Yazi *p) {
    return p->uzunluk <= s->uzunluk && memcmp(s->d, p->d, (size_t)p->uzunluk) == 0;
}

_Bool kolay_biter(Yazi *s, Yazi *p) {
    return p->uzunluk <= s->uzunluk && memcmp(s->d + s->uzunluk - p->uzunluk, p->d, (size_t)p->uzunluk) == 0;
}

/// Parçanın ilk geçtiği yerin karakter sırası; yoksa -1.
int64_t kolay_yazi_bul(Yazi *s, Yazi *p) {
    if (p->uzunluk == 0) return 0;
    const char *q = memmem(s->d, (size_t)s->uzunluk, p->d, (size_t)p->uzunluk);
    if (!q) return -1;
    int64_t n = 0;
    for (const char *c = s->d; c < q; c++)
        if ((*c & 0xC0) != 0x80) n++;
    return n;
}

/// Parçanın kaç kez geçtiği (örtüşmeyen)
int64_t kolay_say(Yazi *s, Yazi *p, const char *konum, const char *kod) {
    if (p->uzunluk == 0) kolay_hata_tur(HT_GENEL, "Aranan parça boş olamaz.", konum, kod);
    int64_t n = 0;
    const char *c = s->d, *son = s->d + s->uzunluk;
    const char *q;
    while ((q = memmem(c, (size_t)(son - c), p->d, (size_t)p->uzunluk))) {
        n++;
        c = q + p->uzunluk;
    }
    return n;
}

/// tekrar() ile oluşturulabilecek en uzun yazı (yorumlayıcıdaki YAZI_SINIRI ile aynı)
#define YAZI_SINIRI ((int64_t)1 << 32)

Yazi *kolay_tekrar(Yazi *s, int64_t n, const char *konum, const char *kod) {
    if (n < 0) {
        char msg[100];
        snprintf(msg, sizeof msg, "Tekrar sayısı eksi olamaz: %lld.", (long long)n);
        kolay_hata_tur(HT_GENEL, msg, konum, kod);
    }
    if (s->uzunluk > 0 && n > YAZI_SINIRI / s->uzunluk) kolay_hata_tur(HT_YAKALANAMAZ, "Bellek yetmedi.", konum, kod);
    Yazi *y = yazi_ayir(s->uzunluk * n);
    for (int64_t i = 0; i < n; i++) memcpy(y->d + i * s->uzunluk, s->d, (size_t)s->uzunluk);
    return y;
}

/// f-yazıda {x:.2}: virgülden sonra n basamak (yorumlayıcıda format!("{x:.n$}"))
Yazi *kolay_hassas(double x, int64_t n) {
    if (!isfinite(x)) {
        char buf[32];
        ondalik_yaz(x, 64, buf);
        return yazi_c(buf);
    }
    int k = snprintf(NULL, 0, "%.*f", (int)n, x);
    Yazi *y = yazi_ayir(k);
    snprintf(y->d, (size_t)k + 1, "%.*f", (int)n, x);
    return y;
}

/// f-yazıda {n:,}: "-1234567.5" -> "-1.234.567,5" (yorumlayıcıda interp::grupla_tr)
Yazi *kolay_grupla(Yazi *s) {
    const char *p = s->d, *son = s->d + s->uzunluk;
    int isaret = p < son && *p == '-';
    const char *tam = p + isaret, *nokta = memchr(tam, '.', (size_t)(son - tam));
    const char *tam_son = nokta ? nokta : son;
    if (tam == tam_son || memchr(s->d, 'e', (size_t)s->uzunluk)) return yazi_yap(s->d, (size_t)s->uzunluk);
    for (const char *c = tam; c < tam_son; c++)
        if (*c < '0' || *c > '9') return yazi_yap(s->d, (size_t)s->uzunluk);
    Yazi *y = yazi_ayir(0);
    if (isaret) yazi_ekle_c(&y, "-", 1);
    int64_t n = tam_son - tam;
    for (int64_t i = 0; i < n; i++) {
        if (i > 0 && (n - i) % 3 == 0) yazi_ekle_c(&y, ".", 1);
        yazi_ekle_c(&y, tam + i, 1);
    }
    if (nokta) {
        yazi_ekle_c(&y, ",", 1);
        yazi_ekle_c(&y, nokta + 1, (size_t)(son - nokta - 1));
    }
    return y;
}

/// f-yazıda genişlik: kip 0 sola, 1 sağa, 2 ortaya, 3 sıfırla (yorumlayıcıda interp::hizala)
Yazi *kolay_hizala(Yazi *s, int64_t genislik, Yazi *dolgu, int64_t kip) {
    int64_t n = kolay_yazi_uzunluk(s);
    if (n >= genislik) return yazi_yap(s->d, (size_t)s->uzunluk);
    int64_t bos = genislik - n, sol = kip == 0 ? 0 : kip == 2 ? bos / 2 : bos;
    Yazi *y = yazi_ayir(0);
    const char *d = s->d;
    size_t dn = (size_t)s->uzunluk;
    if (kip == 3) {
        if (dn > 0 && *d == '-') {
            yazi_ekle_c(&y, "-", 1);
            d++;
            dn--;
        }
        for (int64_t i = 0; i < bos; i++) yazi_ekle_c(&y, "0", 1);
        yazi_ekle_c(&y, d, dn);
        return y;
    }
    for (int64_t i = 0; i < sol; i++) yazi_ekle_c(&y, dolgu->d, (size_t)dolgu->uzunluk);
    yazi_ekle_c(&y, d, dn);
    for (int64_t i = 0; i < bos - sol; i++) yazi_ekle_c(&y, dolgu->d, (size_t)dolgu->uzunluk);
    return y;
}

/// Liste<Yazı>.birleşik(ayraç): ["a", "b"] -> "a, b"
Yazi *kolay_birlesik(Liste *l, Yazi *ayrac) {
    Yazi *y = yazi_ayir(0);
    for (int64_t i = 0; i < l->uzunluk; i++) {
        Yazi *e = *(Yazi **)(l->veri + i * (int64_t)sizeof(Yazi *));
        if (i > 0) yazi_ekle_c(&y, ayrac->d, (size_t)ayrac->uzunluk);
        yazi_ekle_c(&y, e->d, (size_t)e->uzunluk);
    }
    return y;
}

/// i. karakterin başladığı bayt (i >= 0); yoksa NULL. Önce i'yi geçmeyen tam blokları atlar.
static const unsigned char *karakter_basi(Yazi *s, int64_t i) {
    const unsigned char *p = (const unsigned char *)s->d, *son = p + s->uzunluk;
    int64_t k = 0; // p'den önce başlayan karakter sayısı
    for (; son - p >= 8; p += 8) {
        int64_t c = blok_karakter(p);
        if (k + c > i) break;
        k += c;
    }
    for (; p < son; p++) {
        if ((*p & 0xC0) == 0x80) continue;
        if (k == i) return p;
        k++;
    }
    return NULL;
}

Yazi *kolay_karakter(Yazi *s, int64_t i, const char *konum, const char *kod) {
    // eksi sıra sondan sayar: önce karakter sayısı gerekir; artı sırada gerekmez
    int64_t real = i < 0 ? i + kolay_yazi_uzunluk(s) : i;
    const unsigned char *bas = real < 0 ? NULL : karakter_basi(s, real);
    if (!bas) {
        char msg[200];
        snprintf(msg, sizeof msg, "Yazının %lld. karakteri yok; yazı %lld karakterli.", (long long)i, (long long)kolay_yazi_uzunluk(s));
        kolay_hata_tur(HT_INDEKS, msg, konum, kod);
    }
    const unsigned char *son = (const unsigned char *)s->d + s->uzunluk, *p = bas + 1;
    while (p < son && (*p & 0xC0) == 0x80) p++;
    return yazi_yap((const char *)bas, (size_t)(p - bas));
}

/// "a,b,c".böl(",") -> ["a", "b", "c"]; ayraç boşsa karakterlere böler.
Liste *kolay_yazi_bol(Yazi *s, Yazi *ayrac, const Tip *liste_tipi) {
    Liste *l = kolay_liste_yeni(liste_tipi, 0);
    Liste *(*ekle)(Liste *, void *) = kolay_liste_ekle;
    if (ayrac->uzunluk == 0) {
        int64_t k = 0;
        Yazi *c;
        while ((c = kolay_yazi_sonraki(s, &k))) l = ekle(l, &c);
        return l;
    }
    const char *p = s->d, *son = s->d + s->uzunluk;
    for (;;) {
        const char *q = memmem(p, (size_t)(son - p), ayrac->d, (size_t)ayrac->uzunluk);
        const char *e = q ? q : son;
        Yazi *parca = yazi_yap(p, (size_t)(e - p));
        l = ekle(l, &parca);
        if (!q) break;
        p = q + ayrac->uzunluk;
    }
    return l;
}

/// Yazıdan tam sayı (Rust'taki gibi: isteğe bağlı +/- ve ardından sadece rakamlar). Başarısızsa 0.
static int tama_coz(Yazi *s, int64_t *v) {
    const char *b, *e;
    kirp_sinir(s, &b, &e);
    size_t n = (size_t)(e - b);
    char buf[64];
    if (n == 0 || n >= sizeof buf) return 0;
    memcpy(buf, b, n);
    buf[n] = 0;
    size_t k = (buf[0] == '+' || buf[0] == '-') ? 1 : 0;
    if (k == n) return 0;
    for (size_t i = k; i < n; i++)
        if (buf[i] < '0' || buf[i] > '9') return 0;
    errno = 0;
    long long x = strtoll(buf, NULL, 10);
    if (errno == ERANGE) return 0;
    *v = x;
    return 1;
}

/// Yazıdan ondalık sayı (virgül de ayırıcı). Başarısızsa 0.
static int ondaliga_coz(Yazi *s, double *v) {
    const char *b, *e;
    kirp_sinir(s, &b, &e);
    size_t n = (size_t)(e - b);
    char buf[512];
    if (n == 0 || n >= sizeof buf) return 0;
    memcpy(buf, b, n);
    buf[n] = 0;
    for (char *c = buf; *c; c++) {
        if (*c == ',') *c = '.';
        // Rust'ın kabul etmediği biçimler: onaltılık (0x..) ve nan(...)
        if (*c == 'x' || *c == 'X' || *c == '(' || *c == ' ') return 0;
    }
    char *end;
    double x = strtod(buf, &end);
    if (*end != 0) return 0;
    *v = x;
    return 1;
}

/// Sonuç 'veya' değeri: sayı ya da Hata.Dönüşüm (ut: sonucun tipi, ör. Tam(64) veya Hata.Dönüşüm)
static void sayi_ya_da_hata(void *out, const Tip *ut, int ok, const void *deger, Yazi *s, const char *ne) {
    for (int64_t i = 0; i < ut->n; i++) {
        const Tip *m = ut->tipler[i];
        int hata = m->tur == T_TUR;
        if (hata == !ok) {
            *(int64_t *)out = i;
            memset((char *)out + 8, 0, 16);
            if (ok) {
                memcpy((char *)out + 8, deger, 8);
            } else {
                char msg[400];
                snprintf(msg, sizeof msg, "\"%.200s\" bir %s çevrilemez.", s->d, ne);
                Baslik *o = kolay_nesne_yeni(m, m->nesne_boyut);
                *(Yazi **)((char *)o + m->ofsetler[0]) = yazi_c(msg);
                *(void **)((char *)out + 8) = o;
            }
            return;
        }
    }
}

void kolay_yazi_tama(Yazi *s, void *out, const Tip *ut) {
    int64_t v = 0;
    int ok = tama_coz(s, &v);
    sayi_ya_da_hata(out, ut, ok, &v, s, "tam sayıya");
}

void kolay_yazi_ondaliga(Yazi *s, void *out, const Tip *ut) {
    double v = 0;
    int ok = ondaliga_coz(s, &v);
    sayi_ya_da_hata(out, ut, ok, &v, s, "ondalık sayıya");
}
