// Kolay çalışma kütüphanesi: başlatma, hatalar, bellek ve referans sayımı.

// Büyük bellek isteyen işlemden önce (sabit boyutlu liste) kaynaktaki yeri; "Bellek yetmedi." hatası
// bu satırı gösterir. İşlem bitince NULL ile sıfırlanır.
static const char *bellek_konum = NULL, *bellek_kod = NULL;

void kolay_bellek_yeri(const char *konum, const char *kod) {
    bellek_konum = konum;
    bellek_kod = kod;
}

static _Noreturn void bellek_yetmedi(void) {
    kolay_hata_tur(HT_YAKALANAMAZ, "Bellek yetmedi.", bellek_konum ? bellek_konum : "?", bellek_kod ? bellek_kod : "");
}

void *bellek(size_t n) {
    void *p = malloc(n ? n : 1);
    if (!p) bellek_yetmedi();
    return p;
}

void *bellek_sifir(size_t n) {
    void *p = calloc(1, n ? n : 1);
    if (!p) bellek_yetmedi();
    return p;
}

// ------------------------------------------------------------ hatalar ve dene/yakala
//
// Her `dene` bloğu girişte bir DeneCerceve'yi zincire ekler ve setjmp yapar (derleyici, codegen/komut.rs).
// Hata olunca (çalışma hatası ya da `fırlat`) değer en içteki çerçeveye yazılır, çerçeve zincirden
// çıkarılır ve longjmp ile bloğun yakala kısmına dönülür. Çerçeve yoksa mesaj yazılıp program biter.
//
// Temizlik kayıtları: dene bloğu içinden çağrılan fonksiyonlar, derleyicinin ürettiği "dene içi"
// kopyalarıdır (codegen/mod.rs temizlik_kaydi). Bu kopyalar girişte yığınlarındaki bir Temizlik kaydını
// zincire ekler, dönerken çıkarır. Hata atlamasından önce, dene bloğuna girildikten sonra eklenen
// kayıtlardaki (yani atlanan fonksiyonların) değişkenler bırakılır. Bırakılmayanlar: atlanan
// fonksiyonların yazmaçtaki geçici değerleri ve lambda/fonksiyon değeri olarak çağrılan fonksiyonların
// değişkenleri (hata başına küçük bir sızıntı olabilir).

typedef struct TemizlikTablo {
    int64_t n;
    const Tip *tipler[];
} TemizlikTablo;

typedef struct Temizlik {
    struct Temizlik *ust;
    const TemizlikTablo *tablo;
    void *yerler[]; // değişkenlerin adresleri (depolama biçiminde, sahipli ya da sıfır)
} Temizlik;

Temizlik *kolay_temizlik_ust;

typedef struct DeneCerceve {
    jmp_buf jb;
    struct DeneCerceve *ust;
    Temizlik *temizlik;          // girişteki temizlik zinciri
    const Tip *tip;              // fırlatılan değerin (statik) tipi
    _Alignas(16) char deger[32]; // değer, depolama biçiminde (sahipliği çerçevededir)
    char *msg;                   // yakalanmazsa yazılacak mesaj
    const char *konum, *kod;
} DeneCerceve;

_Static_assert(sizeof(DeneCerceve) <= 1024, "derleyici dene çerçevesi için 1024 bayt ayırır");

static DeneCerceve *dene_ust;

// Derleyicinin ürettiği tablolar: hazır hata türleri (hata_turu sırasıyla), tür sayısı, alt tür tablosu
extern const Tip *const kolay_hata_tipleri[6];
extern const int64_t kolay_tur_sayisi;
extern const char kolay_altmi[];

void kolay_dene_gir(DeneCerceve *c) {
    c->ust = dene_ust;
    c->temizlik = kolay_temizlik_ust;
    dene_ust = c;
}

/// Gövde bitti (ya da dur/devam/döndür ile çıkıldı): çerçeve ve içindekiler zincirden çıkar.
void kolay_dene_cik(DeneCerceve *c) { dene_ust = c->ust; }

const Tip *kolay_dene_tip(DeneCerceve *c) { return c->tip; }
void *kolay_dene_deger(DeneCerceve *c) { return c->deger; }

/// Yakalanan değer kullanılmadı (dal yok ya da değişkensiz dal): bırakılır.
void kolay_dene_birak(DeneCerceve *c) {
    kolay_deger_birak(c->deger, c->tip);
    free(c->msg);
}

/// Yakalanan değer bir değişkene taşındı.
void kolay_dene_bitti(DeneCerceve *c) { free(c->msg); }

static _Noreturn void firlat_ic(const Tip *t, const void *deger, char *msg, const char *konum, const char *kod) {
    DeneCerceve *c = dene_ust;
    if (!c) {
        fflush(stdout);
        fprintf(stderr, "ÇALIŞMA HATASI (%s): %s\n%s", konum, msg, kod);
        exit(1);
    }
    dene_ust = c->ust;
    c->tip = t;
    memmove(c->deger, deger, (size_t)t->boyut);
    c->msg = msg;
    c->konum = konum;
    c->kod = kod;
    // atlanan fonksiyonların değişkenleri (değer önce çerçeveye taşındı: atlanan bir fonksiyonun yığınında olabilir)
    while (kolay_temizlik_ust != c->temizlik) {
        Temizlik *k = kolay_temizlik_ust;
        kolay_temizlik_ust = k->ust;
        for (int64_t i = 0; i < k->tablo->n; i++) kolay_deger_birak(k->yerler[i], k->tablo->tipler[i]);
    }
    longjmp(c->jb, 1);
}

/// Hiçbir yakala dalı uymadı: aynı değer dıştaki bloğa gider.
_Noreturn void kolay_dene_yeniden(DeneCerceve *c) { firlat_ic(c->tip, c->deger, c->msg, c->konum, c->kod); }

static char *kopya_c(const char *s) {
    size_t n = strlen(s);
    char *k = malloc(n + 1);
    if (!k) {
        fflush(stdout);
        fprintf(stderr, "ÇALIŞMA HATASI: Bellek yetmedi.\n");
        exit(1);
    }
    memcpy(k, s, n + 1);
    return k;
}

/// Türü belli çalışma hatası: dene içindeyse Hata.X nesnesi olarak fırlatılır.
_Noreturn void kolay_hata_tur(int tur, const char *msg, const char *konum, const char *kod) {
    if (tur == HT_YAKALANAMAZ || !dene_ust) {
        fflush(stdout);
        fprintf(stderr, "ÇALIŞMA HATASI (%s): %s\n%s", konum, msg, kod);
        exit(1);
    }
    const Tip *t = kolay_hata_tipleri[tur];
    Baslik *o = kolay_nesne_yeni(t, t->nesne_boyut);
    *(Yazi **)((char *)o + t->ofsetler[0]) = yazi_c(msg);
    firlat_ic(t, &o, kopya_c(msg), konum, kod);
}

/// kolay_hata_tur gibi, ama mesaj oluşturulmuş bir Kolay yazısı (sahipliği alınır; hata nesnesinin mesajı olur).
_Noreturn void kolay_hata_tur_yazi(int tur, Yazi *y, const char *konum, const char *kod) {
    if (tur == HT_YAKALANAMAZ || !dene_ust) kolay_hata_tur(tur, y->d, konum, kod);
    const Tip *t = kolay_hata_tipleri[tur];
    Baslik *o = kolay_nesne_yeni(t, t->nesne_boyut);
    *(Yazi **)((char *)o + t->ofsetler[0]) = y;
    firlat_ic(t, &o, kopya_c(y->d), konum, kod);
}

_Noreturn void kolay_hata(const char *msg, const char *konum, const char *kod) { kolay_hata_tur(HT_GENEL, msg, konum, kod); }

/// Nesne Hata ya da alt türü mü? (değer, tür/arayüz tipinde bir işaretçi)
static int hata_nesnesi_mi(const Tip *t, const void *p) {
    if (t->tur != T_TUR && t->tur != T_ARAYUZ) return 0;
    const Baslik *o = *(Baslik *const *)p;
    if (o->tip->tur != T_TUR) return 0;
    return kolay_altmi[o->tip->no * kolay_tur_sayisi + kolay_hata_tipleri[0]->no] != 0;
}

/// `fırlat değer` (değerin sahipliği fırlatmaya geçer). 'veya' değerinde üyenin kendi tipi kullanılır.
_Noreturn void kolay_firlat(const Tip *t, void *p, const char *konum, const char *kod) {
    if (t->tur == T_VEYA) {
        int64_t e = *(int64_t *)p;
        t = t->tipler[e];
        p = (char *)p + 8;
    }
    char *msg;
    if (hata_nesnesi_mi(t, p)) {
        const Baslik *o = *(Baslik *const *)p;
        msg = kopya_c((*(Yazi *const *)((const char *)o + o->tip->ofsetler[0]))->d);
    } else {
        Yazi *y = yazi_c("Yakalanmamış değer fırlatıldı: ");
        deger_yaz(&y, p, t, 0, 1);
        yazi_ekle_c(&y, " (", 2);
        yazi_ekle_c(&y, t->ad, strlen(t->ad));
        yazi_ekle_c(&y, ").", 2);
        msg = kopya_c(y->d);
        kolay_birak(y);
    }
    firlat_ic(t, p, msg, konum, kod);
}

/// Derlenmiş kodun, dene bloğu olan fonksiyonlarda yerel değişkenlerinin adresini "kaçırması": böylece
/// LLVM onları yazmaçta tutmaz; longjmp sonrası yakala dalı değişkenlerin son değerini görür.
void kolay_kacir(void *p) {
    static void *volatile son;
    son = p;
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
    kolay_hata_tur(HT_DONUSUM, msg, konum, kod);
}

_Noreturn void kolay_tur_degil_nesne(const char *hedef, void *nesne, const char *konum, const char *kod) {
    kolay_tur_degil(hedef, ((Baslik *)nesne)->tip->ad, konum, kod);
}

// ------------------------------------------------------------ başlatma ve derin özyineleme

/// Derleyicinin ürettiği fonksiyon tablosu: { adres, konum, kaynak satırı }
typedef struct {
    const char *adres;
    const char *konum;
    const char *kod;
} FonkBilgi;
extern const int64_t kolay_fonk_sayi;
extern const FonkBilgi kolay_fonk_tablo[];
// Kolay fonksiyonlarının bulunduğu bölümün sınırları
#ifdef _WIN32
// COFF: bağlayıcı kolay_kod$a, kolay_kod$m (Kolay fonksiyonları), kolay_kod$z sırasıyla birleştirir.
// Gövdeler farklı: bağlayıcı aynı fonksiyonları tek fonksiyona indirebilir.
__attribute__((section("kolay_kod$a"), used, noinline)) static int kolay_kod_bas(void) { return 1; }
__attribute__((section("kolay_kod$z"), used, noinline)) static int kolay_kod_son(void) { return 2; }
#define KOD_BAS ((const char *)kolay_kod_bas)
#define KOD_SON ((const char *)kolay_kod_son)
#else
// ELF: bağlayıcı tanımlar
extern char __start_kolay_kod[] __attribute__((weak));
extern char __stop_kolay_kod[] __attribute__((weak));
#define KOD_BAS __start_kolay_kod
#define KOD_SON __stop_kolay_kod
#endif

/// Komut adresinin içinde bulunduğu Kolay fonksiyonu (yoksa NULL).
static const FonkBilgi *fonk_bul(const char *pc) {
    if (!pc || !KOD_BAS || pc < KOD_BAS || pc >= KOD_SON) return NULL;
    const FonkBilgi *en = NULL;
    for (int64_t i = 0; i < kolay_fonk_sayi; i++) {
        const FonkBilgi *f = &kolay_fonk_tablo[i];
        if (f->adres <= pc && (!en || f->adres > en->adres)) en = f;
    }
    return en;
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

static void derin_ozyineleme(const char *pc) {
    const FonkBilgi *f = fonk_bul(pc);
    if (f) {
        yaz_stderr("ÇALIŞMA HATASI (");
        yaz_stderr(f->konum);
        yaz_stderr("): Çok derin özyineleme: bir fonksiyon kendini çok fazla çağırdı.\n");
        yaz_stderr(f->kod);
    } else {
        yaz_stderr("ÇALIŞMA HATASI: Çok derin özyineleme: bir fonksiyon kendini çok fazla çağırdı.\n");
    }
}

static const char *const BELLEK_HATASI =
    "İÇ HATA: Derlenmiş program bellek hatasıyla durdu. Bu Kolay derleyicisinin bir hatası, lütfen bildirin.\n";

/// Ctrl+C: Türkçe mesajla durur (yorumlayıcıdaki main.rs ctrl_c_yakala ile aynı).
static void ctrl_c(int sig) {
    (void)sig;
    yaz_stderr("\nProgram durduruldu (Ctrl+C).\n");
    _exit(130);
}

typedef void (*kolay_fn)(void);

#ifdef _WIN32

// Yığın taşması Windows'ta bir istisnadır (sinyal değil); işleyici, ayrılmış son 64 KB'ta çalışır.
static LONG WINAPI istisna_yakala(EXCEPTION_POINTERS *e) {
    DWORD kod = e->ExceptionRecord->ExceptionCode;
    if (kod != EXCEPTION_STACK_OVERFLOW && kod != EXCEPTION_ACCESS_VIOLATION) return EXCEPTION_CONTINUE_SEARCH;
    fflush(stdout);
    if (kod == EXCEPTION_STACK_OVERFLOW) {
#if defined(_M_X64) || defined(__x86_64__)
        derin_ozyineleme((const char *)e->ContextRecord->Rip);
#elif defined(_M_ARM64) || defined(__aarch64__)
        derin_ozyineleme((const char *)e->ContextRecord->Pc);
#else
        derin_ozyineleme(NULL);
#endif
    } else {
        yaz_stderr(BELLEK_HATASI);
    }
    _exit(1);
}

static DWORD WINAPI calistir_is(LPVOID arg) {
    ULONG pay = 1 << 16;
    SetThreadStackGuarantee(&pay);
    ((kolay_fn)arg)();
    return 0;
}

/// Programı büyük yığınlı (1 GB, yorumlayıcıyla aynı) bir iş parçacığında çalıştırır.
void kolay_calistir(kolay_fn ana) {
    setlocale(LC_CTYPE, ".UTF-8");
    // Yazılar UTF-8: konsol da UTF-8 gösterir. Satır sonları "\n" kalır (metin kipi "\r\n" yapardı).
    UINT eski_cp = GetConsoleOutputCP(), eski_giris = GetConsoleCP();
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCP(CP_UTF8);
    _setmode(_fileno(stdout), _O_BINARY);
    _setmode(_fileno(stderr), _O_BINARY);
    // Windows'ta satır tamponu yok (_IOLBF tam tampon gibi davranır): ekrana tamponsuz yazılır.
    if (isatty(1)) setvbuf(stdout, NULL, _IONBF, 0);
    else setvbuf(stdout, NULL, _IOFBF, 1 << 16);
    signal(SIGINT, ctrl_c);
    AddVectoredExceptionHandler(1, istisna_yakala);
    HANDLE t = CreateThread(NULL, (SIZE_T)1 << 30, calistir_is, (LPVOID)ana, STACK_SIZE_PARAM_IS_A_RESERVATION, NULL);
    if (t) {
        WaitForSingleObject(t, INFINITE);
        CloseHandle(t);
    } else {
        calistir_is((LPVOID)ana);
    }
    fflush(stdout);
    if (eski_cp) SetConsoleOutputCP(eski_cp);
    if (eski_giris) SetConsoleCP(eski_giris);
}

#else

static char *yigin_alt;

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

static void segv_yakala(int sig, siginfo_t *info, void *ctx) {
    (void)sig;
    char *adr = (char *)info->si_addr;
    fflush(stdout);
    if (yigin_alt && adr >= yigin_alt - (1 << 20) && adr < yigin_alt + (1 << 16)) {
        derin_ozyineleme(komut_adresi(ctx));
    } else {
        yaz_stderr(BELLEK_HATASI);
    }
    _exit(1);
}

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

#endif
