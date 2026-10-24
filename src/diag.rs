// Kaynak dosyalar, konum bilgisi ve Türkçe hata mesajlarının ekrana basılması.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub file: u32,
    pub line: u32,
    pub col: u32,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn to(self, other: Span) -> Span {
        if other.file != self.file || other.end < self.start {
            return self;
        }
        Span { end: other.end, ..self }
    }
}

#[derive(Debug, Clone)]
pub struct Diag {
    pub msg: String,
    pub span: Span,
}

impl Diag {
    pub fn new(span: Span, msg: impl Into<String>) -> Diag {
        Diag { msg: msg.into(), span }
    }
}

pub type CResult<T> = Result<T, Diag>;

pub fn err<T>(span: Span, msg: impl Into<String>) -> CResult<T> {
    Err(Diag::new(span, msg))
}

pub struct SourceFile {
    pub name: String,
    pub path: PathBuf,
    pub text: String,
}

#[derive(Default)]
pub struct Sources {
    pub files: Vec<SourceFile>,
}

impl Sources {
    pub fn add(&mut self, path: PathBuf, text: String) -> u32 {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        self.files.push(SourceFile { name, path, text });
        (self.files.len() - 1) as u32
    }

    pub fn name(&self, file: u32) -> &str {
        self.files.get(file as usize).map(|f| f.name.as_str()).unwrap_or("?")
    }

    pub fn render(&self, kind: &str, d: &Diag) -> String {
        let mut out = String::new();
        let Some(f) = self.files.get(d.span.file as usize) else {
            return format!("{kind}: {}\n", d.msg);
        };
        if d.span.line == 0 {
            return format!("{kind} ({}): {}\n", f.name, d.msg);
        }
        out.push_str(&format!("{kind} ({}, satır {}): {}\n", f.name, d.span.line, d.msg));
        if let Some(line_text) = f.text.lines().nth(d.span.line as usize - 1) {
            let num = d.span.line.to_string();
            let pad = " ".repeat(num.len());
            out.push_str(&format!("  {num} | {line_text}\n"));
            let col = d.span.col.max(1) as usize;
            let line_start = line_start_byte(&f.text, d.span.line);
            let width = match f.text.get(d.span.start as usize..d.span.end as usize) {
                Some(t) if d.span.end > d.span.start && (d.span.end as usize) <= line_start + line_text.len() => {
                    t.chars().count().max(1)
                }
                _ => 1,
            };
            let indent: String = line_text
                .chars()
                .take(col - 1)
                .map(|c| if c == '\t' { '\t' } else { ' ' })
                .collect();
            out.push_str(&format!("  {pad} | {indent}{}\n", "^".repeat(width)));
        }
        out
    }
}

fn line_start_byte(text: &str, line: u32) -> usize {
    if line <= 1 {
        return 0;
    }
    let mut n = 1;
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            n += 1;
            if n == line {
                return i + 1;
            }
        }
    }
    text.len()
}
