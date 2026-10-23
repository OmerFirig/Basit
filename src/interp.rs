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
    pub msg: String,
    pub loc: Loc,
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
    Err(Unwind::Err(RtErr { msg: msg.into(), loc }))
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
    pub(crate) depth: u32,
    pub(crate) pool: Vec<Vec<Value>>,
    /// Fonksiyonların ve dosyaların bytecode'u (vm.rs)
    pub(crate) chunks: Vec<crate::vm::Chunk<'p>>,
    pub(crate) mchunks: Vec<crate::vm::Chunk<'p>>,
    /// dış (C) fonksiyonların adresleri (fonksiyon numarasına göre)
    pub(crate) dis: Vec<usize>,
}

pub(crate) const MAX_DEPTH: u32 = 100_000;

pub(crate) fn norm_index(i: i64, len: usize, loc: Loc) -> R<usize> {
    let real = if i < 0 { i.checked_add(len as i64).unwrap_or(-1) } else { i };
    if real < 0 || real >= len as i64 {
        if len == 0 {
            return rt(loc, format!("Listenin {i}. indeksine erişilmek istendi, ama liste boş."));
        }
        return rt(loc, format!("Listenin {i}. indeksine erişilmek istendi, ama liste {len} elemanlı (0 .. {}).", len - 1));
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
    Unwind::Err(RtErr { msg: format!("{} anahtarı sözlükte yok.", fmt_value(kv, None, prog, false)), loc })
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
            depth: 0,
            pool: Vec::new(),
            chunks: prog.funcs.iter().map(|f| crate::vm::compile_func(prog, f)).collect(),
            mchunks: prog.modules.iter().map(|m| crate::vm::compile_module(prog, m)).collect(),
            dis: Vec::new(),
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
                            return rt(*loc, BELIRSIZ);
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
        match self.compound_value(op, cur, rhs, ty, loc) {
            Ok(new) => self.write_resolved(&rp, new, f, loc)?,
            Err((e, keep)) => {
                self.write_resolved(&rp, keep, f, loc)?;
                return Err(e);
            }
        }
        Ok(())
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
                    Value::Range(r) => {
                        let mut i = r.start;
                        loop {
                            if let Some(e) = r.end {
                                if (r.inclusive && i > e) || (!r.inclusive && i >= e) {
                                    break;
                                }
                            }
                            self.set_slot(binds[0], Value::int(i), f);
                            if let Some(c) = self.loop_ctl(body, f)? {
                                return Ok(c);
                            }
                            i = match i.checked_add(r.step) {
                                Some(x) => x,
                                None => break,
                            };
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
            _ => unreachable!(),
        }
        Ok(Ctl::Normal)
    }

    pub(crate) fn truthy(&mut self, e: &TExpr, f: &mut Frame) -> R<bool> {
        match self.eval(e, f)? {
            Value::Bool(b) => Ok(b),
            _ => unreachable!("Mantık bekleniyordu"),
        }
    }

    // ------------------------------------------------------------ yerler (atama hedefleri)

    pub(crate) fn resolve_place(&mut self, p: &TPlace, f: &mut Frame) -> R<(Root, Vec<Step>)> {
        match p {
            TPlace::Var(Slot::Local(i)) => Ok((Root::Local(*i), vec![])),
            TPlace::Var(Slot::Global(i)) => Ok((Root::Global(*i), vec![])),
            TPlace::Bu => Ok(match &f.bu {
                BuRef::Val(_) => (Root::Bu, vec![]),
                BuRef::Alias(g, steps) => (Root::Global(*g), steps.clone()),
            }),
            TPlace::Field(b, n) => {
                let (r, mut s) = self.resolve_place(b, f)?;
                s.push(Step::Field(*n));
                Ok((r, s))
            }
            TPlace::ListIndex(b, i) => {
                let (r, mut s) = self.resolve_place(b, f)?;
                let Value::Int(iv) = self.eval(i, f)? else { unreachable!() };
                let Some(ix) = iv.as_i64() else { return rt(i.loc, "İndeks çok büyük.") };
                s.push(Step::Idx(ix));
                Ok((r, s))
            }
            TPlace::DictKey(b, k) => {
                let (r, mut s) = self.resolve_place(b, f)?;
                let kv = self.eval(k, f)?;
                s.push(Step::Key(key_of(&kv, k.loc)?, kv));
                Ok((r, s))
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
            return rt(loc, BELIRSIZ);
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
        self.write_resolved(&rp, v, f, loc)
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
                    return rt(e.loc, BELIRSIZ);
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
                        return rt(
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
                self.call(*fid, vals, BuRef::Val(Value::Hic), e.loc)?.0
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
            TK::Range { start, end, inclusive } => {
                let s = match start {
                    Some(x) => self.eval_i64(x, f)?,
                    None => 0,
                };
                let en = match end {
                    Some(x) => Some(self.eval_i64(x, f)?),
                    None => None,
                };
                Value::Range(Rc::new(RangeV { start: s, end: en, inclusive: *inclusive, step: 1 }))
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
                    let _ = self.out.write_all(line.as_bytes());
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
                        bm_mut(*m, target, vals, e.loc, prog)?
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
                        let Some(ix) = ix.as_i64() else { return rt(e.loc, "İndeks çok büyük.") };
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
            TK::Slice(x, r) => {
                let v = self.eval(x, f)?;
                let Value::Range(r) = self.eval(r, f)? else { unreachable!() };
                let Value::List(l) = v else { unreachable!() };
                let len = l.items.len() as i64;
                let norm = |i: i64| -> i64 { (if i < 0 { i.saturating_add(len) } else { i }).clamp(0, len) };
                let s = norm(r.start);
                let en = match r.end {
                    Some(e) => norm(if r.inclusive { e.saturating_add(1) } else { e }),
                    None => len,
                };
                let items = if s < en { l.items[s as usize..en as usize].to_vec() } else { vec![] };
                Value::List(Rc::new(ListV { items }))
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
                        return rt(loc, BELIRSIZ);
                    }
                    let bu = walk_ref(root, &rp.1, loc, self.prog)?;
                    self.dispatch(bu, view, mid, name, loc)?
                }
            };
            let (_, steps) = rp;
            return Ok(self.call(fid, vals, BuRef::Alias(g, steps), loc)?.0);
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
                Ok(ret)
            }
            Ok((ret, BuRef::Alias(..))) => Ok(ret),
            Err(e) => Err(e),
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
            return rt(loc, "Çok derin özyineleme: bir fonksiyon kendini çok fazla çağırdı.");
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
        slots.resize(ch.nregs as usize, Value::Uninit);
        let mut frame = Frame { slots, bu };
        self.depth += 1;
        let r = self.run_chunk(ch, &mut frame);
        self.depth -= 1;
        let Frame { mut slots, bu } = frame;
        slots.clear();
        self.pool.push(slots);
        Ok((r?, bu))
    }

    pub(crate) fn neg_value(&self, v: Value, ty: &Type, loc: Loc) -> R<Value> {
        Ok(match (&v, ty) {
            (Value::Int(i), Type::Int { bits, sign }) => {
                let r = match i {
                    Int::S(a) => a.checked_neg().map(Int::S).unwrap_or_else(|| Int::from_big(-BigInt::from(*a))),
                    Int::B(b) => Int::from_big(-(**b).clone()),
                };
                if !fits_int(&r, *bits, *sign) {
                    return rt(loc, format!("{} taştı: -({i}) = {r}, ama {} sınırları {}.", self.tn(ty), self.tn(ty), bounds_text(*bits, *sign)));
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
                    return rt(loc, "Sıfıra bölme hatası.");
                }
                let r = match (&x, &y) {
                    (Int::S(p), Int::S(q)) => {
                        let fast = match op {
                            BinOp::Add => p.checked_add(*q),
                            BinOp::Sub => p.checked_sub(*q),
                            BinOp::Mul => p.checked_mul(*q),
                            BinOp::Div => p.checked_div(*q),
                            BinOp::Rem => p.checked_rem(*q),
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
                    return rt(loc, self.overflow_msg(op, &x, &y, &r, *bits, *sign, ty));
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
                    _ => unreachable!(),
                };
                Ok(Value::Float(round_float(r, *bits)))
            }
            _ => unreachable!("sayı bekleniyordu"),
        }
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
                    return rt(loc, format!("{} değeri {} tipine sığmıyor ({}).", group_digits(&i.to_string()), self.tn(to), bounds_text(*bits, *sign)));
                }
                Ok(v)
            }
            (Value::Float(x), Type::Int { bits, sign }) => {
                if !x.is_finite() {
                    return rt(loc, format!("{} tam sayıya çevrilemez.", fmt_float(*x, 64)));
                }
                let t = BigInt::from_f64(x.trunc()).unwrap_or_default();
                let i = Int::from_big(t);
                if !fits_int(&i, *bits, *sign) {
                    return rt(loc, format!("{} değeri {} tipine sığmıyor ({}).", fmt_float(*x, 64), self.tn(to), bounds_text(*bits, *sign)));
                }
                Ok(Value::Int(i))
            }
            (Value::Int(i), Type::Float(b)) => {
                let r = round_float(i.to_f64(), *b);
                if r.is_infinite() {
                    return rt(loc, format!("{} değeri {} tipine sığmıyor ({}).", group_digits(&i.to_string()), self.tn(to), crate::types::float_limit_text(*b)));
                }
                Ok(Value::Float(r))
            }
            (Value::Float(x), Type::Float(b)) => {
                let r = round_float(*x, *b);
                if r.is_infinite() && x.is_finite() {
                    return rt(loc, format!("{} değeri {} tipine sığmıyor ({}).", fmt_float(*x, 64), self.tn(to), crate::types::float_limit_text(*b)));
                }
                Ok(Value::Float(r))
            }
            _ => {
                if value_matches(&v, to, prog) {
                    let _ = from;
                    Ok(v)
                } else {
                    rt(loc, format!("Bu değer {} değil, {}.", self.tn(to), kind_name(&v, prog)))
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

pub(crate) fn i128_op(op: BinOp, a: i128, b: i128) -> i128 {
    // İki i64'ün sonucu i128'e her zaman sığar.
    match op {
        BinOp::Add => a + b,
        BinOp::Sub => a - b,
        BinOp::Mul => a * b,
        BinOp::Div => a / b,
        BinOp::Rem => a % b,
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
        (Value::Range(_), Type::Range) => true,
        (Value::Obj(o), Type::Named(t)) => prog.is_subtype(o.ty, *t),
        (Value::Var(x), Type::Named(t)) => prog.is_subtype(x.ty, *t),
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
    matches!(m, BM::Ekle | BM::ArayaEkle | BM::Sil | BM::Cikar | BM::Sirala | BM::Ters | BM::Temizle | BM::DSil | BM::DTemizle)
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
                        return rt(loc, format!("{i}. konuma eklenemez; liste {len} elemanlı (0 .. {len} arası olmalı)."));
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
                        None => rt(loc, "Liste boş, eleman çıkarılamaz."),
                    };
                }
                BM::Sirala => sort_values(&mut l.items),
                BM::Ters => l.items.reverse(),
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
                return rt(loc, format!("Yazının {i}. karakteri yok; yazı {n} karakterli."));
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
        (BM::Tama, Value::Str(s)) => match s.trim().parse::<i64>() {
            Ok(n) => Value::int(n),
            Err(_) => return rt(loc, format!("\"{s}\" bir tam sayıya çevrilemez.")),
        },
        (BM::Ondaliga, Value::Str(s)) => match parse_float(s) {
            Some(x) => Value::Float(x),
            None => return rt(loc, format!("\"{s}\" bir ondalık sayıya çevrilemez.")),
        },
        (BM::FTama, Value::Float(x)) => {
            if !x.is_finite() || x.trunc() >= 9_223_372_036_854_775_808.0 || x.trunc() < -9_223_372_036_854_775_808.0 {
                return rt(loc, format!("{} değeri Tam(64) tipine sığmıyor ({}).", fmt_float(*x, 64), bounds_text(Some(64), Sign::Signed)));
            }
            Value::int(x.trunc() as i64)
        }
        (BM::Adim, Value::Range(r)) => {
            let n = arg_i64(&args[0], loc)?;
            if n <= 0 {
                return rt(loc, format!("Adım pozitif olmalı, ama {n} verildi."));
            }
            Value::Range(Rc::new(RangeV { step: n, ..**r }))
        }
        _ => unreachable!("yerleşik metot {m:?}"),
    })
}

/// Yazıyı ondalık sayıya çevirir. Virgül de ondalık ayırıcı kabul edilir ("3,5").
pub fn parse_float(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse::<f64>().ok()
}
