// Fırlatma analizi (tip denetiminden sonra): hangi fonksiyonun hangi tipte değerler fırlatabileceği,
// her `yakala` dalına hangi değerlerin gelebileceği ve tipsiz `yakala diğer` dalının değişken tipi.
//
// Kurallar:
// - Tipsiz dalın tipi, o blokta fırlatılabilenlerin 'veya'sıdır. Çalışma hataları (Hata.SıfıraBölme,
//   Hata.İndeks ...) hemen her işlemde olabileceği için her blokta fırlatılabilir sayılır; Hata'nın alt
//   türleri tek bir `Hata` üyesinde birleşir: `Tam(64) veya Hata`.
// - Fonksiyon değerleri (lambda, isimli fonksiyon değeri) çağrılınca programda fırlatılan her şey
//   fırlatılabilir sayılır. C fonksiyonları Kolay değeri fırlatamaz.

use crate::check::Checker;
use crate::diag::Diag;
use crate::tir::*;
use crate::types::*;
use std::collections::HashMap;

fn ekle(v: &mut Vec<Type>, t: &Type) {
    for m in t.members() {
        if m != Type::Never && !v.contains(&m) {
            v.push(m);
        }
    }
}

struct Analiz<'a> {
    c: &'a Checker,
    /// fonksiyonların dışarı kaçırabildiği tipler
    e: &'a [Vec<Type>],
    /// programda fırlatılabilen her şey
    hepsi: &'a [Type],
    /// true: sadece fırlatma yerlerini topla (dene blokları bir şey yakalamaz, çağrılara bakılmaz)
    topla: bool,
}

impl Analiz<'_> {
    fn stmts(&self, ss: &[TS], out: &mut Vec<Type>) {
        for s in ss {
            self.stmt(s, out);
        }
    }

    fn stmt(&self, s: &TS, out: &mut Vec<Type>) {
        match s {
            TS::Let(_, e) | TS::Expr(e) | TS::Return(Some(e)) => self.expr(e, out),
            TS::Assign(p, e, _) | TS::Compound(p, _, e, _, _) => {
                self.place(p, out);
                self.expr(e, out);
            }
            TS::If(bs, el) => {
                for (c, b) in bs {
                    self.expr(c, out);
                    self.stmts(b, out);
                }
                if let Some(b) = el {
                    self.stmts(b, out);
                }
            }
            TS::Kere(c, b) | TS::While(c, b) => {
                self.expr(c, out);
                self.stmts(b, out);
            }
            TS::ForEach { iter, body, .. } => {
                self.expr(iter, out);
                self.stmts(body, out);
            }
            TS::CFor(i, c, st, b) => {
                self.stmt(i, out);
                self.expr(c, out);
                self.stmt(st, out);
                self.stmts(b, out);
            }
            TS::Match(m) => self.match_(m, out),
            TS::Throw(e) => {
                self.expr(e, out);
                ekle(out, &e.ty);
            }
            TS::Dene { body, catches } => {
                let mut ic = Vec::new();
                self.stmts(body, &mut ic);
                // dalı olmayan dene her şeyi yok sayar; dallar sadece tam yakaladıklarını durdurur
                if self.topla {
                    for t in &ic {
                        ekle(out, t);
                    }
                } else if !catches.is_empty() {
                    for t in &ic {
                        if !catches.iter().any(|c| self.tam_yakalar(c, t)) {
                            ekle(out, t);
                        }
                    }
                }
                for c in catches {
                    self.stmts(&c.body, out);
                }
            }
            TS::Declare(..) | TS::Break | TS::Continue | TS::Return(None) | TS::RunModule(_) => {}
        }
    }

    /// Dal, bu tipteki her değeri yakalar mı?
    fn tam_yakalar(&self, c: &TCatch, t: &Type) -> bool {
        match &c.ty {
            None => true,
            Some(ct) => self.c.assignable(t, ct),
        }
    }

    fn place(&self, p: &TPlace, out: &mut Vec<Type>) {
        match p {
            TPlace::Field(b, _) => self.place(b, out),
            TPlace::ListIndex(b, e) | TPlace::DictKey(b, e) => {
                self.place(b, out);
                self.expr(e, out);
            }
            TPlace::Var(_) | TPlace::Bu => {}
        }
    }

    fn recv(&self, r: &Recv, out: &mut Vec<Type>) {
        match r {
            Recv::Place(p) => self.place(p, out),
            Recv::Value(x) => self.expr(x, out),
        }
    }

    fn func(&self, f: FuncId, out: &mut Vec<Type>) {
        if !self.topla {
            for t in &self.e[f] {
                ekle(out, t);
            }
        }
    }

    fn hepsi(&self, out: &mut Vec<Type>) {
        for t in self.hepsi {
            ekle(out, t);
        }
    }

    fn match_(&self, m: &TMatch, out: &mut Vec<Type>) {
        self.expr(&m.scrut, out);
        for a in &m.arms {
            if let Some(g) = &a.guard {
                self.expr(g, out);
            }
            match &a.body {
                TArmBody::Expr(e) => self.expr(e, out),
                TArmBody::Block(b) => self.stmts(b, out),
            }
        }
    }

    fn exprs(&self, v: &[TExpr], out: &mut Vec<Type>) {
        for x in v {
            self.expr(x, out);
        }
    }

    fn expr(&self, e: &TExpr, out: &mut Vec<Type>) {
        match &e.kind {
            TK::Call(fid, args) => {
                self.exprs(args, out);
                self.func(*fid, out);
            }
            TK::Method { recv, view, name, args, .. } => {
                self.recv(recv, out);
                self.exprs(args, out);
                for ((_, v, m), f) in &self.c.dispatch {
                    if v == view && m.as_str() == &**name {
                        self.func(*f, out);
                    }
                }
            }
            TK::Direct { func, recv, args } => {
                self.recv(recv, out);
                self.exprs(args, out);
                self.func(*func, out);
            }
            TK::CallValue(f, args) => {
                self.expr(f, out);
                self.exprs(args, out);
                self.hepsi(out);
            }
            TK::BMethod { recv, m, args } => {
                self.recv(recv, out);
                self.exprs(args, out);
                if is_hof(*m) {
                    self.hepsi(out);
                }
            }
            TK::Unwrap(x) => {
                self.expr(x, out);
                for m in x.ty.members() {
                    if self.c.is_hata(&m) {
                        ekle(out, &m);
                    }
                }
            }
            TK::FStr(ps) => {
                for p in ps {
                    if let TFPart::Expr(x) = p {
                        self.expr(x, out);
                    }
                }
            }
            TK::List(v) | TK::Builtin(_, v) | TK::New(_, v) | TK::Variant(_, _, v) => self.exprs(v, out),
            TK::Dict(v) => {
                for (a, b) in v {
                    self.expr(a, out);
                    self.expr(b, out);
                }
            }
            TK::Range { start, end, step, .. } => {
                self.expr(start, out);
                self.expr(end, out);
                if let Some(x) = step {
                    self.expr(x, out);
                }
            }
            TK::Slice { base, start, end, .. } => {
                self.expr(base, out);
                for x in [start, end].into_iter().flatten() {
                    self.expr(x, out);
                }
            }
            TK::Neg(x) | TK::Not(x) | TK::Convert(x) | TK::Olarak(x) | TK::Field(x, _) => self.expr(x, out),
            TK::Arith(_, a, b) | TK::Concat(a, b) | TK::Cmp(_, a, b) | TK::And(a, b) | TK::Or(a, b) | TK::Index(a, b) => {
                self.expr(a, out);
                self.expr(b, out);
            }
            TK::Match(m) => self.match_(m, out),
            TK::Lambda { caps, bu, .. } => {
                self.exprs(caps, out);
                if let Some(b) = bu {
                    self.expr(b, out);
                }
            }
            TK::Int(..) | TK::Float(_) | TK::Str(_) | TK::Bool(_) | TK::Hic | TK::Var(_) | TK::Bu | TK::Default | TK::FuncRef(_) => {}
        }
    }

    /// `dene` bloklarının yakala dallarını doldurur: kaynaklar, tipsiz dalların tipi, uyarılar.
    fn doldur(&self, ss: &mut [TS], tipler: &mut HashMap<(u32, u32), Type>, uyarilar: &mut Vec<Diag>) {
        for s in ss {
            match s {
                TS::If(bs, el) => {
                    for (_, b) in bs.iter_mut() {
                        self.doldur(b, tipler, uyarilar);
                    }
                    if let Some(b) = el {
                        self.doldur(b, tipler, uyarilar);
                    }
                }
                TS::Kere(_, b) | TS::While(_, b) | TS::ForEach { body: b, .. } => self.doldur(b, tipler, uyarilar),
                TS::CFor(i, _, st, b) => {
                    self.doldur(std::slice::from_mut(&mut **i), tipler, uyarilar);
                    self.doldur(std::slice::from_mut(&mut **st), tipler, uyarilar);
                    self.doldur(b, tipler, uyarilar);
                }
                TS::Match(m) => {
                    for a in &mut m.arms {
                        if let TArmBody::Block(b) = &mut a.body {
                            self.doldur(b, tipler, uyarilar);
                        }
                    }
                }
                TS::Dene { body, catches } => {
                    let mut tum = Vec::new();
                    self.stmts(body, &mut tum);
                    for h in &self.c.hata_turleri {
                        ekle(&mut tum, &Type::Named(*h));
                    }
                    for j in 0..catches.len() {
                        // önceki dalların tam yakaladıkları buraya gelmez
                        let havuz: Vec<Type> = tum.iter().filter(|t| !catches[..j].iter().any(|c| self.tam_yakalar(c, t))).cloned().collect();
                        let c = &mut catches[j];
                        match c.ty.clone() {
                            Some(ct) => {
                                let uyar = |t: &Type| -> Option<bool> {
                                    if self.c.assignable(t, &ct) {
                                        Some(false)
                                    } else if matches!((t, &ct), (Type::Named(a), Type::Named(b)) if self.c.is_subtype(*b, *a)) {
                                        Some(true)
                                    } else {
                                        None
                                    }
                                };
                                if !tum.iter().any(|t| uyar(t).is_some()) {
                                    let yazi = self.c.texts.get(c.span.file as usize).and_then(|t| t.get(c.span.start as usize..c.span.end as usize)).unwrap_or("yakala");
                                    let tn = self.c.type_name(&ct);
                                    uyarilar.push(Diag::new(c.span, format!("Bu blokta {tn} fırlatılamaz, '{yazi}: {tn}' dalı hiç çalışmaz.")));
                                }
                                c.kaynak = havuz.iter().filter_map(|t| uyar(t).map(|d| (t.clone(), d))).collect();
                            }
                            None => {
                                let mut u: Vec<Type> = havuz.iter().filter(|t| !self.c.is_hata(t)).cloned().collect();
                                if havuz.iter().any(|t| self.c.is_hata(t)) {
                                    u.push(Type::Named(self.c.hata_turleri[0]));
                                }
                                let ut = if u.is_empty() { Type::Named(self.c.hata_turleri[0]) } else { make_union(u) };
                                tipler.insert((c.span.file, c.span.start), ut);
                                // tipler oturmadıysa (son çare) sadece değişkene atanabilenler kalır
                                let hata0 = Type::Named(self.c.hata_turleri[0]);
                                let bind_ty = c.bind_ty.clone();
                                c.kaynak = havuz
                                    .iter()
                                    .filter(|t| self.c.assignable(t, &bind_ty) || (self.c.is_hata(t) && self.c.assignable(&hata0, &bind_ty)))
                                    .map(|t| (t.clone(), false))
                                    .collect();
                            }
                        }
                    }
                    self.doldur(body, tipler, uyarilar);
                    for c in catches.iter_mut() {
                        self.doldur(&mut c.body, tipler, uyarilar);
                    }
                }
                _ => {}
            }
        }
    }
}

impl Checker {
    /// Dönüş: tipsiz yakala dallarının değişken tipleri (dalın yerine göre)
    pub(crate) fn firlat_analizi(&mut self, modules: &mut [Vec<TS>]) -> HashMap<(u32, u32), Type> {
        let mut bodies: Vec<Option<TFunc>> = self.funcs.iter_mut().map(|f| f.body.take()).collect();
        let n = bodies.len();
        let bos: Vec<Vec<Type>> = vec![Vec::new(); n];
        // 1. programda fırlatılabilen her şey
        let mut hepsi = Vec::new();
        {
            let a = Analiz { c: self, e: &bos, hepsi: &[], topla: true };
            for b in bodies.iter().flatten() {
                a.stmts(&b.body, &mut hepsi);
            }
            for m in modules.iter() {
                a.stmts(m, &mut hepsi);
            }
            for h in &self.hata_turleri {
                ekle(&mut hepsi, &Type::Named(*h));
            }
        }
        // 2. fonksiyonların dışarı kaçırabildikleri (sabit nokta)
        let mut e = bos.clone();
        loop {
            let mut degisti = false;
            for fid in 0..n {
                let Some(b) = &bodies[fid] else { continue };
                let mut out = e[fid].clone();
                let a = Analiz { c: self, e: &e, hepsi: &hepsi, topla: false };
                a.stmts(&b.body, &mut out);
                if out.len() != e[fid].len() {
                    e[fid] = out;
                    degisti = true;
                }
            }
            if !degisti {
                break;
            }
        }
        // 3. yakala dalları
        let mut tipler = HashMap::new();
        let mut uyarilar = Vec::new();
        {
            let a = Analiz { c: self, e: &e, hepsi: &hepsi, topla: false };
            for b in bodies.iter_mut().flatten() {
                a.doldur(&mut b.body, &mut tipler, &mut uyarilar);
            }
            for m in modules.iter_mut() {
                a.doldur(m, &mut tipler, &mut uyarilar);
            }
        }
        for (f, b) in self.funcs.iter_mut().zip(bodies) {
            f.body = b;
        }
        self.warnings.extend(uyarilar);
        tipler
    }
}
