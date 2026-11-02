//! Checks on decompiled text: names used without a declaration, which would be
//! globals, and whether two output trees are the same.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

use crate::luau::{clip, read_text, walk, Args};

/// `"` strings become `""`, so nothing in them counts as code.
fn without_strings(text: &str) -> String {
    let c: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < c.len() {
        if c[i] == '"' {
            let mut j = i + 1;
            let closed = loop {
                match c.get(j) {
                    Some('"') => break true,
                    Some('\\') if c.get(j + 1).is_some_and(|n| *n != '\n') => j += 2,
                    Some('\\' | '\n') | None => break false,
                    Some(_) => j += 1,
                }
            };
            if closed {
                out.push_str("\"\"");
                i = j + 1;
                continue;
            }
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

fn name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn list_char(c: char) -> bool {
    name_char(c) || c == ',' || c == ' '
}

fn add_names(found: &mut HashSet<String>, list: &str) {
    found.extend(list.split(',').map(|x| x.trim().to_string()));
}

/// After `local`: the names, up to ` =`, `(` or the end of the line.
fn local_names(rest: &str) -> Option<&str> {
    for (at, c) in rest.char_indices() {
        if at > 0 && (rest[at..].starts_with(" =") || c == '(' || c == '\n') {
            return Some(&rest[..at]);
        }
        if !list_char(c) {
            return None;
        }
    }
    (!rest.is_empty()).then_some(rest)
}

fn word_starts(text: &str, at: usize) -> bool {
    !text[..at].chars().next_back().is_some_and(name_char)
}

fn declared(text: &str) -> HashSet<String> {
    let mut found = HashSet::new();
    for (at, m) in text.match_indices("local ") {
        if !word_starts(text, at) {
            continue;
        }
        let rest = &text[at + m.len()..];
        let list = rest.strip_prefix("function ").and_then(local_names).or_else(|| local_names(rest));
        if let Some(list) = list {
            add_names(&mut found, list);
        }
    }
    for (at, m) in text.match_indices("for ") {
        if !word_starts(text, at) {
            continue;
        }
        let rest = &text[at + m.len()..];
        for (k, c) in rest.char_indices() {
            if k > 0 && (rest[k..].starts_with(" in ") || rest[k..].starts_with(" = ")) {
                add_names(&mut found, &rest[..k]);
                break;
            }
            if !list_char(c) {
                break;
            }
        }
    }
    for (at, m) in text.match_indices("function") {
        let rest = &text[at + m.len()..];
        let named = rest.find(|c: char| !(name_char(c) || c == ' ' || c == '.' || c == ':'));
        if let Some(open) = named.filter(|o| rest[*o..].starts_with('(')) {
            if let Some(close) = rest[open..].find(')') {
                add_names(&mut found, &rest[open + 1..open + close]);
            }
        }
    }
    found
}

/// v12, v3x2, v3_1, v3x2_1, arg1_2: the names the decompiler makes up.
fn generated(word: &str) -> bool {
    let digits = |s: &str| s.bytes().take_while(u8::is_ascii_digit).count();
    if let Some(rest) = word.strip_prefix("arg") {
        let n = digits(rest);
        return n > 0 && rest[n..].strip_prefix('_').is_some_and(|t| !t.is_empty() && digits(t) == t.len());
    }
    let Some(mut rest) = word.strip_prefix('v') else { return false };
    let n = digits(rest);
    if n == 0 {
        return false;
    }
    rest = &rest[n..];
    if let Some(t) = rest.strip_prefix('x') {
        let n = digits(t);
        if n > 0 {
            rest = &t[n..];
        }
    }
    while let Some(t) = rest.strip_prefix('_') {
        let n = digits(t);
        if n == 0 {
            return false;
        }
        rest = &t[n..];
    }
    rest.is_empty()
}

fn used_names(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut before = '\n';
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if !name_char(c) {
            before = c;
            continue;
        }
        let mut end = at + c.len_utf8();
        while let Some((k, n)) = chars.peek().copied().filter(|e| name_char(e.1)) {
            end = k + n.len_utf8();
            chars.next();
        }
        if !matches!(before, '.' | ':' | '"') && generated(&text[at..end]) {
            found.insert(text[at..end].to_string());
        }
        before = 'a';
    }
    found
}

pub struct Undeclared {
    pub names: usize,
    pub files: usize,
    pub examples: Vec<String>,
}

pub fn undeclared(roots: &[String]) -> Undeclared {
    let mut out = Undeclared { names: 0, files: 0, examples: Vec::new() };
    for root in roots {
        let mut files = Vec::new();
        walk(Path::new(root), &mut files);
        files.sort();
        for path in files.iter().filter(|p| p.extension().is_some_and(|x| x == "lua")) {
            let text = without_strings(&read_text(&path.to_string_lossy()));
            let known = declared(&text);
            let bad: Vec<String> = used_names(&text).into_iter().filter(|n| !known.contains(n)).collect();
            if !bad.is_empty() {
                out.files += 1;
                out.names += bad.len();
                if out.examples.len() < 8 {
                    out.examples.push(format!("  {} {:?}", path.display(), &bad[..bad.len().min(5)]));
                }
            }
        }
    }
    out
}

pub fn undeclared_main(args: &Args) -> u8 {
    let found = undeclared(&args.0);
    println!("{} undeclared generated names in {} files", found.names, found.files);
    for line in &found.examples {
        println!("{line}");
    }
    u8::from(found.names > 0)
}

fn tree(root: &str) -> BTreeMap<String, String> {
    let mut files = Vec::new();
    walk(Path::new(root), &mut files);
    let name = |p: &Path| p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/");
    files.iter().map(|p| (name(p), p.to_string_lossy().into_owned())).collect()
}

/// Line endings do not count.
pub fn same_main(args: &Args) -> u8 {
    let [a, b] = args.0.as_slice() else {
        eprintln!("harness same A B");
        return 2;
    };
    let (ta, tb) = (tree(a), tree(b));
    let missing = ta.keys().filter(|k| !tb.contains_key(*k)).count();
    let extra = tb.keys().filter(|k| !ta.contains_key(*k)).count();
    let mut differ = Vec::new();
    for (name, p) in &ta {
        let Some(q) = tb.get(name) else { continue };
        let (x, y) = (read_text(p), read_text(q));
        if x != y {
            let line = x.lines().zip(y.lines()).take_while(|(l, m)| l == m).count();
            let shown = |t: &str| clip(t.lines().nth(line).unwrap_or(""), 100);
            differ.push(format!("  {name}: line {}\n    {}\n    {}", line + 1, shown(&x), shown(&y)));
        }
    }
    let common = ta.len() - missing;
    println!("{} files: {} identical, {} differ, {missing} missing, {extra} extra", ta.len(), common - differ.len(), differ.len());
    for d in differ.iter().take(5) {
        println!("{d}");
    }
    u8::from(!differ.is_empty() || missing > 0 || extra > 0)
}
