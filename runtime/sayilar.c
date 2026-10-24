// Kolay çalışma kütüphanesi: sayıların yazıya çevrilmesi, taşma mesajları ve esnek Tam.

// ------------------------------------------------------------ büyük ara sonuçlar (taşma mesajları için)
//
// Taşma mesajında işlemin gerçek sonucu yazılır (yorumlayıcıdaki gibi). Sonuç 128 biti aşabildiği
// için işaret + 256 bitlik büyüklük olarak hesaplanır.

typedef struct {
    uint64_t l[4]; // küçük basamak önce
    int neg;
} Sayi;

static int sayi_sifir(const Sayi *a) { return !(a->l[0] | a->l[1] | a->l[2] | a->l[3]); }

static Sayi sayi(u128 mag, int neg) {
    Sayi s = {{(uint64_t)mag, (uint64_t)(mag >> 64), 0, 0}, 0};
    s.neg = neg && (mag != 0);
    return s;
}

/// Derleyiciden gelen 128 bitlik değer. biçim 0: işaretli, 1: işaretsiz, 2: büyüklük (değer = -bits).
static Sayi sayi_bicim(i128 bits, int32_t bicim) {
    if (bicim == 1) return sayi((u128)bits, 0);
    if (bicim == 2) return sayi((u128)bits, 1);
    if (bits < 0) return sayi((u128)(-(bits + 1)) + 1, 1);
    return sayi((u128)bits, 0);
}

static int buyukluk_kars(const Sayi *a, const Sayi *b) {
    for (int i = 3; i >= 0; i--)
        if (a->l[i] != b->l[i]) return a->l[i] < b->l[i] ? -1 : 1;
    return 0;
}

static Sayi buyukluk_topla(const Sayi *a, const Sayi *b) {
    Sayi r = {{0}, 0};
    u128 c = 0;
    for (int i = 0; i < 4; i++) {
        c += (u128)a->l[i] + b->l[i];
        r.l[i] = (uint64_t)c;
        c >>= 64;
    }
    return r;
}

static Sayi buyukluk_cikar(const Sayi *a, const Sayi *b) { // a >= b
    Sayi r = {{0}, 0};
    uint64_t borc = 0;
    for (int i = 0; i < 4; i++) {
        uint64_t x = a->l[i], y = b->l[i];
        r.l[i] = x - y - borc;
        borc = (x < y) || (x - y < borc);
    }
    return r;
}

static Sayi sayi_topla(Sayi a, Sayi b) {
    Sayi r;
    if (a.neg == b.neg) {
        r = buyukluk_topla(&a, &b);
        r.neg = a.neg;
    } else if (buyukluk_kars(&a, &b) >= 0) {
        r = buyukluk_cikar(&a, &b);
        r.neg = a.neg;
    } else {
        r = buyukluk_cikar(&b, &a);
        r.neg = b.neg;
    }
    if (sayi_sifir(&r)) r.neg = 0;
    return r;
}

static Sayi sayi_carp(Sayi a, Sayi b) {
    Sayi r = {{0}, 0};
    for (int i = 0; i < 2; i++) {
        u128 c = 0;
        for (int j = 0; j < 2; j++) {
            c += (u128)a.l[i] * b.l[j] + r.l[i + j];
            r.l[i + j] = (uint64_t)c;
            c >>= 64;
        }
        r.l[i + 2] += (uint64_t)c;
    }
    r.neg = (a.neg != b.neg) && !sayi_sifir(&r);
    return r;
}

/// 256 bitlik ikiye tümleyen biçimde bit işlemi (& | ^), sonuç işaret + büyüklük
static Sayi sayi_bit(Sayi a, Sayi b, char op) {
    uint64_t x[4], y[4], r[4];
    for (int k = 0; k < 2; k++) {
        Sayi *s = k ? &b : &a;
        uint64_t *o = k ? y : x;
        uint64_t elde = 1;
        for (int i = 0; i < 4; i++) {
            o[i] = s->neg ? ~s->l[i] + elde : s->l[i];
            if (s->neg) elde = elde && o[i] == 0;
        }
    }
    for (int i = 0; i < 4; i++) r[i] = op == '&' ? x[i] & y[i] : op == '|' ? x[i] | y[i] : x[i] ^ y[i];
    Sayi s = {{0}, (int)(r[3] >> 63)};
    uint64_t elde = 1;
    for (int i = 0; i < 4; i++) {
        s.l[i] = s.neg ? ~r[i] + elde : r[i];
        if (s.neg) elde = elde && s.l[i] == 0;
    }
    return s;
}

static void sayi_yazi(Sayi a, char *out) {
    char tmp[100];
    int n = 0;
    do {
        u128 kalan = 0;
        for (int i = 3; i >= 0; i--) {
            u128 cur = (kalan << 64) | a.l[i];
            a.l[i] = (uint64_t)(cur / 10);
            kalan = cur % 10;
        }
        tmp[n++] = (char)('0' + (int)kalan);
    } while (!sayi_sifir(&a));
    int k = 0;
    if (a.neg) out[k++] = '-';
    while (n) out[k++] = tmp[--n];
    out[k] = 0;
}

void tam_yazi_bicim(i128 bits, int32_t bicim, char *out) { sayi_yazi(sayi_bicim(bits, bicim), out); }

// 12345678 -> 12.345.678 (yorumlayıcıdaki gibi; 4 basamağa kadar gruplanmaz)
void grupla(const char *s, char *out) {
    int neg = s[0] == '-';
    const char *d = s + neg;
    int n = (int)strlen(d), k = 0;
    if (neg) out[k++] = '-';
    for (int i = 0; i < n; i++) {
        if (n > 4 && i > 0 && (n - i) % 3 == 0) out[k++] = '.';
        out[k++] = d[i];
    }
    out[k] = 0;
}

// ------------------------------------------------------------ sayıları yazıya çevirme

Yazi *kolay_tam_yazi(i128 bits, int32_t bicim) {
    char s[64];
    tam_yazi_bicim(bits, bicim, s);
    return yazi_c(s);
}

// Rust'taki gibi en kısa, üssüz gösterim: 0.1 + 0.2 -> 0.30000000000000004
static int ayni(double a, double b, int32_t bits) {
    if (bits == 32) return (float)a == (float)b;
    if (bits == 16) return (_Float16)a == (_Float16)b;
    return a == b;
}

void ondalik_yaz(double v, int32_t bits, char *out) {
    if (isnan(v)) { strcpy(out, "TanımsızSayı"); return; }
    if (isinf(v)) { strcpy(out, v > 0 ? "sonsuz" : "-sonsuz"); return; }
    if (bits == 16) {
        // yorumlayıcı ile aynı: sabit basamak sayısıyla en kısa geri dönüşen gösterim
        char buf[64];
        for (int p = 1; p <= 8; p++) {
            snprintf(buf, sizeof buf, "%.*f", p, (double)(_Float16)v);
            if (ayni(strtod(buf, NULL), v, 16)) break;
        }
        int n = (int)strlen(buf);
        while (n > 0 && buf[n - 1] == '0') buf[--n] = 0;
        if (n > 0 && buf[n - 1] == '.') { buf[n++] = '0'; buf[n] = 0; }
        strcpy(out, buf);
        return;
    }
    char sci[64];
    int p;
    for (p = 1; p <= 17; p++) {
        snprintf(sci, sizeof sci, "%.*e", p - 1, v);
        if (ayni(strtod(sci, NULL), v, bits)) break;
    }
    // sci: [-]d.ddde[+-]XX
    char digits[40];
    int nd = 0, k = 0, neg = 0;
    if (sci[k] == '-') { neg = 1; k++; }
    for (; sci[k] && sci[k] != 'e'; k++)
        if (sci[k] != '.') digits[nd++] = sci[k];
    int ex = atoi(sci + k + 1);
    while (nd > 1 && digits[nd - 1] == '0') nd--;
    digits[nd] = 0;
    int o = 0;
    if (neg) out[o++] = '-';
    if (ex >= 0) {
        for (int i = 0; i <= ex; i++) out[o++] = i < nd ? digits[i] : '0';
        out[o++] = '.';
        if (ex + 1 < nd) {
            for (int i = ex + 1; i < nd; i++) out[o++] = digits[i];
        } else {
            out[o++] = '0';
        }
    } else {
        out[o++] = '0';
        out[o++] = '.';
        for (int i = 0; i < -ex - 1; i++) out[o++] = '0';
        for (int i = 0; i < nd; i++) out[o++] = digits[i];
    }
    out[o] = 0;
}

Yazi *kolay_ondalik_yazi(double v, int32_t bits) {
    char out[400];
    ondalik_yaz(v, bits, out);
    return yazi_c(out);
}

// ------------------------------------------------------------ çalışma anı hataları

/// Tam sayı taşması. op: "+", "-", "*", "/", "%" ya da "" (işaret değiştirme: -b)
_Noreturn void kolay_tasma(const char *tip, const char *op, i128 abits, i128 bbits, int32_t bicim, const char *sinir,
                           const char *alt, const char *ust, int32_t pozitif, const char *konum, const char *kod) {
    char sa[100], sb[100], sr[100], ga[140], gb[140], gr[140], msg[900];
    Sayi a = sayi_bicim(abits, bicim), b = sayi_bicim(bbits, bicim), r;
    u128 amag = (u128)a.l[0] | ((u128)a.l[1] << 64), bmag = (u128)b.l[0] | ((u128)b.l[1] << 64);
    sayi_yazi(a, sa);
    sayi_yazi(b, sb);
    grupla(sa, ga);
    grupla(sb, gb);
    switch (op[0]) {
    case '+': r = sayi_topla(a, b); break;
    case '-': b.neg = !b.neg && !sayi_sifir(&b); r = sayi_topla(a, b); break;
    case '*': r = sayi_carp(a, b); break;
    case '/': r = sayi(amag / bmag, a.neg != b.neg); break;
    case '%': r = sayi(amag % bmag, a.neg); break;
    case '&': case '|': case '^': r = sayi_bit(a, b, op[0]); break;
    default: r = b; r.neg = !b.neg && !sayi_sifir(&b); break;
    }
    // üs ve sola kaydırmada sonuç çok büyük olabilir: yazılmaz (yorumlayıcıdaki big_result_msg gibi)
    if (!strcmp(op, "**") || !strcmp(op, "<<")) {
        snprintf(msg, sizeof msg, "%s taştı: %s %s %s sonucu %s sınırlarını aşıyor (%s).", tip, ga, op, gb, tip, sinir);
        kolay_hata(msg, konum, kod);
    }
    sayi_yazi(r, sr);
    grupla(sr, gr);
    int buyuk = !r.neg && !sayi_sifir(&r);
    if (op[0] == 0)
        snprintf(msg, sizeof msg, "%s taştı: -(%s) = %s, ama %s sınırları %s.", tip, gb, gr, tip, sinir);
    else if (buyuk)
        snprintf(msg, sizeof msg, "%s taştı. %s %s %s = %s, ama %s en fazla %s tutabilir.", tip, ga, op, gb, gr, tip, ust);
    else if (pozitif)
        snprintf(msg, sizeof msg, "%s negatif olamaz: %s %s %s = %s.", tip, ga, op, gb, gr);
    else
        snprintf(msg, sizeof msg, "%s taştı. %s %s %s = %s, ama %s en az %s tutabilir.", tip, ga, op, gb, gr, tip, alt);
    kolay_hata(msg, konum, kod);
}

_Noreturn void kolay_sigmiyor(i128 bits, int32_t bicim, const char *tip, const char *sinir, const char *konum, const char *kod) {
    char sv[100], gv[140], msg[600];
    tam_yazi_bicim(bits, bicim, sv);
    grupla(sv, gv);
    snprintf(msg, sizeof msg, "%s değeri %s tipine sığmıyor (%s).", gv, tip, sinir);
    kolay_hata(msg, konum, kod);
}

_Noreturn void kolay_sigmiyor_f(double v, const char *tip, const char *sinir, const char *konum, const char *kod) {
    char sv[400], msg[700];
    ondalik_yaz(v, 64, sv);
    snprintf(msg, sizeof msg, "%s değeri %s tipine sığmıyor (%s).", sv, tip, sinir);
    kolay_hata(msg, konum, kod);
}

_Noreturn void kolay_cevrilemez_f(double v, const char *konum, const char *kod) {
    char sv[400], msg[700];
    ondalik_yaz(v, 64, sv);
    snprintf(msg, sizeof msg, "%s tam sayıya çevrilemez.", sv);
    kolay_hata(msg, konum, kod);
}

/// once + sayı + sonra biçiminde hata (128 bitlik sayı, biçim: bkz. sayi_bicim)
_Noreturn void kolay_sayi_hata(i128 bits, int32_t bicim, const char *once, const char *sonra, const char *konum, const char *kod) {
    char sv[100], msg[400];
    tam_yazi_bicim(bits, bicim, sv);
    snprintf(msg, sizeof msg, "%s%s%s", once, sv, sonra);
    kolay_hata(msg, konum, kod);
}

_Noreturn void kolay_kere_negatif(i128 bits, int32_t bicim, const char *konum, const char *kod) {
    char sv[100], msg[300];
    tam_yazi_bicim(bits, bicim, sv);
    snprintf(msg, sizeof msg, "'kere' için tekrar sayısı negatif olamaz (%s).", sv);
    kolay_hata(msg, konum, kod);
}

// ------------------------------------------------------------ esnek Tam
//
// Esnek Tam 64 bitlik bir kelimedir. En düşük bit 1 ise değer kelimenin içindedir (v >> 1, 63 bit),
// 0 ise kelime bir Buyuk nesnesinin adresidir. Küçük değerlerde hiç bellek ayrılmaz.

static const int64_t ESNEK_UST = 4611686018427387903LL;   // 2^62 - 1
static const int64_t ESNEK_ALT = -4611686018427387904LL;  // -2^62

static inline int esnek_kucuk(int64_t v) { return v & 1; }
static inline int64_t esnek_deger(int64_t v) { return v >> 1; }
static inline int64_t esnek_yap(int64_t x) { return (int64_t)((uint64_t)x << 1) | 1; }

// Geçici büyük sayı: işaret + basamaklar (2^32 tabanı)
typedef struct {
    int isaret;   // -1, 0, 1
    int64_t n;
    uint32_t *r;
    uint32_t yer[4];
} BB;

static void bb_kirp(BB *a) {
    while (a->n > 0 && a->r[a->n - 1] == 0) a->n--;
    if (a->n == 0) a->isaret = 0;
}

static void bb_u128(BB *a, u128 m, int neg) {
    a->r = a->yer;
    a->n = 0;
    while (m) {
        a->r[a->n++] = (uint32_t)m;
        m >>= 32;
    }
    a->isaret = a->n ? (neg ? -1 : 1) : 0;
}

static void bb_esnek(BB *a, int64_t v) {
    if (esnek_kucuk(v)) {
        int64_t x = esnek_deger(v);
        if (x < 0) bb_u128(a, (u128)(-(i128)x), 1);
        else bb_u128(a, (u128)x, 0);
        return;
    }
    Buyuk *b = (Buyuk *)v;
    a->isaret = (int)b->isaret;
    a->n = b->n;
    a->r = b->r;
}

static uint32_t *bb_ayir(int64_t n) { return bellek((size_t)(n ? n : 1) * 4); }

/// Geçici büyük sayıdan esnek Tam (küçükse etiketli, büyükse Buyuk nesnesi). a->r serbest bırakılmaz.
static int64_t bb_esnege(BB *a) {
    bb_kirp(a);
    if (a->n <= 2) {
        u128 m = 0;
        for (int64_t i = a->n - 1; i >= 0; i--) m = (m << 32) | a->r[i];
        if (a->isaret >= 0 && m <= (u128)ESNEK_UST) return esnek_yap((int64_t)m);
        if (a->isaret < 0 && m <= (u128)ESNEK_UST + 1) return esnek_yap((int64_t)(-(i128)m));
    }
    Buyuk *b = bellek(sizeof(Buyuk) + (size_t)a->n * 4);
    b->b.sayac = 1;
    b->b.tip = &kolay_buyuk_tipi;
    b->isaret = a->isaret;
    b->n = a->n;
    memcpy(b->r, a->r, (size_t)a->n * 4);
    return (int64_t)b;
}

static int bb_buyukluk_kars(const BB *a, const BB *b) {
    if (a->n != b->n) return a->n < b->n ? -1 : 1;
    for (int64_t i = a->n - 1; i >= 0; i--)
        if (a->r[i] != b->r[i]) return a->r[i] < b->r[i] ? -1 : 1;
    return 0;
}

static void bb_buyukluk_topla(const BB *a, const BB *b, BB *r) {
    int64_t n = (a->n > b->n ? a->n : b->n) + 1;
    r->r = bb_ayir(n);
    uint64_t c = 0;
    for (int64_t i = 0; i < n; i++) {
        c += (uint64_t)(i < a->n ? a->r[i] : 0) + (i < b->n ? b->r[i] : 0);
        r->r[i] = (uint32_t)c;
        c >>= 32;
    }
    r->n = n;
}

static void bb_buyukluk_cikar(const BB *a, const BB *b, BB *r) { // |a| >= |b|
    r->r = bb_ayir(a->n);
    int64_t borc = 0;
    for (int64_t i = 0; i < a->n; i++) {
        int64_t d = (int64_t)a->r[i] - (i < b->n ? b->r[i] : 0) - borc;
        borc = d < 0;
        r->r[i] = (uint32_t)(d + (borc ? ((int64_t)1 << 32) : 0));
    }
    r->n = a->n;
}

static void bb_topla(const BB *a, const BB *b, BB *r) {
    if (a->isaret == 0) {
        r->r = bb_ayir(b->n);
        memcpy(r->r, b->r, (size_t)b->n * 4);
        r->n = b->n;
        r->isaret = b->isaret;
        return;
    }
    if (b->isaret == 0) {
        r->r = bb_ayir(a->n);
        memcpy(r->r, a->r, (size_t)a->n * 4);
        r->n = a->n;
        r->isaret = a->isaret;
        return;
    }
    if (a->isaret == b->isaret) {
        bb_buyukluk_topla(a, b, r);
        r->isaret = a->isaret;
    } else if (bb_buyukluk_kars(a, b) >= 0) {
        bb_buyukluk_cikar(a, b, r);
        r->isaret = a->isaret;
    } else {
        bb_buyukluk_cikar(b, a, r);
        r->isaret = b->isaret;
    }
    bb_kirp(r);
}

static void bb_carp(const BB *a, const BB *b, BB *r) {
    int64_t n = a->n + b->n;
    r->r = bb_ayir(n);
    memset(r->r, 0, (size_t)(n ? n : 1) * 4);
    for (int64_t i = 0; i < a->n; i++) {
        uint64_t c = 0;
        for (int64_t j = 0; j < b->n; j++) {
            c += (uint64_t)a->r[i] * b->r[j] + r->r[i + j];
            r->r[i + j] = (uint32_t)c;
            c >>= 32;
        }
        int64_t k = i + b->n;
        while (c) {
            c += r->r[k];
            r->r[k++] = (uint32_t)c;
            c >>= 32;
        }
    }
    r->n = n;
    r->isaret = a->isaret * b->isaret;
    bb_kirp(r);
}

/// Bölme sıfıra doğru keser; kalanın işareti bölünenin işaretidir (Rust'taki gibi).
static void bb_bol(const BB *a, const BB *b, BB *q, BB *k) {
    q->r = bb_ayir(a->n);
    q->n = a->n;
    memset(q->r, 0, (size_t)(a->n ? a->n : 1) * 4);
    if (b->n == 1) {
        uint64_t kal = 0;
        for (int64_t i = a->n - 1; i >= 0; i--) {
            uint64_t cur = (kal << 32) | a->r[i];
            q->r[i] = (uint32_t)(cur / b->r[0]);
            kal = cur % b->r[0];
        }
        k->r = bb_ayir(1);
        k->r[0] = (uint32_t)kal;
        k->n = 1;
    } else {
        // bit bit uzun bölme (çok basamaklı bölenler nadir)
        k->r = bb_ayir(b->n + 1);
        k->n = 0;
        for (int64_t i = a->n * 32 - 1; i >= 0; i--) {
            // k = k*2 + bit
            uint32_t tasan = 0;
            for (int64_t j = 0; j < k->n; j++) {
                uint32_t yeni = (k->r[j] >> 31);
                k->r[j] = (k->r[j] << 1) | tasan;
                tasan = yeni;
            }
            if (tasan) k->r[k->n++] = tasan;
            uint32_t bit = (a->r[i / 32] >> (i % 32)) & 1;
            if (bit) {
                if (k->n == 0) k->r[k->n++] = 1;
                else k->r[0] |= 1;
            }
            BB kk = {1, k->n, k->r, {0}}, bb = {1, b->n, b->r, {0}};
            if (bb_buyukluk_kars(&kk, &bb) >= 0) {
                int64_t borc = 0;
                for (int64_t j = 0; j < k->n; j++) {
                    int64_t d = (int64_t)k->r[j] - (j < b->n ? b->r[j] : 0) - borc;
                    borc = d < 0;
                    k->r[j] = (uint32_t)(d + (borc ? ((int64_t)1 << 32) : 0));
                }
                while (k->n > 0 && k->r[k->n - 1] == 0) k->n--;
                q->r[i / 32] |= (uint32_t)1 << (i % 32);
            }
        }
    }
    q->isaret = a->isaret * b->isaret;
    k->isaret = a->isaret;
    bb_kirp(q);
    bb_kirp(k);
}

static void bb_yaz(const BB *a0, Yazi **out) {
    if (a0->isaret == 0) {
        yazi_ekle_c(out, "0", 1);
        return;
    }
    BB a = *a0;
    uint32_t *r = bb_ayir(a.n);
    memcpy(r, a.r, (size_t)a.n * 4);
    a.r = r;
    // 10^9 tabanında basamaklar
    int64_t kap = a.n * 2 + 2, m = 0;
    uint32_t *d = bb_ayir(kap);
    while (a.n > 0) {
        uint64_t kal = 0;
        for (int64_t i = a.n - 1; i >= 0; i--) {
            uint64_t cur = (kal << 32) | a.r[i];
            a.r[i] = (uint32_t)(cur / 1000000000u);
            kal = cur % 1000000000u;
        }
        d[m++] = (uint32_t)kal;
        while (a.n > 0 && a.r[a.n - 1] == 0) a.n--;
    }
    char buf[16];
    if (a0->isaret < 0) yazi_ekle_c(out, "-", 1);
    int k = snprintf(buf, sizeof buf, "%u", d[m - 1]);
    yazi_ekle_c(out, buf, (size_t)k);
    for (int64_t i = m - 2; i >= 0; i--) {
        k = snprintf(buf, sizeof buf, "%09u", d[i]);
        yazi_ekle_c(out, buf, (size_t)k);
    }
    free(r);
    free(d);
}

void esnek_yaz(int64_t v, Yazi **out) {
    if (esnek_kucuk(v)) {
        char buf[32];
        int k = snprintf(buf, sizeof buf, "%lld", (long long)esnek_deger(v));
        yazi_ekle_c(out, buf, (size_t)k);
        return;
    }
    BB a;
    bb_esnek(&a, v);
    bb_yaz(&a, out);
}

static void esnek_metin(int64_t v, char *out, size_t boy) {
    Yazi *y = yazi_ayir(0);
    esnek_yaz(v, &y);
    snprintf(out, boy, "%s", y->d);
    kolay_birak(y);
}

Yazi *kolay_esnek_yazi(int64_t v) {
    Yazi *y = yazi_ayir(0);
    esnek_yaz(v, &y);
    return y;
}

int esnek_kars(int64_t a, int64_t b) {
    if (esnek_kucuk(a) && esnek_kucuk(b)) return (a > b) - (a < b);
    BB x, y;
    bb_esnek(&x, a);
    bb_esnek(&y, b);
    if (x.isaret != y.isaret) return x.isaret < y.isaret ? -1 : 1;
    int c = bb_buyukluk_kars(&x, &y);
    return x.isaret < 0 ? -c : c;
}

int32_t kolay_esnek_kars(int64_t a, int64_t b) { return esnek_kars(a, b); }

uint64_t esnek_ozet(int64_t v) {
    if (esnek_kucuk(v)) return (uint64_t)esnek_deger(v) * 0x9E3779B97F4A7C15ull;
    Buyuk *b = (Buyuk *)v;
    uint64_t h = (uint64_t)b->isaret;
    for (int64_t i = 0; i < b->n; i++) h = (h ^ b->r[i]) * 0x100000001B3ull;
    return h;
}

double esnek_ondalik(int64_t v) {
    if (esnek_kucuk(v)) return (double)esnek_deger(v);
    Buyuk *b = (Buyuk *)v;
    double d = 0;
    for (int64_t i = b->n - 1; i >= 0; i--) d = d * 4294967296.0 + b->r[i];
    return b->isaret < 0 ? -d : d;
}

double kolay_esnek_ondalik(int64_t v) { return esnek_ondalik(v); }

/// Boyutlu Tam'dan esnek Tam (biçim: 0 işaretli, 1 işaretsiz, 2 büyüklük).
int64_t kolay_esnek_tamdan(i128 bits, int32_t bicim) {
    if (bicim == 0 && bits >= ESNEK_ALT && bits <= ESNEK_UST) return esnek_yap((int64_t)bits);
    BB a;
    Sayi s = sayi_bicim(bits, bicim);
    bb_u128(&a, (u128)s.l[0] | ((u128)s.l[1] << 64), s.neg);
    return bb_esnege(&a);
}

/// Yazı ile verilen (çok büyük) sabit: "-123456789012345678901234567890"
int64_t kolay_esnek_metinden(const char *s) {
    int neg = 0;
    if (*s == '-') { neg = 1; s++; }
    BB a = {0, 0, bb_ayir(strlen(s) / 9 + 2), {0}};
    for (; *s; s++) {
        uint64_t c = (uint64_t)(*s - '0');
        for (int64_t i = 0; i < a.n; i++) {
            c += (uint64_t)a.r[i] * 10;
            a.r[i] = (uint32_t)c;
            c >>= 32;
        }
        if (c) a.r[a.n++] = (uint32_t)c;
    }
    a.isaret = neg ? -1 : 1;
    int64_t r = bb_esnege(&a);
    free(a.r);
    return r;
}

// ---- esnek Tam: bit işlemleri, kaydırma, üs (yorumlayıcıdaki num-bigint ile aynı: ikiye tümleyen)

/// Büyüklüğü n kelimelik ikiye tümleyen biçime çevirir (n, büyüklükten en az bir kelime fazla olmalı)
static void bb_tumleyen(const BB *a, uint32_t *o, int64_t n) {
    uint64_t elde = 1;
    for (int64_t i = 0; i < n; i++) {
        uint32_t v = i < a->n ? a->r[i] : 0;
        if (a->isaret < 0) {
            uint64_t t = (uint64_t)(uint32_t)~v + elde;
            o[i] = (uint32_t)t;
            elde = t >> 32;
        } else {
            o[i] = v;
        }
    }
}

static void bb_bit(const BB *x, const BB *y, char op, BB *z) {
    int64_t n = (x->n > y->n ? x->n : y->n) + 1;
    uint32_t *p = bb_ayir(n), *q = bb_ayir(n);
    bb_tumleyen(x, p, n);
    bb_tumleyen(y, q, n);
    for (int64_t i = 0; i < n; i++) p[i] = op == '&' ? p[i] & q[i] : op == '|' ? p[i] | q[i] : p[i] ^ q[i];
    free(q);
    int neg = p[n - 1] >> 31;
    if (neg) { // büyüklüğe geri çevir
        uint64_t elde = 1;
        for (int64_t i = 0; i < n; i++) {
            uint64_t t = (uint64_t)(uint32_t)~p[i] + elde;
            p[i] = (uint32_t)t;
            elde = t >> 32;
        }
    }
    z->r = p;
    z->n = n;
    z->isaret = neg ? -1 : 1;
    bb_kirp(z);
}

static void bb_sola(const BB *x, uint64_t k, BB *z) {
    int64_t kel = (int64_t)(k / 32), bit = (int64_t)(k % 32), n = x->n + kel + 1;
    z->r = bb_ayir(n);
    memset(z->r, 0, (size_t)n * 4);
    for (int64_t i = 0; i < x->n; i++) {
        uint64_t v = (uint64_t)x->r[i] << bit;
        z->r[i + kel] |= (uint32_t)v;
        z->r[i + kel + 1] |= (uint32_t)(v >> 32);
    }
    z->n = n;
    z->isaret = x->isaret;
    bb_kirp(z);
}

/// Sağa kaydırma aşağı yuvarlar (eksi sayılarda -5 >> 1 = -3)
static void bb_saga(const BB *x, uint64_t k, BB *z) {
    int64_t kel = (int64_t)(k / 32), bit = (int64_t)(k % 32);
    int64_t n = x->n - kel > 0 ? x->n - kel : 0;
    z->r = bb_ayir(n + 1);
    memset(z->r, 0, (size_t)(n + 1) * 4);
    int kayip = 0;
    for (int64_t i = 0; i < x->n && i < kel; i++) kayip |= x->r[i] != 0;
    if (kel < x->n && bit) kayip |= (x->r[kel] & ((1u << bit) - 1)) != 0;
    for (int64_t i = 0; i < n; i++) {
        uint64_t v = x->r[i + kel] >> bit;
        if (bit && i + kel + 1 < x->n) v |= (uint64_t)x->r[i + kel + 1] << (32 - bit);
        z->r[i] = (uint32_t)v;
    }
    z->n = n;
    z->isaret = x->isaret;
    if (x->isaret < 0 && kayip) { // büyüklüğe 1 ekle
        uint64_t elde = 1;
        for (int64_t i = 0; i < n && elde; i++) {
            uint64_t t = (uint64_t)z->r[i] + elde;
            z->r[i] = (uint32_t)t;
            elde = t >> 32;
        }
        if (elde) z->r[z->n++] = 1;
    }
    bb_kirp(z);
    if (z->n == 0 && x->isaret < 0 && kayip) { // -1
        z->r[0] = 1;
        z->n = 1;
        z->isaret = -1;
    }
}

static void bb_kopya(const BB *x, BB *z) {
    z->r = bb_ayir(x->n);
    memcpy(z->r, x->r, (size_t)x->n * 4);
    z->n = x->n;
    z->isaret = x->isaret;
}

static void bb_us(const BB *x, uint64_t e, BB *z) {
    BB s, t;
    uint32_t bir = 1;
    BB birb = {1, 1, &bir, {0}};
    bb_kopya(&birb, z);
    bb_kopya(x, &s);
    while (e) {
        if (e & 1) {
            bb_carp(z, &s, &t);
            free(z->r);
            *z = t;
        }
        e >>= 1;
        if (e) {
            bb_carp(&s, &s, &t);
            free(s.r);
            s = t;
        }
    }
    free(s.r);
}

static const char *esnek_op_adi(int64_t op) {
    switch (op) {
    case 'l': return "<<";
    case 'r': return ">>";
    case 'p': return "**";
    default: return NULL;
    }
}

static void bounds_metni(int64_t isaret, char *out) {
    strcpy(out, isaret == 1 ? "0 .. sonsuz" : isaret == 2 ? "-sonsuz .. 0" : "-sonsuz .. sonsuz");
}

/// İşaret kısıtını (Tam(+), Tam(-)) kontrol eder; aşılırsa yorumlayıcıdaki taşma mesajı.
static void esnek_isaret(int64_t r, int64_t isaret, int64_t a, int64_t b, const char *op, const char *tip,
                         const char *konum, const char *kod) {
    if (isaret == 0) return;
    int s = esnek_kars(r, esnek_yap(0));
    if ((isaret == 1 && s >= 0) || (isaret == 2 && s <= 0)) return;
    char sa[2000], sb[2000], sr[2000], ga[2800], gb[2800], gr[2800], msg[9000];
    esnek_metin(a, sa, sizeof sa);
    esnek_metin(b, sb, sizeof sb);
    esnek_metin(r, sr, sizeof sr);
    grupla(sa, ga);
    grupla(sb, gb);
    grupla(sr, gr);
    if (op[0] == 0) {
        char sinir[40];
        bounds_metni(isaret, sinir);
        snprintf(msg, sizeof msg, "%s taştı: -(%s) = %s, ama %s sınırları %s.", tip, gb, gr, tip, sinir);
        kolay_hata(msg, konum, kod);
    }
    if (!strcmp(op, "**") || !strcmp(op, "<<")) { // sonuç çok büyük olabilir: yazılmaz
        char sinir[40];
        bounds_metni(isaret, sinir);
        snprintf(msg, sizeof msg, "%s taştı: %s %s %s sonucu %s sınırlarını aşıyor (%s).", tip, ga, op, gb, tip, sinir);
        kolay_hata(msg, konum, kod);
    }
    if (isaret == 2)
        snprintf(msg, sizeof msg, "%s taştı. %s %s %s = %s, ama %s en fazla 0 tutabilir.", tip, ga, op, gb, gr, tip);
    else
        snprintf(msg, sizeof msg, "%s negatif olamaz: %s %s %s = %s.", tip, ga, op, gb, gr);
    kolay_hata(msg, konum, kod);
}

/// Kaydırma ve üs: eksi ya da çok büyük miktar hatası (yorumlayıcıdaki shift_pow ile aynı mesajlar).
/// Dönüş: miktar (u64'e sığmazsa UINT64_MAX)
static uint64_t esnek_miktar(int64_t op, int64_t a, int64_t b, const char *konum, const char *kod) {
    char sb[2000], msg[2200];
    if (esnek_kars(b, esnek_yap(0)) < 0) {
        esnek_metin(b, sb, sizeof sb);
        if (op == 'p') snprintf(msg, sizeof msg, "Tam sayının üssü negatif olamaz (%s). Ondalık sonuç için: 2.0 ** -1", sb);
        else snprintf(msg, sizeof msg, "Kaydırma miktarı negatif olamaz (%s).", sb);
        kolay_hata(msg, konum, kod);
    }
    uint64_t n = UINT64_MAX;
    if (esnek_kucuk(b)) n = (uint64_t)esnek_deger(b);
    else {
        BB y;
        bb_esnek(&y, b);
        if (y.n <= 2) n = (uint64_t)y.r[0] | (y.n == 2 ? (uint64_t)y.r[1] << 32 : 0);
    }
    int sifir = esnek_kars(a, esnek_yap(0)) == 0;
    int kucuk_taban = sifir || esnek_kars(a, esnek_yap(1)) == 0 || esnek_kars(a, esnek_yap(-1)) == 0;
    if (n > UINT32_MAX && ((op == 'l' && !sifir) || (op == 'p' && !kucuk_taban))) {
        esnek_metin(b, sb, sizeof sb);
        snprintf(msg, sizeof msg, op == 'p' ? "Üs çok büyük (%s)." : "Kaydırma miktarı çok büyük (%s).", sb);
        kolay_hata(msg, konum, kod);
    }
    return n;
}

/// Esnek Tam işlemi. op: '+', '-', '*', '/', '%', '&', '|', '^', 'l' (<<), 'r' (>>), 'p' (**)
int64_t kolay_esnek_islem(int64_t op, int64_t a, int64_t b, int64_t isaret, const char *tip, const char *konum,
                          const char *kod) {
    char ops[2] = {(char)op, 0};
    const char *opad = esnek_op_adi(op) ? esnek_op_adi(op) : ops;
    if ((op == '/' || op == '%') && esnek_kars(b, esnek_yap(0)) == 0) kolay_hata("Sıfıra bölme hatası.", konum, kod);
    int64_t r;
    if (op == 'l' || op == 'r' || op == 'p') {
        uint64_t n = esnek_miktar(op, a, b, konum, kod);
        BB x, z;
        bb_esnek(&x, a);
        if (op == 'p' && x.n <= 1 && (x.n == 0 || x.r[0] == 1)) n = n == 0 ? 0 : (n % 2) + 2; // taban 0, 1, -1
        if (op == 'l' && x.isaret == 0) n = 0;
        if (op == 'r' && n > ((uint64_t)1 << 40)) n = (uint64_t)1 << 40;
        if (op == 'l') bb_sola(&x, n, &z);
        else if (op == 'r') bb_saga(&x, n, &z);
        else bb_us(&x, n, &z);
        r = bb_esnege(&z);
        free(z.r);
    } else if (esnek_kucuk(a) && esnek_kucuk(b)) {
        int64_t x = esnek_deger(a), y = esnek_deger(b);
        i128 s;
        switch (op) {
        case '+': s = (i128)x + y; break;
        case '-': s = (i128)x - y; break;
        case '*': s = (i128)x * y; break;
        case '/': s = (i128)x / y; break;
        case '&': s = x & y; break;
        case '|': s = x | y; break;
        case '^': s = x ^ y; break;
        default: s = (i128)x % y; break;
        }
        if (s >= ESNEK_ALT && s <= ESNEK_UST) {
            r = esnek_yap((int64_t)s);
        } else {
            BB t;
            bb_u128(&t, s < 0 ? (u128)(-s) : (u128)s, s < 0);
            r = bb_esnege(&t);
        }
    } else {
        BB x, y, z, k;
        bb_esnek(&x, a);
        bb_esnek(&y, b);
        switch (op) {
        case '+': bb_topla(&x, &y, &z); break;
        case '-': y.isaret = -y.isaret; bb_topla(&x, &y, &z); break;
        case '*': bb_carp(&x, &y, &z); break;
        case '&': case '|': case '^': bb_bit(&x, &y, (char)op, &z); break;
        default:
            bb_bol(&x, &y, &z, &k);
            if (op == '%') {
                free(z.r);
                z = k;
            } else {
                free(k.r);
            }
            break;
        }
        r = bb_esnege(&z);
        free(z.r);
    }
    esnek_isaret(r, isaret, a, b, opad, tip, konum, kod);
    return r;
}

int64_t kolay_esnek_neg(int64_t a, int64_t isaret, const char *tip, const char *konum, const char *kod) {
    int64_t r = kolay_esnek_islem('-', esnek_yap(0), a, 0, tip, konum, kod);
    esnek_isaret(r, isaret, esnek_yap(0), a, "", tip, konum, kod);
    return r;
}

/// Esnek Tam'ı boyutlu Tam'a çevirir (olarak). Sonuç 128 bitlik biçimde: işaretsiz hedefte (Tam(128, +))
/// işaretsiz, Tam(128, -) hedefinde büyüklük (eksi değer), diğerlerinde işaretli.
i128 kolay_esnek_boyutluya(int64_t a, int64_t bit, int64_t isaret, const char *tip, const char *sinir,
                           const char *konum, const char *kod) {
    BB x;
    bb_esnek(&x, a);
    int sigar = x.n <= 4;
    u128 m = 0;
    if (sigar)
        for (int64_t i = x.n - 1; i >= 0; i--) m = (m << 32) | x.r[i];
    if (sigar) {
        if (isaret == 1) {
            u128 ust = bit >= 128 ? ~(u128)0 : (((u128)1 << bit) - 1);
            sigar = x.isaret >= 0 && m <= ust;
        } else if (isaret == 2) {
            u128 ust = bit >= 128 ? ~(u128)0 : (((u128)1 << bit) - 1);
            sigar = x.isaret <= 0 && m <= ust;
        } else {
            u128 yarim = (u128)1 << (bit - 1);
            sigar = x.isaret >= 0 ? m < yarim : m <= yarim;
        }
    }
    if (!sigar) {
        char sv[2000], gv[2800], msg[3200];
        esnek_metin(a, sv, sizeof sv);
        grupla(sv, gv);
        snprintf(msg, sizeof msg, "%s değeri %s tipine sığmıyor (%s).", gv, tip, sinir);
        kolay_hata(msg, konum, kod);
    }
    if (isaret == 1) return (i128)m;
    if (isaret == 2 && bit >= 128) return (i128)m; // büyüklük biçimi
    return x.isaret < 0 ? -(i128)m : (i128)m;
}

_Noreturn void kolay_sigmiyor_esnek(int64_t v, const char *tip, const char *sinir, const char *konum, const char *kod) {
    char sv[2000], gv[2800], msg[3200];
    esnek_metin(v, sv, sizeof sv);
    grupla(sv, gv);
    snprintf(msg, sizeof msg, "%s değeri %s tipine sığmıyor (%s).", gv, tip, sinir);
    kolay_hata(msg, konum, kod);
}

/// Esnek Tam'ın işaret kısıtına uyup uymadığını kontrol eder (olarak Tam(+) gibi).
int64_t kolay_esnek_kisitla(int64_t a, int64_t isaret, const char *tip, const char *sinir, const char *konum,
                            const char *kod) {
    int s = esnek_kars(a, esnek_yap(0));
    if ((isaret == 1 && s < 0) || (isaret == 2 && s > 0)) {
        char sv[2000], gv[2800], msg[3200];
        esnek_metin(a, sv, sizeof sv);
        grupla(sv, gv);
        snprintf(msg, sizeof msg, "%s değeri %s tipine sığmıyor (%s).", gv, tip, sinir);
        kolay_hata(msg, konum, kod);
    }
    return a;
}

/// Ondalık sayıdan esnek Tam (olarak Tam): sıfıra doğru keser.
int64_t kolay_esnek_ondaliktan(double v, const char *konum, const char *kod) {
    if (!isfinite(v)) kolay_cevrilemez_f(v, konum, kod);
    double t = trunc(v);
    if (t >= -4611686018427387904.0 && t < 4611686018427387904.0) return esnek_yap((int64_t)t);
    int neg = t < 0;
    if (neg) t = -t;
    int ust;
    double m = frexp(t, &ust); // t = m * 2^ust, 0.5 <= m < 1
    uint64_t mant = (uint64_t)ldexp(m, 64);
    int kaydir = ust - 64;
    int64_t n = (kaydir + 64) / 32 + 2;
    BB a = {neg ? -1 : 1, n, bb_ayir(n), {0}};
    memset(a.r, 0, (size_t)n * 4);
    for (int i = 0; i < 64; i++) {
        if ((mant >> i) & 1) {
            int64_t b = i + kaydir;
            if (b >= 0) a.r[b / 32] |= (uint32_t)1 << (b % 32);
        }
    }
    int64_t r = bb_esnege(&a);
    free(a.r);
    return r;
}

/// Döngü sayısı / indeks için 64 bitlik değer. Sığmıyorsa yorumlayıcıdaki gibi hata.
int64_t kolay_esnek_i64(int64_t a, const char *msg, const char *konum, const char *kod) {
    if (esnek_kucuk(a)) return esnek_deger(a);
    BB x;
    bb_esnek(&x, a);
    if (x.n <= 2) {
        u128 m = 0;
        for (int64_t i = x.n - 1; i >= 0; i--) m = (m << 32) | x.r[i];
        if (x.isaret > 0 && m <= (u128)INT64_MAX) return (int64_t)m;
        if (x.isaret < 0 && m <= (u128)INT64_MAX + 1) return (int64_t)(-(i128)m);
    }
    kolay_hata(msg, konum, kod);
}

/// 'kere' için tekrar sayısı: negatifse hata, çok büyükse (zaten bitmeyecek) en büyük sayı.
int64_t kolay_esnek_kere(int64_t a, const char *konum, const char *kod) {
    if (esnek_kars(a, esnek_yap(0)) < 0) {
        char sv[2000], msg[2200];
        esnek_metin(a, sv, sizeof sv);
        snprintf(msg, sizeof msg, "'kere' için tekrar sayısı negatif olamaz (%s).", sv);
        kolay_hata(msg, konum, kod);
    }
    if (esnek_kucuk(a)) return esnek_deger(a);
    return INT64_MAX;
}
