// Kolay çalışma kütüphanesi: her tip için ortak işlemler (eşitlik, özet, yazıya çevirme,
// sıralama, varsayılan değer). Değerler depolama biçiminde, adresleriyle verilir.

/// Depolanmış boyutlu Tam'ı okur. *bicim: 0 işaretli, 1 işaretsiz, 2 büyüklük (bkz. sayilar.c)
static i128 tam_oku(const void *p, const Tip *t, int32_t *bicim) {
    int64_t w = t->bit + (t->isaret == 2);
    *bicim = 0;
    if (w > 128) {
        u128 m;
        memcpy(&m, p, 16);
        *bicim = 2;
        return (i128)m;
    }
    if (t->isaret == 1) {
        if (w <= 8) return *(const uint8_t *)p;
        if (w <= 16) return *(const uint16_t *)p;
        if (w <= 32) return *(const uint32_t *)p;
        if (w <= 64) return *(const uint64_t *)p;
        u128 m;
        memcpy(&m, p, 16);
        *bicim = 1;
        return (i128)m;
    }
    if (w <= 8) return *(const int8_t *)p;
    if (w <= 16) return *(const int16_t *)p;
    if (w <= 32) return *(const int32_t *)p;
    if (w <= 64) return *(const int64_t *)p;
    i128 v;
    memcpy(&v, p, 16);
    return v;
}

static double ondalik_oku(const void *p, const Tip *t) {
    if (t->bit == 16) return (double)*(const _Float16 *)p;
    if (t->bit == 32) return (double)*(const float *)p;
    return *(const double *)p;
}

static int sayisal(const Tip *t) { return t->tur == T_TAM || t->tur == T_ESNEK || t->tur == T_ONDALIK; }

static double sayi_ondalik(const void *p, const Tip *t) {
    if (t->tur == T_ONDALIK) return ondalik_oku(p, t);
    if (t->tur == T_ESNEK) return esnek_ondalik(*(const int64_t *)p);
    int32_t b;
    i128 v = tam_oku(p, t, &b);
    if (b == 1) return (double)(u128)v;
    if (b == 2) return -(double)(u128)v;
    return (double)v;
}

/// Nesne türleri (tür, seçenek, arayüz) için değer bir işaretçidir; gerçek tipi nesnenin başlığındadır.

// ------------------------------------------------------------ eşitlik

_Bool kolay_esit(void *a, void *b, const Tip *t) {
    switch (t->tur) {
    case T_TAM: {
        int32_t ba, bb;
        i128 x = tam_oku(a, t, &ba), y = tam_oku(b, t, &bb);
        return x == y;
    }
    case T_ESNEK:
        return esnek_kars(*(int64_t *)a, *(int64_t *)b) == 0;
    case T_ONDALIK:
        return ondalik_oku(a, t) == ondalik_oku(b, t);
    case T_MANTIK:
        return (*(uint8_t *)a != 0) == (*(uint8_t *)b != 0);
    case T_YAZI:
        return kolay_yazi_esit(*(Yazi **)a, *(Yazi **)b);
    case T_HIC:
        return 1;
    case T_ARALIK:
        return memcmp(a, b, sizeof(Aralik)) == 0;
    case T_LISTE: {
        Liste *x = *(Liste **)a, *y = *(Liste **)b;
        if (x == y) return 1;
        if (x->uzunluk != y->uzunluk) return 0;
        const Tip *e = t->e1;
        for (int64_t i = 0; i < x->uzunluk; i++)
            if (!kolay_esit(x->veri + i * e->boyut, y->veri + i * e->boyut, e)) return 0;
        return 1;
    }
    case T_SOZLUK:
        return sozluk_esit(*(void **)a, *(void **)b);
    case T_TUR: case T_SECENEK: case T_ARAYUZ: {
        Baslik *x = *(Baslik **)a, *y = *(Baslik **)b;
        if (x == y) return 1;
        if (x->tip != y->tip) return 0;
        const Tip *g = x->tip;
        if (g->tur == T_TUR) {
            for (int64_t i = 0; i < g->n; i++)
                if (!kolay_esit((char *)x + g->ofsetler[i], (char *)y + g->ofsetler[i], g->tipler[i])) return 0;
            return 1;
        }
        int64_t ex = *(int64_t *)((char *)x + 16), ey = *(int64_t *)((char *)y + 16);
        if (ex != ey) return 0;
        const Varyant *v = &g->varyantlar[ex];
        for (int64_t i = 0; i < v->n; i++)
            if (!kolay_esit((char *)x + v->ofsetler[i], (char *)y + v->ofsetler[i], v->tipler[i])) return 0;
        return 1;
    }
    case T_VEYA: {
        int64_t ex = *(int64_t *)a, ey = *(int64_t *)b;
        const Tip *ux = t->tipler[ex], *uy = t->tipler[ey];
        if (ex == ey) return kolay_esit((char *)a + 8, (char *)b + 8, ux);
        // Tam ile Ondalık sayı olarak karşılaştırılır (yorumlayıcıdaki gibi)
        if (sayisal(ux) && sayisal(uy)) return sayi_ondalik((char *)a + 8, ux) == sayi_ondalik((char *)b + 8, uy);
        return 0;
    }
    default:
        return 0;
    }
}

// ------------------------------------------------------------ özet (sözlük anahtarları için)

uint64_t deger_ozet(const void *p, const Tip *t) {
    switch (t->tur) {
    case T_TAM: {
        int32_t b;
        i128 v = tam_oku(p, t, &b);
        uint64_t lo = (uint64_t)v, hi = (uint64_t)((u128)v >> 64);
        return (lo ^ (hi * 0x9E3779B97F4A7C15ull)) * 0xBF58476D1CE4E5B9ull;
    }
    case T_ESNEK:
        return esnek_ozet(*(const int64_t *)p);
    case T_MANTIK:
        return *(const uint8_t *)p ? 0x51ED27 : 0x2F;
    case T_YAZI: {
        const Yazi *y = *(Yazi *const *)p;
        uint64_t h = 0xCBF29CE484222325ull;
        for (int64_t i = 0; i < y->uzunluk; i++) h = (h ^ (unsigned char)y->d[i]) * 0x100000001B3ull;
        return h;
    }
    case T_SECENEK: case T_ARAYUZ: case T_TUR: {
        const Baslik *x = *(Baslik *const *)p;
        const Tip *g = x->tip;
        uint64_t h = (uint64_t)g->no * 0x9E3779B97F4A7C15ull;
        if (g->tur == T_SECENEK) {
            int64_t e = *(const int64_t *)((const char *)x + 16);
            h ^= (uint64_t)e * 0xBF58476D1CE4E5B9ull;
            const Varyant *v = &g->varyantlar[e];
            for (int64_t i = 0; i < v->n; i++) h = (h ^ deger_ozet((const char *)x + v->ofsetler[i], v->tipler[i])) * 0x100000001B3ull;
        }
        return h;
    }
    default:
        return 0;
    }
}

// ------------------------------------------------------------ yazıya çevirme (yorumlayıcıyla aynı biçim)

static void tirnakla(Yazi **out, const Yazi *s) {
    yazi_ekle_c(out, "\"", 1);
    for (int64_t i = 0; i < s->uzunluk; i++) {
        char c = s->d[i];
        switch (c) {
        case '"': yazi_ekle_c(out, "\\\"", 2); break;
        case '\\': yazi_ekle_c(out, "\\\\", 2); break;
        case '\n': yazi_ekle_c(out, "\\n", 2); break;
        case '\t': yazi_ekle_c(out, "\\t", 2); break;
        default: yazi_ekle_c(out, &c, 1);
        }
    }
    yazi_ekle_c(out, "\"", 1);
}

static void cyaz(Yazi **out, const char *s) { yazi_ekle_c(out, s, strlen(s)); }

/// ust: en dış değer mi (yazılar tırnaksız yazılır); ipucu: Ondalık boyutu tipten alınsın mı
/// (yorumlayıcıda 'veya' içindeki değerler için ipucu yoktur, Ondalık 64 bit gibi yazılır).
void deger_yaz(Yazi **out, const void *p, const Tip *t, int ust, int ipucu) {
    char buf[400];
    switch (t->tur) {
    case T_TAM: {
        int32_t b;
        i128 v = tam_oku(p, t, &b);
        tam_yazi_bicim(v, b, buf);
        cyaz(out, buf);
        break;
    }
    case T_ESNEK:
        esnek_yaz(*(const int64_t *)p, out);
        break;
    case T_ONDALIK:
        ondalik_yaz(ondalik_oku(p, t), ipucu ? (int32_t)t->bit : 64, buf);
        cyaz(out, buf);
        break;
    case T_MANTIK:
        cyaz(out, *(const uint8_t *)p ? "doğru" : "yanlış");
        break;
    case T_YAZI: {
        const Yazi *y = *(Yazi *const *)p;
        if (ust) yazi_ekle_c(out, y->d, (size_t)y->uzunluk);
        else tirnakla(out, y);
        break;
    }
    case T_HIC:
        cyaz(out, "Hiç");
        break;
    case T_ARALIK: {
        const Aralik *a = p;
        const char *op = (a->bayrak & 2) ? "..=" : "..";
        char son[32] = "";
        if (a->bayrak & 1) snprintf(son, sizeof son, "%lld", (long long)a->end);
        if (a->step == 1) snprintf(buf, sizeof buf, "%lld%s%s", (long long)a->start, op, son);
        else snprintf(buf, sizeof buf, "(%lld%s%s).adım(%lld)", (long long)a->start, op, son, (long long)a->step);
        cyaz(out, buf);
        break;
    }
    case T_LISTE: {
        const Liste *l = *(Liste *const *)p;
        const Tip *e = t->e1;
        cyaz(out, "[");
        for (int64_t i = 0; i < l->uzunluk; i++) {
            if (i) cyaz(out, ", ");
            deger_yaz(out, l->veri + i * e->boyut, e, 0, ipucu);
        }
        cyaz(out, "]");
        break;
    }
    case T_SOZLUK:
        sozluk_yaz(out, *(void *const *)p, ipucu);
        break;
    case T_TUR: case T_SECENEK: case T_ARAYUZ: {
        const Baslik *x = *(Baslik *const *)p;
        const Tip *g = x->tip;
        cyaz(out, g->ad);
        if (g->tur == T_TUR) {
            cyaz(out, "(");
            for (int64_t i = 0; i < g->n; i++) {
                if (i) cyaz(out, ", ");
                cyaz(out, g->adlar[i]);
                cyaz(out, ": ");
                deger_yaz(out, (const char *)x + g->ofsetler[i], g->tipler[i], 0, 1);
            }
            cyaz(out, ")");
            break;
        }
        const Varyant *v = &g->varyantlar[*(const int64_t *)((const char *)x + 16)];
        cyaz(out, ".");
        cyaz(out, v->ad);
        if (v->n) {
            cyaz(out, "(");
            for (int64_t i = 0; i < v->n; i++) {
                if (i) cyaz(out, ", ");
                cyaz(out, v->adlar[i]);
                cyaz(out, ": ");
                deger_yaz(out, (const char *)x + v->ofsetler[i], v->tipler[i], 0, 1);
            }
            cyaz(out, ")");
        }
        break;
    }
    case T_VEYA:
        deger_yaz(out, (const char *)p + 8, t->tipler[*(const int64_t *)p], ust, 0);
        break;
    default:
        break;
    }
}

/// Değeri yazıya çevirir (yazdır, f-yazı, yazıya()).
Yazi *kolay_yaziya(void *p, const Tip *t) {
    Yazi *y = yazi_ayir(0);
    deger_yaz(&y, p, t, 1, 1);
    return y;
}

// ------------------------------------------------------------ sıralama karşılaştırması

/// Sayı ve yazılar için tam düzen. TanımsızSayı her sayıdan büyük sayılır.
int deger_kars(const void *a, const void *b, const Tip *t) {
    switch (t->tur) {
    case T_TAM: {
        int32_t ba, bb;
        i128 x = tam_oku(a, t, &ba), y = tam_oku(b, t, &bb);
        if (ba == 1) return ((u128)x > (u128)y) - ((u128)x < (u128)y);
        if (ba == 2) return ((u128)y > (u128)x) - ((u128)y < (u128)x);
        return (x > y) - (x < y);
    }
    case T_ESNEK:
        return esnek_kars(*(const int64_t *)a, *(const int64_t *)b);
    case T_ONDALIK: {
        double x = ondalik_oku(a, t), y = ondalik_oku(b, t);
        if (isnan(x) || isnan(y)) return isnan(x) - isnan(y);
        return (x > y) - (x < y);
    }
    case T_YAZI:
        return kolay_yazi_kars(*(Yazi *const *)a, *(Yazi *const *)b);
    case T_MANTIK:
        return (int)*(const uint8_t *)a - (int)*(const uint8_t *)b;
    default:
        return 0;
    }
}

// ------------------------------------------------------------ varsayılan değerler ve kopyalama

void kolay_varsayilan(void *out, const Tip *t) {
    memset(out, 0, (size_t)t->boyut);
    switch (t->tur) {
    case T_ESNEK:
        *(int64_t *)out = 1; // etiketli 0
        break;
    case T_YAZI:
        *(Yazi **)out = bos_yazi();
        break;
    case T_LISTE: {
        int64_t n = t->sabit_boy > 0 ? t->sabit_boy : 0;
        Liste *l = kolay_liste_yeni(t, n);
        for (int64_t i = 0; i < n; i++) kolay_varsayilan(l->veri + i * t->e1->boyut, t->e1);
        *(Liste **)out = l;
        break;
    }
    case T_SOZLUK:
        *(void **)out = kolay_sozluk_yeni(t);
        break;
    case T_SECENEK:
        *(void **)out = t->varyantlar[0].sabit;
        break;
    case T_VEYA: {
        int64_t etiket = 0;
        for (int64_t i = 0; i < t->n; i++)
            if (t->tipler[i]->tur == T_HIC) etiket = i;
        *(int64_t *)out = etiket;
        if (t->tipler[etiket]->tur != T_HIC) kolay_varsayilan((char *)out + 8, t->tipler[etiket]);
        break;
    }
    default:
        break;
    }
}

void deger_kopyala_tut(void *hedef, const void *kaynak, const Tip *t) {
    memcpy(hedef, kaynak, (size_t)t->boyut);
    kolay_deger_tut(hedef, t);
}
