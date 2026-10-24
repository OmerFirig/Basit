// Lexer: kaynak kodu token'lara ayırır. Klavye modu (tr/us), yorumlar ve f-yazılar burada çözülür.

use crate::diag::{err, CResult, Diag, Span};
use num_bigint::BigInt;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Tr,
    Us,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Tr => "tr",
            Mode::Us => "us",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Kw {
    Ata,
    Sabit,
    Fonksiyon,
    Dondur,
    Eger,
    Degilse,
    Yap,
    Icindeki,
    Her,
    Icin,
    Surece,
    Kere,
    Dur,
    Devam,
    Dogru,
    Yanlis,
    Tur,
    Yeniden,
    Bu,
    Arayuz,
    Secenek,
    Eslestir,
    Veya,
    Ve,
    Degil,
    Dahil,
    Olarak,
    Hic,
}

/// (anahtar kelime, tr yazımı, us yazımı)
pub const KEYWORDS: &[(Kw, &str, &str)] = &[
    (Kw::Ata, "ata", "ata"),
    (Kw::Sabit, "sabit", "sabit"),
    (Kw::Fonksiyon, "fonksiyon", "fonksiyon"),
    (Kw::Dondur, "döndür", "dondur"),
    (Kw::Eger, "eğer", "eger"),
    (Kw::Degilse, "değilse", "degilse"),
    (Kw::Yap, "yap", "yap"),
    (Kw::Icindeki, "içindeki", "icindeki"),
    (Kw::Her, "her", "her"),
    (Kw::Icin, "için", "icin"),
    (Kw::Surece, "sürece", "surece"),
    (Kw::Kere, "kere", "kere"),
    (Kw::Dur, "dur", "dur"),
    (Kw::Devam, "devam", "devam"),
    (Kw::Dogru, "doğru", "dogru"),
    (Kw::Yanlis, "yanlış", "yanlis"),
    (Kw::Tur, "tür", "tur"),
    (Kw::Yeniden, "yeniden", "yeniden"),
    (Kw::Bu, "bu", "bu"),
    (Kw::Arayuz, "arayüz", "arayuz"),
    (Kw::Secenek, "seçenek", "secenek"),
    (Kw::Eslestir, "eşleştir", "eslestir"),
    (Kw::Veya, "veya", "veya"),
    (Kw::Ve, "ve", "ve"),
    (Kw::Degil, "değil", "degil"),
    (Kw::Dahil, "dahil", "dahil"),
    (Kw::Olarak, "olarak", "olarak"),
    (Kw::Hic, "Hiç", "Hic"),
];

/// Dilin hazır isimleri (tipler, fonksiyonlar, metotlar): (tr yazımı, us yazımı).
/// İki modda aynı yazılanlar (Tam, Liste, ekle...) burada yok.
pub const BUILTIN_NAMES: &[(&str, &str)] = &[
    ("Yazı", "Yazi"),
    ("Ondalık", "Ondalik"),
    ("Mantık", "Mantik"),
    ("Sözlük", "Sozluk"),
    ("yazdır", "yazdir"),
    ("çıkar", "cikar"),
    ("içerir", "icerir"),
    ("sırala", "sirala"),
    ("sıralı", "sirali"),
    ("ters_çevir", "ters_cevir"),
    ("dönüşüm", "donusum"),
    ("en_büyük", "en_buyuk"),
    ("en_küçük", "en_kucuk"),
    ("göre", "gore"),
    ("büyük_harf", "buyuk_harf"),
    ("küçük_harf", "kucuk_harf"),
    ("böl", "bol"),
    ("kırp", "kirp"),
    ("ondalığa", "ondaliga"),
    ("yazıya", "yaziya"),
    ("değerler", "degerler"),
    ("adım", "adim"),
    ("adıyla", "adiyla"),
    ("dış", "dis"),
    ("kütüphane", "kutuphane"),
    ("kütüphanesindeki", "kutuphanesindeki"),
    ("Güvensiz", "Guvensiz"),
];

pub fn kw_text(kw: Kw, mode: Mode) -> &'static str {
    for (k, tr, us) in KEYWORDS {
        if *k == kw {
            return if mode == Mode::Tr { tr } else { us };
        }
    }
    "?"
}

#[derive(Clone, Debug, PartialEq)]
pub enum FPart {
    Lit(String),
    Expr { src: String, line: u32, col: u32, byte: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Int(BigInt),
    Float(f64),
    Str(String),
    FStr(Vec<FPart>),
    Ident(String),
    Kw(Kw),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBrack,
    RBrack,
    Comma,
    Colon,
    Semi,
    Dot,
    DotDot,
    DotDotEq,
    Arrow,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    AndAnd,
    OrOr,
    Bang,
    // bit işlemleri ve üs
    Amp,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,
    StarStar,
    AmpEq,
    PipeEq,
    CaretEq,
    ShlEq,
    ShrEq,
    StarStarEq,
    /// `#Güvensiz` gibi yönerge (kelime, # olmadan)
    Directive(String),
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
    pub nl_before: bool,
    /// Kaynaktaki yazımı (çeviri aracı için).
    pub raw_start: u32,
    pub raw_end: u32,
}

pub fn describe(t: &Tok, mode: Mode) -> String {
    match t {
        Tok::Int(n) => format!("'{n}'"),
        Tok::Float(f) => format!("'{f}'"),
        Tok::Str(_) | Tok::FStr(_) => "yazı".into(),
        Tok::Ident(s) => format!("'{s}'"),
        Tok::Kw(k) => format!("'{}'", kw_text(*k, mode)),
        Tok::LParen => "'('".into(),
        Tok::RParen => "')'".into(),
        Tok::LBrace => "'{'".into(),
        Tok::RBrace => "'}'".into(),
        Tok::LBrack => "'['".into(),
        Tok::RBrack => "']'".into(),
        Tok::Comma => "','".into(),
        Tok::Colon => "':'".into(),
        Tok::Semi => "';'".into(),
        Tok::Dot => "'.'".into(),
        Tok::DotDot => "'..'".into(),
        Tok::DotDotEq => "'..='".into(),
        Tok::Arrow => "'->'".into(),
        Tok::Plus => "'+'".into(),
        Tok::Minus => "'-'".into(),
        Tok::Star => "'*'".into(),
        Tok::Slash => "'/'".into(),
        Tok::Percent => "'%'".into(),
        Tok::Assign => "'='".into(),
        Tok::EqEq => "'=='".into(),
        Tok::Ne => "'!='".into(),
        Tok::Lt => "'<'".into(),
        Tok::Le => "'<='".into(),
        Tok::Gt => "'>'".into(),
        Tok::Ge => "'>='".into(),
        Tok::PlusEq => "'+='".into(),
        Tok::MinusEq => "'-='".into(),
        Tok::StarEq => "'*='".into(),
        Tok::SlashEq => "'/='".into(),
        Tok::AndAnd => "'&&'".into(),
        Tok::OrOr => "'||'".into(),
        Tok::Bang => "'!'".into(),
        Tok::Amp => "'&'".into(),
        Tok::Pipe => "'|'".into(),
        Tok::Caret => "'^'".into(),
        Tok::Tilde => "'~'".into(),
        Tok::Shl => "'<<'".into(),
        Tok::Shr => "'>>'".into(),
        Tok::StarStar => "'**'".into(),
        Tok::AmpEq => "'&='".into(),
        Tok::PipeEq => "'|='".into(),
        Tok::CaretEq => "'^='".into(),
        Tok::ShlEq => "'<<='".into(),
        Tok::ShrEq => "'>>='".into(),
        Tok::StarStarEq => "'**='".into(),
        Tok::Directive(w) => format!("'#{w}'"),
        Tok::Eof => "dosya sonu".into(),
    }
}

/// Dosyanın ilk dolu satırındaki `klavye("..")` yönergesini bulur.
/// Dönen değer: (seçilen mod, yönergenin bulunduğu satırın bayt aralığı).
pub fn find_klavye(text: &str, file: u32) -> CResult<Option<(Mode, std::ops::Range<usize>)>> {
    let mut offset = 0;
    for (i, line) in text.split_inclusive('\n').enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            offset += line.len();
            continue;
        }
        if !trimmed.starts_with("klavye") {
            return Ok(None);
        }
        let rest = trimmed["klavye".len()..].trim_start();
        if !rest.starts_with('(') {
            return Ok(None);
        }
        let inner = rest[1..].trim_start();
        let span = Span {
            file,
            line: i as u32 + 1,
            col: (line.len() - line.trim_start().len()) as u32 + 1,
            start: (offset + line.len() - line.trim_start().len()) as u32,
            end: (offset + line.trim_end().len()) as u32,
        };
        let mode = if inner.starts_with("\"tr\"") {
            Mode::Tr
        } else if inner.starts_with("\"us\"") {
            Mode::Us
        } else {
            return err(span, "'klavye' içine sadece \"tr\" veya \"us\" yazılabilir.");
        };
        let after = inner[4..].trim_start();
        let after = after.strip_prefix(')').map(|a| a.trim());
        match after {
            Some("") | Some(";") => {}
            _ => return err(span, "'klavye(...)' satırında başka kod olamaz. Örnek: klavye(\"us\")"),
        }
        return Ok(Some((mode, offset..offset + line.trim_end_matches(['\n', '\r']).len())));
    }
    Ok(None)
}

pub struct Lexer<'a> {
    src: &'a str,
    chars: Vec<(usize, char)>,
    pos: usize,
    line: u32,
    col: u32,
    file: u32,
    base_byte: u32,
    mode: Mode,
    nl_before: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str, file: u32, mode: Mode) -> Self {
        Self::with_base(src, file, mode, 1, 1, 0)
    }

    pub fn with_base(src: &'a str, file: u32, mode: Mode, line: u32, col: u32, byte: u32) -> Self {
        Lexer {
            src,
            chars: src.char_indices().collect(),
            pos: 0,
            line,
            col,
            file,
            base_byte: byte,
            mode,
            nl_before: true,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).map(|c| c.1)
    }
    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).map(|c| c.1)
    }
    fn byte(&self) -> usize {
        self.chars.get(self.pos).map(|c| c.0).unwrap_or(self.src.len())
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
            self.nl_before = true;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn here(&self) -> Span {
        let b = self.base_byte + self.byte() as u32;
        Span { file: self.file, line: self.line, col: self.col, start: b, end: b + 1 }
    }

    pub fn tokenize(mut self) -> CResult<Vec<Token>> {
        let mut out = Vec::new();
        loop {
            self.skip_space_and_comments()?;
            let start = self.here();
            let nl = self.nl_before;
            let start_byte = self.byte();
            let Some(c) = self.peek() else {
                out.push(Token { tok: Tok::Eof, span: start, nl_before: true, raw_start: 0, raw_end: 0 });
                return Ok(out);
            };
            let tok = self.next_tok(c, start)?;
            let end_byte = self.byte();
            let span = Span { end: self.base_byte + end_byte as u32, ..start };
            out.push(Token {
                tok,
                span,
                nl_before: nl,
                raw_start: self.base_byte + start_byte as u32,
                raw_end: self.base_byte + end_byte as u32,
            });
            self.nl_before = false;
        }
    }

    fn skip_space_and_comments(&mut self) -> CResult<()> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some('/') if self.peek_at(1) == Some('<') => {
                    let start = self.here();
                    self.bump();
                    self.bump();
                    let mut depth = 1;
                    loop {
                        match self.peek() {
                            None => return err(start, "Çok satırlı yorum '/<' açıldı ama '>/' ile kapatılmadı."),
                            Some('/') if self.peek_at(1) == Some('<') => {
                                self.bump();
                                self.bump();
                                depth += 1;
                            }
                            Some('>') if self.peek_at(1) == Some('/') => {
                                self.bump();
                                self.bump();
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {
                                self.bump();
                            }
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn next_tok(&mut self, c: char, start: Span) -> CResult<Tok> {
        if c.is_ascii_digit() {
            return self.number(start);
        }
        if c == 'f' && self.peek_at(1) == Some('"') {
            self.bump();
            return self.fstring(start);
        }
        if c == '"' {
            return Ok(Tok::Str(self.string(start)?));
        }
        if c.is_alphabetic() || c == '_' {
            return self.word(start);
        }
        self.bump();
        let two = |l: &mut Self, next: char, yes: Tok, no: Tok| {
            if l.peek() == Some(next) {
                l.bump();
                yes
            } else {
                no
            }
        };
        Ok(match c {
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '{' => Tok::LBrace,
            '}' => Tok::RBrace,
            '[' => Tok::LBrack,
            ']' => Tok::RBrack,
            ',' => Tok::Comma,
            ':' => Tok::Colon,
            ';' => Tok::Semi,
            '%' => Tok::Percent,
            '.' => {
                if self.peek() == Some('.') {
                    self.bump();
                    two(self, '=', Tok::DotDotEq, Tok::DotDot)
                } else {
                    Tok::Dot
                }
            }
            '-' => {
                if self.peek() == Some('>') {
                    self.bump();
                    Tok::Arrow
                } else {
                    two(self, '=', Tok::MinusEq, Tok::Minus)
                }
            }
            '+' => two(self, '=', Tok::PlusEq, Tok::Plus),
            '*' if self.peek() == Some('*') => {
                self.bump();
                two(self, '=', Tok::StarStarEq, Tok::StarStar)
            }
            '*' => two(self, '=', Tok::StarEq, Tok::Star),
            '/' => two(self, '=', Tok::SlashEq, Tok::Slash),
            '=' => two(self, '=', Tok::EqEq, Tok::Assign),
            '!' => two(self, '=', Tok::Ne, Tok::Bang),
            '<' if self.peek() == Some('<') => {
                self.bump();
                two(self, '=', Tok::ShlEq, Tok::Shl)
            }
            '>' if self.peek() == Some('>') => {
                self.bump();
                two(self, '=', Tok::ShrEq, Tok::Shr)
            }
            '<' => two(self, '=', Tok::Le, Tok::Lt),
            '>' => two(self, '=', Tok::Ge, Tok::Gt),
            '&' if self.peek() == Some('&') => {
                self.bump();
                Tok::AndAnd
            }
            '|' if self.peek() == Some('|') => {
                self.bump();
                Tok::OrOr
            }
            '&' => two(self, '=', Tok::AmpEq, Tok::Amp),
            '|' => two(self, '=', Tok::PipeEq, Tok::Pipe),
            '^' => two(self, '=', Tok::CaretEq, Tok::Caret),
            '~' => Tok::Tilde,
            '#' if self.peek().is_some_and(|c| c.is_alphabetic()) => {
                return match self.word(start)? {
                    Tok::Ident(w) => Ok(Tok::Directive(w)),
                    _ => err(start, "Tanınmayan yönerge. Şu an sadece #Güvensiz var."),
                };
            }
            '#' => return err(start, "'#' işareti sadece yönergelerde kullanılır (#Güvensiz). Dosya eklemek için: dahil et(\"./dosya.kolay\")"),
            _ => return err(start, format!("Tanınmayan karakter: '{c}'")),
        })
    }

    fn number(&mut self, start: Span) -> CResult<Tok> {
        if self.peek() == Some('0') && matches!(self.peek_at(1), Some('x' | 'X')) {
            return self.hex_number(start);
        }
        let mut digits = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                digits.push(c);
                self.bump();
            } else if c == '_' && self.peek_at(1).is_some_and(|d| d.is_ascii_digit()) {
                self.bump();
            } else {
                break;
            }
        }
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|d| d.is_ascii_digit()) {
            self.bump();
            digits.push('.');
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    digits.push(c);
                    self.bump();
                } else if c == '_' && self.peek_at(1).is_some_and(|d| d.is_ascii_digit()) {
                    self.bump();
                } else {
                    break;
                }
            }
            return digits
                .parse::<f64>()
                .map(Tok::Float)
                .map_err(|_| Diag::new(start, "Geçersiz ondalık sayı."));
        }
        if self.peek().is_some_and(|c| c.is_alphabetic()) {
            return err(self.here(), "Sayının hemen ardından harf gelemez.");
        }
        Ok(Tok::Int(digits.parse::<BigInt>().unwrap()))
    }

    /// Onaltılık sayı: 0xFF, 0x1F_FF (büyük/küçük harf fark etmez)
    fn hex_number(&mut self, start: Span) -> CResult<Tok> {
        self.bump();
        self.bump();
        let mut digits = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_hexdigit() {
                digits.push(c);
                self.bump();
            } else if c == '_' && self.peek_at(1).is_some_and(|d| d.is_ascii_hexdigit()) && !digits.is_empty() {
                self.bump();
            } else {
                break;
            }
        }
        if digits.is_empty() {
            return err(start, "'0x' sonrasında onaltılık rakamlar (0-9, A-F) yazılmalı. Örnek: 0xFF");
        }
        if self.peek().is_some_and(|c| c.is_alphanumeric() || c == '.') {
            return err(self.here(), "Onaltılık sayıda geçersiz karakter. Kullanılabilenler: 0-9 ve A-F.");
        }
        Ok(Tok::Int(BigInt::parse_bytes(digits.as_bytes(), 16).unwrap()))
    }

    fn escape(&mut self, start: Span) -> CResult<char> {
        Ok(match self.bump() {
            Some('n') => '\n',
            Some('t') => '\t',
            Some('r') => '\r',
            Some('0') => '\0',
            Some('"') => '"',
            Some('\\') => '\\',
            Some('{') => '{',
            Some('}') => '}',
            Some(c) => return err(start, format!("Tanınmayan kaçış dizisi: '\\{c}'")),
            None => return err(start, "Yazı kapatılmadı."),
        })
    }

    fn string(&mut self, start: Span) -> CResult<String> {
        self.bump();
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return err(start, "Yazı '\"' ile açıldı ama kapatılmadı."),
                Some('"') => {
                    self.bump();
                    return Ok(s);
                }
                Some('\\') => {
                    let here = self.here();
                    self.bump();
                    s.push(self.escape(here)?);
                }
                Some(c) => {
                    s.push(c);
                    self.bump();
                }
            }
        }
    }

    fn fstring(&mut self, start: Span) -> CResult<Tok> {
        self.bump(); // "
        let mut parts = Vec::new();
        let mut lit = String::new();
        loop {
            match self.peek() {
                None => return err(start, "f-yazı '\"' ile açıldı ama kapatılmadı."),
                Some('"') => {
                    self.bump();
                    break;
                }
                Some('\\') => {
                    let here = self.here();
                    self.bump();
                    lit.push(self.escape(here)?);
                }
                Some('{') if self.peek_at(1) == Some('{') => {
                    self.bump();
                    self.bump();
                    lit.push('{');
                }
                Some('}') if self.peek_at(1) == Some('}') => {
                    self.bump();
                    self.bump();
                    lit.push('}');
                }
                Some('}') => {
                    return err(self.here(), "f-yazıda tek '}' kullanılamaz. Süslü parantezin kendisi için '}}' yazın.");
                }
                Some('{') => {
                    let open = self.here();
                    self.bump();
                    if !lit.is_empty() {
                        parts.push(FPart::Lit(std::mem::take(&mut lit)));
                    }
                    let (line, col, byte) = (self.line, self.col, self.base_byte + self.byte() as u32);
                    let begin = self.byte();
                    let mut depth = 0;
                    loop {
                        match self.peek() {
                            None | Some('\n') => return err(open, "f-yazıda '{' açıldı ama '}' ile kapatılmadı."),
                            Some('{') => depth += 1,
                            Some('}') if depth == 0 => break,
                            Some('}') => depth -= 1,
                            Some('"') => {
                                let here = self.here();
                                self.string(here)?;
                                continue;
                            }
                            _ => {}
                        }
                        self.bump();
                    }
                    let src = self.src[begin..self.byte()].to_string();
                    self.bump(); // }
                    if src.trim().is_empty() {
                        return err(open, "f-yazıda '{}' boş olamaz. İçine bir değer yazın, örneğin {ad}.");
                    }
                    parts.push(FPart::Expr { src, line, col, byte });
                }
                Some(c) => {
                    lit.push(c);
                    self.bump();
                }
            }
        }
        if !lit.is_empty() {
            parts.push(FPart::Lit(lit));
        }
        Ok(Tok::FStr(parts))
    }

    fn word(&mut self, start: Span) -> CResult<Tok> {
        let mut w = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                w.push(c);
                self.bump();
            } else {
                break;
            }
        }
        let span = Span { end: self.base_byte + self.byte() as u32, ..start };
        for (k, tr, us) in KEYWORDS {
            match self.mode {
                Mode::Tr if w == *tr => return Ok(Tok::Kw(*k)),
                Mode::Us if w == *us => return Ok(Tok::Kw(*k)),
                Mode::Us if w == *tr => {
                    return err(span, format!("'{tr}' us klavye modunda kullanılamaz, '{us}' yazın."));
                }
                _ => {}
            }
        }
        if self.mode == Mode::Us {
            for (tr, us) in BUILTIN_NAMES {
                if w == *us {
                    return Ok(Tok::Ident(tr.to_string()));
                }
                if w == *tr {
                    return err(span, format!("'{tr}' us klavye modunda kullanılamaz, '{us}' yazın."));
                }
            }
        }
        Ok(Tok::Ident(w))
    }
}

/// Kaynak kodu diğer klavye moduna çevirir (`kolay çevir`).
pub fn convert(text: &str, from: Mode, to: Mode) -> CResult<String> {
    if from == to {
        return Ok(text.to_string());
    }
    let mut body = text.to_string();
    let directive = find_klavye(text, 0)?;
    if let Some((_, range)) = &directive {
        body.replace_range(range.clone(), &" ".repeat(range.len()));
    }
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    collect_edits(&body, from, to, 1, 1, 0, &mut edits)?;
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = body;
    for (s, e, rep) in edits {
        out.replace_range(s..e, &rep);
    }
    Ok(match directive {
        Some((_, range)) => {
            let mut o = out;
            o.replace_range(range.clone(), &pad_to(&format!("klavye(\"{}\")", to.name()), range.len()));
            o.lines().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n") + if text.ends_with('\n') { "\n" } else { "" }
        }
        None if to == Mode::Us => format!("klavye(\"us\")\n{out}"),
        None => out,
    })
}

fn pad_to(s: &str, len: usize) -> String {
    if s.len() >= len { s.to_string() } else { format!("{s}{}", " ".repeat(len - s.len())) }
}

fn collect_edits(
    src: &str,
    from: Mode,
    to: Mode,
    line: u32,
    col: u32,
    byte: u32,
    edits: &mut Vec<(usize, usize, String)>,
) -> CResult<()> {
    let toks = Lexer::with_base(src, 0, from, line, col, byte).tokenize()?;
    for t in toks {
        let (s, e) = (t.raw_start as usize, t.raw_end as usize);
        match &t.tok {
            Tok::Kw(k) => {
                let new = kw_text(*k, to);
                edits.push((s, e, new.to_string()));
            }
            Tok::Ident(name) => {
                for (tr, us) in BUILTIN_NAMES {
                    if name == tr {
                        edits.push((s, e, if to == Mode::Us { us.to_string() } else { tr.to_string() }));
                    }
                }
            }
            Tok::Directive(name) => {
                for (tr, us) in BUILTIN_NAMES {
                    if name == tr {
                        edits.push((s, e, format!("#{}", if to == Mode::Us { us } else { tr })));
                    }
                }
            }
            Tok::FStr(parts) => {
                for p in parts {
                    if let FPart::Expr { src: esrc, line, col, byte } = p {
                        let mut inner = Vec::new();
                        collect_edits(esrc, from, to, *line, *col, *byte, &mut inner)?;
                        edits.extend(inner);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
