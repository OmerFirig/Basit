// Bytecode yorumlayıcı: fonksiyon ve dosya gövdeleri bir kez yazmaç tabanlı komutlara çevrilir,
// sonra sıkı bir döngüde çalıştırılır (ağaç yorumlayıcıdan birkaç kat hızlı).
//
// Yazmaçlar: çerçevenin ilk `nslots` yuvası TIR'daki değişkenlerdir (Slot::Local), sonrası geçici
// değerler içindir. Sık kullanılan yapılar (aritmetik, karşılaştırma, döngüler, çağrılar, alan ve
// liste erişimi...) için özel komutlar vardır. Nadir ve karmaşık yapılar (desenler, 'olarak',
// çoğu hazır metot...) ağaç yorumlayıcıya (interp.rs) devredilir: Eval ve Exec komutları. İki yol
// aynı çerçeveyi kullandığı için davranış birebir aynıdır.

use crate::ast::BinOp;
use crate::interp::{rt, BuRef, Ctl, Frame, Interp, Unwind, R};
use crate::tir::*;
use crate::types::{Type, TypeId, TAM64};
use crate::values::*;
use std::rc::Rc;

/// İşlenen: yazmaç, tek kullanımlık geçici yazmaç ya da sabit.
/// Geçici (T) değerler okunduktan sonra yazmaçtan silinir: yazmaçta kalan bir referans, değiştirilen
/// listenin her seferinde kopyalanmasına (yazınca kopyala) yol açardı.
#[derive(Clone, Copy, Debug)]
pub(crate) enum A {
    R(u32),
    T(u32),
    K(u32),
    /// dosya değişkeni (doğrudan okunur; atanmış olduğu önceden CheckG ile kontrol edilir)
    G(u32),
}

pub(crate) enum FsPart {
    Lit(Rc<str>),
    Val(A, u32),
}

const YOK: u32 = u32::MAX;

pub(crate) enum Ins<'p> {
    Mov(u32, A),
    LoadG(u32, u32),
    StoreG(u32, A),
    Uninit(u32),
    CheckG(u32),
    ArithG64(BinOp, u32, A),
    CompoundFieldBu(u32, BinOp, A, u32),
    Arith64(BinOp, u32, A, A),
    Arith(BinOp, u32, A, A, u32),
    Cmp64(BinOp, u32, A, A),
    Cmp(BinOp, u32, A, A),
    /// karşılaştırma doğru değilse atla (döngü ve koşullar için birleşik komut)
    JmpIfNot64(BinOp, A, A, u32),
    /// karşılaştırma doğruysa atla (koşulu sonda olan döngüler)
    JmpIf64(BinOp, A, A, u32),
    Not(u32, A),
    Neg(u32, A, u32),
    Jmp(u32),
    JmpF(A, u32),
    JmpT(A, u32),
    Call(u32, FuncId, u32, u32),
    CallM { d: u32, recv: &'p TPlace, view: TypeId, mid: u32, name: &'p str, args: u32, n: u32 },
    CallMV { d: u32, recv: u32, view: TypeId, mid: u32, name: &'p str, args: u32, n: u32 },
    CallD { d: u32, func: FuncId, recv: &'p TPlace, args: u32, n: u32 },
    CallDV { d: u32, func: FuncId, recv: u32, args: u32, n: u32 },
    Ret(A),
    RetV,
    Print(u32, u32, u32),
    Eval(u32, &'p TExpr, u32, u32),
    Exec(&'p TS, u32, u32),
    IterList(u32, u32, Slot, u32),
    /// Aralık döngüsü: (taban, başlangıç, bitiş, adım, dahil mi). Taban yazmaçları: ilk değer, adım,
    /// tur sayısı (işaretsiz), sayaç. Liste oluşturulmaz.
    RangeInit(u32, A, A, A, bool),
    /// sayaç < tur sayısı ise değişkene `ilk + sayaç * adım` yazılır, değilse çıkış
    IterRange(u32, Slot, u32),
    RangeStep(u32),
    KereCheck(A),
    Field(u32, A, u32),
    FieldBu(u32, u32),
    SetIdx(Slot, A, A),
    SetFieldBu(u32, A),
    AssignPlace(&'p TPlace, A),
    CompoundPlace(&'p TPlace, BinOp, A, u32),
    Index(u32, A, A),
    Len(u32, A),
    Push(Slot, A),
    New(u32, TypeId, u32, u32),
    Variant(u32, TypeId, u32, u32, u32),
    Concat(u32, A, A),
    FStr(u32, u32),
    IntToFloat(u32, A, u32),
    /// Aralık listesi (değer olarak): hedef, başlangıç, bitiş, adım, dahil mi
    MkRange(u32, A, A, A, bool),
    MkList(u32, u32, u32),
    LoadBu(u32),
    RunModule(usize),
}

pub(crate) struct Chunk<'p> {
    pub code: Vec<Ins<'p>>,
    pub locs: Vec<Loc>,
    pub consts: Vec<Value>,
    pub types: Vec<Type>,
    pub fstrs: Vec<Vec<FsPart>>,
    pub nregs: u32,
}

// ------------------------------------------------------------ derleme (TIR -> bytecode)

struct Comp<'p> {
    ch: Chunk<'p>,
    next: u32,
    /// döngüler: (devam hedefi, dur için yamalanacak komutlar)
    loops: Vec<(u32, Vec<usize>)>,
    loc: Loc,
    /// Bu dosyanın kendi değişkenleri (dosya kodu derleniyorsa). Bunların atanmış olduğu derleme anında
    /// akış analiziyle kontrol edildiği için okunurken CheckG gerekmez (derleyicideki gibi).
    own_globals: std::collections::HashSet<u32>,
}

pub(crate) fn compile_func<'p>(prog: &'p Program, f: &'p TFunc) -> Chunk<'p> {
    compile_body(prog, &f.body, f.nslots, Default::default())
}

pub(crate) fn compile_module<'p>(prog: &'p Program, stmts: &'p [TS]) -> Chunk<'p> {
    let mut own = std::collections::HashMap::new();
    crate::codegen::collect_slots_stmts(stmts, &mut own, true, prog);
    compile_body(prog, stmts, 0, own.into_keys().collect())
}

fn compile_body<'p>(prog: &'p Program, body: &'p [TS], nslots: u32, own_globals: std::collections::HashSet<u32>) -> Chunk<'p> {
    let _ = prog;
    let mut c = Comp {
        ch: Chunk { code: Vec::new(), locs: Vec::new(), consts: Vec::new(), types: Vec::new(), fstrs: Vec::new(), nregs: nslots },
        next: nslots,
        loops: Vec::new(),
        loc: Loc::default(),
        own_globals,
    };
    for s in body {
        c.stmt(s);
    }
    c.emit(Ins::RetV);
    c.ch
}

impl<'p> Comp<'p> {
    fn emit(&mut self, i: Ins<'p>) -> usize {
        self.ch.code.push(i);
        self.ch.locs.push(self.loc);
        self.ch.code.len() - 1
    }
    fn pc(&self) -> u32 {
        self.ch.code.len() as u32
    }
    fn temp(&mut self) -> u32 {
        let t = self.next;
        self.next += 1;
        self.ch.nregs = self.ch.nregs.max(self.next);
        t
    }
    fn temps(&mut self, n: u32) -> u32 {
        let t = self.next;
        self.next += n;
        self.ch.nregs = self.ch.nregs.max(self.next);
        t
    }
    fn konst(&mut self, v: Value) -> A {
        self.ch.consts.push(v);
        A::K(self.ch.consts.len() as u32 - 1)
    }
    fn ty(&mut self, t: &Type) -> u32 {
        self.ch.types.push(t.clone());
        self.ch.types.len() as u32 - 1
    }
    fn patch(&mut self, at: usize, target: u32) {
        match &mut self.ch.code[at] {
            Ins::Jmp(t) | Ins::JmpF(_, t) | Ins::JmpT(_, t) | Ins::JmpIfNot64(_, _, _, t) | Ins::JmpIf64(_, _, _, t) => *t = target,
            Ins::IterList(_, _, _, t) | Ins::IterRange(_, _, t) => *t = target,
            Ins::Exec(_, b, _) | Ins::Eval(_, _, b, _) => *b = target,
            _ => unreachable!(),
        }
    }
    /// Ağaç yorumlayıcıya devredilen komut/ifade için döngü hedefleri (dur yamalanır)
    fn loop_targets(&mut self) -> (u32, bool) {
        match self.loops.last() {
            Some((c, _)) => (*c, true),
            None => (YOK, false),
        }
    }
    fn exec_fallback(&mut self, s: &'p TS) {
        let (cont, in_loop) = self.loop_targets();
        let at = self.emit(Ins::Exec(s, YOK, cont));
        if in_loop {
            self.loops.last_mut().unwrap().1.push(at);
        }
    }

    // ------------------------------------------------------------ ifadeler

    /// Sırayla hesaplanan ifadeler. Dosya değişkeni doğrudan (kullanıldığı anda) okunduğu için,
    /// sonraki ifadelerden biri onu değiştirebiliyorsa (çağrı gibi) değeri önceden kopyalanır.
    fn exprs(&mut self, es: &[&'p TExpr]) -> Vec<A> {
        let mut out = Vec::with_capacity(es.len());
        for (i, e) in es.iter().enumerate() {
            let a = self.expr(e);
            let a = match a {
                A::G(g) if es[i + 1..].iter().any(|x| !is_pure(x)) => {
                    let t = self.temp();
                    self.emit(Ins::LoadG(t, g));
                    A::T(t)
                }
                a => a,
            };
            out.push(a);
        }
        out
    }

    /// İfadeyi hesaplar; değeri tutan işleneni döndürür (yerel değişken ve sabitlerde kopya yok).
    fn expr(&mut self, e: &'p TExpr) -> A {
        match &e.kind {
            TK::Var(Slot::Local(i)) => return A::R(*i),
            TK::Var(Slot::Global(g)) => {
                self.loc = e.loc;
                if !self.own_globals.contains(g) {
                    self.emit(Ins::CheckG(*g));
                }
                return A::G(*g);
            }
            TK::Int(n, small) => {
                return match small {
                    Some(v) => self.konst(Value::Int(Int::S(*v))),
                    None => self.konst(Value::Int(Int::from_big(n.clone()))),
                };
            }
            TK::Float(x) => {
                let bits = if let Type::Float(b) = e.ty { b } else { 64 };
                return self.konst(Value::Float(round_float(*x, bits)));
            }
            TK::Str(s) => return self.konst(Value::Str(s.clone())),
            TK::Bool(b) => return self.konst(Value::Bool(*b)),
            TK::Hic => return self.konst(Value::Hic),
            TK::Convert(x) if is_int_widen(&x.ty, &e.ty) => return self.expr(x),
            _ => {}
        }
        let d = self.temp();
        self.expr_to(e, d);
        A::T(d)
    }

    /// İfadenin değerini `d` yazmacına yazar.
    fn expr_to(&mut self, e: &'p TExpr, d: u32) {
        let saved = self.loc;
        self.loc = e.loc;
        match &e.kind {
            TK::Var(Slot::Local(_)) | TK::Int(..) | TK::Float(_) | TK::Str(_) | TK::Bool(_) | TK::Hic => {
                let a = self.expr(e);
                self.emit(Ins::Mov(d, a));
            }
            TK::Var(Slot::Global(g)) => {
                self.emit(Ins::LoadG(d, *g));
            }
            TK::Convert(x) if is_int_widen(&x.ty, &e.ty) => self.expr_to(x, d),
            TK::Convert(x) if x.ty.is_int() && e.ty.is_float() && x.ty != Type::Int { bits: None, sign: crate::types::Sign::Signed } => {
                let a = self.expr(x);
                let Type::Float(b) = e.ty else { unreachable!() };
                self.emit(Ins::IntToFloat(d, a, b));
            }
            TK::Arith(op, l, r) => {
                let v = self.exprs(&[l, r]);
                let (a, b) = (v[0], v[1]);
                if e.ty == TAM64 {
                    self.emit(Ins::Arith64(*op, d, a, b));
                } else {
                    let t = self.ty(&e.ty);
                    self.emit(Ins::Arith(*op, d, a, b, t));
                }
            }
            TK::Cmp(op, l, r) => {
                let v = self.exprs(&[l, r]);
                let (a, b) = (v[0], v[1]);
                if l.ty == TAM64 {
                    self.emit(Ins::Cmp64(*op, d, a, b));
                } else {
                    self.emit(Ins::Cmp(*op, d, a, b));
                }
            }
            TK::Not(x) => {
                let a = self.expr(x);
                self.emit(Ins::Not(d, a));
            }
            TK::Neg(x) => {
                let a = self.expr(x);
                let t = self.ty(&e.ty);
                self.emit(Ins::Neg(d, a, t));
            }
            TK::And(l, r) | TK::Or(l, r) => {
                // Sonuç önce geçici yazmaçta oluşur: `x = y ve x` yazımında sağ taraf x'in eski değerini görmeli.
                let is_and = matches!(e.kind, TK::And(..));
                let t = self.temp();
                self.expr_to(l, t);
                let j = if is_and { self.emit(Ins::JmpF(A::R(t), 0)) } else { self.emit(Ins::JmpT(A::R(t), 0)) };
                self.expr_to(r, t);
                let end = self.pc();
                self.patch(j, end);
                self.emit(Ins::Mov(d, A::T(t)));
            }
            TK::Concat(l, r) => {
                let v = self.exprs(&[l, r]);
                let (a, b) = (v[0], v[1]);
                self.emit(Ins::Concat(d, a, b));
            }
            TK::FStr(parts) => {
                let xs: Vec<&'p TExpr> = parts.iter().filter_map(|p| if let TFPart::Expr(x) = p { Some(x) } else { None }).collect();
                let mut vals = self.exprs(&xs).into_iter();
                let mut ps = Vec::new();
                for p in parts {
                    match p {
                        TFPart::Lit(s) => ps.push(FsPart::Lit(s.clone())),
                        TFPart::Expr(x) => {
                            let a = vals.next().unwrap();
                            let t = self.ty(&x.ty);
                            ps.push(FsPart::Val(a, t));
                        }
                    }
                }
                self.ch.fstrs.push(ps);
                let idx = self.ch.fstrs.len() as u32 - 1;
                self.emit(Ins::FStr(d, idx));
            }
            TK::Call(fid, args) => {
                let base = self.args(args);
                self.emit(Ins::Call(d, *fid, base, args.len() as u32));
            }
            TK::Method { recv, view, name, mid, args } => {
                let base = self.args(args);
                match recv {
                    Recv::Place(p) => {
                        self.emit(Ins::CallM { d, recv: p, view: *view, mid: *mid, name, args: base, n: args.len() as u32 });
                    }
                    Recv::Value(x) => {
                        let r = self.temp();
                        self.expr_to(x, r);
                        self.emit(Ins::CallMV { d, recv: r, view: *view, mid: *mid, name, args: base, n: args.len() as u32 });
                    }
                }
            }
            TK::Direct { func, recv, args } => {
                let base = self.args(args);
                match recv {
                    Recv::Place(p) => {
                        self.emit(Ins::CallD { d, func: *func, recv: p, args: base, n: args.len() as u32 });
                    }
                    Recv::Value(x) => {
                        let r = self.temp();
                        self.expr_to(x, r);
                        self.emit(Ins::CallDV { d, func: *func, recv: r, args: base, n: args.len() as u32 });
                    }
                }
            }
            TK::Builtin(Builtin::Yazdir, args) => {
                let base = self.args(args);
                let t0 = self.ch.types.len() as u32;
                for a in args {
                    self.ty(&a.ty);
                }
                self.emit(Ins::Print(base, args.len() as u32, t0));
            }
            TK::Field(x, fid) => {
                if let TK::Bu = x.kind {
                    self.emit(Ins::FieldBu(d, *fid));
                } else {
                    let a = self.expr(x);
                    self.emit(Ins::Field(d, a, *fid));
                }
            }
            TK::Index(x, i) => {
                let v = self.exprs(&[x, i]);
                let (a, b) = (v[0], v[1]);
                self.emit(Ins::Index(d, a, b));
            }
            TK::BMethod { recv: Recv::Value(x), m: BM::Uzunluk | BM::DUzunluk | BM::SUzunluk, .. } => {
                let a = self.expr(x);
                self.emit(Ins::Len(d, a));
            }
            TK::BMethod { recv: Recv::Place(TPlace::Var(s)), m: BM::Ekle, args } => {
                let a = self.expr(&args[0]);
                self.emit(Ins::Push(*s, a));
                let h = self.konst(Value::Hic);
                self.emit(Ins::Mov(d, h));
            }
            TK::New(tid, args) => {
                let base = self.args(args);
                self.emit(Ins::New(d, *tid, base, args.len() as u32));
            }
            TK::Variant(tid, idx, args) => {
                let base = self.args(args);
                self.emit(Ins::Variant(d, *tid, *idx, base, args.len() as u32));
            }
            TK::Range { start, end, inclusive, step } => {
                let (s, en, k) = self.range_args(start, end, step.as_deref());
                self.loc = e.loc;
                self.emit(Ins::MkRange(d, s, en, k, *inclusive));
            }
            TK::List(items) => {
                let base = self.args(items);
                self.emit(Ins::MkList(d, base, items.len() as u32));
            }
            TK::Bu => {
                self.emit(Ins::LoadBu(d));
            }
            _ => {
                let (cont, in_loop) = self.loop_targets();
                let at = self.emit(Ins::Eval(d, e, YOK, cont));
                if in_loop {
                    self.loops.last_mut().unwrap().1.push(at);
                }
            }
        }
        self.loc = saved;
    }

    /// Dosya değişkeni işlenenini geçici yazmaca kopyalar (sonradan çalışacak kod değiştirebilir).
    fn materialize(&mut self, a: A) -> A {
        match a {
            A::G(g) => {
                let t = self.temp();
                self.emit(Ins::LoadG(t, g));
                A::T(t)
            }
            a => a,
        }
    }

    /// Argümanları ardışık geçici yazmaçlara hesaplar; ilk yazmacı döndürür.
    fn args(&mut self, args: &'p [TExpr]) -> u32 {
        let base = self.temps(args.len() as u32);
        for (i, a) in args.iter().enumerate() {
            self.expr_to(a, base + i as u32);
        }
        base
    }

    /// Koşul doğru değilse atlanacak komutu yayar; yamalanacak komutun yerini döndürür.
    fn cond_jump(&mut self, c: &'p TExpr) -> usize {
        if let TK::Cmp(op, l, r) = &c.kind {
            if l.ty == TAM64 {
                let v = self.exprs(&[l, r]);
                let (a, b) = (v[0], v[1]);
                self.loc = c.loc;
                return self.emit(Ins::JmpIfNot64(*op, a, b, 0));
            }
        }
        let a = self.expr(c);
        self.emit(Ins::JmpF(a, 0))
    }

    /// Koşul doğruysa `target`a atlar (döngü koşulu sonda).
    fn cond_jump_true(&mut self, c: &'p TExpr, target: u32) {
        if let TK::Cmp(op, l, r) = &c.kind {
            if l.ty == TAM64 {
                let v = self.exprs(&[l, r]);
                self.loc = c.loc;
                self.emit(Ins::JmpIf64(*op, v[0], v[1], target));
                return;
            }
        }
        let a = self.expr(c);
        self.emit(Ins::JmpT(a, target));
    }

    /// Koşulu sonda olan döngü: atla -> [gövde, adım] -> koşul doğruysa başa.
    fn bottom_loop(&mut self, c: &'p TExpr, body: &'p [TS], step: Option<&'p TS>) {
        let j0 = self.emit(Ins::Jmp(0));
        let top = self.pc();
        self.loops.push((YOK, Vec::new()));
        let body_start = self.ch.code.len();
        self.block(body);
        let cont = self.pc();
        self.fix_continue(body_start, cont);
        if let Some(st) = step {
            self.stmt(st);
        }
        let cond = self.pc();
        self.patch(j0, cond);
        let mark = self.next;
        self.cond_jump_true(c, top);
        self.next = mark;
        let exit = self.pc();
        let (_, breaks) = self.loops.pop().unwrap();
        for b in breaks {
            self.patch(b, exit);
        }
    }

    // ------------------------------------------------------------ komutlar

    fn block(&mut self, stmts: &'p [TS]) {
        for s in stmts {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &'p TS) {
        let mark = self.next;
        self.stmt_inner(s);
        self.next = mark;
    }

    fn store_slot(&mut self, slot: Slot, e: &'p TExpr) {
        match slot {
            Slot::Local(i) => self.expr_to(e, i),
            Slot::Global(g) => {
                let a = self.expr(e);
                self.loc = e.loc;
                self.emit(Ins::StoreG(g, a));
            }
        }
    }

    fn stmt_inner(&mut self, s: &'p TS) {
        match s {
            TS::Let(slot, e) => self.store_slot(*slot, e),
            TS::Assign(TPlace::Var(slot), e, _) => self.store_slot(*slot, e),
            TS::Declare(Slot::Local(i), _) => {
                self.emit(Ins::Uninit(*i));
            }
            TS::Declare(..) => self.exec_fallback(s),
            TS::Assign(p, e, loc) => {
                match p {
                    TPlace::ListIndex(b, i) if matches!(**b, TPlace::Var(_)) => {
                        let TPlace::Var(slot) = **b else { unreachable!() };
                        let v = self.exprs(&[e, i]);
                        let (a, ia) = (v[0], v[1]);
                        self.loc = *loc;
                        self.emit(Ins::SetIdx(slot, ia, a));
                    }
                    TPlace::Field(b, fid) if matches!(**b, TPlace::Bu) => {
                        let a = self.expr(e);
                        self.loc = *loc;
                        self.emit(Ins::SetFieldBu(*fid, a));
                    }
                    _ => {
                        let a = self.expr(e);
                        let a = self.materialize(a);
                        self.loc = *loc;
                        self.emit(Ins::AssignPlace(p, a));
                    }
                }
            }
            TS::Compound(TPlace::Var(Slot::Global(g)), op, e, ty, loc) if *ty == TAM64 => {
                let b = self.expr(e);
                self.loc = *loc;
                // atanmamış değişken ArithG64'ün yavaş yolunda (compound_place) yakalanır
                self.emit(Ins::ArithG64(*op, *g, b));
            }
            TS::Compound(TPlace::Field(bp, fid), op, e, ty, loc) if matches!(**bp, TPlace::Bu) && *ty != Type::Str => {
                let b = self.expr(e);
                self.loc = *loc;
                let t = self.ty(ty);
                self.emit(Ins::CompoundFieldBu(*fid, *op, b, t));
            }
            TS::Compound(TPlace::Var(Slot::Local(i)), op, e, ty, loc) if *ty != Type::Str => {
                let b = self.expr(e);
                self.loc = *loc;
                if *ty == TAM64 {
                    self.emit(Ins::Arith64(*op, *i, A::R(*i), b));
                } else {
                    let t = self.ty(ty);
                    self.emit(Ins::Arith(*op, *i, A::R(*i), b, t));
                }
            }
            TS::Compound(p, op, e, ty, loc) => {
                let b = self.expr(e);
                let b = self.materialize(b);
                self.loc = *loc;
                let t = self.ty(ty);
                self.emit(Ins::CompoundPlace(p, *op, b, t));
            }
            TS::Expr(e) => {
                let d = self.temp();
                self.expr_to(e, d);
            }
            TS::If(branches, else_) => {
                let mut ends = Vec::new();
                for (c, b) in branches {
                    let j = self.cond_jump(c);
                    self.block(b);
                    ends.push(self.emit(Ins::Jmp(0)));
                    let next = self.pc();
                    self.patch(j, next);
                }
                if let Some(b) = else_ {
                    self.block(b);
                }
                let end = self.pc();
                for j in ends {
                    self.patch(j, end);
                }
            }
            TS::While(c, body) => self.bottom_loop(c, body, None),
            TS::CFor(init, c, step, body) => {
                self.stmt(init);
                self.bottom_loop(c, body, Some(step));
            }
            TS::Kere(c, body) if c.ty == TAM64 => {
                let n = self.temp();
                self.expr_to(c, n);
                self.loc = c.loc;
                self.emit(Ins::KereCheck(A::R(n)));
                let ctr = self.temp();
                let zero = self.konst(Value::int(0));
                self.emit(Ins::Mov(ctr, zero));
                let top = self.pc();
                let j = self.emit(Ins::JmpIfNot64(BinOp::Lt, A::R(ctr), A::R(n), 0));
                self.loops.push((YOK, Vec::new()));
                let body_start = self.ch.code.len();
                self.block(body);
                let step_pc = self.pc();
                self.fix_continue(body_start, step_pc);
                let one = self.konst(Value::int(1));
                self.emit(Ins::Arith64(BinOp::Add, ctr, A::R(ctr), one));
                self.emit(Ins::Jmp(top));
                self.end_loop(j);
            }
            TS::ForEach { iter, binds, body } if matches!(iter.kind, TK::Range { .. }) => {
                let TK::Range { start, end, inclusive, step } = &iter.kind else { unreachable!() };
                let (s, en, k) = self.range_args(start, end, step.as_deref());
                let base = self.temps(4);
                self.loc = iter.loc;
                self.emit(Ins::RangeInit(base, s, en, k, *inclusive));
                let top = self.pc();
                let j = self.emit(Ins::IterRange(base, binds[0], 0));
                self.loops.push((YOK, Vec::new()));
                let body_start = self.ch.code.len();
                self.block(body);
                let step_pc = self.pc();
                self.fix_continue(body_start, step_pc);
                self.emit(Ins::RangeStep(base));
                self.emit(Ins::Jmp(top));
                self.end_loop(j);
            }
            TS::ForEach { iter, binds, body } if matches!(iter.ty, Type::List(..)) => {
                let l = self.temp();
                self.expr_to(iter, l);
                let i = self.temp();
                let zero = self.konst(Value::int(0));
                self.emit(Ins::Mov(i, zero));
                let top = self.pc();
                self.loc = iter.loc;
                let j = self.emit(Ins::IterList(l, i, binds[0], 0));
                self.loops.push((top, Vec::new()));
                self.block(body);
                self.emit(Ins::Jmp(top));
                self.end_loop(j);
            }
            TS::Break => {
                let at = self.emit(Ins::Jmp(0));
                self.loops.last_mut().expect("döngü").1.push(at);
            }
            TS::Continue => {
                let c = self.loops.last().expect("döngü").0;
                let at = self.emit(Ins::Jmp(c));
                if c == YOK {
                    // adım komutunun yeri sonra belli olur (fix_continue)
                    let _ = at;
                }
            }
            TS::Return(Some(e)) => {
                let a = self.expr(e);
                self.emit(Ins::Ret(a));
            }
            TS::Return(None) => {
                self.emit(Ins::RetV);
            }
            TS::RunModule(m) => {
                self.emit(Ins::RunModule(*m));
            }
            _ => self.exec_fallback(s),
        }
    }

    /// Aralığın başlangıç, bitiş ve adımı (soldan sağa, her biri bir kez hesaplanır)
    fn range_args(&mut self, start: &'p TExpr, end: &'p TExpr, step: Option<&'p TExpr>) -> (A, A, A) {
        match step {
            Some(k) => {
                let v = self.exprs(&[start, end, k]);
                (v[0], v[1], v[2])
            }
            None => {
                let v = self.exprs(&[start, end]);
                let one = self.konst(Value::int(1));
                (v[0], v[1], one)
            }
        }
    }

    /// Döngü gövdesindeki `devam` atlamaları (hedefi YOK olanlar) adım komutuna yönlendirilir.
    fn fix_continue(&mut self, from: usize, step_pc: u32) {
        let depth = self.loops.len();
        let _ = depth;
        for k in from..self.ch.code.len() {
            match &mut self.ch.code[k] {
                Ins::Jmp(t) if *t == YOK => *t = step_pc,
                Ins::Exec(_, _, c) | Ins::Eval(_, _, _, c) if *c == YOK => *c = step_pc,
                _ => {}
            }
        }
        if let Some(l) = self.loops.last_mut() {
            l.0 = step_pc;
        }
    }

    fn end_loop(&mut self, cond_jump: usize) {
        let exit = self.pc();
        self.patch(cond_jump, exit);
        let (_, breaks) = self.loops.pop().unwrap();
        for b in breaks {
            self.patch(b, exit);
        }
    }
}

/// Tam -> Tam genişletmesi çalışma anında bir şey yapmaz (değerler zaten aynı biçimde).
fn is_int_widen(from: &Type, to: &Type) -> bool {
    from.is_int() && to.is_int()
}

// ------------------------------------------------------------ çalıştırma

impl<'p> Interp<'p> {
    #[inline(always)]
    fn get(&self, ch: &Chunk<'p>, f: &mut Frame, a: A) -> Value {
        self.take(ch, f, a)
    }

    /// Okunan tek kullanımlık geçici değeri yazmaçtan siler.
    #[inline(always)]
    fn clr(f: &mut Frame, a: A) {
        if let A::T(r) = a {
            f.slots[r as usize] = Value::Hic;
        }
    }

    /// İşlenenin değerini alır: geçici yazmaçtaysa taşınır (referans sayısı gereksiz artmasın;
    /// yoksa değiştirilen listeler her seferinde kopyalanırdı), değişken ve sabitlerde kopyalanır.
    #[inline(always)]
    fn take(&self, ch: &Chunk<'p>, f: &mut Frame, a: A) -> Value {
        match a {
            A::T(r) => std::mem::replace(&mut f.slots[r as usize], Value::Hic),
            A::R(r) => f.slots[r as usize].clone(),
            A::K(k) => ch.consts[k as usize].clone(),
            A::G(g) => self.globals[g as usize].clone(),
        }
    }

    /// Çağrı argümanları: çerçeve yuvaları da bu vektörde tutulacağı için havuzdan alınır.
    fn call_args(&mut self, f: &mut Frame, base: u32, n: u32) -> Vec<Value> {
        let mut v = self.pool.pop().unwrap_or_default();
        for r in base..base + n {
            v.push(std::mem::replace(&mut f.slots[r as usize], Value::Hic));
        }
        v
    }

    fn take_regs(f: &mut Frame, base: u32, n: u32) -> Vec<Value> {
        let mut v = Vec::with_capacity(n as usize);
        for r in base..base + n {
            v.push(std::mem::replace(&mut f.slots[r as usize], Value::Hic));
        }
        v
    }

    #[inline(always)]
    fn gr<'a>(&'a self, ch: &'a Chunk<'p>, f: &'a Frame, a: A) -> &'a Value {
        match a {
            A::R(r) | A::T(r) => &f.slots[r as usize],
            A::K(k) => &ch.consts[k as usize],
            A::G(g) => &self.globals[g as usize],
        }
    }

    #[inline(always)]
    fn gi(&self, ch: &Chunk<'p>, f: &Frame, a: A) -> Option<i64> {
        match self.gr(ch, f, a) {
            Value::Int(Int::S(v)) => Some(*v),
            _ => None,
        }
    }

    /// Ağaç yorumlayıcıdan gelen kontrol akışı (dur/devam/döndür) bytecode'a çevrilir.
    fn ctl_jump(&self, c: Ctl, brk: u32, cont: u32, pc: &mut usize) -> Option<Value> {
        match c {
            Ctl::Normal => None,
            Ctl::Break => {
                *pc = brk as usize;
                None
            }
            Ctl::Continue => {
                *pc = cont as usize;
                None
            }
            Ctl::Return(v) => Some(v),
        }
    }

    /// Metodun nesnesine değiştirmek için erişim (dosya değişkenindeyse yol üzerinden).
    fn bu_obj_mut<'a>(&'a mut self, f: &'a mut Frame, loc: Loc) -> R<&'a mut Value> {
        let prog = self.prog;
        match &mut f.bu {
            BuRef::Val(v) => Ok(v),
            BuRef::Alias(g, steps) => {
                let root = &mut self.globals[*g as usize];
                crate::interp::walk_mut(root, steps, false, loc, prog)
            }
        }
    }

    /// Bytecode'u çalıştırır; dönüş değerini verir.
    pub(crate) fn run_chunk(&mut self, ch: &Chunk<'p>, f: &mut Frame) -> R<Value> {
        let prog = self.prog;
        let mut pc = 0usize;
        loop {
            let ins = &ch.code[pc];
            pc += 1;
            match ins {
                Ins::Mov(d, a) => {
                    let v = self.take(ch, f, *a);
                    f.slots[*d as usize] = v;
                }
                Ins::LoadG(d, g) => {
                    let v = &self.globals[*g as usize];
                    if matches!(v, Value::Uninit) {
                        return rt(ch.locs[pc - 1], crate::interp::BELIRSIZ);
                    }
                    f.slots[*d as usize] = v.clone();
                }
                Ins::StoreG(g, a) => {
                    let v = self.take(ch, f, *a);
                    self.globals[*g as usize] = v;
                }
                Ins::Uninit(d) => f.slots[*d as usize] = Value::Uninit,
                Ins::CheckG(g) => {
                    if matches!(self.globals[*g as usize], Value::Uninit) {
                        return rt(ch.locs[pc - 1], crate::interp::BELIRSIZ);
                    }
                }
                Ins::ArithG64(op, g, b) => {
                    if let (Value::Int(Int::S(x)), Some(y)) = (&self.globals[*g as usize], self.gi(ch, f, *b)) {
                        let x = *x;
                        let r = match op {
                            BinOp::Add => x.checked_add(y),
                            BinOp::Sub => x.checked_sub(y),
                            BinOp::Mul => x.checked_mul(y),
                            BinOp::Div if y != 0 => x.checked_div(y),
                            BinOp::Rem if y != 0 => x.checked_rem(y),
                            _ => None,
                        };
                        if let Some(r) = r {
                            self.globals[*g as usize] = Value::Int(Int::S(r));
                            continue;
                        }
                    }
                    let rhs = self.take(ch, f, *b);
                    self.compound_place(&TPlace::Var(Slot::Global(*g)), *op, rhs, &TAM64, f, ch.locs[pc - 1])?;
                }
                Ins::CompoundFieldBu(fid, op, a, t) => {
                    let rhs = self.take(ch, f, *a);
                    let loc = ch.locs[pc - 1];
                    let ty = &ch.types[*t as usize];
                    let cur = {
                        let obj = self.bu_obj_mut(f, loc)?;
                        let Value::Obj(rc) = obj else { unreachable!() };
                        let idx = crate::interp::field_idx(prog, rc.ty, *fid, loc)?;
                        std::mem::replace(&mut Rc::make_mut(rc).fields[idx], Value::Hic)
                    };
                    let r = self.compound_value(*op, cur, rhs, ty, loc);
                    let obj = self.bu_obj_mut(f, loc)?;
                    let Value::Obj(rc) = obj else { unreachable!() };
                    let idx = crate::interp::field_idx(prog, rc.ty, *fid, loc)?;
                    let o = Rc::make_mut(rc);
                    match r {
                        Ok(v) => o.fields[idx] = v,
                        Err((e, keep)) => {
                            o.fields[idx] = keep;
                            return Err(e);
                        }
                    }
                }
                Ins::Arith64(op, d, a, b) => {
                    if let (Some(x), Some(y)) = (self.gi(ch, f, *a), self.gi(ch, f, *b)) {
                        let r = match op {
                            BinOp::Add => x.checked_add(y),
                            BinOp::Sub => x.checked_sub(y),
                            BinOp::Mul => x.checked_mul(y),
                            BinOp::Div if y != 0 => x.checked_div(y),
                            BinOp::Rem if y != 0 => x.checked_rem(y),
                            _ => None,
                        };
                        if let Some(r) = r {
                            set_int(&mut f.slots[*d as usize], r);
                            continue;
                        }
                    }
                    let (va, vb) = (self.get(ch, f, *a), self.get(ch, f, *b));
                    let r = self.arith(*op, va, vb, &TAM64, ch.locs[pc - 1])?;
                    f.slots[*d as usize] = r;
                }
                Ins::Arith(op, d, a, b, t) => {
                    let (va, vb) = (self.get(ch, f, *a), self.get(ch, f, *b));
                    let r = self.arith(*op, va, vb, &ch.types[*t as usize], ch.locs[pc - 1])?;
                    f.slots[*d as usize] = r;
                }
                Ins::Cmp64(op, d, a, b) => {
                    let r = match (self.gi(ch, f, *a), self.gi(ch, f, *b)) {
                        (Some(x), Some(y)) => cmp_i64(*op, x, y),
                        _ => crate::interp::compare(*op, self.gr(ch, f, *a), self.gr(ch, f, *b)),
                    };
                    set_bool(&mut f.slots[*d as usize], r);
                }
                Ins::Cmp(op, d, a, b) => {
                    let r = crate::interp::compare(*op, self.gr(ch, f, *a), self.gr(ch, f, *b));
                    Self::clr(f, *a);
                    Self::clr(f, *b);
                    f.slots[*d as usize] = Value::Bool(r);
                }
                Ins::JmpIf64(op, a, b, t) => {
                    let r = match (self.gi(ch, f, *a), self.gi(ch, f, *b)) {
                        (Some(x), Some(y)) => cmp_i64(*op, x, y),
                        _ => crate::interp::compare(*op, self.gr(ch, f, *a), self.gr(ch, f, *b)),
                    };
                    if r {
                        pc = *t as usize;
                    }
                }
                Ins::JmpIfNot64(op, a, b, t) => {
                    let r = match (self.gi(ch, f, *a), self.gi(ch, f, *b)) {
                        (Some(x), Some(y)) => cmp_i64(*op, x, y),
                        _ => crate::interp::compare(*op, self.gr(ch, f, *a), self.gr(ch, f, *b)),
                    };
                    if !r {
                        pc = *t as usize;
                    }
                }
                Ins::Not(d, a) => {
                    let Value::Bool(b) = *self.gr(ch, f, *a) else { unreachable!() };
                    f.slots[*d as usize] = Value::Bool(!b);
                }
                Ins::Neg(d, a, t) => {
                    let v = self.get(ch, f, *a);
                    let r = self.neg_value(v, &ch.types[*t as usize], ch.locs[pc - 1])?;
                    f.slots[*d as usize] = r;
                }
                Ins::Jmp(t) => pc = *t as usize,
                Ins::JmpF(a, t) => {
                    if let Value::Bool(false) = self.gr(ch, f, *a) {
                        pc = *t as usize;
                    }
                }
                Ins::JmpT(a, t) => {
                    if let Value::Bool(true) = self.gr(ch, f, *a) {
                        pc = *t as usize;
                    }
                }
                Ins::Call(d, fid, base, n) => {
                    let mut args = self.pool.pop().unwrap_or_default();
                    for r in *base..*base + *n {
                        args.push(std::mem::replace(&mut f.slots[r as usize], Value::Hic));
                    }
                    let r = self.call_plain(*fid, args, ch.locs[pc - 1])?;
                    f.slots[*d as usize] = r;
                }
                Ins::CallM { d, recv, view, mid, name, args, n } => {
                    let vals = self.call_args(f, *args, *n);
                    let r = self.call_on_place(recv, None, (*view, *mid, name), vals, f, ch.locs[pc - 1])?;
                    f.slots[*d as usize] = r;
                }
                Ins::CallMV { d, recv, view, mid, name, args, n } => {
                    let vals = self.call_args(f, *args, *n);
                    let bu = std::mem::replace(&mut f.slots[*recv as usize], Value::Hic);
                    let fid = self.dispatch(&bu, *view, *mid, name, ch.locs[pc - 1])?;
                    let r = self.call(fid, vals, BuRef::Val(bu), ch.locs[pc - 1])?.0;
                    f.slots[*d as usize] = r;
                }
                Ins::CallD { d, func, recv, args, n } => {
                    let vals = self.call_args(f, *args, *n);
                    let r = self.call_on_place(recv, Some(*func), (0, 0, ""), vals, f, ch.locs[pc - 1])?;
                    f.slots[*d as usize] = r;
                }
                Ins::CallDV { d, func, recv, args, n } => {
                    let vals = self.call_args(f, *args, *n);
                    let bu = std::mem::replace(&mut f.slots[*recv as usize], Value::Hic);
                    let r = self.call(*func, vals, BuRef::Val(bu), ch.locs[pc - 1])?.0;
                    f.slots[*d as usize] = r;
                }
                Ins::Ret(a) => return Ok(self.take(ch, f, *a)),
                Ins::RetV => return Ok(Value::Hic),
                Ins::Print(base, n, t0) => {
                    let mut line = String::new();
                    for i in 0..*n {
                        if i > 0 {
                            line.push(' ');
                        }
                        let v = std::mem::replace(&mut f.slots[(*base + i) as usize], Value::Hic);
                        write_value(&mut line, &v, Some(&ch.types[(*t0 + i) as usize]), prog, true);
                    }
                    line.push('\n');
                    use std::io::Write;
                    let _ = self.out.write_all(line.as_bytes());
                }
                Ins::Eval(d, e, brk, cont) => match self.eval(e, f) {
                    Ok(v) => f.slots[*d as usize] = v,
                    Err(Unwind::Ctl(c)) => {
                        if let Some(v) = self.ctl_jump(c, *brk, *cont, &mut pc) {
                            return Ok(v);
                        }
                    }
                    Err(e) => return Err(e),
                },
                Ins::Exec(s, brk, cont) => {
                    let c = match self.exec(s, f) {
                        Ok(c) => c,
                        Err(Unwind::Ctl(c)) => c,
                        Err(e) => return Err(e),
                    };
                    if let Some(v) = self.ctl_jump(c, *brk, *cont, &mut pc) {
                        return Ok(v);
                    }
                }
                Ins::IterList(l, i, bind, exit) => {
                    let idx = match &f.slots[*i as usize] {
                        Value::Int(Int::S(v)) => *v as usize,
                        _ => unreachable!(),
                    };
                    let item = match &f.slots[*l as usize] {
                        Value::List(lv) => lv.items.get(idx).cloned(),
                        _ => unreachable!(),
                    };
                    match item {
                        None => pc = *exit as usize,
                        Some(v) => {
                            f.slots[*i as usize] = Value::int(idx as i64 + 1);
                            self.set_slot(*bind, v, f);
                        }
                    }
                }
                Ins::RangeInit(base, s, e, k, incl) => {
                    let loc = ch.locs[pc - 1];
                    let (s, e, k) = (self.gi(ch, f, *s), self.gi(ch, f, *e), self.gi(ch, f, *k));
                    let (Some(s), Some(e), Some(k)) = (s, e, k) else { unreachable!("Tam(64) bekleniyordu") };
                    let Some(p) = range_plan(s, e, *incl, k) else { return rt(loc, "Adım sıfır olamaz.") };
                    let b = *base as usize;
                    f.slots[b] = Value::int(p.first);
                    f.slots[b + 1] = Value::int(p.step);
                    f.slots[b + 2] = Value::int(p.count as i64);
                    f.slots[b + 3] = Value::int(0);
                }
                Ins::IterRange(base, bind, exit) => {
                    let b = *base as usize;
                    let (Value::Int(Int::S(first)), Value::Int(Int::S(step)), Value::Int(Int::S(count)), Value::Int(Int::S(k))) =
                        (&f.slots[b], &f.slots[b + 1], &f.slots[b + 2], &f.slots[b + 3])
                    else {
                        unreachable!()
                    };
                    if (*k as u64) >= (*count as u64) {
                        pc = *exit as usize;
                    } else {
                        let v = first.wrapping_add(k.wrapping_mul(*step));
                        match bind {
                            Slot::Local(i) => set_int(&mut f.slots[*i as usize], v),
                            Slot::Global(g) => self.globals[*g as usize] = Value::int(v),
                        }
                    }
                }
                Ins::RangeStep(base) => {
                    if let Value::Int(Int::S(k)) = &mut f.slots[*base as usize + 3] {
                        *k = k.wrapping_add(1);
                    }
                }
                Ins::KereCheck(a) => {
                    if let Value::Int(Int::S(n)) = self.gr(ch, f, *a) {
                        if *n < 0 {
                            return rt(ch.locs[pc - 1], format!("'kere' için tekrar sayısı negatif olamaz ({n})."));
                        }
                    }
                }
                Ins::Field(d, a, fid) => {
                    let v = self.field_of(self.gr(ch, f, *a), *fid, ch.locs[pc - 1])?;
                    Self::clr(f, *a);
                    f.slots[*d as usize] = v;
                }
                Ins::FieldBu(d, fid) => {
                    let v = match &f.bu {
                        BuRef::Val(v) => self.field_of(v, *fid, ch.locs[pc - 1])?,
                        BuRef::Alias(g, steps) => {
                            let o = crate::interp::walk_ref(&self.globals[*g as usize], steps, ch.locs[pc - 1], prog)?;
                            self.field_of(o, *fid, ch.locs[pc - 1])?
                        }
                    };
                    f.slots[*d as usize] = v;
                }
                Ins::SetIdx(slot, i, a) => {
                    let v = self.take(ch, f, *a);
                    let loc = ch.locs[pc - 1];
                    let ix = match self.gr(ch, f, *i) {
                        Value::Int(n) => match n.as_i64() {
                            Some(x) => x,
                            None => return rt(loc, "İndeks çok büyük."),
                        },
                        _ => unreachable!(),
                    };
                    let target = match slot {
                        Slot::Local(s) => &mut f.slots[*s as usize],
                        Slot::Global(g) => {
                            let t = &mut self.globals[*g as usize];
                            if matches!(t, Value::Uninit) {
                                return rt(loc, crate::interp::BELIRSIZ);
                            }
                            t
                        }
                    };
                    let Value::List(rc) = target else { unreachable!() };
                    let real = crate::interp::norm_index(ix, rc.items.len(), loc)?;
                    Rc::make_mut(rc).items[real] = v;
                }
                Ins::SetFieldBu(fid, a) => {
                    let v = self.take(ch, f, *a);
                    let loc = ch.locs[pc - 1];
                    let obj = self.bu_obj_mut(f, loc)?;
                    let Value::Obj(rc) = obj else { unreachable!() };
                    let idx = crate::interp::field_idx(prog, rc.ty, *fid, loc)?;
                    Rc::make_mut(rc).fields[idx] = v;
                }
                Ins::AssignPlace(p, a) => {
                    let v = self.take(ch, f, *a);
                    self.assign_place(p, v, f, ch.locs[pc - 1])?;
                }
                Ins::CompoundPlace(p, op, a, t) => {
                    let rhs = self.take(ch, f, *a);
                    self.compound_place(p, *op, rhs, &ch.types[*t as usize], f, ch.locs[pc - 1])?;
                }
                Ins::Index(d, a, i) => {
                    let loc = ch.locs[pc - 1];
                    let v = match (self.gr(ch, f, *a), self.gr(ch, f, *i)) {
                        (Value::List(l), Value::Int(ix)) => {
                            let Some(ix) = ix.as_i64() else { return rt(loc, "İndeks çok büyük.") };
                            let real = crate::interp::norm_index(ix, l.items.len(), loc)?;
                            l.items[real].clone()
                        }
                        (Value::Dict(d), iv) => match d.map.get(&crate::interp::key_of(iv, loc)?) {
                            Some((_, v)) => v.clone(),
                            None => return Err(crate::interp::missing_key(prog, iv, loc)),
                        },
                        _ => unreachable!(),
                    };
                    Self::clr(f, *a);
                    Self::clr(f, *i);
                    f.slots[*d as usize] = v;
                }
                Ins::Len(d, a) => {
                    let n = match self.gr(ch, f, *a) {
                        Value::List(l) => l.items.len(),
                        Value::Dict(dv) => dv.map.len(),
                        Value::Str(s) => s.chars().count(),
                        _ => unreachable!(),
                    };
                    Self::clr(f, *a);
                    f.slots[*d as usize] = Value::int(n as i64);
                }
                Ins::Push(slot, a) => {
                    let v = self.take(ch, f, *a);
                    let loc = ch.locs[pc - 1];
                    let target = match slot {
                        Slot::Local(s) => &mut f.slots[*s as usize],
                        Slot::Global(g) => {
                            let t = &mut self.globals[*g as usize];
                            if matches!(t, Value::Uninit) {
                                return rt(loc, crate::interp::BELIRSIZ);
                            }
                            t
                        }
                    };
                    let Value::List(rc) = target else { unreachable!() };
                    Rc::make_mut(rc).items.push(v);
                }
                Ins::New(d, tid, base, n) => {
                    let fs = Self::take_regs(f, *base, *n);
                    f.slots[*d as usize] = Value::Obj(Rc::new(ObjV { ty: *tid, fields: fs }));
                }
                Ins::Variant(d, tid, idx, base, n) => {
                    let fs = Self::take_regs(f, *base, *n);
                    f.slots[*d as usize] = Value::Var(Rc::new(VarV { ty: *tid, idx: *idx, fields: fs }));
                }
                Ins::Concat(d, a, b) => {
                    let (Value::Str(x), Value::Str(y)) = (self.gr(ch, f, *a), self.gr(ch, f, *b)) else { unreachable!() };
                    let mut s = String::with_capacity(x.len() + y.len());
                    s.push_str(x);
                    s.push_str(y);
                    Self::clr(f, *a);
                    Self::clr(f, *b);
                    f.slots[*d as usize] = Value::string(s);
                }
                Ins::FStr(d, idx) => {
                    let mut s = String::new();
                    for p in &ch.fstrs[*idx as usize] {
                        match p {
                            FsPart::Lit(l) => s.push_str(l),
                            FsPart::Val(a, t) => {
                                write_value(&mut s, self.gr(ch, f, *a), Some(&ch.types[*t as usize]), prog, true);
                                Self::clr(f, *a);
                            }
                        }
                    }
                    f.slots[*d as usize] = Value::string(s);
                }
                Ins::IntToFloat(d, a, bits) => {
                    let Value::Int(i) = self.gr(ch, f, *a) else { unreachable!() };
                    f.slots[*d as usize] = Value::Float(round_float(i.to_f64(), *bits));
                }
                Ins::MkRange(d, s, e, k, incl) => {
                    let loc = ch.locs[pc - 1];
                    let (s, e, k) = (self.gi(ch, f, *s), self.gi(ch, f, *e), self.gi(ch, f, *k));
                    let (Some(s), Some(e), Some(k)) = (s, e, k) else { unreachable!("Tam(64) bekleniyordu") };
                    let Some(p) = range_plan(s, e, *incl, k) else { return rt(loc, "Adım sıfır olamaz.") };
                    f.slots[*d as usize] = crate::interp::range_list(p, loc)?;
                }
                Ins::MkList(d, base, n) => {
                    let items = Self::take_regs(f, *base, *n);
                    f.slots[*d as usize] = Value::List(Rc::new(ListV { items }));
                }
                Ins::LoadBu(d) => {
                    let v = self.bu_value(f, ch.locs[pc - 1])?;
                    f.slots[*d as usize] = v;
                }
                Ins::RunModule(m) => self.run_module(*m)?,
            }
        }
    }
}

/// Yazmaçta zaten küçük tam sayı varsa yerinde günceller (eski değeri yok etme kodu çalışmaz).
#[inline(always)]
fn set_int(slot: &mut Value, r: i64) {
    if let Value::Int(Int::S(x)) = slot {
        *x = r;
    } else {
        *slot = Value::Int(Int::S(r));
    }
}

#[inline(always)]
fn set_bool(slot: &mut Value, r: bool) {
    if let Value::Bool(x) = slot {
        *x = r;
    } else {
        *slot = Value::Bool(r);
    }
}

#[inline(always)]
fn cmp_i64(op: BinOp, x: i64, y: i64) -> bool {
    match op {
        BinOp::Eq => x == y,
        BinOp::Ne => x != y,
        BinOp::Lt => x < y,
        BinOp::Le => x <= y,
        BinOp::Gt => x > y,
        BinOp::Ge => x >= y,
        _ => unreachable!(),
    }
}

