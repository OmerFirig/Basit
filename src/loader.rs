// Dosyaları okur, `dahil et` ile eklenen dosyaları bulur ve hepsini ayrıştırır.

use crate::ast::{Stmt, StmtKind};
use crate::diag::{err, CResult, Diag, Sources, Span};
use crate::lexer::{find_klavye, Lexer, Mode};
use crate::parser::parse_file;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct ParsedModule {
    pub file: u32,
    pub mode: Mode,
    pub stmts: Vec<Stmt>,
    /// (komut sırası, eklenen modül)
    pub deps: Vec<(usize, usize)>,
}

/// Modüller, bağımlılıklar önce gelecek şekilde sıralı döner. Ana dosya en sondadır.
pub fn load(main: &Path, default_mode: Mode, sources: &mut Sources) -> CResult<Vec<ParsedModule>> {
    let mut mods = Vec::new();
    let mut by_path = HashMap::new();
    let mut stack = Vec::new();
    load_one(main, default_mode, sources, &mut mods, &mut by_path, &mut stack, None, None)?;
    Ok(mods)
}

fn load_one(
    path: &Path,
    default_mode: Mode,
    sources: &mut Sources,
    mods: &mut Vec<ParsedModule>,
    by_path: &mut HashMap<PathBuf, usize>,
    stack: &mut Vec<PathBuf>,
    from: Option<Span>,
    shown: Option<&str>,
) -> CResult<usize> {
    let at = from.unwrap_or(Span { file: u32::MAX, ..Default::default() });
    let canon = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let shown = shown.map(str::to_string).unwrap_or_else(|| path.display().to_string());
            return err(at, format!("'{shown}' dosyası bulunamadı."));
        }
    };
    if let Some(&i) = by_path.get(&canon) {
        return Ok(i);
    }
    if let Some(pos) = stack.iter().position(|p| *p == canon) {
        let chain: Vec<String> = stack[pos..]
            .iter()
            .chain(std::iter::once(&canon))
            .map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
            .collect();
        return err(at, format!("Döngüsel dahil etme: {}", chain.join(" → ")));
    }
    let text = match std::fs::read_to_string(&canon) {
        Ok(t) => t,
        Err(e) => return err(at, format!("'{}' okunamadı: {e}", path.display())),
    };
    let file = sources.add(canon.clone(), text.clone());
    let (mode, lex_text) = match find_klavye(&text, file)? {
        Some((m, range)) => {
            let mut t = text.clone();
            t.replace_range(range.clone(), &" ".repeat(range.len()));
            (m, t)
        }
        None => (default_mode, text),
    };
    let toks = Lexer::new(&lex_text, file, mode).tokenize()?;
    let stmts = parse_file(toks, mode, file)?;

    stack.push(canon.clone());
    let dir = canon.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut deps = Vec::new();
    for (i, s) in stmts.iter().enumerate() {
        if let StmtKind::Dahil { path: p, .. } = &s.kind {
            let target = dir.join(p);
            let idx = load_one(&target, default_mode, sources, mods, by_path, stack, Some(s.span), Some(p))
                .map_err(|d| if d.span.file == u32::MAX { Diag::new(s.span, d.msg) } else { d })?;
            deps.push((i, idx));
        }
    }
    stack.pop();

    let id = mods.len();
    mods.push(ParsedModule { file, mode, stmts, deps });
    by_path.insert(canon, id);
    Ok(id)
}
