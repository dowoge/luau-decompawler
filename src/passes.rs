//! Source-level passes over decompiled Luau text. Each pass only rewrites a
//! shape it fully understands and leaves anything else alone.
//!
//! The matchers are hand-written: each one states the pattern it implements,
//! with the same greedy/anchored meaning the pattern would have.

use std::sync::OnceLock;

use crate::hash::{key, HashMap, HashSet};

pub type CallMap = HashMap<String, Vec<Vec<String>>>;

// ------------------------------------------------------------ small helpers

#[inline]
fn is_start(b: u8) -> bool {
    WORD[b as usize] & 2 != 0
}

#[inline]
fn is_word(b: u8) -> bool {
    WORD[b as usize] != 0
}

/// Per byte: 1 may be in a word, 2 may also start one.
const WORD: [u8; 256] = {
    let mut table = [0; 256];
    let mut i = 0;
    while i < 256 {
        let c = i as u8;
        if c.is_ascii_digit() {
            table[i] = 1;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            table[i] = 3;
        }
        i += 1;
    }
    table
};

fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// `str.strip()`.
fn strip(s: &str) -> &str {
    // nearly always an indent and then plain characters at both ends
    let start = s.as_bytes().iter().position(|c| *c != b' ').unwrap_or(s.len());
    let rest = &s[start..];
    match rest.as_bytes() {
        [] => rest,
        [first, .., last] if is_plain(*first) && is_plain(*last) => rest,
        [only] if is_plain(*only) => rest,
        _ => rest.trim_matches(is_space),
    }
}

/// Is this byte a character that is not space of any kind?
#[inline]
fn is_plain(b: u8) -> bool {
    (0x21..0x7f).contains(&b)
}

#[inline]
fn rstrip(s: &str) -> &str {
    match s.as_bytes().last() {
        Some(b) if is_plain(*b) => s,
        _ => s.trim_end_matches(is_space),
    }
}

#[inline]
fn is_blank(s: &str) -> bool {
    strip(s).is_empty()
}

pub fn indent_of(line: &str) -> usize {
    line.as_bytes().iter().take_while(|&&b| b == b' ').count()
}

fn spaces(n: usize) -> String {
    " ".repeat(n)
}

/// `^[A-Za-z_][A-Za-z0-9_]*$`
fn is_token(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty() && is_start(b[0]) && b.iter().all(|&c| is_word(c))
}

/// End of the run of word characters starting at `i`.
fn word_end(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && is_word(b[i]) {
        i += 1;
    }
    i
}

/// `^(ident)(\.ident)*` from `i`: the end of the chain, or None.
fn ident_chain_end(b: &[u8], i: usize) -> Option<usize> {
    if i >= b.len() || !is_start(b[i]) {
        return None;
    }
    let mut e = word_end(b, i);
    while e + 1 < b.len() && b[e] == b'.' && is_start(b[e + 1]) {
        e = word_end(b, e + 1);
    }
    Some(e)
}

/// `s` without its first `a` and last `b` bytes, empty when the two ends cross.
fn inner(s: &str, a: usize, b: usize) -> &str {
    if a + b >= s.len() {
        ""
    } else {
        &s[a..s.len() - b]
    }
}

fn find_from(hay: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if needle.is_empty() {
        return if start <= hay.len() { Some(start) } else { None };
    }
    if hay.len() < needle.len() || start > hay.len() - needle.len() {
        return None;
    }
    let first = needle[0];
    let last = hay.len() - needle.len();
    let mut i = start;
    while i <= last {
        match hay[i..=last].iter().position(|&c| c == first) {
            None => return None,
            Some(p) => {
                i += p;
                if &hay[i..i + needle.len()] == needle {
                    return Some(i);
                }
                i += 1;
            }
        }
    }
    None
}

/// Byte length of the character at `i` when it is a letter or `_`, else 0.
#[inline]
fn alpha_len(s: &str, i: usize) -> usize {
    let b = s.as_bytes()[i];
    if b < 0x80 {
        return usize::from(is_start(b));
    }
    if !s.is_char_boundary(i) {
        return 0;
    }
    let c = s[i..].chars().next().unwrap();
    if c.is_alphabetic() {
        c.len_utf8()
    } else {
        0
    }
}

#[inline]
fn alnum_len(s: &str, i: usize) -> usize {
    let b = s.as_bytes()[i];
    if b < 0x80 {
        return usize::from(is_word(b));
    }
    if !s.is_char_boundary(i) {
        return 0;
    }
    let c = s[i..].chars().next().unwrap();
    if c.is_alphanumeric() {
        c.len_utf8()
    } else {
        0
    }
}

/// Identifier tokens of a line, outside strings, not after a `.`/`:`. `{...}`
/// in backtick strings is code, so backticks are not quotes here. `f` returns
/// false to stop early.
fn for_each_word<'a>(s: &'a str, mut f: impl FnMut(&'a str) -> bool) {
    let b = s.as_bytes();
    let code = code_mask(s);
    let mut i = 0;
    while i < b.len() {
        if !code[i] {
            i += 1;
            continue;
        }
        let n = alpha_len(s, i);
        if n > 0 {
            let mut j = i + n;
            while j < b.len() {
                let m = alnum_len(s, j);
                if m == 0 {
                    break;
                }
                j += m;
            }
            let prev = if i > 0 { b[i - 1] } else { 0 };
            if prev != b'.' && prev != b':' && !f(&s[i..j]) {
                return;
            }
            i = j;
            continue;
        }
        i += 1;
    }
}

/// Which bytes of a line are code rather than string text: text is inside
/// "..." and '...', and inside `...` but for its {} interpolations, which
/// are code again, strings and all.
fn code_mask(s: &str) -> Vec<bool> {
    if !s.bytes().any(|c| matches!(c, b'"' | b'\'' | b'`')) {
        return vec![true; s.len()];
    }
    #[derive(Clone, Copy)]
    enum M {
        Code(u32), // brace depth since this code began
        Quote(u8),
        Tick,
    }
    let b = s.as_bytes();
    let mut mask = vec![false; b.len()];
    let mut stack = vec![M::Code(0)];
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        match *stack.last().unwrap() {
            M::Code(depth) => match c {
                b'"' | b'\'' => stack.push(M::Quote(c)),
                b'`' => stack.push(M::Tick),
                b'{' => {
                    *stack.last_mut().unwrap() = M::Code(depth + 1);
                    mask[i] = true;
                }
                b'}' if depth == 0 && stack.len() > 1 => {
                    stack.pop(); // the end of an interpolation
                }
                b'}' => {
                    *stack.last_mut().unwrap() = M::Code(depth.saturating_sub(1));
                    mask[i] = true;
                }
                _ => mask[i] = true,
            },
            M::Quote(q) => {
                if c == b'\\' {
                    i += 2;
                    continue;
                }
                if c == q {
                    stack.pop();
                }
            }
            M::Tick => {
                if c == b'\\' {
                    i += 2;
                    continue;
                }
                if c == b'`' {
                    stack.pop();
                } else if c == b'{' {
                    stack.push(M::Code(0));
                }
            }
        }
        i += 1;
    }
    mask
}

fn word_keys_into(set: &mut HashSet<u64>, line: &str) {
    for_each_word(line, |w| {
        set.insert(key(w.as_bytes()));
        true
    });
}

fn words_into(set: &mut HashSet<String>, line: &str) {
    for_each_word(line, |w| {
        if !set.contains(w) {
            set.insert(w.to_string());
        }
        true
    });
}

fn has_word(line: &str, name: &str) -> bool {
    if !line.contains(name) {
        return false;
    }
    let mut found = false;
    for_each_word(line, |w| {
        if w == name {
            found = true;
            false
        } else {
            true
        }
    });
    found
}

/// Rename identifier `old` (a variable, not a field) outside strings.
fn replace_token(line: &mut String, old: &str, new: &str) {
    if !line.contains(old) {
        return;
    }
    let s: &str = line;
    let b = s.as_bytes();
    let code = code_mask(s);
    let mut out = String::with_capacity(s.len() + new.len());
    let mut i = 0;
    let mut plain = 0; // start of the text not yet copied
    while i < b.len() {
        if !code[i] {
            i += 1;
            continue;
        }
        let n = alpha_len(s, i);
        if n > 0 {
            let mut j = i + n;
            while j < b.len() {
                let m = alnum_len(s, j);
                if m == 0 {
                    break;
                }
                j += m;
            }
            let prev = if i > 0 { b[i - 1] } else { 0 };
            if &s[i..j] == old && prev != b'.' && prev != b':' {
                out.push_str(&s[plain..i]);
                out.push_str(new);
                plain = j;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    if plain == 0 {
        return; // nothing matched
    }
    out.push_str(&s[plain..]);
    *line = out;
}

/// Calls `f(index, char, depth)` for each byte outside string literals;
/// `f` returns false to stop.
fn scan_depth(s: &[u8], mut f: impl FnMut(usize, u8, i32) -> bool) {
    let mut depth = 0;
    let mut quote = 0u8;
    let mut i = 0;
    while i < s.len() {
        let ch = s[i];
        if quote != 0 {
            if ch == b'\\' {
                i += 2;
                continue;
            }
            if ch == quote {
                quote = 0;
            }
        } else if ch == b'"' || ch == b'\'' || ch == b'`' {
            quote = ch;
        } else {
            match ch {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                _ => {}
            }
            if !f(i, ch, depth) {
                return;
            }
        }
        i += 1;
    }
}

fn split_top(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0;
    scan_depth(s.as_bytes(), |i, ch, depth| {
        if ch == b',' && depth == 0 {
            parts.push(strip(&s[start..i]).to_string());
            start = i + 1;
        }
        true
    });
    parts.push(strip(&s[start..]).to_string());
    parts
}

fn matching_paren(s: &[u8], open_at: usize) -> Option<usize> {
    let mut found = None;
    scan_depth(&s[open_at..], |i, ch, depth| {
        if ch == b')' && depth == 0 {
            found = Some(open_at + i);
            false
        } else {
            true
        }
    });
    found
}

/// `^(v\d+(x\d+)?(_\d+)*|arg\d+(_\d+)?)$`
fn is_generated(s: &str) -> bool {
    let b = s.as_bytes();
    let digits = |mut i: usize| -> Option<usize> {
        let st = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i > st {
            Some(i)
        } else {
            None
        }
    };
    if b.first() == Some(&b'v') {
        let Some(mut i) = digits(1) else { return false };
        if i < b.len() && b[i] == b'x' {
            match digits(i + 1) {
                Some(e) => i = e,
                None => return false,
            }
        }
        while i < b.len() {
            if b[i] != b'_' {
                return false;
            }
            match digits(i + 1) {
                Some(e) => i = e,
                None => return false,
            }
        }
        return true;
    }
    if s.starts_with("arg") {
        let Some(mut i) = digits(3) else { return false };
        if i < b.len() {
            if b[i] != b'_' {
                return false;
            }
            match digits(i + 1) {
                Some(e) => i = e,
                None => return false,
            }
        }
        return i == b.len();
    }
    false
}

/// `v\d+(x\d+)?`
fn is_plain_v(s: &str) -> bool {
    let b = s.as_bytes();
    if b.first() != Some(&b'v') {
        return false;
    }
    let mut i = 1;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == 1 {
        return false;
    }
    if i < b.len() {
        if b[i] != b'x' {
            return false;
        }
        let st = i + 1;
        i = st;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == st {
            return false;
        }
    }
    i == b.len()
}

/// `(v\d+(x\d+)?|arg\d+)_{depth}`
fn is_depth_name(s: &str, depth: usize) -> bool {
    let tail = format!("_{depth}");
    let Some(head) = s.strip_suffix(tail.as_str()) else { return false };
    if is_plain_v(head) {
        return true;
    }
    match head.strip_prefix("arg") {
        Some(d) => !d.is_empty() && d.bytes().all(|c| c.is_ascii_digit()),
        None => false,
    }
}

fn pass_keyword(s: &str) -> bool {
    matches!(
        s,
        "and" | "break" | "do" | "else" | "elseif" | "end" | "false" | "for" | "function" | "if"
            | "in" | "local" | "nil" | "not" | "or" | "repeat" | "return" | "then" | "true"
            | "until" | "while" | "continue" | "self" | "type" | "typeof" | "export"
    )
}

fn str_isupper(s: &str) -> bool {
    let mut cased = false;
    for c in s.chars() {
        if c.is_lowercase() {
            return false;
        }
        if c.is_uppercase() {
            cased = true;
        }
    }
    cased
}

fn lower_first(s: &str) -> String {
    if s.is_empty() || str_isupper(s) {
        return s.to_string();
    }
    let mut it = s.chars();
    let c = it.next().unwrap();
    let mut out: String = c.to_lowercase().collect();
    out.push_str(it.as_str());
    out
}

fn first_upper(s: &str) -> bool {
    s.chars().next().is_some_and(str_isupper_char)
}

fn str_isupper_char(c: char) -> bool {
    c.is_uppercase()
}

/// `^local (TOKEN(, TOKEN)*)( = .*)?$` on text starting at "local ": the name
/// list and the ` = ...` part.
fn parse_local_list(s: &str) -> Option<(&str, Option<&str>)> {
    let r = s.strip_prefix("local ")?;
    let b = r.as_bytes();
    if b.is_empty() || !is_start(b[0]) {
        return None;
    }
    let mut e = word_end(b, 0);
    while e + 2 < b.len() && b[e] == b',' && b[e + 1] == b' ' && is_start(b[e + 2]) {
        e = word_end(b, e + 2);
    }
    let rest = &r[e..];
    if rest.is_empty() {
        return Some((&r[..e], None));
    }
    if rest.starts_with(" = ") && !rest.contains('\n') {
        return Some((&r[..e], Some(rest)));
    }
    None
}

/// `function(?: [A-Za-z_][A-Za-z0-9_.:]*)?\(([^()]*)\)$` searched in a
/// right-stripped line: the parameter text.
fn func_open(line: &str) -> Option<&str> {
    let s = rstrip(line);
    let body = s.strip_suffix(')')?;
    let open = body.rfind('(')?;
    let params = &body[open + 1..];
    if params.contains(')') {
        return None;
    }
    let head = &s[..open];
    if head.ends_with("function") {
        return Some(params);
    }
    let h = head.as_bytes();
    let mut k = h.len();
    while k > 0 && (is_word(h[k - 1]) || h[k - 1] == b'.' || h[k - 1] == b':') {
        k -= 1;
    }
    if k == h.len() || !is_start(h[k]) {
        return None;
    }
    if head[..k].ends_with("function ") {
        Some(params)
    } else {
        None
    }
}

fn block_end(lines: &[String], i: usize) -> usize {
    let pad = indent_of(&lines[i]);
    for (j, l) in lines.iter().enumerate().skip(i + 1) {
        if !is_blank(l) && indent_of(l) <= pad {
            return j;
        }
    }
    lines.len() - 1
}

fn split_params(s: &str) -> Vec<String> {
    s.split(',').map(strip).filter(|x| !x.is_empty()).map(str::to_string).collect()
}

/// (header index, end index, params) of every function in the text,
/// including the whole file as the outermost one (header -1).
fn functions(lines: &[String]) -> Vec<(i64, usize, Vec<String>)> {
    let mut found = vec![(-1, lines.len(), Vec::new())];
    for (i, line) in lines.iter().enumerate() {
        if let Some(p) = func_open(line) {
            found.push((i as i64, block_end(lines, i), split_params(p)));
        }
    }
    found
}

/// `(?<![A-Za-z0-9_.])NAME[.:]` anywhere in `l`.
fn used_as_object(l: &str, name: &str) -> bool {
    let b = l.as_bytes();
    let n = name.as_bytes();
    let mut i = 0;
    while let Some(p) = find_from(b, n, i) {
        let e = p + n.len();
        let prev_ok = p == 0 || !(is_word(b[p - 1]) || b[p - 1] == b'.');
        if prev_ok && e < b.len() && (b[e] == b'.' || b[e] == b':') {
            return true;
        }
        i = p + 1;
    }
    false
}

/// `\b` before byte `p`, when the text at `p` starts with a word character.
fn boundary_before(s: &str, p: usize) -> bool {
    match s[..p].chars().next_back() {
        None => true,
        Some(c) => !(c.is_alphanumeric() || c == '_'),
    }
}

/// `\b` after byte `e`, when the text before `e` ends with a word character.
fn boundary_after(s: &str, e: usize) -> bool {
    match s[e..].chars().next() {
        None => true,
        Some(c) => !(c.is_alphanumeric() || c == '_'),
    }
}

/// `\bNAME\b` in `s`.
fn find_word_bounded(s: &str, name: &str) -> Option<usize> {
    let b = s.as_bytes();
    let n = name.as_bytes();
    let mut i = 0;
    while let Some(p) = find_from(b, n, i) {
        let e = p + n.len();
        if boundary_before(s, p) && boundary_after(s, e) {
            return Some(p);
        }
        i = p + 1;
    }
    None
}

// ------------------------------------------------------------ jump clean-up

fn balanced_parens_depth(line: &[u8]) -> i32 {
    let mut d = 0;
    let mut quote = 0u8;
    let mut k = 0;
    while k < line.len() {
        let ch = line[k];
        if quote != 0 {
            if ch == b'\\' {
                k += 2;
                continue;
            }
            if ch == quote {
                quote = 0;
            }
        } else if ch == b'"' || ch == b'\'' || ch == b'`' {
            quote = ch;
        } else if matches!(ch, b'(' | b'[' | b'{') {
            d += 1;
        } else if matches!(ch, b')' | b']' | b'}') {
            d -= 1;
        }
        k += 1;
    }
    d
}

/// `function\([^()]*\)$` searched in a right-stripped string.
fn ends_function_header(s: &str) -> bool {
    let Some(body) = s.strip_suffix(')') else { return false };
    let Some(open) = body.rfind('(') else { return false };
    !body[open + 1..].contains(')') && s[..open].ends_with("function")
}

/// `function\([^)]*\)$` searched in a right-stripped string.
fn ends_function_header_loose(s: &str) -> bool {
    let Some(body) = s.strip_suffix(')') else { return false };
    let from = body.rfind(')').map_or(0, |p| p + 1);
    body[from..].contains("function(")
}

/// `(^|\s)(then|do|else|repeat)$`
fn ends_block_word(s: &str) -> bool {
    for w in ["then", "do", "else", "repeat"] {
        if let Some(head) = s.strip_suffix(w) {
            match head.chars().next_back() {
                None => return true,
                Some(c) if c.is_whitespace() || ('\x1c'..='\x1f').contains(&c) => return true,
                _ => {}
            }
        }
    }
    false
}

/// Last line of the statement starting on line i (a closure or table written
/// over several lines continues below it).
fn statement_end(lines: &[String], i: usize) -> usize {
    let ind = indent_of(&lines[i]);
    let mut depth = balanced_parens_depth(lines[i].as_bytes());
    let mut j = i;
    while j + 1 < lines.len() {
        if depth > 0 {
            j += 1;
            depth += balanced_parens_depth(lines[j].as_bytes());
            continue;
        }
        if ends_function_header(rstrip(&lines[j])) {
            // `return function() ... end` with no brackets around it
            let mut k = j + 1;
            while k < lines.len()
                && !(indent_of(&lines[k]) <= ind && strip(&lines[k]).starts_with("end"))
            {
                k += 1;
            }
            if k >= lines.len() {
                break;
            }
            j = k;
            let skip = indent_of(&lines[j]) + 3;
            let b = lines[j].as_bytes();
            depth += balanced_parens_depth(if skip < b.len() { &b[skip..] } else { &[] });
            continue;
        }
        break;
    }
    j
}

/// Luau requires return/break/continue to end their block. One followed by
/// more statements of the same block (reached through a jump) becomes
/// `do return end` and friends. Statements starting with `(` get a `;`.
pub fn wrap_mid_block_exits(lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        let stripped = line.trim_start_matches(' ');
        let ind = &line[..line.len() - stripped.len()];
        if stripped.starts_with('(') && !out.is_empty() {
            // only after an expression could `(` read as a call of it; a block
            // opener before it needs nothing (and `;` there is not allowed)
            let prev = rstrip(out.last().unwrap());
            let opener = ends_block_word(prev) || ends_function_header_loose(prev);
            if !opener && !prev.ends_with(';') {
                let new = format!("{prev};");
                *out.last_mut().unwrap() = new;
            }
        }
        let word = stripped.split(' ').next().unwrap();
        if matches!(word, "return" | "break" | "continue") {
            let last = statement_end(&lines, i);
            let nxt: &str = lines.get(last + 1).map_or("", |s| s.as_str());
            let n_stripped = nxt.trim_start_matches(' ');
            let n_ind = nxt.len() - n_stripped.len();
            let ends_block = n_ind < ind.len()
                || matches!(
                    n_stripped.split(' ').next().unwrap(),
                    "end" | "else" | "elseif" | "until" | "end)"
                )
                || n_stripped.starts_with("end")
                || n_stripped.starts_with("::")
                || n_stripped.starts_with("until");
            if !ends_block && !nxt.is_empty() {
                if last == i {
                    out.push(format!("{ind}do {stripped} end"));
                } else {
                    out.push(format!("{ind}do"));
                    for l in &lines[i..=last] {
                        out.push(format!("    {l}"));
                    }
                    out.push(format!("{ind}end"));
                }
                i = last + 1;
                continue;
            }
        }
        out.push(line.clone());
        i += 1;
    }
    out
}

/// An else block that is one if statement becomes elseif branches.
pub fn merge_elseif(body: Vec<String>, ind: usize) -> Vec<String> {
    let pad = spaces(ind);
    let inner_pad = spaces(ind + 4);
    let if_head = format!("{inner_pad}if ");
    let inner_end = format!("{inner_pad}end");
    if body.len() >= 2
        && body[0].starts_with(&if_head)
        && body[0].ends_with(" then")
        && body[body.len() - 1] == inner_end
    {
        // every other line at the inner level must belong to that same if
        let deeper = format!("{inner_pad} ");
        let inner_else = format!("{inner_pad}else");
        let inner_elseif = format!("{inner_pad}elseif ");
        let same_if = body[1..body.len() - 1]
            .iter()
            .filter(|l| l.starts_with(&inner_pad) && !l.starts_with(&deeper))
            .all(|l| *l == inner_else || (l.starts_with(&inner_elseif) && l.ends_with(" then")));
        if same_if {
            let mut out = Vec::with_capacity(body.len());
            out.push(format!("{pad}else{}", &body[0][ind + 4..]));
            for l in &body[1..body.len() - 1] {
                out.push(if l.starts_with(&inner_pad) { l[4..].to_string() } else { l.clone() });
            }
            return out;
        }
    }
    let mut out = Vec::with_capacity(body.len() + 1);
    out.push(format!("{pad}else"));
    out.extend(body);
    out
}

/// `^ *(for .* do|while .* do|repeat)$`
fn loop_open(l: &str) -> bool {
    let s = l.trim_start_matches(' ');
    if s.contains('\n') {
        return false;
    }
    s == "repeat"
        || (s.starts_with("for ") && s.ends_with(" do") && s.len() >= 7)
        || (s.starts_with("while ") && s.ends_with(" do") && s.len() >= 9)
}

/// `^( *)::L(\d+)::$`
fn parse_label(l: &str) -> Option<(usize, &str)> {
    let ind = indent_of(l);
    let n = l[ind..].strip_prefix("::L")?.strip_suffix("::")?;
    if !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()) {
        Some((ind, n))
    } else {
        None
    }
}

/// `\bgoto L<n>\b`
fn has_goto(l: &str, n: &str) -> bool {
    let needle = format!("goto L{n}");
    find_word_bounded(l, &needle).is_some()
}

fn goto_to_break(l: &str, n: &str) -> String {
    let needle = format!("goto L{n}");
    let b = l.as_bytes();
    let nb = needle.as_bytes();
    let mut out = String::with_capacity(l.len());
    let mut i = 0;
    let mut plain = 0;
    while let Some(p) = find_from(b, nb, i) {
        let e = p + nb.len();
        if boundary_before(l, p) && boundary_after(l, e) {
            out.push_str(&l[plain..p]);
            out.push_str("break");
            plain = e;
            i = e;
        } else {
            i = p + 1;
        }
    }
    out.push_str(&l[plain..]);
    out
}

/// `\bgoto L\d+\b` anywhere.
fn has_any_goto(l: &str) -> bool {
    let b = l.as_bytes();
    let mut i = 0;
    while let Some(p) = find_from(b, b"goto L", i) {
        let mut e = p + 6;
        let st = e;
        while e < b.len() && b[e].is_ascii_digit() {
            e += 1;
        }
        if e > st && boundary_before(l, p) && boundary_after(l, e) {
            return true;
        }
        i = p + 1;
    }
    false
}

/// `\bWORD` at p, then `(break|continue)\b`.
fn exit_word_at(s: &str, p: usize) -> bool {
    for w in ["break", "continue"] {
        if s[p..].starts_with(w) && boundary_after(s, p + w.len()) {
            return true;
        }
    }
    false
}

/// `(^|\bthen |\bdo )(break|continue)\b`
fn has_exit_word(s: &str) -> bool {
    let b = s.as_bytes();
    if exit_word_at(s, 0) {
        return true;
    }
    for head in [&b"then "[..], &b"do "[..]] {
        let mut i = 0;
        while let Some(p) = find_from(b, head, i) {
            if boundary_before(s, p) && exit_word_at(s, p + head.len()) {
                return true;
            }
            i = p + 1;
        }
    }
    false
}

/// `^(nil|true|false|-?[0-9][0-9.e+-]*|"[^"]*"|\{\})$`
fn simple_const(s: &str) -> bool {
    if matches!(s, "nil" | "true" | "false" | "{}") {
        return true;
    }
    let b = s.as_bytes();
    if b.len() >= 2 && b[0] == b'"' && b[b.len() - 1] == b'"' && !b[1..b.len() - 1].contains(&b'"') {
        return true;
    }
    let i = if b.first() == Some(&b'-') { 1 } else { 0 };
    i < b.len()
        && b[i].is_ascii_digit()
        && b[i + 1..].iter().all(|c| c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'+' | b'-'))
}

/// Block openers between line g and the label's block (indent `lim`).
fn enumerate_openers(lines: &[String], g: usize, lim: usize) -> Vec<usize> {
    let mut found = Vec::new();
    let mut cur = indent_of(&lines[g]);
    for k in (0..g).rev() {
        let s = &lines[k];
        if is_blank(s) {
            continue;
        }
        let ki = indent_of(s);
        if ki < cur {
            if ki < lim {
                break;
            }
            let st = strip(s);
            if !st.starts_with("else") {
                found.push(k);
            }
            cur = ki;
            if ki == lim {
                break;
            }
        }
    }
    found
}

/// `^(if|for|while|repeat|do|local function)\b`
fn starts_block(l: &str) -> bool {
    for w in ["if", "for", "while", "repeat", "do", "local function"] {
        if l.starts_with(w) && boundary_after(l, w.len()) {
            return true;
        }
    }
    false
}

/// `^(IDENT) = (.+)$` / `(.*)$`: name and value.
fn simple_assign(s: &str) -> Option<(&str, &str)> {
    let b = s.as_bytes();
    if b.is_empty() || !is_start(b[0]) {
        return None;
    }
    let e = word_end(b, 0);
    let rest = s[e..].strip_prefix(" = ")?;
    if rest.contains('\n') {
        return None;
    }
    Some((&s[..e], rest))
}

/// `if C then ...; return else E end`: nothing runs after the first arm, so E
/// can follow the `if`, and a jump into E has no arm to enter. The compiler
/// gives both forms the same code after `return`, but not after `break`.
fn hoist_else(lines: &mut Vec<String>, li: usize, gotos: &[usize]) -> bool {
    for h in enclosing(lines, li) {
        if strip(&lines[h]) != "else" {
            continue;
        }
        let Some(top) = if_of(lines, h) else { continue };
        let Some(stop) = if_end(lines, h) else { continue };
        if !strip(&lines[top]).starts_with("if ") || arm_end(lines, top) != h {
            continue; // `elseif` arms: the else depends on them
        }
        if gotos.iter().all(|g| h < *g && *g < stop) {
            continue;
        }
        let inner = indent_of(&lines[top]) + 4;
        let last = lines[top + 1..h].iter().rev().find(|l| !is_blank(l));
        let leaves = last.is_some_and(|l| {
            let s = strip(l);
            indent_of(l) == inner && (s == "return" || s.starts_with("return "))
        });
        if !leaves {
            continue;
        }
        let pad = spaces(indent_of(&lines[top]));
        lines[stop] = String::new();
        lines[h] = format!("{pad}end");
        for l in &mut lines[h + 1..stop] {
            if let Some(rest) = l.strip_prefix("    ") {
                *l = rest.to_string();
            }
        }
        lines.remove(stop);
        return true;
    }
    false
}

/// Gotos that remain are mostly the early returns of local functions the
/// compiler inlined: a jump forward to the end of the inlined body.
///   * no loop in between: `repeat <body> until true`, the gotos `break`
///   * out of a search loop whose fall-through just sets defaults: the
///     defaults move before the loop and the goto becomes `break`
/// Returns the new lines, or None while any goto is left.
/// Luau has no goto; these are what is left of compound conditions and of
/// local functions the compiler inlined, so every jump goes forward. The neat
/// forms come first, then the general method, and a jump backward is left for
/// the dispatcher. Returns the new lines, or None while any goto is left.
pub fn resolve_gotos(mut lines: Vec<String>) -> Option<Vec<String>> {
    for _ in 0..400 {
        let mut label = None;
        for (i, l) in lines.iter().enumerate() {
            if let Some((ind, n)) = parse_label(l) {
                label = Some((i, ind, n.to_string()));
                break;
            }
        }
        let Some((li, lab_ind, n)) = label else { break };
        let gotos: Vec<usize> =
            (0..lines.len()).filter(|&j| has_goto(&lines[j], &n)).collect();
        if gotos.is_empty() {
            lines.remove(li);
            continue;
        }
        if neat_goto(&mut lines, li, lab_ind, &n, &gotos).is_none() {
            let mut hoisted = false;
            while let Some(at) = lines.iter().position(|l| parse_label(l).is_some_and(|p| p.1 == n)) {
                let now: Vec<usize> = (0..lines.len()).filter(|&j| has_goto(&lines[j], &n)).collect();
                if !hoist_else(&mut lines, at, &now) {
                    break;
                }
                hoisted = true;
            }
            if !hoisted {
                lines = eliminate_label(lines, &n)?;
            }
        }
    }
    if lines.iter().any(|l| has_any_goto(l) || parse_label(l).is_some()) {
        return None;
    }
    Some(lines)
}

/// The jumps to one label, when they have a tidy form:
///   * no loop in between: `repeat <body> until true`, the gotos `break`
///   * out of a search loop whose fall-through just sets defaults: the
///     defaults move before the loop and the goto becomes `break`
fn neat_goto(lines: &mut Vec<String>, li: usize, lab_ind: usize, n: &str, gotos: &[usize]) -> Option<()> {
    // a short ending that leaves the function: copy it to every jump
    let mut tail: Vec<String> = Vec::new();
    let mut k = li + 1;
    while k < lines.len() && (is_blank(&lines[k]) || indent_of(&lines[k]) >= lab_ind) {
        if !is_blank(&lines[k]) {
            tail.push(lines[k][lab_ind..].to_string());
        }
        k += 1;
    }
    let simple = tail.iter().all(|l| {
        indent_of(l) == 0
            && !starts_block(l)
            && !l.contains("function")
            && !l.contains("::")
            && !l.contains("goto")
    });
    if !tail.is_empty()
        && simple
        && tail.len() <= 6
        && (tail[tail.len() - 1].starts_with("return") || k >= lines.len())
    {
        let mut body = tail.clone();
        if !body[body.len() - 1].starts_with("return") {
            body.push("return".to_string());
        }
        let cond_tail = format!(" then goto L{n} end");
        let bare = format!("goto L{n}");
        let mut new = Vec::with_capacity(lines.len() + gotos.len() * body.len());
        for (j, l) in lines.iter().enumerate() {
            if j == li {
                continue;
            }
            let ind = indent_of(l);
            let s = &l[ind..];
            if s.starts_with("if ")
                && s.ends_with(cond_tail.as_str())
                && s.len() >= 3 + cond_tail.len()
            {
                let cond = &s[3..s.len() - cond_tail.len()];
                let pad = &l[..ind];
                new.push(format!("{pad}if {cond} then"));
                for x in &body {
                    new.push(format!("{pad}    {x}"));
                }
                new.push(format!("{pad}end"));
                continue;
            }
            if s == bare {
                let pad = &l[..ind];
                for x in &body {
                    new.push(format!("{pad}{x}"));
                }
                continue;
            }
            new.push(l.clone());
        }
        if !new.iter().any(|l| has_goto(l, n)) {
            *lines = new;
            return Some(());
        }
    }
    if gotos.iter().any(|&g| g > li) {
        return None;
    }
    let pad = spaces(lab_ind);
    // the statement (at the label's indent) holding the first goto
    let g0 = gotos[0];
    let mut start = None;
    for k in (0..=g0).rev() {
        let s = &lines[k];
        if is_blank(s) {
            continue;
        }
        let ki = indent_of(s);
        if ki < lab_ind {
            start = Some(k + 1);
            break;
        }
        let st = strip(s);
        if ki == lab_ind
            && !(st.starts_with("else")
                || st.starts_with("end")
                || st.starts_with("until")
                || st.starts_with("::"))
        {
            start = Some(k);
            break;
        }
    }
    let start = start.unwrap_or(0);
    if indent_of(&lines[g0]) < lab_ind {
        return None;
    }
    // the label is not in the block the jumps start from
    for l in &lines[start..li] {
        if !is_blank(l) {
            let ki = indent_of(l);
            if ki < lab_ind || (ki == lab_ind && strip(l).starts_with("else")) {
                return None;
            }
        }
    }
    let loops: Vec<Vec<usize>> = gotos
        .iter()
        .map(|&g| {
            enumerate_openers(&lines, g, lab_ind)
                .into_iter()
                .filter(|&k| loop_open(&lines[k]))
                .collect()
        })
        .collect();
    if loops.iter().all(|v| v.is_empty()) {
        // repeat ... until true around [start, li); a break/continue in
        // there that belongs to an outer loop would be caught by it
        for k in start..li {
            if has_exit_word(strip(&lines[k]))
                && !enumerate_openers(&lines, k, lab_ind + 4)
                    .iter()
                    .any(|&o| loop_open(&lines[o]))
            {
                return None;
            }
        }
        let mut later = HashSet::default();
        for l in &lines[li + 1..] {
            words_into(&mut later, l);
        }
        let mut hoist: Vec<String> = Vec::new();
        let mut new_body: Vec<String> = Vec::with_capacity(li - start);
        for l in &lines[start..li] {
            let s = strip(l);
            let m = if indent_of(l) == lab_ind { parse_local_list(s) } else { None };
            let mut rewritten: Option<String> = None;
            if let Some((names, value)) = m {
                let list: Vec<&str> = names.split(", ").collect();
                if list.iter().any(|x| later.contains(*x)) {
                    for x in &list {
                        hoist.push(x.to_string());
                    }
                    match value {
                        Some(v) => rewritten = Some(format!("{pad}{names}{v}")),
                        None => continue,
                    }
                }
            }
            let text = rewritten.as_deref().unwrap_or(l);
            let text = goto_to_break(text, &n);
            new_body.push(if is_blank(&text) { text } else { format!("    {text}") });
        }
        let mut repl = Vec::with_capacity(new_body.len() + 3);
        if !hoist.is_empty() {
            let mut seen = HashSet::default();
            let uniq: Vec<&str> = hoist
                .iter()
                .filter(|x| seen.insert(x.as_str()))
                .map(|x| x.as_str())
                .collect();
            repl.push(format!("{pad}local {}", uniq.join(", ")));
        }
        repl.push(format!("{pad}repeat"));
        repl.extend(new_body);
        repl.push(format!("{pad}until true"));
        lines.splice(start..=li, repl);
        return Some(());
    }
    // a search loop: every goto sits directly in the same loop, which the
    // label follows after a few constant assignments
    let lo = {
        let mut heads: Vec<usize> =
            loops.iter().filter(|v| !v.is_empty()).map(|v| v[0]).collect();
        heads.sort_unstable();
        heads.dedup();
        if heads.len() != 1 || loops.iter().any(|v| v.len() != 1) {
            return None;
        }
        heads[0]
    };
    if indent_of(&lines[lo]) != lab_ind {
        return None;
    }
    let mut lend = None;
    for k in lo + 1..li {
        if !is_blank(&lines[k]) && indent_of(&lines[k]) <= lab_ind {
            lend = Some(k);
            break;
        }
    }
    let lend = lend?;
    if strip(&lines[lend]) != "end" {
        return None;
    }
    let tail_lines: Vec<String> = lines[lend + 1..li].to_vec();
    let mut targets: Vec<&str> = Vec::new();
    for l in &tail_lines {
        let rest = l.strip_prefix(pad.as_str())?;
        let (name, value) = simple_assign(rest)?;
        if value.is_empty() || !simple_const(value) {
            return None;
        }
        targets.push(name);
    }
    let mut loop_words = HashSet::default();
    for l in &lines[lo..=lend] {
        match simple_assign(strip(l)) {
            Some((_, value)) => words_into(&mut loop_words, value),
            None => words_into(&mut loop_words, l),
        }
    }
    if targets.iter().any(|t| loop_words.contains(*t)) {
        return None; // the loop reads a default it would now see early
    }
    let mut repl = tail_lines.clone();
    for l in &lines[lo..=lend] {
        repl.push(goto_to_break(l, &n));
    }
    lines.splice(lo..=li, repl);
    Some(())
}

// Forward gotos, after Erosa and Hendren: a jump moves out of blocks and into
// them, carried by a flag, until it stands beside its label and becomes `if`.
// Blocks are read from indentation, which is regular here.

fn parent_line(lines: &[String], i: usize) -> Option<usize> {
    let ind = indent_of(&lines[i]);
    (0..i).rev().find(|&k| !is_blank(&lines[k]) && indent_of(&lines[k]) < ind)
}

/// The block headers around line i, innermost first.
fn enclosing(lines: &[String], i: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut at = i;
    while let Some(h) = parent_line(lines, at) {
        out.push(h);
        at = h;
    }
    out
}

/// The line ending the block opened at h: its `end`, `until`, `else`...
fn arm_end(lines: &[String], h: usize) -> usize {
    let ind = indent_of(&lines[h]);
    (h + 1..lines.len())
        .find(|&k| !is_blank(&lines[k]) && indent_of(&lines[k]) <= ind)
        .unwrap_or(lines.len())
}

fn is_arm(s: &str) -> bool {
    s == "else" || s.starts_with("elseif ")
}

/// The `if` line of the chain the arm h belongs to.
fn if_of(lines: &[String], h: usize) -> Option<usize> {
    let ind = indent_of(&lines[h]);
    let mut at = h;
    while is_arm(strip(&lines[at])) {
        at = (0..at).rev().find(|&k| !is_blank(&lines[k]) && indent_of(&lines[k]) <= ind)?;
        if indent_of(&lines[at]) != ind {
            return None;
        }
    }
    strip(&lines[at]).starts_with("if ").then_some(at)
}

/// The `end` of the chain the arm h belongs to.
fn if_end(lines: &[String], h: usize) -> Option<usize> {
    let mut e = arm_end(lines, h);
    while e < lines.len() && is_arm(strip(&lines[e])) {
        e = arm_end(lines, e);
    }
    (e < lines.len() && strip(&lines[e]) == "end").then_some(e)
}

/// `if C then goto L end` or `goto L`: Some(C) or None.
fn goto_condition(line: &str, n: &str) -> Option<Option<String>> {
    let s = strip(line);
    if s == format!("goto L{n}") {
        return Some(None);
    }
    let tail = format!(" then goto L{n} end");
    let c = s.strip_prefix("if ")?.strip_suffix(tail.as_str())?;
    Some(Some(c.to_string()))
}

fn negated(c: &str) -> String {
    if is_token(c) {
        format!("not {c}")
    } else {
        format!("not ({c})")
    }
}

fn indented(lines: &[String]) -> Vec<String> {
    lines.iter().map(|l| if is_blank(l) { l.clone() } else { format!("    {l}") }).collect()
}

/// Lines [from, to) at indent ind run only when `test` holds. Locals they
/// declare for what follows their block move out in front of it.
fn guard_lines(
    lines: &mut Vec<String>,
    from: usize,
    to: usize,
    ind: usize,
    test: &str,
    block_stop: usize,
) -> Option<()> {
    let mut later = HashSet::default();
    for l in &lines[to..block_stop.min(lines.len())] {
        words_into(&mut later, l);
    }
    let pad = spaces(ind);
    let (mut hoist, mut body): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for l in &lines[from..to] {
        if indent_of(l) == ind && !is_blank(l) {
            let s = strip(l);
            if let Some((names, value)) = parse_local_list(s) {
                if names.split(", ").any(|x| later.contains(x)) {
                    hoist.extend(names.split(", ").map(String::from));
                    if let Some(v) = value {
                        body.push(format!("{pad}{names}{v}"));
                    }
                    continue;
                }
            } else if let Some(rest) = s.strip_prefix("local function ") {
                let name = &rest[..rest.find('(')?];
                if later.contains(name) {
                    return None;
                }
            }
        }
        body.push(l.clone());
    }
    let mut new = Vec::with_capacity(body.len() + 3);
    if !hoist.is_empty() {
        new.push(format!("{pad}local {}", hoist.join(", ")));
    }
    new.push(format!("{pad}if {test} then"));
    new.extend(indented(&body));
    new.push(format!("{pad}end"));
    lines.splice(from..to, new);
    Some(())
}

/// After the line at `after`, unless that is said there already.
fn check_flag_after(lines: &mut Vec<String>, after: usize, ind: usize, flag: &str, n: &str) {
    let text = format!("{}if {flag} then goto L{n} end", spaces(ind));
    if lines.get(after + 1) != Some(&text) {
        lines.insert(after + 1, text);
    }
}

fn holds_label(lines: &[String], from: usize, to: usize) -> bool {
    lines[from..to].iter().any(|l| parse_label(l).is_some())
}

/// `if C then` or `elseif C then`: the word and C.
fn arm_condition(s: &str) -> Option<(&'static str, &str)> {
    let (word, c) = match s.strip_prefix("if ") {
        Some(c) => ("if", c),
        None => ("elseif", s.strip_prefix("elseif ")?),
    };
    Some((word, c.strip_suffix(" then")?))
}

fn eliminate_label(mut lines: Vec<String>, n: &str) -> Option<Vec<String>> {
    let flag = format!("jump_{n}");
    let mut declared = false;
    for _ in 0..2000 {
        let li = lines.iter().position(|l| parse_label(l).is_some_and(|p| p.1 == n))?;
        let Some(g) = lines.iter().position(|l| has_goto(l, n)) else {
            lines.remove(li);
            // the jumps may have gone without needing it
            if lines.iter().filter(|l| has_word(l, &flag)).count() == 1 {
                lines.retain(|l| !has_word(l, &flag));
            }
            return Some(lines);
        };
        if g > li {
            return None; // a loop the decompiler did not see as one
        }
        let cond = goto_condition(&lines[g], n)?;
        let (around_g, around_l) = (enclosing(&lines, g), enclosing(&lines, li));
        let shared = around_g.iter().rev().zip(around_l.iter().rev()).take_while(|(a, b)| a == b).count();
        let ind = indent_of(&lines[g]);
        let pad = spaces(ind);
        let carried = cond.as_deref() == Some(flag.as_str());
        // the flag lives in the innermost block holding the jump and the label
        if !declared {
            let mut top = if around_g.len() == shared { g } else { around_g[around_g.len() - shared - 1] };
            if is_arm(strip(&lines[top])) {
                top = if_of(&lines, top)?;
            }
            lines.insert(top, format!("{}local {flag} = false", spaces(indent_of(&lines[top]))));
            declared = true;
            continue;
        }
        if shared < around_g.len() && shared < around_l.len() {
            // to a later arm of the same `if`: the end of that arm becomes
            // an `if` of its own after the chain, which that arm also enters
            let (from, to) = (around_g[around_g.len() - shared - 1], around_l[around_l.len() - shared - 1]);
            if if_of(&lines, from).is_some() && if_of(&lines, from) == if_of(&lines, to) {
                if around_l.len() != shared + 1 {
                    return None; // deeper in that arm
                }
                let e = arm_end(&lines, to);
                let stop = if_end(&lines, to)?;
                // what the arm declares before the label must not be needed after it
                let mut tail_words = HashSet::default();
                for l in &lines[li + 1..e] {
                    words_into(&mut tail_words, l);
                }
                let label_ind = indent_of(&lines[li]);
                for l in &lines[to + 1..li] {
                    if indent_of(l) != label_ind {
                        continue;
                    }
                    if let Some((names, _)) = parse_local_list(strip(l)) {
                        if names.split(", ").any(|x| tail_words.contains(x)) {
                            return None;
                        }
                    } else if strip(l).starts_with("local function ") {
                        return None;
                    }
                }
                let head = spaces(indent_of(&lines[to]));
                let mut lifted = vec![format!("{head}if {flag} then"), lines[li].clone()];
                lifted.extend(lines[li + 1..e].iter().cloned());
                lifted.push(format!("{head}end"));
                lines.splice(stop + 1..stop + 1, lifted);
                lines.splice(li..e, [format!("{}{flag} = true", spaces(label_ind))]);
                continue;
            }
        }
        if shared < around_g.len() {
            // out of the innermost block around the jump
            let h = around_g[0];
            let head = indent_of(&lines[h]);
            let raise = match &cond {
                Some(c) if !carried => vec![
                    format!("{pad}if {c} then"),
                    format!("{pad}    {flag} = true"),
                ],
                Some(c) => vec![format!("{pad}if {c} then")],
                None => vec![format!("{pad}{flag} = true")],
            };
            if loop_open(&lines[h]) {
                let e = arm_end(&lines, h);
                if e >= lines.len() {
                    return None;
                }
                check_flag_after(&mut lines, e, head, &flag, n);
                let mut new = raise;
                new.push(format!("{pad}{}break", if cond.is_some() { "    " } else { "" }));
                if cond.is_some() {
                    new.push(format!("{pad}end"));
                }
                lines.splice(g..=g, new);
                continue;
            }
            let s = strip(&lines[h]);
            if !(s.starts_with("if ") || is_arm(s)) {
                return None;
            }
            let e = arm_end(&lines, h);
            let stop = if_end(&lines, h)?;
            let rest: Vec<String> = lines[g + 1..e].to_vec();
            let empty = rest.iter().all(|l| is_blank(l));
            if holds_label(&lines, g + 1, e) {
                return None;
            }
            check_flag_after(&mut lines, stop, head, &flag, n);
            let mut new = Vec::with_capacity(rest.len() + 4);
            match &cond {
                None => new.push(format!("{pad}{flag} = true")), // what follows never runs
                Some(_) if empty && carried => {} // the check after the block says it
                Some(c) if carried => {
                    new.push(format!("{pad}if {} then", negated(c)));
                    new.extend(indented(&rest));
                    new.push(format!("{pad}end"));
                }
                Some(c) => {
                    new.push(format!("{pad}if {c} then"));
                    new.push(format!("{pad}    {flag} = true"));
                    if !empty {
                        new.push(format!("{pad}else"));
                        new.extend(indented(&rest));
                    }
                    new.push(format!("{pad}end"));
                }
            }
            lines.splice(g..e, new);
            continue;
        }
        // the jump's block holds the label: reach the statement it is in
        let block_stop = around_g.first().map_or(lines.len(), |h| arm_end(&lines, *h));
        if around_l.len() == around_g.len() {
            // beside it: what lies between runs only without the jump
            let blank = lines[g + 1..li].iter().all(|l| is_blank(l));
            match &cond {
                None => {
                    if holds_label(&lines, g + 1, li) {
                        return None;
                    }
                    lines.drain(g..li);
                }
                Some(c) if blank => {
                    let keep = c.contains('(').then(|| format!("{pad}if {c} then end"));
                    lines.splice(g..li, keep);
                }
                Some(c) => {
                    let test = negated(c);
                    lines.remove(g);
                    guard_lines(&mut lines, g, li - 1, ind, &test, block_stop)?;
                }
            }
            continue;
        }
        // into a block after it: set the flag, skip what lies before that
        // `if`, and open the arm to the flag
        let depth = around_g.len();
        let arm_of = |lines: &[String]| -> Option<usize> {
            let li = lines.iter().position(|l| parse_label(l).is_some_and(|p| p.1 == n))?;
            let around = enclosing(lines, li);
            around.get(around.len() - depth - 1).copied()
        };
        if loop_open(&lines[arm_of(&lines)?]) {
            return None; // into a loop: no source does that
        }
        let raised = match &cond {
            Some(_) if carried => Vec::new(),
            Some(c) => vec![
                format!("{pad}if {c} then"),
                format!("{pad}    {flag} = true"),
                format!("{pad}end"),
            ],
            None => vec![format!("{pad}{flag} = true")],
        };
        let after = g + raised.len();
        lines.splice(g..=g, raised);
        let first = if_of(&lines, arm_of(&lines)?)?;
        if lines[after..first].iter().any(|l| !is_blank(l)) {
            if cond.is_none() {
                if holds_label(&lines, after, first) {
                    return None;
                }
                lines.drain(after..first);
            } else {
                let stop = around_g.first().map_or(lines.len(), |h| arm_end(&lines, *h));
                guard_lines(&mut lines, after, first, ind, &negated(&flag), stop)?;
            }
        }
        let arm = arm_of(&lines)?;
        let first = if_of(&lines, arm)?;
        // the arms before it test nothing when the flag is up
        let mut k = first;
        while k < arm {
            let s = strip(&lines[k]).to_string();
            let p = spaces(indent_of(&lines[k]));
            let (word, c) = arm_condition(&s)?;
            lines[k] = format!("{p}{word} not {flag} and ({c}) then");
            k = arm_end(&lines, k);
        }
        let s = strip(&lines[arm]).to_string();
        let p = spaces(indent_of(&lines[arm]));
        if s != "else" {
            let (word, c) = arm_condition(&s)?;
            if c != flag {
                let c = if c.starts_with("if ") { format!("({c})") } else { c.to_string() };
                lines[arm] = format!("{p}{word} {flag} or {c} then");
            }
        }
        lines.insert(arm + 1, format!("{p}    if {flag} then goto L{n} end"));
    }
    None
}

// ------------------------------------------------------------ if-expressions

/// `^ident(\.ident)*$`
fn is_lvalue(s: &str) -> bool {
    ident_chain_end(s.as_bytes(), 0) == Some(s.len())
}

fn wrap_value(v: &str) -> String {
    if v.starts_with("if ") || v.starts_with("function") {
        format!("({v})")
    } else {
        v.to_string()
    }
}

/// `if c then x = a elseif d then x = b else x = c end`, one assignment per
/// branch, becomes `x = if c then a elseif d then b else c`.
pub fn if_expressions(lines: Vec<String>) -> Vec<String> {
    const LIMIT: usize = 150;
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        let ind = indent_of(line);
        let s = strip(line);
        let mut got: Option<(String, Vec<String>, Vec<String>, usize)> = None;
        if s.starts_with("if ") && s.ends_with(" then") {
            let pad = spaces(ind);
            let mut conds = vec![inner(s, 3, 5).to_string()];
            let mut vals: Vec<String> = Vec::new();
            let mut name: Option<String> = None;
            let mut j = i + 1;
            loop {
                if j + 1 >= lines.len() {
                    break;
                }
                let (body, nxt) = (&lines[j], &lines[j + 1]);
                if indent_of(body) != ind + 4 || !body.contains(" = ") {
                    break;
                }
                let Some((lhs, val)) = strip(body).split_once(" = ") else { break };
                if lhs.starts_with("local ")
                    || !is_lvalue(lhs)
                    || name.as_deref().is_some_and(|n| n != lhs)
                {
                    break;
                }
                name = Some(lhs.to_string());
                vals.push(val.to_string());
                let ns = strip(nxt);
                if indent_of(nxt) != ind {
                    break;
                } else if ns.starts_with("elseif ") && ns.ends_with(" then") {
                    conds.push(inner(ns, 7, 5).to_string());
                    j += 2;
                } else if ns == "else" {
                    j += 2;
                    if j + 1 < lines.len()
                        && indent_of(&lines[j]) == ind + 4
                        && lines[j + 1].len() == pad.len() + 3
                        && lines[j + 1].starts_with(&pad)
                        && lines[j + 1].ends_with("end")
                    {
                        let st = strip(&lines[j]);
                        let (lhs, val) = st.split_once(" = ").unwrap_or((st, ""));
                        if Some(lhs) == name.as_deref() {
                            vals.push(val.to_string());
                            got = Some((name.clone().unwrap(), conds.clone(), vals.clone(), j + 2));
                        }
                    }
                    break;
                } else {
                    break;
                }
            }
            if let Some((name, conds, vals, after)) = got {
                let vals: Vec<String> = vals.iter().map(|v| wrap_value(v)).collect();
                let mut expr = format!("if {} then {}", conds[0], vals[0]);
                for (c, v) in conds[1..].iter().zip(vals[1..vals.len() - 1].iter()) {
                    expr.push_str(&format!(" elseif {c} then {v}"));
                }
                expr.push_str(&format!(" else {}", vals[vals.len() - 1]));
                let text = format!("{pad}{name} = {expr}");
                if text.chars().count() <= LIMIT && vals.iter().all(|v| !v.contains('\n')) {
                    out.push(text);
                    i = after;
                    continue;
                }
            }
        }
        out.push(line.clone());
        i += 1;
    }
    out
}

// ---------------------------------------------------------------- uninlining

/// (name, params, first body line, end line) of each `local NAME = function`.
fn function_bodies(lines: &[String]) -> Vec<(String, Vec<String>, usize, usize)> {
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        // ^( *)local (IDENT) = function\(([^)]*)\)$
        let ind = indent_of(line);
        let Some(r) = line[ind..].strip_prefix("local ") else { continue };
        let b = r.as_bytes();
        if b.is_empty() || !is_start(b[0]) {
            continue;
        }
        let e = word_end(b, 0);
        let Some(rest) = r[e..].strip_prefix(" = function(") else { continue };
        let Some(params) = rest.strip_suffix(')') else { continue };
        if params.contains(')') {
            continue;
        }
        let pad = &line[..ind];
        for (j, l) in lines.iter().enumerate().skip(i + 1) {
            if l.len() == ind + 3 && l.starts_with(pad) && l.ends_with("end") {
                found.push((r[..e].to_string(), split_params(params), i + 1, j));
                break;
            }
        }
    }
    found
}

fn top_level_comma(s: &str) -> bool {
    let mut depth = 0;
    let mut quote = 0u8;
    for ch in s.bytes() {
        if quote != 0 {
            if ch == quote {
                quote = 0;
            }
        } else if ch == b'"' || ch == b'\'' {
            quote = ch;
        } else if matches!(ch, b'(' | b'[' | b'{') {
            depth += 1;
        } else if matches!(ch, b')' | b']' | b'}') {
            depth -= 1;
        } else if ch == b',' && depth == 0 {
            return true;
        }
    }
    false
}

/// The expression a helper returns, when its body is only returns:
/// `return e`, or `if c then return a end ... return b` (an if-expression).
fn return_template(body: &[String]) -> Option<String> {
    let pad = body.first().map_or(0, |l| indent_of(l));
    let lines: Vec<&str> = body.iter().map(|l| l.get(pad..).unwrap_or("")).collect();
    if lines.len() == 1 && lines[0].starts_with("return ") && !top_level_comma(&lines[0][7..]) {
        return Some(lines[0][7..].to_string());
    }
    let mut conds: Vec<&str> = Vec::new();
    let mut vals: Vec<&str> = Vec::new();
    let mut i = 0;
    while i + 2 < lines.len()
        && lines[i].starts_with("if ")
        && lines[i].ends_with(" then")
        && lines[i + 1].starts_with("    return ")
        && lines[i + 2] == "end"
    {
        conds.push(inner(lines[i], 3, 5));
        vals.push(&lines[i + 1][11..]);
        i += 3;
    }
    if !conds.is_empty() && i == lines.len() - 1 && lines[i].starts_with("return ") {
        vals.push(&lines[i][7..]);
        let mut expr = format!("if {} then {}", conds[0], vals[0]);
        for (c, v) in conds[1..].iter().zip(vals[1..vals.len() - 1].iter()) {
            expr.push_str(&format!(" elseif {c} then {v}"));
        }
        expr.push_str(&format!(" else {}", vals[vals.len() - 1]));
        return Some(expr);
    }
    None
}

/// `^-?[A-Za-z_0-9.]+(\([^()]*\)|\[[^\[\]]*\]|\.ident|:ident\([^()]*\))*$`
fn is_atom(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    if b.first() == Some(&b'-') {
        i = 1;
    }
    let st = i;
    while i < b.len() && (is_word(b[i]) || b[i] == b'.') {
        i += 1;
    }
    if i == st {
        return false;
    }
    let paren_body = |mut k: usize| -> Option<usize> {
        // k just after `(`: [^()]* then `)`
        while k < b.len() && b[k] != b'(' && b[k] != b')' {
            k += 1;
        }
        if k < b.len() && b[k] == b')' {
            Some(k + 1)
        } else {
            None
        }
    };
    while i < b.len() {
        match b[i] {
            b'(' => match paren_body(i + 1) {
                Some(e) => i = e,
                None => return false,
            },
            b'[' => {
                let mut k = i + 1;
                while k < b.len() && b[k] != b'[' && b[k] != b']' {
                    k += 1;
                }
                if k < b.len() && b[k] == b']' {
                    i = k + 1;
                } else {
                    return false;
                }
            }
            b'.' => {
                if i + 1 < b.len() && is_start(b[i + 1]) {
                    i = word_end(b, i + 1);
                } else {
                    return false;
                }
            }
            b':' => {
                if i + 1 < b.len() && is_start(b[i + 1]) {
                    let e = word_end(b, i + 1);
                    if e < b.len() && b[e] == b'(' {
                        match paren_body(e + 1) {
                            Some(x) => i = x,
                            None => return false,
                        }
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            _ => return false,
        }
    }
    !s.contains('\n')
}

fn balanced_all(s: &str) -> bool {
    let mut depth = 0;
    let mut quote = 0u8;
    for ch in s.bytes() {
        if quote != 0 {
            if ch == quote {
                quote = 0;
            }
            continue;
        }
        if ch == b'"' || ch == b'\'' {
            quote = ch;
        } else if matches!(ch, b'(' | b'[' | b'{') {
            depth += 1;
        } else if matches!(ch, b')' | b']' | b'}') {
            depth -= 1;
            if depth < 0 {
                return false;
            }
        }
    }
    depth == 0 && quote == 0
}

#[derive(PartialEq)]
enum Tok {
    Lit(String),
    Hole(usize),
}

/// Split a template into literal text and parameter holes.
fn compile_template(expr: &str, params: &[String]) -> Vec<Tok> {
    let b = expr.as_bytes();
    let mut toks = Vec::new();
    let mut plain = 0;
    let mut i = 0;
    while i < b.len() {
        if is_word(b[i]) {
            let e = word_end(b, i);
            if let Some(p) = params.iter().position(|x| x.as_str() == &expr[i..e]) {
                if i > plain {
                    toks.push(Tok::Lit(expr[plain..i].to_string()));
                }
                toks.push(Tok::Hole(p));
                plain = e;
            }
            i = e;
        } else {
            i += 1;
        }
    }
    if plain < b.len() {
        toks.push(Tok::Lit(expr[plain..].to_string()));
    }
    toks
}

/// Can an argument printed here stand alone? Inside parentheses or between
/// commas anything goes; elsewhere only a simple operand does.
fn hole_ok(text: &str, before: &str, after: &str) -> bool {
    if text.is_empty() || !balanced_all(text) || text != strip(text) {
        return false;
    }
    let arg_slot = (before.ends_with('(') || before.ends_with(", "))
        && (after.starts_with(',') || after.starts_with(')'));
    arg_slot || is_atom(text)
}

fn match_at(line: &str, i: usize, toks: &[Tok], nparams: usize) -> Option<(usize, Vec<String>)> {
    let mut caps: Vec<Option<&str>> = vec![None; nparams];
    let mut pos = i;
    for (k, tok) in toks.iter().enumerate() {
        let hole = match tok {
            Tok::Lit(t) => {
                if !line[pos..].starts_with(t.as_str()) {
                    return None;
                }
                pos += t.len();
                continue;
            }
            Tok::Hole(h) => *h,
        };
        let before: &str = match k.checked_sub(1).map(|p| &toks[p]) {
            Some(Tok::Lit(t)) => t,
            _ => "",
        };
        let mut found = None;
        match toks.get(k + 1) {
            None => {
                // the hole ends the template: take the longest simple operand
                let mut e = line.len();
                while e > pos {
                    if line.is_char_boundary(e) && hole_ok(&line[pos..e], before, "") {
                        found = Some(e);
                        break;
                    }
                    e -= 1;
                }
            }
            Some(Tok::Lit(nxt)) => {
                let mut start = pos;
                while let Some(e) = find_from(line.as_bytes(), nxt.as_bytes(), start) {
                    if hole_ok(&line[pos..e], before, nxt) {
                        found = Some(e);
                        break;
                    }
                    start = e + 1;
                }
            }
            Some(Tok::Hole(_)) => return None,
        }
        let found = found?;
        let cap = &line[pos..found];
        if caps[hole].is_some_and(|c| c != cap) {
            return None;
        }
        caps[hole] = Some(cap);
        pos = found;
    }
    let mut out = Vec::with_capacity(nparams);
    for c in caps {
        out.push(c?.to_string());
    }
    Some((pos, out))
}

/// `^\s*([-+*/%^]|//|\.\.)?=(?!=)`
fn is_assignment_tail(s: &str) -> bool {
    let t = s.trim_start_matches(is_space);
    let b = t.as_bytes();
    let eq = |k: usize| b.get(k) == Some(&b'=') && b.get(k + 1) != Some(&b'=');
    if eq(0) {
        return true;
    }
    if matches!(b.first(), Some(b'-' | b'+' | b'*' | b'/' | b'%' | b'^')) && eq(1) {
        return true;
    }
    (t.starts_with("//") || t.starts_with("..")) && eq(2)
}

/// Luau inlines small local functions at -O2. Where the body of such a helper
/// appears verbatim with its parameters filled in, put the call back.
/// The helpers each line may call again: `--@copies NAME...` opens a
/// function's body with those whose code line info shows there. A function
/// without one has none, whatever is around it. (regions, region of each line)
fn copies_by_line(lines: &[String]) -> (Vec<Vec<String>>, Vec<usize>) {
    let mut regions: Vec<Vec<String>> = vec![Vec::new()];
    let mut open: Vec<(usize, usize)> = Vec::new(); // (header indent, region)
    let mut at = Vec::with_capacity(lines.len());
    for line in lines {
        if !is_blank(line) {
            let ind = indent_of(line);
            while open.last().is_some_and(|o| ind <= o.0) {
                open.pop();
            }
            if let Some(names) = line.trim_start().strip_prefix("--@copies ") {
                let here = open.last().map_or(0, |o| o.1);
                regions[here] = names.split(' ').filter(|n| !n.is_empty()).map(String::from).collect();
            }
        }
        at.push(open.last().map_or(0, |o| o.1));
        if func_open(line).is_some() && !line.trim_end().ends_with("end") {
            regions.push(Vec::new());
            open.push((indent_of(line), regions.len() - 1));
        }
    }
    (regions, at)
}

/// Text outside string literals.
fn code_only(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut quote = None;
    let mut esc = false;
    for c in text.chars() {
        match quote {
            Some(q) => {
                if esc {
                    esc = false;
                } else if c == '\\' {
                    esc = true;
                } else if c == q {
                    quote = None;
                }
            }
            None if c == '"' || c == '\'' || c == '`' => {
                quote = Some(c);
                out.push(' ');
            }
            None => out.push(c),
        }
    }
    out
}

/// Does this text call anything? `Vector3.new(0, 1, 0)` with numbers only is
/// how a vector constant prints, and no call.
fn has_call(text: &str) -> bool {
    let code = code_only(text);
    let b = code.as_bytes();
    (0..b.len()).any(|i| {
        let call = b[i] == b'(' && i > 0 && (is_word(b[i - 1]) || b[i - 1] == b')' || b[i - 1] == b']');
        if !call || !code[..i].ends_with("Vector3.new") {
            return call;
        }
        let args = code[i + 1..].split(')').next().unwrap_or("");
        !args.bytes().all(|c| c.is_ascii_digit() || b" ,.-+eE".contains(&c))
    })
}

/// Is what follows evaluated only sometimes?
fn branches(text: &str) -> bool {
    let code = code_only(text);
    let b = code.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if is_start(b[i]) && (i == 0 || !is_word(b[i - 1])) {
            let e = word_end(b, i);
            if matches!(&code[i..e], "and" | "or" | "if" | "then" | "else" | "elseif") {
                return true;
            }
            i = e;
        } else {
            i += 1;
        }
    }
    false
}

/// Does this text read anything a call could change: a name that is no
/// keyword, a field, an index?
fn reads_state(text: &str) -> bool {
    let code = code_only(text);
    let b = code.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if is_start(b[i]) && (i == 0 || !is_word(b[i - 1])) {
            let e = word_end(b, i);
            if !matches!(&code[i..e], "and" | "or" | "not" | "nil" | "true" | "false" | "if" | "then" | "else" | "elseif") {
                return true;
            }
            i = e;
        } else {
            i += 1;
        }
    }
    false
}

/// A call evaluates its arguments, in order, before the body runs; the copy
/// evaluated each where the body reads it. That only tells apart when an
/// argument calls something: then it must be read once, on every path, after
/// nothing the call could change, with the arguments in their order.
fn order_safe(toks: &[Tok], caps: &[String]) -> bool {
    let effects: Vec<bool> = caps.iter().map(|c| has_call(c)).collect();
    if !effects.contains(&true) {
        return true;
    }
    let mut seen: Vec<usize> = Vec::new();
    let (mut read, mut sometimes) = (false, false);
    for tok in toks {
        match tok {
            Tok::Lit(t) => {
                read |= reads_state(t);
                sometimes |= branches(t);
            }
            Tok::Hole(h) => {
                let first = !seen.contains(h);
                if effects[*h] && (!first || read || sometimes) {
                    return false;
                }
                if first {
                    if seen.last().is_some_and(|l| l > h) {
                        return false;
                    }
                    seen.push(*h);
                }
            }
        }
    }
    true
}

pub fn uninline(lines: Vec<String>) -> Vec<String> {
    let mut lines = lines;
    let (copies, region) = copies_by_line(&lines);
    let marked: Vec<bool> = lines.iter().map(|l| l.trim_start().starts_with("--@copies ")).collect();
    let mut templates = Vec::new();
    for (name, params, b0, b1) in function_bodies(&lines) {
        // the copies marker opening a helper that holds copies itself is no
        // statement of it
        let body: Vec<String> = (b0..b1).filter(|i| !marked[*i]).map(|i| lines[i].clone()).collect();
        let Some(expr) = return_template(&body) else { continue };
        if params.is_empty() {
            continue;
        }
        let toks = compile_template(&expr, &params);
        let literal: usize = toks
            .iter()
            .map(|t| match t {
                Tok::Lit(s) => s.chars().count(),
                Tok::Hole(_) => 0,
            })
            .sum();
        let used: HashSet<usize> = toks
            .iter()
            .filter_map(|t| match t {
                Tok::Hole(h) => Some(*h),
                Tok::Lit(_) => None,
            })
            .collect();
        if toks.is_empty()
            || !matches!(toks[0], Tok::Lit(_))
            || literal < 10
            || used.len() != params.len()
        {
            continue;
        }
        templates.push((name, toks, params.len(), b1));
    }
    // two helpers one could be a copy of: neither
    let twins: Vec<bool> = (0..templates.len())
        .map(|i| (0..templates.len()).any(|j| j != i && templates[j].1 == templates[i].1))
        .collect();
    let mut out = std::mem::take(&mut lines);
    for (k, (name, toks, n, def_end)) in templates.iter().enumerate() {
        if twins[k] {
            continue;
        }
        let Tok::Lit(head) = &toks[0] else { unreachable!() };
        let tail_alnum = match &toks[toks.len() - 1] {
            Tok::Lit(s) => s.chars().next_back().is_some_and(char::is_alphanumeric),
            Tok::Hole(_) => true,
        };
        for (j, line) in out.iter_mut().enumerate().skip(def_end + 1) {
            if !line.contains(head.as_str()) || !copies[region[j]].iter().any(|c| c == name) {
                continue;
            }
            loop {
                let mut changed = false;
                let mut at = find_from(line.as_bytes(), head.as_bytes(), 0);
                while let Some(i) = at {
                    let next = |l: &String| find_from(l.as_bytes(), head.as_bytes(), i + 1);
                    let b = line.as_bytes();
                    if i > 0 && (is_word(b[i - 1]) || b[i - 1] == b'.') {
                        at = next(line);
                        continue;
                    }
                    let mut m = match_at(line, i, toks, *n);
                    if m.as_ref().is_some_and(|x| is_assignment_tail(&line[x.0..])) {
                        m = None; // `f(x) = v` is not Luau: the body is a target here
                    }
                    if let Some((end, caps)) = m {
                        if (end < b.len() && is_word(b[end]) && tail_alnum) || !order_safe(toks, &caps) {
                            at = next(line);
                            continue;
                        }
                        *line = format!("{}{name}({}){}", &line[..i], caps.join(", "), &line[end..]);
                        changed = true;
                        break;
                    }
                    at = next(line);
                }
                if !changed {
                    break;
                }
            }
        }
    }
    let mut kept = Vec::with_capacity(out.len());
    for (line, m) in out.into_iter().zip(marked) {
        if !m {
            kept.push(line);
        }
    }
    kept
}

// ------------------------------------------------------------- simple passes

/// `^( *)for (.+) in (TOKEN), (TOKEN), (TOKEN) do$`
fn parse_for3(line: &str) -> Option<(usize, &str, &str, &str, &str)> {
    let ind = indent_of(line);
    let s = line[ind..].strip_prefix("for ")?.strip_suffix(" do")?;
    let b = s.as_bytes();
    let back = |e: usize| -> Option<usize> {
        // start of the token ending at e
        let mut k = e;
        while k > 0 && is_word(b[k - 1]) {
            k -= 1;
        }
        if k < e && is_start(b[k]) {
            Some(k)
        } else {
            None
        }
    };
    let c0 = back(b.len())?;
    let c1 = s[..c0].strip_suffix(", ")?.len();
    let b0 = back(c1)?;
    let b1 = s[..b0].strip_suffix(", ")?.len();
    let a0 = back(b1)?;
    let vars = s[..a0].strip_suffix(" in ")?;
    if vars.is_empty() || vars.contains('\n') {
        return None;
    }
    Some((ind, vars, &s[a0..b1], &s[b0..c1], &s[c0..]))
}

/// `local a = t; local b = nil; local c = nil; for k, v in a, b, c do` is how
/// a plain `for k, v in t do` compiles: put it back.
fn generic_for(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut i = 0;
    while i < out.len() {
        let mut repl: Option<Vec<String>> = None;
        if i >= 3 {
            if let Some((ind, vars, a, b, c)) = parse_for3(&out[i]) {
                let pad = spaces(ind);
                // ^{pad}(local )?{name} = rest
                let def = |line: &String, name: &str| -> Option<(bool, String)> {
                    let r = line.strip_prefix(pad.as_str())?;
                    let (is_local, r2) = match r.strip_prefix("local ") {
                        Some(x) => (true, x),
                        None => (false, r),
                    };
                    let v = r2.strip_prefix(name)?.strip_prefix(" = ")?;
                    Some((is_local, v.to_string()))
                };
                let d0 = def(&out[i - 3], a);
                let d1 = def(&out[i - 2], b).filter(|x| x.1 == "nil");
                let d2 = def(&out[i - 1], c).filter(|x| x.1 == "nil");
                if let (Some(d0), Some(d1), Some(d2)) = (d0, d1, d2) {
                    if !d0.1.is_empty() {
                        let mut locals = Vec::new();
                        for (n, is_local) in [(a, d0.0), (b, d1.0), (c, d2.0)] {
                            if is_local {
                                locals.push(n.to_string());
                            }
                        }
                        // the state registers mean nothing outside the loop,
                        // but a name declared here may be assigned again later
                        // in its block
                        let mut fend = out.len();
                        for k in i + 1..out.len() {
                            if !is_blank(&out[k]) && indent_of(&out[k]) < ind {
                                fend = k;
                                break;
                            }
                        }
                        let mut later = HashSet::default();
                        for l in &out[i + 1..fend] {
                            words_into(&mut later, l);
                        }
                        let keep: Vec<&str> = locals
                            .iter()
                            .filter(|n| later.contains(n.as_str()))
                            .map(|n| n.as_str())
                            .collect();
                        let mut r = Vec::with_capacity(2);
                        if !keep.is_empty() {
                            r.push(format!("{pad}local {}", keep.join(", ")));
                        }
                        // `local a = f()` kept one value: a call in the header would spread
                        let source = if d0.1.ends_with(')') { format!("({})", d0.1) } else { d0.1.clone() };
                        r.push(format!("{pad}for {vars} in {source} do"));
                        repl = Some(r);
                    }
                }
            }
        }
        if let Some(r) = repl {
            let n = r.len();
            out.splice(i - 3..=i, r);
            i = i - 3 + n;
            continue;
        }
        i += 1;
    }
    out
}

const TOP_OPS: [(&str, i32); 16] = [
    (" or ", 1),
    (" and ", 2),
    (" < ", 3),
    (" > ", 3),
    (" <= ", 3),
    (" >= ", 3),
    (" == ", 3),
    (" ~= ", 3),
    (" .. ", 5),
    (" + ", 6),
    (" - ", 6),
    (" * ", 7),
    (" / ", 7),
    (" // ", 7),
    (" % ", 7),
    (" ^ ", 9),
];

/// Lowest precedence of a binary operator at depth 0 (None if atom).
fn lowest_top_op(s: &str) -> Option<i32> {
    if s.starts_with("if ") {
        return Some(0);
    }
    let b = s.as_bytes();
    let mut top = vec![false; b.len()];
    scan_depth(b, |i, _, d| {
        if d == 0 {
            top[i] = true;
        }
        true
    });
    let mut low = None;
    for (op, prec) in TOP_OPS {
        let mut k = 0;
        while let Some(p) = find_from(b, op.as_bytes(), k) {
            if top[p] {
                low = Some(low.map_or(prec, |x: i32| x.min(prec)));
                break;
            }
            k = p + 1;
        }
    }
    low
}

/// `x = x + e` -> `x += e` (Luau), when e is the whole right operand.
fn compound_assign(lines: Vec<String>) -> Vec<String> {
    const OPS: [(&str, i32); 7] =
        [("+", 6), ("-", 6), ("*", 7), ("/", 7), ("//", 7), ("%", 7), ("..", 5)];
    let mut out = lines;
    for line in out.iter_mut() {
        // ^( *)(ident(\.ident)*) = (.+)$
        let ind = indent_of(line);
        let b = line.as_bytes();
        let Some(e) = ident_chain_end(b, ind) else { continue };
        let Some(rhs) = line[e..].strip_prefix(" = ") else { continue };
        if rhs.is_empty() || line[ind..].starts_with("local ") {
            continue;
        }
        let lhs = &line[ind..e];
        let mut new = None;
        for (op, prec) in OPS {
            let head = format!("{lhs} {op} ");
            if let Some(rest) = rhs.strip_prefix(head.as_str()) {
                let low = lowest_top_op(rest);
                let first = lowest_top_op(rhs);
                if first == Some(prec)
                    && (low.is_none_or(|l| l > prec) || (op == ".." && low.is_some_and(|l| l >= prec)))
                {
                    let mut rest = rest;
                    if rest.starts_with('(') && matching_paren(rest.as_bytes(), 0) == Some(rest.len() - 1) {
                        rest = &rest[1..rest.len() - 1]; // `x += (a + b)`: the operand is all of it
                    }
                    new = Some(format!("{}{lhs} {op}= {rest}", &line[..ind]));
                }
                break;
            }
        }
        if let Some(n) = new {
            *line = n;
        }
    }
    out
}

/// `\("((?:[^"\\]|\\.)*)"\):format\(` searched from `pos`:
/// (match start, string start, string end, match end).
fn interp_search(b: &[u8], pos: usize) -> Option<(usize, usize, usize, usize)> {
    let mut p = pos;
    while let Some(q) = find_from(b, b"(\"", p) {
        let st = q + 2;
        let mut i = st;
        let mut closed = false;
        while i < b.len() {
            match b[i] {
                b'"' => {
                    closed = true;
                    break;
                }
                b'\\' => {
                    if i + 1 >= b.len() || b[i + 1] == b'\n' {
                        break;
                    }
                    i += 2;
                }
                _ => i += 1,
            }
        }
        if closed && b[i..].starts_with(b"\"):format(") {
            return Some((q, st, i, i + 10));
        }
        p = q + 1;
    }
    None
}

/// `("a%*b"):format(x)` is how Luau compiles `` `a{x}b` ``.
fn interpolation(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    for line in out.iter_mut() {
        if !line.contains("):format(") {
            continue;
        }
        let mut pos = 0;
        while let Some((m0, f0, f1, m1)) = interp_search(line.as_bytes(), pos) {
            let fmt = line[f0..f1].to_string();
            let Some(close) = matching_paren(line.as_bytes(), m1 - 1) else { break };
            let args = split_top(&line[m1..close]);
            let holes = fmt.matches("%*").count();
            let rest = fmt.replace("%*", "").replace("%%", "");
            // a call is a statement, an interpolated string is not
            let statement = line[..m0].trim().is_empty();
            if holes == 0
                || statement
                || holes != args.len()
                || rest.contains('%')
                || fmt.contains('`')
                || args.iter().any(|a| a.is_empty() || a.contains('\n'))
            {
                pos = m1;
                continue;
            }
            let mut text = String::from("`");
            for (k, piece) in fmt.split("%*").enumerate() {
                text.push_str(&piece.replace("%%", "%").replace('{', "\\{").replace("\\\"", "\""));
                if k < args.len() {
                    text.push('{');
                    text.push_str(&args[k]);
                    text.push('}');
                }
            }
            text.push('`');
            // grouped to keep one value (`return (s:format(x))`): a string is one
            let (mut from, mut to) = (m0, close + 1);
            let call_paren = |c: Option<char>| c.is_some_and(|c| is_word(c as u8) || c == ')' || c == ']' || c == '"' || c == '\'');
            if line[..m0].ends_with('(')
                && line[close + 1..].starts_with(')')
                && !call_paren(line[..m0 - 1].chars().next_back())
                && !line[close + 2..].starts_with([':', '.', '[', '(', '"', '\'', '{', '`'])
            {
                from = m0 - 1;
                to = close + 2;
            }
            let new = format!("{}{}{}", &line[..from], text, &line[to..]);
            pos = from + text.len();
            *line = new;
        }
    }
    out
}

/// `local f = function(...)` -> `local function f(...)`, and
/// `T.k = function(self, ...)` -> `function T:k(...)` when the first
/// parameter is used as an object.
fn local_functions(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    for i in 0..out.len() {
        let ind = indent_of(&out[i]);
        let new;
        {
            let line = &out[i];
            let pad = &line[..ind];
            let s = &line[ind..];
            if s.contains('\n') {
                continue;
            }
            // ^( *)local (TOKEN) = function\((.*)\)$
            let local = s.strip_prefix("local ").and_then(|r| {
                let b = r.as_bytes();
                if b.is_empty() || !is_start(b[0]) {
                    return None;
                }
                let e = word_end(b, 0);
                let p = r[e..].strip_prefix(" = function(")?.strip_suffix(')')?;
                Some((&r[..e], p))
            });
            if let Some((name, params)) = local {
                new = format!("{pad}local function {name}({params})");
            } else {
                // ^( *)(TOKEN(\.TOKEN)*)\.(TOKEN) = function\((.*)\)$
                let b = s.as_bytes();
                let Some(e) = ident_chain_end(b, 0) else { continue };
                let Some(params) =
                    s[e..].strip_prefix(" = function(").and_then(|r| r.strip_suffix(')'))
                else {
                    continue;
                };
                let Some(dot) = s[..e].rfind('.') else { continue };
                let (obj, key) = (&s[..dot], &s[dot + 1..e]);
                let plist = split_params(params);
                let end = block_end(&out, i);
                let body = &out[i + 1..end.max(i + 1)];
                let first = plist.first().map(|x| x.as_str()).unwrap_or("");
                let used_as_obj = !first.is_empty()
                    && is_generated(first)
                    && body.iter().any(|l| used_as_object(l, first));
                let taken = body.iter().any(|l| has_word(l, "self"));
                if used_as_obj && !taken {
                    let hdr = format!("{pad}function {obj}:{key}({})", plist[1..].join(", "));
                    let first = first.to_string();
                    for l in out[i + 1..end.max(i + 1)].iter_mut() {
                        replace_token(l, &first, "self");
                    }
                    out[i] = hdr;
                    continue;
                }
                new = format!("{pad}function {obj}.{key}({params})");
            }
        }
        out[i] = new;
    }
    out
}

/// `^( *)local (TOKEN(?:, TOKEN)*)$`: indent and names.
fn parse_hoist(line: &str) -> Option<(usize, Vec<String>)> {
    let ind = indent_of(line);
    let (names, value) = parse_local_list(&line[ind..])?;
    if value.is_some() {
        return None;
    }
    Some((ind, names.split(", ").map(str::to_string).collect()))
}

/// `^ *(for|while) .*\bNAME\b`
fn loop_header_mentions(l: &str, name: &str) -> bool {
    let s = l.trim_start_matches(' ');
    let r = if let Some(r) = s.strip_prefix("for ") {
        r
    } else if let Some(r) = s.strip_prefix("while ") {
        r
    } else {
        return false;
    };
    let r = r.split('\n').next().unwrap();
    // the boundary before NAME may be the space just consumed
    find_word_bounded(r, name).is_some()
}

/// Hoisted `local a, b` lines: a name whose every use sits in the block where
/// it is first assigned is declared there instead.
fn sink_declarations(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut dropped = vec![false; out.len()];
    let mut funcs = functions(&out);
    funcs.sort_by_key(|f| -f.0);
    for (fi, fend, _) in funcs {
        let start = (fi + 1) as usize;
        if start >= out.len() {
            continue;
        }
        let Some((ind0, names)) = parse_hoist(&out[start]) else { continue };
        let mut keep: Vec<String> = Vec::new();
        for name in names {
            let lim = fend.min(out.len());
            let uses: Vec<usize> = (start + 1..lim).filter(|&j| has_word(&out[j], &name)).collect();
            if uses.is_empty() {
                keep.push(name); // not seen in range: keep it rather than guess
                continue;
            }
            let first = uses[0];
            // ^( *)NAME = (.+)$
            let ind = indent_of(&out[first]);
            let value = out[first][ind..]
                .strip_prefix(name.as_str())
                .and_then(|r| r.strip_prefix(" = "))
                .filter(|v| !v.is_empty() && !v.contains('\n'));
            let Some(value) = value else {
                keep.push(name);
                continue;
            };
            if has_word(value, &name) {
                keep.push(name);
                continue;
            }
            // the block holding the first assignment: from its opener up to
            // the line that drops below its indent
            let mut stop = fend;
            for j in first + 1..fend {
                if !is_blank(&out[j]) && indent_of(&out[j]) < ind {
                    stop = j;
                    break;
                }
            }
            if uses.iter().all(|&u| first <= u && u < stop)
                && !uses[1..].iter().any(|&j| loop_header_mentions(&out[j], &name))
            {
                // a loop around the first assignment could carry the value to
                // its next pass: only allowed when the assignment is the
                // first thing that touches the name inside that loop
                let new = format!("{}local {name} = {value}", &out[first][..ind]);
                out[first] = new;
            } else {
                keep.push(name);
            }
        }
        if keep.is_empty() {
            out[start] = String::new();
            dropped[start] = true;
        } else {
            out[start] = format!("{}local {}", spaces(ind0), keep.join(", "));
        }
    }
    out.into_iter().zip(dropped).filter(|(_, d)| !*d).map(|(l, _)| l).collect()
}

fn singular(name: &str) -> Option<String> {
    if let Some(h) = name.strip_suffix("ies") {
        return Some(format!("{h}y")); // always differs from name
    }
    if name.ends_with("ss") || name.ends_with("us") || name.ends_with("is") {
        return None;
    }
    for t in ["xes", "ches", "shes", "sses"] {
        if name.ends_with(t) {
            return Some(name[..name.len() - 2].to_string());
        }
    }
    name.strip_suffix('s').map(str::to_string)
}

/// `...("GROUP")` at the very end: text before `(`, and the group.
fn ends_quoted_call(s: &str) -> Option<(&str, &str)> {
    let t = s.strip_suffix("\")")?;
    let q = t.rfind('"')?;
    let before = t[..q].strip_suffix('(')?;
    Some((before, &t[q + 1..]))
}

fn all_alpha(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphabetic())
}

fn call_name(rhs: &str) -> Option<String> {
    let quoted = ends_quoted_call(rhs);
    if let Some((before, g)) = quoted {
        if before.ends_with(":IsA") && all_alpha(g) {
            return Some(format!("is{g}"));
        }
    }
    let r = |s: &str| Some(s.to_string());
    if rhs.ends_with(":Clone()") {
        return r("clone");
    }
    if rhs.starts_with("table.find(") {
        return r("index");
    }
    if rhs.starts_with("string.split(") || rhs.contains(":split(") {
        return r("parts");
    }
    if rhs.starts_with("table.concat(") {
        return r("joined");
    }
    if rhs.starts_with("table.clone(") {
        return r("copy");
    }
    if let Some(t) = rhs.strip_prefix('#') {
        let b = t.as_bytes();
        if !b.is_empty() && is_start(b[0]) && b.iter().all(|&c| is_word(c) || c == b'.') {
            return r("count");
        }
    }
    if matches!(
        rhs,
        "os.clock()" | "tick()" | "time()" | "workspace:GetServerTimeNow()" | "os.time()"
    ) {
        return r("now");
    }
    if rhs.contains(":Raycast(") {
        return r("raycastResult");
    }
    if rhs.contains("TweenService:Create(")
        || rhs.find(":Create(").is_some_and(|p| rhs[p + 8..].contains("TweenInfo"))
    {
        return r("tween");
    }
    if rhs.contains(":LoadAnimation(") {
        return r("animationTrack");
    }
    if rhs.starts_with("TweenInfo.new(") {
        return r("tweenInfo");
    }
    if rhs.starts_with("RaycastParams.new(") {
        return r("raycastParams");
    }
    if rhs.starts_with("OverlapParams.new(") {
        return r("overlapParams");
    }
    if rhs.starts_with("tostring(") {
        return r("str");
    }
    if rhs.starts_with("tonumber(") {
        return r("num");
    }
    if rhs.starts_with("string.format(") || rhs.starts_with('`') {
        return r("text");
    }
    if rhs.starts_with("math.random(") {
        return r("roll");
    }
    for (tail, name) in [
        (":GetMouse()", "mouse"),
        (":GetPivot()", "pivot"),
        (":GetChildren()", "children"),
        (":GetDescendants()", "descendants"),
        (":GetPlayers()", "players"),
    ] {
        if rhs.ends_with(tail) {
            return r(name);
        }
    }
    if rhs.contains(":GetPlayerFromCharacter(") {
        return r("player");
    }
    if rhs.contains(":InvokeServer(") {
        return r("response");
    }
    if rhs.contains(":GetAsync(") {
        return r("data");
    }
    if rhs.starts_with("coroutine.create(") {
        return r("thread");
    }
    if rhs.starts_with("task.spawn(") || rhs.starts_with("task.delay(") || rhs.starts_with("task.defer(") {
        return r("thread");
    }
    if let Some(t) = rhs.strip_prefix("Instance.new(\"") {
        let b = t.as_bytes();
        let mut e = 0;
        while e < b.len() && b[e].is_ascii_alphabetic() {
            e += 1;
        }
        if e > 0 && b.get(e) == Some(&b'"') {
            return Some(lower_first(&t[..e]));
        }
    }
    // ^.*:WaitForChild\("(IDENT)": the last occurrence that fits
    let tag = ":WaitForChild(\"";
    for (p, _) in rhs.rmatch_indices(tag) {
        let t = &rhs[p + tag.len()..];
        let b = t.as_bytes();
        if !b.is_empty() && is_start(b[0]) {
            let e = word_end(b, 0);
            if b.get(e) == Some(&b'"') {
                return Some(camel(&t[..e]));
            }
        }
    }
    None
}

/// `x = Instance.new(...)` and then `x.Name = "Coins"`: x is `coins`.
fn named_instance(rhs: &str, name: &str, at: &[usize], out: &[String]) -> Option<String> {
    if !rhs.starts_with("Instance.new(") {
        return None;
    }
    let head = format!("{name}.Name = \"");
    let v = at.iter().find_map(|&j| strip(&out[j]).strip_prefix(head.as_str()).and_then(|r| r.strip_suffix('"')))?;
    let b = v.as_bytes();
    (!b.is_empty() && is_start(b[0]) && b.iter().all(|&c| is_word(c))).then(|| camel(v))
}

/// A local's spelling of an instance's name: `Humanoid` is `humanoid`, `UI` `ui`.
fn camel(v: &str) -> String {
    if str_isupper(v) {
        v.to_ascii_lowercase()
    } else {
        lower_first(v)
    }
}

/// A natural variable name for a value, from how it was computed.
fn derive_name(rhs: &str) -> Option<String> {
    let rhs = strip(rhs);
    let b = rhs.as_bytes();
    // ^ident(\.ident)*\.(ident)$
    if ident_chain_end(b, 0) == Some(b.len()) {
        if let Some(dot) = rhs.rfind('.') {
            return Some(rhs[dot + 1..].to_string());
        }
    }
    let quoted = ends_quoted_call(rhs);
    if let Some((before, g)) = quoted {
        if before.ends_with(":GetService") && all_alpha(g) {
            return Some(g.to_string());
        }
    }
    if let Some(r) = rhs.strip_prefix("require(") {
        // ^require\(.*[.:](?:WaitForChild|FindFirstChild)\("(IDENT)"\)\)$
        if let Some(t) = r.strip_suffix(')') {
            if let Some((before, g)) = ends_quoted_call(t) {
                let named = before
                    .strip_suffix("WaitForChild")
                    .or_else(|| before.strip_suffix("FindFirstChild"));
                if let Some(h) = named {
                    if (h.ends_with('.') || h.ends_with(':')) && is_token(g) {
                        return Some(g.to_string());
                    }
                }
            }
            // ^require\(.*\.(IDENT)\)$
            if let Some(dot) = t.rfind('.') {
                if is_token(&t[dot + 1..]) {
                    return Some(t[dot + 1..].to_string());
                }
            }
        }
    }
    // ^.*[.:](?:WaitForChild|...)\("(IDENT)"(, [^()]*)?\)$
    if let Some(s) = rhs.strip_suffix(')') {
        if let Some(open) = s.rfind('(') {
            let t = &s[open + 1..];
            if let Some(q) = t.strip_prefix('"') {
                if !q.is_empty() && is_start(q.as_bytes()[0]) {
                    let e = word_end(q.as_bytes(), 0);
                    if let Some(rest) = q[e..].strip_prefix('"') {
                        let tail_ok = rest.is_empty()
                            || (rest.starts_with(", ") && !rest.contains(')'));
                        if tail_ok {
                            let head = &rhs[..open];
                            for m in [
                                "WaitForChild",
                                "FindFirstChild",
                                "FindFirstChildOfClass",
                                "FindFirstChildWhichIsA",
                                "FindFirstAncestor",
                                "FindFirstAncestorOfClass",
                                "FindFirstAncestorWhichIsA",
                            ] {
                                if let Some(h) = head.strip_suffix(m) {
                                    if h.ends_with('.') || h.ends_with(':') {
                                        return Some(camel(&q[..e]));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some((before, g)) = quoted {
        if before.ends_with(":GetAttribute") && is_token(g) {
            return Some(lower_first(g));
        }
    }
    // ^Instance\.new\("([A-Za-z]+)"(, .*)?\)$
    if let Some(t) = rhs.strip_prefix("Instance.new(\"") {
        let c = t.as_bytes();
        let mut e = 0;
        while e < c.len() && c[e].is_ascii_alphabetic() {
            e += 1;
        }
        if e > 0 {
            if let Some(rest) = t[e..].strip_prefix('"') {
                if rest == ")" || (rest.starts_with(", ") && rest.ends_with(')')) {
                    return Some(lower_first(&t[..e]));
                }
            }
        }
    }
    // ^([A-Z][A-Za-z0-9_]*)\.new\(.*\)$
    if !b.is_empty() && b[0].is_ascii_uppercase() {
        let e = word_end(b, 0);
        if let Some(rest) = rhs[e..].strip_prefix(".new(") {
            if rest.ends_with(')') && !is_skip_type(&rhs[..e]) {
                return Some(lower_first(&rhs[..e]));
            }
        }
        // `FrameClock()`: a function spelled as a class makes one
        if rhs[e..].starts_with('(')
            && matching_paren(b, e) == Some(b.len() - 1)
            && !is_skip_type(&rhs[..e])
            && !str_isupper(&rhs[..e])
        {
            return Some(lower_first(&rhs[..e]));
        }
    }
    // ^.*[.:]Get([A-Z][A-Za-z0-9_]*)\([^()]*\)$
    if let Some(s) = rhs.strip_suffix(')') {
        if let Some(open) = s.rfind('(') {
            if !s[open + 1..].contains(')') {
                let h = rhs[..open].as_bytes();
                let mut k = h.len();
                while k > 0 && is_word(h[k - 1]) {
                    k -= 1;
                }
                if k > 0 && (h[k - 1] == b'.' || h[k - 1] == b':') {
                    if let Some(g) = rhs[k..open].strip_prefix("Get") {
                        if g.as_bytes().first().is_some_and(|c| c.is_ascii_uppercase())
                            && !matches!(g, "Service" | "Attribute" | "Output")
                        {
                            return Some(lower_first(g));
                        }
                    }
                }
            }
        }
    }
    // ^TOKEN\((?:ident\.)+([A-Z][A-Z0-9_]+)\)$
    if !b.is_empty() && is_start(b[0]) {
        let e = word_end(b, 0);
        if let Some(t) = rhs[e..].strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
            if let Some(dot) = t.rfind('.') {
                let (head, tail) = (&t[..dot], &t[dot + 1..]);
                let tb = tail.as_bytes();
                if tb.len() >= 2
                    && tb[0].is_ascii_uppercase()
                    && tb[1..].iter().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == b'_')
                    && ident_chain_end(head.as_bytes(), 0) == Some(head.len())
                {
                    return Some(tail.to_string());
                }
            }
        }
    }
    if let Some(n) = call_name(rhs) {
        return Some(n);
    }
    // ^.*:GetPropertyChangedSignal\("([A-Za-z0-9_]+)"\):(?:Connect|Once)\(
    let tag = ":GetPropertyChangedSignal(\"";
    for (p, _) in rhs.rmatch_indices(tag) {
        let t = &rhs[p + tag.len()..];
        let e = word_end(t.as_bytes(), 0);
        let rest = &t[e..];
        if e > 0 && (rest.starts_with("\"):Connect(") || rest.starts_with("\"):Once(")) {
            return Some(format!("{}ChangedConnection", lower_first(&t[..e])));
        }
    }
    // ^.*[.:]([A-Z][A-Za-z0-9_]*):(?:Connect|Once|ConnectParallel)\(
    for p in (0..b.len()).rev() {
        if (b[p] == b'.' || b[p] == b':')
            && b.get(p + 1).is_some_and(|c| c.is_ascii_uppercase())
        {
            let e = word_end(b, p + 1);
            let rest = &rhs[e..];
            if rest.starts_with(":Connect(")
                || rest.starts_with(":Once(")
                || rest.starts_with(":ConnectParallel(")
            {
                return Some(format!("{}Connection", lower_first(&rhs[p + 1..e])));
            }
        }
    }
    None
}

fn is_skip_type(s: &str) -> bool {
    matches!(
        s,
        "Vector3" | "Vector2" | "CFrame" | "Color3" | "UDim2" | "UDim" | "Instance" | "TweenInfo"
            | "NumberSequence" | "ColorSequence" | "Ray" | "Region3" | "NumberRange" | "Rect"
            | "BrickColor" | "vector"
    )
}

/// `^ *for (.+) in (.+) do$`: the variables and the source, group 1 greedy.
fn parse_for_in(line: &str) -> Option<(&str, &str)> {
    let s = line.trim_start_matches(' ').strip_prefix("for ")?.strip_suffix(" do")?;
    if s.contains('\n') {
        return None;
    }
    let mut end = s.len();
    while let Some(p) = s[..end].rfind(" in ") {
        if p >= 1 && p + 4 < s.len() {
            return Some((&s[..p], &s[p + 4..]));
        }
        // an overlap is not possible for " in ", so stepping back one is safe
        end = p + 3;
    }
    None
}

/// `^ *for (TOKEN) = .+ do$`: the variable.
fn parse_for_num(line: &str) -> Option<&str> {
    let s = line.trim_start_matches(' ').strip_prefix("for ")?;
    let b = s.as_bytes();
    if b.is_empty() || !is_start(b[0]) {
        return None;
    }
    let e = word_end(b, 0);
    let rest = s[e..].strip_prefix(" = ")?.strip_suffix(" do")?;
    if rest.is_empty() || rest.contains('\n') {
        return None;
    }
    Some(&s[..e])
}

/// `^ *for (.*\bNAME\b.*) in (.+) do$`
fn for_mentions(line: &str, name: &str) -> bool {
    let Some(s) = line
        .trim_start_matches(' ')
        .strip_prefix("for ")
        .and_then(|r| r.strip_suffix(" do"))
    else {
        return false;
    };
    if s.contains('\n') {
        return false;
    }
    let mut end = s.len();
    while let Some(p) = s[..end].rfind(" in ") {
        if p + 4 < s.len() {
            return find_word_bounded(&s[..p], name).is_some();
        }
        end = p + 3;
    }
    false
}

/// `([A-Za-z_][A-Za-z0-9_]*)(?:\(\))?\)?$` searched in the loop source.
fn for_base(src: &str) -> Option<String> {
    let t = src.replace("ipairs(", "").replace("pairs(", "");
    let body = if let Some(x) = t.strip_suffix("())") {
        x
    } else if let Some(x) = t.strip_suffix("()") {
        x
    } else if let Some(x) = t.strip_suffix(')') {
        x
    } else {
        t.as_str()
    };
    let b = body.as_bytes();
    let mut k = b.len();
    while k > 0 && is_word(b[k - 1]) {
        k -= 1;
    }
    while k < b.len() && b[k].is_ascii_digit() {
        k += 1;
    }
    if k < b.len() {
        Some(body[k..].to_string())
    } else {
        None
    }
}

enum DefKind {
    Single,
    Other,
}

/// How a stripped line defines NAME, if it does:
///   `^(local )?NAME = `                                 Single
///   `^(local )?([A-Za-z0-9_]+, )*NAME(, [A-Za-z0-9_]+)* = ` or
///   `^NAME [-+*/%.]+= `                                 Other
fn def_kind(s: &str, name: &str) -> Option<DefKind> {
    if !s.contains(name) {
        return None;
    }
    let r = s.strip_prefix("local ").unwrap_or(s);
    if r.strip_prefix(name).is_some_and(|t| t.starts_with(" = ")) {
        return Some(DefKind::Single);
    }
    // without "local " the same pattern may apply to the whole line
    if s.strip_prefix(name).is_some_and(|t| t.starts_with(" = ")) {
        return Some(DefKind::Single);
    }
    for cand in [r, s] {
        let b = cand.as_bytes();
        let mut i = 0;
        let mut seen = false;
        let ok = loop {
            let e = word_end(b, i);
            if e == i {
                break false;
            }
            if &cand[i..e] == name {
                seen = true;
            }
            if cand[e..].starts_with(", ") {
                i = e + 2;
                continue;
            }
            break cand[e..].starts_with(" = ");
        };
        if ok && seen {
            return Some(DefKind::Other);
        }
    }
    if let Some(t) = s.strip_prefix(name).and_then(|t| t.strip_prefix(' ')) {
        let b = t.as_bytes();
        let mut i = 0;
        while i < b.len() && matches!(b[i], b'-' | b'+' | b'*' | b'/' | b'%' | b'.') {
            i += 1;
        }
        if i > 0 && t[i..].starts_with("= ") {
            return Some(DefKind::Other);
        }
    }
    None
}

/// Per line: the next one with text that is indented less, where the block it
/// sits in has ended; lines.len() if none is.
fn less_indented(lines: &[String]) -> Vec<u32> {
    let mut found = vec![lines.len() as u32; lines.len()];
    // later lines nothing after them hides, least indented first
    let mut open: Vec<(usize, u32)> = Vec::new();
    for (i, l) in lines.iter().enumerate().rev() {
        if is_blank(l) {
            continue;
        }
        let ind = indent_of(l);
        while open.last().is_some_and(|o| o.0 >= ind) {
            open.pop();
        }
        if let Some(o) = open.last() {
            found[i] = o.1;
        }
        open.push((ind, i as u32));
    }
    found
}

/// Generated names (v5_1, arg2_1) get natural ones where the value says what
/// it is: one definition, and the name is free in the scope.
fn rename_variables(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut funcs = functions(&out);
    funcs.sort_by_key(|f| f.0);
    // renames leave indents alone, and these only ever narrow things down
    let mut index: Option<(WordIndex, Vec<u32>)> = None;
    for (fi, fend, _) in &funcs {
        let fi = *fi;
        let lo = (fi + 1) as usize;
        let hi = (*fend).min(out.len());
        if lo >= hi {
            continue;
        }
        // the generated names that belong to this function: declared in it
        let mut own: HashSet<String> = HashSet::default();
        for l in &out[lo..hi] {
            let s = l.trim_start_matches(' ');
            if let Some((names, _)) = parse_local_list(s) {
                for n in names.split(", ") {
                    own.insert(n.to_string());
                }
            }
            if let Some((vars, _)) = parse_for_in(l) {
                for x in vars.split(',') {
                    own.insert(strip(x).to_string());
                }
            } else if let Some(v) = parse_for_num(l) {
                own.insert(v.to_string());
            }
        }
        // generated names carry their function's depth: `v3` at file level,
        // `v3_1`/`arg1_1` one function deep, and so on
        let depth = if fi < 0 { 0 } else { indent_of(&out[fi as usize]) / 4 + 1 };
        let mut own: Vec<String> = own
            .into_iter()
            .filter(|n| {
                is_generated(n) && if depth == 0 { is_plain_v(n) } else { is_depth_name(n, depth) }
            })
            .collect();
        if own.is_empty() {
            continue;
        }
        own.sort();
        let (index, block_ends) = index.get_or_insert_with(|| (WordIndex::new(&out), less_indented(&out)));
        // names in use anywhere in this function (and so unavailable)
        let mut used: HashSet<u64> = HashSet::default();
        for l in &out[(fi.max(0) as usize)..hi] {
            word_keys_into(&mut used, l);
        }
        for name in &own {
            // a generated name is never the tail of a number, as `x10` can be
            // of `0x10`: it is a word of its own wherever it stands
            let at: Vec<usize> = index.within(name, lo, hi).iter().map(|j| *j as usize).collect();
            let mut defs: Vec<Option<usize>> = Vec::new();
            for &j in &at {
                match def_kind(strip(&out[j]), name) {
                    Some(DefKind::Single) => defs.push(Some(j)),
                    Some(DefKind::Other) => defs.push(None),
                    None => {}
                }
            }
            let mut new: Option<String> = None;
            let fors: Vec<usize> = at.iter().copied().filter(|&j| for_mentions(&out[j], name)).collect();
            if !fors.is_empty() && defs.is_empty() {
                let (vars, src) = parse_for_in(&out[fors[0]]).unwrap();
                let vars_: Vec<&str> = vars.split(',').map(strip).collect();
                let end = block_end(&out, fors[0]);
                let body_uses = at.iter().any(|&j| fors[0] < j && j < end && has_word(&out[j], name));
                let idx = vars_.iter().position(|x| *x == name.as_str());
                let mut base = for_base(src);
                if let Some(b) = &base {
                    if b.starts_with("Get") && b.len() > 3 {
                        base = Some(lower_first(&b[3..]));
                    }
                }
                // `_` would hide a global `_` the body reads (one declared
                // there is its own)
                let declares = |l: &str| {
                    let s = l.trim_start();
                    let names = s
                        .strip_prefix("for ")
                        .and_then(|r| r.split(" in ").next())
                        .or_else(|| s.strip_prefix("local ").and_then(|r| r.split(" = ").next()));
                    names.is_some_and(|n| n.split(',').any(|v| v.trim() == "_"))
                };
                let underscore = (fors[0] + 1..end).any(|j| has_word(&out[j], "_") && !declares(&out[j]));
                if idx == Some(0) && vars_.len() > 1 {
                    new = Some(
                        if !body_uses && !underscore {
                            "_"
                        } else if src.contains("ipairs(") {
                            "i"
                        } else {
                            "key"
                        }
                        .to_string(),
                    );
                } else if idx == Some(vars_.len() - 1) && base.as_deref().is_some_and(|b| !b.is_empty()) {
                    new = singular(base.as_deref().unwrap());
                    if let Some(n) = &new {
                        if !n.is_empty() {
                            new = Some(lower_first(n));
                        }
                    }
                }
            } else if defs.len() == 1 && defs[0].is_some() {
                let st = strip(&out[defs[0].unwrap()]);
                let rhs = st.split_once(" = ").unwrap().1;
                new = named_instance(rhs, name, &at, &out).or_else(|| derive_name(rhs));
            }
            let Some(new) = new else { continue };
            if new.is_empty() || pass_keyword(&new) || !is_token(&new) {
                continue;
            }
            // the variable's scope: its loop, or the block its `local` is in;
            // the new name only has to be free there
            let (mut s_lo, mut s_hi) = (lo, hi);
            if !fors.is_empty() && defs.is_empty() {
                s_lo = fors[0];
                s_hi = block_end(&out, fors[0]) + 1;
            } else if let Some(Some(d)) = defs.first() {
                let d = *d;
                if out[d].trim_start_matches(is_space).starts_with("local ") {
                    s_lo = d;
                    s_hi = hi.min(block_ends[d] as usize);
                }
            }
            if at.iter().any(|&j| (j < s_lo || s_hi <= j) && has_word(&out[j], name)) {
                s_lo = lo; // used outside that block after all
                s_hi = hi;
            }
            if new != "_" {
                let mut taken = false;
                for j in s_lo..s_hi {
                    if has_word(&out[j], &new) {
                        taken = true;
                        break;
                    }
                }
                if taken || ((s_lo, s_hi) == (lo, hi) && used.contains(&key(new.as_bytes()))) {
                    continue;
                }
            }
            let hits: Vec<u32> = at.iter().filter(|j| (s_lo..s_hi).contains(*j)).map(|j| *j as u32).collect();
            for j in &hits {
                replace_token(&mut out[*j as usize], name, &new);
            }
            index.add(&new, &hits);
            used.insert(key(new.as_bytes()));
        }
    }
    out
}

fn signal_params(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "PlayerAdded" | "PlayerRemoving" | "Triggered" | "TriggerEnded" | "OnServerEvent" => &["player"],
        "PromptShown" | "PromptHidden" => &["prompt"],
        "Equipped" => &["mouse"],
        "KeyframeReached" => &["keyframeName"],
        "MarkerReached" => &["parameter"],
        "MouseEnter" | "MouseLeave" | "MouseMoved" => &["x", "y"],
        "CharacterAdded" | "CharacterRemoving" | "CharacterAppearanceLoaded" => &["character"],
        "ChildAdded" | "ChildRemoved" => &["child"],
        "DescendantAdded" | "DescendantRemoving" => &["descendant"],
        "AncestryChanged" => &["child", "parent"],
        "Touched" | "TouchEnded" => &["hit"],
        "InputBegan" | "InputChanged" | "InputEnded" => &["input", "gameProcessed"],
        "TouchStarted" | "TouchMoved" | "TouchEnded_" => &["touch", "gameProcessed"],
        "Heartbeat" | "RenderStepped" | "PreRender" | "PreSimulation" | "PostSimulation"
        | "PreAnimation" => &["deltaTime"],
        "Stepped" => &["time", "deltaTime"],
        "HealthChanged" => &["health"],
        "StateChanged" => &["oldState", "newState"],
        "Running" => &["speed"],
        "MoveToFinished" => &["reached"],
        "Chatted" => &["message"],
        "AttributeChanged" => &["attribute"],
        "FocusLost" => &["enterPressed", "input"],
        "Activated" => &["input", "clickCount"],
        "PromptTriggered" => &["prompt", "player"],
        "MouseButton1Click" => &[],
        "LastInputTypeChanged" => &["inputType"],
        "CameraTypeChanged" => &["cameraType"],
        _ => return None,
    })
}

/// `[.:]([A-Za-z0-9_]+):(?:Connect|Once|ConnectParallel)\(function\(([^()]*)\)$`
/// searched in a right-stripped line: signal and parameter text.
fn signal_handler(line: &str) -> Option<(&str, &str)> {
    let s = rstrip(line);
    let body = s.strip_suffix(')')?;
    let open = body.rfind('(')?;
    let params = &body[open + 1..];
    if params.contains(')') {
        return None;
    }
    let h = s[..open].strip_suffix("(function")?;
    let t = h
        .strip_suffix(":ConnectParallel")
        .or_else(|| h.strip_suffix(":Connect"))
        .or_else(|| h.strip_suffix(":Once"))?;
    let b = t.as_bytes();
    let mut k = b.len();
    while k > 0 && is_word(b[k - 1]) {
        k -= 1;
    }
    if k == b.len() || k == 0 || (b[k - 1] != b'.' && b[k - 1] != b':') {
        return None;
    }
    Some((&t[k..], params))
}

/// Handlers of well-known Roblox events get the parameter names the
/// documentation gives them.
fn callback_params(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    for i in 0..out.len() {
        let Some((sig, params)) = signal_handler(&out[i]) else { continue };
        let Some(names) = signal_params(sig) else { continue };
        let params = split_params(params);
        // up to its end line: what follows the `end` there is the enclosing
        // code's (`end), x.Other:Connect(function(a)` opens the next handler)
        let end = block_end(&out, i).max(i + 1);
        let mut taken = HashSet::default();
        for l in &out[i..end] {
            words_into(&mut taken, l);
        }
        for (old, new) in params.iter().zip(names.iter()) {
            if !is_generated(old) || taken.contains(*new) {
                continue;
            }
            for l in out[i..end].iter_mut() {
                replace_token(l, old, new);
            }
            taken.insert((*new).to_string());
        }
    }
    out
}

/// `^( *)local (TOKEN)(?:, (TOKEN))? = x?pcall\(`
fn parse_pcall(line: &str) -> Option<(usize, String, Option<String>)> {
    let ind = indent_of(line);
    let r = line[ind..].strip_prefix("local ")?;
    let b = r.as_bytes();
    if b.is_empty() || !is_start(b[0]) {
        return None;
    }
    let e = word_end(b, 0);
    let (second, rest) = if let Some(t) = r[e..].strip_prefix(", ") {
        let c = t.as_bytes();
        if c.is_empty() || !is_start(c[0]) {
            return None;
        }
        let f = word_end(c, 0);
        (Some(t[..f].to_string()), &t[f..])
    } else {
        (None, &r[e..])
    };
    let v = rest.strip_prefix(" = ")?;
    if v.starts_with("pcall(") || v.starts_with("xpcall(") {
        Some((ind, r[..e].to_string(), second))
    } else {
        None
    }
}

/// `local a, b = pcall(...)` -> `local success, result = pcall(...)`.
fn pcall_results(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    for i in 0..out.len() {
        let Some((ind, first, second)) = parse_pcall(&out[i]) else { continue };
        let mut end = out.len();
        for k in i + 1..out.len() {
            if !is_blank(&out[k]) && indent_of(&out[k]) < ind {
                end = k;
                break;
            }
        }
        let mut taken = HashSet::default();
        for l in &out[i..end] {
            words_into(&mut taken, l);
        }
        for (old, new) in [(Some(first), "success"), (second, "result")] {
            if let Some(old) = old {
                if is_generated(&old) && !taken.contains(new) {
                    for l in out[i..end].iter_mut() {
                        replace_token(l, &old, new);
                    }
                    taken.insert(new.to_string());
                }
            }
        }
    }
    out
}

/// `ReplicatedStorage/Shared/Sim/Car` -> Car; `.../table-util/init` ->
/// TableUtil (an init module is named after its folder).
fn ident_from_path(path: Option<&str>) -> Option<String> {
    let mut parts: Vec<&str> =
        path.unwrap_or("").split(['/', '\\']).filter(|x| !x.is_empty()).collect();
    while let Some(l) = parts.last() {
        if matches!(l.to_lowercase().as_str(), "init" | "init.client" | "init.server" | "main") {
            parts.pop();
        } else {
            break;
        }
    }
    let last = *parts.last()?;
    let mut base = last;
    for ext in [".client", ".server", ".spec", ".lua", ".luau"] {
        if last.len() >= ext.len()
            && last.is_char_boundary(last.len() - ext.len())
            && last[last.len() - ext.len()..].eq_ignore_ascii_case(ext)
        {
            base = &last[..last.len() - ext.len()];
            break;
        }
    }
    let words_: Vec<&str> =
        base.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect();
    if words_.is_empty() {
        return None;
    }
    let mut name = if words_.len() == 1 {
        words_[0].to_string()
    } else {
        let mut s = String::new();
        for w in &words_ {
            s.push_str(&w[..1].to_ascii_uppercase());
            s.push_str(&w[1..]);
        }
        s
    };
    if name.as_bytes()[0].is_ascii_digit() {
        name.insert(0, '_');
    }
    if is_token(&name) && !pass_keyword(&name) {
        Some(name)
    } else {
        None
    }
}

/// The table a ModuleScript returns is named after the script.
pub fn module_table(lines: Vec<String>, path: Option<&str>) -> Vec<String> {
    let Some(name) = ident_from_path(path) else { return lines };
    let last = lines.iter().rev().find(|l| !is_blank(l)).map_or("", |l| l.as_str());
    let Some(var) = last.strip_prefix("return ").filter(|v| is_plain_v(v)) else {
        return lines;
    };
    let var = var.to_string();
    let head = format!("local {var} = ");
    let declared = lines.iter().any(|l| {
        l.strip_prefix(head.as_str()).is_some_and(|r| {
            (r.starts_with('{') && r.ends_with('}') && r.len() >= 2) || r.starts_with("setmetatable(")
        })
    });
    if !declared {
        return lines;
    }
    let mut taken = false;
    for l in &lines {
        if has_word(l, &name) {
            taken = true;
            break;
        }
    }
    if taken {
        return lines;
    }
    let mut out = lines;
    for l in out.iter_mut() {
        replace_token(l, &var, &name);
    }
    out
}

const USAGE_TYPES: &[(&str, &[(&str, i32)])] = &[
    (
        "player",
        &[
            ("Character", 2), ("UserId", 3), ("DisplayName", 2), ("Kick", 3), ("LoadCharacter", 3),
            ("CharacterAdded", 3), ("PlayerGui", 3), ("Backpack", 2), ("GetMouse", 3), ("Team", 1),
            ("AccountAge", 3), ("MembershipType", 3),
        ],
    ),
    ("character", &[("Humanoid", 2), ("HumanoidRootPart", 3), ("PrimaryPart", 1), ("GetPivot", 1)]),
    (
        "humanoid",
        &[
            ("WalkSpeed", 3), ("JumpPower", 3), ("JumpHeight", 3), ("MoveDirection", 3),
            ("HipHeight", 3), ("Health", 2), ("MaxHealth", 3), ("ChangeState", 3), ("GetState", 3),
            ("Died", 3), ("RootPart", 2), ("MoveTo", 2), ("UseJumpPower", 3), ("AutoRotate", 3),
        ],
    ),
    ("input", &[("KeyCode", 3), ("UserInputType", 3), ("UserInputState", 3), ("Delta", 1)]),
    (
        "track",
        &[
            ("AdjustSpeed", 3), ("TimePosition", 2), ("IsPlaying", 3), ("Looped", 2), ("Length", 1),
            ("AdjustWeight", 3), ("KeyframeReached", 3), ("GetMarkerReachedSignal", 3),
        ],
    ),
    ("sound", &[("SoundId", 3), ("PlaybackSpeed", 3), ("Volume", 2), ("TimeLength", 2)]),
    (
        "camera",
        &[
            ("FieldOfView", 3), ("ViewportSize", 3), ("CameraType", 3), ("CameraSubject", 3),
            ("ViewportPointToRay", 3), ("WorldToViewportPoint", 3), ("ScreenPointToRay", 3),
        ],
    ),
    (
        "part",
        &[
            ("Anchored", 3), ("CanCollide", 3), ("Material", 2), ("Touched", 3),
            ("AssemblyLinearVelocity", 3), ("BrickColor", 2), ("Transparency", 1),
        ],
    ),
    (
        "gui",
        &[
            ("AbsoluteSize", 3), ("AbsolutePosition", 3), ("ZIndex", 2),
            ("BackgroundTransparency", 2), ("TextColor3", 2), ("Text", 1), ("Visible", 1),
        ],
    ),
    ("connection", &[("Disconnect", 3), ("Connected", 2)]),
    ("tween", &[("Play", 1), ("Cancel", 2), ("Completed", 2)]),
];

/// Which lines mention which word. A line listed for a word may no longer
/// use it (renames leave entries behind), the word may sit in a string, and
/// words are known by their key, so this only narrows down the lines the exact
/// matchers have to try.
struct WordIndex {
    ids: HashMap<u64, u32>,
    lines: Vec<Vec<u32>>,
}

impl WordIndex {
    fn new(lines: &[String]) -> WordIndex {
        let ids = HashMap::with_capacity_and_hasher(lines.len(), Default::default());
        let mut index = WordIndex { ids, lines: Vec::with_capacity(lines.len()) };
        for (j, l) in lines.iter().enumerate() {
            let b = l.as_bytes();
            let mut i = 0;
            while i < b.len() {
                if !is_word(b[i]) {
                    i += 1;
                    continue;
                }
                let e = word_end(b, i);
                let id = index.intern(&l[i..e]);
                let list = &mut index.lines[id];
                if list.last() != Some(&(j as u32)) {
                    list.push(j as u32);
                }
                i = e;
            }
        }
        index
    }

    fn intern(&mut self, word: &str) -> usize {
        let next = self.lines.len() as u32;
        let id = *self.ids.entry(key(word.as_bytes())).or_insert(next);
        if id == next {
            self.lines.push(Vec::new());
        }
        id as usize
    }

    /// Lines in lo..hi that mention word, ascending.
    fn within(&self, word: &str, lo: usize, hi: usize) -> &[u32] {
        match self.ids.get(&key(word.as_bytes())) {
            Some(&id) => {
                let list = &self.lines[id as usize];
                let a = list.partition_point(|j| (*j as usize) < lo);
                let b = list.partition_point(|j| (*j as usize) < hi);
                &list[a..b.max(a)]
            }
            None => &[],
        }
    }

    /// These lines now mention word too.
    fn add(&mut self, word: &str, more: &[u32]) {
        let id = self.intern(word);
        let list = &mut self.lines[id];
        list.extend_from_slice(more);
        list.sort_unstable();
        list.dedup();
    }
}

/// Make the tables built on first use now, outside any job.
pub fn warm() {
    usage_members();
}

/// USAGE_TYPES the other way round: member -> (type, weight).
fn usage_members() -> &'static HashMap<&'static str, Vec<(usize, i32)>> {
    static TABLE: OnceLock<HashMap<&'static str, Vec<(usize, i32)>>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table: HashMap<&'static str, Vec<(usize, i32)>> = HashMap::default();
        for (k, (_, members)) in USAGE_TYPES.iter().enumerate() {
            for (m, weight) in *members {
                table.entry(m).or_default().push((k, *weight));
            }
        }
        table
    })
}

/// What a value is, from the members it is used with.
fn usage_name(name: &str, out: &[String], at: &[u32]) -> Option<String> {
    let mut seen: HashSet<&str> = HashSet::default();
    let n = name.as_bytes();
    for j in at {
        let l = &out[*j as usize];
        let b = l.as_bytes();
        let mut i = 0;
        // (?<![A-Za-z0-9_.])NAME[.:](IDENT)
        while let Some(p) = find_from(b, n, i) {
            i = p + 1;
            if p > 0 && (is_word(b[p - 1]) || b[p - 1] == b'.') {
                continue;
            }
            let e = p + n.len();
            if e >= b.len() || (b[e] != b'.' && b[e] != b':') {
                continue;
            }
            if e + 1 < b.len() && is_start(b[e + 1]) {
                let f = word_end(b, e + 1);
                seen.insert(&l[e + 1..f]);
            }
            // `x:FindFirstChild("Humanoid")` says x is a character
            if b[e] == b':' {
                let t = &l[e + 1..];
                for m in ["FindFirstChild(\"", "WaitForChild(\"", "FindFirstChildOfClass(\""] {
                    if let Some(r) = t.strip_prefix(m) {
                        if r.starts_with("Humanoid\"") || r.starts_with("HumanoidRootPart\"") {
                            seen.insert("Humanoid");
                        }
                    }
                }
            }
        }
    }
    let members = usage_members();
    let mut scores = [0; USAGE_TYPES.len()];
    for m in &seen {
        for (kind, weight) in members.get(*m).map_or(&[][..], |list| list) {
            scores[*kind] += weight;
        }
    }
    let mut best: Option<(&str, i32)> = None;
    for ((kind, _), s) in USAGE_TYPES.iter().zip(scores) {
        if s != 0 && best.is_none_or(|b| s > b.1) {
            best = Some((kind, s));
        }
    }
    let (kind, s) = best?;
    if s >= 3 {
        Some(kind.to_string())
    } else {
        None
    }
}

/// The first parameter of `setEnabled` is `enabled`, of `addItem` `item`, of
/// `getItemByID` `id`: what the function's name says it takes.
fn param_from_function_name(fname: &str) -> Option<String> {
    let last = fname.rsplit(['.', ':']).next()?;
    if let Some(at) = last.find("By").filter(|a| last[a + 2..].starts_with(|c: char| c.is_ascii_uppercase())) {
        let noun = &last[at + 2..];
        return Some(if str_isupper(noun) { noun.to_ascii_lowercase() } else { lower_first(noun) });
    }
    const VERBS: &[&str] = &[
        "set", "add", "remove", "insert", "update", "destroy", "handle", "process", "apply", "show", "hide",
        "open", "close", "equip", "unequip", "select", "register", "unregister", "track", "untrack", "spawn",
        "kill", "damage", "heal", "give", "take", "load", "save", "play", "stop", "push", "pop", "use",
        "toggle", "enable", "disable", "clear", "reset", "render", "draw", "build", "attach", "detach",
        "bind", "unbind", "connect", "disconnect", "send", "fire", "on", "is", "has", "can", "with",
    ];
    let split = last.find(|c: char| c.is_ascii_uppercase())?;
    let (verb, noun) = (&last[..split], &last[split..]);
    if !VERBS.contains(&verb) || noun.len() < 2 {
        return None;
    }
    let noun = if str_isupper(noun) { noun.to_ascii_lowercase() } else { lower_first(noun) };
    (is_token(&noun) && !pass_keyword(&noun)).then_some(noun)
}

/// A variable passed to a function defined in this file takes the name that
/// function gives the parameter in that place.
fn passed_name(name: &str, out: &[String], at: &[u32], signatures: &HashMap<String, Vec<String>>) -> Option<String> {
    for j in at {
        let l = &out[*j as usize];
        let b = l.as_bytes();
        let mut from = 0;
        while let Some(open) = l[from..].find('(').map(|x| x + from) {
            from = open + 1;
            // the callee: a dotted name right before the parenthesis
            let mut k = open;
            while k > 0 && (is_word(b[k - 1]) || b[k - 1] == b'.' || b[k - 1] == b':') {
                k -= 1;
            }
            let callee = l[k..open].replace(':', ".");
            let method = l[k..open].contains(':');
            let Some(params) = signatures.get(&callee) else { continue };
            // the arguments at the top level of these parentheses
            let (mut depth, mut slot, mut start) = (0i32, 0usize, open + 1);
            let mut quote = 0u8;
            let mut i = open + 1;
            while i < b.len() {
                let c = b[i];
                if quote != 0 {
                    if c == b'\\' {
                        i += 1;
                    } else if c == quote {
                        quote = 0;
                    }
                } else if c == b'"' || c == b'\'' || c == b'`' {
                    quote = c;
                } else if c == b'(' || c == b'{' || c == b'[' {
                    depth += 1;
                } else if (c == b')' || c == b'}' || c == b']') && depth > 0 {
                    depth -= 1;
                } else if (c == b',' || c == b')') && depth == 0 {
                    if l[start..i].trim() == name {
                        // a method call's arguments start after its self
                        let at = if method { slot } else { slot };
                        let p = params.get(at)?;
                        if !is_generated(p) && p != "_" && p != "..." && p != name {
                            return Some(p.clone());
                        }
                    }
                    if c == b')' {
                        break;
                    }
                    slot += 1;
                    start = i + 1;
                }
                i += 1;
            }
        }
    }
    None
}

/// The kind of value a variable holds, as a short name: by the value it is
/// given, else by what is done with it.
fn kind_name(name: &str, out: &[String], at: &[u32], defs: &[usize]) -> Option<String> {
    let arith = |r: &str| [" + ", " - ", " * ", " / ", " // ", " % ", " ^ "].iter().any(|o| r.contains(o));
    let logic = |r: &str| [" and ", " or ", " == ", " ~= ", " < ", " > ", " <= ", " >= ", " .. "].iter().any(|o| r.contains(o));
    let lines = || at.iter().map(|j| strip(&out[*j as usize]));
    if defs.len() == 1 {
        let st = strip(&out[defs[0]]);
        if let Some((_, rhs)) = st.split_once(" = ") {
            let r = rhs.trim();
            // started at 0 and only ever added to: by one a count, else a total
            if r == "0" {
                let add = format!("{name} += ");
                let steps: Vec<&str> = lines().filter_map(|l| l.strip_prefix(add.as_str())).collect();
                let other = format!("{name} ");
                let changed = lines().any(|l| l.starts_with(&other) && !l.starts_with(&add) && l.contains("= "));
                if !steps.is_empty() && !changed {
                    return Some(if steps.iter().all(|s| *s == "1") { "count" } else { "total" }.to_string());
                }
            }
            let kind = if r.starts_with('{') {
                Some("t")
            } else if r == "true" || r == "false" {
                Some("flag")
            } else if r.starts_with('"') || r.starts_with('`') || (r.contains(" .. ") && !r.contains(" and ") && !r.contains(" or ")) {
                Some("s")
            } else if r.starts_with('#') || r.trim_start_matches('-').parse::<f64>().is_ok() || (arith(r) && !logic(r)) {
                Some("n")
            } else if r.starts_with("function") {
                Some("fn")
            } else if r.starts_with("not ") || [" == ", " ~= ", " < ", " > ", " <= ", " >= "].iter().any(|o| r.contains(o)) && !r.contains(" and ") && !r.contains(" or ") {
                Some("ok")
            } else if r.ends_with(']') && !r.contains('(') {
                Some("value")
            } else {
                None
            };
            if let Some(k) = kind {
                return Some(k.to_string());
            }
        }
    }
    // iterated: a list when its keys go unused or count, else a table
    for l in lines() {
        let Some((vars, src)) = l.strip_prefix("for ").and_then(|r| r.strip_suffix(" do")).and_then(|r| r.split_once(" in ")) else {
            continue;
        };
        let key = vars.split(", ").next().unwrap_or("");
        if src == name || src == format!("ipairs({name})") {
            return Some(if key == "_" || key == "i" || src.starts_with("ipairs(") { "list" } else { "t" }.to_string());
        }
        if src == format!("pairs({name})") {
            return Some("t".to_string());
        }
    }
    // by what is done with it
    let (mut table, mut number, mut string) = (0, 0, 0);
    let (mut flag, mut passed) = (0, 0);
    for j in at {
        let l = &out[*j as usize];
        let b = l.as_bytes();
        let mut from = 0;
        while let Some(p) = l[from..].find(name).map(|x| x + from) {
            from = p + name.len();
            if p > 0 && (is_word(b[p - 1]) || b[p - 1] == b'.' || b[p - 1] == b':') {
                continue;
            }
            if from < b.len() && is_word(b[from]) {
                continue;
            }
            let after = &l[from..];
            let before = &l[..p];
            if after.starts_with('[') || before.ends_with('#') || (after.starts_with('.') && after[1..].starts_with(|c: char| c.is_ascii_lowercase())) {
                table += 1;
            }
            if [" + ", " - ", " * ", " / ", " % ", " // ", " ^ "].iter().any(|o| after.starts_with(o) || before.ends_with(o)) {
                number += 1;
            }
            if after.starts_with(" ..") || before.ends_with(".. ") {
                string += 1;
            }
            // compared with a constant: of its kind
            for op in [" == ", " ~= "] {
                let other = after.strip_prefix(op).or_else(|| before.strip_suffix(op).map(|_| ""));
                if let Some(o) = after.strip_prefix(op) {
                    if o.starts_with('"') {
                        string += 1;
                    } else if o.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
                        number += 1;
                    }
                }
                let _ = other;
            }
            if [" < ", " > ", " <= ", " >= "].iter().any(|o| after.starts_with(o) || before.ends_with(o)) {
                number += 1;
            }
            // tested for truth only
            let s = before.trim_end();
            if (s.ends_with("if") || s.ends_with("not") || s.ends_with("and") || s.ends_with("or") || s.ends_with("while"))
                && (after.starts_with(" then") || after.starts_with(" and ") || after.starts_with(" or ") || after.starts_with(" do") || after.is_empty() || after.starts_with(')'))
            {
                flag += 1;
            }
            if (before.ends_with('(') || before.ends_with(", ")) && (after.starts_with(')') || after.starts_with(',')) {
                passed += 1;
            }
        }
    }
    let best = [(table, "t"), (number, "n"), (string, "s")].into_iter().max_by_key(|x| x.0)?;
    if best.0 > 0 {
        return Some(best.1.to_string());
    }
    if flag > 0 && passed == 0 {
        return Some("flag".to_string());
    }
    (passed > 0).then(|| "value".to_string())
}

/// A name from what the variable is passed to or done with: the argument
/// slot of a well-known call, a class it is tested for, being called.
fn slot_name(name: &str, out: &[String], at: &[u32]) -> Option<String> {
    // (text before the name, text after it, the name it gives), strongest first
    const SLOTS: &[(&str, &str, &str)] = &[
        (":FireClient(", "", "player"),
        (":InvokeClient(", "", "player"),
        (":GetPlayerFromCharacter(", ")", "character"),
        (":FindFirstChild(", ")", "childName"),
        (":WaitForChild(", "", "childName"),
        (":GetAttribute(", ")", "attribute"),
        (":SetAttribute(", ",", "attribute"),
        (":GetService(", ")", "serviceName"),
        ("require(", ")", "module"),
        ("task.wait(", ")", "seconds"),
        ("task.delay(", ",", "seconds"),
        ("ipairs(", ")", "list"),
        ("table.insert(", ",", "list"),
        ("table.remove(", "", "list"),
        ("table.sort(", "", "list"),
        ("table.concat(", "", "list"),
        ("table.find(", ",", "list"),
        ("string.format(", ",", "format"),
        ("pairs(", ")", "map"),
    ];
    let mut class: Option<String> = None;
    let mut called = false;
    let mut found: Option<(usize, &str)> = None;
    const ISA: &str = ":IsA(\"";
    for j in at {
        let l = &out[*j as usize];
        let b = l.as_bytes();
        let mut isa_at: Option<usize> = None;
        let mut from = 0;
        // each place the name stands, and what is right before and after it
        while let Some(p) = l[from..].find(name).map(|x| x + from) {
            from = p + 1;
            let rest = &l[p + name.len()..];
            if isa_at.is_none() && rest.starts_with(ISA) {
                isa_at = Some(p);
            }
            let next = rest.as_bytes().first().copied().unwrap_or(b' ');
            if !is_word(next) {
                let head = &l[..p];
                for (k, (before, after, n)) in SLOTS.iter().enumerate() {
                    if found.is_some_and(|f| f.0 <= k) {
                        break;
                    }
                    if head.ends_with(before) && (after.is_empty() || rest.starts_with(after)) {
                        found = Some((k, n));
                        break;
                    }
                }
            }
            // NAME( called, not NAME.f( or a.NAME(
            let before_ok = p == 0 || !(is_word(b[p - 1]) || b[p - 1] == b'.' || b[p - 1] == b':');
            if before_ok && next == b'(' {
                called = true;
            }
        }
        // NAME:IsA("Class"), the first in the line
        if let Some(p) = isa_at {
            let ok = p == 0 || !is_word(b[p - 1]);
            let rest = &l[p + name.len() + ISA.len()..];
            if let Some(c) = rest.split('"').next().filter(|c| ok && is_token(c)) {
                class = Some(lower_first(c));
            }
        }
    }
    if let Some(c) = class {
        return Some(c);
    }
    if let Some((_, n)) = found {
        return Some(n.to_string());
    }
    called.then(|| "callback".to_string())
}

/// `^(?:get|Get|...|calculate|Calculate)([A-Z][A-Za-z0-9_]*)$`: the group.
fn getter_noun(s: &str) -> Option<&str> {
    for p in [
        "get", "Get", "find", "Find", "make", "Make", "create", "Create", "build", "Build",
        "compute", "Compute", "calculate", "Calculate",
    ] {
        if let Some(r) = s.strip_prefix(p) {
            let b = r.as_bytes();
            if b.first().is_some_and(|c| c.is_ascii_uppercase()) && b.iter().all(|&c| is_word(c)) {
                return Some(r);
            }
        }
    }
    None
}

fn function_name_of(header: &str) -> Option<&str> {
    // function (?:[A-Za-z_][A-Za-z0-9_.]*[.:])?([A-Za-z_][A-Za-z0-9_]*)\(
    let b = header.as_bytes();
    let mut i = 0;
    while let Some(p) = find_from(b, b"function ", i) {
        i = p + 1;
        let st = p + 9;
        let mut e = st;
        while e < b.len() && (is_word(b[e]) || b[e] == b'.' || b[e] == b':') {
            e += 1;
        }
        if e == st || e >= b.len() || b[e] != b'(' {
            continue;
        }
        let run = &header[st..e];
        let sep = run.rfind(['.', ':']);
        let (prefix, name) = match sep {
            Some(k) => (&run[..=k], &run[k + 1..]),
            None => ("", run),
        };
        if !is_token(name) {
            continue;
        }
        if !prefix.is_empty() {
            let h = &prefix[..prefix.len() - 1];
            if h.is_empty()
                || !is_start(h.as_bytes()[0])
                || !h.bytes().all(|c| is_word(c) || c == b'.')
            {
                continue;
            }
        }
        return Some(name);
    }
    // ^ *local (IDENT) = function\(
    let s = header.trim_start_matches(' ').strip_prefix("local ")?;
    let c = s.as_bytes();
    if c.is_empty() || !is_start(c[0]) {
        return None;
    }
    let e = word_end(c, 0);
    if s[e..].starts_with(" = function(") {
        Some(&s[..e])
    } else {
        None
    }
}

/// `function [A-Za-z_][A-Za-z0-9_.]*:` / `function [A-Za-z_][A-Za-z0-9_]*\.`
fn header_has(header: &str, dots_in_name: bool, sep: u8) -> bool {
    let b = header.as_bytes();
    let mut i = 0;
    while let Some(p) = find_from(b, b"function ", i) {
        i = p + 1;
        let st = p + 9;
        if st >= b.len() || !is_start(b[st]) {
            continue;
        }
        let mut e = st;
        while e < b.len() && (is_word(b[e]) || (dots_in_name && b[e] == b'.')) {
            e += 1;
        }
        if e < b.len() && b[e] == sep {
            return true;
        }
    }
    false
}

/// `^ *[A-Za-z_][A-Za-z0-9_.]*\.[A-Za-z_][A-Za-z0-9_]* = function\(`
fn is_field_function(header: &str) -> bool {
    let s = header.trim_start_matches(' ');
    let b = s.as_bytes();
    if b.is_empty() || !is_start(b[0]) {
        return false;
    }
    let mut e = 0;
    while e < b.len() && (is_word(b[e]) || b[e] == b'.') {
        e += 1;
    }
    if !s[e..].starts_with(" = function(") {
        return false;
    }
    match s[..e].rfind('.') {
        Some(d) if d > 0 => is_token(&s[d + 1..e]),
        _ => false,
    }
}

fn key_of(kind: &str, name: &str) -> String {
    let mut k = String::with_capacity(kind.len() + name.len());
    k.push_str(kind);
    k.push_str(name);
    k
}

/// (kind, function name) -> argument lists, kind "" f(...), "." t.f(...),
/// ":" t:f(...); only for the keys asked for, since finding a call's closing
/// parenthesis and splitting its arguments is what costs.
pub fn collect_calls(lines: &[String], calls: &mut CallMap, wanted: &HashSet<String>) {
    let mut key = String::new();
    for l in lines {
        let Some(first) = l.find('(') else { continue };
        if func_open(l).is_some() {
            // ^ *(local )?function : a definition, not a call
            let t = l.trim_start_matches(' ');
            if t.starts_with("function ") || t.starts_with("local function ") {
                continue;
            }
        }
        let b = l.as_bytes();
        let mut next = Some(first);
        while let Some(e) = next {
            next = l[e + 1..].find('(').map(|p| e + 1 + p);
            // (?:(?<![A-Za-z0-9_.:])|([.:]))(IDENT)\(
            let mut st = e;
            while st > 0 && is_word(b[st - 1]) {
                st -= 1;
            }
            if st == e || !is_start(b[st]) {
                continue;
            }
            let k = usize::from(st > 0 && (b[st - 1] == b'.' || b[st - 1] == b':'));
            let name = &l[st..e];
            key.clear();
            key.push_str(&l[st - k..st]);
            key.push_str(name);
            if !wanted.contains(&key) || pass_keyword(name) {
                continue;
            }
            if let Some(close) = matching_paren(b, e) {
                if close > 0 {
                    let args = split_top(&l[e + 1..close]);
                    match calls.get_mut(&key) {
                        Some(v) => v.push(args),
                        None => {
                            calls.insert(key.clone(), vec![args]);
                        }
                    }
                }
            }
        }
    }
}

/// `^ *[A-Za-z_][A-Za-z0-9_.]*\.(IDENT) = NAME$`
fn stored_field<'a>(line: &'a str, tail: &str) -> Option<&'a str> {
    let s = line.trim_start_matches(' ').strip_suffix(tail)?;
    let b = s.as_bytes();
    if b.is_empty() || !is_start(b[0]) || !b.iter().all(|&c| is_word(c) || c == b'.') {
        return None;
    }
    let d = s.rfind('.')?;
    if d > 0 && is_token(&s[d + 1..]) {
        Some(&s[d + 1..])
    } else {
        None
    }
}

/// `local a, b = f(...)` with f ending in `return set, key`: a is `set`.
fn returned_name(name: &str, def: &str, returns: &HashMap<String, usize>, out: &[String]) -> Option<String> {
    let (lhs, rhs) = def.strip_prefix("local ").unwrap_or(def).split_once(" = ")?;
    let at = lhs.split(", ").position(|n| n == name)?;
    let open = rhs.find('(')?;
    if !rhs.ends_with(')') || matching_paren(rhs.as_bytes(), open) != Some(rhs.len() - 1) {
        return None;
    }
    let callee = rhs[..open].replace(':', ".");
    let list = strip(&out[*returns.get(&callee)?]).strip_prefix("return ")?;
    let n = list.split(", ").nth(at)?;
    (is_token(n) && !is_generated(n) && !pass_keyword(n)).then(|| n.to_string())
}

/// `xs[k] = name` or `table.insert(xs, name)`: name is one of xs.
fn stored_element(line: &str, name: &str) -> Option<String> {
    let s = strip(line);
    if let Some(lhs) = s.strip_suffix(name).and_then(|x| x.strip_suffix(" = ")) {
        let lhs = lhs.strip_suffix(']')?;
        // the `[` of the last index
        let b = lhs.as_bytes();
        let mut depth = 0;
        let mut k = b.len();
        while k > 0 {
            k -= 1;
            match b[k] {
                b']' => depth += 1,
                b'[' if depth == 0 => return element_noun(&lhs[..k]),
                b'[' => depth -= 1,
                _ => {}
            }
        }
        return None;
    }
    let inner = s.strip_prefix("table.insert(")?.strip_suffix(')')?;
    let args = split_top(inner);
    if args.len() == 2 && args[1].trim() == name {
        return element_noun(args[0].trim());
    }
    None
}

/// `[{,] (IDENT) = NAME[,}]` searched in the line.
fn stored_in_table<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let b = line.as_bytes();
    let n = name.as_bytes();
    let mut i = 0;
    // from the name: a space would stop this at every word
    while let Some(at) = find_from(b, n, i) {
        i = at + 1;
        let e = at + n.len();
        if at < 3 || &b[at - 3..at] != b" = " || e >= b.len() || (b[e] != b',' && b[e] != b'}') {
            continue;
        }
        let p = at - 3;
        let mut k = p;
        while k > 0 && is_word(b[k - 1]) {
            k -= 1;
        }
        if k == p || !is_start(b[k]) {
            // the identifier may begin later in the run of word characters,
            // but then what precedes it is a word character, not a space
            continue;
        }
        if k >= 2 && b[k - 1] == b' ' && (b[k - 2] == b'{' || b[k - 2] == b',') {
            return Some(&line[k..p]);
        }
    }
    None
}

/// How a function with this header is called: "" f(...), "." t.f(...), ":" t:f(...).
fn call_kind(header: &str) -> &'static str {
    if header_has(header, true, b':') {
        ":"
    } else if header_has(header, false, b'.') || is_field_function(header) {
        "."
    } else {
        ""
    }
}

/// What a call-site argument suggests calling the parameter.
fn vote_name(arg: &str) -> Option<String> {
    let a = strip(arg);
    if pass_keyword(a) || matches!(a, "true" | "false" | "nil") {
        return None; // a literal says nothing of the parameter's name
    }
    if is_token(a) && !is_generated(a) {
        Some(a.to_string())
    } else {
        derive_name(a)
    }
}

/// How often each name was passed for one parameter, first seen first.
#[derive(Default, Clone, PartialEq)]
pub struct ParamVotes {
    order: Vec<(String, usize)>,
    at: HashMap<String, usize>,
}

impl ParamVotes {
    fn vote(&mut self, name: &str, count: usize) {
        match self.at.get(name) {
            Some(&at) => self.order[at].1 += count,
            None => {
                self.at.insert(name.to_string(), self.order.len());
                self.order.push((name.to_string(), count));
            }
        }
    }
}

/// The call sites of one `t.f` / `t:f`.
#[derive(Default, Clone, PartialEq)]
pub struct KeyVotes {
    total: usize,
    params: Vec<ParamVotes>,
}

/// One script's call sites as other scripts see them. Finding what an argument
/// suggests is the costly part, and this is the only place it is done.
pub type Tally = HashMap<String, KeyVotes>;

pub fn tally(calls: &CallMap) -> Tally {
    let mut out = Tally::default();
    for (key, sites) in calls {
        let kv = out.entry(key.clone()).or_default();
        kv.total = sites.len();
        for args in sites {
            if kv.params.len() < args.len() {
                kv.params.resize_with(args.len(), ParamVotes::default);
            }
            for (i, a) in args.iter().enumerate() {
                let Some(n) = vote_name(a) else { continue };
                if !n.is_empty() && (n == "self" || !is_generated(&n)) {
                    kv.params[i].vote(&n, 1);
                }
            }
        }
    }
    out
}

/// Call sites in the other scripts of a place. Only what parameter naming
/// asks of them is kept: per defined function and parameter, the names
/// passed. Sites only ever arrive, so tallies grow instead of being redone,
/// and a place with thousands of `:Get(...)` calls stays linear.
#[derive(Default)]
pub struct Extra {
    keys: HashMap<String, KeyVotes>,
    wanted: HashSet<String>,
}

impl Extra {
    /// Functions any script defines; call sites of anything else are skipped.
    pub fn want(&mut self, keys: impl IntoIterator<Item = String>) {
        self.wanted.extend(keys);
    }

    pub fn wanted(&self) -> &HashSet<String> {
        &self.wanted
    }

    /// Would naming a script that defines `key` see anything from here?
    pub fn knows(&self, key: &str) -> bool {
        self.keys.contains_key(key)
    }

    /// One script's call sites of one function, after those of earlier scripts:
    /// a name keeps its place from the first time anyone passed it.
    pub fn add(&mut self, key: &str, votes: &KeyVotes) {
        let Some(kv) = self.keys.get_mut(key) else {
            self.keys.insert(key.to_string(), votes.clone());
            return;
        };
        kv.total += votes.total;
        if kv.params.len() < votes.params.len() {
            kv.params.resize_with(votes.params.len(), ParamVotes::default);
        }
        for (p, q) in kv.params.iter_mut().zip(&votes.params) {
            for (n, count) in &q.order {
                p.vote(n, *count);
            }
        }
    }
}

/// The `t.f` / `t:f` functions these lines define, as call-site keys.
pub fn defined_keys(lines: &[String]) -> Vec<String> {
    let mut keys = Vec::new();
    for (fi, _, _) in functions(lines) {
        if fi < 0 {
            continue;
        }
        let header = &lines[fi as usize];
        let kind = call_kind(header);
        if kind.is_empty() {
            continue;
        }
        if let Some(name) = function_name_of(header) {
            keys.push(key_of(kind, name));
        }
    }
    keys
}

/// Second naming round, for generated names still left:
///   * `local x = setmetatable({}, T)` -> self (outside methods)
///   * stored into a field (`self.speed = x`, `{speed = x}`) -> speed
///   * returned at the end of `GetFoo` -> foo
///   * used like a known Roblox type (`x.Character`, `x.UserId`) -> player
/// Parameters take part too, and are named from their call sites first.
/// Names by convention and by role where the decompiler had none: loop
/// variables (`i`, `k`, `v`, or the singular of what is iterated), and `_`
/// for a parameter nothing reads.
pub fn role_names(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    loop_names(&mut out);
    unused_params(&mut out);
    out
}

fn word_in(out: &[String], lo: usize, hi: usize, w: &str) -> bool {
    (lo..=hi.min(out.len().saturating_sub(1))).any(|j| has_word(&out[j], w))
}

fn rename_in(out: &mut [String], lo: usize, hi: usize, old: &str, new: &str) {
    let lines: Vec<usize> = (lo..=hi.min(out.len().saturating_sub(1))).collect();
    rename_lines(out, &lines, old, new);
}

/// Does `w` stand as a word anywhere in the raw line, strings included?
fn raw_word(l: &str, w: &str) -> bool {
    let b = l.as_bytes();
    let mut from = 0;
    while let Some(p) = l[from..].find(w).map(|x| x + from) {
        from = p + w.len();
        let before = p > 0 && (is_word(b[p - 1]) || b[p - 1] == b'.');
        let after = from < b.len() && is_word(b[from]);
        if !before && !after {
            return true;
        }
    }
    false
}

/// Rename `old` to `new` on these lines, or leave them as they were if some
/// use of it is out of reach (a nested interpolated string). Whether it did.
fn rename_lines(out: &mut [String], lines: &[usize], old: &str, new: &str) -> bool {
    let saved: Vec<String> = lines.iter().map(|j| out[*j].clone()).collect();
    for j in lines {
        replace_token(&mut out[*j], old, new);
    }
    // raw text, strings and interpolations included: a use replace_token
    // could not reach shows there
    if lines.iter().any(|j| raw_word(&out[*j], old)) {
        for (j, l) in lines.iter().zip(saved) {
            out[*j] = l;
        }
        return false;
    }
    true
}

/// The singular of what a loop goes over, if that says: `players` (or
/// `self.players`, `ipairs(players)`) to `player`, `x:GetChildren()` to `child`.
pub(crate) fn element_noun(expr: &str) -> Option<String> {
    let e = expr.trim();
    let inner = e
        .strip_prefix("ipairs(")
        .or_else(|| e.strip_prefix("pairs("))
        .and_then(|x| x.strip_suffix(')'))
        .unwrap_or(e);
    for (call, noun) in [
        (":GetChildren()", "child"),
        (":GetDescendants()", "descendant"),
        (":GetPlayers()", "player"),
        (":GetTouchingParts()", "part"),
        (":GetTags()", "tag"),
        (":GetPlayingAnimationTracks()", "track"),
        (":GetAccessories()", "accessory"),
    ] {
        if inner.ends_with(call) {
            return Some(noun.to_string());
        }
    }
    let b = inner.as_bytes();
    let mut k = b.len();
    while k > 0 && is_word(b[k - 1]) {
        k -= 1;
    }
    if k == b.len() || (k > 0 && b[k - 1] != b'.') {
        return None;
    }
    let word = &inner[k..];
    if is_generated(word) || word.len() < 4 {
        return None;
    }
    let lower = if str_isupper(word) { word.to_ascii_lowercase() } else { lower_first(word) };
    let one = if let Some(s) = lower.strip_suffix("children") {
        format!("{s}child")
    } else if let Some(s) = lower.strip_suffix("ies") {
        format!("{s}y")
    } else if let Some(s) = ["ches", "shes", "sses", "xes"].iter().find_map(|suf| lower.strip_suffix(suf).map(|s| (s, suf))) {
        format!("{}{}", s.0, &s.1[..s.1.len() - 2])
    } else if lower.ends_with("ss") || lower.ends_with("us") || lower.ends_with("is") {
        return None;
    } else {
        lower.strip_suffix('s')?.to_string()
    };
    (one.len() >= 2 && is_token(&one) && !pass_keyword(&one)).then_some(one)
}

/// `for v4_1 = 1, n do` to `for i = 1, n do`; `for k, v in pairs(t) do`; the
/// value of a loop over `players` is `player`. A name the loop already
/// mentions is passed over.
fn loop_names(out: &mut [String]) {
    for i in 0..out.len() {
        let s = strip(&out[i]).to_string();
        let Some(head) = s.strip_prefix("for ").and_then(|r| r.strip_suffix(" do")) else { continue };
        let (vars, expr) = match head.split_once(" in ") {
            Some((v, e)) => (v.to_string(), Some(e.to_string())),
            None => match head.split_once(" = ") {
                Some((v, _)) => (v.to_string(), None),
                None => continue,
            },
        };
        let end = block_end(out, i);
        let vars: Vec<String> = vars.split(',').map(|x| x.trim().to_string()).collect();
        for (k, var) in vars.iter().enumerate() {
            if !is_generated(var) {
                continue;
            }
            let mut cands: Vec<String> = Vec::new();
            match (&expr, k) {
                (None, _) => cands.extend(["i", "j", "k", "n"].map(String::from)),
                (Some(e), 0) if e.starts_with("ipairs(") => cands.extend(["i", "j", "index"].map(String::from)),
                (Some(_), 0) => cands.extend(["k", "key"].map(String::from)),
                (Some(e), 1) => {
                    cands.extend(element_noun(e));
                    cands.extend(["v", "value"].map(String::from));
                }
                _ => continue,
            }
            if let Some(new) = cands.into_iter().find(|c| !word_in(out, i, end, c)) {
                rename_in(out, i, end, var, &new);
            }
        }
    }
}

/// A parameter nothing in its function reads is `_`.
fn unused_params(out: &mut [String]) {
    for i in 0..out.len() {
        let Some(params) = func_open(&out[i]).map(split_params) else { continue };
        if !params.iter().any(|p| is_generated(p)) {
            continue;
        }
        let end = block_end(out, i).saturating_sub(1).max(i); // its `end` line is not its own
        // a `_` the body reads (a global, say) would become this parameter;
        // one a loop or local declares there is its own
        let declares_underscore = |l: &str| {
            let s = l.trim_start();
            let names = s
                .strip_prefix("for ")
                .and_then(|r| r.split(" in ").next().map(|n| n.split(" = ").next().unwrap_or(n)))
                .or_else(|| s.strip_prefix("local ").and_then(|r| r.split(" = ").next()));
            names.is_some_and(|n| n.split(',').any(|v| v.trim() == "_"))
        };
        let reads_underscore = (i + 1..=end).any(|j| has_word(&out[j], "_") && !declares_underscore(&out[j]));
        if reads_underscore {
            continue;
        }
        for p in &params {
            if is_generated(p) && !word_in(out, i + 1, end, p) {
                replace_token(&mut out[i], p, "_");
            }
        }
    }
}

pub fn more_names(lines: Vec<String>, extra: &Extra) -> Vec<String> {
    let mut out = lines;
    let mut index = WordIndex::new(&out);
    // the same pass over the same text would find nothing new
    if name_pass(&mut out, &mut index, extra) {
        name_pass(&mut out, &mut index, extra);
    }
    out
}

/// One look at every function. Says whether anything was renamed.
fn name_pass(out: &mut Vec<String>, index: &mut WordIndex, extra: &Extra) -> bool {
    let mut renamed = false;
    let mut funcs = functions(out);
    funcs.sort_by_key(|f| f.0);
    // the module's own table: `self` passed around in its methods is one
    let mut module: Option<String> = None;
    {
        let last = out.iter().rev().find(|l| !is_blank(l)).map_or("", |l| l.as_str());
        if let Some(m) = last.strip_prefix("return ") {
            if is_token(m) && !is_generated(m) {
                let head = format!("local {m} = ");
                if out.iter().any(|l| l.starts_with(&head)) {
                    module = Some(m.to_string());
                }
            }
        }
    }
    // which call sites this file asks about
    let mut asked: HashSet<String> = HashSet::default();
    for (fi, _, _) in &funcs {
        if *fi >= 0 {
            let header = &out[*fi as usize];
            if let Some(f) = function_name_of(header) {
                asked.insert(key_of(call_kind(header), f));
            }
        }
    }
    let mut calls = CallMap::default();
    collect_calls(out, &mut calls, &asked);
    let no_sites: Vec<Vec<String>> = Vec::new();
    // what each function defined here calls its parameters, as calls name it
    let mut signatures: HashMap<String, Vec<String>> = HashMap::default();
    // and the line where it returns at its end (`return set, key`), read
    // when asked: an earlier function's names may change on the way
    let mut returns: HashMap<String, usize> = HashMap::default();
    for (fi, fend, params) in &funcs {
        if *fi >= 0 {
            if let Some(f) = function_name_of(&out[*fi as usize]) {
                let f = f.replace(':', ".");
                signatures.entry(f.clone()).or_insert_with(|| params.clone());
                let from = (*fi as usize + 1).min(out.len());
                let last = (from..(*fend).min(out.len())).rev().find(|j| !is_blank(&out[*j]));
                if let Some(j) = last.filter(|j| strip(&out[*j]).starts_with("return ")) {
                    returns.entry(f).or_insert(j);
                }
            }
        }
    }
    for (fi, fend, params) in &funcs {
        // the main chunk (-1): its own locals carry no depth
        let top = *fi < 0;
        let fi = (*fi).max(0) as usize;
        let (lo, hi) = if top { (0, out.len()) } else { (fi + 1, (*fend).min(out.len())) };
        let depth = if top { 0 } else { indent_of(&out[fi]) / 4 + 1 };
        let header = if top { String::new() } else { out[fi].clone() };
        let fname = function_name_of(&header).map(str::to_string);
        let scope_end = (hi + 1).min(out.len());
        let in_scope = |out: &[String], index: &WordIndex, word: &str| {
            index.within(word, fi, scope_end).iter().any(|j| has_word(&out[*j as usize], word))
        };
        // names this function's renames have used or given up
        let mut touched: Vec<String> = Vec::new();
        let mut cands: Vec<(String, String)> = Vec::new();
        // parameters, from what callers pass
        let kind = call_kind(&header);
        let (site_a, site_b): (&[Vec<String>], Option<&KeyVotes>) = match &fname {
            Some(f) => {
                let key = key_of(kind, f);
                let a = calls.get(&key).unwrap_or(&no_sites);
                // only t.f / t:f calls reach across scripts
                let b = if kind.is_empty() { None } else { extra.keys.get(&key) };
                (a, b)
            }
            None => (&no_sites, None),
        };
        let nsites = site_a.len() + site_b.map_or(0, |k| k.total);
        if nsites != 0 {
            for (i, pname) in params.iter().enumerate() {
                if !is_generated(pname) {
                    continue;
                }
                let mut votes: Vec<(String, usize)> = Vec::new();
                let mut seen: HashMap<String, usize> = HashMap::default();
                let self_name = match &module {
                    Some(m) if kind != ":" => Some(lower_first(m)),
                    _ => None,
                };
                let mut vote = |n: &str, count: usize| {
                    let n = if n == "self" {
                        match &self_name {
                            Some(s) => s.as_str(),
                            None => return,
                        }
                    } else {
                        n
                    };
                    if n.is_empty() || is_generated(n) {
                        return;
                    }
                    match seen.get(n) {
                        Some(&at) => votes[at].1 += count,
                        None => {
                            seen.insert(n.to_string(), votes.len());
                            votes.push((n.to_string(), count));
                        }
                    }
                };
                for args in site_a {
                    if i < args.len() {
                        if let Some(n) = vote_name(&args[i]) {
                            vote(&n, 1);
                        }
                    }
                }
                if let Some(param) = site_b.and_then(|k| k.params.get(i)) {
                    for (n, count) in &param.order {
                        vote(n, *count);
                    }
                }
                let mut best: Option<&(String, usize)> = None;
                for v in &votes {
                    if best.is_none_or(|b| v.1 > b.1) {
                        best = Some(v);
                    }
                }
                if let Some((b, count)) = best {
                    if count * 2 >= nsites {
                        let v = if first_upper(b) && !str_isupper(b) { lower_first(b) } else { b.clone() };
                        match cands.iter_mut().find(|c| c.0 == *pname) {
                            Some(c) => c.1 = v,
                            None => cands.push((pname.clone(), v)),
                        }
                    }
                }
            }
        }
        let mut own: HashSet<String> = params.iter().cloned().collect();
        for l in &out[lo..hi.max(lo)] {
            if let Some((names, _)) = parse_local_list(l.trim_start_matches(' ')) {
                for n in names.split(", ") {
                    own.insert(n.to_string());
                }
            }
        }
        let mut own: Vec<String> = own
            .into_iter()
            .filter(|n| if top { is_generated(n) && !n.contains('_') } else { is_depth_name(n, depth) })
            .collect();
        own.sort();
        for name in &own {
            if cands.iter().any(|c| c.0 == *name) {
                continue;
            }
            let eq = format!("{name} = ");
            let at = index.within(name, lo, hi.max(lo));
            let defs: Vec<usize> = at
                .iter()
                .map(|j| *j as usize)
                .filter(|&j| {
                    let s = out[j].trim_start_matches(' ');
                    s.starts_with(&eq) || s.strip_prefix("local ").is_some_and(|t| t.starts_with(&eq))
                })
                .collect();
            let mut new: Option<String> = None;
            if defs.len() == 1 {
                let st = strip(&out[defs[0]]);
                let rhs = st.split_once(" = ").unwrap().1;
                if rhs.starts_with("setmetatable(") && !in_scope(&out, &index, "self") {
                    new = Some("self".to_string());
                }
            }
            if new.is_none() {
                let own_index = format!("{name}.__index = {name}");
                if at.iter().any(|j| strip(&out[*j as usize]) == own_index) {
                    new = Some("Class".to_string());
                }
            }
            if new.is_none() {
                let tail = format!(" = {name}");
                for j in at {
                    let l = &out[*j as usize];
                    let m = stored_field(l, &tail)
                        .or_else(|| stored_in_table(l, name))
                        .filter(|f| !f.starts_with("__"));
                    if let (Some(f), true) = (m, defs.len() <= 1) {
                        new = Some(if !first_upper(f) || str_isupper(f) {
                            f.to_string()
                        } else {
                            lower_first(f)
                        });
                        break;
                    }
                }
            }
            if new.is_none() && !returns.is_empty() {
                // the declaration, `local a, b = f()` among them
                let declared = at.iter().map(|j| strip(&out[*j as usize])).find(|l| {
                    l.strip_prefix("local ")
                        .and_then(|r| r.split_once(" = "))
                        .is_some_and(|(lhs, _)| lhs.split(", ").any(|n| n == name))
                });
                if let Some(d) = declared {
                    new = returned_name(name, d, &returns, out);
                }
            }
            if new.is_none() && defs.len() <= 2 {
                if let Some(noun) = fname.as_deref().and_then(getter_noun) {
                    let last =
                        out[lo..hi.max(lo)].iter().rev().find(|l| !is_blank(l)).map_or("", |l| strip(l));
                    if last.strip_prefix("return ") == Some(name.as_str()) {
                        new = Some(lower_first(noun));
                    }
                }
            }
            if new.is_none() {
                new = usage_name(name, &out, at);
            }
            if new.is_none() {
                new = slot_name(name, &out, at);
            }
            if new.is_none() && params.first() == Some(name) {
                new = fname.as_deref().and_then(param_from_function_name);
            }
            if new.is_none() {
                new = passed_name(name, &out, at, &signatures);
            }
            if new.is_none() {
                new = at.iter().find_map(|j| stored_element(&out[*j as usize], name));
            }
            if new.is_none() {
                // last: the kind of value, `n2` if `n` is taken
                if let Some(base) = kind_name(name, &out, at, &defs) {
                    let taken = |c: &str| {
                        in_scope(&out, &index, c) || touched.iter().any(|t| t == c) || cands.iter().any(|x| x.1 == c)
                    };
                    new = std::iter::once(base.clone())
                        .chain((2..=20).map(|k| format!("{base}{k}")))
                        .find(|c| !taken(c));
                }
            }
            if let Some(n) = new {
                // taken here: the same name with a number
                let taken = |c: &str| {
                    in_scope(&out, &index, c) || touched.iter().any(|t| t == c) || cands.iter().any(|x| x.1 == c)
                };
                let sep = if n.ends_with(|c: char| c.is_ascii_digit()) { "_" } else { "" };
                let free = (!n.is_empty() && is_token(&n) && !pass_keyword(&n))
                    .then(|| {
                        std::iter::once(n.clone()).chain((2..=20).map(|k| format!("{n}{sep}{k}"))).find(|c| !taken(c))
                    })
                    .flatten();
                if let Some(n) = free {
                    cands.push((name.clone(), n));
                }
            }
        }
        for (old, new) in &cands {
            if !is_token(new)
                || pass_keyword(new)
                || touched.contains(new)
                || in_scope(&out, &index, new)
            {
                continue;
            }
            // up to the line that ends the function: what follows its `end`
            // there is the enclosing code's (`end}, function(a)` opens the next)
            let stop = hi;
            // every line with the word, strings included: the index keeps them all
            let hits: Vec<u32> = index.within(old, fi, stop).to_vec();
            let lines: Vec<usize> = hits.iter().map(|j| *j as usize).collect();
            if !rename_lines(out, &lines, old, new) {
                continue; // a use it could not reach: inside a string's interpolation
            }
            renamed |= !hits.is_empty();
            index.add(new, &hits);
            touched.push(old.clone());
            touched.push(new.clone());
        }
    }
    renamed
}

/// A blank line between top-level statements and function definitions.
pub fn blank_lines(lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + lines.len() / 8);
    for (i, line) in lines.iter().enumerate() {
        let s = strip(line);
        let opens = indent_of(line) == 0
            && func_open(line).is_some()
            && (s.starts_with("local function ") || s.starts_with("function ") || {
                // ^[A-Za-z_][A-Za-z0-9_.]* = function\(
                let b = s.as_bytes();
                let mut e = 0;
                while e < b.len() && (is_word(b[e]) || b[e] == b'.') {
                    e += 1;
                }
                !b.is_empty() && is_start(b[0]) && s[e..].starts_with(" = function(")
            });
        if opens {
            let prev = out.last().map_or("", |l| strip(l));
            if !prev.is_empty() && !prev.starts_with("--") {
                out.push(String::new());
            }
        }
        out.push(line.clone());
        if line == "end" && i + 1 < lines.len() && !is_blank(&lines[i + 1]) {
            out.push(String::new());
        }
    }
    out
}

#[derive(Default)]
struct BlockWords {
    function: i32,
    end: i32,
    until: i32,
    repeat: i32,
    do_: i32,
}

/// Block keywords of a line, outside strings (`{...}` in backtick strings is
/// code, but never holds blocks).
fn code_words(line: &str) -> BlockWords {
    let b = line.as_bytes();
    let mut plain: Vec<u8> = Vec::with_capacity(b.len());
    let mut quote = 0u8;
    let mut k = 0;
    while k < b.len() {
        let ch = b[k];
        if quote != 0 {
            if ch == b'\\' {
                k += 2;
                continue;
            }
            if ch == quote {
                quote = 0;
            }
            plain.push(b' ');
        } else if ch == b'"' || ch == b'\'' || ch == b'`' {
            quote = ch;
            plain.push(b' ');
        } else if ch == b'-' && b.get(k + 1) == Some(&b'-') {
            break;
        } else {
            plain.push(ch);
        }
        k += 1;
    }
    // (?<![A-Za-z0-9_.:])(function|end|until|repeat|do)(?![A-Za-z0-9_])
    let mut w = BlockWords::default();
    let mut i = 0;
    while i < plain.len() {
        if is_word(plain[i]) {
            let e = word_end(&plain, i);
            let before_ok = i == 0 || !(plain[i - 1] == b'.' || plain[i - 1] == b':');
            if before_ok {
                match &plain[i..e] {
                    b"function" => w.function += 1,
                    b"end" => w.end += 1,
                    b"until" => w.until += 1,
                    b"repeat" => w.repeat += 1,
                    b"do" => w.do_ += 1,
                    _ => {}
                }
            }
            i = e;
        } else {
            i += 1;
        }
    }
    w
}

/// Indentation recomputed from the code: four spaces per open block. Opening:
/// `function`, an if/while/for statement, `do`, `repeat`; closing: `end`,
/// `until`. A line that starts by closing a block (or with else/elseif) sits
/// one level out.
fn reindent(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut depth: i64 = 0;
    let mut skip_do = 0; // a loop header's own `do`, possibly on a later line
    for line in &lines {
        let s = strip(line);
        if s.is_empty() {
            out.push(String::new());
            continue;
        }
        let w = code_words(s);
        let mut opens = (w.function + w.repeat) as i64;
        let closes = (w.end + w.until) as i64;
        let first = s.bytes().position(|c| matches!(c, b' ' | b'(' | b',' | b')')).map_or(s, |e| &s[..e]);
        if first == "if" {
            opens += 1; // a statement if always has an `end`, wherever `then` is
        }
        if first == "while" || first == "for" {
            opens += 1;
            skip_do += 1;
        }
        let mut do_count = w.do_;
        while do_count != 0 && skip_do != 0 {
            do_count -= 1;
            skip_do -= 1;
        }
        opens += do_count as i64;
        let lead_close = matches!(first, "end" | "until" | "else" | "elseif")
            || s.starts_with("end)")
            || s.starts_with("end,")
            || s.starts_with("end}")
            || s.starts_with('}');
        let level = if lead_close { depth - 1 } else { depth }.max(0) as usize;
        let mut l = String::with_capacity(level * 4 + s.len());
        for _ in 0..level {
            l.push_str("    ");
        }
        l.push_str(s);
        out.push(l);
        depth = (depth + opens - closes).max(0);
    }
    out
}

/// Everything in `tidy` before the cross-script naming rounds.
pub fn tidy_before_names(lines: Vec<String>) -> Vec<String> {
    let lines = reindent(lines);
    let lines = generic_for(lines);
    let lines = compound_assign(lines);
    let lines = interpolation(lines);
    let lines = local_functions(lines);
    let lines = sink_declarations(lines);
    let lines = rename_variables(lines);
    let lines = callback_params(lines);
    pcall_results(lines)
}
