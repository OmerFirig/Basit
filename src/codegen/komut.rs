// Komutlar: atama yerleri, döngüler, eşleştir.

use super::duzen::{int_parts, is_esnek, is_fixed_int, width};
use super::{slot_tag, Gen, BASLIK};
use crate::diag::CResult;
use crate::tir::*;
use crate::types::{fits, int_bounds, Sign, Type, TypeId, TAM64};
use num_bigint::BigInt;

/// Atama yerinin bir adımı (önce bütün indeks/anahtar ifadeleri hesaplanır, sonra yürünür)
pub(super) enum Step {
    Field(u32),
    /// liste indeksi (i64), hata konumu
    Idx(String, Loc),
    /// sözlük anahtarının bellekteki yeri, anahtar değeri (bırakılacak), konum
    Key(String, String, Loc),
}

pub(super) enum Root {
    Var(Slot),
    Bu,
}

impl<'p> Gen<'p> {
    pub(super) fn stmts(&mut self, stmts: &[TS]) -> CResult<()> {
        for s in stmts {
            self.stmt(s)?;
        }
        Ok(())
    }

    // ------------------------------------------------------------ değişkenler

    /// Değişkene değer yazar: yeni değer saklanır (sahipliği alınır ya da tutulur), eskisi bırakılır.
    pub(super) fn store_slot(&mut self, slot: Slot, v: &str, t: &Type) {
        let ty = self.llty(t);
        let p = self.slot_ptr(slot);
        if self.is_rc(t) {
            self.keep_ref(v, t);
            let old = self.tmp();
            let tg = slot_tag(&p);
            self.emit(&format!("{old} = load {ty}, ptr {p}{tg}"));
            self.emit(&format!("store {ty} {v}, ptr {p}{tg}"));
            self.release(&old, t);
        } else if ty != "void" {
            self.emit(&format!("store {ty} {v}, ptr {p}{}", slot_tag(&p)));
        }
        if let Slot::Global(g) = slot {
            self.emit(&format!("store i1 true, ptr @g{g}_h, !tbaa !2"));
        }
    }

    /// Değişkeni okur (ödünç; referans tutulmaz).
    pub(super) fn load_slot(&mut self, slot: Slot, t: &Type, loc: Loc) -> String {
        let lt = self.llty(t);
        if let Slot::Global(g) = slot {
            self.global_check(g, loc);
        }
        let p = self.slot_ptr(slot);
        let v = self.tmp();
        self.emit(&format!("{v} = load {lt}, ptr {p}{}", slot_tag(&p)));
        v
    }

    // ------------------------------------------------------------ atama yerleri

    /// Yerin kökünü ve adımlarını hazırlar; indeks ve anahtar ifadeleri burada (soldan sağa) hesaplanır.
    pub(super) fn place_steps(&mut self, p: &TPlace, steps: &mut Vec<Step>) -> CResult<Root> {
        Ok(match p {
            TPlace::Var(s) => Root::Var(*s),
            TPlace::Bu => Root::Bu,
            TPlace::Field(b, f) => {
                let r = self.place_steps(b, steps)?;
                steps.push(Step::Field(*f));
                r
            }
            TPlace::ListIndex(b, i) => {
                let r = self.place_steps(b, steps)?;
                let v = self.expr(i)?;
                let iv = self.to_index(&v, &i.ty, i.loc);
                self.drop_temp(&v);
                steps.push(Step::Idx(iv, i.loc));
                r
            }
            TPlace::DictKey(b, k) => {
                let r = self.place_steps(b, steps)?;
                let v = self.expr(k)?;
                let kp = self.spill(&v, &k.ty);
                steps.push(Step::Key(kp, v, k.loc));
                r
            }
        })
    }

    fn root_info(&mut self, r: &Root, loc: Loc) -> (String, Type) {
        match r {
            Root::Var(s) => {
                let t = self.slot_ty(*s).cloned().expect("değişken tipi");
                if let Slot::Global(g) = s {
                    self.global_check(*g, loc);
                }
                (self.slot_ptr(*s), t)
            }
            Root::Bu => (self.f.bu.clone().expect("bu"), Type::Named(self.f.owner.expect("tür"))),
        }
    }

    /// Statik tür `s` içindeki `fid` alanının tipi
    pub(super) fn field_type(&self, s: TypeId, fid: u32) -> Type {
        let idx = self.prog.field_pos[s][fid as usize];
        match &self.prog.types[s].kind {
            TypeDefKind::Tur { fields, .. } => fields[idx as usize].ty.clone(),
            _ => unreachable!("alan sadece türlerde olur"),
        }
    }

    /// Adresteki değeri okur (ödünç). `reg`: adres bir değişken yuvası (yazmaç biçimi) mi
    pub(super) fn load_at(&mut self, addr: &str, t: &Type, reg: bool) -> String {
        if reg {
            let ll = self.llty(t);
            let v = self.tmp();
            self.emit(&format!("{v} = load {ll}, ptr {addr}{}", slot_tag(addr)));
            v
        } else {
            self.load_mem(addr, t)
        }
    }

    fn store_raw(&mut self, addr: &str, v: &str, t: &Type, reg: bool) {
        if reg {
            let ll = self.llty(t);
            self.emit(&format!("store {ll} {v}, ptr {addr}{}", slot_tag(addr)));
        } else {
            self.store_mem(addr, v, t);
        }
    }

    /// Adrese yeni değer yazar; eski değer bırakılır.
    pub(super) fn store_at(&mut self, addr: &str, v: &str, t: &Type, reg: bool) {
        if self.is_rc(t) {
            self.keep_ref(v, t);
            let old = self.load_at(addr, t, reg);
            self.store_raw(addr, v, t, reg);
            self.release(&old, t);
        } else {
            self.store_raw(addr, v, t, reg);
        }
    }

    /// Adresteki nesnenin (liste, sözlük, tür) tek sahibi olunmasını sağlar ve nesneyi döndürür.
    /// Paylaşılıyorsa kopyalanıp kopyası adrese yazılır (değer semantiği).
    pub(super) fn make_unique(&mut self, addr: &str, t: &Type, reg: bool) -> String {
        let c = self.load_at(addr, t, reg);
        let (rc, u) = (self.tmp(), self.tmp());
        self.emit(&format!("{rc} = load i64, ptr {c}{BASLIK}"));
        self.emit(&format!("{u} = icmp eq i64 {rc}, 1"));
        let from = self.f.cur.clone();
        let (cp, ok) = (self.new_label(), self.new_label());
        self.term(&format!("br i1 {u}, label %{ok}, label %{cp}, !prof !0"));
        self.block(&cp);
        let n = self.tmp();
        self.emit(&format!("{n} = call ptr @kolay_benzersiz(ptr {c})"));
        self.store_raw(addr, &n, t, reg);
        let from2 = self.f.cur.clone();
        self.block(&ok);
        let r = self.tmp();
        self.emit(&format!("{r} = phi ptr [ {c}, %{from} ], [ {n}, %{from2} ]"));
        r
    }

    /// Değiştirilecek yerin adresi. Yol üzerindeki her nesnenin tek sahibi olunur.
    /// `create`: sözlükte anahtar yoksa eklensin mi (atama) ya da hata mı (+= gibi).
    pub(super) fn place_mut_addr(&mut self, p: &TPlace, create: bool, loc: Loc) -> CResult<(String, Type, bool)> {
        let (a, t, r, _) = self.place_mut_addr_pin(p, create, loc)?;
        Ok((a, t, r))
    }

    /// place_mut_addr gibi; ayrıca yolun ilk nesnesini (değişkendeki liste/sözlük/nesne) döndürür.
    /// Metot çağrısı sürerken bu nesne tutulursa elemanın adresi geçerli kalır.
    pub(super) fn place_mut_addr_pin(&mut self, p: &TPlace, create: bool, loc: Loc) -> CResult<(String, Type, bool, Option<String>)> {
        let mut steps = Vec::new();
        let root = self.place_steps(p, &mut steps)?;
        let (mut addr, mut ty) = self.root_info(&root, loc);
        let mut reg = true;
        let mut first = None;
        for st in steps {
            let c = self.make_unique(&addr, &ty, reg);
            if first.is_none() {
                first = Some(c.clone());
            }
            match st {
                Step::Field(fid) => {
                    let Type::Named(s) = ty else { unreachable!() };
                    addr = self.field_ptr(&c, s, fid);
                    ty = self.field_type(s, fid);
                }
                Step::Idx(i, l) => {
                    let Type::List(e, _) = &ty else { unreachable!() };
                    let e = (**e).clone();
                    addr = self.elem_ptr(&c, &i, &e, l);
                    ty = e;
                }
                Step::Key(kp, kv, l) => {
                    let Type::Dict(_, v) = &ty else { unreachable!() };
                    let v = (**v).clone();
                    let a = self.tmp();
                    if create {
                        self.emit(&format!("{a} = call ptr @kolay_sozluk_yer(ptr {c}, ptr {kp})"));
                    } else {
                        let (k, kd) = self.where_(l);
                        self.emit(&format!("{a} = call ptr @kolay_sozluk_al(ptr {c}, ptr {kp}, ptr {k}, ptr {kd})"));
                    }
                    self.drop_temp(&kv);
                    addr = a;
                    ty = v;
                }
            }
            reg = false;
        }
        Ok((addr, ty, reg, first))
    }

    /// Yerdeki değeri okur (değiştirmeden). Değer tutulur (sahipli döner).
    pub(super) fn place_value(&mut self, p: &TPlace, loc: Loc) -> CResult<String> {
        let mut steps = Vec::new();
        let root = self.place_steps(p, &mut steps)?;
        let (addr, mut ty) = self.root_info(&root, loc);
        let mut cur = self.load_at(&addr, &ty, true);
        for st in steps {
            match st {
                Step::Field(fid) => {
                    let Type::Named(s) = ty else { unreachable!() };
                    let fp = self.field_ptr(&cur, s, fid);
                    ty = self.field_type(s, fid);
                    cur = self.load_mem(&fp, &ty);
                }
                Step::Idx(i, l) => {
                    let Type::List(e, _) = &ty else { unreachable!() };
                    let e = (**e).clone();
                    let ep = self.elem_ptr(&cur, &i, &e, l);
                    cur = self.load_mem(&ep, &e);
                    ty = e;
                }
                Step::Key(kp, kv, l) => {
                    let Type::Dict(_, v) = &ty else { unreachable!() };
                    let v = (**v).clone();
                    let (k, kd) = self.where_(l);
                    let a = self.tmp();
                    self.emit(&format!("{a} = call ptr @kolay_sozluk_al(ptr {cur}, ptr {kp}, ptr {k}, ptr {kd})"));
                    self.drop_temp(&kv);
                    cur = self.load_mem(&a, &v);
                    ty = v;
                }
            }
        }
        if self.is_rc(&ty) {
            self.retain(&cur, &ty);
            self.own(cur.clone(), &ty);
        }
        Ok(cur)
    }

    /// Liste elemanının adresi (sınır kontrolüyle; eksi indeks sondan sayar)
    pub(super) fn elem_ptr(&mut self, list: &str, i: &str, e: &Type, loc: Loc) -> String {
        let (lp, len, neg, adj, real, ok) = (self.tmp(), self.tmp(), self.tmp(), self.tmp(), self.tmp(), self.tmp());
        self.emit(&format!("{lp} = getelementptr i8, ptr {list}, i64 16"));
        self.emit(&format!("{len} = load i64, ptr {lp}{BASLIK}"));
        self.emit(&format!("{neg} = icmp slt i64 {i}, 0"));
        self.emit(&format!("{adj} = add i64 {i}, {len}"));
        self.emit(&format!("{real} = select i1 {neg}, i64 {adj}, i64 {i}"));
        self.emit(&format!("{ok} = icmp ult i64 {real}, {len}"));
        let (bad, good) = (self.new_label(), self.new_label());
        self.term(&format!("br i1 {ok}, label %{good}, label %{bad}, !prof !0"));
        self.block(&bad);
        let (k, kd) = self.where_(loc);
        self.emit(&format!("call void @kolay_indeks_hatasi(i64 {i}, i64 {len}, ptr {k}, ptr {kd})"));
        self.term("unreachable");
        self.block(&good);
        let st = self.stor(e);
        let (dp, ep) = (self.tmp(), self.tmp());
        self.emit(&format!("{dp} = getelementptr i8, ptr {list}, i64 32"));
        self.emit(&format!("{ep} = getelementptr {}, ptr {dp}, i64 {real}", st.ll));
        ep
    }

    /// İndeks değerini i64'e çevirir (64 bite sığmıyorsa yorumlayıcıdaki gibi hata).
    pub(super) fn to_index(&mut self, v: &str, t: &Type, loc: Loc) -> String {
        if is_esnek(t) {
            let msg = self.cstr("İndeks çok büyük.");
            let (k, kd) = self.where_(loc);
            let r = self.tmp();
            self.emit(&format!("{r} = call i64 @kolay_esnek_i64(i64 {v}, ptr {msg}, ptr {k}, ptr {kd})"));
            return r;
        }
        let w = width(t);
        let (_, s) = int_parts(t);
        if w <= 64 && !(w == 64 && s == Sign::Pos) {
            if w == 64 {
                return v.to_string();
            }
            let r = self.tmp();
            let op = if s == Sign::Pos { "zext" } else { "sext" };
            self.emit(&format!("{r} = {op} i{w} {v} to i64"));
            return r;
        }
        // geniş tipler: i64 aralığında mı?
        let big = w.max(65) + 1;
        let (x, c1, c2, c) = (self.tmp(), self.tmp(), self.tmp(), self.tmp());
        let op = if s == Sign::Pos { "zext" } else { "sext" };
        self.emit(&format!("{x} = {op} i{w} {v} to i{big}"));
        self.emit(&format!("{c1} = icmp slt i{big} {x}, -9223372036854775808"));
        self.emit(&format!("{c2} = icmp sgt i{big} {x}, 9223372036854775807"));
        self.emit(&format!("{c} = or i1 {c1}, {c2}"));
        let (bad, ok) = (self.new_label(), self.new_label());
        self.br_cold(&c, &bad, &ok);
        self.block(&bad);
        self.error_call_tur(hata_turu::INDEKS, "İndeks çok büyük.", loc);
        self.block(&ok);
        let r = self.tmp();
        self.emit(&format!("{r} = trunc i{big} {x} to i64"));
        r
    }

    // ------------------------------------------------------------ komutlar

    pub(super) fn stmt(&mut self, s: &TS) -> CResult<()> {
        match s {
            TS::Let(slot, e) => {
                let v = self.expr(e)?;
                let t = self.slot_ty(*slot).cloned().unwrap_or(e.ty.clone());
                self.store_slot(*slot, &v, &t);
            }
            TS::Declare(..) => {}
            TS::Assign(TPlace::Var(slot), e, _) => {
                let v = self.expr(e)?;
                let t = self.slot_ty(*slot).cloned().unwrap_or(e.ty.clone());
                self.store_slot(*slot, &v, &t);
            }
            TS::Assign(place, e, loc) => {
                // önce değer, sonra yer (değer hesaplanırken yer değişebilir)
                let v = self.expr(e)?;
                let (addr, ty, reg) = self.place_mut_addr(place, true, *loc)?;
                self.store_at(&addr, &v, &ty, reg);
            }
            TS::Compound(place, op, e, ty, loc) => self.compound(place, *op, e, ty, *loc)?,
            TS::Expr(e) => {
                let v = self.expr(e)?;
                self.drop_temp(&v);
            }
            TS::If(branches, else_) => {
                let end = self.new_label();
                for (c, b) in branches {
                    let cv = self.expr(c)?;
                    let (then, next) = (self.new_label(), self.new_label());
                    self.term(&format!("br i1 {cv}, label %{then}, label %{next}"));
                    self.block(&then);
                    self.stmts(b)?;
                    if !self.f.terminated {
                        self.term(&format!("br label %{end}"));
                    }
                    self.block(&next);
                }
                if let Some(b) = else_ {
                    self.stmts(b)?;
                }
                self.block(&end);
            }
            TS::Kere(c, body) => self.kere(c, body)?,
            TS::ForEach { iter, binds, body } => self.foreach(iter, binds, body)?,
            TS::While(c, body) => {
                let (cond, bodyl, exit) = (self.new_label(), self.new_label(), self.new_label());
                self.block(&cond);
                let cv = self.expr(c)?;
                self.term(&format!("br i1 {cv}, label %{bodyl}, label %{exit}"));
                self.block(&bodyl);
                self.f.loops.push((cond.clone(), exit.clone(), self.f.deneler.len()));
                self.stmts(body)?;
                self.f.loops.pop();
                if !self.f.terminated {
                    self.term(&format!("br label %{cond}"));
                }
                self.block(&exit);
            }
            TS::CFor(init, c, step, body) => {
                self.stmt(init)?;
                let (cond, bodyl, stepl, exit) = (self.new_label(), self.new_label(), self.new_label(), self.new_label());
                self.block(&cond);
                let cv = self.expr(c)?;
                self.term(&format!("br i1 {cv}, label %{bodyl}, label %{exit}"));
                self.block(&bodyl);
                self.f.loops.push((stepl.clone(), exit.clone(), self.f.deneler.len()));
                self.stmts(body)?;
                self.f.loops.pop();
                self.block(&stepl);
                self.stmt(step)?;
                if !self.f.terminated {
                    self.term(&format!("br label %{cond}"));
                }
                self.block(&exit);
            }
            TS::Break => {
                let (_, b, d) = self.f.loops.last().cloned().expect("döngü");
                self.dene_kes(d);
                self.term(&format!("br label %{b}"));
            }
            TS::Continue => {
                let (c, _, d) = self.f.loops.last().cloned().expect("döngü");
                self.dene_kes(d);
                self.term(&format!("br label %{c}"));
            }
            TS::Return(e) => match e {
                Some(e) => {
                    let v = self.expr(e)?;
                    let ret = self.f.ret.clone();
                    let lt = self.llty(&ret);
                    if self.is_rc(&ret) {
                        self.keep_ref(&v, &ret);
                    }
                    self.dene_kes(0);
                    self.release_rc_slots();
                    self.term(&format!("ret {lt} {v}"));
                }
                None => {
                    self.dene_kes(0);
                    self.release_rc_slots();
                    self.term("ret void");
                }
            },
            TS::Match(m) => {
                self.match_code(m, None)?;
            }
            TS::RunModule(m) => self.emit(&format!("call void @m{m}()")),
            TS::Throw(e) => {
                // değerin sahipliği fırlatmaya geçer
                let v = self.expr(e)?;
                self.keep_ref(&v, &e.ty);
                let p = self.spill(&v, &e.ty);
                let t = self.tip(&e.ty);
                let (k, kd) = self.where_(e.loc);
                self.emit(&format!("call void @kolay_firlat(ptr {t}, ptr {p}, ptr {k}, ptr {kd})"));
                self.term("unreachable");
            }
            TS::Dene { body, catches } => self.dene(body, catches)?,
        }
        Ok(())
    }

    /// dur/devam/döndür ile dene bloklarından çıkılırken çerçeveler zincirden çıkarılır
    /// (`d`: hedefin içinde olduğu dene sayısı).
    fn dene_kes(&mut self, d: usize) {
        if let Some(fr) = self.f.deneler.get(d).cloned() {
            self.emit(&format!("call void @kolay_dene_cik(ptr {fr})"));
        }
    }

    /// `dene { } yakala ...`: çerçeve zincire eklenir ve setjmp yapılır. Hata olunca (longjmp) setjmp
    /// ikinci kez, sıfırdan farklı döner: fırlatılan değerin tipine göre yakala dalı seçilir.
    fn dene(&mut self, body: &[TS], catches: &[TCatch]) -> CResult<()> {
        let fr = self.alloca("[1024 x i8]");
        self.emit(&format!("call void @kolay_dene_gir(ptr {fr})"));
        let (r, z) = (self.tmp(), self.tmp());
        self.emit(&format!("{r} = call i32 @_setjmp(ptr {fr}{}) returns_twice", super::SETJMP_ARGS));
        self.emit(&format!("{z} = icmp eq i32 {r}, 0"));
        let (govde, yakala, son) = (self.new_label(), self.new_label(), self.new_label());
        self.term(&format!("br i1 {z}, label %{govde}, label %{yakala}"));
        self.block(&govde);
        self.f.deneler.push(fr.clone());
        let res = self.stmts(body);
        self.f.deneler.pop();
        res?;
        if !self.f.terminated {
            self.emit(&format!("call void @kolay_dene_cik(ptr {fr})"));
            self.term(&format!("br label %{son}"));
        }
        self.block(&yakala);
        if catches.is_empty() {
            // dal yok: hata yok sayılır
            self.emit(&format!("call void @kolay_dene_birak(ptr {fr})"));
            self.term(&format!("br label %{son}"));
            self.block(&son);
            return Ok(());
        }
        let (tip, p) = (self.tmp(), self.tmp());
        self.emit(&format!("{tip} = call ptr @kolay_dene_tip(ptr {fr})"));
        self.emit(&format!("{p} = call ptr @kolay_dene_deger(ptr {fr})"));
        let dallar: Vec<String> = catches.iter().map(|_| self.new_label()).collect();
        for (c, dal) in catches.iter().zip(&dallar) {
            for (st, dinamik) in &c.kaynak {
                let t = self.tip(st);
                let eq = self.tmp();
                self.emit(&format!("{eq} = icmp eq ptr {tip}, {t}"));
                let (evet, hayir) = (self.new_label(), self.new_label());
                self.term(&format!("br i1 {eq}, label %{evet}, label %{hayir}"));
                self.block(&evet);
                if *dinamik {
                    // Hata tipinde fırlatılan değer Hata.Dönüşüm dalına uyabilir: nesnenin gerçek türüne bakılır
                    let Some(Type::Named(hedef)) = &c.ty else { unreachable!() };
                    let o = self.tmp();
                    self.emit(&format!("{o} = load ptr, ptr {p}"));
                    let ok = self.dyn_is_subtype(&o, *hedef);
                    let uyar = self.new_label();
                    self.term(&format!("br i1 {ok}, label %{uyar}, label %{hayir}"));
                    self.block(&uyar);
                }
                match c.bind {
                    Some(slot) => {
                        let v = self.load_mem(&p, st);
                        let v = self.own(v, st);
                        let cv = self.convert(&v, st, &c.bind_ty, Loc::default())?;
                        self.store_slot(slot, &cv, &c.bind_ty);
                        self.emit(&format!("call void @kolay_dene_bitti(ptr {fr})"));
                    }
                    None => self.emit(&format!("call void @kolay_dene_birak(ptr {fr})")),
                }
                self.term(&format!("br label %{dal}"));
                self.block(&hayir);
            }
        }
        // hiçbir dal uymadı: değer dıştaki bloğa gider
        self.emit(&format!("call void @kolay_dene_yeniden(ptr {fr})"));
        self.term("unreachable");
        for (c, dal) in catches.iter().zip(&dallar) {
            self.block(dal);
            self.stmts(&c.body)?;
            if !self.f.terminated {
                self.term(&format!("br label %{son}"));
            }
        }
        self.block(&son);
        Ok(())
    }

    fn compound(&mut self, place: &TPlace, op: crate::ast::BinOp, e: &TExpr, ty: &Type, loc: Loc) -> CResult<()> {
        let rhs = self.expr(e)?;
        if let TPlace::Var(slot) = place {
            let cur = self.load_slot(*slot, ty, loc);
            let p = self.slot_ptr(*slot);
            if *ty == Type::Str {
                // Değişkenin kendi referansı kolay_ekle'ye devredilir; başka kullanıcısı yoksa
                // yazı yerinde büyütülür.
                let t = self.tmp();
                self.emit(&format!("{t} = call ptr @kolay_ekle(ptr {cur}, ptr {rhs})"));
                self.emit(&format!("store ptr {t}, ptr {p}{}", slot_tag(&p)));
                self.drop_temp(&rhs);
            } else {
                let new = self.arith(op, &cur, &rhs, ty, loc)?;
                self.drop_temp(&rhs);
                let lt = self.llty(ty);
                if self.is_rc(ty) {
                    // esnek Tam: yeni değer saklanır, eskisi bırakılır
                    self.disown(&new);
                    self.emit(&format!("store {lt} {new}, ptr {p}{}", slot_tag(&p)));
                    self.release(&cur, ty);
                } else {
                    self.emit(&format!("store {lt} {new}, ptr {p}{}", slot_tag(&p)));
                }
            }
            return Ok(());
        }
        let (addr, pty, reg) = self.place_mut_addr(place, false, loc)?;
        let cur = self.load_at(&addr, &pty, reg);
        if *ty == Type::Str {
            let t = self.tmp();
            self.emit(&format!("{t} = call ptr @kolay_ekle(ptr {cur}, ptr {rhs})"));
            self.store_raw(&addr, &t, &pty, reg);
            self.drop_temp(&rhs);
        } else {
            let new = self.arith(op, &cur, &rhs, ty, loc)?;
            self.drop_temp(&rhs);
            self.store_at(&addr, &new, &pty, reg);
        }
        Ok(())
    }

    fn kere(&mut self, c: &TExpr, body: &[TS]) -> CResult<()> {
        let n0 = self.expr(c)?;
        let (n, lt, cmp_op) = if is_esnek(&c.ty) {
            let (k, kd) = self.where_(c.loc);
            let n = self.tmp();
            self.emit(&format!("{n} = call i64 @kolay_esnek_kere(i64 {n0}, ptr {k}, ptr {kd})"));
            self.drop_temp(&n0);
            (n, "i64".to_string(), "slt")
        } else {
            let lt = self.llty(&c.ty);
            let (_, sign) = int_parts(&c.ty);
            if sign != Sign::Pos {
                let neg = self.tmp();
                self.emit(&format!("{neg} = icmp slt {lt} {n0}, 0"));
                let (bad, ok) = (self.new_label(), self.new_label());
                self.br_cold(&neg, &bad, &ok);
                self.block(&bad);
                let (mag, negf) = self.int128_form(&n0, &c.ty);
                let mag = self.i128_ptr(&mag);
                let (k, kd) = self.where_(c.loc);
                self.emit(&format!("call void @kolay_kere_negatif(ptr {mag}, i32 {negf}, ptr {k}, ptr {kd})"));
                self.term("unreachable");
                self.block(&ok);
            }
            (n0, lt, if sign == Sign::Pos { "ult" } else { "slt" })
        };
        let ctr = self.alloca(&lt);
        self.emit(&format!("store {lt} 0, ptr {ctr}"));
        let (cond, bodyl, step, exit) = (self.new_label(), self.new_label(), self.new_label(), self.new_label());
        self.block(&cond);
        let cur = self.tmp();
        self.emit(&format!("{cur} = load {lt}, ptr {ctr}"));
        let cmp = self.tmp();
        self.emit(&format!("{cmp} = icmp {cmp_op} {lt} {cur}, {n}"));
        self.term(&format!("br i1 {cmp}, label %{bodyl}, label %{exit}"));
        self.block(&bodyl);
        self.f.loops.push((step.clone(), exit.clone(), self.f.deneler.len()));
        self.stmts(body)?;
        self.f.loops.pop();
        self.block(&step);
        let cur2 = self.tmp();
        self.emit(&format!("{cur2} = load {lt}, ptr {ctr}"));
        let inc = self.tmp();
        self.emit(&format!("{inc} = add {lt} {cur2}, 1"));
        self.emit(&format!("store {lt} {inc}, ptr {ctr}"));
        self.term(&format!("br label %{cond}"));
        self.block(&exit);
        Ok(())
    }

    // ------------------------------------------------------------ her ... için

    fn foreach(&mut self, iter: &TExpr, binds: &[Slot], body: &[TS]) -> CResult<()> {
        if let TK::Range { start, end, inclusive, step } = &iter.kind {
            return self.for_range(start, end, *inclusive, step.as_deref(), iter.loc, binds[0], body);
        }
        match &iter.ty {
            Type::List(e, _) => {
                let e = (**e).clone();
                let v = self.expr(iter)?;
                // döngü, başladığı andaki liste üzerinde döner (gövde değişkeni değiştirse bile)
                let h = self.hidden_slot(&v, &iter.ty);
                let l = self.tmp();
                self.emit(&format!("{l} = load ptr, ptr {h}"));
                let (lp, len) = (self.tmp(), self.tmp());
                self.emit(&format!("{lp} = getelementptr i8, ptr {l}, i64 16"));
                self.emit(&format!("{len} = load i64, ptr {lp}{BASLIK}"));
                self.counted_loop(&len, body, |me, i| {
                    let st = me.stor(&e);
                    let (dp, ep) = (me.tmp(), me.tmp());
                    me.emit(&format!("{dp} = getelementptr i8, ptr {l}, i64 32"));
                    me.emit(&format!("{ep} = getelementptr {}, ptr {dp}, i64 {i}", st.ll));
                    let x = me.load_mem(&ep, &e);
                    me.store_slot(binds[0], &x, &e);
                    Ok(None)
                })
            }
            Type::Dict(k, v) => {
                let (kt, vt) = ((**k).clone(), (**v).clone());
                let d0 = self.expr(iter)?;
                let h = self.hidden_slot(&d0, &iter.ty);
                let d = self.tmp();
                self.emit(&format!("{d} = load ptr, ptr {h}"));
                let n = self.tmp();
                self.emit(&format!("{n} = call i64 @kolay_sozluk_girdi_sayisi(ptr {d})"));
                self.counted_loop(&n, body, |me, i| {
                    let kp = me.tmp();
                    me.emit(&format!("{kp} = call ptr @kolay_sozluk_anahtar_i(ptr {d}, i64 {i})"));
                    let dead = me.tmp();
                    me.emit(&format!("{dead} = icmp eq ptr {kp}, null"));
                    let live = me.new_label();
                    let skip = me.f.loops.last().unwrap().0.clone();
                    me.term(&format!("br i1 {dead}, label %{skip}, label %{live}"));
                    me.block(&live);
                    let x = me.load_mem(&kp, &kt);
                    me.store_slot(binds[0], &x, &kt);
                    if binds.len() > 1 {
                        let vp = me.tmp();
                        me.emit(&format!("{vp} = call ptr @kolay_sozluk_deger_i(ptr {d}, i64 {i})"));
                        let y = me.load_mem(&vp, &vt);
                        me.store_slot(binds[1], &y, &vt);
                    }
                    Ok(None)
                })
            }
            Type::Str => {
                let s0 = self.expr(iter)?;
                let h = self.hidden_slot(&s0, &Type::Str);
                let s = self.tmp();
                self.emit(&format!("{s} = load ptr, ptr {h}"));
                let pos = self.alloca("i64");
                self.emit(&format!("store i64 0, ptr {pos}"));
                let (cond, bodyl, exit) = (self.new_label(), self.new_label(), self.new_label());
                self.block(&cond);
                let c = self.tmp();
                self.emit(&format!("{c} = call ptr @kolay_yazi_sonraki(ptr {s}, ptr {pos})"));
                let done = self.tmp();
                self.emit(&format!("{done} = icmp eq ptr {c}, null"));
                self.term(&format!("br i1 {done}, label %{exit}, label %{bodyl}"));
                self.block(&bodyl);
                self.own(c.clone(), &Type::Str);
                self.store_slot(binds[0], &c, &Type::Str);
                self.f.loops.push((cond.clone(), exit.clone(), self.f.deneler.len()));
                self.stmts(body)?;
                self.f.loops.pop();
                if !self.f.terminated {
                    self.term(&format!("br label %{cond}"));
                }
                self.block(&exit);
                Ok(())
            }
            _ => unreachable!("üzerinde dönülemeyen tip"),
        }
    }

    /// 0 .. n-1 sayan döngü; `head` her turun başında (gövdeden önce) çalışır.
    fn counted_loop(
        &mut self,
        n: &str,
        body: &[TS],
        head: impl FnOnce(&mut Self, &str) -> CResult<Option<()>>,
    ) -> CResult<()> {
        let ctr = self.alloca("i64");
        self.emit(&format!("store i64 0, ptr {ctr}"));
        let (cond, bodyl, step, exit) = (self.new_label(), self.new_label(), self.new_label(), self.new_label());
        self.block(&cond);
        let i = self.tmp();
        self.emit(&format!("{i} = load i64, ptr {ctr}"));
        let c = self.tmp();
        self.emit(&format!("{c} = icmp slt i64 {i}, {n}"));
        self.term(&format!("br i1 {c}, label %{bodyl}, label %{exit}"));
        self.block(&bodyl);
        self.f.loops.push((step.clone(), exit.clone(), self.f.deneler.len()));
        head(self, &i)?;
        self.stmts(body)?;
        self.f.loops.pop();
        self.block(&step);
        let (i2, inc) = (self.tmp(), self.tmp());
        self.emit(&format!("{i2} = load i64, ptr {ctr}"));
        self.emit(&format!("{inc} = add i64 {i2}, 1"));
        self.emit(&format!("store i64 {inc}, ptr {ctr}"));
        self.term(&format!("br label %{cond}"));
        self.block(&exit);
        Ok(())
    }

    /// Aralık döngüsü: liste oluşturulmadan sayılır. Tur sayısı ve ilk değer döngüden önce bir kez
    /// hesaplanır (values.rs range_plan ile aynı kural); döngü 0 .. tur sayısı kadar sayar ve değeri
    /// `ilk + k * adım` olarak bulur. Sınırlar sabitse LLVM hepsini derleme anında hesaplar.
    #[allow(clippy::too_many_arguments)]
    fn for_range(&mut self, start: &TExpr, end: &TExpr, incl: bool, step: Option<&TExpr>, loc: Loc, bind: Slot, body: &[TS]) -> CResult<()> {
        let s = self.expr(start)?;
        let e = self.expr(end)?;
        let k = match step {
            Some(x) => self.expr(x)?,
            None => "1".into(),
        };
        let (first, st, count) = self.range_plan(&s, &e, incl, &k, step.is_some(), loc);
        let ctr = self.alloca("i64");
        self.emit(&format!("store i64 0, ptr {ctr}"));
        let (cond, bodyl, stepl, exit) = (self.new_label(), self.new_label(), self.new_label(), self.new_label());
        self.block(&cond);
        let i = self.tmp();
        self.emit(&format!("{i} = load i64, ptr {ctr}"));
        let c = self.tmp();
        self.emit(&format!("{c} = icmp ult i64 {i}, {count}"));
        self.term(&format!("br i1 {c}, label %{bodyl}, label %{exit}"));
        self.block(&bodyl);
        let (m, cur) = (self.tmp(), self.tmp());
        self.emit(&format!("{m} = mul i64 {i}, {st}"));
        self.emit(&format!("{cur} = add i64 {first}, {m}"));
        self.store_slot(bind, &cur, &TAM64);
        self.f.loops.push((stepl.clone(), exit.clone(), self.f.deneler.len()));
        self.stmts(body)?;
        self.f.loops.pop();
        self.block(&stepl);
        let (i2, inc) = (self.tmp(), self.tmp());
        self.emit(&format!("{i2} = load i64, ptr {ctr}"));
        self.emit(&format!("{inc} = add i64 {i2}, 1"));
        self.emit(&format!("store i64 {inc}, ptr {ctr}"));
        self.term(&format!("br label %{cond}"));
        self.block(&exit);
        Ok(())
    }

    /// (ilk değer, adım, tur sayısı). `s..e` listesi: yön s > e ise -1; uzunluk |e - s| (+1 dahilse).
    /// Adım k: k > 0 baştan, k < 0 sondan başlar; tur sayısı (uzunluk - 1) / |k| + 1.
    fn range_plan(&mut self, s: &str, e: &str, incl: bool, k: &str, has_step: bool, loc: Loc) -> (String, String, String) {
        if has_step {
            let z = self.tmp();
            self.emit(&format!("{z} = icmp eq i64 {k}, 0"));
            let (bad, ok) = (self.new_label(), self.new_label());
            self.br_cold(&z, &bad, &ok);
            self.block(&bad);
            self.error_call("Adım sıfır olamaz.", loc);
            self.block(&ok);
        }
        let t = |me: &mut Self| me.tmp();
        let (down, dir) = (t(self), t(self));
        self.emit(&format!("{down} = icmp sgt i64 {s}, {e}"));
        self.emit(&format!("{dir} = select i1 {down}, i64 -1, i64 1"));
        let (du, dd, dist) = (t(self), t(self), t(self));
        self.emit(&format!("{du} = sub i64 {e}, {s}"));
        self.emit(&format!("{dd} = sub i64 {s}, {e}"));
        self.emit(&format!("{dist} = select i1 {down}, i64 {dd}, i64 {du}"));
        let len = if incl {
            // 2^64 elemanlı olamaz: en büyük sayıda kalır
            let (l1, w, l) = (t(self), t(self), t(self));
            self.emit(&format!("{l1} = add i64 {dist}, 1"));
            self.emit(&format!("{w} = icmp eq i64 {l1}, 0"));
            self.emit(&format!("{l} = select i1 {w}, i64 -1, i64 {l1}"));
            l
        } else {
            dist
        };
        let (count, first) = if k == "1" {
            (len.clone(), s.to_string())
        } else {
            let (neg, nk, abs) = (t(self), t(self), t(self));
            self.emit(&format!("{neg} = icmp slt i64 {k}, 0"));
            self.emit(&format!("{nk} = sub i64 0, {k}"));
            self.emit(&format!("{abs} = select i1 {neg}, i64 {nk}, i64 {k}"));
            let (l1, q, c1, z, c) = (t(self), t(self), t(self), t(self), t(self));
            self.emit(&format!("{l1} = sub i64 {len}, 1"));
            self.emit(&format!("{q} = udiv i64 {l1}, {abs}"));
            self.emit(&format!("{c1} = add i64 {q}, 1"));
            self.emit(&format!("{z} = icmp eq i64 {len}, 0"));
            self.emit(&format!("{c} = select i1 {z}, i64 0, i64 {c1}"));
            // sondan başlarken ilk değer listenin son elemanı: s + (uzunluk - 1) * yön
            let (m, last, f) = (t(self), t(self), t(self));
            self.emit(&format!("{m} = mul i64 {l1}, {dir}"));
            self.emit(&format!("{last} = add i64 {s}, {m}"));
            self.emit(&format!("{f} = select i1 {neg}, i64 {last}, i64 {s}"));
            (c, f)
        };
        let st = t(self);
        self.emit(&format!("{st} = mul i64 {dir}, {k}"));
        (first, st, count)
    }

    // ------------------------------------------------------------ eşleştir

    pub(super) fn match_code(&mut self, m: &TMatch, result: Option<(&str, &Type)>) -> CResult<()> {
        let v0 = self.expr(&m.scrut)?;
        let sty = m.scrut.ty.clone();
        // sahipli değer dallar boyunca yaşamalı: gizli değişkende tutulur
        let v = if self.f.owned.contains(&v0) {
            let h = self.hidden_slot(&v0, &sty);
            let ll = self.llty(&sty);
            let x = self.tmp();
            self.emit(&format!("{x} = load {ll}, ptr {h}"));
            x
        } else {
            v0
        };
        let end = self.new_label();
        for arm in &m.arms {
            let (bodyl, next) = (self.new_label(), self.new_label());
            match &arm.pats {
                None => self.term(&format!("br label %{bodyl}")),
                Some(ps) => {
                    for (i, p) in ps.iter().enumerate() {
                        let no = if i + 1 == ps.len() { next.clone() } else { self.new_label() };
                        self.pat_branch(p, &v, &sty, &bodyl, &no, m.loc)?;
                        if i + 1 < ps.len() {
                            self.block(&no);
                        }
                    }
                }
            }
            self.block(&bodyl);
            if let Some(ps) = &arm.pats {
                if ps.len() == 1 {
                    self.pat_bind(&ps[0], &v, &sty);
                }
            }
            if let Some(g) = &arm.guard {
                let gv = self.expr(g)?;
                let ok = self.new_label();
                self.term(&format!("br i1 {gv}, label %{ok}, label %{next}"));
                self.block(&ok);
            }
            match &arm.body {
                TArmBody::Expr(e) => {
                    let val = self.expr(e)?;
                    match result {
                        Some((ptr, ty)) if !self.f.terminated && e.ty != Type::Never => {
                            let lt = self.llty(ty);
                            if self.is_rc(ty) {
                                self.keep_ref(&val, ty);
                            }
                            self.emit(&format!("store {lt} {val}, ptr {ptr}"));
                        }
                        _ => self.drop_temp(&val),
                    }
                }
                TArmBody::Block(b) => self.stmts(b)?,
            }
            if !self.f.terminated {
                self.term(&format!("br label %{end}"));
            }
            self.block(&next);
        }
        if result.is_some() {
            self.term("unreachable");
        }
        self.block(&end);
        Ok(())
    }

    /// Desen uyarsa `yes`, uymazsa `no` etiketine dallanır.
    fn pat_branch(&mut self, p: &TPat, v: &str, sty: &Type, yes: &str, no: &str, loc: Loc) -> CResult<()> {
        if let TPat::Wild = p {
            self.term(&format!("br label %{yes}"));
            return Ok(());
        }
        if let Type::Union(ms) = sty {
            return self.union_pat_branch(p, v, ms, yes, no, loc);
        }
        let cond: String = match p {
            TPat::Wild => unreachable!(),
            TPat::Int(n) => self.int_pat_eq(v, sty, n),
            TPat::Range(a, b, incl) => self.int_pat_range(v, sty, a, b, *incl),
            TPat::Str(s) => {
                if *sty != Type::Str {
                    "false".into()
                } else {
                    let c = self.str_const(s);
                    let t = self.tmp();
                    self.emit(&format!("{t} = call zeroext i1 @kolay_yazi_esit(ptr {v}, ptr {c})"));
                    t
                }
            }
            TPat::Bool(b) => {
                let t = self.tmp();
                self.emit(&format!("{t} = icmp eq i1 {v}, {b}"));
                t
            }
            TPat::Float(x) => {
                let Type::Float(bits) = sty else { unreachable!() };
                let lt = self.llty(sty);
                let t = self.tmp();
                self.emit(&format!("{t} = fcmp oeq {lt} {v}, {}", super::ifade::float_const(*x, *bits)));
                t
            }
            TPat::Hic => (*sty == Type::Hic).to_string(),
            TPat::Variant { ty, idx, .. } => {
                let tag = self.variant_tag(v);
                let (t, same) = (self.tmp(), self.tmp());
                self.emit(&format!("{t} = icmp eq i64 {tag}, {idx}"));
                if let Type::Named(s) = sty {
                    if *s != *ty {
                        // arayüz tipindeki değer: önce gerçek türü kontrol edilir
                        let sub = self.dyn_is_subtype(v, *ty);
                        self.emit(&format!("{same} = and i1 {sub}, {t}"));
                        same
                    } else {
                        t
                    }
                } else {
                    t
                }
            }
            TPat::TypeTest { ty, .. } => match (sty, ty) {
                (Type::Named(s), Type::Named(t)) => {
                    if self.prog.is_subtype(*s, *t) {
                        "true".into()
                    } else {
                        self.dyn_is_subtype(v, *t)
                    }
                }
                _ => "true".into(),
            },
        };
        self.term(&format!("br i1 {cond}, label %{yes}, label %{no}"));
        Ok(())
    }

    /// 'veya' tipindeki değer için desen: önce etiket, sonra (gerekirse) değerin kendisi.
    fn union_pat_branch(&mut self, p: &TPat, v: &str, ms: &[Type], yes: &str, no: &str, loc: Loc) -> CResult<()> {
        let tag = self.union_tag(v);
        // (üye sırası, değer de kontrol edilecek mi)
        let mut cands: Vec<(usize, bool)> = Vec::new();
        for (i, m) in ms.iter().enumerate() {
            let c = match (p, m) {
                (TPat::Int(_) | TPat::Range(..), Type::Int { .. }) => Some(true),
                (TPat::Float(_), Type::Float(_)) => Some(true),
                (TPat::Str(_), Type::Str) => Some(true),
                (TPat::Bool(_), Type::Bool) => Some(true),
                (TPat::Hic, Type::Hic) => Some(false),
                (TPat::Variant { ty, .. }, Type::Named(t)) if self.prog.is_subtype(*ty, *t) || ty == t => Some(true),
                (TPat::TypeTest { ty, .. }, m) => match (ty, m) {
                    (Type::Named(t), Type::Named(u)) => {
                        if self.prog.is_subtype(*u, *t) {
                            Some(false)
                        } else if self.prog.is_subtype(*t, *u) {
                            Some(true)
                        } else {
                            None
                        }
                    }
                    (Type::Int { .. }, Type::Int { .. })
                    | (Type::Float(_), Type::Float(_))
                    | (Type::List(..), Type::List(..))
                    | (Type::Dict(..), Type::Dict(..)) => Some(false),
                    (a, b) if a == b => Some(false),
                    _ => None,
                },
                _ => None,
            };
            if let Some(check) = c {
                cands.push((i, check));
            }
        }
        for (k, (i, check)) in cands.iter().enumerate() {
            let next = if k + 1 == cands.len() { no.to_string() } else { self.new_label() };
            let c = self.tmp();
            self.emit(&format!("{c} = icmp eq i64 {tag}, {i}"));
            if *check {
                let inner = self.new_label();
                self.term(&format!("br i1 {c}, label %{inner}, label %{next}"));
                self.block(&inner);
                let m = ms[*i].clone();
                let x = self.union_unpack(v, &m);
                self.pat_branch(p, &x, &m, yes, &next, loc)?;
            } else {
                self.term(&format!("br i1 {c}, label %{yes}, label %{next}"));
            }
            if k + 1 < cands.len() {
                self.block(&next);
            }
        }
        if cands.is_empty() {
            self.term(&format!("br label %{no}"));
        }
        Ok(())
    }

    /// Uyan desenin değişkenlerine değer verir.
    fn pat_bind(&mut self, p: &TPat, v: &str, sty: &Type) {
        match p {
            TPat::Variant { ty, idx, binds } => {
                if binds.iter().all(Option::is_none) {
                    return;
                }
                let obj = if let Type::Union(_) = sty { self.union_unpack(v, &Type::Named(*ty)) } else { v.to_string() };
                let TypeDefKind::Secenek { variants } = &self.prog.types[*ty].kind else { unreachable!() };
                let fields = variants[*idx as usize].fields.clone();
                for (j, b) in binds.iter().enumerate() {
                    if let Some(s) = b {
                        let off = self.variant_offsets[*ty][*idx as usize][j];
                        let fp = self.tmp();
                        self.emit(&format!("{fp} = getelementptr i8, ptr {obj}, i64 {off}"));
                        let x = self.load_mem(&fp, &fields[j].1);
                        self.store_slot(*s, &x, &fields[j].1);
                    }
                }
            }
            TPat::TypeTest { ty, bind: Some(s) } => {
                let x = if let Type::Union(_) = sty {
                    if let Type::Union(_) = ty {
                        v.to_string()
                    } else {
                        self.union_unpack(v, ty)
                    }
                } else {
                    v.to_string()
                };
                self.store_slot(*s, &x, ty);
            }
            _ => {}
        }
    }

    fn int_pat_eq(&mut self, v: &str, sty: &Type, n: &BigInt) -> String {
        if is_esnek(sty) {
            let c = self.esnek_const(n);
            let r = self.tmp();
            self.emit(&format!("{r} = call i32 @kolay_esnek_kars(i64 {v}, i64 {c})"));
            self.drop_temp(&c);
            let t = self.tmp();
            self.emit(&format!("{t} = icmp eq i32 {r}, 0"));
            return t;
        }
        if !is_fixed_int(sty) {
            return "false".into();
        }
        let (b, s) = int_parts(sty);
        if !fits(n, Some(b), s) {
            return "false".into();
        }
        let w = width(sty);
        let t = self.tmp();
        self.emit(&format!("{t} = icmp eq i{w} {v}, {}", super::ifade::int_const(n, w)));
        t
    }

    fn int_pat_range(&mut self, v: &str, sty: &Type, a: &BigInt, b: &BigInt, incl: bool) -> String {
        if is_esnek(sty) {
            let (ca, cb) = (self.esnek_const(a), self.esnek_const(b));
            let (r1, r2) = (self.tmp(), self.tmp());
            self.emit(&format!("{r1} = call i32 @kolay_esnek_kars(i64 {v}, i64 {ca})"));
            self.emit(&format!("{r2} = call i32 @kolay_esnek_kars(i64 {v}, i64 {cb})"));
            self.drop_temp(&ca);
            self.drop_temp(&cb);
            let (t1, t2, t3) = (self.tmp(), self.tmp(), self.tmp());
            self.emit(&format!("{t1} = icmp sge i32 {r1}, 0"));
            self.emit(&format!("{t2} = icmp {} i32 {r2}, 0", if incl { "sle" } else { "slt" }));
            self.emit(&format!("{t3} = and i1 {t1}, {t2}"));
            return t3;
        }
        let (bits, s) = int_parts(sty);
        let (lo, hi) = int_bounds(Some(bits), s);
        let (lo, hi) = (lo.unwrap(), hi.unwrap());
        let hi_incl = if incl { b.clone() } else { b - 1 };
        if a > &hi || hi_incl < lo || a > &hi_incl {
            return "false".into();
        }
        let a = if a < &lo { lo.clone() } else { a.clone() };
        let h = if hi_incl > hi { hi } else { hi_incl };
        let w = width(sty);
        let (ge, le) = if s == Sign::Pos { ("uge", "ule") } else { ("sge", "sle") };
        let (t1, t2, t3) = (self.tmp(), self.tmp(), self.tmp());
        self.emit(&format!("{t1} = icmp {ge} i{w} {v}, {}", super::ifade::int_const(&a, w)));
        self.emit(&format!("{t2} = icmp {le} i{w} {v}, {}", super::ifade::int_const(&h, w)));
        self.emit(&format!("{t3} = and i1 {t1}, {t2}"));
        t3
    }
}

/// Komutlarda (iç içe olanlar dahil) dene bloğu var mı?
pub(super) fn dene_var(stmts: &[TS]) -> bool {
    stmts.iter().any(|s| match s {
        TS::Dene { .. } => true,
        TS::If(bs, el) => bs.iter().any(|(_, b)| dene_var(b)) || el.as_deref().is_some_and(dene_var),
        TS::Kere(_, b) | TS::While(_, b) | TS::ForEach { body: b, .. } => dene_var(b),
        TS::CFor(_, _, _, b) => dene_var(b),
        TS::Match(m) => m.arms.iter().any(|a| matches!(&a.body, TArmBody::Block(b) if dene_var(b))),
        _ => false,
    })
}
