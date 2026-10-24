// Yorumlayıcı: TIR'ı doğrudan çalıştırır (`kolay kod.kolay`).

use crate::ast::BinOp;
use crate::tir::*;
use crate::types::{bounds_text, group_digits, int_bounds, Sign, Type, TypeId};
use crate::values::*;
use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToPrimitive};
use std::cmp::Ordering;
use std::io::{BufRead, Write};
use std::rc::Rc;

pub struct RtErr {
    /// Yakalanmazsa yazılacak mesaj
    pub msg: String,
    pub loc: Loc,
    /// Çalışma hatasının türü (tir::hata_turu); fırlatılan değerde kullanılmaz
    pub tur: u8,
    /// `fırlat` ile fırlatılan değer ve statik tipi (çalışma hatasında None: yakalanınca Hata.X nesnesi olur)
    pub deger: Option<Box<(Value, Type)>>,
}

pub enum Unwind {
    Err(RtErr),
    Ctl(Ctl),
}

pub enum Ctl {
    Normal,
    Break,
    Continue,
    Return(Value),
}

pub(crate) type R<T> = Result<T, Unwind>;

pub(crate) fn rt<T>(loc: Loc, msg: impl Into<String>) -> R<T> {
    rtk(hata_turu::GENEL, loc, msg)
}

/// Türü belli çalışma hatası (yakalanınca hangi Hata.X nesnesi olacağı)
pub(crate) fn rtk<T>(tur: u8, loc: Loc, msg: impl Into<String>) -> R<T> {
    Err(Unwind::Err(RtErr { msg: msg.into(), loc, tur, deger: None }))
}

/// Hazır hata nesnesi: Hata.X(mesaj: ...)
pub(crate) fn hata_nesnesi(prog: &Program, tur: u8, msg: &str) -> Value {
    Value::Obj(Rc::new(ObjV { ty: prog.hata_turleri[tur as usize], fields: vec![Value::str(msg)] }))
}

pub(crate) const BELIRSIZ: &str = "Değişken henüz belirsiz (bir fonksiyon, değişken tanımlanmadan önce çağrıldı).";

/// Metodun çalıştığı nesne (`bu`).
pub(crate) enum BuRef {
    /// Nesne bu çerçevenin içinde (yerel değişkenden alınıp çağrı bitince geri yazılır).
    Val(Value),
    /// Nesne bir dosya değişkeninin (global) içinde; metot onu yerinde değiştirir. Böylece metot
    /// çalışırken başka fonksiyonlar aynı nesneyi okuduğunda güncel halini görür.
    Alias(u32, Vec<Step>),
}

pub(crate) struct Frame {
    pub(crate) slots: Vec<Value>,
    pub(crate) bu: BuRef,
}

#[derive(Clone, Copy)]
pub(crate) enum Root {
    Local(u32),
    Global(u32),
    Bu,
}

#[derive(Clone)]
pub(crate) enum Step {
    Field(u32),
    Idx(i64),
    Key(Key, Value),
}

pub struct Interp<'p> {
    pub(crate) prog: &'p Program,
    pub(crate) globals: Vec<Value>,
    pub(crate) ran: Vec<bool>,
    pub(crate) out: std::io::BufWriter<std::io::Stdout>,
    /// Çıktı ekrana gidiyorsa her satır hemen yazılır (uzun süren programda yazılanlar görünsün);
    /// dosyaya ya da boruya gidiyorsa hız için tamponda biriktirilir.
    pub(crate) line_flush: bool,
    pub(crate) depth: u32,
    pub(crate) pool: Vec<Vec<Value>>,
    /// yer adımları için tekrar kullanılan listeler (resolve_place)
    pub(crate) step_pool: Vec<Vec<Step>>,
    /// Fonksiyonların ve dosyaların bytecode'u (vm.rs)
    pub(crate) chunks: Vec<crate::vm::Chunk<'p>>,
    pub(crate) mchunks: Vec<crate::vm::Chunk<'p>>,
    /// dış (C) fonksiyonların adresleri (fonksiyon numarasına göre)
    pub(crate) dis: Vec<usize>,
}

pub(crate) const MAX_DEPTH: u32 = 100_000;

/// REPL'de Ctrl+C basıldı mı (çalışan hücre döngü turlarında durdurulur). Programlar Ctrl+C'de
/// doğrudan kapanır (main.rs), bu bayrak sadece REPL'de kullanılır.
pub(crate) static KESME: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[inline(always)]
pub(crate) fn kesildi() -> bool {
    KESME.load(std::sync::atomic::Ordering::Relaxed)
}

pub(crate) const DURDURULDU: &str = "Durduruldu (Ctrl+C).";

/// Komutun konumu (Ctrl+C mesajı için yaklaşık yer)
fn stmt_loc(s: &TS) -> Loc {
    match s {
        TS::Let(_, e) | TS::Expr(e) | TS::Kere(e, _) | TS::While(e, _) => e.loc,
        TS::Assign(_, _, l) | TS::Compound(_, _, _, _, l) => *l,
        TS::ForEach { iter, .. } => iter.loc,
        _ => Loc::default(),
    }
}

pub(crate) fn norm_index(i: i64, len: usize, loc: Loc) -> R<usize> {
    let real = if i < 0 { i.checked_add(len as i64).unwrap_or(-1) } else { i };
    if real < 0 || real >= len as i64 {
        if len == 0 {
            return rtk(hata_turu::INDEKS, loc, format!("Listenin {i}. indeksine erişilmek istendi, ama liste boş."));
        }
        return rtk(hata_turu::INDEKS, loc, format!("Listenin {i}. indeksine erişilmek istendi, ama liste {len} elemanlı (0 .. {}).", len - 1));
    }
    Ok(real as usize)
}

pub(crate) fn field_idx(prog: &Program, ty: TypeId, fid: u32, loc: Loc) -> R<usize> {
    match prog.field_pos[ty].get(fid as usize) {
        Some(&i) if i != u32::MAX => Ok(i as usize),
        _ => rt(
            loc,
            format!(
                "'{}' alanı bulunamadı: metodu çalışan nesne bu sırada başka bir {} değeriyle değiştirildi.",
                prog.field_names[fid as usize], prog.types[ty].name
            ),
        ),
    }
}

pub(crate) fn missing_key(prog: &Program, kv: &Value, loc: Loc) -> Unwind {
    Unwind::Err(RtErr { msg: format!("{} anahtarı sözlükte yok.", fmt_value(kv, None, prog, false)), loc, tur: hata_turu::ANAHTAR, deger: None })
}

pub(crate) fn key_of(v: &Value, loc: Loc) -> R<Key> {
    match v.key() {
        Some(k) => Ok(k),
        None => rt(loc, "Bu değer sözlük anahtarı olarak kullanılamaz."),
    }
}

/// Yeri değiştirmek için yürür (paylaşılan nesneler gerekiyorsa kopyalanır).
pub(crate) fn walk_mut<'a>(mut cur: &'a mut Value, steps: &[Step], create_last: bool, loc: Loc, prog: &Program) -> R<&'a mut Value> {
    let n = steps.len();
    for (i, st) in steps.iter().enumerate() {
        cur = match st {
            Step::Field(fid) => match cur {
                Value::Obj(rc) => {
                    let idx = field_idx(prog, rc.ty, *fid, loc)?;
                    &mut Rc::make_mut(rc).fields[idx]
                }
                _ => return rt(loc, format!("'{}' alanına erişilemedi.", prog.field_names[*fid as usize])),
            },
            Step::Idx(ix) => match cur {
                Value::List(rc) => {
                    let real = norm_index(*ix, rc.items.len(), loc)?;
                    &mut Rc::make_mut(rc).items[real]
                }
                _ => return rt(loc, "Liste bekleniyordu."),
            },
            Step::Key(k, kv) => match cur {
                Value::Dict(rc) => {
                    let d = Rc::make_mut(rc);
                    if i == n - 1 && create_last {
                        let e = d.map.entry(k.clone()).or_insert_with(|| (kv.clone(), Value::Hic));
                        &mut e.1
                    } else {
                        match d.map.get_mut(k) {
                            Some(e) => &mut e.1,
                            None => return Err(missing_key(prog, kv, loc)),
                        }
                    }
                }
                _ => return rt(loc, "Sözlük bekleniyordu."),
            },
        };
    }
    Ok(cur)
}

/// Yeri sadece okumak için yürür (hiçbir şey kopyalanmaz).
pub(crate) fn walk_ref<'a>(mut cur: &'a Value, steps: &[Step], loc: Loc, prog: &Program) -> R<&'a Value> {
    for st in steps {
        cur = match (st, cur) {
            (Step::Field(fid), Value::Obj(o)) => &o.fields[field_idx(prog, o.ty, *fid, loc)?],
            (Step::Idx(ix), Value::List(l)) => &l.items[norm_index(*ix, l.items.len(), loc)?],
            (Step::Key(k, kv), Value::Dict(d)) => match d.map.get(k) {
                Some(e) => &e.1,
                None => return Err(missing_key(prog, kv, loc)),
            },
            (Step::Field(fid), _) => return rt(loc, format!("'{}' alanına erişilemedi.", prog.field_names[*fid as usize])),
            (Step::Idx(_), _) => return rt(loc, "Liste bekleniyordu."),
            (Step::Key(..), _) => return rt(loc, "Sözlük bekleniyordu."),
        };
    }
    Ok(cur)
}

impl<'p> Interp<'p> {
    pub fn new(prog: &'p Program) -> Self {
        Interp {
            prog,
            globals: vec![Value::Uninit; prog.nglobals as usize],
            ran: vec![false; prog.modules.len()],
            out: std::io::BufWriter::new(std::io::stdout()),
            line_flush: std::io::IsTerminal::is_terminal(&std::io::stdout()),
            depth: 0,
            pool: Vec::new(),
            step_pool: Vec::new(),
            chunks: prog.funcs.iter().map(|f| crate::vm::compile_func(prog, f)).collect(),
            mchunks: prog.modules.iter().map(|m| crate::vm::compile_module(prog, m)).collect(),
            dis: Vec::new(),
        }
    }

    /// yazdır'ın bir satırı
    pub(crate) fn write_line(&mut self, line: &str) {
        let _ = self.out.write_all(line.as_bytes());
        if self.line_flush {
            let _ = self.out.flush();
        }
    }

    /// REPL hücresini çalıştırır; son satır bir ifadeyse değerini döndürür.
    pub fn run_cell(&mut self, stmts: &'p [TS], value: Option<&'p TExpr>) -> Result<Option<Value>, RtErr> {
        let ch = crate::vm::compile_cell(self.prog, stmts, value);
        let mut f = Frame { slots: vec![Value::Uninit; ch.nregs as usize], bu: BuRef::Val(Value::Hic) };
        let r = self.run_chunk(&ch, &mut f);
        let _ = self.out.flush();
        match r {
            Ok(v) => Ok(value.map(|_| v)),
            Err(Unwind::Ctl(_)) => Ok(None),
            Err(Unwind::Err(e)) => Err(e),
        }
    }

    pub fn run(&mut self) -> Result<(), RtErr> {
        let r = self.run_module(self.prog.main_module);
        let _ = self.out.flush();
        match r {
            Ok(()) | Err(Unwind::Ctl(_)) => Ok(()),
            Err(Unwind::Err(e)) => Err(e),
        }
    }

    pub(crate) fn run_module(&mut self, m: usize) -> R<()> {
        if self.ran[m] {
            return Ok(());
        }
        self.ran[m] = true;
        let ch: *const crate::vm::Chunk<'p> = &self.mchunks[m];
        let ch = unsafe { &*ch };
        let mut f = Frame { slots: vec![Value::Uninit; ch.nregs as usize], bu: BuRef::Val(Value::Hic) };
        self.run_chunk(ch, &mut f)?;
        Ok(())
    }

    pub(crate) fn exec_block(&mut self, stmts: &[TS], f: &mut Frame) -> R<Ctl> {
        for s in stmts {
            match self.exec(s, f) {
                Ok(Ctl::Normal) => {}
                Ok(c) | Err(Unwind::Ctl(c)) => return Ok(c),
                Err(e) => return Err(e),
            }
        }
        Ok(Ctl::Normal)
    }

    pub(crate) fn set_slot(&mut self, slot: Slot, v: Value, f: &mut Frame) {
        match slot {
            Slot::Local(i) => f.slots[i as usize] = v,
            Slot::Global(i) => self.globals[i as usize] = v,
        }
    }

    pub(crate) fn loop_ctl(&mut self, body: &[TS], f: &mut Frame) -> R<Option<Ctl>> {
        if kesildi() {
            return rtk(hata_turu::YAKALANAMAZ, body.first().map(stmt_loc).unwrap_or_default(), DURDURULDU);
        }
        match self.exec_block(body, f)? {
            Ctl::Normal | Ctl::Continue => Ok(None),
            Ctl::Break => Ok(Some(Ctl::Normal)),
            r @ Ctl::Return(_) => Ok(Some(r)),
        }
    }

    /// İşlem sonucunu hesaplar: yazılarda ekleme (yerinde), sayılarda aritmetik.
    pub(crate) fn compound_value(&self, op: BinOp, cur: Value, rhs: Value, ty: &Type, loc: Loc) -> Result<Value, (Unwind, Value)> {
        if *ty == Type::Str {
            let (Value::Str(mut a), Value::Str(b)) = (cur, rhs) else { unreachable!() };
            Rc::make_mut(&mut a).push_str(&b);
            return Ok(Value::Str(a));
        }
        let keep = cur.clone();
        self.arith(op, cur, rhs, ty, loc).map_err(|e| (e, keep))
    }

    pub(crate) fn exec(&mut self, s: &TS, f: &mut Frame) -> R<Ctl> {
        match s {
            TS::Let(slot, e) => {
                let v = self.eval(e, f)?;
                self.set_slot(*slot, v, f);
            }
            TS::Declare(slot, _) => self.set_slot(*slot, Value::Uninit, f),
            TS::Assign(TPlace::Var(slot), e, _) => {
                let v = self.eval(e, f)?;
                self.set_slot(*slot, v, f);
            }
            TS::Compound(place, op, e, ty, loc) => {
                let rhs = self.eval(e, f)?;
                self.compound_place(place, *op, rhs, ty, f, *loc)?;
            }
            TS::Expr(e) => {
                self.eval(e, f)?;
            }
            TS::If(branches, else_) => {
                for (c, b) in branches {
                    if self.truthy(c, f)? {
                        return self.exec_block(b, f);
                    }
                }
                if let Some(b) = else_ {
                    return self.exec_block(b, f);
                }
            }
            TS::Break => return Ok(Ctl::Break),
            TS::Continue => return Ok(Ctl::Continue),
            TS::Return(e) => {
                let v = match e {
                    Some(e) => self.eval(e, f)?,
                    None => Value::Hic,
                };
                return Ok(Ctl::Return(v));
            }
            _ => return self.exec_cold(s, f),
        }
        Ok(Ctl::Normal)
    }

    /// `yer op= değer` (yazılarda +=, sayılarda aritmetik)
    pub(crate) fn compound_place(&mut self, place: &TPlace, op: BinOp, rhs: Value, ty: &Type, f: &mut Frame, loc: Loc) -> R<()> {
        if let TPlace::Var(slot) = place {
                let loc = &loc;
                let (op, ty) = (&op, ty);
                let cur = match slot {
                    Slot::Local(i) => std::mem::replace(&mut f.slots[*i as usize], Value::Hic),
                    Slot::Global(i) => {
                        if matches!(self.globals[*i as usize], Value::Uninit) {
                            return rtk(hata_turu::YAKALANAMAZ, *loc, BELIRSIZ);
                        }
                        std::mem::replace(&mut self.globals[*i as usize], Value::Hic)
                    }
                };
                match self.compound_value(*op, cur, rhs, ty, *loc) {
                    Ok(new) => self.set_slot(*slot, new, f),
                    Err((e, keep)) => {
                        self.set_slot(*slot, keep, f);
                        return Err(e);
                    }
                }
                return Ok(());
        }
        let rp = self.resolve_place(place, f)?;
        let cur = self.take_resolved(&rp, f, loc)?;
        let r = match self.compound_value(op, cur, rhs, ty, loc) {
            Ok(new) => self.write_resolved(&rp, new, f, loc),
            Err((e, keep)) => {
                self.write_resolved(&rp, keep, f, loc)?;
                Err(e)
            }
        };
        self.recycle(rp.1);
        r
    }

    #[inline(never)]
    pub(crate) fn exec_cold(&mut self, s: &TS, f: &mut Frame) -> R<Ctl> {
        match s {
            TS::Assign(place, e, loc) => {
                let v = self.eval(e, f)?;
                self.assign_place(place, v, f, *loc)?;
            }
            TS::Kere(c, body) => {
                let n = self.eval(c, f)?;
                let Value::Int(n) = n else { unreachable!() };
                if n.cmp_int(&Int::S(0)) == Ordering::Less {
                    return rt(c.loc, format!("'kere' için tekrar sayısı negatif olamaz ({n})."));
                }
                // Tam(128, +) gibi 64 bite sığmayan sayılar da olabilir.
                let n: u128 = match &n {
                    Int::S(v) => *v as u128,
                    Int::B(b) => b.to_u128().unwrap_or(u128::MAX),
                };
                let mut i: u128 = 0;
                while i < n {
                    if let Some(c) = self.loop_ctl(body, f)? {
                        return Ok(c);
                    }
                    i += 1;
                }
            }
            TS::ForEach { iter, binds, body } => {
                if let TK::Range { start, end, inclusive, step } = &iter.kind {
                    // aralık listesi oluşturulmadan sayılır
                    let plan = self.range_plan(start, end, *inclusive, step.as_deref(), iter.loc, f)?;
                    for i in 0..plan.count {
                        self.set_slot(binds[0], Value::int(plan.get(i)), f);
                        if let Some(c) = self.loop_ctl(body, f)? {
                            return Ok(c);
                        }
                    }
                    return Ok(Ctl::Normal);
                }
                let v = self.eval(iter, f)?;
                match v {
                    Value::List(l) => {
                        for item in l.items.iter() {
                            self.set_slot(binds[0], item.clone(), f);
                            if let Some(c) = self.loop_ctl(body, f)? {
                                return Ok(c);
                            }
                        }
                    }
                    Value::Dict(d) => {
                        for (k, v) in d.map.values() {
                            self.set_slot(binds[0], k.clone(), f);
                            if binds.len() > 1 {
                                self.set_slot(binds[1], v.clone(), f);
                            }
                            if let Some(c) = self.loop_ctl(body, f)? {
                                return Ok(c);
                            }
                        }
                    }
                    Value::Str(s) => {
                        let mut buf = [0u8; 4];
                        for ch in s.chars() {
                            self.set_slot(binds[0], Value::str(ch.encode_utf8(&mut buf)), f);
                            if let Some(c) = self.loop_ctl(body, f)? {
                                return Ok(c);
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            TS::While(c, body) => {
                while self.truthy(c, f)? {
                    if let Some(c) = self.loop_ctl(body, f)? {
                        return Ok(c);
                    }
                }
            }
            TS::CFor(init, c, step, body) => {
                match self.exec(init, f)? {
                    Ctl::Normal => {}
                    other => return Ok(other),
                }
                while self.truthy(c, f)? {
                    if let Some(c) = self.loop_ctl(body, f)? {
                        return Ok(c);
                    }
                    self.exec(step, f)?;
                }
            }
            TS::Match(m) => {
                if let Some(i) = self.choose_arm(m, f)? {
                    match &m.arms[i].body {
                        TArmBody::Block(b) => return self.exec_block(b, f),
                        TArmBody::Expr(e) => {
                            self.eval(e, f)?;
                        }
                    }
                }
            }
            TS::RunModule(m) => self.run_module(*m)?,
            TS::Throw(e) => {
                let v = self.eval(e, f)?;
                return Err(Unwind::Err(self.firlat(v, &e.ty, e.loc)));
            }
            TS::Dene { body, catches } => {
                return match self.exec_block(body, f) {
                    Err(Unwind::Err(e)) => match self.yakala_sec(e, catches, f)? {
                        None => Ok(Ctl::Normal),
                        Some(i) => self.exec_block(&catches[i].body, f),
                    },
                    r => r,
                };
            }
            _ => unreachable!(),
        }
        Ok(Ctl::Normal)
    }

    /// `fırlat değer`: 'veya' tipindeki değerin kendi üyesinin tipi kullanılır.
    pub(crate) fn firlat(&self, v: Value, ty: &Type, loc: Loc) -> RtErr {
        let prog = self.prog;
        let ty = match ty {
            Type::Union(ms) => ms.iter().find(|m| value_matches(&v, m, prog)).cloned().unwrap_or_else(|| ty.clone()),
            t => t.clone(),
        };
        let msg = match &v {
            // Hata nesnesi: yakalanmazsa mesajı yazılır
            Value::Obj(o) if prog.is_subtype(o.ty, prog.hata_turleri[0]) => match &o.fields[0] {
                Value::Str(m) => m.to_string(),
                _ => String::new(),
            },
            _ => format!("Yakalanmamış değer fırlatıldı: {} ({}).", fmt_value(&v, Some(&ty), prog, false), self.tn(&ty)),
        };
        RtErr { msg, loc, tur: hata_turu::GENEL, deger: Some(Box::new((v, ty))) }
    }

    /// Hatayı yakalayan ilk dalı seçer ve değişkenine değeri yazar. Dal yoksa hata yok sayılır (None);
    /// hiçbir dal uymuyorsa (ya da hata yakalanamazsa) hata olduğu gibi geri döner.
    pub(crate) fn yakala_sec(&mut self, e: RtErr, catches: &[TCatch], f: &mut Frame) -> R<Option<usize>> {
        if e.tur == hata_turu::YAKALANAMAZ {
            return Err(Unwind::Err(e));
        }
        if catches.is_empty() {
            return Ok(None);
        }
        let prog = self.prog;
        let (v, ty) = match &e.deger {
            Some(d) => (d.0.clone(), d.1.clone()),
            None => (hata_nesnesi(prog, e.tur, &e.msg), Type::Named(prog.hata_turleri[e.tur as usize])),
        };
        for (i, c) in catches.iter().enumerate() {
            let Some((_, dinamik)) = c.kaynak.iter().find(|k| k.0 == ty) else { continue };
            if *dinamik && !c.ty.as_ref().is_some_and(|t| value_matches(&v, t, prog)) {
                continue;
            }
            if let Some(slot) = c.bind {
                // Tam -> Ondalık gibi biçim değiştiren tek dönüştürme
                let v = match (&v, &c.bind_ty) {
                    (Value::Int(n), Type::Float(b)) => Value::Float(round_float(n.to_f64(), *b)),
                    _ => v,
                };
                self.set_slot(slot, v, f);
            }
            return Ok(Some(i));
        }
        Err(Unwind::Err(e))
    }

    pub(crate) fn truthy(&mut self, e: &TExpr, f: &mut Frame) -> R<bool> {
        match self.eval(e, f)? {
            Value::Bool(b) => Ok(b),
            _ => unreachable!("Mantık bekleniyordu"),
        }
    }

    // ------------------------------------------------------------ yerler (atama hedefleri)

    /// Yerin kökü ve adımları. Adım listesi havuzdan alınır (her metot çağrısında bellek ayrılmasın);
    /// işi biten liste `recycle` ile geri verilir.
    pub(crate) fn resolve_place(&mut self, p: &TPlace, f: &mut Frame) -> R<(Root, Vec<Step>)> {
        let mut steps = self.step_pool.pop().unwrap_or_default();
        let r = self.resolve_into(p, f, &mut steps)?;
        Ok((r, steps))
    }

    pub(crate) fn recycle(&mut self, mut steps: Vec<Step>) {
        if self.step_pool.len() < 64 {
            steps.clear();
            self.step_pool.push(steps);
        }
    }

    fn resolve_into(&mut self, p: &TPlace, f: &mut Frame, s: &mut Vec<Step>) -> R<Root> {
        match p {
            TPlace::Var(Slot::Local(i)) => Ok(Root::Local(*i)),
            TPlace::Var(Slot::Global(i)) => Ok(Root::Global(*i)),
            TPlace::Bu => Ok(match &f.bu {
                BuRef::Val(_) => Root::Bu,
                BuRef::Alias(g, steps) => {
                    s.extend_from_slice(steps);
                    Root::Global(*g)
                }
            }),
            TPlace::Field(b, n) => {
                let r = self.resolve_into(b, f, s)?;
                s.push(Step::Field(*n));
                Ok(r)
            }
            TPlace::ListIndex(b, i) => {
                let r = self.resolve_into(b, f, s)?;
                let Value::Int(iv) = self.eval(i, f)? else { unreachable!() };
                let Some(ix) = iv.as_i64() else { return rtk(hata_turu::INDEKS, i.loc, "İndeks çok büyük.") };
                s.push(Step::Idx(ix));
                Ok(r)
            }
            TPlace::DictKey(b, k) => {
                let r = self.resolve_into(b, f, s)?;
                let kv = self.eval(k, f)?;
                s.push(Step::Key(key_of(&kv, k.loc)?, kv));
                Ok(r)
            }
        }
    }

    /// Kökteki değer (dosya değişkeni henüz atanmamışsa Türkçe hata).
    pub(crate) fn root_mut<'a>(globals: &'a mut [Value], f: &'a mut Frame, root: Root, loc: Loc) -> R<&'a mut Value> {
        let v = match root {
            Root::Local(i) => &mut f.slots[i as usize],
            Root::Global(i) => &mut globals[i as usize],
            Root::Bu => match &mut f.bu {
                BuRef::Val(v) => v,
                BuRef::Alias(..) => unreachable!("takma ad resolve_place içinde açılır"),
            },
        };
        if matches!(v, Value::Uninit) {
            return rtk(hata_turu::YAKALANAMAZ, loc, BELIRSIZ);
        }
        Ok(v)
    }

    pub(crate) fn write_resolved(&mut self, rp: &(Root, Vec<Step>), v: Value, f: &mut Frame, loc: Loc) -> R<()> {
        let prog = self.prog;
        if rp.1.is_empty() {
            // Değişkenin kendisine atama: önceki değeri belirsiz olabilir.
            match rp.0 {
                Root::Local(i) => f.slots[i as usize] = v,
                Root::Global(i) => self.globals[i as usize] = v,
                Root::Bu => match &mut f.bu {
                    BuRef::Val(b) => *b = v,
                    BuRef::Alias(..) => unreachable!(),
                },
            }
            return Ok(());
        }
        let root = Self::root_mut(&mut self.globals, f, rp.0, loc)?;
        let slot = walk_mut(root, &rp.1, true, loc, prog)?;
        *slot = v;
        Ok(())
    }

    pub(crate) fn assign_place(&mut self, p: &TPlace, v: Value, f: &mut Frame, loc: Loc) -> R<()> {
        let rp = self.resolve_place(p, f)?;
        let r = self.write_resolved(&rp, v, f, loc);
        self.recycle(rp.1);
        r
    }

    pub(crate) fn take_resolved(&mut self, rp: &(Root, Vec<Step>), f: &mut Frame, loc: Loc) -> R<Value> {
        let prog = self.prog;
        let root = Self::root_mut(&mut self.globals, f, rp.0, loc)?;
        let slot = walk_mut(root, &rp.1, false, loc, prog)?;
        Ok(std::mem::replace(slot, Value::Hic))
    }

    // ------------------------------------------------------------ ifadeler

    pub(crate) fn bu_value(&self, f: &Frame, loc: Loc) -> R<Value> {
        match &f.bu {
            BuRef::Val(v) => Ok(v.clone()),
            BuRef::Alias(g, steps) => Ok(walk_ref(&self.globals[*g as usize], steps, loc, self.prog)?.clone()),
        }
    }

    pub(crate) fn field_of(&self, v: &Value, fid: u32, loc: Loc) -> R<Value> {
        match v {
            Value::Obj(o) => Ok(o.fields[field_idx(self.prog, o.ty, fid, loc)?].clone()),
            _ => unreachable!(),
        }
    }

    pub(crate) fn eval(&mut self, e: &TExpr, f: &mut Frame) -> R<Value> {
        let prog = self.prog;
        Ok(match &e.kind {
            TK::Int(n, small) => match small {
                Some(v) => Value::Int(Int::S(*v)),
                None => Value::Int(Int::B(Rc::new(n.clone()))),
            },
            TK::Float(x) => {
                let bits = if let Type::Float(b) = e.ty { b } else { 64 };
                Value::Float(round_float(*x, bits))
            }
            TK::Str(s) => Value::Str(s.clone()),
            TK::Bool(b) => Value::Bool(*b),
            TK::Hic => Value::Hic,
            TK::Var(Slot::Local(i)) => f.slots[*i as usize].clone(),
            TK::Var(Slot::Global(i)) => {
                let v = &self.globals[*i as usize];
                if matches!(v, Value::Uninit) {
                    return rtk(hata_turu::YAKALANAMAZ, e.loc, BELIRSIZ);
                }
                v.clone()
            }
            TK::Not(x) => {
                let b = self.truthy(x, f)?;
                Value::Bool(!b)
            }
            TK::Arith(op, l, r) => {
                let a = self.eval(l, f)?;
                let b = self.eval(r, f)?;
                self.arith(*op, a, b, &e.ty, e.loc)?
            }
            TK::Cmp(op, l, r) => {
                let a = self.eval(l, f)?;
                let b = self.eval(r, f)?;
                Value::Bool(compare(*op, &a, &b))
            }
            TK::And(l, r) => Value::Bool(self.truthy(l, f)? && self.truthy(r, f)?),
            TK::Or(l, r) => Value::Bool(self.truthy(l, f)? || self.truthy(r, f)?),
            TK::Convert(x) => {
                let v = self.eval(x, f)?;
                match (&v, &e.ty) {
                    (Value::Int(i), Type::Float(b)) => Value::Float(round_float(i.to_f64(), *b)),
                    (Value::List(l), Type::List(_, Some(n))) if l.items.len() != *n => {
                        return rtk(hata_turu::DONUSUM, 
                            e.loc,
                            format!("{} tam {n} eleman almalı, ama {} elemanlı bir liste verildi.", self.tn(&e.ty), l.items.len()),
                        );
                    }
                    _ => v,
                }
            }
            TK::Call(fid, args) => {
                let mut vals = self.pool.pop().unwrap_or_default();
                for a in args {
                    vals.push(self.eval(a, f)?);
                }
                self.call_plain(*fid, vals, e.loc)?
            }
            TK::Variant(tid, idx, args) => {
                let mut fs = Vec::with_capacity(args.len());
                for a in args {
                    fs.push(self.eval(a, f)?);
                }
                Value::Var(Rc::new(VarV { ty: *tid, idx: *idx, fields: fs }))
            }
            TK::Field(x, fid) => match &x.kind {
                // Sık kullanılan durumlarda nesne kopyalanmadan doğrudan okunur.
                TK::Bu => match &f.bu {
                    BuRef::Val(v) => self.field_of(v, *fid, e.loc)?,
                    BuRef::Alias(g, steps) => {
                        let v = walk_ref(&self.globals[*g as usize], steps, e.loc, prog)?;
                        self.field_of(v, *fid, e.loc)?
                    }
                },
                TK::Var(Slot::Local(i)) => self.field_of(&f.slots[*i as usize], *fid, e.loc)?,
                _ => {
                    let v = self.eval(x, f)?;
                    self.field_of(&v, *fid, e.loc)?
                }
            },
            _ => return self.eval_cold(e, f),
        })
    }

    /// Sık çalışmayan ifadeler ayrı fonksiyonda: eval'in yığın çerçevesi küçük kalsın diye.
    #[inline(never)]
    pub(crate) fn eval_cold(&mut self, e: &TExpr, f: &mut Frame) -> R<Value> {
        let prog = self.prog;
        Ok(match &e.kind {
            TK::Bu => self.bu_value(f, e.loc)?,
            TK::FStr(parts) => {
                let mut s = String::new();
                for p in parts {
                    match p {
                        TFPart::Lit(l) => s.push_str(l),
                        TFPart::Expr(x) => {
                            let v = self.eval(x, f)?;
                            write_value(&mut s, &v, Some(&x.ty), prog, true);
                        }
                    }
                }
                Value::string(s)
            }
            TK::List(items) => {
                let mut v = Vec::with_capacity(items.len());
                for it in items {
                    v.push(self.eval(it, f)?);
                }
                Value::List(Rc::new(ListV { items: v }))
            }
            TK::Dict(items) => {
                let mut d = DictV::default();
                for (k, v) in items {
                    let kv = self.eval(k, f)?;
                    let vv = self.eval(v, f)?;
                    d.map.insert(key_of(&kv, k.loc)?, (kv, vv));
                }
                Value::Dict(Rc::new(d))
            }
            TK::Default => default_value(&e.ty),
            TK::Range { start, end, inclusive, step } => {
                let plan = self.range_plan(start, end, *inclusive, step.as_deref(), e.loc, f)?;
                range_list(plan, e.loc)?
            }
            TK::Neg(x) => {
                let v = self.eval(x, f)?;
                self.neg_value(v, &e.ty, e.loc)?
            }
            TK::Concat(l, r) => {
                let a = self.eval(l, f)?;
                let b = self.eval(r, f)?;
                let (Value::Str(a), Value::Str(b)) = (a, b) else { unreachable!() };
                let mut s = String::with_capacity(a.len() + b.len());
                s.push_str(&a);
                s.push_str(&b);
                Value::string(s)
            }
            TK::Olarak(x) => {
                let v = self.eval(x, f)?;
                self.olarak(v, &x.ty, &e.ty, e.loc)?
            }
            TK::Method { recv, view, name, mid, args } => {
                let mut vals = self.pool.pop().unwrap_or_default();
                for a in args {
                    vals.push(self.eval(a, f)?);
                }
                match recv {
                    Recv::Value(r) => {
                        let bu = self.eval(r, f)?;
                        let fid = self.dispatch(&bu, *view, *mid, name, e.loc)?;
                        self.call(fid, vals, BuRef::Val(bu), e.loc)?.0
                    }
                    Recv::Place(p) => self.call_on_place(p, None, (*view, *mid, name), vals, f, e.loc)?,
                }
            }
            TK::Direct { func, recv, args } => {
                let mut vals = self.pool.pop().unwrap_or_default();
                for a in args {
                    vals.push(self.eval(a, f)?);
                }
                match recv {
                    Recv::Value(r) => {
                        let bu = self.eval(r, f)?;
                        self.call(*func, vals, BuRef::Val(bu), e.loc)?.0
                    }
                    Recv::Place(p) => self.call_on_place(p, Some(*func), (0, 0, ""), vals, f, e.loc)?,
                }
            }
            TK::Builtin(b, args) => match b {
                Builtin::Yazdir => {
                    let mut line = String::new();
                    for (i, a) in args.iter().enumerate() {
                        let v = self.eval(a, f)?;
                        if i > 0 {
                            line.push(' ');
                        }
                        write_value(&mut line, &v, Some(&a.ty), prog, true);
                    }
                    line.push('\n');
                    self.write_line(&line);
                    Value::Hic
                }
                Builtin::Oku => {
                    if let Some(a) = args.first() {
                        let Value::Str(p) = self.eval(a, f)? else { unreachable!() };
                        let _ = write!(self.out, "{p}");
                    }
                    let _ = self.out.flush();
                    let mut line = String::new();
                    let _ = std::io::stdin().lock().read_line(&mut line);
                    while line.ends_with('\n') || line.ends_with('\r') {
                        line.pop();
                    }
                    Value::string(line)
                }
            },
            TK::Lambda { func, caps, bu } => {
                let mut cs = Vec::with_capacity(caps.len());
                for c in caps {
                    cs.push(self.eval(c, f)?);
                }
                let bu = match bu {
                    Some(b) => Some(self.eval(b, f)?),
                    None => None,
                };
                Value::Func(Rc::new(FuncV { fid: *func, caps: cs, bu }))
            }
            TK::FuncRef(fid) => Value::Func(Rc::new(FuncV { fid: *fid, caps: vec![], bu: None })),
            TK::CallValue(fe, args) => {
                let Value::Func(fv) = self.eval(fe, f)? else { unreachable!() };
                let mut vals = self.pool.pop().unwrap_or_default();
                for a in args {
                    vals.push(self.eval(a, f)?);
                }
                self.call_value(&fv, vals, e.loc)?
            }
            TK::BMethod { recv, m, args } if is_hof(*m) || matches!(m, BM::Toplam | BM::EnBuyuk | BM::EnKucuk) => {
                self.list_hof(recv, *m, args, &e.ty, e.loc, f)?
            }
            TK::BMethod { recv, m, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval(a, f)?);
                }
                match recv {
                    Recv::Place(p) => {
                        let rp = self.resolve_place(p, f)?;
                        let root = Self::root_mut(&mut self.globals, f, rp.0, e.loc)?;
                        let target = walk_mut(root, &rp.1, false, e.loc, prog)?;
                        let r = bm_mut(*m, target, vals, e.loc, prog)?;
                        self.recycle(rp.1);
                        r
                    }
                    Recv::Value(r) => {
                        let mut v = self.eval(r, f)?;
                        if is_mutating(*m) {
                            bm_mut(*m, &mut v, vals, e.loc, prog)?
                        } else {
                            bm_val(*m, v, vals, &r.ty, e.loc, prog)?
                        }
                    }
                }
            }
            TK::New(tid, args) => {
                let mut fs = Vec::with_capacity(args.len());
                for a in args {
                    fs.push(self.eval(a, f)?);
                }
                Value::Obj(Rc::new(ObjV { ty: *tid, fields: fs }))
            }
            TK::Index(x, i) => {
                let v = self.eval(x, f)?;
                let iv = self.eval(i, f)?;
                match v {
                    Value::List(l) => {
                        let Value::Int(ix) = iv else { unreachable!() };
                        let Some(ix) = ix.as_i64() else { return rtk(hata_turu::INDEKS, e.loc, "İndeks çok büyük.") };
                        let real = norm_index(ix, l.items.len(), e.loc)?;
                        l.items[real].clone()
                    }
                    Value::Dict(d) => match d.map.get(&key_of(&iv, e.loc)?) {
                        Some((_, v)) => v.clone(),
                        None => return Err(missing_key(prog, &iv, e.loc)),
                    },
                    _ => unreachable!(),
                }
            }
            TK::Slice { base, start, end, inclusive } => {
                let v = self.eval(base, f)?;
                let s = match start {
                    Some(x) => Some(self.eval_i64(x, f)?),
                    None => None,
                };
                let en = match end {
                    Some(x) => Some(self.eval_i64(x, f)?),
                    None => None,
                };
                match v {
                    Value::List(l) => {
                        let b = slice_bounds(l.items.len(), s, en, *inclusive);
                        Value::List(Rc::new(ListV { items: apply_slice(&l.items, b) }))
                    }
                    Value::Str(t) => {
                        let cs: Vec<char> = t.chars().collect();
                        let b = slice_bounds(cs.len(), s, en, *inclusive);
                        Value::string(apply_slice(&cs, b).into_iter().collect())
                    }
                    _ => unreachable!(),
                }
            }
            TK::Unwrap(x) => {
                // `dene` içinde: dönen değer Hata ise fırlatılır
                let v = self.eval(x, f)?;
                if let Value::Obj(o) = &v {
                    if prog.is_subtype(o.ty, prog.hata_turleri[0]) {
                        let ty = x.ty.members().into_iter().find(|m| value_matches(&v, m, prog)).unwrap_or(Type::Named(o.ty));
                        return Err(Unwind::Err(self.firlat(v, &ty, e.loc)));
                    }
                }
                match (&v, &e.ty) {
                    (Value::Int(n), Type::Float(b)) => Value::Float(round_float(n.to_f64(), *b)),
                    _ => v,
                }
            }
            TK::Match(m) => match self.choose_arm(m, f)? {
                Some(i) => match &m.arms[i].body {
                    TArmBody::Expr(x) => self.eval(x, f)?,
                    TArmBody::Block(b) => match self.exec_block(b, f)? {
                        Ctl::Normal => Value::Hic,
                        c => return Err(Unwind::Ctl(c)),
                    },
                },
                None => Value::Hic,
            },
            _ => unreachable!(),
        })
    }

    /// Bir yerdeki (değişken, alan, liste elemanı...) nesnenin metodunu çağırır.
    /// Nesne bir dosya değişkenindeyse yerinde çalışılır; değilse çağrı süresince alınıp geri yazılır.
    pub(crate) fn call_on_place(
        &mut self,
        p: &TPlace,
        direct: Option<FuncId>,
        (view, mid, name): (TypeId, u32, &str),
        vals: Vec<Value>,
        f: &mut Frame,
        loc: Loc,
    ) -> R<Value> {
        let rp = self.resolve_place(p, f)?;
        if let Root::Global(g) = rp.0 {
            let fid = match direct {
                Some(fid) => fid,
                None => {
                    let root = &self.globals[g as usize];
                    if matches!(root, Value::Uninit) {
                        return rtk(hata_turu::YAKALANAMAZ, loc, BELIRSIZ);
                    }
                    let bu = walk_ref(root, &rp.1, loc, self.prog)?;
                    self.dispatch(bu, view, mid, name, loc)?
                }
            };
            let (_, steps) = rp;
            let (ret, bu) = self.call(fid, vals, BuRef::Alias(g, steps), loc)?;
            if let BuRef::Alias(_, steps) = bu {
                self.recycle(steps);
            }
            return Ok(ret);
        }
        let bu = self.take_resolved(&rp, f, loc)?;
        let fid = match direct {
            Some(fid) => fid,
            None => match self.dispatch(&bu, view, mid, name, loc) {
                Ok(x) => x,
                Err(err) => {
                    self.write_resolved(&rp, bu, f, loc)?;
                    return Err(err);
                }
            },
        };
        match self.call(fid, vals, BuRef::Val(bu), loc) {
            Ok((ret, BuRef::Val(bu))) => {
                self.write_resolved(&rp, bu, f, loc)?;
                self.recycle(rp.1);
                Ok(ret)
            }
            Ok((ret, BuRef::Alias(..))) => Ok(ret),
            Err(e) => Err(e),
        }
    }

    /// Aralığın sınırlarını (başlangıç, bitiş, adım sırasıyla) hesaplar.
    pub(crate) fn range_plan(&mut self, start: &TExpr, end: &TExpr, inclusive: bool, step: Option<&TExpr>, loc: Loc, f: &mut Frame) -> R<RangePlan> {
        let s = self.eval_i64(start, f)?;
        let e = self.eval_i64(end, f)?;
        let n = match step {
            Some(x) => self.eval_i64(x, f)?,
            None => 1,
        };
        match range_plan(s, e, inclusive, n) {
            Some(p) => Ok(p),
            None => rt(loc, "Adım sıfır olamaz."),
        }
    }

    pub(crate) fn eval_i64(&mut self, e: &TExpr, f: &mut Frame) -> R<i64> {
        match self.eval(e, f)? {
            Value::Int(i) => i.as_i64().map_or_else(|| rt(e.loc, "Sayı çok büyük."), Ok),
            _ => unreachable!(),
        }
    }

    pub(crate) fn tn(&self, t: &Type) -> String {
        type_display(t, self.prog)
    }

    pub(crate) fn dispatch(&self, bu: &Value, view: TypeId, mid: u32, name: &str, loc: Loc) -> R<FuncId> {
        let dyn_ty = match bu {
            Value::Obj(o) => o.ty,
            Value::Var(v) => v.ty,
            _ => return rt(loc, format!("'{name}' çağrılamadı.")),
        };
        match self.prog.dispatch.get(&(dyn_ty, view, mid)) {
            Some(f) => Ok(*f),
            None => rt(loc, format!("'{}' içinde '{name}' bulunamadı.", self.prog.types[dyn_ty].name)),
        }
    }

    pub(crate) fn call(&mut self, fid: FuncId, args: Vec<Value>, bu: BuRef, loc: Loc) -> R<(Value, BuRef)> {
        if self.depth >= MAX_DEPTH {
            return rtk(hata_turu::YAKALANAMAZ, loc, "Çok derin özyineleme: bir fonksiyon kendini çok fazla çağırdı.");
        }
        let func = &self.prog.funcs[fid];
        if func.dis.is_some() {
            // C fonksiyonu da ekrana yazabilir: önce Kolay'ın tamponu boşaltılır (sıra korunur)
            use std::io::Write as _;
            let _ = self.out.flush();
            let r = crate::dis::call(self.dis[fid], &func.params, &func.ret, &args, loc);
            let mut a = args;
            a.clear();
            self.pool.push(a);
            return Ok((r?, bu));
        }
        // Bytecode değişmez ve Interp yaşadıkça yaşar; çağrı süresince ödünç alınması güvenli.
        let ch: *const crate::vm::Chunk<'p> = &self.chunks[fid];
        let ch = unsafe { &*ch };
        let mut slots = args;
        fill_slots(&mut slots, ch.nregs);
        let mut frame = Frame { slots, bu };
        self.depth += 1;
        let r = self.run_chunk(ch, &mut frame);
        self.depth -= 1;
        let Frame { mut slots, bu } = frame;
        slots.clear();
        self.pool.push(slots);
        Ok((r?, bu))
    }

    /// Fonksiyon değerini çağırır (lambda ise yakaladığı değerler kendi yuvalarına yazılır).
    pub(crate) fn call_value(&mut self, fv: &FuncV, args: Vec<Value>, loc: Loc) -> R<Value> {
        let Some(caps) = &self.prog.funcs[fv.fid].caps else {
            return self.call_plain(fv.fid, args, loc);
        };
        if self.depth >= MAX_DEPTH {
            return rtk(hata_turu::YAKALANAMAZ, loc, "Çok derin özyineleme: bir fonksiyon kendini çok fazla çağırdı.");
        }
        let ch: *const crate::vm::Chunk<'p> = &self.chunks[fv.fid];
        let ch = unsafe { &*ch };
        let mut slots = args;
        fill_slots(&mut slots, ch.nregs);
        for ((slot, _), v) in caps.iter().zip(&fv.caps) {
            slots[*slot as usize] = v.clone();
        }
        let bu = BuRef::Val(fv.bu.clone().unwrap_or(Value::Hic));
        let mut frame = Frame { slots, bu };
        self.depth += 1;
        let r = self.run_chunk(ch, &mut frame);
        self.depth -= 1;
        let mut slots = frame.slots;
        slots.clear();
        self.pool.push(slots);
        r
    }

    /// Fonksiyon alan liste metotları ve toplam/en_büyük/en_küçük
    #[allow(clippy::too_many_arguments)]
    fn list_hof(&mut self, recv: &Recv, m: BM, args: &[TExpr], ty: &Type, loc: Loc, f: &mut Frame) -> R<Value> {
        let fv = match args.first() {
            Some(a) => match self.eval(a, f)? {
                Value::Func(fv) => Some(fv),
                _ => unreachable!(),
            },
            None => None,
        };
        let list = match recv {
            Recv::Value(x) => self.eval(x, f)?,
            Recv::Place(p) => {
                let rp = self.resolve_place(p, f)?;
                let prog = self.prog;
                let root = Self::root_mut(&mut self.globals, f, rp.0, loc)?;
                let v = walk_ref(root, &rp.1, loc, prog)?.clone();
                self.recycle(rp.1);
                v
            }
        };
        let Value::List(l) = list else { unreachable!() };
        let call = |me: &mut Self, x: &Value| -> R<Value> {
            let mut a = me.pool.pop().unwrap_or_default();
            a.push(x.clone());
            me.call_value(fv.as_ref().unwrap(), a, loc)
        };
        let yeni = |items: Vec<Value>| Value::List(Rc::new(ListV { items }));
        Ok(match m {
            BM::Donusum => {
                let mut out = Vec::with_capacity(l.items.len());
                for x in &l.items {
                    out.push(call(self, x)?);
                }
                yeni(out)
            }
            BM::Filtre => {
                let mut out = Vec::new();
                for x in &l.items {
                    if let Value::Bool(true) = call(self, x)? {
                        out.push(x.clone());
                    }
                }
                yeni(out)
            }
            BM::Hepsi | BM::Herhangi => {
                let aranan = m == BM::Herhangi;
                for x in &l.items {
                    if let Value::Bool(b) = call(self, x)? {
                        if b == aranan {
                            return Ok(Value::Bool(aranan));
                        }
                    }
                }
                Value::Bool(!aranan)
            }
            BM::SiralaGore | BM::SiraliGore => {
                let mut keys = Vec::with_capacity(l.items.len());
                for x in &l.items {
                    keys.push(call(self, x)?);
                }
                // anahtara göre kararlı sıralama (eşit anahtarlarda eski sıra korunur)
                let mut idx: Vec<usize> = (0..l.items.len()).collect();
                if keys.iter().all(|k| matches!(k, Value::Str(_))) {
                    let sk: Vec<_> = keys.iter().map(|k| if let Value::Str(s) = k { tr_sort_key(s) } else { unreachable!() }).collect();
                    idx.sort_by(|a, b| sk[*a].cmp(&sk[*b]));
                } else {
                    idx.sort_by(|a, b| cmp_values(&keys[*a], &keys[*b]));
                }
                let sirali = yeni(idx.iter().map(|&i| l.items[i].clone()).collect());
                match (m, recv) {
                    (BM::SiralaGore, Recv::Place(p)) => {
                        self.assign_place(p, sirali, f, loc)?;
                        Value::Hic
                    }
                    (BM::SiralaGore, _) => Value::Hic,
                    _ => sirali,
                }
            }
            BM::Toplam => {
                let mut acc = match ty {
                    Type::Float(_) => Value::Float(0.0),
                    _ => Value::int(0),
                };
                for x in &l.items {
                    acc = self.arith(BinOp::Add, acc, x.clone(), ty, loc)?;
                }
                acc
            }
            _ => {
                let buyuk = m == BM::EnBuyuk;
                let Some(mut best) = l.items.first().cloned() else {
                    return rtk(hata_turu::INDEKS, loc, if buyuk { "Liste boş; en büyük eleman yok." } else { "Liste boş; en küçük eleman yok." });
                };
                for x in &l.items[1..] {
                    let o = cmp_values(x, &best);
                    if (buyuk && o == Ordering::Greater) || (!buyuk && o == Ordering::Less) {
                        best = x.clone();
                    }
                }
                best
            }
        })
    }

    /// Metot olmayan fonksiyon çağrısı (en sık durum): `call` ile aynı, ama nesne (`bu`) taşımadığı
    /// için dönüş değeri küçük kalır.
    pub(crate) fn call_plain(&mut self, fid: FuncId, args: Vec<Value>, loc: Loc) -> R<Value> {
        if self.depth >= MAX_DEPTH || self.prog.funcs[fid].dis.is_some() {
            return Ok(self.call(fid, args, BuRef::Val(Value::Hic), loc)?.0);
        }
        let ch: *const crate::vm::Chunk<'p> = &self.chunks[fid];
        let ch = unsafe { &*ch };
        let mut slots = args;
        fill_slots(&mut slots, ch.nregs);
        let mut frame = Frame { slots, bu: BuRef::Val(Value::Hic) };
        self.depth += 1;
        let r = self.run_chunk(ch, &mut frame);
        self.depth -= 1;
        let mut slots = frame.slots;
        slots.clear();
        self.pool.push(slots);
        r
    }

    pub(crate) fn neg_value(&self, v: Value, ty: &Type, loc: Loc) -> R<Value> {
        Ok(match (&v, ty) {
            (Value::Int(i), Type::Int { bits, sign }) => {
                let r = match i {
                    Int::S(a) => a.checked_neg().map(Int::S).unwrap_or_else(|| Int::from_big(-BigInt::from(*a))),
                    Int::B(b) => Int::from_big(-(**b).clone()),
                };
                if !fits_int(&r, *bits, *sign) {
                    return rtk(hata_turu::TASMA, loc, format!("{} taştı: -({}) = {}, ama {} sınırları {}.", self.tn(ty), group_digits(&i.to_string()), group_digits(&r.to_string()), self.tn(ty), bounds_text(*bits, *sign)));
                }
                Value::Int(r)
            }
            (Value::Float(x), _) => Value::Float(-x),
            _ => unreachable!(),
        })
    }

    pub(crate) fn arith(&self, op: BinOp, a: Value, b: Value, ty: &Type, loc: Loc) -> R<Value> {
        match ty {
            Type::Int { bits, sign } => {
                let (Value::Int(x), Value::Int(y)) = (a, b) else { unreachable!() };
                if matches!(op, BinOp::Div | BinOp::Rem) && y.is_zero() {
                    return rtk(hata_turu::SIFIRA_BOLME, loc, "Sıfıra bölme hatası.");
                }
                if matches!(op, BinOp::Shl | BinOp::Shr | BinOp::Pow) {
                    return self.shift_pow(op, &x, &y, *bits, *sign, ty, loc).map(Value::Int);
                }
                let r = match (&x, &y) {
                    (Int::S(p), Int::S(q)) => {
                        let fast = match op {
                            BinOp::Add => p.checked_add(*q),
                            BinOp::Sub => p.checked_sub(*q),
                            BinOp::Mul => p.checked_mul(*q),
                            BinOp::Div => p.checked_div(*q),
                            BinOp::Rem => p.checked_rem(*q),
                            BinOp::BitAnd => Some(p & q),
                            BinOp::BitOr => Some(p | q),
                            BinOp::BitXor => Some(p ^ q),
                            _ => unreachable!(),
                        };
                        match fast {
                            Some(v) => Int::S(v),
                            None => Int::from_i128(i128_op(op, *p as i128, *q as i128)),
                        }
                    }
                    _ => Int::from_big(big_op(op, &x.to_big(), &y.to_big())),
                };
                if !fits_int(&r, *bits, *sign) {
                    return rtk(hata_turu::TASMA, loc, self.overflow_msg(op, &x, &y, &r, *bits, *sign, ty));
                }
                Ok(Value::Int(r))
            }
            Type::Float(bits) => {
                let (p, q) = (a.as_f64(), b.as_f64());
                let r = match op {
                    BinOp::Add => p + q,
                    BinOp::Sub => p - q,
                    BinOp::Mul => p * q,
                    // Sıfıra bölme tanımsızdır: sonuç TanımsızSayı (sonsuz değil).
                    BinOp::Div if q == 0.0 => f64::NAN,
                    BinOp::Div => p / q,
                    BinOp::Rem => p % q,
                    BinOp::Pow => p.powf(q),
                    _ => unreachable!(),
                };
                Ok(Value::Float(round_float(r, *bits)))
            }
            _ => unreachable!("sayı bekleniyordu"),
        }
    }

    /// `<<`, `>>` ve `**` (tam sayılarda). Derlenmiş programdakiyle (codegen/ifade.rs) aynı kurallar ve mesajlar.
    #[allow(clippy::too_many_arguments)]
    fn shift_pow(&self, op: BinOp, x: &Int, y: &Int, bits: Option<u32>, sign: Sign, ty: &Type, loc: Loc) -> R<Int> {
        if y.cmp_int(&Int::S(0)) == Ordering::Less {
            return rt(
                loc,
                match op {
                    BinOp::Pow => format!("Tam sayının üssü negatif olamaz ({y}). Ondalık sonuç için: 2.0 ** -1"),
                    _ => format!("Kaydırma miktarı negatif olamaz ({y})."),
                },
            );
        }
        let too_big = || rtk(hata_turu::TASMA, loc, self.big_result_msg(op, x, y, ty, bits, sign));
        // boyutlu tiplerde sonuç en fazla 129 bit: büyük üs/kaydırmada sonucu hesaplamadan karar verilir
        let n: u64 = match y.as_i64() {
            Some(n) => n as u64,
            None => u64::MAX,
        };
        let r = match op {
            BinOp::Shr => {
                let n = n.min(1 << 20) as usize;
                Int::from_big(x.to_big() >> n)
            }
            BinOp::Shl => {
                if x.is_zero() {
                    Int::S(0)
                } else if bits.is_some() && n > 130 {
                    return too_big();
                } else if n > u32::MAX as u64 {
                    return rtk(hata_turu::TASMA, loc, format!("Kaydırma miktarı çok büyük ({y})."));
                } else {
                    Int::from_big(x.to_big() << (n as usize))
                }
            }
            _ => {
                // |x| <= 1 ise sonuç her üste küçük kalır
                let xb = x.to_big();
                let small = xb >= BigInt::from(-1) && xb <= BigInt::from(1);
                if small {
                    let n = (n % 2) as u32 + if n == 0 { 0 } else { 2 };
                    Int::from_big(xb.pow(n))
                } else if bits.is_some() && n > 130 {
                    return too_big();
                } else if n > u32::MAX as u64 {
                    return rtk(hata_turu::TASMA, loc, format!("Üs çok büyük ({y})."));
                } else {
                    Int::from_big(xb.pow(n as u32))
                }
            }
        };
        if !fits_int(&r, bits, sign) {
            return if op == BinOp::Shr { rtk(hata_turu::TASMA, loc, self.overflow_msg(op, x, y, &r, bits, sign, ty)) } else { too_big() };
        }
        Ok(r)
    }

    /// Üs ve sola kaydırmada taşma: sonuç çok büyük olabileceği için yazılmaz.
    fn big_result_msg(&self, op: BinOp, x: &Int, y: &Int, ty: &Type, bits: Option<u32>, sign: Sign) -> String {
        let t = self.tn(ty);
        format!(
            "{t} taştı: {} {} {} sonucu {t} sınırlarını aşıyor ({}).",
            group_digits(&x.to_string()),
            op.symbol(),
            group_digits(&y.to_string()),
            bounds_text(bits, sign)
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn overflow_msg(&self, op: BinOp, x: &Int, y: &Int, r: &Int, bits: Option<u32>, sign: Sign, ty: &Type) -> String {
        let t = self.tn(ty);
        let (lo, hi) = int_bounds(bits, sign);
        let g = |v: &Int| group_digits(&v.to_string());
        let expr = format!("{} {} {} = {}", g(x), op.symbol(), g(y), g(r));
        let rb = r.to_big();
        if hi.as_ref().is_some_and(|h| rb > *h) {
            format!("{t} taştı. {expr}, ama {t} en fazla {} tutabilir.", group_digits(&hi.unwrap().to_string()))
        } else if sign == Sign::Pos {
            format!("{t} negatif olamaz: {expr}.")
        } else {
            format!("{t} taştı. {expr}, ama {t} en az {} tutabilir.", group_digits(&lo.unwrap().to_string()))
        }
    }

    pub(crate) fn olarak(&self, v: Value, from: &Type, to: &Type, loc: Loc) -> R<Value> {
        let prog = self.prog;
        match (&v, to) {
            (Value::Int(i), Type::Int { bits, sign }) => {
                if !fits_int(i, *bits, *sign) {
                    return rtk(hata_turu::DONUSUM, loc, format!("{} değeri {} tipine sığmıyor ({}).", group_digits(&i.to_string()), self.tn(to), bounds_text(*bits, *sign)));
                }
                Ok(v)
            }
            (Value::Float(x), Type::Int { bits, sign }) => {
                if !x.is_finite() {
                    return rtk(hata_turu::DONUSUM, loc, format!("{} tam sayıya çevrilemez.", fmt_float(*x, 64)));
                }
                let t = BigInt::from_f64(x.trunc()).unwrap_or_default();
                let i = Int::from_big(t);
                if !fits_int(&i, *bits, *sign) {
                    return rtk(hata_turu::DONUSUM, loc, format!("{} değeri {} tipine sığmıyor ({}).", fmt_float(*x, 64), self.tn(to), bounds_text(*bits, *sign)));
                }
                Ok(Value::Int(i))
            }
            (Value::Int(i), Type::Float(b)) => {
                let r = round_float(i.to_f64(), *b);
                if r.is_infinite() {
                    return rtk(hata_turu::DONUSUM, loc, format!("{} değeri {} tipine sığmıyor ({}).", group_digits(&i.to_string()), self.tn(to), crate::types::float_limit_text(*b)));
                }
                Ok(Value::Float(r))
            }
            (Value::Float(x), Type::Float(b)) => {
                let r = round_float(*x, *b);
                if r.is_infinite() && x.is_finite() {
                    return rtk(hata_turu::DONUSUM, loc, format!("{} değeri {} tipine sığmıyor ({}).", fmt_float(*x, 64), self.tn(to), crate::types::float_limit_text(*b)));
                }
                Ok(Value::Float(r))
            }
            _ => {
                if value_matches(&v, to, prog) {
                    let _ = from;
                    Ok(v)
                } else {
                    rtk(hata_turu::DONUSUM, loc, format!("Bu değer {} değil, {}.", self.tn(to), kind_name(&v, prog)))
                }
            }
        }
    }

    // ------------------------------------------------------------ eşleştir

    pub(crate) fn choose_arm(&mut self, m: &TMatch, f: &mut Frame) -> R<Option<usize>> {
        let v = self.eval(&m.scrut, f)?;
        for (i, arm) in m.arms.iter().enumerate() {
            let ok = match &arm.pats {
                None => true,
                Some(ps) => {
                    let mut any = false;
                    for p in ps {
                        if self.pat_match(p, &v, f) {
                            any = true;
                            break;
                        }
                    }
                    any
                }
            };
            if !ok {
                continue;
            }
            if let Some(g) = &arm.guard {
                if !self.truthy(g, f)? {
                    continue;
                }
            }
            return Ok(Some(i));
        }
        Ok(None)
    }

    pub(crate) fn pat_match(&mut self, p: &TPat, v: &Value, f: &mut Frame) -> bool {
        match (p, v) {
            (TPat::Wild, _) => true,
            (TPat::Int(n), Value::Int(i)) => i.eq_big(n),
            (TPat::Float(x), Value::Float(y)) => x == y,
            (TPat::Str(s), Value::Str(t)) => **s == ***t,
            (TPat::Bool(a), Value::Bool(b)) => a == b,
            (TPat::Hic, Value::Hic) => true,
            (TPat::Range(a, b, incl), Value::Int(i)) => {
                i.cmp_big(a) != Ordering::Less
                    && match i.cmp_big(b) {
                        Ordering::Less => true,
                        Ordering::Equal => *incl,
                        Ordering::Greater => false,
                    }
            }
            (TPat::Variant { ty, idx, binds }, Value::Var(x)) => {
                if x.ty != *ty || x.idx != *idx {
                    return false;
                }
                for (b, fv) in binds.iter().zip(&x.fields) {
                    if let Some(s) = b {
                        self.set_slot(*s, fv.clone(), f);
                    }
                }
                true
            }
            (TPat::TypeTest { ty, bind }, _) => {
                if !value_matches(v, ty, self.prog) {
                    return false;
                }
                if let Some(s) = bind {
                    self.set_slot(*s, v.clone(), f);
                }
                true
            }
            _ => false,
        }
    }
}

/// Çerçevenin yuvalarını (parametrelerden sonrası) belirsiz değerle doldurur. `resize` her yuva için
/// değeri kopyalar (clone); burada doğrudan yazılır.
#[inline(always)]
fn fill_slots(slots: &mut Vec<Value>, n: u32) {
    let n = n as usize;
    if slots.len() < n {
        slots.extend(std::iter::repeat_with(|| Value::Uninit).take(n - slots.len()));
    }
}

pub(crate) fn i128_op(op: BinOp, a: i128, b: i128) -> i128 {
    // İki i64'ün sonucu i128'e her zaman sığar.
    match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => a / b,
        BinOp::Rem => a % b,
        BinOp::BitAnd => a & b,
        BinOp::BitOr => a | b,
        BinOp::BitXor => a ^ b,
        _ => unreachable!(),
    }
}

pub(crate) fn big_op(op: BinOp, a: &BigInt, b: &BigInt) -> BigInt {
    match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => a / b,
        BinOp::Rem => a % b,
        BinOp::BitAnd => a & b,
        BinOp::BitOr => a | b,
        BinOp::BitXor => a ^ b,
        _ => unreachable!(),
    }
}

pub(crate) fn compare(op: BinOp, a: &Value, b: &Value) -> bool {
    match op {
        BinOp::Eq => values_eq(a, b),
        BinOp::Ne => !values_eq(a, b),
        _ => {
            if let (Value::Float(x), Value::Float(y)) = (a, b) {
                return match op {
                    BinOp::Lt => x < y,
                    BinOp::Le => x <= y,
                    BinOp::Gt => x > y,
                    BinOp::Ge => x >= y,
                    _ => unreachable!(),
                };
            }
            let o = match (a, b) {
                (Value::Int(Int::S(x)), Value::Int(Int::S(y))) => x.cmp(y),
                _ => cmp_values(a, b),
            };
            match op {
                BinOp::Lt => o == Ordering::Less,
                BinOp::Le => o != Ordering::Greater,
                BinOp::Gt => o == Ordering::Greater,
                BinOp::Ge => o != Ordering::Less,
                _ => unreachable!(),
            }
        }
    }
}

pub fn value_matches(v: &Value, ty: &Type, prog: &Program) -> bool {
    match (v, ty) {
        (_, Type::Union(ms)) => ms.iter().any(|m| value_matches(v, m, prog)),
        (Value::Int(_), Type::Int { .. }) => true,
        (Value::Float(_), Type::Float(_)) => true,
        (Value::Str(_), Type::Str) => true,
        (Value::Bool(_), Type::Bool) => true,
        (Value::Hic, Type::Hic) => true,
        (Value::List(_), Type::List(..)) => true,
        (Value::Dict(_), Type::Dict(..)) => true,
        (Value::Obj(o), Type::Named(t)) => prog.is_subtype(o.ty, *t),
        (Value::Var(x), Type::Named(t)) => prog.is_subtype(x.ty, *t),
        (Value::Func(_), Type::Func(..)) => true,
        _ => false,
    }
}

pub fn default_value(t: &Type) -> Value {
    match t {
        Type::Int { .. } => Value::int(0),
        Type::Float(_) => Value::Float(0.0),
        Type::Str => Value::str(""),
        Type::Bool => Value::Bool(false),
        Type::List(e, Some(n)) => Value::List(Rc::new(ListV { items: vec![default_value(e); *n] })),
        Type::List(_, None) => Value::List(Rc::new(ListV { items: vec![] })),
        Type::Dict(..) => Value::Dict(Rc::new(DictV::default())),
        Type::Union(ms) => {
            if ms.contains(&Type::Hic) {
                Value::Hic
            } else {
                default_value(&ms[0])
            }
        }
        Type::Named(id) => Value::Var(Rc::new(VarV { ty: *id, idx: 0, fields: vec![] })),
        _ => Value::Hic,
    }
}

pub(crate) fn is_mutating(m: BM) -> bool {
    matches!(m, BM::Ekle | BM::ArayaEkle | BM::Sil | BM::Cikar | BM::Sirala | BM::TersCevir | BM::Temizle | BM::DSil | BM::DTemizle)
}

pub(crate) fn arg_i64(v: &Value, loc: Loc) -> R<i64> {
    match v {
        Value::Int(i) => i.as_i64().map_or_else(|| rt(loc, "Sayı çok büyük."), Ok),
        _ => unreachable!(),
    }
}

pub(crate) fn sort_values(items: &mut [Value]) {
    if items.iter().all(|v| matches!(v, Value::Str(_))) {
        // Türkçe sıralama anahtarı her yazı için bir kez hesaplanır.
        items.sort_by_cached_key(|v| match v {
            Value::Str(s) => tr_sort_key(s),
            _ => unreachable!(),
        });
    } else {
        items.sort_by(cmp_values);
    }
}

pub(crate) fn bm_mut(m: BM, target: &mut Value, args: Vec<Value>, loc: Loc, prog: &Program) -> R<Value> {
    match target {
        Value::List(rc) => {
            let l = Rc::make_mut(rc);
            match m {
                BM::Ekle => l.items.push(args.into_iter().next().unwrap()),
                BM::ArayaEkle => {
                    let mut it = args.into_iter();
                    let i = arg_i64(&it.next().unwrap(), loc)?;
                    let len = l.items.len() as i64;
                    let real = if i < 0 { i + len } else { i };
                    if real < 0 || real > len {
                        return rtk(hata_turu::INDEKS, loc, format!("{i}. konuma eklenemez; liste {len} elemanlı (0 .. {len} arası olmalı)."));
                    }
                    l.items.insert(real as usize, it.next().unwrap());
                }
                BM::Sil => {
                    let i = arg_i64(&args[0], loc)?;
                    let real = norm_index(i, l.items.len(), loc)?;
                    l.items.remove(real);
                }
                BM::Cikar => {
                    return match l.items.pop() {
                        Some(v) => Ok(v),
                        None => rtk(hata_turu::INDEKS, loc, "Liste boş, eleman çıkarılamaz."),
                    };
                }
                BM::Sirala => sort_values(&mut l.items),
                BM::TersCevir => l.items.reverse(),
                BM::Temizle => l.items.clear(),
                _ => unreachable!(),
            }
            Ok(Value::Hic)
        }
        Value::Dict(rc) => {
            let d = Rc::make_mut(rc);
            match m {
                BM::DSil => {
                    let k = &args[0];
                    if d.map.shift_remove(&key_of(k, loc)?).is_none() {
                        return Err(missing_key(prog, k, loc));
                    }
                }
                BM::DTemizle => d.map.clear(),
                _ => unreachable!(),
            }
            Ok(Value::Hic)
        }
        _ => unreachable!(),
    }
}

pub(crate) fn bm_val(m: BM, v: Value, args: Vec<Value>, ty: &Type, loc: Loc, prog: &Program) -> R<Value> {
    Ok(match (m, &v) {
        (BM::Yaziya, _) => Value::string(fmt_value(&v, Some(ty), prog, true)),
        (BM::Uzunluk, Value::List(l)) => Value::int(l.items.len() as i64),
        (BM::Icerir, Value::List(l)) => Value::Bool(l.items.iter().any(|x| values_eq(x, &args[0]))),
        (BM::Bul, Value::List(l)) => match l.items.iter().position(|x| values_eq(x, &args[0])) {
            Some(i) => Value::int(i as i64),
            None => Value::Hic,
        },
        (BM::LAdim, Value::List(l)) => {
            let n = arg_i64(&args[0], loc)?;
            if n == 0 {
                return rt(loc, "Adım sıfır olamaz.");
            }
            let m = n.unsigned_abs() as usize;
            let items: Vec<Value> =
                if n > 0 { l.items.iter().step_by(m).cloned().collect() } else { l.items.iter().rev().step_by(m).cloned().collect() };
            Value::List(Rc::new(ListV { items }))
        }
        (BM::DUzunluk, Value::Dict(d)) => Value::int(d.map.len() as i64),
        (BM::DIcerir, Value::Dict(d)) => Value::Bool(d.map.contains_key(&key_of(&args[0], loc)?)),
        (BM::Anahtarlar, Value::Dict(d)) => Value::List(Rc::new(ListV { items: d.map.values().map(|(k, _)| k.clone()).collect() })),
        (BM::Degerler, Value::Dict(d)) => Value::List(Rc::new(ListV { items: d.map.values().map(|(_, v)| v.clone()).collect() })),
        (BM::SUzunluk, Value::Str(s)) => Value::int(s.chars().count() as i64),
        (BM::BuyukHarf, Value::Str(s)) => Value::string(tr_upper(s)),
        (BM::KucukHarf, Value::Str(s)) => Value::string(tr_lower(s)),
        (BM::Karakter, Value::Str(s)) => {
            let i = arg_i64(&args[0], loc)?;
            let n = s.chars().count() as i64;
            let real = if i < 0 { i + n } else { i };
            if real < 0 || real >= n {
                return rtk(hata_turu::INDEKS, loc, format!("Yazının {i}. karakteri yok; yazı {n} karakterli."));
            }
            Value::string(s.chars().nth(real as usize).unwrap().to_string())
        }
        (BM::SIcerir, Value::Str(s)) => {
            let Value::Str(p) = &args[0] else { unreachable!() };
            Value::Bool(s.contains(p.as_str()))
        }
        (BM::Bol, Value::Str(s)) => {
            let Value::Str(sep) = &args[0] else { unreachable!() };
            let items: Vec<Value> = if sep.is_empty() {
                s.chars().map(|c| Value::string(c.to_string())).collect()
            } else {
                s.split(sep.as_str()).map(Value::str).collect()
            };
            Value::List(Rc::new(ListV { items }))
        }
        (BM::Kirp, Value::Str(s)) => Value::str(s.trim()),
        (BM::Degistir, Value::Str(s)) => {
            let (Value::Str(eski), Value::Str(yeni)) = (&args[0], &args[1]) else { unreachable!() };
            if eski.is_empty() {
                return rt(loc, "Değiştirilecek parça boş olamaz.");
            }
            Value::string(s.replace(eski.as_str(), yeni))
        }
        (BM::Baslar, Value::Str(s)) => {
            let Value::Str(p) = &args[0] else { unreachable!() };
            Value::Bool(s.starts_with(p.as_str()))
        }
        (BM::Biter, Value::Str(s)) => {
            let Value::Str(p) = &args[0] else { unreachable!() };
            Value::Bool(s.ends_with(p.as_str()))
        }
        // karakter sırası (bayt değil); yoksa Hiç
        (BM::SBul, Value::Str(s)) => {
            let Value::Str(p) = &args[0] else { unreachable!() };
            match s.find(p.as_str()) {
                Some(b) => Value::int(s[..b].chars().count() as i64),
                None => Value::Hic,
            }
        }
        (BM::Say, Value::Str(s)) => {
            let Value::Str(p) = &args[0] else { unreachable!() };
            if p.is_empty() {
                return rt(loc, "Aranan parça boş olamaz.");
            }
            Value::int(s.matches(p.as_str()).count() as i64)
        }
        (BM::Tekrar, Value::Str(s)) => {
            let n = arg_i64(&args[0], loc)?;
            if n < 0 {
                return rt(loc, format!("Tekrar sayısı eksi olamaz: {n}."));
            }
            if (s.len() as u128) * (n as u128) > YAZI_SINIRI as u128 {
                return rtk(hata_turu::YAKALANAMAZ, loc, "Bellek yetmedi.");
            }
            Value::string(s.repeat(n as usize))
        }
        (BM::Hassas, Value::Float(x)) => {
            let n = arg_i64(&args[0], loc)? as usize;
            Value::string(if x.is_finite() { format!("{x:.n$}") } else { fmt_float(*x, 64) })
        }
        (BM::Grupla, Value::Str(s)) => Value::string(grupla_tr(s)),
        (BM::Hizala, Value::Str(s)) => {
            let (Value::Str(dolgu), kip) = (&args[1], arg_i64(&args[2], loc)?) else { unreachable!() };
            Value::string(hizala(s, arg_i64(&args[0], loc)? as usize, dolgu, kip))
        }
        (BM::Birlesik, Value::List(l)) => {
            let Value::Str(ayrac) = &args[0] else { unreachable!() };
            let parcalar: Vec<&str> = l.items.iter().map(|x| match x {
                Value::Str(y) => y.as_str(),
                _ => unreachable!("yazı listesi"),
            }).collect();
            Value::string(parcalar.join(ayrac.as_str()))
        }
        // başarısızsa Hata.Dönüşüm değeri döner (dene içinde fırlatılır)
        (BM::Tama, Value::Str(s)) => match s.trim().parse::<i64>() {
            Ok(n) => Value::int(n),
            Err(_) => hata_nesnesi(prog, hata_turu::DONUSUM, &format!("\"{s}\" bir tam sayıya çevrilemez.")),
        },
        (BM::Ondaliga, Value::Str(s)) => match parse_float(s) {
            Some(x) => Value::Float(x),
            None => hata_nesnesi(prog, hata_turu::DONUSUM, &format!("\"{s}\" bir ondalık sayıya çevrilemez.")),
        },
        (BM::FTama, Value::Float(x)) => {
            if !x.is_finite() || x.trunc() >= 9_223_372_036_854_775_808.0 || x.trunc() < -9_223_372_036_854_775_808.0 {
                return rtk(hata_turu::DONUSUM, loc, format!("{} değeri Tam(64) tipine sığmıyor ({}).", fmt_float(*x, 64), bounds_text(Some(64), Sign::Signed)));
            }
            Value::int(x.trunc() as i64)
        }
        (BM::Sirali, Value::List(l)) => {
            let mut items = l.items.clone();
            sort_values(&mut items);
            Value::List(Rc::new(ListV { items }))
        }
        (BM::Tersi, Value::List(l)) => Value::List(Rc::new(ListV { items: l.items.iter().rev().cloned().collect() })),
        _ => unreachable!("yerleşik metot {m:?}"),
    })
}

/// Aralık listesi değer olarak kullanılınca oluşturulur (döngüde oluşturulmaz).
/// Mesajlar derlenmiş programdakiyle (runtime/koleksiyon.c kolay_aralik_liste) aynı.
pub(crate) fn range_list(plan: RangePlan, loc: Loc) -> R<Value> {
    if plan.count > MAX_ARALIK {
        return rt(loc, aralik_buyuk_mesaj(plan.count));
    }
    let mut items = Vec::new();
    if items.try_reserve_exact(plan.count as usize).is_err() {
        return rtk(hata_turu::YAKALANAMAZ, loc, "Bellek yetmedi.");
    }
    items.extend((0..plan.count).map(|i| Value::int(plan.get(i))));
    Ok(Value::List(Rc::new(ListV { items })))
}

/// f-yazıda `{n:,}`: sayı yazısının tam kısmı noktayla gruplanır, ondalık ayırıcı virgül olur (Türk usulü):
/// "-1234567.5" -> "-1.234.567,5". Sayı olmayan yazılar ("TanımsızSayı") değişmez.
/// Derlenmiş programda: runtime/yazilar.c kolay_grupla
pub(crate) fn grupla_tr(s: &str) -> String {
    let (isaret, govde) = match s.strip_prefix('-') {
        Some(g) => ("-", g),
        None => ("", s),
    };
    let (tam, kesir) = match govde.split_once('.') {
        Some((t, k)) => (t, Some(k)),
        None => (govde, None),
    };
    if tam.is_empty() || !tam.bytes().all(|c| c.is_ascii_digit()) {
        return s.to_string();
    }
    let mut out = String::from(isaret);
    for (i, c) in tam.chars().enumerate() {
        if i > 0 && (tam.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    if let Some(k) = kesir {
        out.push(',');
        out.push_str(k);
    }
    out
}

/// f-yazıda genişlik: kip 0 sola, 1 sağa, 2 ortaya (artan sağa), 3 sıfırla (eksi işaretinden sonra).
/// Genişlik karakter sayısıdır. Derlenmiş programda: runtime/yazilar.c kolay_hizala
pub(crate) fn hizala(s: &str, genislik: usize, dolgu: &str, kip: i64) -> String {
    let n = s.chars().count();
    if n >= genislik {
        return s.to_string();
    }
    let bos = genislik - n;
    match kip {
        0 => format!("{s}{}", dolgu.repeat(bos)),
        1 => format!("{}{s}", dolgu.repeat(bos)),
        2 => format!("{}{s}{}", dolgu.repeat(bos / 2), dolgu.repeat(bos - bos / 2)),
        _ => match s.strip_prefix('-') {
            Some(r) => format!("-{}{r}", "0".repeat(bos)),
            None => format!("{}{s}", "0".repeat(bos)),
        },
    }
}

/// tekrar() ile oluşturulabilecek en uzun yazı (bayt); derlenmiş programda da aynı (runtime/yazilar.c)
pub(crate) const YAZI_SINIRI: u64 = 1 << 32;

/// Değer olarak oluşturulabilecek en büyük aralık listesi (2^30 eleman, 8 GB)
pub(crate) const MAX_ARALIK: u64 = 1 << 30;

pub(crate) fn aralik_buyuk_mesaj(n: u64) -> String {
    format!(
        "Aralık çok büyük ({} eleman), liste olarak oluşturulamaz. Döngüde doğrudan kullanın: a..b içindeki her i için yap {{ }}",
        group_digits(&n.to_string())
    )
}

/// Yazıyı ondalık sayıya çevirir. Virgül de ondalık ayırıcı kabul edilir ("3,5").
pub fn parse_float(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse::<f64>().ok()
}
