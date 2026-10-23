// Kolay çalışma kütüphanesi: Liste ve Sözlük.
//
// Liste: elemanlar başlıktan hemen sonra, depolama biçiminde, art arda durur (bellekte tek parça).
// Değiştirilmeden önce derleyici kolay_benzersiz ile tek sahip olunmasını sağlar (yazınca kopyala).
// Büyüyen listeler yeniden ayrılabilir; bu yüzden ekleyen fonksiyonlar listenin yeni adresini döndürür.
//
// Sözlük: eklenme sırasını koruyan karma tablo (yorumlayıcıdaki IndexMap gibi). Girdiler sırayla bir
// dizide durur; silinen girdi işaretlenir, tablo büyürken silinenler atılır.

// ------------------------------------------------------------ Liste

static int64_t eleman_boy(const Liste *l) { return l->b.tip->e1->boyut; }

Liste *kolay_liste_yeni(const Tip *t, int64_t n) {
    int64_t kap = n < 4 ? 4 : n;
    Liste *l = bellek_sifir(sizeof(Liste) + (size_t)(kap * t->e1->boyut));
    l->b.sayac = 1;
    l->b.tip = t;
    l->uzunluk = n;
    l->kapasite = kap;
    return l;
}

void liste_icerik_birak(Liste *l) {
    const Tip *e = l->b.tip->e1;
    for (int64_t i = 0; i < l->uzunluk; i++) kolay_deger_birak(l->veri + i * e->boyut, e);
}

Liste *liste_kopya(Liste *l) {
    const Tip *e = l->b.tip->e1;
    Liste *k = kolay_liste_yeni(l->b.tip, l->uzunluk);
    memcpy(k->veri, l->veri, (size_t)(l->uzunluk * e->boyut));
    for (int64_t i = 0; i < l->uzunluk; i++) kolay_deger_tut(k->veri + i * e->boyut, e);
    return k;
}

static Liste *liste_yer_ac(Liste *l, int64_t gerek) {
    if (gerek <= l->kapasite) return l;
    int64_t kap = l->kapasite * 2;
    if (kap < gerek) kap = gerek;
    Liste *n = realloc(l, sizeof(Liste) + (size_t)(kap * eleman_boy(l)));
    if (!n) kolay_hata("Bellek yetmedi.", "?", "");
    n->kapasite = kap;
    return n;
}

/// Sona ekler; değerin sahipliği listeye geçer.
Liste *kolay_liste_ekle(Liste *l, void *deger) {
    int64_t eb = eleman_boy(l);
    l = liste_yer_ac(l, l->uzunluk + 1);
    memcpy(l->veri + l->uzunluk * eb, deger, (size_t)eb);
    l->uzunluk++;
    return l;
}

_Noreturn void kolay_indeks_hatasi(int64_t i, int64_t len, const char *konum, const char *kod) {
    char msg[300];
    if (len == 0)
        snprintf(msg, sizeof msg, "Listenin %lld. indeksine erişilmek istendi, ama liste boş.", (long long)i);
    else
        snprintf(msg, sizeof msg, "Listenin %lld. indeksine erişilmek istendi, ama liste %lld elemanlı (0 .. %lld).",
                 (long long)i, (long long)len, (long long)(len - 1));
    kolay_hata(msg, konum, kod);
}

_Noreturn void kolay_liste_boyut_hatasi(const char *tip, int64_t n, int64_t len, const char *konum, const char *kod) {
    char msg[400];
    snprintf(msg, sizeof msg, "%s tam %lld eleman almalı, ama %lld elemanlı bir liste verildi.", tip, (long long)n,
             (long long)len);
    kolay_hata(msg, konum, kod);
}

static int64_t gercek_indeks(int64_t i, int64_t len, const char *konum, const char *kod) {
    int64_t r = i < 0 ? i + len : i;
    if (r < 0 || r >= len) kolay_indeks_hatasi(i, len, konum, kod);
    return r;
}

Liste *kolay_liste_araya(Liste *l, int64_t i, void *deger, const char *konum, const char *kod) {
    int64_t len = l->uzunluk, eb = eleman_boy(l);
    int64_t r = i < 0 ? i + len : i;
    if (r < 0 || r > len) {
        char msg[300];
        snprintf(msg, sizeof msg, "%lld. konuma eklenemez; liste %lld elemanlı (0 .. %lld arası olmalı).", (long long)i,
                 (long long)len, (long long)len);
        kolay_hata(msg, konum, kod);
    }
    l = liste_yer_ac(l, len + 1);
    memmove(l->veri + (r + 1) * eb, l->veri + r * eb, (size_t)((len - r) * eb));
    memcpy(l->veri + r * eb, deger, (size_t)eb);
    l->uzunluk++;
    return l;
}

void kolay_liste_sil(Liste *l, int64_t i, const char *konum, const char *kod) {
    int64_t r = gercek_indeks(i, l->uzunluk, konum, kod), eb = eleman_boy(l);
    kolay_deger_birak(l->veri + r * eb, l->b.tip->e1);
    memmove(l->veri + r * eb, l->veri + (r + 1) * eb, (size_t)((l->uzunluk - r - 1) * eb));
    l->uzunluk--;
}

/// Son elemanı çıkarıp *cikti'ya taşır (sahiplik çağırana geçer).
void kolay_liste_cikar(Liste *l, void *cikti, const char *konum, const char *kod) {
    if (l->uzunluk == 0) kolay_hata("Liste boş, eleman çıkarılamaz.", konum, kod);
    int64_t eb = eleman_boy(l);
    l->uzunluk--;
    memcpy(cikti, l->veri + l->uzunluk * eb, (size_t)eb);
}

void kolay_liste_temizle(Liste *l) {
    liste_icerik_birak(l);
    l->uzunluk = 0;
}

void kolay_liste_ters(Liste *l) {
    int64_t eb = eleman_boy(l);
    char tmp[64];
    for (int64_t i = 0, j = l->uzunluk - 1; i < j; i++, j--) {
        memcpy(tmp, l->veri + i * eb, (size_t)eb);
        memcpy(l->veri + i * eb, l->veri + j * eb, (size_t)eb);
        memcpy(l->veri + j * eb, tmp, (size_t)eb);
    }
}

// Kararlı birleştirme sıralaması (yorumlayıcıdaki gibi eşit elemanların sırası korunur)
static void birlestir_sirala(char *a, char *tmp, int64_t n, int64_t eb, const Tip *t) {
    if (n < 2) return;
    if (n <= 12) {
        char x[64];
        for (int64_t i = 1; i < n; i++) {
            memcpy(x, a + i * eb, (size_t)eb);
            int64_t j = i - 1;
            while (j >= 0 && deger_kars(a + j * eb, x, t) > 0) {
                memcpy(a + (j + 1) * eb, a + j * eb, (size_t)eb);
                j--;
            }
            memcpy(a + (j + 1) * eb, x, (size_t)eb);
        }
        return;
    }
    int64_t m = n / 2;
    birlestir_sirala(a, tmp, m, eb, t);
    birlestir_sirala(a + m * eb, tmp, n - m, eb, t);
    if (deger_kars(a + (m - 1) * eb, a + m * eb, t) <= 0) return;
    int64_t i = 0, j = m, k = 0;
    while (i < m && j < n) {
        if (deger_kars(a + j * eb, a + i * eb, t) < 0) memcpy(tmp + (k++) * eb, a + (j++) * eb, (size_t)eb);
        else memcpy(tmp + (k++) * eb, a + (i++) * eb, (size_t)eb);
    }
    while (i < m) memcpy(tmp + (k++) * eb, a + (i++) * eb, (size_t)eb);
    while (j < n) memcpy(tmp + (k++) * eb, a + (j++) * eb, (size_t)eb);
    memcpy(a, tmp, (size_t)(n * eb));
}

void kolay_liste_sirala(Liste *l) {
    int64_t eb = eleman_boy(l);
    char *tmp = bellek((size_t)(l->uzunluk * eb));
    birlestir_sirala(l->veri, tmp, l->uzunluk, eb, l->b.tip->e1);
    free(tmp);
}

_Bool kolay_liste_icerir(Liste *l, void *deger) {
    const Tip *e = l->b.tip->e1;
    for (int64_t i = 0; i < l->uzunluk; i++)
        if (kolay_esit(l->veri + i * e->boyut, deger, e)) return 1;
    return 0;
}

int64_t kolay_liste_bul(Liste *l, void *deger) {
    const Tip *e = l->b.tip->e1;
    for (int64_t i = 0; i < l->uzunluk; i++)
        if (kolay_esit(l->veri + i * e->boyut, deger, e)) return i;
    return -1;
}

/// a[bas..son] (bayrak: 1 bitişi var, 2 bitiş dahil); sınırlar listeye sığacak şekilde kırpılır.
Liste *kolay_liste_dilim(Liste *l, const Tip *sonuc_tipi, int64_t bas, int64_t son, int64_t bayrak) {
    int64_t len = l->uzunluk;
#define NORM(i) ((i) < 0 ? (((i) + len) < 0 ? 0 : ((i) + len)) : ((i) > len ? len : (i)))
    int64_t s = NORM(bas);
    int64_t e = len;
    if (bayrak & 1) {
        int64_t x = (bayrak & 2) ? (son == INT64_MAX ? son : son + 1) : son;
        e = NORM(x);
    }
#undef NORM
    int64_t n = s < e ? e - s : 0, eb = eleman_boy(l);
    Liste *k = kolay_liste_yeni(sonuc_tipi, n);
    memcpy(k->veri, l->veri + s * eb, (size_t)(n * eb));
    for (int64_t i = 0; i < n; i++) kolay_deger_tut(k->veri + i * eb, l->b.tip->e1);
    return k;
}

// ------------------------------------------------------------ Sözlük

typedef struct {
    Baslik b;
    int64_t adet;       // canlı girdi sayısı
    int64_t kullanilan; // girdi dizisinde kullanılan yer (silinenler dahil)
    int64_t kapasite;   // girdi dizisinin kapasitesi
    char *girdiler;
    int64_t *tablo;     // 0: boş, -1: silinmiş, k: (k-1). girdi
    int64_t tablo_boy;  // 2'nin kuvveti
} Sozluk;

// Girdi: [özet 8][canlı 8][anahtar][değer]
static void duzen(const Tip *t, int64_t *ok, int64_t *od, int64_t *eb) {
    const Tip *k = t->e1, *v = t->e2;
    *ok = hizala(16, k->hiza);
    *od = hizala(*ok + k->boyut, v->hiza);
    *eb = hizala(*od + v->boyut, 16);
}

void *kolay_sozluk_yeni(const Tip *t) {
    Sozluk *d = bellek_sifir(sizeof(Sozluk));
    d->b.sayac = 1;
    d->b.tip = t;
    return d;
}

#define GIRDI(d, i, eb) ((d)->girdiler + (i) * (eb))

static int64_t sozluk_ara(Sozluk *d, const void *k, uint64_t h, int64_t *yuva) {
    if (!d->tablo_boy) {
        if (yuva) *yuva = -1;
        return -1;
    }
    const Tip *kt = d->b.tip->e1;
    int64_t ok, od, eb;
    duzen(d->b.tip, &ok, &od, &eb);
    int64_t maske = d->tablo_boy - 1, i = (int64_t)(h & (uint64_t)maske), ilk_bos = -1;
    for (;;) {
        int64_t s = d->tablo[i];
        if (s == 0) {
            if (yuva) *yuva = ilk_bos >= 0 ? ilk_bos : i;
            return -1;
        }
        if (s < 0) {
            if (ilk_bos < 0) ilk_bos = i;
        } else {
            char *g = GIRDI(d, s - 1, eb);
            if (*(uint64_t *)g == h && kolay_esit(g + ok, (void *)k, kt)) {
                if (yuva) *yuva = i;
                return s - 1;
            }
        }
        i = (i + 1) & maske;
    }
}

static void tablo_kur(Sozluk *d, int64_t boy) {
    int64_t ok, od, eb;
    duzen(d->b.tip, &ok, &od, &eb);
    // silinmiş girdileri at (sıra korunur)
    int64_t j = 0;
    for (int64_t i = 0; i < d->kullanilan; i++) {
        char *g = GIRDI(d, i, eb);
        if (!*(uint64_t *)(g + 8)) continue;
        if (i != j) memcpy(GIRDI(d, j, eb), g, (size_t)eb);
        j++;
    }
    d->kullanilan = j;
    free(d->tablo);
    d->tablo = bellek_sifir((size_t)boy * 8);
    d->tablo_boy = boy;
    for (int64_t i = 0; i < d->kullanilan; i++) {
        uint64_t h = *(uint64_t *)GIRDI(d, i, eb);
        int64_t y = (int64_t)(h & (uint64_t)(boy - 1));
        while (d->tablo[y]) y = (y + 1) & (boy - 1);
        d->tablo[y] = i + 1;
    }
}

int64_t kolay_sozluk_uzunluk(void *dp) { return ((Sozluk *)dp)->adet; }

void *kolay_sozluk_bul(void *dp, void *anahtar) {
    Sozluk *d = dp;
    uint64_t h = deger_ozet(anahtar, d->b.tip->e1);
    int64_t i = sozluk_ara(d, anahtar, h, NULL);
    if (i < 0) return NULL;
    int64_t ok, od, eb;
    duzen(d->b.tip, &ok, &od, &eb);
    return GIRDI(d, i, eb) + od;
}

static _Noreturn void anahtar_yok(Sozluk *d, void *anahtar, const char *konum, const char *kod) {
    Yazi *y = yazi_ayir(0);
    deger_yaz(&y, anahtar, d->b.tip->e1, 0, 0);
    yazi_ekle_c(&y, " anahtarı sözlükte yok.", strlen(" anahtarı sözlükte yok."));
    kolay_hata(y->d, konum, kod);
}

void *kolay_sozluk_al(void *dp, void *anahtar, const char *konum, const char *kod) {
    void *p = kolay_sozluk_bul(dp, anahtar);
    if (!p) anahtar_yok(dp, anahtar, konum, kod);
    return p;
}

_Bool kolay_sozluk_icerir(void *dp, void *anahtar) { return kolay_sozluk_bul(dp, anahtar) != NULL; }

/// Anahtarın değer yerini döndürür; yoksa (anahtarın bir kopyası ve boş değerle) ekler.
void *kolay_sozluk_yer(void *dp, void *anahtar) {
    Sozluk *d = dp;
    const Tip *kt = d->b.tip->e1;
    int64_t ok, od, eb;
    duzen(d->b.tip, &ok, &od, &eb);
    uint64_t h = deger_ozet(anahtar, kt);
    int64_t yuva;
    int64_t i = sozluk_ara(d, anahtar, h, &yuva);
    if (i >= 0) return GIRDI(d, i, eb) + od;
    if ((d->kullanilan + 1) * 2 > d->tablo_boy) {
        int64_t boy = d->tablo_boy ? d->tablo_boy : 8;
        while ((d->adet + 1) * 2 > boy) boy *= 2;
        if ((d->kullanilan + 1) * 2 > boy) boy *= 2;
        tablo_kur(d, boy);
        sozluk_ara(d, anahtar, h, &yuva);
    }
    if (d->kullanilan == d->kapasite) {
        d->kapasite = d->kapasite ? d->kapasite * 2 : 8;
        d->girdiler = realloc(d->girdiler, (size_t)(d->kapasite * eb));
        if (!d->girdiler) kolay_hata("Bellek yetmedi.", "?", "");
    }
    char *g = GIRDI(d, d->kullanilan, eb);
    memset(g, 0, (size_t)eb);
    *(uint64_t *)g = h;
    *(uint64_t *)(g + 8) = 1;
    deger_kopyala_tut(g + ok, anahtar, kt);
    d->tablo[yuva] = d->kullanilan + 1;
    d->kullanilan++;
    d->adet++;
    return g + od;
}

void kolay_sozluk_sil(void *dp, void *anahtar, const char *konum, const char *kod) {
    Sozluk *d = dp;
    const Tip *t = d->b.tip;
    uint64_t h = deger_ozet(anahtar, t->e1);
    int64_t yuva;
    int64_t i = sozluk_ara(d, anahtar, h, &yuva);
    if (i < 0) anahtar_yok(d, anahtar, konum, kod);
    int64_t ok, od, eb;
    duzen(t, &ok, &od, &eb);
    char *g = GIRDI(d, i, eb);
    kolay_deger_birak(g + ok, t->e1);
    kolay_deger_birak(g + od, t->e2);
    *(uint64_t *)(g + 8) = 0;
    d->tablo[yuva] = -1;
    d->adet--;
}

void sozluk_icerik_birak(void *dp) {
    Sozluk *d = dp;
    const Tip *t = d->b.tip;
    int64_t ok, od, eb;
    duzen(t, &ok, &od, &eb);
    for (int64_t i = 0; i < d->kullanilan; i++) {
        char *g = GIRDI(d, i, eb);
        if (!*(uint64_t *)(g + 8)) continue;
        kolay_deger_birak(g + ok, t->e1);
        kolay_deger_birak(g + od, t->e2);
    }
    free(d->girdiler);
    free(d->tablo);
}

void kolay_sozluk_temizle(void *dp) {
    Sozluk *d = dp;
    sozluk_icerik_birak(d);
    d->girdiler = NULL;
    d->tablo = NULL;
    d->adet = d->kullanilan = d->kapasite = d->tablo_boy = 0;
}

void *sozluk_kopya(void *dp) {
    Sozluk *d = dp;
    const Tip *t = d->b.tip;
    int64_t ok, od, eb;
    duzen(t, &ok, &od, &eb);
    Sozluk *k = kolay_sozluk_yeni(t);
    k->kapasite = d->adet ? d->adet : 0;
    k->girdiler = k->kapasite ? bellek((size_t)(k->kapasite * eb)) : NULL;
    for (int64_t i = 0; i < d->kullanilan; i++) {
        char *g = GIRDI(d, i, eb);
        if (!*(uint64_t *)(g + 8)) continue;
        char *h = GIRDI(k, k->kullanilan, eb);
        memcpy(h, g, (size_t)eb);
        kolay_deger_tut(h + ok, t->e1);
        kolay_deger_tut(h + od, t->e2);
        k->kullanilan++;
    }
    k->adet = k->kullanilan;
    if (k->adet) {
        int64_t boy = 8;
        while (k->adet * 2 >= boy) boy *= 2;
        tablo_kur(k, boy);
    }
    return k;
}

int64_t kolay_sozluk_girdi_sayisi(void *dp) { return ((Sozluk *)dp)->kullanilan; }

void *kolay_sozluk_anahtar_i(void *dp, int64_t i) {
    Sozluk *d = dp;
    int64_t ok, od, eb;
    duzen(d->b.tip, &ok, &od, &eb);
    char *g = GIRDI(d, i, eb);
    return *(uint64_t *)(g + 8) ? g + ok : NULL;
}

void *kolay_sozluk_deger_i(void *dp, int64_t i) {
    Sozluk *d = dp;
    int64_t ok, od, eb;
    duzen(d->b.tip, &ok, &od, &eb);
    char *g = GIRDI(d, i, eb);
    return *(uint64_t *)(g + 8) ? g + od : NULL;
}

static Liste *sozluk_liste(Sozluk *d, const Tip *liste_tipi, int deger_mi) {
    const Tip *t = d->b.tip;
    int64_t ok, od, eb;
    duzen(t, &ok, &od, &eb);
    Liste *l = kolay_liste_yeni(liste_tipi, 0);
    const Tip *et = deger_mi ? t->e2 : t->e1;
    char tmp[64];
    for (int64_t i = 0; i < d->kullanilan; i++) {
        char *g = GIRDI(d, i, eb);
        if (!*(uint64_t *)(g + 8)) continue;
        deger_kopyala_tut(tmp, g + (deger_mi ? od : ok), et);
        l = kolay_liste_ekle(l, tmp);
    }
    return l;
}

Liste *kolay_sozluk_anahtarlar(void *dp, const Tip *liste_tipi) { return sozluk_liste(dp, liste_tipi, 0); }
Liste *kolay_sozluk_degerler(void *dp, const Tip *liste_tipi) { return sozluk_liste(dp, liste_tipi, 1); }

_Bool sozluk_esit(void *ap, void *bp) {
    Sozluk *a = ap, *b = bp;
    if (a == b) return 1;
    if (a->adet != b->adet) return 0;
    const Tip *t = a->b.tip;
    int64_t ok, od, eb;
    duzen(t, &ok, &od, &eb);
    for (int64_t i = 0; i < a->kullanilan; i++) {
        char *g = GIRDI(a, i, eb);
        if (!*(uint64_t *)(g + 8)) continue;
        void *v = kolay_sozluk_bul(b, g + ok);
        if (!v || !kolay_esit(g + od, v, t->e2)) return 0;
    }
    return 1;
}

void sozluk_yaz(Yazi **out, void *dp, int ipucu) {
    Sozluk *d = dp;
    const Tip *t = d->b.tip;
    int64_t ok, od, eb;
    duzen(t, &ok, &od, &eb);
    yazi_ekle_c(out, "{", 1);
    int ilk = 1;
    for (int64_t i = 0; i < d->kullanilan; i++) {
        char *g = GIRDI(d, i, eb);
        if (!*(uint64_t *)(g + 8)) continue;
        if (!ilk) yazi_ekle_c(out, ", ", 2);
        ilk = 0;
        deger_yaz(out, g + ok, t->e1, 0, ipucu);
        yazi_ekle_c(out, ": ", 2);
        deger_yaz(out, g + od, t->e2, 0, ipucu);
    }
    yazi_ekle_c(out, "}", 1);
}
