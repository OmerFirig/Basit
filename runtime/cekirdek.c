// Kolay çalışma kütüphanesi: başlatma, hatalar, bellek ve referans sayımı.

void *bellek(size_t n) {
    void *p = malloc(n ? n : 1);
    if (!p) kolay_hata("Bellek yetmedi.", "?", "");
    return p;
}

void *bellek_sifir(size_t n) {
    void *p = calloc(1, n ? n : 1);
    if (!p) kolay_hata("Bellek yetmedi.", "?", "");
    return p;
}

_Noreturn void kolay_hata(const char *msg, const char *konum, const char *kod) {
    fflush(stdout);
    fprintf(stderr, "ÇALIŞMA HATASI (%s): %s\n%s", konum, msg, kod);
    exit(1);
}

_Noreturn void kolay_hata_sayi(const char *once, int64_t n, const char *sonra, const char *konum, const char *kod) {
    char msg[400];
    snprintf(msg, sizeof msg, "%s%lld%s", once, (long long)n, sonra);
    kolay_hata(msg, konum, kod);
}

// ------------------------------------------------------------ referans sayımı

const Tip kolay_buyuk_tipi = {.tur = T_BUYUK, .boyut = 8, .hiza = 8, .ad = "Tam"};

void kolay_tut(void *p) {
    Baslik *b = p;
    if (b && b->sayac >= 0) b->sayac++;
}

static void icerik_birak(Baslik *b) {
    const Tip *t = b->tip;
    switch (t->tur) {
    case T_LISTE:
        liste_icerik_birak((Liste *)b);
        break;
    case T_SOZLUK:
        sozluk_icerik_birak(b);
        break;
    case T_TUR:
    case T_FONK:
        for (int64_t i = 0; i < t->n; i++) kolay_deger_birak((char *)b + t->ofsetler[i], t->tipler[i]);
        break;
    case T_SECENEK: {
        const Varyant *v = &t->varyantlar[*(int64_t *)((char *)b + 16)];
        for (int64_t i = 0; i < v->n; i++) kolay_deger_birak((char *)b + v->ofsetler[i], v->tipler[i]);
        break;
    }
    default:
        break;
    }
}

void kolay_birak(void *p) {
    Baslik *b = p;
    if (!b || b->sayac <= 0) return;
    if (--b->sayac) return;
    icerik_birak(b);
    free(b);
}

void kolay_esnek_tut(int64_t v) {
    if (v && !(v & 1)) kolay_tut((void *)v);
}

void kolay_esnek_birak(int64_t v) {
    if (v && !(v & 1)) kolay_birak((void *)v);
}

static int isaretci_mi(const Tip *t) {
    switch (t->tur) {
    case T_YAZI: case T_LISTE: case T_SOZLUK: case T_TUR: case T_SECENEK: case T_ARAYUZ: case T_FONK:
        return 1;
    default:
        return 0;
    }
}

/// Depolanmış bir değerin referansını tutar (p: değerin adresi).
void kolay_deger_tut(void *p, const Tip *t) {
    if (isaretci_mi(t)) kolay_tut(*(void **)p);
    else if (t->tur == T_ESNEK) kolay_esnek_tut(*(int64_t *)p);
    else if (t->tur == T_VEYA) kolay_deger_tut((char *)p + 8, t->tipler[*(int64_t *)p]);
}

void kolay_deger_birak(void *p, const Tip *t) {
    if (isaretci_mi(t)) kolay_birak(*(void **)p);
    else if (t->tur == T_ESNEK) kolay_esnek_birak(*(int64_t *)p);
    else if (t->tur == T_VEYA) kolay_deger_birak((char *)p + 8, t->tipler[*(int64_t *)p]);
}

/// 'veya' değeri yazmaçlarda: (etiket, birinci kelime). Sadece birinci kelime işaretçi olabilir.
void kolay_veya_tut(const Tip *t, int64_t etiket, int64_t w0) {
    const Tip *u = t->tipler[etiket];
    if (isaretci_mi(u)) kolay_tut((void *)w0);
    else if (u->tur == T_ESNEK) kolay_esnek_tut(w0);
}

void kolay_veya_birak(const Tip *t, int64_t etiket, int64_t w0) {
    const Tip *u = t->tipler[etiket];
    if (isaretci_mi(u)) kolay_birak((void *)w0);
    else if (u->tur == T_ESNEK) kolay_esnek_birak(w0);
}

/// Değiştirilecek nesnenin tek sahibi olunmasını sağlar (yazınca kopyala).
/// Başka kullanıcısı yoksa nesnenin kendisi, varsa kopyası döner; eski nesne bırakılır.
void *kolay_benzersiz(void *p) {
    Baslik *b = p;
    if (b->sayac == 1) return p;
    void *k;
    const Tip *t = b->tip;
    switch (t->tur) {
    case T_LISTE:
        k = liste_kopya((Liste *)b);
        break;
    case T_SOZLUK:
        k = sozluk_kopya(b);
        break;
    case T_YAZI: {
        Yazi *y = (Yazi *)b;
        k = yazi_yap(y->d, (size_t)y->uzunluk);
        break;
    }
    case T_TUR: {
        Baslik *n = bellek((size_t)t->nesne_boyut);
        memcpy(n, b, (size_t)t->nesne_boyut);
        n->sayac = 1;
        for (int64_t i = 0; i < t->n; i++) kolay_deger_tut((char *)n + t->ofsetler[i], t->tipler[i]);
        k = n;
        break;
    }
    case T_SECENEK: {
        const Varyant *v = &t->varyantlar[*(int64_t *)((char *)b + 16)];
        Baslik *n = bellek((size_t)v->boyut);
        memcpy(n, b, (size_t)v->boyut);
        n->sayac = 1;
        for (int64_t i = 0; i < v->n; i++) kolay_deger_tut((char *)n + v->ofsetler[i], v->tipler[i]);
        k = n;
        break;
    }
    default:
        return p;
    }
    kolay_birak(p);
    return k;
}

void *kolay_nesne_yeni(const Tip *t, int64_t boyut) {
    Baslik *b = bellek_sifir((size_t)boyut);
    b->sayac = 1;
    b->tip = t;
    return b;
}

// ------------------------------------------------------------ tür kontrolü hataları

_Noreturn void kolay_tur_degil(const char *hedef, const char *bulunan, const char *konum, const char *kod) {
    char msg[400];
    snprintf(msg, sizeof msg, "Bu değer %s değil, %s.", hedef, bulunan);
    kolay_hata(msg, konum, kod);
}

_Noreturn void kolay_tur_degil_nesne(const char *hedef, void *nesne, const char *konum, const char *kod) {
    kolay_tur_degil(hedef, ((Baslik *)nesne)->tip->ad, konum, kod);
}

// ------------------------------------------------------------ başlatma ve derin özyineleme

static char *yigin_alt;

/// Derleyicinin ürettiği fonksiyon tablosu: { adres, konum, kaynak satırı }
typedef struct {
    const char *adres;
    const char *konum;
    const char *kod;
} FonkBilgi;
extern const int64_t kolay_fonk_sayi;
extern const FonkBilgi kolay_fonk_tablo[];
// Kolay fonksiyonlarının bulunduğu bölümün sınırları (bağlayıcı tanımlar)
extern char __start_kolay_kod[] __attribute__((weak));
extern char __stop_kolay_kod[] __attribute__((weak));

/// Komut adresinin içinde bulunduğu Kolay fonksiyonu (yoksa NULL).
static const FonkBilgi *fonk_bul(const char *pc) {
    if (!pc || !__start_kolay_kod || pc < __start_kolay_kod || pc >= __stop_kolay_kod) return NULL;
    const FonkBilgi *en = NULL;
    for (int64_t i = 0; i < kolay_fonk_sayi; i++) {
        const FonkBilgi *f = &kolay_fonk_tablo[i];
        if (f->adres <= pc && (!en || f->adres > en->adres)) en = f;
    }
    return en;
}

static const char *komut_adresi(void *ctx) {
    ucontext_t *uc = (ucontext_t *)ctx;
#if defined(__x86_64__)
    return (const char *)uc->uc_mcontext.gregs[REG_RIP];
#elif defined(__aarch64__)
    return (const char *)uc->uc_mcontext.pc;
#else
    (void)uc;
    return NULL;
#endif
}

static void yaz_stderr(const char *s) {
    size_t n = strlen(s);
    while (n > 0) {
        ssize_t k = write(2, s, n);
        if (k <= 0) break;
        s += k;
        n -= (size_t)k;
    }
}

static void segv_yakala(int sig, siginfo_t *info, void *ctx) {
    (void)sig;
    char *adr = (char *)info->si_addr;
    fflush(stdout);
    if (yigin_alt && adr >= yigin_alt - (1 << 20) && adr < yigin_alt + (1 << 16)) {
        const FonkBilgi *f = fonk_bul(komut_adresi(ctx));
        if (f) {
            yaz_stderr("ÇALIŞMA HATASI (");
            yaz_stderr(f->konum);
            yaz_stderr("): Çok derin özyineleme: bir fonksiyon kendini çok fazla çağırdı.\n");
            yaz_stderr(f->kod);
        } else {
            yaz_stderr("ÇALIŞMA HATASI: Çok derin özyineleme: bir fonksiyon kendini çok fazla çağırdı.\n");
        }
    } else {
        yaz_stderr("İÇ HATA: Derlenmiş program bellek hatasıyla durdu. Bu Kolay derleyicisinin bir hatası, lütfen bildirin.\n");
    }
    _exit(1);
}

typedef void (*kolay_fn)(void);

static void *calistir_is(void *arg) {
    // Yığın taşınca sinyal işleyicisi ayrı küçük bir yığında çalışır.
    static char alt_yigin[1 << 16];
    stack_t ss = {.ss_sp = alt_yigin, .ss_size = sizeof alt_yigin, .ss_flags = 0};
    sigaltstack(&ss, NULL);
    pthread_attr_t a;
    if (pthread_getattr_np(pthread_self(), &a) == 0) {
        void *adr;
        size_t boy;
        if (pthread_attr_getstack(&a, &adr, &boy) == 0) yigin_alt = (char *)adr;
        pthread_attr_destroy(&a);
    }
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = segv_yakala;
    sa.sa_flags = SA_SIGINFO | SA_ONSTACK;
    sigaction(SIGSEGV, &sa, NULL);
    sigaction(SIGBUS, &sa, NULL);
    ((kolay_fn)arg)();
    return NULL;
}

/// Ctrl+C: Türkçe mesajla durur (yorumlayıcıdaki main.rs ctrl_c_yakala ile aynı).
static void ctrl_c(int sig) {
    (void)sig;
    yaz_stderr("\nProgram durduruldu (Ctrl+C).\n");
    _exit(130);
}

/// Programı büyük yığınlı (1 GB, yorumlayıcıyla aynı) bir iş parçacığında çalıştırır.
void kolay_calistir(kolay_fn ana) {
    setlocale(LC_CTYPE, "C.UTF-8");
    // Çıktı ekrandaysa her satır hemen yazılır; dosya ya da boruya gidiyorsa hız için tamponlanır.
    if (isatty(1)) setvbuf(stdout, NULL, _IOLBF, 1 << 16);
    else setvbuf(stdout, NULL, _IOFBF, 1 << 16);
    signal(SIGINT, ctrl_c);
    pthread_attr_t at;
    pthread_attr_init(&at);
    pthread_t t;
    if (pthread_attr_setstacksize(&at, (size_t)1 << 30) == 0 && pthread_create(&t, &at, calistir_is, (void *)ana) == 0) {
        pthread_join(t, NULL);
    } else {
        calistir_is((void *)ana);
    }
    fflush(stdout);
}
