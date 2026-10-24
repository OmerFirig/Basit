// Parser: token listesinden sözdizimi ağacı (AST) kurar.

use crate::ast::*;
use crate::diag::{err, CResult, Span};
use crate::lexer::{describe, kw_text, FPart, Kw, Lexer, Mode, Tok, Token};
use std::mem::discriminant;

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    mode: Mode,
    file: u32,
}

pub fn parse_file(toks: Vec<Token>, mode: Mode, file: u32) -> CResult<Vec<Stmt>> {
    let mut p = Parser { toks, pos: 0, mode, file };
    let mut out = Vec::new();
    while !p.at(&Tok::Eof) {
        if p.eat(&Tok::Semi) {
            continue;
        }
        out.push(p.stmt()?);
        p.end_stmt()?;
    }
    Ok(out)
}

fn starts_upper(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_uppercase())
}

impl Parser {
    fn tok(&self) -> &Token {
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }
    fn peek(&self) -> &Tok {
        &self.tok().tok
    }
    fn peek_n(&self, n: usize) -> &Token {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)]
    }
    fn span(&self) -> Span {
        self.tok().span
    }
    fn prev_span(&self) -> Span {
        self.toks[self.pos.saturating_sub(1)].span
    }
    fn advance(&mut self) -> Token {
        let t = self.tok().clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn at(&self, t: &Tok) -> bool {
        discriminant(self.peek()) == discriminant(t)
    }
    fn at_kw(&self, k: Kw) -> bool {
        matches!(self.peek(), Tok::Kw(x) if *x == k)
    }
    fn eat(&mut self, t: &Tok) -> bool {
        if self.at(t) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn eat_kw(&mut self, k: Kw) -> bool {
        if self.at_kw(k) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn found(&self) -> String {
        describe(self.peek(), self.mode)
    }
    fn kw(&self, k: Kw) -> &'static str {
        kw_text(k, self.mode)
    }
    fn expect(&mut self, t: &Tok, what: &str) -> CResult<Span> {
        if self.at(t) {
            Ok(self.advance().span)
        } else {
            err(self.span(), format!("{what} bekleniyordu, ama {} bulundu.", self.found()))
        }
    }
    fn expect_kw(&mut self, k: Kw, hint: &str) -> CResult<Span> {
        if self.at_kw(k) {
            Ok(self.advance().span)
        } else {
            let mut msg = format!("'{}' bekleniyordu, ama {} bulundu.", self.kw(k), self.found());
            if !hint.is_empty() {
                msg.push(' ');
                msg.push_str(hint);
            }
            err(self.span(), msg)
        }
    }
    fn ident(&mut self, what: &str) -> CResult<(String, Span)> {
        match self.peek().clone() {
            Tok::Ident(s) => Ok((s, self.advance().span)),
            Tok::Kw(k) => err(
                self.span(),
                format!("{what} bekleniyordu, ama '{}' bir anahtar kelime ve isim olarak kullanılamaz.", self.kw(k)),
            ),
            _ => err(self.span(), format!("{what} bekleniyordu, ama {} bulundu.", self.found())),
        }
    }

    fn end_stmt(&mut self) -> CResult<()> {
        if self.eat(&Tok::Semi) {
            while self.eat(&Tok::Semi) {}
            return Ok(());
        }
        if self.at(&Tok::RBrace) || self.at(&Tok::Eof) || self.tok().nl_before {
            return Ok(());
        }
        err(
            self.span(),
            format!(
                "Beklenmeyen {}. Komutları ayırmak için yeni satıra geçin ya da ';' kullanın.",
                self.found()
            ),
        )
    }

    fn block(&mut self) -> CResult<Block> {
        let open = self.expect(&Tok::LBrace, "'{'")?;
        let mut out = Vec::new();
        loop {
            if self.eat(&Tok::Semi) {
                continue;
            }
            if self.eat(&Tok::RBrace) {
                return Ok(out);
            }
            if self.at(&Tok::Eof) {
                return err(open, "'{' açıldı ama '}' ile kapatılmadı.");
            }
            out.push(self.stmt()?);
            self.end_stmt()?;
        }
    }

    fn loop_body(&mut self) -> CResult<Block> {
        if !self.at_kw(Kw::Yap) {
            return err(
                self.span(),
                format!("Döngü gövdesi '{} {{ }}' ile başlamalı, ama {} bulundu.", self.kw(Kw::Yap), self.found()),
            );
        }
        self.advance();
        self.block()
    }

    // ---------------------------------------------------------------- komutlar

    fn stmt(&mut self) -> CResult<Stmt> {
        let start = self.span();
        let kind = match self.peek().clone() {
            Tok::Kw(Kw::Ata) => return self.ata(),
            Tok::Kw(Kw::Sabit) => {
                return err(start, format!("Sabit tanımı '{} {}' ile başlar: ata sabit x = 5", self.kw(Kw::Ata), self.kw(Kw::Sabit)));
            }
            Tok::Kw(Kw::Fonksiyon) => {
                self.advance();
                StmtKind::Fonksiyon(self.fn_rest(start, Yeniden::Yok)?)
            }
            Tok::Kw(Kw::Tur) => StmtKind::Tur(self.tur()?),
            Tok::Kw(Kw::Arayuz) => StmtKind::Arayuz(self.arayuz()?),
            Tok::Kw(Kw::Secenek) => StmtKind::Secenek(self.secenek()?),
            Tok::Kw(Kw::Eger) => self.eger()?,
            Tok::Kw(Kw::Dur) => {
                self.advance();
                StmtKind::Dur
            }
            Tok::Kw(Kw::Devam) => {
                self.advance();
                StmtKind::Devam
            }
            Tok::Kw(Kw::Dondur) => {
                self.advance();
                if self.at(&Tok::RBrace) || self.at(&Tok::Eof) || self.at(&Tok::Semi) || self.tok().nl_before {
                    StmtKind::Dondur(None)
                } else {
                    StmtKind::Dondur(Some(self.expr()?))
                }
            }
            Tok::Kw(Kw::Eslestir) => StmtKind::Match(self.match_expr()?),
            Tok::Kw(Kw::Dahil) => self.dahil(None)?,
            Tok::Kw(Kw::Yeniden) => {
                return err(start, format!("'{}' sadece bir tür içinde kullanılabilir.", self.kw(Kw::Yeniden)));
            }
            Tok::Ident(name) if name == "klavye" && self.peek_n(1).tok == Tok::LParen => {
                return err(start, "'klavye(...)' sadece dosyanın ilk satırında kullanılabilir.");
            }
            Tok::Directive(d) => {
                if d != "Güvensiz" {
                    return err(start, format!("Tanınmayan yönerge '#{d}'. Şu an sadece #Güvensiz var."));
                }
                self.advance();
                let is_dis = match (self.peek(), &self.peek_n(1).tok, &self.peek_n(2).tok) {
                    (Tok::Ident(d), Tok::Ident(k), _) if d == "dış" && k == "kütüphane" => true,
                    (Tok::Str(_), Tok::Ident(k), _) if k == "kütüphanesindeki" => true,
                    (Tok::Ident(_), Tok::Ident(a), Tok::Ident(d)) if a == "adıyla" && d == "dış" => true,
                    _ => false,
                };
                if !is_dis {
                    return err(start, "#Güvensiz sadece dış kütüphane tanımlarının önüne yazılır.");
                }
                return self.dis();
            }
            Tok::Ident(name) if name == "dış" && matches!(&self.peek_n(1).tok, Tok::Ident(a) if a == "kütüphane") => {
                return self.unsafe_needed(start);
            }
            Tok::Str(_) if matches!(&self.peek_n(1).tok, Tok::Ident(a) if a == "kütüphanesindeki") => {
                return self.unsafe_needed(start);
            }
            Tok::Ident(_)
                if matches!(&self.peek_n(1).tok, Tok::Ident(a) if a == "adıyla")
                    && !self.peek_n(1).nl_before
                    && matches!(&self.peek_n(2).tok, Tok::Ident(d) if d == "dış") =>
            {
                return self.unsafe_needed(start);
            }
            Tok::Ident(name)
                if matches!(&self.peek_n(1).tok, Tok::Ident(a) if a == "adıyla") && !self.peek_n(1).nl_before =>
            {
                let span = self.advance().span;
                self.advance();
                self.dahil(Some((name, span)))?
            }
            Tok::Ident(name) if self.peek_n(1).tok == Tok::Colon && !self.peek_n(1).nl_before => {
                let name_span = self.advance().span;
                self.advance();
                let ty = self.ty()?;
                self.expect(&Tok::Assign, "'='")?;
                let value = self.expr()?;
                StmtKind::Ata { sabit: false, explicit_ata: false, name, name_span, ty: Some(ty), value: Some(value) }
            }
            Tok::LParen if self.peek_n(1).tok == Tok::Kw(Kw::Ata) => self.cfor()?,
            _ => {
                let e = self.expr()?;
                match self.peek().clone() {
                    Tok::Kw(Kw::Kere) => {
                        self.advance();
                        StmtKind::Kere { count: e, body: self.loop_body()? }
                    }
                    Tok::Kw(Kw::Icindeki) => {
                        self.advance();
                        self.expect_kw(Kw::Her, "Örnek: liste içindeki her x için yap { }")?;
                        let pat = self.her_pattern()?;
                        self.expect_kw(Kw::Icin, "Örnek: liste içindeki her x için yap { }")?;
                        StmtKind::Her { iter: e, pat, body: self.loop_body()? }
                    }
                    Tok::Kw(Kw::Surece) => {
                        self.advance();
                        StmtKind::Surece { cond: e, body: self.loop_body()? }
                    }
                    Tok::Kw(Kw::Yap) => {
                        return err(
                            self.span(),
                            format!(
                                "'{}' öncesinde döngü türü eksik: '5 kere yap', 'koşul sürece yap' ya da 'liste içindeki her x için yap' yazın.",
                                self.kw(Kw::Yap)
                            ),
                        );
                    }
                    _ => return self.finish_simple(e, start),
                }
            }
        };
        Ok(Stmt { kind, span: start.to(self.prev_span()) })
    }

    fn finish_simple(&mut self, e: Expr, start: Span) -> CResult<Stmt> {
        let op = match self.peek() {
            Tok::Assign => Some(None),
            Tok::PlusEq => Some(Some(BinOp::Add)),
            Tok::MinusEq => Some(Some(BinOp::Sub)),
            Tok::StarEq => Some(Some(BinOp::Mul)),
            Tok::SlashEq => Some(Some(BinOp::Div)),
            Tok::StarStarEq => Some(Some(BinOp::Pow)),
            Tok::AmpEq => Some(Some(BinOp::BitAnd)),
            Tok::PipeEq => Some(Some(BinOp::BitOr)),
            Tok::CaretEq => Some(Some(BinOp::BitXor)),
            Tok::ShlEq => Some(Some(BinOp::Shl)),
            Tok::ShrEq => Some(Some(BinOp::Shr)),
            _ => None,
        };
        let kind = if let Some(op) = op {
            self.advance();
            let value = self.expr()?;
            StmtKind::Assign { target: e, op, value }
        } else {
            StmtKind::Expr(e)
        };
        Ok(Stmt { kind, span: start.to(self.prev_span()) })
    }

    fn ata(&mut self) -> CResult<Stmt> {
        let start = self.advance().span;
        let sabit = self.eat_kw(Kw::Sabit);
        let (name, name_span) = self.ident("Değişken adı")?;
        let ty = if self.eat(&Tok::Colon) { Some(self.ty()?) } else { None };
        let value = if self.eat(&Tok::Assign) { Some(self.expr()?) } else { None };
        if ty.is_none() && value.is_none() {
            return err(
                name_span,
                format!("'{name}' için bir tip ya da değer yazılmalı. Örnek: ata {name}: Tam  ya da  ata {name} = 0"),
            );
        }
        Ok(Stmt {
            kind: StmtKind::Ata { sabit, explicit_ata: true, name, name_span, ty, value },
            span: start.to(self.prev_span()),
        })
    }

    /// #Güvensiz yazılmadan dış kütüphane tanımı
    fn unsafe_needed(&self, start: Span) -> CResult<Stmt> {
        err(
            start,
            "Dış kütüphane kullanımı güvensizdir: C fonksiyonlarının imzaları kontrol edilemez, yanlış imza programı çökertebilir. Tanımın önüne #Güvensiz yazın.",
        )
    }

    /// `#Güvensiz` sonrası: üç yazımdan biri
    ///   dış kütüphane("m") { fonksiyon cos(x: Ondalık) -> Ondalık }
    ///   mat adıyla dış kütüphane("m") { ... }
    ///   "m" kütüphanesindeki fonksiyon cos(x: Ondalık) -> Ondalık
    fn dis(&mut self) -> CResult<Stmt> {
        let start = self.span();
        let alias = match (self.peek().clone(), &self.peek_n(1).tok) {
            (Tok::Ident(n), Tok::Ident(a)) if a == "adıyla" => {
                let s = self.advance().span;
                self.advance();
                Some((n, s))
            }
            _ => None,
        };
        if let Tok::Str(lib) = self.peek().clone() {
            if alias.is_none() {
                let lib_span = self.advance().span;
                self.advance(); // kütüphanesindeki
                self.expect_kw(Kw::Fonksiyon, "Örnek: \"m\" kütüphanesindeki fonksiyon cos(x: Ondalık) -> Ondalık")?;
                let f = self.dis_fn()?;
                return Ok(Stmt { kind: StmtKind::Dis { alias: None, lib, lib_span, funcs: vec![f] }, span: start.to(self.prev_span()) });
            }
        }
        match self.peek() {
            Tok::Ident(d) if d == "dış" => {
                self.advance();
            }
            _ => return err(self.span(), "'dış kütüphane(\"...\")' bekleniyordu."),
        }
        match self.peek() {
            Tok::Ident(k) if k == "kütüphane" => {
                self.advance();
            }
            _ => return err(self.span(), "'dış kütüphane(\"...\")' bekleniyordu."),
        }
        self.expect(&Tok::LParen, "'('")?;
        let (lib, lib_span) = match self.peek().clone() {
            Tok::Str(s) => (s, self.advance().span),
            _ => return err(self.span(), "Kütüphane adı tırnak içinde yazılmalı: dış kütüphane(\"m\")"),
        };
        self.expect(&Tok::RParen, "')'")?;
        self.expect(&Tok::LBrace, "'{'")?;
        let mut funcs = Vec::new();
        loop {
            if self.eat(&Tok::Semi) {
                continue;
            }
            if self.eat(&Tok::RBrace) {
                break;
            }
            self.expect_kw(Kw::Fonksiyon, "Dış kütüphane içinde sadece fonksiyon imzaları yazılır.")?;
            funcs.push(self.dis_fn()?);
            self.end_stmt()?;
        }
        Ok(Stmt { kind: StmtKind::Dis { alias, lib, lib_span, funcs }, span: start.to(self.prev_span()) })
    }

    fn dis_fn(&mut self) -> CResult<DisFn> {
        let (name, span) = self.ident("Fonksiyon adı")?;
        let params = self.params("Parametrelerin")?;
        let ret = if self.eat(&Tok::Arrow) { Some(self.ty()?) } else { None };
        let symbol = if self.eat(&Tok::Assign) {
            match self.peek().clone() {
                Tok::Str(s) => {
                    self.advance();
                    Some(s)
                }
                _ => return err(self.span(), "C'deki ad tırnak içinde yazılmalı: = \"sqrt\""),
            }
        } else {
            None
        };
        if self.at(&Tok::LBrace) && !self.tok().nl_before {
            return err(self.span(), "Dış fonksiyonların gövdesi olmaz, sadece imzası yazılır.");
        }
        Ok(DisFn { name, span, params, ret, symbol })
    }

    fn dahil(&mut self, alias: Option<(String, Span)>) -> CResult<StmtKind> {
        self.expect_kw(Kw::Dahil, "Örnek: m adıyla dahil et(\"./dosya.kolay\")")?;
        match self.peek() {
            Tok::Ident(s) if s == "et" => {
                self.advance();
            }
            _ => return err(self.span(), "'dahil et(\"./dosya.kolay\")' şeklinde yazılmalı."),
        }
        self.expect(&Tok::LParen, "'('")?;
        let path = match self.peek().clone() {
            Tok::Str(s) => {
                self.advance();
                s
            }
            _ => return err(self.span(), "Dosya yolu tırnak içinde yazılmalı: dahil et(\"./dosya.kolay\")"),
        };
        self.expect(&Tok::RParen, "')'")?;
        Ok(StmtKind::Dahil { path, alias })
    }

    fn cfor(&mut self) -> CResult<StmtKind> {
        self.expect(&Tok::LParen, "'('")?;
        let init = self.ata()?;
        self.expect(&Tok::Semi, "C tarzı döngüde başlangıçtan sonra ';'")?;
        let cond = self.expr()?;
        self.expect(&Tok::Semi, "C tarzı döngüde koşuldan sonra ';'")?;
        let step_start = self.span();
        let e = self.expr()?;
        let step = self.finish_simple(e, step_start)?;
        self.expect(&Tok::RParen, "')'")?;
        self.expect_kw(Kw::Surece, "Örnek: (ata i = 0; i < 10; i += 1) sürece yap { }")?;
        let body = self.loop_body()?;
        Ok(StmtKind::CFor { init: Box::new(init), cond, step: Box::new(step), body })
    }

    fn her_pattern(&mut self) -> CResult<Vec<(String, Span)>> {
        if self.eat(&Tok::LParen) {
            let mut v = vec![self.ident("Değişken adı")?];
            while self.eat(&Tok::Comma) {
                v.push(self.ident("Değişken adı")?);
            }
            self.expect(&Tok::RParen, "')'")?;
            Ok(v)
        } else {
            Ok(vec![self.ident("Döngü değişkeninin adı")?])
        }
    }

    fn eger(&mut self) -> CResult<StmtKind> {
        self.advance();
        let mut branches = vec![(self.expr()?, self.block()?)];
        let mut else_ = None;
        while self.at_kw(Kw::Degilse) {
            self.advance();
            if self.eat_kw(Kw::Eger) {
                branches.push((self.expr()?, self.block()?));
            } else {
                else_ = Some(self.block()?);
                break;
            }
        }
        Ok(StmtKind::Eger { branches, else_ })
    }

    fn params(&mut self, what: &str) -> CResult<Vec<Param>> {
        self.expect(&Tok::LParen, "'('")?;
        let mut out = Vec::new();
        while !self.at(&Tok::RParen) {
            let start = self.span();
            if self.at_kw(Kw::Ata) {
                return err(start, format!("{what} başına '{}' yazılmaz. Örnek: (x: Tam)", self.kw(Kw::Ata)));
            }
            let sabit = self.eat_kw(Kw::Sabit);
            let (name, span) = self.ident("İsim")?;
            if !self.at(&Tok::Colon) {
                return err(self.span(), format!("'{name}' için tip yazılmalı. Örnek: {name}: Tam"));
            }
            self.advance();
            let ty = self.ty()?;
            out.push(Param { name, ty, sabit, span: span.to(self.prev_span()).to(start) });
            out.last_mut().unwrap().span = span;
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        self.expect(&Tok::RParen, "')'")?;
        Ok(out)
    }

    fn fn_rest(&mut self, start: Span, yeniden: Yeniden) -> CResult<FnDecl> {
        let (name, _) = self.ident("Fonksiyon adı")?;
        let params = self.params("Parametrelerin")?;
        let ret = if self.eat(&Tok::Arrow) { Some(self.ty()?) } else { None };
        let body = self.block()?;
        Ok(FnDecl { name, span: start, params, ret, body, yeniden })
    }

    fn type_name(&mut self, what: &str) -> CResult<(String, Span)> {
        let (name, span) = self.ident(what)?;
        if !starts_upper(&name) {
            let mut chars = name.chars();
            let first = chars.next().unwrap();
            let fixed: String = turkish_upper_char(first) + chars.as_str();
            return err(span, format!("{what} büyük harfle başlamalı: '{fixed}' gibi."));
        }
        Ok((name, span))
    }

    fn path_list(&mut self) -> CResult<Vec<(Vec<String>, Span)>> {
        self.expect(&Tok::LBrack, "'['")?;
        let mut out = Vec::new();
        while !self.at(&Tok::RBrack) {
            let (first, span) = self.ident("Tür adı")?;
            let mut path = vec![first];
            while self.eat(&Tok::Dot) {
                path.push(self.ident("Tür adı")?.0);
            }
            out.push((path, span.to(self.prev_span())));
            if !self.eat(&Tok::Comma) {
                break;
            }
        }
        self.expect(&Tok::RBrack, "']'")?;
        Ok(out)
    }

    fn tur(&mut self) -> CResult<TurDecl> {
        let start = self.advance().span;
        let (name, _) = self.type_name("Tür adı")?;
        let parents = if self.at(&Tok::LBrack) { self.path_list()? } else { Vec::new() };
        let fields = if self.at(&Tok::LParen) { self.params("Alanların")? } else { Vec::new() };
        if self.at(&Tok::LBrack) {
            return err(
                self.span(),
                format!("Ebeveynler alanlardan önce yazılır: {} {name}[Ebeveyn](alan: Tam)", self.kw(Kw::Tur)),
            );
        }
        let mut methods = Vec::new();
        if self.at(&Tok::LBrace) {
            self.advance();
            loop {
                if self.eat(&Tok::Semi) {
                    continue;
                }
                if self.eat(&Tok::RBrace) {
                    break;
                }
                methods.push(self.method()?);
                self.end_stmt()?;
            }
        }
        Ok(TurDecl { name, span: start, parents, fields, methods })
    }

    fn method(&mut self) -> CResult<FnDecl> {
        let start = self.span();
        match self.peek() {
            Tok::Kw(Kw::Fonksiyon) => {
                self.advance();
                self.fn_rest(start, Yeniden::Yok)
            }
            Tok::Kw(Kw::Yeniden) => {
                self.advance();
                if self.at_kw(Kw::Fonksiyon) {
                    return err(
                        self.span(),
                        format!("'{}' kelimesinden sonra '{}' yazılmaz. Örnek: yeniden tanıt() {{ }}", self.kw(Kw::Yeniden), self.kw(Kw::Fonksiyon)),
                    );
                }
                if matches!(self.peek_n(1).tok, Tok::Dot) {
                    let (parent, pspan) = self.ident("Ebeveyn adı")?;
                    self.advance();
                    self.fn_rest(start, Yeniden::Ebeveyn(parent, pspan))
                } else {
                    self.fn_rest(start, Yeniden::Genel)
                }
            }
            Tok::Eof => err(self.span(), "'{' açıldı ama '}' ile kapatılmadı."),
            _ => err(
                self.span(),
                format!(
                    "Tür içinde sadece '{}' ya da '{}' yazılabilir, ama {} bulundu. Alanlar tür adından sonra parantez içinde yazılır.",
                    self.kw(Kw::Fonksiyon),
                    self.kw(Kw::Yeniden),
                    self.found()
                ),
            ),
        }
    }

    fn arayuz(&mut self) -> CResult<ArayuzDecl> {
        let start = self.advance().span;
        let (name, _) = self.type_name("Arayüz adı")?;
        self.expect(&Tok::LBrace, "'{'")?;
        let mut sigs = Vec::new();
        loop {
            if self.eat(&Tok::Semi) {
                continue;
            }
            if self.eat(&Tok::RBrace) {
                break;
            }
            let s = self.expect_kw(Kw::Fonksiyon, "Arayüz içinde sadece fonksiyon imzaları yazılır.")?;
            let (fname, _) = self.ident("Fonksiyon adı")?;
            let params = self.params("Parametrelerin")?;
            let ret = if self.eat(&Tok::Arrow) { Some(self.ty()?) } else { None };
            if self.at(&Tok::LBrace) && !self.tok().nl_before {
                return err(self.span(), "Arayüzdeki fonksiyonların gövdesi olmaz, sadece imzası yazılır.");
            }
            sigs.push(FnSig { name: fname, span: s, params, ret });
            self.end_stmt()?;
        }
        Ok(ArayuzDecl { name, span: start, sigs })
    }

    fn secenek(&mut self) -> CResult<SecenekDecl> {
        let start = self.advance().span;
        let (name, _) = self.type_name("Seçenek adı")?;
        let interfaces = if self.at(&Tok::LBrack) { self.path_list()? } else { Vec::new() };
        self.expect(&Tok::LBrace, "'{'")?;
        let mut variants = Vec::new();
        let mut methods = Vec::new();
        loop {
            if self.eat(&Tok::Semi) || self.eat(&Tok::Comma) {
                continue;
            }
            if self.eat(&Tok::RBrace) {
                break;
            }
            if self.at_kw(Kw::Fonksiyon) || self.at_kw(Kw::Yeniden) {
                methods.push(self.method()?);
            } else {
                let (vname, span) = self.type_name("Seçenek adı")?;
                let fields = if self.at(&Tok::LParen) && !self.tok().nl_before { self.params("Alanların")? } else { Vec::new() };
                variants.push(VariantDecl { name: vname, span, fields });
            }
            if !self.at(&Tok::Comma) {
                self.end_stmt()?;
            }
        }
        if variants.is_empty() {
            return err(start, format!("'{name}' en az bir seçenek içermeli."));
        }
        Ok(SecenekDecl { name, span: start, interfaces, variants, methods })
    }

    // ---------------------------------------------------------------- tipler

    pub fn ty(&mut self) -> CResult<TypeExpr> {
        let first = self.type_atom()?;
        if !self.at_kw(Kw::Veya) {
            return Ok(first);
        }
        let start = first.span;
        let mut parts = vec![first];
        while self.eat_kw(Kw::Veya) {
            parts.push(self.type_atom()?);
        }
        Ok(TypeExpr { kind: TypeKind::Union(parts), span: start.to(self.prev_span()) })
    }

    fn type_atom(&mut self) -> CResult<TypeExpr> {
        let start = self.span();
        let mut path = Vec::new();
        if self.eat_kw(Kw::Hic) {
            path.push("Hiç".to_string());
        } else {
            path.push(self.ident("Tip adı")?.0);
            while self.at(&Tok::Dot) && matches!(self.peek_n(1).tok, Tok::Ident(_)) {
                self.advance();
                path.push(self.ident("Tip adı")?.0);
            }
        }
        let mut generics = Vec::new();
        if self.at(&Tok::Lt) {
            self.advance();
            loop {
                generics.push(self.ty()?);
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
            match self.peek() {
                Tok::Gt => {
                    self.advance();
                }
                Tok::Ge => {
                    // "Liste<Tam>= [..]" yazımında '>=' tek token olarak gelir; ikiye böl.
                    let t = &mut self.toks[self.pos];
                    t.tok = Tok::Assign;
                }
                Tok::Shr => {
                    // "Liste<Liste<Tam>>": '>>' iki '>' demek; biri burada kullanılır
                    let t = &mut self.toks[self.pos];
                    t.tok = Tok::Gt;
                }
                Tok::ShrEq => {
                    let t = &mut self.toks[self.pos];
                    t.tok = Tok::Ge;
                }
                _ => return err(self.span(), format!("'>' bekleniyordu, ama {} bulundu.", self.found())),
            }
        }
        let mut args = None;
        if self.at(&Tok::LParen) && !self.tok().nl_before {
            self.advance();
            let mut v = Vec::new();
            while !self.at(&Tok::RParen) {
                match self.peek().clone() {
                    Tok::Int(n) => {
                        let s = self.advance().span;
                        let n: u32 = n.try_into().map_err(|_| crate::diag::Diag::new(s, "Bit sayısı çok büyük."))?;
                        v.push(TypeArg::Num(n));
                    }
                    Tok::Plus => {
                        self.advance();
                        v.push(TypeArg::Plus);
                    }
                    Tok::Minus => {
                        self.advance();
                        v.push(TypeArg::Minus);
                    }
                    _ => {
                        return err(self.span(), format!("Tip parantezi içinde bit sayısı, '+' ya da '-' bekleniyordu, ama {} bulundu.", self.found()));
                    }
                }
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
            self.expect(&Tok::RParen, "')'")?;
            args = Some(v);
        }
        let mut dims = Vec::new();
        while self.at(&Tok::LBrack) && !self.tok().nl_before {
            self.advance();
            match self.peek().clone() {
                Tok::Int(n) => {
                    let s = self.advance().span;
                    let n: usize = n.try_into().map_err(|_| crate::diag::Diag::new(s, "Liste boyutu çok büyük."))?;
                    dims.push(Some(n));
                }
                _ => dims.push(None),
            }
            self.expect(&Tok::RBrack, "']'")?;
        }
        Ok(TypeExpr { kind: TypeKind::Path { path, generics, args, dims }, span: start.to(self.prev_span()) })
    }

    // ---------------------------------------------------------------- ifadeler

    pub fn expr(&mut self) -> CResult<Expr> {
        self.or_expr()
    }

    fn bin(l: Expr, op: BinOp, r: Expr) -> Expr {
        let span = l.span.to(r.span);
        Expr { kind: ExprKind::Binary(op, Box::new(l), Box::new(r)), span }
    }

    fn or_expr(&mut self) -> CResult<Expr> {
        let mut l = self.and_expr()?;
        while (self.at_kw(Kw::Veya) || self.at(&Tok::OrOr)) && !self.tok().nl_before {
            self.advance();
            let r = self.and_expr()?;
            l = Self::bin(l, BinOp::Or, r);
        }
        Ok(l)
    }

    fn and_expr(&mut self) -> CResult<Expr> {
        let mut l = self.not_expr()?;
        while (self.at_kw(Kw::Ve) || self.at(&Tok::AndAnd)) && !self.tok().nl_before {
            self.advance();
            let r = self.not_expr()?;
            l = Self::bin(l, BinOp::And, r);
        }
        Ok(l)
    }

    fn not_expr(&mut self) -> CResult<Expr> {
        if self.at_kw(Kw::Degil) || self.at(&Tok::Bang) {
            let start = self.advance().span;
            let e = self.not_expr()?;
            let span = start.to(e.span);
            return Ok(Expr { kind: ExprKind::Not(Box::new(e)), span });
        }
        self.cmp_expr()
    }

    fn cmp_op(&self) -> Option<BinOp> {
        if self.tok().nl_before {
            return None;
        }
        Some(match self.peek() {
            Tok::EqEq => BinOp::Eq,
            Tok::Ne => BinOp::Ne,
            Tok::Lt => BinOp::Lt,
            Tok::Le => BinOp::Le,
            Tok::Gt => BinOp::Gt,
            Tok::Ge => BinOp::Ge,
            _ => return None,
        })
    }

    fn cmp_expr(&mut self) -> CResult<Expr> {
        let l = self.range_expr()?;
        if let Some(op) = self.cmp_op() {
            self.advance();
            let r = self.range_expr()?;
            if self.cmp_op().is_some() {
                return err(
                    self.span(),
                    format!("Karşılaştırmalar zincirlenemez. 'a < b < c' yerine 'a < b {} b < c' yazın.", self.kw(Kw::Ve)),
                );
            }
            return Ok(Self::bin(l, op, r));
        }
        Ok(l)
    }

    fn can_start_expr(&self) -> bool {
        if self.tok().nl_before {
            return false;
        }
        matches!(
            self.peek(),
            Tok::Int(_)
                | Tok::Float(_)
                | Tok::Str(_)
                | Tok::FStr(_)
                | Tok::Ident(_)
                | Tok::LParen
                | Tok::LBrack
                | Tok::Minus
                | Tok::Tilde
                | Tok::Kw(Kw::Bu)
        )
    }

    fn range_expr(&mut self) -> CResult<Expr> {
        if self.at(&Tok::DotDot) || self.at(&Tok::DotDotEq) {
            let t = self.advance();
            let inclusive = t.tok == Tok::DotDotEq;
            let end = if self.can_start_expr() { Some(Box::new(self.bitor_expr()?)) } else { None };
            let span = t.span.to(self.prev_span());
            return Ok(Expr { kind: ExprKind::Range { start: None, end, inclusive }, span });
        }
        let l = self.bitor_expr()?;
        if (self.at(&Tok::DotDot) || self.at(&Tok::DotDotEq)) && !self.tok().nl_before {
            let t = self.advance();
            let inclusive = t.tok == Tok::DotDotEq;
            let end = if self.can_start_expr() { Some(Box::new(self.bitor_expr()?)) } else { None };
            if inclusive && end.is_none() {
                return err(t.span, "'..=' sonrasında bir bitiş değeri yazılmalı.");
            }
            let span = l.span.to(self.prev_span());
            return Ok(Expr { kind: ExprKind::Range { start: Some(Box::new(l)), end, inclusive }, span });
        }
        Ok(l)
    }

    /// Bit işlemleri (Python'daki öncelik sırasıyla): | < ^ < & < kaydırma < toplama
    fn bin_level(&mut self, ops: &[(Tok, BinOp)], next: fn(&mut Self) -> CResult<Expr>) -> CResult<Expr> {
        let mut l = next(self)?;
        loop {
            if self.tok().nl_before {
                break;
            }
            let Some(op) = ops.iter().find(|(t, _)| self.at(t)).map(|x| x.1) else { break };
            self.advance();
            let r = next(self)?;
            l = Self::bin(l, op, r);
        }
        Ok(l)
    }

    fn bitor_expr(&mut self) -> CResult<Expr> {
        self.bin_level(&[(Tok::Pipe, BinOp::BitOr)], Self::bitxor_expr)
    }

    fn bitxor_expr(&mut self) -> CResult<Expr> {
        self.bin_level(&[(Tok::Caret, BinOp::BitXor)], Self::bitand_expr)
    }

    fn bitand_expr(&mut self) -> CResult<Expr> {
        self.bin_level(&[(Tok::Amp, BinOp::BitAnd)], Self::shift_expr)
    }

    fn shift_expr(&mut self) -> CResult<Expr> {
        self.bin_level(&[(Tok::Shl, BinOp::Shl), (Tok::Shr, BinOp::Shr)], Self::add_expr)
    }

    fn add_expr(&mut self) -> CResult<Expr> {
        let mut l = self.mul_expr()?;
        loop {
            if self.tok().nl_before {
                break;
            }
            let op = match self.peek() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let r = self.mul_expr()?;
            l = Self::bin(l, op, r);
        }
        Ok(l)
    }

    fn mul_expr(&mut self) -> CResult<Expr> {
        let mut l = self.olarak_expr()?;
        loop {
            if self.tok().nl_before {
                break;
            }
            let op = match self.peek() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                Tok::Percent => BinOp::Rem,
                _ => break,
            };
            self.advance();
            let r = self.olarak_expr()?;
            l = Self::bin(l, op, r);
        }
        Ok(l)
    }

    fn olarak_expr(&mut self) -> CResult<Expr> {
        let mut e = self.unary()?;
        while self.at_kw(Kw::Olarak) && !self.tok().nl_before {
            self.advance();
            let t = self.type_atom()?;
            let span = e.span.to(t.span);
            e = Expr { kind: ExprKind::Olarak(Box::new(e), t), span };
        }
        Ok(e)
    }

    fn unary(&mut self) -> CResult<Expr> {
        if self.at(&Tok::Tilde) {
            let start = self.advance().span;
            let e = self.unary()?;
            let span = start.to(e.span);
            return Ok(Expr { kind: ExprKind::BitNot(Box::new(e)), span });
        }
        if self.at(&Tok::Minus) {
            let start = self.advance().span;
            let e = self.unary()?;
            let span = start.to(e.span);
            return Ok(match e.kind {
                ExprKind::Int(n) => Expr { kind: ExprKind::Int(-n), span },
                ExprKind::Float(f) => Expr { kind: ExprKind::Float(-f), span },
                _ => Expr { kind: ExprKind::Neg(Box::new(e)), span },
            });
        }
        self.pow_expr()
    }

    /// `a ** b`: sağdan birleşir ve tekli eksiden önce gelir (Python'daki gibi: -2 ** 2 = -4)
    fn pow_expr(&mut self) -> CResult<Expr> {
        let base = self.postfix()?;
        if self.at(&Tok::StarStar) && !self.tok().nl_before {
            self.advance();
            let exp = self.unary()?;
            return Ok(Self::bin(base, BinOp::Pow, exp));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> CResult<Expr> {
        let mut e = self.primary()?;
        loop {
            if self.at(&Tok::LParen) && !self.tok().nl_before {
                self.advance();
                let mut args = Vec::new();
                while !self.at(&Tok::RParen) {
                    let name = match (self.peek().clone(), &self.peek_n(1).tok) {
                        (Tok::Ident(n), Tok::Colon) => {
                            let s = self.advance().span;
                            self.advance();
                            Some((n, s))
                        }
                        _ => None,
                    };
                    args.push(Arg { name, value: self.expr()? });
                    if !self.eat(&Tok::Comma) {
                        break;
                    }
                }
                self.expect(&Tok::RParen, "')' ya da ','")?;
                let span = e.span.to(self.prev_span());
                e = Expr { kind: ExprKind::Call { callee: Box::new(e), args }, span };
            } else if self.at(&Tok::LBrack) && !self.tok().nl_before {
                self.advance();
                let idx = self.expr()?;
                self.expect(&Tok::RBrack, "']'")?;
                let span = e.span.to(self.prev_span());
                e = Expr { kind: ExprKind::Index(Box::new(e), Box::new(idx)), span };
            } else if self.at(&Tok::Dot) {
                self.advance();
                let (name, nspan) = self.ident("Alan ya da metot adı")?;
                let span = e.span.to(nspan);
                e = Expr { kind: ExprKind::Field(Box::new(e), name, nspan), span };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn primary(&mut self) -> CResult<Expr> {
        let t = self.advance();
        let span = t.span;
        let kind = match t.tok {
            Tok::Int(n) => ExprKind::Int(n),
            Tok::Float(f) => ExprKind::Float(f),
            Tok::Str(s) => ExprKind::Str(s),
            Tok::FStr(parts) => {
                let mut out = Vec::new();
                for p in parts {
                    match p {
                        FPart::Lit(s) => out.push(FPartE::Lit(s)),
                        FPart::Expr { src, line, col, byte } => {
                            let toks = Lexer::with_base(&src, self.file, self.mode, line, col, byte).tokenize()?;
                            let mut sub = Parser { toks, pos: 0, mode: self.mode, file: self.file };
                            let e = sub.expr()?;
                            if !sub.at(&Tok::Eof) {
                                return err(sub.span(), format!("f-yazı içindeki ifadede beklenmeyen {}.", sub.found()));
                            }
                            out.push(FPartE::Expr(e));
                        }
                    }
                }
                ExprKind::FStr(out)
            }
            Tok::Kw(Kw::Dogru) => ExprKind::Bool(true),
            Tok::Kw(Kw::Yanlis) => ExprKind::Bool(false),
            Tok::Kw(Kw::Hic) => ExprKind::Hic,
            Tok::Kw(Kw::Bu) => ExprKind::Bu,
            Tok::Ident(s) => ExprKind::Ident(s),
            Tok::LParen => {
                let e = self.expr()?;
                self.expect(&Tok::RParen, "')'")?;
                return Ok(Expr { kind: e.kind, span: span.to(self.prev_span()) });
            }
            Tok::LBrack => {
                let mut items = Vec::new();
                while !self.at(&Tok::RBrack) {
                    items.push(self.expr()?);
                    if !self.eat(&Tok::Comma) {
                        break;
                    }
                }
                self.expect(&Tok::RBrack, "']' ya da ','")?;
                ExprKind::List(items)
            }
            Tok::LBrace => {
                let mut items = Vec::new();
                while !self.at(&Tok::RBrace) {
                    let k = self.expr()?;
                    self.expect(&Tok::Colon, "Sözlükte anahtardan sonra ':'")?;
                    let v = self.expr()?;
                    items.push((k, v));
                    if !self.eat(&Tok::Comma) {
                        break;
                    }
                }
                self.expect(&Tok::RBrace, "'}' ya da ','")?;
                ExprKind::Dict(items)
            }
            Tok::Kw(Kw::Eslestir) => {
                self.pos -= 1;
                let m = self.match_expr()?;
                ExprKind::Match(Box::new(m))
            }
            other => {
                return err(span, format!("Bir değer bekleniyordu, ama {} bulundu.", describe(&other, self.mode)));
            }
        };
        Ok(Expr { kind, span: span.to(self.prev_span()) })
    }

    // ---------------------------------------------------------------- eşleştir

    fn match_expr(&mut self) -> CResult<Match> {
        let start = self.advance().span;
        let scrutinee = self.expr()?;
        self.expect(&Tok::LBrace, "'{'")?;
        let mut arms = Vec::new();
        loop {
            if self.eat(&Tok::Semi) || self.eat(&Tok::Comma) {
                continue;
            }
            if self.eat(&Tok::RBrace) {
                break;
            }
            if self.at(&Tok::Eof) {
                return err(self.span(), "'{' açıldı ama '}' ile kapatılmadı.");
            }
            let arm_start = self.span();
            let pats = if self.eat_kw(Kw::Degilse) {
                None
            } else {
                let mut v = vec![self.pattern()?];
                while self.eat_kw(Kw::Veya) {
                    v.push(self.pattern()?);
                }
                Some(v)
            };
            let guard = if self.eat_kw(Kw::Eger) { Some(self.expr()?) } else { None };
            self.expect(&Tok::Arrow, "'->'")?;
            let body = if self.at(&Tok::LBrace) {
                ArmBody::Block(self.block()?)
            } else if self.at_kw(Kw::Dur) || self.at_kw(Kw::Devam) || self.at_kw(Kw::Dondur) {
                ArmBody::Stmt(Box::new(self.stmt()?))
            } else {
                let start = self.span();
                let e = self.expr()?;
                // atama da yazılabilir: `Kırmızı -> sayaç += 1` (tek komutluk blok gibi)
                if matches!(self.peek(), Tok::Assign | Tok::PlusEq | Tok::MinusEq | Tok::StarEq | Tok::SlashEq) {
                    ArmBody::Block(vec![self.finish_simple(e, start)?])
                } else {
                    ArmBody::Expr(e)
                }
            };
            arms.push(Arm { pats, guard, body, span: arm_start.to(self.prev_span()) });
            if !self.at(&Tok::Comma) {
                self.end_stmt()?;
            }
        }
        Ok(Match { scrutinee, arms, span: start })
    }

    fn pat_int(&mut self) -> CResult<Option<num_bigint::BigInt>> {
        let neg = self.at(&Tok::Minus) && matches!(self.peek_n(1).tok, Tok::Int(_));
        if neg {
            self.advance();
        }
        if let Tok::Int(n) = self.peek().clone() {
            self.advance();
            return Ok(Some(if neg { -n } else { n }));
        }
        Ok(None)
    }

    fn pattern(&mut self) -> CResult<Pattern> {
        let start = self.span();
        if let Some(n) = self.pat_int()? {
            if self.at(&Tok::DotDot) || self.at(&Tok::DotDotEq) {
                let inclusive = self.advance().tok == Tok::DotDotEq;
                let Some(m) = self.pat_int()? else {
                    return err(self.span(), "Aralık deseninde bitiş sayısı bekleniyordu. Örnek: 0..=12");
                };
                return Ok(Pattern::Range(n, m, inclusive, start.to(self.prev_span())));
            }
            return Ok(Pattern::Int(n, start.to(self.prev_span())));
        }
        let t = self.advance();
        Ok(match t.tok {
            Tok::Minus if matches!(self.peek(), Tok::Float(_)) => {
                let Tok::Float(f) = self.advance().tok else { unreachable!() };
                Pattern::Float(-f, start.to(self.prev_span()))
            }
            Tok::Float(f) => Pattern::Float(f, t.span),
            Tok::Str(s) => Pattern::Str(s, t.span),
            Tok::Kw(Kw::Dogru) => Pattern::Bool(true, t.span),
            Tok::Kw(Kw::Yanlis) => Pattern::Bool(false, t.span),
            Tok::Kw(Kw::Hic) => Pattern::Hic(t.span),
            Tok::Ident(s) if s == "_" => Pattern::Wild,
            Tok::Ident(s) => {
                let mut path = vec![s];
                while self.eat(&Tok::Dot) {
                    path.push(self.ident("İsim")?.0);
                }
                let binds = if self.at(&Tok::LParen) {
                    self.advance();
                    let mut v = Vec::new();
                    while !self.at(&Tok::RParen) {
                        let (n, s) = self.ident("Değişken adı")?;
                        v.push(if n == "_" { None } else { Some((n, s)) });
                        if !self.eat(&Tok::Comma) {
                            break;
                        }
                    }
                    self.expect(&Tok::RParen, "')'")?;
                    Some(v)
                } else {
                    None
                };
                Pattern::Path { path, binds, span: start.to(self.prev_span()) }
            }
            other => {
                return err(t.span, format!("Desen bekleniyordu (sayı, yazı, seçenek ya da tip), ama {} bulundu.", describe(&other, self.mode)));
            }
        })
    }
}

pub fn turkish_upper_char(c: char) -> String {
    match c {
        'i' => "İ".into(),
        'ı' => "I".into(),
        _ => c.to_uppercase().collect(),
    }
}
