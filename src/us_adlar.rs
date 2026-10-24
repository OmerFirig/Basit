// `klavye("us")` dosyalarında Türkçe harfli adlara erişim.
//
// us modunda Türkçe harf yazılamadığı için `tr` modundaki bir kütüphanenin `karekök` fonksiyonu
// `m.karekok` diye yazılır. Ayrıştırmadan sonra, us modundaki her dosyanın ağacında, dosyanın
// kendisinde tanımlanmamış ve dahil ettiği tr dosyalarında da tam karşılığı olmayan her ad,
// Türkçe harfleri sadeleştirilince eşit olduğu adla değiştirilir (karekok → karekök).
// - Dosyanın kendi adları (değişken, fonksiyon, tür, alan, parametre...) hiç değiştirilmez.
//   Metot adları hariç: bir tr arayüzünü ya da ebeveyni uygulayan metot (`ciz` → `çiz`) her yerde
//   birlikte değiştiği için tutarlı kalır.
// - Sadeleştirince birden çok ada denk geliyorsa (`seker`: `şeker` ve `seker`) tahmin yapılmaz, hata verilir.
// - tr modundaki dosyalar ve tam bulunan adlar değişmez ("şeker ile seker iki ayrı addır" kuralı geçerli).

use crate::ast::*;
use crate::diag::{CResult, Diag, Span};
use crate::lexer::Mode;
use crate::loader::ParsedModule;
use std::collections::{HashMap, HashSet};

/// Türkçe harfleri İngilizce klavyedeki karşılıklarına çevirir: karekök → karekok
pub fn sadelestir(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ç' => 'c',
            'Ç' => 'C',
            'ğ' => 'g',
            'Ğ' => 'G',
            'ı' | 'î' => 'i',
            'İ' | 'Î' => 'I',
            'ö' => 'o',
            'Ö' => 'O',
            'ş' => 's',
            'Ş' => 'S',
            'ü' | 'û' => 'u',
            'Ü' | 'Û' => 'U',
            'â' => 'a',
            'Â' => 'A',
            c => c,
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Tur {
    /// Dosyanın kendi tanımladığı ad (değişmez)
    Tanim,
    /// Dosyanın kendi tanımladığı, `.` ile ya da adlı argümanla erişilen ad: alan, parametre, seçenek
    TanimUye,
    /// Metot tanımı: tr'deki bir metotla eşleşiyorsa kullanımlarıyla birlikte değişir
    Metot,
    /// Kullanım
    Ad,
    /// `.` ile (x.ad, m.ad, Renk.Kırmızı) ya da adlı argümanla (f(ad: 1)) kullanım: dosyanın kendi
    /// değişkenleri bu adı gizlemez, sadece kendi alanları, parametreleri ve seçenekleri gizler
    Uye,
}

/// us modundaki modüllerde Türkçe harfli adları çözer (bkz. dosya başı).
pub fn coz(mods: &mut [ParsedModule]) -> CResult<()> {
    for i in 0..mods.len() {
        if mods[i].mode != Mode::Us {
            continue;
        }
        // dahil ettiği (dolaylı olanlar dahil) tr modülleri
        let mut gorulen = HashSet::new();
        let mut yigin: Vec<usize> = mods[i].deps.iter().map(|d| d.1).collect();
        let mut yabanci: HashSet<String> = HashSet::new();
        while let Some(m) = yigin.pop() {
            if !gorulen.insert(m) {
                continue;
            }
            yigin.extend(mods[m].deps.iter().map(|d| d.1));
            if mods[m].mode == Mode::Tr {
                acik_adlar(&mods[m].stmts, &mut yabanci);
            }
        }
        // sadeleştirilmiş ad → tr'deki adlar (sadece Türkçe harf içerenler ya da onlarla çakışanlar önemli)
        let mut esler: HashMap<String, Vec<String>> = HashMap::new();
        for a in &yabanci {
            esler.entry(sadelestir(a)).or_default().push(a.clone());
        }
        esler.retain(|k, v| v.len() > 1 || v[0] != *k);
        if esler.is_empty() {
            continue;
        }
        for v in esler.values_mut() {
            v.sort();
        }
        let (mut kendi, mut kendi_uye) = (HashSet::new(), HashSet::new());
        gez_blok(&mut mods[i].stmts, &mut |ad, _, t| {
            if t == Tur::TanimUye {
                kendi_uye.insert(ad.clone());
            }
            if matches!(t, Tur::Tanim | Tur::TanimUye) {
                kendi.insert(ad.clone());
            }
        });
        let mut hata: Option<Diag> = None;
        gez_blok(&mut mods[i].stmts, &mut |ad, span, t| {
            let gizli = match t {
                Tur::Tanim | Tur::TanimUye => true,
                Tur::Ad => kendi.contains(ad.as_str()),
                Tur::Uye => kendi_uye.contains(ad.as_str()),
                Tur::Metot => false,
            };
            if gizli || hata.is_some() {
                return;
            }
            let Some(adaylar) = esler.get(&sadelestir(ad)) else { return };
            match adaylar.as_slice() {
                [tek] => *ad = tek.clone(),
                _ => {
                    let liste: Vec<String> = adaylar.iter().map(|a| format!("'{a}'")).collect();
                    hata = Some(Diag::new(
                        span,
                        format!(
                            "'{ad}' dahil edilen dosyalarda birden çok ada karşılık geliyor: {}. Türkçe karakterlerle doğru adı yazmak için dosyanın başındaki klavye(\"us\") satırını kaldırın.",
                            liste.join(", ")
                        ),
                    ))
                }
            }
        });
        if let Some(d) = hata {
            return Err(d);
        }
    }
    Ok(())
}

/// Bir modülün dışarıdan kullanılabilen adları: üst düzey tanımlar, türlerin alanları ve metotları,
/// seçenekler, parametre adları (adlı argümanlar için).
fn acik_adlar(stmts: &[Stmt], out: &mut HashSet<String>) {
    let params = |ps: &[Param], out: &mut HashSet<String>| out.extend(ps.iter().map(|p| p.name.clone()));
    let fonk = |f: &FnDecl, out: &mut HashSet<String>| {
        out.insert(f.name.clone());
        params(&f.params, out);
    };
    let ata = |s: &Stmt, out: &mut HashSet<String>| {
        if let StmtKind::Ata { name, .. } = &s.kind {
            out.insert(name.clone());
        }
    };
    for s in stmts {
        match &s.kind {
            StmtKind::Ata { .. } => ata(s, out),
            StmtKind::Fonksiyon(f) => fonk(f, out),
            StmtKind::Tur(t) => {
                out.insert(t.name.clone());
                params(&t.fields, out);
                t.methods.iter().chain(&t.ortak_fonk).for_each(|f| fonk(f, out));
                t.alanlar.iter().chain(&t.ortak).for_each(|s| ata(s, out));
            }
            StmtKind::Arayuz(a) => {
                out.insert(a.name.clone());
                for s in &a.sigs {
                    out.insert(s.name.clone());
                    params(&s.params, out);
                }
            }
            StmtKind::Secenek(d) => {
                out.insert(d.name.clone());
                for v in &d.variants {
                    out.insert(v.name.clone());
                    params(&v.fields, out);
                }
                d.methods.iter().for_each(|f| fonk(f, out));
            }
            StmtKind::Dahil { alias: Some((a, _)), .. } => {
                out.insert(a.clone());
            }
            StmtKind::Dis { alias, funcs, .. } => {
                out.extend(alias.iter().map(|a| a.0.clone()));
                for f in funcs {
                    out.insert(f.name.clone());
                    params(&f.params, out);
                }
            }
            _ => {}
        }
    }
}

// ------------------------------------------------------------ ağacı gezme

type Ziyaret<'a> = dyn FnMut(&mut String, Span, Tur) + 'a;

fn gez_blok(b: &mut [Stmt], f: &mut Ziyaret) {
    for s in b {
        gez_komut(s, f);
    }
}

fn gez_fonk(d: &mut FnDecl, metot: bool, f: &mut Ziyaret) {
    f(&mut d.name, d.span, if metot { Tur::Metot } else { Tur::Tanim });
    for (g, sp) in &mut d.generics {
        f(g, *sp, Tur::Tanim);
    }
    gez_params(&mut d.params, f);
    if let Some(r) = &mut d.ret {
        gez_tip(r, f);
    }
    if let Yeniden::Ebeveyn(p, sp) = &mut d.yeniden {
        f(p, *sp, Tur::Ad);
    }
    gez_blok(&mut d.body, f);
}

fn gez_params(ps: &mut [Param], f: &mut Ziyaret) {
    for p in ps {
        f(&mut p.name, p.span, Tur::TanimUye);
        gez_tip(&mut p.ty, f);
        if let Some(v) = &mut p.varsayilan {
            gez_ifade(v, f);
        }
    }
}

fn gez_yol(path: &mut [String], span: Span, f: &mut Ziyaret) {
    for (i, p) in path.iter_mut().enumerate() {
        f(p, span, if i == 0 { Tur::Ad } else { Tur::Uye });
    }
}

fn gez_tip(t: &mut TypeExpr, f: &mut Ziyaret) {
    let span = t.span;
    match &mut t.kind {
        TypeKind::Path { path, generics, .. } => {
            gez_yol(path, span, f);
            generics.iter_mut().for_each(|g| gez_tip(g, f));
        }
        TypeKind::Union(ts) => ts.iter_mut().for_each(|g| gez_tip(g, f)),
        TypeKind::Func { generics, params, ret } => {
            for g in generics {
                f(g, span, Tur::Tanim);
            }
            params.iter_mut().for_each(|g| gez_tip(g, f));
            if let Some(r) = ret {
                gez_tip(r, f);
            }
        }
    }
}

fn gez_komut(s: &mut Stmt, f: &mut Ziyaret) {
    match &mut s.kind {
        StmtKind::Ata { name, name_span, ty, value, .. } => {
            f(name, *name_span, Tur::Tanim);
            if let Some(t) = ty {
                gez_tip(t, f);
            }
            if let Some(v) = value {
                gez_ifade(v, f);
            }
        }
        StmtKind::Assign { target, value, .. } => {
            gez_ifade(target, f);
            gez_ifade(value, f);
        }
        StmtKind::Expr(e) | StmtKind::Firlat(e) | StmtKind::Dondur(Some(e)) => gez_ifade(e, f),
        StmtKind::Eger { branches, else_ } => {
            for (c, b) in branches {
                gez_ifade(c, f);
                gez_blok(b, f);
            }
            if let Some(b) = else_ {
                gez_blok(b, f);
            }
        }
        StmtKind::Kere { count: e, body } | StmtKind::Surece { cond: e, body } => {
            gez_ifade(e, f);
            gez_blok(body, f);
        }
        StmtKind::Her { iter, pat, body } => {
            gez_ifade(iter, f);
            for (n, sp) in pat {
                f(n, *sp, Tur::Tanim);
            }
            gez_blok(body, f);
        }
        StmtKind::CFor { init, cond, step, body } => {
            gez_komut(init, f);
            gez_ifade(cond, f);
            gez_komut(step, f);
            gez_blok(body, f);
        }
        StmtKind::Match(m) => gez_eslestir(m, f),
        StmtKind::Fonksiyon(d) => gez_fonk(d, false, f),
        StmtKind::Tur(t) => {
            f(&mut t.name, t.span, Tur::Tanim);
            for (g, sp) in &mut t.generics {
                f(g, *sp, Tur::Tanim);
            }
            for (p, sp) in &mut t.parents {
                gez_yol(p, *sp, f);
            }
            gez_params(&mut t.fields, f);
            for s in t.alanlar.iter_mut().chain(&mut t.ortak) {
                if let StmtKind::Ata { name, name_span, .. } = &mut s.kind {
                    f(name, *name_span, Tur::TanimUye);
                }
            }
            gez_blok(&mut t.alanlar, f);
            gez_blok(&mut t.ortak, f);
            t.methods.iter_mut().for_each(|d| gez_fonk(d, true, f));
            t.ortak_fonk.iter_mut().for_each(|d| gez_fonk(d, true, f));
        }
        StmtKind::Arayuz(a) => {
            f(&mut a.name, a.span, Tur::Tanim);
            for s in &mut a.sigs {
                f(&mut s.name, s.span, Tur::Metot);
                gez_params(&mut s.params, f);
                if let Some(r) = &mut s.ret {
                    gez_tip(r, f);
                }
            }
        }
        StmtKind::Secenek(d) => {
            f(&mut d.name, d.span, Tur::Tanim);
            for (p, sp) in &mut d.interfaces {
                gez_yol(p, *sp, f);
            }
            for v in &mut d.variants {
                f(&mut v.name, v.span, Tur::TanimUye);
                gez_params(&mut v.fields, f);
            }
            d.methods.iter_mut().for_each(|m| gez_fonk(m, true, f));
        }
        StmtKind::Dahil { alias, .. } => {
            if let Some((a, sp)) = alias {
                f(a, *sp, Tur::Tanim);
            }
        }
        StmtKind::Dis { alias, funcs, .. } => {
            if let Some((a, sp)) = alias {
                f(a, *sp, Tur::Tanim);
            }
            for d in funcs {
                f(&mut d.name, d.span, Tur::Tanim);
                gez_params(&mut d.params, f);
                if let Some(r) = &mut d.ret {
                    gez_tip(r, f);
                }
            }
        }
        StmtKind::Dene { body, catches } => {
            gez_blok(body, f);
            for c in catches {
                f(&mut c.name, c.name_span, Tur::Tanim);
                if let Some(t) = &mut c.ty {
                    gez_tip(t, f);
                }
                gez_blok(&mut c.body, f);
            }
        }
        StmtKind::Dondur(None) | StmtKind::Dur | StmtKind::Devam => {}
    }
}

fn gez_eslestir(m: &mut Match, f: &mut Ziyaret) {
    gez_ifade(&mut m.scrutinee, f);
    for a in &mut m.arms {
        for p in a.pats.iter_mut().flatten() {
            match p {
                Pattern::Path { path, binds, span } => {
                    gez_yol(path, *span, f);
                    for (n, sp) in binds.iter_mut().flatten().flatten() {
                        f(n, *sp, Tur::Tanim);
                    }
                }
                Pattern::Typed { name, ty, .. } => {
                    if let Some((n, sp)) = name {
                        f(n, *sp, Tur::Tanim);
                    }
                    gez_tip(ty, f);
                }
                _ => {}
            }
        }
        if let Some(g) = &mut a.guard {
            gez_ifade(g, f);
        }
        match &mut a.body {
            ArmBody::Expr(e) => gez_ifade(e, f),
            ArmBody::Block(b) => gez_blok(b, f),
            ArmBody::Stmt(s) => gez_komut(s, f),
        }
    }
}

fn gez_ifade(e: &mut Expr, f: &mut Ziyaret) {
    let span = e.span;
    match &mut e.kind {
        ExprKind::Ident(n) => f(n, span, Tur::Ad),
        ExprKind::FStr(ps) => {
            for p in ps {
                match p {
                    FPartE::Lit(_) => {}
                    FPartE::Expr(x) | FPartE::Bicimli(x, ..) => gez_ifade(x, f),
                }
            }
        }
        ExprKind::List(xs) => xs.iter_mut().for_each(|x| gez_ifade(x, f)),
        ExprKind::Dict(kv) => {
            for (k, v) in kv {
                gez_ifade(k, f);
                gez_ifade(v, f);
            }
        }
        ExprKind::Neg(x) | ExprKind::Not(x) | ExprKind::BitNot(x) => gez_ifade(x, f),
        ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
            gez_ifade(a, f);
            gez_ifade(b, f);
        }
        ExprKind::Range { start, end, .. } => {
            for x in [start, end].into_iter().flatten() {
                gez_ifade(x, f);
            }
        }
        ExprKind::Call { callee, args } => {
            gez_ifade(callee, f);
            for a in args {
                if let Some((n, sp)) = &mut a.name {
                    f(n, *sp, Tur::Uye);
                }
                gez_ifade(&mut a.value, f);
            }
        }
        ExprKind::Field(x, n, sp) => {
            gez_ifade(x, f);
            f(n, *sp, Tur::Uye);
        }
        ExprKind::Olarak(x, t) => {
            gez_ifade(x, f);
            gez_tip(t, f);
        }
        ExprKind::Match(m) => gez_eslestir(m, f),
        ExprKind::Lambda { generics, params, body } => {
            for (g, sp) in generics {
                f(g, *sp, Tur::Tanim);
            }
            for p in params {
                f(&mut p.name, p.span, Tur::Tanim);
                if let Some(t) = &mut p.ty {
                    gez_tip(t, f);
                }
            }
            match body {
                LambdaBody::Expr(x) => gez_ifade(x, f),
                LambdaBody::Block(b) => gez_blok(b, f),
            }
        }
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Str(_) | ExprKind::Bool(_) | ExprKind::Hic | ExprKind::Bu => {}
    }
}
