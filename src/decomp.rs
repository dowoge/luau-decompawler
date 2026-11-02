//! One function at a time: bytecode to Luau statements.
//!
//! Values wait in `pending` to be folded into the expression that reads them;
//! liveness and reaching definitions (analysis.rs) decide what folds and what
//! becomes a variable. Trial folds run against an undo log: `mark()` and
//! `rollback()` replace copying the name tables at every conditional jump.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::OnceLock;

use crate::analysis::{effects, full_effects, twice, Flow, RegSet};
use crate::bytecode::{is_cond_jump, is_for, is_forgprep, op, op_name, Insn, Module, Proto, K};
use crate::expr::*;
use crate::hash::{key, HashMap, HashSet};
use crate::inline::{last_refusal, match_call, Arg, Inlined, Up};
use crate::passes;

/// Variables: a key below this is a register (register-named output), at or
/// above it a web root from analysis.rs.
const KW: u32 = 1024;

const KEYWORDS: [&str; 22] = [
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local",
    "nil", "not", "or", "repeat", "return", "then", "true", "until", "while", "continue",
];

fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

/// Make what is built on first use now, outside any job.
pub fn warm() {
    sp(0);
}

fn sp(n: usize) -> &'static str {
    static S: OnceLock<String> = OnceLock::new();
    &S.get_or_init(|| " ".repeat(1 << 16))[..n]
}

/// LUAU_WHY=all: every refused helper match, as it happens.
static WHY_ALL: OnceLock<bool> = OnceLock::new();

/// `--info`: a comment above each function on what the bytecode holds for it.
pub static INFO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `--info`, once names are final: each function's comment above the line it
/// starts on, the main function's first. Each says the function's number in
/// the module, its name and line, its parameters, registers and instructions,
/// the flags the compiler gave it, its upvalues as this text names them, and
/// its constants.
pub fn info_comments(lines: Vec<String>, m: &Module) -> Vec<String> {
    // how each function's creator captures its upvalues
    let mut kinds: HashMap<usize, Vec<u8>> = HashMap::default();
    for p in &m.protos {
        for (i, ins) in p.code.iter().enumerate() {
            let child = match ins.op {
                op::NEWCLOSURE => {
                    let d = ins.d();
                    p.children.get(if d < 0 { p.children.len() as i32 + d } else { d } as usize).copied()
                }
                op::DUPCLOSURE => match p.k.get(kindex(p, ins.d() as i64)) {
                    Some(K::Closure(c)) => Some(*c),
                    _ => None,
                },
                _ => None,
            };
            if let Some(c) = child {
                let caps = p.code[i + 1..].iter().take_while(|c| c.op == op::CAPTURE).map(|c| c.a() as u8);
                kinds.entry(c).or_insert_with(|| caps.collect());
            }
        }
    }
    let mut above: HashMap<usize, Vec<String>> = HashMap::default();
    for (i, l) in lines.iter().enumerate() {
        let Some(rest) = l.trim_start().strip_prefix(passes::INFO_MARK) else { continue };
        let mut parts = rest.split('|');
        let Some(idx) = parts.next().and_then(|n| n.parse::<usize>().ok()).filter(|n| *n < m.protos.len()) else {
            continue;
        };
        let names: Vec<&str> = parts.filter(|n| !n.is_empty()).collect();
        // the line the function starts on: the nearest one above, but blanks
        let Some(head) = (0..i).rev().find(|j| !lines[*j].trim().is_empty() && !passes::info_marker(&lines[*j])) else {
            continue;
        };
        let pad = sp(passes::indent_of(&lines[head]));
        let block = info_block(m, idx, &names, kinds.get(&idx).map_or(&[][..], Vec::as_slice), pad);
        above.entry(head).or_default().extend(block);
    }
    let mut out = info_block(m, m.main, &[], &[], "");
    out.reserve(lines.len());
    for (i, l) in lines.into_iter().enumerate() {
        if let Some(block) = above.remove(&i) {
            out.extend(block);
        }
        if !passes::info_marker(&l) {
            out.push(l);
        }
    }
    out
}

/// One function's comment, at indent `pad`.
fn info_block(m: &Module, idx: usize, names: &[&str], kinds: &[u8], pad: &str) -> Vec<String> {
    let p = &m.protos[idx];
    let plural = |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let title = match p.name.as_deref().filter(|n| !n.is_empty()) {
        _ if idx == m.main => format!("main function (function {idx})"),
        Some(n) => format!("function {idx}: {n}, line {}", p.linedefined),
        None => format!("function {idx}: anonymous, line {}", p.linedefined),
    };
    let mut props = vec![plural(p.numparams, "parameter", "parameters")];
    if p.vararg != 0 {
        props.push("vararg".to_string());
    }
    props.push(plural(p.maxstack, "register", "registers"));
    props.push(plural(p.code.len(), "instruction", "instructions"));
    if !p.children.is_empty() {
        props.push(plural(p.children.len(), "nested function", "nested functions"));
    }
    for (bit, what) in [(1, "native module"), (2, "cold"), (4, "has native functions"), (8, "inlinable"), (16, "uses export")] {
        if p.flags & bit != 0 {
            props.push(what.to_string());
        }
    }
    let mut out = vec![format!("{pad}-- {title}"), format!("{pad}--   {}", props.join(", "))];
    let ups: Vec<String> = (0..p.nups)
        .map(|i| {
            let name = names.get(i).copied().unwrap_or("?");
            // captured by reference: a variable the creator may still change
            if kinds.get(i) == Some(&1) {
                format!("{name} (by reference)")
            } else {
                name.to_string()
            }
        })
        .collect();
    info_list(pad, "upvalues", &ups, &mut out);
    let ks: Vec<String> = (0..p.k.len()).map(|i| info_constant(m, p, i)).collect();
    info_list(pad, "constants", &ks, &mut out);
    out
}

/// `--   what (n): a, b, c`, continued on further lines past 110 columns.
fn info_list(pad: &str, what: &str, items: &[String], out: &mut Vec<String>) {
    if items.is_empty() {
        out.push(format!("{pad}--   {what}: none"));
        return;
    }
    let mut line = format!("{pad}--   {what} ({}):", items.len());
    for (i, item) in items.iter().enumerate() {
        let piece = if i + 1 < items.len() { format!("{item},") } else { item.clone() };
        if line.len() + 1 + piece.len() > 110 && !line.ends_with(':') {
            out.push(std::mem::replace(&mut line, format!("{pad}--     {piece}")));
        } else {
            line.push(' ');
            line.push_str(&piece);
        }
    }
    out.push(line);
}

/// A constant as the comment shows it: as a value, an import by its path, a
/// template by its keys, a function by its number and name.
fn info_constant(m: &Module, p: &Proto, i: usize) -> String {
    match &p.k[i] {
        K::TableK(v) => {
            let keys: Vec<String> = v
                .iter()
                .filter_map(|(key, val)| {
                    let name = kstr(m, p, *key as usize)?;
                    let field = field_text(name);
                    Some(if *val < 0 { field } else { format!("{field} = {}", kexpr(m, p, *val as i64)) })
                })
                .collect();
            if keys.is_empty() {
                "{}".to_string()
            } else {
                format!("{{ {} }}", keys.join(", "))
            }
        }
        K::Closure(c) => match m.protos.get(*c).and_then(|f| f.name.as_deref()).filter(|n| !n.is_empty()) {
            Some(n) => format!("<function {c}: {n}>"),
            None => format!("<function {c}>"),
        },
        K::Class => "<class>".to_string(),
        _ => kexpr(m, p, i as i64),
    }
}

pub fn kstr<'a>(m: &'a Module, p: &Proto, i: usize) -> Option<&'a str> {
    match &p.k[i] {
        K::Str(Some(s)) => Some(&m.text[*s]),
        K::Str(None) => Some(""),
        _ => None,
    }
}

fn kstr_or<'a>(m: &'a Module, p: &Proto, i: usize, d: &'a str) -> &'a str {
    match kstr(m, p, i) {
        Some(s) if !s.is_empty() => s,
        _ => d,
    }
}

fn index_suffix(name: &str) -> String {
    if is_ident(name) && !is_keyword(name) {
        format!(".{name}")
    } else {
        format!("[{}]", lua_string(name.as_bytes()))
    }
}

pub fn import_expr(m: &Module, p: &Proto, iid: u32) -> String {
    let count = (iid >> 30) as usize;
    let ids = [(iid >> 20) & 1023, (iid >> 10) & 1023, iid & 1023];
    let mut parts = ids[..count.min(3)].iter().map(|i| {
        let i = *i as usize;
        match kstr(m, p, i) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => format!("K{i}"),
        }
    });
    let mut out = parts.next().expect("import with no parts");
    for x in parts {
        out.push_str(&index_suffix(&x));
    }
    out
}

fn kindex(p: &Proto, i: i64) -> usize {
    if i < 0 {
        (p.k.len() as i64 + i) as usize
    } else {
        i as usize
    }
}

/// Fields a DUPTABLE template already holds (type 8 keeps values).
/// A template's keys in order, as a constructor's fields: those with a value
/// as `key = value`, the others as slots their stores fill.
fn template_slots(m: &Module, p: &Proto, i: i64) -> Vec<String> {
    let K::TableK(v) = &p.k[kindex(p, i)] else { return Vec::new() };
    let mut out = Vec::with_capacity(v.len());
    for (key, val) in v {
        let name = if (*key as usize) < p.k.len() { kstr(m, p, *key as usize) } else { None };
        let Some(name) = name else { continue };
        let field = field_text(name);
        if *val < 0 {
            out.push(format!("{SLOT}{field}"));
        } else {
            out.push(format!("{field} = {}", kexpr(m, p, *val as i64)));
        }
    }
    out
}

/// A key as a constructor writes it: `name`, or `["any text"]`.
fn field_text(name: &str) -> String {
    if is_ident(name) && !is_keyword(name) {
        name.to_string()
    } else {
        format!("[{}]", lua_string(name.as_bytes()))
    }
}

fn template_fields(m: &Module, p: &Proto, i: i64) -> Vec<String> {
    let K::TableK(v) = &p.k[kindex(p, i)] else { return Vec::new() };
    let mut out = Vec::with_capacity(v.len());
    for (key, val) in v {
        let name = if (*key as usize) < p.k.len() { kstr(m, p, *key as usize) } else { None };
        let Some(name) = name else { continue };
        if *val < 0 {
            continue;
        }
        let field = if is_ident(name) && !is_keyword(name) {
            name.to_string()
        } else {
            format!("[{}]", lua_string(name.as_bytes()))
        };
        out.push(format!("{field} = {}", kexpr(m, p, *val as i64)));
    }
    out
}

pub fn kexpr(m: &Module, p: &Proto, i: i64) -> String {
    if i >= p.k.len() as i64 {
        return format!("K{i}");
    }
    match &p.k[kindex(p, i)] {
        K::Nil => "nil".into(),
        K::Bool(b) => if *b { "true" } else { "false" }.into(),
        K::Number(v) => fmt_number(*v),
        K::Integer(v) => v.to_string(),
        K::Str(Some(s)) => lua_string(&m.strings[*s]),
        K::Str(None) => "\"\"".into(),
        K::Import(v) => import_expr(m, p, *v),
        K::Vector(v) => {
            format!("Vector3.new({}, {}, {})", fmt_number(v[0]), fmt_number(v[1]), fmt_number(v[2]))
        }
        K::Closure(v) => format!("<function {v}>"),
        K::TableK(_) => {
            let fields = template_fields(m, p, i);
            if fields.is_empty() {
                "{}".into()
            } else {
                format!("{{ {} }}", fields.join(", "))
            }
        }
        K::Class => format!("K{i}"),
    }
}

fn module_globals(m: &Module) -> HashSet<String> {
    let mut names = HashSet::default();
    for p in &m.protos {
        for ins in &p.code {
            match ins.op {
                op::GETGLOBAL | op::SETGLOBAL => {
                    if let Some(s) = kstr(m, p, ins.aux as usize) {
                        if !s.is_empty() {
                            names.insert(s.to_string());
                        }
                    }
                }
                op::GETIMPORT => {
                    let e = import_expr(m, p, ins.aux);
                    let head = e.split('.').next().unwrap();
                    names.insert(head.to_string());
                }
                _ => {}
            }
        }
    }
    names
}

type Body = Rc<(Vec<String>, Vec<String>)>;

/// One module being decompiled.
pub struct Ctx<'m> {
    m: &'m Module,
    globals: HashSet<String>,
    /// A closure's text depends only on its proto, what it captures and its
    /// depth, and every enclosing pass asks for it again.
    memo: RefCell<HashMap<(usize, i32, Vec<Rc<str>>, u64), Body>>,
    /// LUAU_WHY: report what stopped a helper being recovered.
    why: bool,
    /// Counts renamed helpers: closure text calls helpers by name.
    helper_names: std::cell::Cell<u64>,
    noflat: bool,
    /// Functions held in named locals so far, which may have been inlined.
    helpers: RefCell<Vec<(usize, Rc<str>)>>,
    /// Per function: the source lines it spans.
    spans: Vec<(u32, u32)>,
    /// Per function, which of its upvalues a closure shares by reference.
    byref_ups: Vec<Vec<bool>>,
    /// Functions no other lists as a child: only ever inlined, like
    /// `(function() ... end)()`, since the compiler adds a child with its closure.
    orphans: Vec<usize>,
    /// Definitions the compiler dropped, per parent: (line, function).
    defs: HashMap<usize, Vec<(u32, usize)>>,
    /// What a dropped function's upvalues are called, from a call that showed it.
    def_upvals: RefCell<HashMap<usize, Vec<Rc<str>>>>,
}

enum Undo {
    Pending(usize, Option<E>),
    Name(usize, Option<Rc<str>>),
    Decl(usize),
    Undecl(usize),
    Used(Rc<str>),
    Hoist(bool),
    DefKey(E, Option<(usize, u32)>),
    Goto(i32),
    Consumed(usize, u32),
    /// A constructor had this many fields.
    Fields(E, usize),
    /// A constructor's field at this index was this.
    Field(E, usize, String),
    /// A constructor was complete: its fields and text before that.
    Sealed(E, Vec<String>, String),
}

/// A point to roll back to. While one lives, the trail behind it stays.
#[derive(Clone)]
struct Mark {
    trail: usize,
    decl: usize,
    _live: Rc<()>,
}

struct RegionFail;

struct Dec<'c, 'm> {
    ctx: &'c Ctx<'m>,
    m: &'m Module,
    p: &'m Proto,
    idx: usize,
    depth: i32,
    upvals: Vec<Rc<str>>,
    suffix: String,
    flow: Flow,
    cur_pc: i32,
    key_pc: Option<i32>,
    flat_regs: bool,
    /// Past the local limit even so: values wait through the tests on their
    /// way (`if` expressions, comparisons, `and`/`or` choices).
    tight: bool,
    flat: bool,
    regname: Vec<Option<Rc<str>>>,
    declared: Vec<bool>,
    used: HashSet<Rc<str>>,
    param_names: Vec<Rc<str>>,
    once_cache: HashMap<(i32, u32, i32), bool>,
    /// Register-named output: per web, a helper made here whose calls a
    /// later pass puts back by name (filled when that output starts).
    helper_webs: Vec<bool>,
    hoisted: Vec<Rc<str>>,
    hoisted_set: HashSet<Rc<str>>,
    /// Closure constants with no upvalues loaded at several places: one
    /// closure in Luau, declared once at the top under this name.
    shared_closures: HashMap<usize, Rc<str>>,
    exits: Vec<i32>,
    /// The [start, end) of the blocks being printed, innermost last.
    ranges: Vec<(i32, i32)>,
    loops: Vec<(i32, i32)>,
    goto_targets: BTreeSet<i32>,
    label_pcs: BTreeSet<i32>,
    backward: Vec<i32>,
    repeat_back: Vec<i32>,
    labels: Vec<bool>,
    incoming: Vec<u32>,
    /// Lowest pc of a jump to each pc.
    first_jump: Vec<i32>,
    consumed: Vec<u32>,
    pending: BTreeMap<usize, E>,
    base_indent: i32,
    join_ok: Option<i32>,
    in_region: bool,
    region_steps: u32,
    region_written: RegSet,
    region_join: Option<i32>,
    region_pc: i32,
    /// What was still waiting after the region's first test.
    region_head: Option<BTreeSet<usize>>,
    region_ind: usize,
    region_lines: Option<Vec<String>>,
    iter_call: Option<(usize, String)>,
    trail: Vec<Undo>,
    /// One more count for every Mark alive: none, and the trail can go.
    marks: Rc<()>,
    decl_log: Vec<usize>,
    watch_used: bool,
    flat_used: Option<HashSet<Rc<str>>>,
    /// Where stmt continues when it consumed more than its own instruction.
    skip_to: Option<i32>,
    /// The register and_or_value left its result in.
    joined_value: Option<usize>,
    /// Per pc: the helper inlined there, and its name.
    inlined: RefCell<Vec<Option<Option<(Rc<Inlined>, Rc<str>)>>>>,
    refusals: RefCell<Vec<(usize, String)>>,
    /// Dropped definitions still to come, next last.
    defs_left: Vec<(u32, usize)>,
    /// Computing part of a larger value: what it computes waits for that one.
    in_value: bool,
    value_reg: Option<usize>,
}

fn is_generated_v(s: &str, allow_x: bool) -> bool {
    let b = s.as_bytes();
    if b.first() != Some(&b'v') {
        return false;
    }
    let mut i = 1;
    let digits = |i: &mut usize| {
        let st = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i > st
    };
    if !digits(&mut i) {
        return false;
    }
    if allow_x && i < b.len() && b[i] == b'x' {
        i += 1;
        if !digits(&mut i) {
            return false;
        }
    }
    if i < b.len() && b[i] == b'_' {
        i += 1;
        if !digits(&mut i) {
            return false;
        }
    }
    i == b.len()
}

/// A call, as a statement can be one: a name, then what follows it with no
/// space outside brackets and strings, ending in `)` (`a.b:c(x, y)`,
/// `t[k](x)`; not `f(x) + 1`, `not f(x)`, `if c then f() else g()`).
fn is_call_text(text: &str) -> bool {
    let b = text.as_bytes();
    if !b.first().is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_') || b.last() != Some(&b')') {
        return false;
    }
    if text.starts_with("function") || text.starts_with("not(") {
        return false;
    }
    let (mut depth, mut quote) = (0i32, 0u8);
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == b'\\' {
                i += 1;
            } else if c == quote {
                quote = 0;
            }
        } else if matches!(c, b'"' | b'\'' | b'`') {
            quote = c;
        } else if matches!(c, b'(' | b'[' | b'{') {
            depth += 1;
        } else if matches!(c, b')' | b']' | b'}') {
            depth -= 1;
        } else if depth == 0 && c == b' ' {
            return false;
        }
        i += 1;
    }
    true
}

/// `(?<![A-Za-z0-9_.:])name(?![A-Za-z0-9_])`
fn contains_ident(text: &str, name: &str) -> bool {
    let t = text.as_bytes();
    let mut from = 0;
    while let Some(off) = text[from..].find(name) {
        let i = from + off;
        let before = i > 0 && {
            let c = t[i - 1];
            c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b':')
        };
        let e = i + name.len();
        let after = e < t.len() && (t[e].is_ascii_alphanumeric() || t[e] == b'_');
        if !before && !after {
            return true;
        }
        from = i + 1;
        while !text.is_char_boundary(from) {
            from += 1;
        }
    }
    false
}

/// The names contains_ident finds in a text: runs of letters, digits and `_`
/// not right after `.` or `:`.
fn ident_words(text: &str) -> impl Iterator<Item = &str> {
    let t = text.as_bytes();
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < t.len() {
            if t[i].is_ascii_alphanumeric() || t[i] == b'_' {
                let s = i;
                while i < t.len() && (t[i].is_ascii_alphanumeric() || t[i] == b'_') {
                    i += 1;
                }
                if s == 0 || !matches!(t[s - 1], b'.' | b':') {
                    return Some(&text[s..i]);
                }
            } else {
                i += 1;
            }
        }
        None
    })
}

/// The quoted identifier ending `text` as `<prefix>("name")`.
fn tail_string_arg<'t>(text: &'t str, close: &str) -> Option<(&'t str, &'t str)> {
    let s = text.strip_suffix(close)?;
    let q = s.rfind('"')?;
    let name = &s[q + 1..];
    let before = s[..q].strip_suffix('(')?;
    Some((before, name))
}

/// Names a value suggests for itself, best first: its function's name, the
/// field it was read from, the service, module or child it is, what it is
/// an element of, what a well-known call returns.
fn name_bases(text: &str, hint: Option<&str>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    out.extend(hint.map(String::from));
    if let Some(t) = field_tail(text) {
        out.push(t.to_string());
    }
    let lower = |s: &str| -> String {
        let mut c = s.chars();
        match c.next() {
            Some(_) if s.chars().all(|x| x.is_ascii_uppercase() || x == '_' || x.is_ascii_digit()) => s.to_ascii_lowercase(),
            Some(f) => f.to_ascii_lowercase().to_string() + c.as_str(),
            None => String::new(),
        }
    };
    if let Some((before, g)) = tail_string_arg(text, "\")") {
        if before.ends_with(":GetService") {
            out.push(g.to_string());
        }
        for f in [".WaitForChild", ":WaitForChild", ".FindFirstChild", ":FindFirstChild"] {
            if before.ends_with(f) {
                out.push(g.to_string());
            }
        }
        for f in [
            ":FindFirstChildOfClass", ":FindFirstChildWhichIsA", ":FindFirstAncestorOfClass",
            ":FindFirstAncestorWhichIsA", ":FindFirstAncestor",
        ] {
            if before.ends_with(f) {
                out.push(lower(g));
            }
        }
        if before == "Instance.new" || before.ends_with(":GetAttribute") {
            out.push(lower(g));
        }
    }
    if text.starts_with("require(") && !text.contains('\n') {
        if let Some((before, g)) = tail_string_arg(text, "\"))") {
            for f in [".WaitForChild", ":WaitForChild", ".FindFirstChild", ":FindFirstChild"] {
                if before.len() >= 8 + f.len() && before.ends_with(f) {
                    out.push(g.to_string());
                }
            }
        }
        if let Some(s) = text.strip_suffix(')') {
            if let Some(dot) = s.rfind('.') {
                if dot >= 8 {
                    out.push(s[dot + 1..].to_string());
                }
            }
        }
    }
    // `x and x.Value`: the field
    if let Some((a, b)) = text.split_once(" and ") {
        if let Some(f) = b.strip_prefix(a).and_then(|r| r.strip_prefix('.')).filter(|f| is_ident(f)) {
            out.push(lower(f));
        }
    }
    // an element of something plural: `holders[k]`, `self.items[i]`
    if text.ends_with(']') && !text.contains('\n') {
        if let Some(open) = text.find('[') {
            if let Some(n) = passes::element_noun(&text[..open]) {
                out.push(n);
            }
        }
    }
    // what well-known calls give
    for (tail, name) in [
        (":GetChildren()", "children"),
        (":GetDescendants()", "descendants"),
        (":GetPlayers()", "players"),
        (":GetTouchingParts()", "parts"),
        (":GetAttributes()", "attributes"),
        (":GetMouse()", "mouse"),
        (":GetPivot()", "pivot"),
        (":GetFullName()", "fullName"),
        (":Clone()", "clone"),
        (":GetTags()", "tags"),
    ] {
        if text.ends_with(tail) && !text.contains('\n') {
            out.push(name.to_string());
        }
    }
    for (head, name) in [
        (":GetPlayerFromCharacter(", "player"),
        (":LoadAnimation(", "track"),
        (":Raycast(", "result"),
        (":FindFirstChild(", "child"),
        (":WaitForChild(", "child"),
        (":GetPropertyChangedSignal(", "signal"),
        (":GetAttributeChangedSignal(", "signal"),
    ] {
        if let Some(at) = text.find(head) {
            if text.ends_with(')') && !text[..at].contains('(') {
                out.push(name.to_string());
            }
        }
    }
    for (head, name) in [
        ("table.create(", "list"),
        ("table.clone(", "copy"),
        ("table.concat(", "text"),
        ("string.format(", "text"),
        ("string.split(", "parts"),
        ("buffer.create(", "buf"),
        ("os.clock(", "now"),
        ("tick(", "now"),
        ("tostring(", "text"),
        ("setmetatable(", "object"),
        ("TweenInfo.new(", "tweenInfo"),
        ("Color3.fromRGB(", "color"),
        ("Color3.new(", "color"),
        ("UDim2.new(", "size"),
        ("UDim2.fromOffset(", "size"),
        ("UDim2.fromScale(", "size"),
        ("CFrame.new(", "cframe"),
        ("Vector3.new(", "position"),
        ("Vector2.new(", "point"),
        ("NumberSequence.new(", "sequence"),
        ("RaycastParams.new(", "params"),
        ("OverlapParams.new(", "params"),
    ] {
        if text.starts_with(head) && text.ends_with(')') && !text.contains('\n') {
            out.push(name.to_string());
        }
    }
    out
}

fn field_tail(text: &str) -> Option<&str> {
    let b = text.as_bytes();
    if b.is_empty() || !(b[0].is_ascii_alphabetic() || b[0] == b'_') {
        return None;
    }
    if !b.iter().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.')) {
        return None;
    }
    let dot = text.rfind('.')?;
    let tail = &text[dot + 1..];
    is_ident(tail).then_some(tail)
}

pub fn decompile_lines(m: &Module, noflat: bool) -> Vec<String> {
    let spans = m
        .protos
        .iter()
        .map(|p| (p.linedefined, p.lines.iter().copied().max().unwrap_or(0)))
        .collect();
    let mut made: HashSet<usize> = HashSet::default();
    for p in &m.protos {
        for ins in &p.code {
            if ins.op == op::NEWCLOSURE {
                let d = ins.d();
                let i = if d < 0 { p.children.len() as i32 + d } else { d };
                made.extend(p.children.get(i as usize));
            } else if ins.op == op::DUPCLOSURE {
                if let K::Closure(c) = &p.k[kindex(p, ins.d() as i64)] {
                    made.insert(*c);
                }
            }
        }
    }
    let spans: Vec<(u32, u32)> = spans;
    let closureless = |i: &usize| *i != m.main && !made.contains(i) && !m.protos[*i].lines.is_empty();
    let unnamed = |i: usize| m.protos[i].name.as_deref().is_none_or(str::is_empty);
    let orphans: Vec<usize> = (0..m.protos.len()).filter(closureless).filter(|i| unnamed(*i)).collect();
    // `local function f`, every call inlined: the compiler made no closure,
    // but the source had the definition, inside the smallest function around it
    let mut defs: HashMap<usize, Vec<(u32, usize)>> = HashMap::default();
    let mut helpers = Vec::new();
    for i in (0..m.protos.len()).filter(closureless).filter(|i| !unnamed(*i)) {
        let (from, to) = spans[i];
        let around = (0..m.protos.len())
            .filter(|o| *o != i && spans[*o].0 <= from && to <= spans[*o].1)
            .min_by_key(|o| spans[*o].1 - spans[*o].0);
        if let Some(parent) = around {
            defs.entry(parent).or_default().push((from, i));
            helpers.push((i, Rc::from(m.protos[i].name.as_deref().unwrap_or(""))));
        }
    }
    defs.values_mut().for_each(|v| v.sort_unstable());
    // per function, its upvalues some closure creating it shares by reference
    let mut byref_ups: Vec<Vec<bool>> = m.protos.iter().map(|p| vec![false; p.nups]).collect();
    for p in &m.protos {
        for (i, ins) in p.code.iter().enumerate() {
            let child = match ins.op {
                op::NEWCLOSURE => {
                    let d = ins.d();
                    let k = if d < 0 { p.children.len() as i32 + d } else { d };
                    p.children.get(k as usize).copied()
                }
                op::DUPCLOSURE => match &p.k[kindex(p, ins.d() as i64)] {
                    K::Closure(c) => Some(*c),
                    _ => None,
                },
                _ => None,
            };
            let Some(c) = child.filter(|c| *c < byref_ups.len()) else { continue };
            for (u, cap) in p.code[i + 1..].iter().take_while(|x| x.op == op::CAPTURE).enumerate() {
                if cap.a() == 1 {
                    if let Some(slot) = byref_ups[c].get_mut(u) {
                        *slot = true;
                    }
                }
            }
        }
    }
    let ctx = Ctx {
        m,
        byref_ups,
        globals: module_globals(m),
        memo: RefCell::new(HashMap::default()),
        helper_names: std::cell::Cell::new(0),
        why: std::env::var_os("LUAU_WHY").is_some(),
        noflat,
        helpers: RefCell::new(helpers),
        spans,
        orphans,
        defs,
        def_upvals: RefCell::new(HashMap::default()),
    };
    let mut d = Dec::new(&ctx, m.main, Vec::new(), -1);
    let (_, body) = d.decompile();
    body
}

impl<'c, 'm> Dec<'c, 'm> {
    fn new(ctx: &'c Ctx<'m>, idx: usize, upvals: Vec<Rc<str>>, depth: i32) -> Self {
        Self::with_flow(ctx, idx, upvals, depth, None)
    }

    /// new(), with the analysis of this function if it was made already.
    fn with_flow(ctx: &'c Ctx<'m>, idx: usize, upvals: Vec<Rc<str>>, depth: i32, flow: Option<Flow>) -> Self {
        let m = ctx.m;
        let p = &m.protos[idx];
        let upvals: Vec<Rc<str>> = if upvals.is_empty() {
            (0..p.nups).map(|i| Rc::from(format!("upval{i}"))).collect()
        } else {
            upvals
        };
        let own = ctx.spans[idx];
        let foreign = |line: u32| {
            let inside = |s: &(u32, u32)| own.0 <= s.0 && s.1 <= own.1 && *s != own;
            line < own.0 || line > own.1 || ctx.spans.iter().any(|s| inside(s) && s.0 <= line && line <= s.1)
        };
        let flow = flow.unwrap_or_else(|| Flow::new(p, &foreign));
        let suffix = if depth < 0 { String::new() } else { format!("_{}", depth + 1) };
        let size = p.sizecode.max(0) as usize + 2;
        let mut d = Dec {
            ctx,
            m,
            p,
            idx,
            depth,
            used: upvals.iter().cloned().collect(),
            upvals,
            suffix,
            regname: vec![None; KW as usize + flow.ndefs],
            declared: vec![false; KW as usize + flow.ndefs],
            flow,
            cur_pc: 0,
            key_pc: None,
            flat_regs: false,
            tight: false,
            flat: false,
            param_names: Vec::new(),
            once_cache: HashMap::default(),
            helper_webs: Vec::new(),
            hoisted: Vec::new(),
            hoisted_set: HashSet::default(),
            shared_closures: HashMap::default(),
            exits: Vec::new(),
            ranges: Vec::new(),
            loops: Vec::new(),
            goto_targets: BTreeSet::new(),
            label_pcs: BTreeSet::new(),
            backward: vec![-1; size],
            repeat_back: vec![-1; size],
            labels: vec![false; size],
            incoming: vec![0; size],
            first_jump: vec![i32::MAX; size],
            consumed: vec![0; size],
            pending: BTreeMap::new(),
            base_indent: 0,
            join_ok: None,
            in_region: false,
            region_steps: 0,
            region_written: RegSet::EMPTY,
            region_join: None,
            region_pc: 0,
            region_head: None,
            region_ind: 0,
            region_lines: None,
            iter_call: None,
            trail: Vec::new(),
            marks: Rc::new(()),
            decl_log: Vec::new(),
            watch_used: false,
            flat_used: None,
            skip_to: None,
            joined_value: None,
            inlined: RefCell::new(vec![None; p.lines.len()]),
            refusals: RefCell::new(Vec::new()),
            defs_left: Vec::new(),
            in_value: false,
            value_reg: None,
        };
        for i in 0..p.numparams {
            let name: Rc<str> = Rc::from(format!("arg{}{}", i + 1, d.suffix));
            let key = KW + d.flow.entry[i];
            d.regname[key as usize] = Some(name.clone());
            d.declared[key as usize] = true;
            d.regname[i] = Some(name.clone());
            d.declared[i] = true;
            d.param_names.push(name);
        }
        // named locals from debug info, when the script kept it
        for (name, s, reg) in &p.locals {
            if *reg < p.numparams && *s == 0 {
                let k = d.var_key(*reg) as usize;
                d.regname[k] = Some(Rc::from(name.as_str()));
            }
        }
        d.collect_labels();
        for ins in &p.code {
            if let Some(jt) = ins.jump_target() {
                if !matches!(ins.op, op::FORNLOOP | op::FORGLOOP | op::FORNPREP) && !is_forgprep(ins.op) {
                    if let Some(c) = d.incoming.get_mut(jt.max(-1) as usize) {
                        *c += 1;
                    }
                }
            }
        }
        d
    }

    fn collect_labels(&mut self) {
        let p = self.p;
        for ins in &p.code {
            let (pc, d, o) = (ins.pc, ins.d(), ins.op);
            if matches!(o, op::JUMP | op::JUMPBACK) || is_cond_jump(o) {
                self.set_label(pc + 1 + d, pc);
            } else if o == op::JUMPX {
                self.set_label(ins.jump_target().unwrap(), pc);
            } else if o == op::LOADB && ins.c() != 0 && ins.c() != 1 {
                self.set_label(pc + 1 + ins.c() as i32, pc);
            }
            let head = pc + 1 + d;
            if head <= pc && head >= 0 {
                if matches!(o, op::JUMPBACK | op::JUMP) {
                    let h = head as usize;
                    self.backward[h] = self.backward[h].max(pc);
                } else if is_cond_jump(o) {
                    let h = head as usize;
                    self.repeat_back[h] = self.repeat_back[h].max(pc);
                }
            }
        }
    }

    fn set_label(&mut self, pc: i32, from: i32) {
        if pc >= 0 {
            if let Some(l) = self.labels.get_mut(pc as usize) {
                *l = true;
                let f = &mut self.first_jump[pc as usize];
                *f = from.min(*f);
            }
        }
    }

    #[inline]
    fn has_label(&self, pc: i32) -> bool {
        pc >= 0 && self.labels.get(pc as usize).copied().unwrap_or(false)
    }

    #[inline]
    fn back_of(table: &[i32], pc: i32) -> Option<i32> {
        if pc < 0 {
            return None;
        }
        match table.get(pc as usize) {
            Some(&v) if v >= 0 => Some(v),
            _ => None,
        }
    }

    #[inline]
    fn is_loop_head(&self, pc: i32) -> bool {
        Self::back_of(&self.backward, pc).is_some()
    }

    #[inline]
    fn is_repeat_head(&self, pc: i32) -> bool {
        Self::back_of(&self.repeat_back, pc).is_some()
    }

    #[inline]
    fn valid(&self, pc: i32) -> bool {
        self.flow.idx(pc).is_some()
    }

    #[inline]
    fn insn(&self, pc: i32) -> Insn {
        self.p.code[self.flow.idx(pc).expect("no instruction at pc")]
    }

    #[inline]
    fn counter(table: &[u32], pc: i32) -> u32 {
        if pc < 0 {
            return 0;
        }
        table.get(pc as usize).copied().unwrap_or(0)
    }

    // -- variables
    fn nearest_def_key(&self, r: usize, pc: i32) -> u32 {
        let list = self.flow.def_pcs.get(r).map(Vec::as_slice).unwrap_or(&[]);
        let k = list.partition_point(|d| *d <= pc);
        if k > 0 {
            self.flow.def_web(list[k - 1], r).unwrap()
        } else {
            self.flow.entry[r]
        }
    }

    /// The variable register r held before the variable `key` began.
    fn prev_key(&self, r: usize, key: u32) -> Option<u32> {
        if key < KW {
            return None;
        }
        let web = key - KW;
        let list = self.flow.def_pcs.get(r).map(Vec::as_slice).unwrap_or(&[]);
        let start = self.flow.web_span[web as usize].0;
        let first = list.partition_point(|d| *d < start);
        let best = list[..first].iter().rev().find(|d| self.flow.def_web(**d, r) != Some(web));
        Some(KW + match best {
            Some(d) => self.flow.def_web(*d, r).unwrap(),
            None => *self.flow.entry.get(r)?,
        })
    }

    /// The variable register r stands for at the current instruction.
    fn var_key(&self, r: usize) -> u32 {
        let pc = self.key_pc.unwrap_or(self.cur_pc);
        let w = self.flow.use_web(pc, r).or_else(|| self.flow.def_web(pc, r));
        if self.flat_regs {
            // too many variables for one function: name registers, except
            // variables a closure shares and helpers whose calls come back
            // by name (those must stay apart)
            return match w {
                Some(w) if self.apart(w) => KW + w,
                _ => r as u32,
            };
        }
        if let Some(w) = w {
            return KW + w;
        }
        if r >= self.flow.nregs {
            return r as u32;
        }
        KW + self.nearest_def_key(r, pc)
    }

    fn live_after(&self, pc: i32, r: usize) -> bool {
        self.shared(pc, r) || self.flow.idx(pc).is_some_and(|i| self.flow.live_out(&self.p.code, i).has(r))
    }

    fn live_at(&self, pc: i32, r: usize) -> bool {
        self.flow.use_web(pc, r).is_some_and(|w| self.flow.byref_only[w as usize])
            || self.flow.idx(pc).is_some_and(|i| self.flow.live_in(i).has(r))
    }

    /// Is the variable written or read at pc in r shared with a closure
    /// (captured by reference, before its upvalue is closed)?
    fn shared(&self, pc: i32, r: usize) -> bool {
        let w = self.flow.def_web(pc, r).or_else(|| self.flow.use_web(pc, r));
        w.is_some_and(|w| self.flow.byref_only[w as usize])
    }

    fn is_used(&self, s: &str) -> bool {
        self.used.contains(s) || self.ctx.globals.contains(s)
    }

    fn add_used(&mut self, s: &Rc<str>) {
        if self.used.insert(s.clone()) {
            self.trail.push(Undo::Used(s.clone()));
        }
    }

    fn set_name(&mut self, key: u32, name: Rc<str>) {
        let old = self.regname[key as usize].replace(name);
        self.trail.push(Undo::Name(key as usize, old));
    }

    fn declare(&mut self, key: u32) {
        let k = key as usize;
        if !self.declared[k] {
            self.declared[k] = true;
            self.trail.push(Undo::Decl(k));
            self.decl_log.push(k);
        }
    }

    fn undeclare(&mut self, key: u32) {
        let k = key as usize;
        if self.declared[k] {
            self.declared[k] = false;
            self.trail.push(Undo::Undecl(k));
        }
    }

    fn mark(&self) -> Mark {
        Mark { trail: self.trail.len(), decl: self.decl_log.len(), _live: self.marks.clone() }
    }

    fn rollback(&mut self, m: Mark) {
        if self.watch_used {
            // the dispatcher form reserves the names known at this point
            self.flat_used = Some(self.used.clone());
            self.watch_used = false;
        }
        while self.trail.len() > m.trail {
            match self.trail.pop().unwrap() {
                Undo::Pending(r, Some(e)) => {
                    self.pending.insert(r, e);
                }
                Undo::Pending(r, None) => {
                    self.pending.remove(&r);
                }
                Undo::Name(k, old) => self.regname[k] = old,
                Undo::Decl(k) => self.declared[k] = false,
                Undo::Undecl(k) => self.declared[k] = true,
                Undo::Used(s) => {
                    self.used.remove(&s);
                }
                Undo::DefKey(e, old) => e.borrow_mut().def_key = old,
                Undo::Goto(pc) => {
                    self.goto_targets.remove(&pc);
                }
                Undo::Consumed(pc, n) => self.consumed[pc] -= n,
                Undo::Fields(e, n) => {
                    if let Some(f) = e.borrow_mut().fields.as_mut() {
                        f.truncate(n);
                    }
                }
                Undo::Field(e, at, old) => {
                    if let Some(f) = e.borrow_mut().fields.as_mut() {
                        if at < f.len() {
                            f[at] = old;
                        }
                    }
                }
                Undo::Sealed(e, fields, text) => {
                    let mut x = e.borrow_mut();
                    x.fields = Some(fields);
                    x.text = text;
                }
                Undo::Hoist(first) => {
                    let name = self.hoisted.pop().expect("hoisted out of step with the log");
                    if first {
                        self.hoisted_set.remove(&name);
                    }
                }
            }
        }
        self.decl_log.truncate(m.decl);
    }

    /// A store gives a template key its value, in the key's place.
    fn fill_slot(&mut self, e: &E, at: usize, text: String) {
        let mut x = e.borrow_mut();
        let x = &mut *x;
        let names = text.as_bytes().split(|c| !is_ident_byte(*c));
        x.names_read.extend(names.filter(|n| !n.is_empty()).map(key));
        let f = x.fields.as_mut().expect("a table under construction");
        let old = std::mem::replace(&mut f[at], text);
        self.trail.push(Undo::Field(e.clone(), at, old));
    }

    /// These fields join the constructor e.
    fn add_fields(&mut self, e: &E, new: impl IntoIterator<Item = String>) {
        let mut x = e.borrow_mut();
        let x = &mut *x;
        let f = x.fields.as_mut().expect("a table under construction");
        self.trail.push(Undo::Fields(e.clone(), f.len()));
        for text in new {
            // kept after an undo too: at worst a value is written out early
            let names = text.as_bytes().split(|c| !is_ident_byte(*c));
            x.names_read.extend(names.filter(|n| !n.is_empty()).map(key));
            f.push(text);
        }
    }

    /// The constructor e is complete: its text is final.
    fn seal(&mut self, e: &E) {
        let mut x = e.borrow_mut();
        if x.fields.is_some() {
            let text = x.render();
            let fields = x.fields.take().unwrap_or_default();
            let old = std::mem::replace(&mut x.text, text);
            self.trail.push(Undo::Sealed(e.clone(), fields, old));
        }
    }

    fn add_goto(&mut self, pc: i32) {
        if self.goto_targets.insert(pc) {
            self.trail.push(Undo::Goto(pc));
        }
    }

    fn consume(&mut self, pc: i32, n: u32) {
        self.consumed[pc as usize] += n;
        self.trail.push(Undo::Consumed(pc as usize, n));
    }

    /// e is the value of variable `key` at register r. Values are shared, so
    /// this has to be undone with whatever trial decided it.
    fn set_def_key(&mut self, e: &E, r: usize, key: u32) {
        let old = e.borrow_mut().def_key.replace((r, key));
        self.trail.push(Undo::DefKey(e.clone(), old));
    }

    /// What no mark can roll back to holds every value the function made
    /// until now: let it go.
    #[inline]
    fn forget(&mut self) {
        if self.trail.len() > 4096 && Rc::strong_count(&self.marks) == 1 {
            self.trail.clear();
        }
    }

    fn pset(&mut self, r: usize, e: E) {
        self.forget();
        let old = self.pending.insert(r, e);
        self.trail.push(Undo::Pending(r, old));
    }

    fn ppop(&mut self, r: usize) -> Option<E> {
        let old = self.pending.remove(&r);
        if old.is_some() {
            self.forget();
            self.trail.push(Undo::Pending(r, old.clone()));
        }
        old
    }

    fn reg(&self, r: usize) -> Rc<str> {
        match &self.regname[self.var_key(r) as usize] {
            Some(n) => n.clone(),
            None => Rc::from(format!("v{r}{}", self.suffix)),
        }
    }

    fn fresh_name(&mut self, r: usize) -> Rc<str> {
        let mut name = format!("v{r}{}", self.suffix);
        let mut k = 2;
        while self.is_used(&name) {
            name = format!("v{r}x{k}{}", self.suffix);
            k += 1;
        }
        let name: Rc<str> = Rc::from(name);
        self.add_used(&name);
        name
    }

    fn name_for(&mut self, r: usize) -> Rc<str> {
        let key = self.var_key(r);
        if let Some(n) = &self.regname[key as usize] {
            return n.clone();
        }
        let name = self.fresh_name(r);
        self.set_name(key, name.clone());
        name
    }

    fn get(&mut self, r: usize) -> E {
        if let Some(e) = self.ppop(r) {
            self.seal(&e);
            return e;
        }
        ex_pure(&*self.reg(r))
    }

    /// Write out pending values that are still live (register stores).
    fn flush(&mut self, out: &mut Vec<String>, ind: usize, keep: &[usize]) {
        if self.pending.is_empty() {
            return;
        }
        let keys: Vec<usize> = self.pending.keys().copied().collect();
        for r in keys {
            let Some(v) = self.pending.get(&r) else { continue }; // written out on the way
            if keep.contains(&r) || v.borrow().method {
                continue; // a method lookup waits for its call
            }
            let e = self.get(r);
            self.assign(out, ind, r, &e);
        }
    }

    fn nice_name(&self, text: &str, hint: Option<&str>) -> Option<String> {
        // what the value says it is, best first; a name taken gets a number
        for base in name_bases(text, hint) {
            if let Some(n) = self.free_name(&base) {
                return Some(n);
            }
        }
        None
    }

    /// `base`, or `base2` ... `base9` when that is taken.
    fn free_name(&self, base: &str) -> Option<String> {
        if !is_ident(base) || is_keyword(base) || is_generated_v(base, true) {
            return None;
        }
        if !self.is_used(base) {
            return Some(base.to_string());
        }
        if base.ends_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        (2..=9).map(|n| format!("{base}{n}")).find(|c| !self.is_used(c))
    }

    fn assign(&mut self, out: &mut Vec<String>, ind: usize, r: usize, e: &E) {
        let old_key = self.key_pc;
        let dk = e.borrow().def_key;
        let key = match dk {
            Some((dr, k)) if dr == r => {
                if self.flat_regs && (k < KW || !self.apart(k - KW)) {
                    r as u32 // named by register in this mode
                } else {
                    k
                }
            }
            _ => self.var_key(r),
        };
        self.key_pc = None;
        self.assign_key(out, ind, r, key, e);
        self.key_pc = old_key;
    }

    fn assign_key(&mut self, out: &mut Vec<String>, ind: usize, r: usize, key: u32, e: &E) {
        let x = e.borrow();
        let k = key as usize;
        if self.regname[k].is_none() {
            // x = f(x): a new live range of a variable that is still around
            if let Some(prev) = self.prev_key(r, key) {
                if let Some(pname) = self.regname[prev as usize].clone() {
                    if !pname.is_empty() && prev != key && contains_ident(&x.text, &pname) {
                        self.set_name(key, pname);
                        if self.declared[prev as usize] {
                            self.declare(key);
                        }
                    }
                }
            }
        }
        let generic = match &self.regname[k] {
            None => true,
            Some(n) => is_generated_v(n, true),
        };
        let mut nice = self.nice_name(&x.text, x.hint.as_deref());
        if nice.is_some() && key >= KW && self.flow.web_defs[(key - KW) as usize] > 1 {
            nice = None; // set in several places: one value's name would mislead
        }
        if let Some(n) = nice {
            if !self.declared[k] && generic {
                let n: Rc<str> = Rc::from(n);
                self.set_name(key, n.clone());
                self.add_used(&n);
            }
        }
        if self.regname[k].is_none() {
            let n = self.fresh_name(r);
            self.set_name(key, n);
        }
        let name = self.regname[k].clone().unwrap();
        let pad = sp(ind);
        if let Some(child) = x.closure {
            let mut helpers = self.ctx.helpers.borrow_mut();
            if helpers.iter().any(|h| h.0 == child && h.1 != name) {
                self.ctx.helper_names.set(self.ctx.helper_names.get() + 1);
            }
            helpers.retain(|h| h.0 != child);
            helpers.push((child, name.clone()));
            self.inlined.borrow_mut().fill(None); // asked too early, the answer was no
        }
        let waiting = self.waiting_on(r, &name);
        if !waiting.is_empty() && self.declared[k] {
            if let Some(line) = self.multi_assign(r, &name, &x.text, &waiting) {
                out.push(format!("{pad}{line}"));
                return;
            }
        }
        for q in waiting {
            let v = self.get(q);
            self.assign(out, ind, q, &v);
        }
        if self.declared[k] {
            out.push(format!("{pad}{name} = {}", x.text));
        } else if ind as i32 > self.base_indent && !self.ends_in_block(key) {
            // first set inside a branch or loop: declared at the top instead,
            // so later code outside the block still sees it
            self.declare(key);
            out.push(format!("{pad}{name} = {}", x.text));
            self.hoist(name);
        } else {
            self.declare(key);
            match self.definition_rest(&x, None) {
                Some(rest) => out.push(format!("{pad}local function {name}{rest}")),
                None => out.push(format!("{pad}local {name} = {}", x.text)),
            }
        }
    }

    /// What follows `function` in a definition statement for a closure value,
    /// when the source wrote one: the compiler names a function only for
    /// `function f()`, `local function f()`, `function t.k()` (and a
    /// constructor's `k = function`), never for `f = function()` or `local f =
    /// function()`. `want`: the name the target has to match.
    fn definition_rest(&self, x: &X, want: Option<&str>) -> Option<String> {
        let child = x.closure?;
        let name = self.m.protos[child].name.as_deref().filter(|n| !n.is_empty())?;
        if want.is_some_and(|w| w != name) {
            return None;
        }
        x.text.strip_prefix("function(").map(|r| format!("({r}"))
    }

    /// Is the variable `key` written first and then all used inside the block
    /// being printed? It is that block's local then: declared any further out
    /// it would outlive a pass of a loop, and closures would share it.
    fn ends_in_block(&self, key: u32) -> bool {
        if key < KW || self.flat {
            return false;
        }
        let Some((start, end)) = self.ranges.last().copied() else { return false };
        let (write, read, last) = self.flow.web_span[(key - KW) as usize];
        start <= write && write < read && last < end
    }

    /// Is this helper created by a closure in this very function?
    fn helper_is_local(&self, helper: usize) -> bool {
        self.p.code.iter().any(|ins| match ins.op {
            op::NEWCLOSURE => self.p.children.get(ins.d().max(0) as usize) == Some(&helper),
            op::DUPCLOSURE => matches!(self.p.k.get(ins.d().max(0) as usize), Some(K::Closure(c)) if *c == helper),
            _ => false,
        })
    }

    /// LUAU_WHY: remember how far a helper got.
    fn note_refusal(&self, helper: usize, why: (usize, String)) {
        if self.ctx.why {
            let name = self.m.protos[helper].name.as_deref().unwrap_or("(anon)");
            if WHY_ALL.get_or_init(|| std::env::var("LUAU_WHY").is_ok_and(|v| v == "all")).to_owned() {
                let within = self.p.name.as_deref().unwrap_or("(anon)");
                eprintln!("REFUSE\t{within}\t{}\t{name}\t{}\t{}", self.cur_pc, why.0, why.1);
            }
            self.refusals.borrow_mut().push((why.0, format!("{name}: {}", why.1)));
        }
    }

    /// The local helper whose code the compiler copied in at pc, if any.
    fn inline_at(&self, pc: i32) -> Option<(Rc<Inlined>, Rc<str>)> {
        if self.flat {
            return None;
        }
        let line = *self.p.lines.get(pc as usize)?;
        if let Some(known) = &self.inlined.borrow()[pc as usize] {
            return known.clone();
        }
        let start = self.flow.idx(pc)?;
        let own = self.ctx.spans[self.idx];
        let mut helpers = self.ctx.helpers.borrow().clone();
        // a closure made just now has no name yet; its first call may be next
        for child in self.pending.values().filter_map(|e| e.borrow().closure) {
            let named = self.m.protos[child].name.as_deref().filter(|n| !n.is_empty());
            if let Some(n) = named.filter(|_| helpers.iter().all(|h| h.0 != child)) {
                helpers.push((child, Rc::from(n)));
            }
        }
        let mut found = None;
        for (idx, name) in helpers {
            let (from, to) = self.ctx.spans[idx];
            if idx == self.idx || line < from || line > to || (from <= own.0 && own.1 <= to) {
                continue; // not its lines, or it encloses this function
            }
            // another local of this function under that name would take the
            // call; one is the helper itself, when it is defined here
            let same = self.regname.iter().flatten().filter(|n| **n == name).count();
            let here = self.flow.ndefs > 0 && self.helper_is_local(idx);
            if same > usize::from(here) {
                continue;
            }
            let Some(call) = match_call(self.m, self.p, idx, start) else {
                self.note_refusal(idx, last_refusal());
                continue;
            };
            // made here, its upvalues are what it captured: code reading some
            // other register only looks like it (a line after a copy keeps
            // the helper's line too)
            let caps = self.captures_of(idx);
            if let Some(caps) = &caps {
                let wrong = call.upvals.iter().enumerate().any(|(k, u)| u.is_some_and(|u| caps.get(k) != Some(&u)));
                if wrong {
                    self.note_refusal(idx, (usize::MAX, "an upvalue the closure did not capture".to_string()));
                    continue;
                }
            }
            // a local written for the helper's upvalue: only a closure whose
            // captures show it does that
            if !call.upval_writes.is_empty() && caps.is_none() {
                self.note_refusal(idx, (usize::MAX, "writes a local no capture shows".to_string()));
                continue;
            }
            let leftover = call.written.iter().any(|r| {
                !call.holds_result(*r)
                    && !call.upval_writes.contains(r)
                    && self.valid(call.end)
                    && self.live_at(call.end, *r)
            });
            let dropped = self.ctx.defs.values().flatten().any(|d| d.1 == idx);
            if !leftover && (!dropped || call.upvals.iter().all(Option::is_some)) {
                found = Some((Rc::new(call), name));
                break;
            }
            let live: Vec<usize> =
                call.written.iter().copied().filter(|r| self.valid(call.end) && self.live_at(call.end, *r)).collect();
            self.note_refusal(idx, (usize::MAX, format!("matched at {} to {}, live {live:?} result {:?}", self.p.code[start].pc, call.end, call.result)));
        }
        // `(function() ... end)()`: inlined, and no closure was ever made
        if found.is_none() {
            for &idx in &self.ctx.orphans {
                let (from, to) = self.ctx.spans[idx];
                if idx == self.idx || line < from || line > to || from < own.0 || to > own.1 {
                    continue;
                }
                let Some(call) = match_call(self.m, self.p, idx, start) else {
                    self.note_refusal(idx, last_refusal());
                    continue;
                };
                let leftover = call
                    .written
                    .iter()
                    .any(|r| !call.holds_result(*r) && self.valid(call.end) && self.live_at(call.end, *r));
                if !leftover && call.upvals.iter().all(Option::is_some) {
                    found = Some((Rc::new(call), Rc::from("")));
                    break;
                }
            }
        }
        self.inlined.borrow_mut()[pc as usize] = Some(found.clone());
        found
    }

    /// What the closure of `child` captures, if this function makes it.
    fn captures_of(&self, child: usize) -> Option<Vec<Up>> {
        let code = &self.p.code;
        for (i, ins) in code.iter().enumerate() {
            let made = match ins.op {
                op::NEWCLOSURE => {
                    let d = ins.d();
                    let k = if d < 0 { self.p.children.len() as i32 + d } else { d };
                    self.p.children.get(k as usize).copied()
                }
                op::DUPCLOSURE => match &self.p.k[kindex(self.p, ins.d() as i64)] {
                    K::Closure(c) => Some(*c),
                    _ => None,
                },
                _ => None,
            };
            if made == Some(child) {
                let caps = code[i + 1..].iter().take_while(|c| c.op == op::CAPTURE);
                return Some(caps.map(|c| if c.a() == 2 { Up::Up(c.b()) } else { Up::Reg(c.b()) }).collect());
            }
        }
        None
    }

    /// A named function in r still waiting to be written goes out now, under
    /// its name, for what reads r to read that: moved on or passed, it was a
    /// local in the source (a function written in a call has no name).
    fn declare_named(&mut self, out: &mut Vec<String>, ind: usize, r: usize) {
        let named = self.pending.get(&r).is_some_and(|e| {
            let x = e.borrow();
            x.closure.is_some() && x.hint.as_deref().is_some_and(|h| !h.is_empty())
        });
        if named {
            let e = self.get(r);
            self.assign(out, ind, r, &e);
        }
    }

    /// A call is about to write out the waiting values not in `keep`. A value
    /// with effects made before one of those (a lower register) goes too:
    /// kept, it would be written after it. Constructors still being built stay.
    fn keep_older_effects(&self, keep: &mut Vec<usize>) {
        let effect = |r: &usize| {
            let x = self.pending[r].borrow();
            match &x.fields {
                None => !x.pure_,
                Some(f) => f.iter().any(|t| t.contains('(')), // built with a call already
            }
        };
        let lowest = self.pending.keys().filter(|r| !keep.contains(r) && effect(r)).min().copied();
        if let Some(low) = lowest {
            keep.retain(|r| !effect(r) || *r > low);
        }
    }

    /// The first read of r in [from, to) that a variable reaches: one in a
    /// branch the compiler folded away has none.
    fn known_read(&self, from: i32, to: i32, r: usize) -> Option<i32> {
        let code = &self.p.code;
        let start = code.partition_point(|i| i.pc < from);
        code[start..].iter().take_while(|i| i.pc < to).map(|i| i.pc).find(|pc| self.flow.use_web(*pc, r).is_some())
    }

    /// Put the call back where a helper's code was copied in. The pc after it.
    fn helper_call(&mut self, out: &mut Vec<String>, ind: usize, pc: i32) -> Option<i32> {
        let (call, name) = self.inline_at(pc)?;
        // the call writes these through its closure: what waits in them goes first
        for r in &call.upval_writes {
            if self.pending.contains_key(r) {
                let e = self.get(*r);
                self.assign(out, ind, *r, &e);
            }
        }
        // a register passed as more than one argument is read once for each:
        // what waits in it is a local first
        let regs: Vec<usize> = call.args.iter().flatten().filter_map(|a| if let Arg::Reg(r, _) = a { Some(*r) } else { None }).collect();
        for (k, r) in regs.iter().enumerate() {
            if regs[..k].contains(r) && self.pending.contains_key(r) {
                let e = self.get(*r);
                self.assign(out, ind, *r, &e);
            }
        }
        let mut args: Vec<String> = Vec::with_capacity(call.args.len());
        for arg in &call.args {
            args.push(match arg {
                Some(Arg::Reg(r, read)) => {
                    self.declare_named(out, ind, *r);
                    let at = if self.flow.use_web(*read, *r).is_some() { *read } else { self.known_read(pc, call.end, *r).unwrap_or(*read) };
                    let old = self.key_pc.replace(at);
                    let v = self.get(*r);
                    self.key_pc = old;
                    let text = v.borrow().text.clone();
                    text
                }
                Some(Arg::Const(text)) => text.clone(),
                None => "nil".to_string(),
            });
        }
        while args.last().is_some_and(|a| a == "nil")
            && (call.args[args.len() - 1].is_none() || call.omitted.get(args.len() - 1).copied().unwrap_or(false))
        {
            args.pop();
        }
        let dropped = self.ctx.defs.values().flatten().any(|d| d.1 == call.helper);
        let mut caps: Vec<Rc<str>> = Vec::with_capacity(call.upvals.len());
        if name.is_empty() || dropped {
            for up in &call.upvals {
                caps.push(match (*up)? {
                    Up::Up(b) => Rc::from(self.upval(b)),
                    Up::Reg(r) => {
                        if self.pending.contains_key(&r) {
                            let e = self.get(r);
                            self.assign(out, ind, r, &e);
                        }
                        // the variable the copy reads, where it first reads it:
                        // before the copy the register may hold another
                        let at = self.known_read(pc, call.end, r).or(self.key_pc);
                        let old = std::mem::replace(&mut self.key_pc, at);
                        let name = self.name_for(r);
                        self.key_pc = old;
                        name
                    }
                });
            }
        }
        let callee = if name.is_empty() {
            format!("({})", self.closure_text(call.helper, caps, ind))
        } else {
            if dropped {
                self.ctx.def_upvals.borrow_mut().insert(call.helper, caps);
            }
            name.to_string()
        };
        let mut keep: Vec<usize> = self
            .pending
            .iter()
            .filter(|(r, e)| e.borrow().pure_ || self.flows_from(call.end, call.result, **r))
            .map(|(r, _)| *r)
            .collect();
        self.keep_older_effects(&mut keep);
        self.flush(out, ind, &keep);
        // that may have just given the helper's closure its name
        let current = self.ctx.helpers.borrow().iter().find(|h| h.0 == call.helper).map(|h| h.1.clone());
        let callee = match current {
            Some(n) if !name.is_empty() => n.to_string(),
            _ => callee,
        };
        let text = format!("{callee}({})", args.join(", "));
        let inside = self.p.code[self.flow.idx(pc)?..]
            .iter()
            .take_while(|i| i.pc < call.end)
            .filter(|i| !is_for(i.op) && i.jump_target() == Some(call.end))
            .count();
        self.consume(call.end, inside as u32);
        self.cur_pc = call.last;
        // the compiler keeps a result nothing reads: it was a plain call
        match call.result.filter(|t| (*t..*t + call.results).any(|r| self.live_at(call.end, r))) {
            Some(t) if call.results > 1 => self.results(out, ind, t, call.results, &text),
            Some(t) => {
                let e = ex(text, 100);
                // a function that always returns one value needs no parentheses
                // around its last use, and the compiler can tell: with them, the
                // code differs
                let code = &self.m.protos[call.helper].code;
                e.borrow_mut().one = !code.iter().filter(|i| i.op == op::RETURN).all(|i| i.b() == 2);
                self.value_done(t, e, pc, call.end);
                // an arm of `if c then f(1) else f(2)`: its jump to the join
                // carries the value on, read there once
                let joins = self.in_region && self.reaches_join_unread(call.last, t);
                if !joins && (self.state_printed_first(call.end, t) || !self.read_once_from(call.end, t)) {
                    let e = self.get(t);
                    self.flush_impure_for(out, ind, t, Some(&e));
                    self.assign(out, ind, t, &e);
                }
            }
            None => out.push(format!("{}{text}", sp(ind))),
        }
        Some(call.end)
    }

    /// A value is in r from the code before `from`, made by a call or by one put
    /// back for copied-in code. Is a global, upvalue or field read after it
    /// written out ahead of it, where the two first meet in one expression?
    /// The call may change what was read, so folded together the read would
    /// come first, and the value has to be a local. Both are followed through
    /// the straight code from `from`.
    fn state_printed_first(&self, from: i32, r: usize) -> bool {
        let code = &self.p.code;
        let Some(start) = self.flow.idx(from) else { return false };
        let (mut val, mut st) = (RegSet::EMPTY, RegSet::EMPTY);
        val.add(r);
        // tests inside the expression (`v and 1 or 0`): what each arm writes
        // depends on what was tested, up to where the arms join
        let mut tests: Vec<(i32, bool, bool)> = Vec::new(); // (join, on the value, on the read)
        for i in start..code.len() {
            let ins = code[i];
            let q = ins.pc;
            tests.retain(|t| t.0 > q);
            let (rd, wr, barrier) = self.effects_at(i);
            let (with_val, with_st) = (rd.intersects(&val), rd.intersects(&st));
            if with_val && with_st {
                return state_operand_first(&ins, &val, &st, i.checked_sub(1).map(|k| code[k]));
            }
            let (s, n) = ins.successors();
            let forward = s[..n].iter().all(|t| *t > q);
            if barrier || ins.op == op::RETURN || !forward {
                return false;
            }
            if n == 2 {
                let target = s[..n].iter().copied().max().unwrap_or(q);
                tests.push((target, with_val, with_st));
            } else if s[0] != q + ins.n {
                // the end of the first arm: the other one is under the test too
                if let Some(last) = tests.last_mut() {
                    last.0 = last.0.max(s[0]);
                }
            }
            let (under_val, under_st) = tests.iter().fold((false, false), |a, t| (a.0 || t.1, a.1 || t.2));
            // a field of the value itself is read after it, as written
            let reads_state = !with_val
                && matches!(
                    ins.op,
                    op::GETGLOBAL | op::GETUPVAL | op::GETTABLE | op::GETTABLEKS | op::GETTABLEN | op::LENGTH
                );
            // read is used up: its expression goes on in what this writes, if
            // anything (a store or a call statement ends it)
            let was_val = val;
            for x in rd.iter() {
                val.remove(x);
                st.remove(x);
            }
            for w in wr.iter() {
                let (v, s) = (with_val || under_val, with_st || reads_state || under_st);
                if v && (w < r || (rd.has(w) && !was_val.has(w))) {
                    return false; // into a local, below the temporaries or in place: written
                }
                if v && s && !val.has(w) && !st.has(w) {
                    // they meet in an arm: what was tested is written first
                    return under_st && !under_val;
                }
                if tests.is_empty() {
                    val.remove(w);
                    st.remove(w);
                }
                if v {
                    val.add(w);
                }
                if s {
                    st.add(w);
                }
            }
            if val.is_empty() && !tests.iter().any(|t| t.1) {
                return false; // used up, and nothing tested on it decides more
            }
        }
        false
    }

    /// `name` is declared at the top of the function.
    /// The variables declared at the top since m, with their names.
    fn hoisted_since(&self, m: &Mark) -> Vec<(usize, Rc<str>)> {
        let mut out: Vec<(usize, Rc<str>)> = Vec::new();
        for u in &self.trail[m.trail.min(self.trail.len())..] {
            let Undo::Decl(k) = u else { continue };
            let Some(name) = self.regname[*k].clone() else { continue };
            if self.declared[*k] && self.hoisted_set.contains(&name) && !out.iter().any(|(j, _)| j == k) {
                out.push((*k, name));
            }
        }
        out
    }

    /// Declared at the top under this name, again after a rollback.
    fn keep_hoisted(&mut self, k: usize, name: Rc<str>) {
        if self.regname[k].as_ref() != Some(&name) {
            self.set_name(k as u32, name.clone());
        }
        self.add_used(&name);
        self.declare(k as u32);
        if !self.hoisted_set.contains(&name) {
            self.hoist(name);
        }
    }

    fn hoist(&mut self, name: Rc<str>) {
        let first = self.hoisted_set.insert(name.clone());
        self.hoisted.push(name);
        self.trail.push(Undo::Hoist(first));
    }

    /// Does writing r change what a waiting value reads? Its variable may be
    /// a new one yet under the old name (`x = f(x)`).
    fn overwrites_waiting(&self, r: usize) -> bool {
        if self.pending.is_empty() {
            return false;
        }
        let key = self.var_key(r);
        let name = match &self.regname[key as usize] {
            Some(n) => Some(n.clone()),
            None => self.prev_key(r, key).and_then(|k| self.regname[k as usize].clone()),
        };
        name.is_some_and(|n| !self.waiting_on(r, &n).is_empty())
    }

    /// Registers whose waiting value reads `name`, which r is about to change.
    fn waiting_on(&self, r: usize, name: &str) -> Vec<usize> {
        let word = key(name.as_bytes());
        let reads = |e: &E| {
            let x = e.borrow();
            if x.constructor || x.closure.is_some() {
                return !x.method && x.names_read.contains(&word);
            }
            !x.method && contains_ident(&x.text, name)
        };
        self.pending.iter().filter(|(q, e)| **q != r && reads(e)).map(|(q, _)| *q).collect()
    }

    /// `a, b = b, a % b` keeps a's new value in a temporary while b is written,
    /// then moves it home. `waiting` holds those temporaries; the moves follow
    /// the write of r at the current instruction. The line that says all of
    /// it, when that is what these instructions are.
    fn multi_assign(&mut self, r: usize, name: &str, value: &str, waiting: &[usize]) -> Option<String> {
        let here = self.insn(self.cur_pc);
        let (_, written, _) = effects(&here, self.p.maxstack);
        if !written.has(r) {
            return None;
        }
        let mut q = self.cur_pc + here.n;
        let (mut targets, mut values) = (Vec::new(), Vec::new());
        for &temp in waiting {
            if !self.valid(q) || self.has_label(q) {
                return None;
            }
            let ins = self.insn(q);
            if ins.op != op::MOVE || ins.b() != temp || self.live_after(q, temp) {
                return None;
            }
            let old = self.key_pc.replace(q);
            let key = self.var_key(ins.a()) as usize;
            self.key_pc = old;
            if !self.declared[key] {
                return None;
            }
            targets.push(self.regname[key].clone()?);
            values.push(self.pending[&temp].borrow().render());
            q += ins.n;
        }
        for &temp in waiting {
            self.ppop(temp);
        }
        targets.push(Rc::from(name));
        values.push(value.to_string());
        self.skip_to = Some(q);
        let lhs: Vec<&str> = targets.iter().map(|n| &**n).collect();
        Some(format!("{} = {}", lhs.join(", "), values.join(", ")))
    }

    /// Does the value this call leaves reach the first instruction that reads
    /// q? Then both are parts of one expression, which says them in this order.
    fn flows_with_call(&self, pc: i32, q: usize) -> bool {
        let call = self.insn(pc);
        let (a, c) = (call.a(), call.c());
        if c == 1 {
            return false;
        }
        let mut flow = RegSet::EMPTY;
        flow.add_range(a, if c == 0 { a + 1 } else { a + c - 1 });
        let mut from = self.flow.idx(pc).expect("no instruction at pc") + 1;
        if self.in_region {
            // a call inside a branch of a value: what matters is who reads
            // that value, after the branches meet
            let Some(j) = self.region_join.and_then(|j| self.flow.idx(j)) else { return false };
            for w in 0..flow.0.len() {
                flow.0[w] |= self.region_written.0[w];
            }
            from = j;
        }
        self.flows_on(from, flow, q)
    }

    /// The same for a helper's result, from the pc after its code.
    fn flows_from(&self, end: i32, result: Option<usize>, q: usize) -> bool {
        let (Some(r), Some(mut from)) = (result, self.flow.idx(end)) else { return false };
        let mut flow = RegSet::EMPTY;
        flow.add(r);
        if self.in_region {
            // in a branch of a value: who reads that value after the join
            let Some(j) = self.region_join.and_then(|j| self.flow.idx(j)) else { return false };
            for w in 0..flow.0.len() {
                flow.0[w] |= self.region_written.0[w];
            }
            from = j;
        }
        self.flows_on(from, flow, q)
    }

    fn flows_on(&self, from: usize, mut flow: RegSet, q: usize) -> bool {
        let code = &self.p.code;
        let mut i = from;
        // a join the walk came to itself, from a test it went through
        let mut joined: Option<i32> = None;
        let building = self.pending.get(&q).is_some_and(|e| e.borrow().fields.is_some());
        while i < code.len() {
            let ins = &code[i];
            // a helper copied in after: a call, whatever branches it has
            // (`Color3.new(f(r), f(g), f(b))` with f copied in three times)
            if i != from {
                if let Some((call, _)) = self.inline_at(ins.pc) {
                    let reads = |r: usize| call.args.iter().any(|x| matches!(x, Some(Arg::Reg(a, _)) if *a == r));
                    if reads(q) || call.written.contains(&q) || call.upvals.iter().flatten().any(|u| *u == Up::Reg(q)) {
                        return false;
                    }
                    if (0..self.p.maxstack).any(|r| flow.has(r) && reads(r)) {
                        for k in call.result.into_iter().flat_map(|t| t..t + call.results) {
                            flow.add(k);
                        }
                    }
                    let Some(next) = self.flow.idx(call.end) else { return false };
                    if next <= i {
                        return false;
                    }
                    joined = Some(call.end); // the copy's own jumps land there
                    i = next;
                    continue;
                }
            }
            let entered = joined == Some(ins.pc);
            if (ins.pc != code[from].pc && self.has_label(ins.pc) && !entered) || self.is_loop_head(ins.pc) {
                return false;
            }
            let (rd, wr, barrier) = effects(ins, self.p.maxstack);
            let carries = (0..self.p.maxstack).any(|r| rd.has(r) && flow.has(r));
            // the method's call follows, with the arguments that were computed first
            let method_args = ins.op == op::NAMECALL
                && code.get(self.flow.idx(ins.pc).unwrap_or(0) + 1).is_some_and(|call| {
                    let read = effects(call, self.p.maxstack).0;
                    (0..self.p.maxstack).any(|r| read.has(r) && flow.has(r))
                });
            // a copy of a variable a call may change: as an argument, an item or
            // a returned value Luau copies the variable in order, before later
            // calls; as an operand, a key, a stored value or a receiver it
            // reads the variable where it is used, after them, so the copy
            // stays a local of its own
            let snapshot = self.pending.get(&q).is_some_and(|e| e.borrow().local_copy);
            if rd.has(q) && snapshot && !matches!(ins.op, op::CALL | op::CALLFB | op::SETLIST | op::RETURN) {
                return false;
            }
            // past a test, a value waits only for its last reader: one that
            // reads it again after would find it folded into this one (a
            // table being built is read by each of its stores)
            if self.tight && !building && rd.has(q) && joined.is_some() && !wr.has(q) && self.live_after(ins.pc, q) {
                return false;
            }
            if rd.has(q) && method_args {
                return true;
            }
            if rd.has(q) {
                let (b, c) = (ins.b(), ins.c());
                let binary = matches!(
                    ins.op,
                    op::ADD | op::SUB | op::MUL | op::DIV | op::MOD | op::POW | op::IDIV | op::AND
                        | op::OR | op::GETTABLE
                );
                return carries && !(binary && c == q && b != q); // `g() + f()` keeps its order
            }
            // `if f() then a else b`, as a constructor's item: the value its
            // arms make flows on from where they meet. Past the local limit,
            // any value a test makes on the way (an `if` expression, a
            // comparison, `a and b or c`) is read with the rest
            let region = if self.tight {
                is_cond_jump(ins.op).then(|| self.closed_region(i)).flatten()
            } else if carries && building && matches!(ins.op, op::JUMPIF | op::JUMPIFNOT) {
                self.if_else_join(i)
            } else {
                None
            };
            if let Some(join) = region {
                let to = self.flow.idx(join).unwrap_or(code.len());
                for arm in &code[i + 1..to] {
                    // (all of a closed region's jumps land inside it)
                    if !self.tight && arm.pc != ins.pc && self.has_label(arm.pc) && arm.pc != ins.jump_target().unwrap_or(-1) {
                        return false;
                    }
                    let (ard, awr, abarrier) = effects(arm, self.p.maxstack);
                    if abarrier || ard.has(q) || awr.has(q) {
                        return false;
                    }
                    for r in 0..self.p.maxstack {
                        if awr.has(r) {
                            flow.add(r);
                        }
                    }
                }
                joined = Some(join);
                i = to;
                continue;
            }
            let (s, n) = ins.successors();
            if barrier || wr.has(q) || !(n == 1 && s[0] == ins.pc + ins.n) {
                return false;
            }
            if carries {
                for r in 0..self.p.maxstack {
                    if wr.has(r) {
                        flow.add(r);
                    }
                }
            }
            i += 1;
        }
        false
    }

    /// Where the forward region the test at code index i opens ends, when
    /// every jump in it lands inside it or at its end, and none from outside
    /// lands in it or there: an `if` expression, a comparison's boolean, an
    /// `and`/`or` choice.
    fn closed_region(&self, i: usize) -> Option<i32> {
        let code = &self.p.code;
        let test = &code[i];
        let mut end = test.jump_target()?;
        if end <= test.pc {
            return None;
        }
        let entered = |pc: i32| self.first_jump.get(pc as usize).is_some_and(|f| *f < test.pc);
        let mut k = i + 1;
        while k < code.len() && code[k].pc < end {
            let c = &code[k];
            if c.op == op::RETURN || is_for(c.op) || self.is_loop_head(c.pc) || (self.has_label(c.pc) && entered(c.pc)) {
                return None;
            }
            if let Some(t) = c.jump_target() {
                if t <= test.pc {
                    return None;
                }
                end = end.max(t);
            }
            k += 1;
        }
        (self.valid(end) && !entered(end) && !self.is_loop_head(end)).then_some(end)
    }

    /// The join of `JUMPIF(NOT) c -> T; A; JUMP -> J; T: B; J:` starting at
    /// code index i, when no jump in it leaves for anywhere else.
    fn if_else_join(&self, i: usize) -> Option<i32> {
        let code = &self.p.code;
        let test = &code[i];
        let t = test.jump_target()?;
        let ti = self.flow.idx(t)?;
        if ti <= i + 1 {
            return None;
        }
        let jump = &code[ti - 1];
        if !matches!(jump.op, op::JUMP | op::JUMPX) {
            return None;
        }
        let j = jump.jump_target()?;
        let ji = self.flow.idx(j)?;
        if ji <= ti {
            return None;
        }
        let inside = |p: i32| test.pc < p && p <= j;
        let stays = code[i + 1..ji].iter().all(|c| c.jump_target().is_none_or(inside) && !matches!(c.op, op::RETURN));
        stays.then_some(j)
    }

    /// Give register r the value e. Simple values wait to be inlined.
    fn put(&mut self, out: &mut Vec<String>, ind: usize, r: usize, e: E, inline: bool) {
        if let Some(old) = self.ppop(r) {
            // the old value was never read: it was a statement in its own right
            let x = old.borrow();
            let has_call = x.fields.as_ref().is_some_and(|f| f.iter().any(|t| t.contains('(')));
            if is_call_text(&x.text) {
                out.push(format!("{}{}", sp(ind), x.text));
            } else if has_call || (x.fields.is_none() && x.text.contains('(') && !x.text.starts_with("function")) {
                // a table nothing reads, built with calls, or another value
                // with calls in it: those still happen
                drop(x);
                self.seal(&old);
                self.assign(out, ind, r, &old);
            }
        }
        let pc = self.cur_pc;
        let inline = inline && !self.overwrites_waiting(r);
        if inline && (self.read_once(pc, r) || (self.in_region && self.reaches_join_unread(pc, r))) {
            // the variable this value was computed for, at this register
            let k = self.flow.def_web(pc, r).map(|w| KW + w).unwrap_or_else(|| self.var_key(r));
            self.set_def_key(&e, r, k);
            self.pset(r, e);
        } else {
            self.flush_impure_for(out, ind, r, Some(&e));
            self.assign(out, ind, r, &e);
        }
    }

    /// A statement is about to be written: values computed before it that
    /// could have side effects (calls, field reads) go out first, so the
    /// order things happen in is kept.
    fn flush_impure(&mut self, out: &mut Vec<String>, ind: usize, r: usize) {
        self.flush_impure_for(out, ind, r, None);
    }

    /// flush_impure for the value about to be written to r, taken out of
    /// pending already: when it has effects, a constructor below it that made
    /// calls goes before it too.
    fn flush_impure_for(&mut self, out: &mut Vec<String>, ind: usize, r: usize, value: Option<&E>) {
        if self.pending.is_empty() {
            return;
        }
        let effects = value.is_some_and(|e| !e.borrow().pure_);
        let goes = |q: usize, x: &X| q != r && !x.pure_ && x.fields.is_none() && !x.method && x.text != "<self>" && !x.multret;
        let keys: Vec<usize> = self.pending.keys().copied().collect();
        for q in keys {
            let Some(v) = self.pending.get(&q) else { continue }; // written out on the way
            let go = {
                let x = v.borrow();
                goes(q, &x)
                    // a constructor that made calls already: before the values
                    // with effects made after it, this one included
                    || q != r
                        && x.fields.as_ref().is_some_and(|f| f.iter().any(|t| t.contains('(')))
                        && (effects && r > q || self.pending.range(q + 1..).any(|(p, w)| goes(*p, &w.borrow())))
            };
            if go {
                let e = self.get(q);
                self.assign(out, ind, q, &e);
            }
        }
    }

    // -- body
    /// The body opens with the helpers whose code the compiler copied in here,
    /// by line info: a later pass rebuilds their calls here and nowhere else.
    fn decompile(&mut self) -> (Vec<String>, Vec<String>) {
        let (params, mut body) = self.decompile_body();
        let names = self.copies_here();
        if !names.is_empty() {
            let base = (self.depth + 1) as usize * 4;
            body.insert(0, format!("{}--@copies {}", sp(base), names.join(" ")));
        }
        (params, body)
    }

    /// Named functions with a line of this function's code in their span, but
    /// for itself and those around it: those hold the line it is defined on.
    /// (A function that is all a copy has only its helper's lines.)
    fn copies_here(&self) -> Vec<&'m str> {
        // not the closures and captures that make a function where it is
        // defined, on its first line: those are no copy of it
        let p = self.p;
        let made = |op: u8| matches!(op, op::NEWCLOSURE | op::DUPCLOSURE | op::CAPTURE);
        let mut lines: Vec<u32> = p.code.iter().filter(|c| !made(c.op)).filter_map(|c| p.lines.get(c.pc as usize).copied()).collect();
        lines.sort_unstable();
        lines.dedup();
        let def = self.p.linedefined;
        let mut names: Vec<&'m str> = Vec::new();
        for (h, proto) in self.m.protos.iter().enumerate() {
            let Some(name) = proto.name.as_deref().filter(|n| !n.is_empty()) else { continue };
            let (from, to) = self.ctx.spans[h];
            if h == self.idx || (from <= def && def <= to) || names.contains(&name) {
                continue;
            }
            let k = lines.partition_point(|l| *l < from);
            if lines.get(k).is_some_and(|l| *l <= to) {
                names.push(name);
            }
        }
        names
    }

    /// copies_here for this function and the functions inside it, whose
    /// text is part of this one's: a later pass puts the helpers' calls back
    /// in all of it, so they stay in scope.
    fn copies_within(&self) -> Vec<&'m str> {
        let protos = &self.m.protos;
        let subtree = |root: usize| {
            let mut all: Vec<usize> = Vec::new();
            let mut stack = vec![root];
            while let Some(i) = stack.pop() {
                all.push(i);
                stack.extend(protos[i].children.iter().copied());
            }
            all
        };
        let inside = subtree(self.idx);
        // per function here, the lines of code that could be a copy: not the
        // closures and captures that make a function where it is defined
        let made = |op: u8| matches!(op, op::NEWCLOSURE | op::DUPCLOSURE | op::CAPTURE);
        let lines: Vec<(usize, Vec<u32>)> = inside
            .iter()
            .map(|&i| {
                let q = &protos[i];
                let mut l: Vec<u32> = q.code.iter().filter(|c| !made(c.op)).filter_map(|c| q.lines.get(c.pc as usize).copied()).collect();
                l.sort_unstable();
                l.dedup();
                (i, l)
            })
            .collect();
        let def = self.p.linedefined;
        let mut names: Vec<&'m str> = Vec::new();
        for (h, proto) in protos.iter().enumerate() {
            let Some(name) = proto.name.as_deref().filter(|n| !n.is_empty()) else { continue };
            let (from, to) = self.ctx.spans[h];
            if h == self.idx || (from <= def && def <= to) || names.contains(&name) {
                continue;
            }
            // its own body is on its lines too
            let own = if inside.contains(&h) { subtree(h) } else { Vec::new() };
            let copied = lines.iter().filter(|(i, _)| !own.contains(i)).any(|(_, l)| {
                let k = l.partition_point(|x| *x < from);
                l.get(k).is_some_and(|x| *x <= to)
            });
            if copied {
                names.push(name);
            }
        }
        names
    }

    /// A function literal with no upvalues is one closure wherever it is
    /// loaded (DUPCLOSURE reuses it): loaded at several places (a loop the
    /// compiler unrolled, a helper copied in twice), printed at each it would
    /// be several. Declared once at the top instead, under a name; the lines.
    /// Not one calling a helper by name, which may be declared later.
    fn share_closures(&mut self, base: usize) -> Vec<(Rc<str>, String)> {
        let p = self.p;
        // constant -> (loads, child); a load counts if it captures nothing but
        // itself (a recursive local function)
        let mut sites: HashMap<usize, (usize, usize)> = HashMap::default();
        let mut mixed: HashSet<usize> = HashSet::default();
        for (i, ins) in p.code.iter().enumerate() {
            if ins.op != op::DUPCLOSURE {
                continue;
            }
            let k = kindex(p, ins.d() as i64);
            let K::Closure(c) = &p.k[k] else { continue };
            let caps = p.code[i + 1..].iter().take_while(|c| c.op == op::CAPTURE);
            let own = caps.clone().all(|c| c.a() != 2 && c.b() == ins.a());
            if own && caps.count() == self.m.protos[*c].nups {
                sites.entry(k).or_insert((0, *c)).0 += 1;
            } else {
                mixed.insert(k);
            }
        }
        let mut keys: Vec<usize> = sites.iter().filter(|(k, v)| v.0 > 1 && !mixed.contains(*k)).map(|(k, _)| *k).collect();
        keys.sort_unstable();
        let mut lines = Vec::new();
        for k in keys {
            let child = sites[&k].1;
            let hint = self.m.protos[child].name.clone().filter(|n| is_ident(n) && !is_keyword(n));
            let stem = hint.unwrap_or_else(|| "shared".to_string());
            let mut name = stem.clone();
            let mut n = 1;
            while self.is_used(&name) {
                n += 1;
                name = format!("{stem}{n}");
            }
            let name: Rc<str> = Rc::from(name.as_str());
            let caps = vec![name.clone(); self.m.protos[child].nups];
            let text = self.closure_text(child, caps, base);
            let helpers = self.ctx.helpers.borrow().iter().map(|h| h.1.clone()).collect::<Vec<_>>();
            if helpers.iter().any(|h| !h.is_empty() && **h != *name && contains_ident(&text, h)) {
                continue;
            }
            self.add_used(&name);
            let body = text.strip_prefix("function").unwrap_or(&text);
            lines.push((name.clone(), format!("{}local function {name}{body}", sp(base))));
            self.shared_closures.insert(k, name);
        }
        lines
    }

    fn decompile_body(&mut self) -> (Vec<String>, Vec<String>) {
        let (params, body) = self.decompile_fitted();
        let given = self.p.numparams as usize;
        let over = |b: &[String]| given + locals_in_scope(b) > LOCAL_LIMIT || register_pressure(b, given) > REG_LIMIT;
        if self.tight || !over(&body) {
            return (params, body);
        }
        // Still past the limit: a value computed before a test waits for its
        // call or table past it, instead of taking a local of its own. Kept
        // only if it needs fewer.
        let flow = std::mem::take(&mut self.flow);
        self.shed();
        let mut again = Dec::with_flow(self.ctx, self.idx, self.upvals.clone(), self.depth, Some(flow));
        again.tight = true;
        let (tparams, tbody) = again.decompile_fitted();
        let fewer = |a: &[String], b: &[String]| {
            let (la, lb) = (locals_in_scope(a), locals_in_scope(b));
            let (ra, rb) = (register_pressure(a, given), register_pressure(b, given));
            (la < lb && ra <= rb) || (ra < rb && la <= lb)
        };
        let (params, body) = if fewer(&tbody, &body) { (tparams, tbody) } else { (params, body) };
        if !over(&body) {
            return (params, body);
        }
        // still past it: the stretches inside the blocks scope_nested made
        // end in blocks of their own as well
        let lines: Vec<String> = body.iter().flat_map(|c| c.split('\n').map(String::from)).collect();
        let base = lines.iter().find(|l| !l.trim().is_empty()).map_or(0, |l| passes::indent_of(l));
        let deep = scope_nested_deep(&lines, base, &self.copies_within());
        if fewer(&deep, &lines) {
            (params, deep)
        } else {
            (params, body)
        }
    }

    /// The body, in fewer locals than Luau's limit if it can be.
    fn decompile_fitted(&mut self) -> (Vec<String>, Vec<String>) {
        let (params, body) = self.decompile_once();
        if !self.flat_regs {
            let peak = peak_locals(&body);
            if peak + self.p.numparams as usize > 180 {
                // the source freed its locals at the end of `do` blocks, which
                // the bytecode does not show: a stretch whose locals nothing
                // after it mentions is a block of its own again
                let lines: Vec<String> = body.iter().flat_map(|c| c.split('\n').map(String::from)).collect();
                let base = lines.iter().find(|l| !l.trim().is_empty()).map_or(0, |l| passes::indent_of(l));
                let scoped = scope_nested(&lines, base, &self.copies_within());
                let (before, after) = (peak_lines(&lines), peak_lines(&scoped));
                if after < before && (peak_locals(&scoped) <= 180 || peak <= 180) {
                    return (params, scoped);
                }
            }
            if peak > 180 {
                // Luau allows 200 locals in scope: one variable per register
                // (shared closure variables kept apart) stays under that
                // the same function: its analysis carries over, and only one
                // is held at a time
                let flow = std::mem::take(&mut self.flow);
                // nothing of this attempt is read again: what it made goes
                // before the next one makes its own
                drop((params, body));
                self.shed();
                let mut again = Dec::with_flow(self.ctx, self.idx, self.upvals.clone(), self.depth, Some(flow));
                again.flat_regs = true;
                again.tight = self.tight;
                let (fparams, fbody) = again.decompile_once();
                self.flow = std::mem::take(&mut again.flow); // for a retry after this one
                // still past the limit: its stretches whose locals nothing after
                // mentions end in `do` blocks, as for the first attempt
                if self.p.numparams as usize + locals_in_scope(&fbody) > LOCAL_LIMIT {
                    let lines: Vec<String> = fbody.iter().flat_map(|c| c.split('\n').map(String::from)).collect();
                    let base = lines.iter().find(|l| !l.trim().is_empty()).map_or(0, |l| passes::indent_of(l));
                    let scoped = scope_nested_deep(&lines, base, &self.copies_within());
                    if locals_in_scope(&scoped) < locals_in_scope(&lines) {
                        return (fparams, scoped);
                    }
                }
                return (fparams, fbody);
            }
        }
        (params, body)
    }

    /// Let go of what printing made, per instruction and per variable.
    fn shed(&mut self) {
        self.regname = Vec::new();
        self.declared = Vec::new();
        self.used = HashSet::default();
        self.once_cache = HashMap::default();
        self.hoisted = Vec::new();
        self.hoisted_set = HashSet::default();
        self.shared_closures = HashMap::default();
        self.goto_targets = BTreeSet::new();
        self.label_pcs = BTreeSet::new();
        self.backward = Vec::new();
        self.repeat_back = Vec::new();
        self.labels = Vec::new();
        self.incoming = Vec::new();
        self.first_jump = Vec::new();
        self.consumed = Vec::new();
        self.pending = BTreeMap::new();
        self.region_head = None;
        self.region_lines = None;
        self.trail = Vec::new();
        self.decl_log = Vec::new();
        self.flat_used = None;
        self.inlined = RefCell::new(Vec::new());
        self.refusals = RefCell::new(Vec::new());
        self.defs_left = Vec::new();
    }

    /// Named by register, does web w keep a name of its own? One a closure
    /// refers to does, and a helper made here whose calls come back by name:
    /// Luau puts the next local in its register once nothing reads it (every
    /// call copied in), and sharing it, the calls would reach that one.
    fn apart(&self, w: u32) -> bool {
        self.flow.byref_webs[w as usize] || self.helper_webs.get(w as usize).copied().unwrap_or(false)
    }

    /// helper_webs for register-named output.
    fn find_helper_webs(&mut self) {
        let names = self.copies_within();
        let mut webs = vec![false; self.flow.ndefs];
        let p = self.p;
        for ins in &p.code {
            let child = match ins.op {
                op::NEWCLOSURE => {
                    let d = ins.d();
                    p.children.get(if d < 0 { (p.children.len() as i32 + d) as usize } else { d as usize }).copied()
                }
                op::DUPCLOSURE => match p.k.get(kindex(p, ins.d() as i64)) {
                    Some(K::Closure(c)) => Some(*c),
                    _ => None,
                },
                _ => None,
            };
            let named = child.and_then(|c| self.m.protos[c].name.as_deref()).is_some_and(|n| names.contains(&n));
            if let Some(w) = self.flow.def_web(ins.pc, ins.a()).filter(|_| named) {
                webs[w as usize] = true;
            }
        }
        self.helper_webs = webs;
    }

    fn decompile_once(&mut self) -> (Vec<String>, Vec<String>) {
        if self.flat_regs && self.helper_webs.is_empty() {
            self.find_helper_webs();
        }
        let p = self.p;
        let mut params: Vec<String> = (0..p.numparams).map(|i| self.reg(i).to_string()).collect();
        if p.vararg != 0 {
            params.push("...".to_string());
        }
        let base = (self.depth + 1) as usize * 4;
        let shared = self.share_closures(base);
        // The first pass learns which jumps stay gotos. Few functions have any:
        // then what it printed is final, since labels are all a second pass adds.
        // what printing it again with new labels starts from; the rest is
        // cleared then (a mark here would keep every value it ever made)
        let (start_names, start_declared, start_used) = (self.regname.clone(), self.declared.clone(), self.used.clone());
        self.base_indent = base as i32;
        let mut out = Vec::new();
        self.reset_defs();
        self.emit_range(&mut out, base, 0, p.sizecode, None, None);
        for _ in 0..6 {
            // a label keeps its code from folding, which may need new labels
            if self.goto_targets.is_subset(&self.label_pcs) {
                break;
            }
            let targets = std::mem::take(&mut self.goto_targets);
            if self.watch_used {
                // the dispatcher form reserves the names known at this point
                self.flat_used = Some(self.used.clone());
                self.watch_used = false;
            }
            self.regname.clone_from(&start_names);
            self.declared.clone_from(&start_declared);
            self.used.clone_from(&start_used);
            self.pending.clear();
            self.trail.clear();
            self.decl_log.clear();
            self.loops.clear();
            self.consumed.iter_mut().for_each(|c| *c = 0);
            self.label_pcs.extend(targets);
            self.hoisted.clear();
            self.hoisted_set.clear();
            self.watch_used = true;
            out.clear();
            self.reset_defs();
            self.emit_range(&mut out, base, 0, p.sizecode, None, None);
        }
        self.fill_defs(&mut out);
        // its loads may all have been inside copies put back as calls
        for (name, line) in shared.into_iter().rev() {
            if out.iter().any(|l| contains_ident(l, &name)) {
                out.insert(0, line);
            }
        }
        // a parameter is declared already: `local x` would replace it with nil
        let mut seen: HashSet<Rc<str>> = self.param_names.iter().cloned().collect();
        let names: Vec<&str> =
            self.hoisted.iter().filter(|n| seen.insert(Rc::clone(n))).map(|n| &**n).collect();
        if !names.is_empty() {
            out.insert(0, format!("{}local {}", sp(base), names.join(", ")));
        }
        // the last pass may have needed none of the labels the first ones asked for
        if !(self.goto_targets.is_empty() && self.label_pcs.is_empty()) && !self.ctx.noflat {
            let split: Vec<String> = out.join("\n").split('\n').map(String::from).collect();
            if let Some(fixed) = passes::resolve_gotos(split) {
                if self.ctx.why {
                    self.report_flags(&fixed);
                }
                return (params, passes::wrap_mid_block_exits(fixed));
            }
            // Luau has no goto: this function's jumps did not all nest, so it
            // is written as blocks dispatched by number instead
            return (params, self.decompile_flat(base));
        }
        (params, passes::wrap_mid_block_exits(out))
    }

    fn emit_range(
        &mut self,
        out: &mut Vec<String>,
        ind: usize,
        start: i32,
        end: i32,
        loop_head: Option<i32>,
        exit: Option<i32>,
    ) {
        self.exits.push(exit.unwrap_or(end));
        self.ranges.push((start, end));
        let mark = self.decl_log.len();
        self.emit_range_body(out, ind, start, end, loop_head);
        self.ranges.pop();
        self.exits.pop();
        if ind as i32 > self.base_indent && self.base_indent >= 0 && self.decl_log.len() > mark {
            // a `local` inside this block ends with it; hoisted names live on
            let added: Vec<usize> = self.decl_log.drain(mark..).collect();
            for k in added {
                if !self.declared[k] {
                    continue;
                }
                let keep = self.regname[k].as_ref().is_some_and(|n| self.hoisted_set.contains(n));
                if keep {
                    self.decl_log.push(k);
                } else {
                    self.undeclare(k as u32);
                }
            }
        }
    }

    fn range_exit(&self) -> Option<i32> {
        self.exits.last().copied()
    }

    fn emit_range_body(&mut self, out: &mut Vec<String>, ind: usize, start: i32, end: i32, loop_head: Option<i32>) {
        let mut pc = start;
        while pc < end {
            if !self.valid(pc) {
                pc += 1;
                continue;
            }
            if self.has_label(pc)
                && pc != start
                && Self::counter(&self.consumed, pc) < Self::counter(&self.incoming, pc)
            {
                self.flush(out, ind, &[]);
            }
            if self.label_pcs.contains(&pc) {
                self.flush(out, ind, &[]);
                out.push(format!("{}::L{pc}::", sp(ind)));
            }
            while let Some(&(line, idx)) = self.defs_left.last() {
                if self.p.lines.get(pc as usize).is_none_or(|l| *l <= line) {
                    break;
                }
                self.defs_left.pop();
                out.push(format!("{}--@def {idx}", sp(ind)));
            }
            // a loop whose test starts with the call opens first; a loop in the
            // helper's own code goes with the helper
            let opens_loop = Some(pc) != loop_head
                && self.inline_at(pc).is_some_and(|(call, _)| {
                    let round = [Self::back_of(&self.backward, pc), Self::back_of(&self.repeat_back, pc)];
                    round.iter().flatten().any(|back| *back >= call.end)
                });
            if !opens_loop {
                if let Some(next) = self.helper_call(out, ind, pc) {
                    pc = next;
                    continue;
                }
            }
            let rep_back = Self::back_of(&self.repeat_back, pc);
            if let Some(rb) = rep_back {
                if rb < end && Some(pc) != loop_head && !self.is_loop_head(pc) {
                    // `repeat ... until c`: the loop's last instruction jumps
                    // back to its head while c does not hold
                    let n = self.insn(rb).n;
                    self.flush(out, ind, &[]);
                    out.push(format!("{}repeat", sp(ind)));
                    self.loops.push((rb, rb + n));
                    let mut body = Vec::new();
                    self.emit_range(&mut body, ind + 4, pc, rb, Some(pc), Some(rb));
                    let (cond, _) = self.cond_expr(rb);
                    self.flush(&mut body, ind + 4, &[]);
                    self.loops.pop();
                    out.append(&mut body);
                    out.push(format!("{}until {}", sp(ind), negate(&cond).borrow().text));
                    pc = rb + n;
                    continue;
                }
            }
            if let Some(back) = Self::back_of(&self.backward, pc) {
                if back < end && Some(pc) != loop_head {
                    // a backward jump closes a loop that starts here
                    self.flush(out, ind, &[]);
                    let at = out.len();
                    out.push(format!("{}while true do", sp(ind)));
                    self.loops.push((pc, back + 1));
                    self.emit_range(out, ind + 4, pc, back + 1, Some(pc), Some(pc));
                    self.flush(out, ind + 4, &[]);
                    self.loops.pop();
                    out.push(format!("{}end", sp(ind)));
                    self.loop_form(out, at, ind, pc, back);
                    pc = back + 1;
                    continue;
                }
            }
            pc = self.stmt(out, ind, pc, end);
        }
    }

    /// `while true do` as it was written, which only the lines tell: `while c
    /// do`, `repeat ... until c` and `while true do` with a test that breaks
    /// compile to the same code, but the jump back carries the line of the
    /// loop's own condition. So a test on that line, ahead of the body, is the
    /// `while` condition; a last test on a line after the body's first is the
    /// `until`. out[at..] is the loop as emitted, its `end` last.
    fn loop_form(&self, out: &mut Vec<String>, at: usize, ind: usize, head: i32, back: i32) {
        let line = |pc: i32| self.p.lines.get(pc as usize).copied();
        let mut first = head;
        while first < back && !self.valid(first) {
            first += 1;
        }
        let (Some(jump_line), Some(head_line)) = (line(back), line(first)) else { return };
        let (pad, inner) = (sp(ind), sp(ind + 4));
        let n = out.len();
        if n < at + 3 || out[at] != format!("{pad}while true do") || out[n - 1] != format!("{pad}end") {
            return;
        }
        // `if X then break end`, on one line or three, at the body's own level
        let exit_test = |i: usize| -> Option<(String, usize)> {
            let l = out.get(i)?;
            let rest = l.strip_prefix(inner)?.strip_prefix("if ")?;
            if l.contains('\n') {
                return None;
            }
            if let Some(t) = rest.strip_suffix(" then break end") {
                return Some((t.to_string(), 1));
            }
            let t = rest.strip_suffix(" then")?;
            let brk = format!("{inner}    break");
            let end = format!("{inner}end");
            (*out.get(i + 1)? == brk && *out.get(i + 2)? == end).then(|| (t.to_string(), 3))
        };
        // jumps left for later passes stay with the code they were made for;
        // `continue` reaches the test in a `while` either way, but in a
        // `repeat` it goes to the `until`, past what came before the test
        let has_word = |lines: &[String], word: &str| {
            lines.iter().any(|l| l.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).any(|w| w == word))
        };
        let plain = |lines: &[String]| !has_word(lines, "goto") && !lines.iter().any(|l| l.contains("::"));
        if jump_line == head_line {
            if let Some((test, used)) = exit_test(at + 1) {
                if at + 1 + used < n && plain(&out[at + 1..n - 1]) {
                    out[at] = format!("{pad}while {} do", passes::loop_condition(&test));
                    out.drain(at + 1..at + 1 + used);
                }
            }
        } else if jump_line > head_line && n >= at + 3 && exit_test(n - 2).is_some_and(|t| t.1 == 1) {
            // `repeat ... until X`, written as such only at the very end
            // (passes::final_forms): `until` sits outside the body's indent
            // but sees its locals, which the passes in between, scoping by
            // indent, would not know. Two spaces mark the loop meanwhile.
            out[at] = format!("{pad}while true  do");
        }
    }

    /// The condition under which the jump at pc is taken, and its target.
    fn cond_expr(&mut self, pc: i32) -> (E, i32) {
        let ins = self.insn(pc);
        let a = ins.a();
        let target = pc + 1 + ins.d();
        self.cur_pc = pc;
        match ins.op {
            op::JUMPIF => (self.get(a), target),
            op::JUMPIFNOT => {
                let v = self.get(a);
                (not_of(&v), target)
            }
            op::JUMPIFEQ | op::JUMPIFLE | op::JUMPIFLT | op::JUMPIFNOTEQ | op::JUMPIFNOTLE
            | op::JUMPIFNOTLT => {
                let other = (ins.aux & 0xFF) as usize;
                let waiting = self.pending.contains_key(&a) && self.pending.contains_key(&other);
                let rhs = self.get(other);
                let lhs = self.get(a);
                let sym = match ins.op {
                    op::JUMPIFEQ => "==",
                    op::JUMPIFLE | op::JUMPIFNOTLE => "<=",
                    op::JUMPIFLT | op::JUMPIFNOTLT => "<",
                    _ => "~=",
                };
                let negated = matches!(ins.op, op::JUMPIFNOTLE | op::JUMPIFNOTLT);
                // `f() > g()` compiles as g() < f() on registers filled in
                // source order: written that way the calls would swap
                if waiting && a > other {
                    return (make_cmp(&rhs, mirror(sym), &lhs, negated), target);
                }
                (make_cmp(&lhs, sym, &rhs, negated), target)
            }
            _ => {
                let neg = ins.aux >> 31 != 0;
                let k = match ins.op {
                    op::JUMPXEQKNIL => "nil".to_string(),
                    op::JUMPXEQKB => if ins.aux & 1 != 0 { "true" } else { "false" }.to_string(),
                    _ => kexpr(self.m, self.p, (ins.aux & 0xFFFFFF) as i64),
                };
                let lhs = self.get(a);
                (make_cmp(&lhs, if neg { "~=" } else { "==" }, &ex_pure(k), false), target)
            }
        }
    }

    /// Run [start, stop) as pure expression code; the value it leaves in r.
    fn value_of(&mut self, start: i32, stop: i32, r: usize, ind: usize, end: i32) -> Option<E> {
        let old = self.join_ok;
        let outer = (self.in_value, self.value_reg);
        self.join_ok = Some(stop); // the value meets the other branch's there
        self.in_value = true;
        self.value_reg = Some(r);
        let v = self.value_of_body(start, stop, r, ind, end);
        (self.in_value, self.value_reg) = outer;
        self.join_ok = old;
        v
    }

    fn value_of_body(&mut self, mut q: i32, stop: i32, r: usize, ind: usize, end: i32) -> Option<E> {
        let mut scratch = Vec::new();
        while q < stop {
            if !self.valid(q) || self.label_pcs.contains(&q) {
                return None;
            }
            if self.inline_at(q).is_some() {
                q = self.stmt(&mut scratch, ind, q, end);
                if !scratch.is_empty() {
                    return None;
                }
                continue;
            }
            // what it steps through ends with the value
            let end = end.min(stop);
            let ins = self.insn(q);
            if is_cond_jump(ins.op)
                || matches!(ins.op, op::JUMP | op::JUMPBACK | op::JUMPX | op::RETURN)
                || is_for(ins.op)
            {
                let folded = if is_cond_jump(ins.op) {
                    let nxt = q + ins.n;
                    let v = self.bool_value(q, nxt, Some(&mut scratch), ind, false);
                    v.or_else(|| {
                        self.joined_value = None;
                        let f = self.and_or_value(q, nxt, stop)?;
                        // read again after its join: a variable, which no
                        // expression declares (the value itself is the caller's)
                        match self.joined_value.take() {
                            Some(j) if (j != r || f < stop) && self.pending.contains_key(&j) && !self.read_once_from(f, j) => None,
                            _ => Some(f),
                        }
                    })
                } else {
                    None
                };
                match folded {
                    Some(f) if f <= stop && scratch.is_empty() => q = f,
                    _ => return None,
                }
                continue;
            }
            q = self.stmt(&mut scratch, ind, q, end);
            if !scratch.is_empty() {
                return None;
            }
        }
        if q != stop || !self.pending.contains_key(&r) {
            return None;
        }
        Some(self.get(r))
    }

    /// `x = a or b`: JUMPIF R -> J over code computing b into R.
    /// `x = a and b`: the same with JUMPIFNOT.
    /// `x = c and a or b`: jump c -> M; a into R; JUMP/JUMPIF R -> J; M: b into R.
    fn and_or_value(&mut self, pc: i32, nxt: i32, end: i32) -> Option<i32> {
        let ins = self.insn(pc);
        let a = ins.a();
        let target = pc + 1 + ins.d();
        if !(nxt < target && target <= end) {
            return None;
        }
        let ind = 0;
        if matches!(ins.op, op::JUMPIF | op::JUMPIFNOT) && self.pending.contains_key(&a) {
            // P1: the tested register is the result
            let s = self.mark();
            let waiting: Vec<usize> = self.pending.keys().copied().collect();
            let first = self.get(a);
            let effects = self.impure_waiting();
            if let Some(rest) = self.value_of(nxt, target, a, ind, end) {
                if !self.second_value(&waiting, target) && !self.ran_inside(&effects) {
                    let op_ = if ins.op == op::JUMPIF { Logic::Or } else { Logic::And };
                    let e = mk_logic(op_, &first, &rest);
                    self.value_done(a, e, pc, target);
                    self.consume(target, 1);
                    return Some(target);
                }
            }
            self.rollback(s);
        }
        // P2: the jump skips a branch that ends by jumping on to the join
        let last = self.last_insn_before(target)?;
        if last <= pc {
            return None;
        }
        let li = self.insn(last);
        if !matches!(li.op, op::JUMP | op::JUMPIF) {
            return None;
        }
        let join = last + 1 + li.d();
        if !(target < join && join <= end) {
            return None;
        }
        let s = self.mark();
        let (cond, _) = self.cond_expr(pc);
        let waiting: Vec<usize> = self.pending.keys().copied().collect();
        let effects = self.impure_waiting();
        let enter = negate(&cond); // the branch [nxt, last) runs when this holds
        // JUMP form: both branches compute it in one register, with their
        // temporaries above it, so it is the lowest register both write
        let reg = if li.op == op::JUMPIF {
            Some(li.a())
        } else {
            (0..self.p.maxstack).find(|r| self.written_in(nxt, last, *r) && self.written_in(target, join, *r))
        };
        let Some(reg) = reg else {
            self.rollback(s);
            return None;
        };
        // a value that can be computed with something waiting below it is an
        // argument: value_region keeps those together
        if li.op == op::JUMP {
            let below: Vec<usize> = self.pending.keys().copied().take_while(|r| *r < reg).collect();
            for r in below {
                let s2 = self.mark();
                let x = self.value_of(nxt, last, r, ind, end);
                self.rollback(s2);
                if x.is_some() {
                    self.rollback(s);
                    return None;
                }
            }
        }
        // `local v` came first: each arm assigns it, as a statement
        if self.region_def(nxt, last, reg).is_some_and(|w| self.declared[(KW + w) as usize]) {
            self.rollback(s);
            return None;
        }
        let Some(x) = self.value_of(nxt, last, reg, ind, end) else {
            self.rollback(s);
            return None;
        };
        if self.second_value(&waiting, join) {
            self.rollback(s);
            return None;
        }
        let gone: Vec<usize> =
            waiting.iter().copied().filter(|r| !self.pending.contains_key(r)).collect();
        if gone.iter().any(|r| self.read_in(target, join, *r)) {
            // both branches read a value from before them: it is a variable
            self.rollback(s);
            return None;
        }
        let Some(y) = self.value_of(target, join, reg, ind, end) else {
            self.rollback(s);
            return None;
        };
        if self.second_value(&waiting, join) || self.ran_inside(&effects) {
            self.rollback(s);
            return None;
        }
        // `c and x or y` runs y when x is false too: with a plain jump over the
        // else, only a constant that cannot be says that
        let e = if li.op == op::JUMP && !truthy_constant(&x) {
            if_expr(&enter, &x, &y)
        } else {
            mk_logic(Logic::Or, &mk_logic(Logic::And, &enter, &x), &y)
        };
        self.value_done(reg, e, pc, join);
        self.consume(join, 1);
        Some(join)
    }

    /// `a.x, b.y = 1, 2`: the stores of one assignment, starting at pc. They
    /// follow each other with nothing between, and the code right before them
    /// computed the last one's value; two statements would load a value between.
    fn store_run(&self, pc: i32) -> Vec<Insn> {
        let store = |i: &Insn| matches!(i.op, op::SETTABLEKS | op::SETTABLEN | op::SETTABLE);
        let code = &self.p.code;
        let Some(i) = self.flow.idx(pc).filter(|i| *i > 0 && store(&code[*i])) else { return Vec::new() };
        let run: Vec<Insn> = code[i..].iter().take_while(|c| store(c) && !self.has_label(c.pc)).copied().collect();
        let computed = run.last().is_some_and(|l| full_effects(&code[i - 1], self.p.maxstack).1.has(l.a()));
        // `{edge = {a = x, b = y}}`: filling the table that goes in last
        let nested = run.iter().any(|s| run.iter().any(|o| s.b() == o.a()));
        if run.len() < 2 || !computed || nested {
            return Vec::new();
        }
        run
    }

    /// Is q a store of such a run, not the last?
    fn inside_store_run(&self, q: i32) -> bool {
        let code = &self.p.code;
        let store = |i: &Insn| matches!(i.op, op::SETTABLEKS | op::SETTABLEN | op::SETTABLE);
        let Some(mut i) = self.flow.idx(q) else { return false };
        while i > 0 && store(&code[i - 1]) {
            i -= 1;
        }
        let run = self.store_run(code[i].pc);
        run.len() > 1 && run[run.len() - 1].pc != q && run.iter().any(|s| s.pc == q)
    }

    /// A table being built in r takes these as its fields.
    fn pending_constructor(&self, r: usize) -> bool {
        self.pending.get(&r).is_some_and(|e| e.borrow().fields.is_some())
    }

    /// After an unconditional jump: nothing jumps into [from, to) either?
    fn nothing_reaches(&self, from: i32, to: i32) -> bool {
        let code = &self.p.code;
        let Some(i) = self.flow.idx(from) else { return false };
        let dead = |c: &Insn| matches!(c.op, op::JUMP | op::JUMPX) && !self.has_label(c.pc);
        from < to && code[i..].iter().take_while(|c| c.pc < to).all(dead)
    }

    /// The variable r's last write in [from, to) belongs to.
    fn region_def(&self, from: i32, to: i32, r: usize) -> Option<u32> {
        let code = &self.p.code;
        let write = code[self.flow.idx(from)?..]
            .iter()
            .take_while(|i| i.pc < to)
            .filter(|i| full_effects(i, self.p.maxstack).1.has(r))
            .last()?;
        self.flow.def_web(write.pc, r)
    }

    /// The values waiting now that have effects (a call, a field read).
    fn impure_waiting(&self) -> Vec<(usize, E)> {
        self.pending.iter().filter(|(_, e)| !e.borrow().pure_).map(|(r, e)| (*r, e.clone())).collect()
    }

    /// Did a branch just walked take one of these in? Written in its arm, the
    /// value would run only when that arm does, and after the test; it ran
    /// first, either way.
    fn ran_inside(&self, effects: &[(usize, E)]) -> bool {
        effects.iter().any(|(r, e)| !self.pending.get(r).is_some_and(|now| Rc::ptr_eq(now, e)))
    }

    /// Did the code just walked leave another value someone needs at the join,
    /// or one nobody reads that still has to happen? One expression cannot
    /// say `a, b = x, y`, and an operand carries no statement: left waiting,
    /// the value would run later, whether or not its branch did.
    fn second_value(&self, waiting: &[usize], join: i32) -> bool {
        self.pending.iter().any(|(r, e)| !waiting.contains(r) && (self.live_at(join, *r) || !e.borrow().pure_))
    }

    /// Is the read at `read` on a later source line than the write at `write`?
    /// One statement rarely spreads a value over lines, and when it does a
    /// variable means the same. Lines outside this function are an inlined
    /// helper's and say nothing about its statements.
    fn later_statement(&self, write: i32, read: i32) -> bool {
        let lines = &self.p.lines;
        let (Some(a), Some(b)) = (lines.get(write as usize), lines.get(read as usize)) else {
            return false;
        };
        b > a && *a >= self.p.linedefined
    }

    /// Does something in [from, to) write r?
    fn written_in(&self, from: i32, to: i32, r: usize) -> bool {
        let code = &self.p.code;
        let start = code.partition_point(|i| i.pc < from);
        code[start..]
            .iter()
            .take_while(|i| i.pc < to)
            .any(|i| full_effects(i, self.p.maxstack).1.has(r))
    }

    /// Is r read in [from, to) before something overwrites it?
    fn read_in(&self, from: i32, to: i32, r: usize) -> bool {
        let code = &self.p.code;
        let start = code.partition_point(|i| i.pc < from);
        for ins in code[start..].iter().take_while(|i| i.pc < to) {
            let (rd, wr) = full_effects(ins, self.p.maxstack);
            if rd.has(r) {
                return true;
            }
            if wr.has(r) {
                return false;
            }
        }
        false
    }

    /// The value of [pc, join) waits in r, as the variable its last write made.
    fn value_done(&mut self, r: usize, e: E, pc: i32, join: i32) {
        let code = &self.p.code;
        let maxstack = self.p.maxstack;
        let from = self.flow.idx(pc).expect("no instruction at pc");
        let last = code[from..]
            .iter()
            .take_while(|i| i.pc < join)
            .filter(|i| full_effects(i, maxstack).1.has(r))
            .last();
        if let Some(w) = last.and_then(|i| self.flow.def_web(i.pc, r)) {
            self.set_def_key(&e, r, KW + w);
        }
        self.pset(r, e);
        self.joined_value = Some(r);
    }

    /// `local x = a == b` compiles to a jump over two LOADBs:
    ///     JUMPcond -> T ; LOADB r X skip-1 ; T: LOADB r Y
    /// so r = Y when the jump's condition holds, else X.
    fn bool_value(&mut self, pc: i32, nxt: i32, out: Option<&mut Vec<String>>, ind: usize, dry: bool) -> Option<i32> {
        let ins = self.insn(pc);
        let target = pc + 1 + ins.d();
        if target != nxt + 1 || !self.valid(nxt) || !self.valid(target) {
            return None;
        }
        let (i1, i2) = (self.insn(nxt), self.insn(target));
        if i1.op != op::LOADB || i2.op != op::LOADB {
            return None;
        }
        if i1.a() != i2.a() || i1.c() != 1 || i2.c() != 0 || (i1.b() != 0) == (i2.b() != 0) {
            return None;
        }
        if dry {
            return Some(target + 1);
        }
        let (cond, _) = self.cond_expr(pc);
        self.cur_pc = target; // the value is complete after the second LOADB
        let v = if i2.b() != 0 { cond } else { negate(&cond) };
        match out {
            Some(o) => self.put(o, ind, i1.a(), v, true),
            None => self.put(&mut Vec::new(), ind, i1.a(), v, true),
        }
        Some(target + 1)
    }

    /// True when the value written to r at pc has exactly one use: the first
    /// read on the straight-line path after pc, with r dead after it
    /// (liveness covers every branch). No other path may join in between,
    /// and no loop may start there (it would be computed on every pass).
    fn read_once(&mut self, pc: i32, r: usize) -> bool {
        if self.value_reg == Some(r) {
            // the value being built: the whole gets the variable, when what is
            // built from it reads it once (`x - x` reads a local)
            return self.read_once_straight(pc, r);
        }
        let join = self.join_ok;
        let key = (pc, r as u32, join.unwrap_or(i32::MIN));
        if let Some(v) = self.once_cache.get(&key) {
            return *v;
        }
        let mut result = false;
        if !self.shared(pc, r) {
            let code = &self.p.code;
            let mut top: Option<usize> = None; // first register past the last multi-value result
            let mut other_arm = -1; // up to here, a jumped-over path still needs r
            let made_by_call = matches!(self.insn(pc).op, op::CALL | op::CALLFB);
            let mut state_read = false; // a global, upvalue or field read on the way
            let mut i = self.flow.idx(pc).expect("no instruction at pc") + 1;
            while i < code.len() {
                let ins = code[i];
                let q = ins.pc;
                let joined = self.has_label(q) && Some(q) != join && self.first_jump[q as usize] <= pc;
                if joined || self.is_loop_head(q) {
                    break; // another path joins here
                }
                // a helper's code may read its argument often; the call, once
                if let Some((call, _)) = self.inline_at(q) {
                    let passed = call.args.iter().flatten().any(|a| matches!(a, Arg::Reg(x, _) if *x == r));
                    if passed {
                        result = !self.live_at(call.end, r);
                        break;
                    }
                    if call.written.contains(&r) || call.upvals.contains(&Some(Up::Reg(r))) {
                        break;
                    }
                    match self.flow.idx(call.end) {
                        Some(next) => {
                            i = next;
                            continue;
                        }
                        None => break,
                    }
                }
                let (rd, wr, barrier) = match top {
                    Some(top) => effects(&ins, top),
                    None => self.effects_at(i),
                };
                if rd.has(r) && twice(&ins, r) {
                    break; // `x * x`: two reads in one instruction
                }
                let qa = ins.a();
                if (ins.op == op::FORNPREP || is_forgprep(ins.op)) && qa <= r && r <= qa + 2 {
                    result = true; // a loop's start/limit/step or iterator: its header
                    break;
                }
                if rd.has(r) && q < other_arm {
                    break; // read in this arm and in the other
                }
                // a multi-line expression uses its parts at the end, and what a
                // call or a list reads directly cannot be a local: those are
                // MOVEd to the top of the stack first
                let temporary = matches!(ins.op, op::SETLIST | op::CALL | op::CALLFB | op::CONCAT)
                    || (ins.op == op::NAMECALL && qa == r && ins.b() == r);
                if rd.has(r) && !temporary && self.later_statement(pc, q) {
                    break; // the source had a variable here
                }
                if rd.has(r) && made_by_call && state_read && self.state_printed_first(pc + self.insn(pc).n, r) {
                    // the call came first: a separate statement in the source,
                    // or an argument of copied-in code that read state first
                    break;
                }
                if matches!(ins.op, op::GETGLOBAL | op::GETUPVAL | op::GETTABLE | op::GETTABLEKS | op::GETTABLEN | op::LENGTH) {
                    state_read = true;
                }
                if rd.has(r) {
                    if matches!(ins.op, op::JUMPIF | op::JUMPIFNOT) && qa == r {
                        // `x = a or b` reads a here and keeps it on the jump
                        // path; and_or_value folds that, and the plain if
                        // path writes it out first. The fall-through path
                        // must not need it, and must give r its other value:
                        // if it does not, this is a test of a variable.
                        let target = q + 1 + ins.d();
                        result = !self.live_at(q + ins.n, r)
                            && (!self.live_at(target, r) || self.written_in(q + ins.n, target, r));
                    } else {
                        result = wr.has(r) || !self.live_after(q, r);
                    }
                    break;
                }
                if (barrier && !self.inside_store_run(q)) || wr.has(r) {
                    break;
                }
                let call = matches!(ins.op, op::CALL | op::CALLFB);
                if (call && ins.c() == 0) || (ins.op == op::GETVARARGS && ins.b() == 0) {
                    top = Some(qa + 1);
                } else if call || matches!(ins.op, op::RETURN | op::SETLIST) {
                    top = None;
                }
                let (s, c) = ins.successors();
                if !(c == 1 && s[0] == q + ins.n) {
                    // a branch: fine only if no other way out still needs r
                    let ahead = |t: i32| t > q && self.first_jump.get(t as usize).is_some_and(|f| *f > pc);
                    let leaves =
                        s[..c].iter().any(|t| *t != q + ins.n && !ahead(*t) && self.live_at(*t, r));
                    for t in &s[..c] {
                        if *t != q + ins.n && self.live_at(*t, r) {
                            other_arm = other_arm.max(*t);
                        }
                    }
                    // over the else of something inside this expression
                    let local_jump = matches!(ins.op, op::JUMP | op::JUMPX | op::LOADB)
                        && s[0] > q
                        && self.first_jump.get(s[0] as usize).is_some_and(|f| *f > pc);
                    if ins.op == op::RETURN || leaves || !(local_jump || s[..c].contains(&(q + ins.n))) {
                        break;
                    }
                }
                i += 1;
            }
        }
        self.once_cache.insert(key, result);
        result
    }

    /// The straight code after pc reads r at most once before writing it (a
    /// jump or a join ends the look: as before, once).
    fn read_once_straight(&self, pc: i32, r: usize) -> bool {
        let code = &self.p.code;
        let Some(i) = self.flow.idx(pc) else { return true };
        let mut reads = 0;
        for ins in &code[i + 1..] {
            if ins.jump_target().is_some() || self.has_label(ins.pc) {
                break;
            }
            // a store into the table being built is part of building it
            let filling = matches!(ins.op, op::SETTABLEKS | op::SETTABLEN) && ins.b() == r && ins.a() != r
                || ins.op == op::SETTABLE && ins.b() == r && ins.a() != r && ins.c() != r
                || ins.op == op::SETLIST && ins.a() == r;
            let (rd, wr) = full_effects(ins, self.p.maxstack);
            if rd.has(r) && !filling {
                reads += if twice(ins, r) { 2 } else { 1 };
            }
            if wr.has(r) || reads > 1 {
                break;
            }
        }
        reads <= 1
    }

    /// A constructor whose table is used more than once is complete: it
    /// becomes the table's variable.
    fn finish_table(&mut self, out: &mut Vec<String>, ind: usize, r: usize, table: &E, pc: i32) {
        let done = table.borrow().build_end == Some(pc)
            && self.pending.get(&r).is_some_and(|e| Rc::ptr_eq(e, table));
        if done {
            self.ppop(r);
            self.seal(table);
            self.flush_impure(out, ind, r);
            self.assign(out, ind, r, table);
        }
    }

    /// (keyed stores, array stores, pc of the last one) into the new table
    /// at r before its first other use.
    fn table_fills(&self, pc: i32, r: usize) -> (bool, bool, Option<i32>) {
        let (mut keyed, mut listed, mut last) = (false, false, None);
        let code = &self.p.code;
        let mut i = self.flow.idx(pc).unwrap() + 1;
        let mut left = self.template_holes(pc);
        while i < code.len() {
            let ins = code[i];
            let (qa, qb) = (ins.a(), ins.b());
            if matches!(ins.op, op::SETTABLEKS | op::SETTABLE | op::SETTABLEN) && qb == r && qa != r {
                keyed = true;
                last = Some(ins.pc);
                left = left.saturating_sub(1);
            } else if ins.op == op::SETLIST && qa == r {
                listed = true;
                last = Some(ins.pc);
                if ins.c() == 0 {
                    break; // all of a call or `...`: the last item, nothing joins behind it
                }
            } else {
                let (rd, wr, barrier) = self.effects_at(i);
                let joined = self.has_label(ins.pc) && self.first_jump[ins.pc as usize] <= pc;
                if rd.has(r) || wr.has(r) || (left == 0 && (barrier || joined)) {
                    break;
                }
            }
            i += 1;
        }
        (keyed, listed, last)
    }

    /// The table made at pc, given only string-keyed stores up to `last`, then
    /// goes into a constructor still being built, from the register Luau made
    /// it in: it was an item written `{ ["k"] = v }` there (a variable has a
    /// register of its own, and `{ k = v }` is made from a template).
    fn item_of_constructor(&self, pc: i32, r: usize, last: Option<i32>) -> bool {
        let made = self.insn(pc);
        let code = &self.p.code;
        let (Some(last), Some(from)) = (last, self.flow.idx(pc)) else { return false };
        let Some(to) = self.flow.idx(last) else { return false };
        if made.op != op::NEWTABLE || made.aux != 0 {
            return false;
        }
        let mut count = 0;
        for ins in &code[from + 1..=to] {
            if matches!(ins.op, op::SETTABLEKS | op::SETTABLE | op::SETTABLEN) && ins.b() == r && ins.a() != r {
                if ins.op != op::SETTABLEKS {
                    return false;
                }
                count += 1;
            }
        }
        if hash_class(count) != made.b() as u8 {
            return false;
        }
        let building = |t: usize| t != r && self.pending.get(&t).is_some_and(|e| e.borrow().fields.is_some());
        for (i, ins) in code.iter().enumerate().skip(to + 1) {
            if ins.jump_target().is_some() || self.has_label(ins.pc) {
                return false;
            }
            let (rd, wr, _) = self.effects_at(i);
            if rd.has(r) {
                let (qa, qb, qc) = (ins.a(), ins.b(), ins.c());
                return match ins.op {
                    op::SETLIST => building(qa) && qb <= r && (qc == 0 || r < qb + qc - 1),
                    op::SETTABLEKS | op::SETTABLEN => qa == r && building(qb),
                    op::SETTABLE => qa == r && qc != r && building(qb),
                    _ => false,
                };
            }
            if wr.has(r) {
                return false;
            }
        }
        false
    }

    /// The registers the stores into the table made at pc read (but the table).
    fn store_operands(&self, pc: i32, r: usize) -> RegSet {
        let code = &self.p.code;
        let mut regs = RegSet::EMPTY;
        let Some(mut i) = self.flow.idx(pc).map(|k| k + 1) else { return regs };
        while i < code.len() {
            let ins = code[i];
            let (rd, wr, _) = self.effects_at(i);
            let store = matches!(ins.op, op::SETTABLEKS | op::SETTABLE | op::SETTABLEN) && ins.b() == r
                || ins.op == op::SETLIST && ins.a() == r;
            if store {
                for x in rd.iter().filter(|x| *x != r) {
                    regs.add(x);
                }
            } else if rd.has(r) || wr.has(r) {
                break;
            }
            i += 1;
        }
        regs
    }

    /// The last store after the table made at pc that still belongs in its
    /// constructor for Luau to size the table the same (pc: none does), or
    /// None if all may. A template holds exactly its constructor's keys. A
    /// NEWTABLE is sized by its constructor's items, or for `{}` by the fields
    /// stored after it; when those stores would size it otherwise, the
    /// constructor takes as many as keep the size.
    fn constructor_limit(&self, pc: i32, r: usize) -> Option<i32> {
        let p = self.p;
        let code = &p.code;
        let made = self.insn(pc);
        // the stores into r up to its first other use, as table_fills sees them
        let mut stores: Vec<Insn> = Vec::new();
        let mut flushed: Option<i32> = None; // the list's last flush so far
        let mut left = self.template_holes(pc);
        let mut i = self.flow.idx(pc)? + 1;
        while i < code.len() {
            let ins = code[i];
            if matches!(ins.op, op::SETTABLEKS | op::SETTABLE | op::SETTABLEN) && ins.b() == r && ins.a() != r {
                stores.push(ins);
                left = left.saturating_sub(1);
            } else if ins.op == op::SETLIST && ins.a() == r {
                flushed = Some(ins.pc);
            } else {
                let (rd, wr, barrier) = self.effects_at(i);
                let joined = self.has_label(ins.pc) && self.first_jump[ins.pc as usize] <= pc;
                if rd.has(r) || wr.has(r) || (left == 0 && (barrier || joined)) {
                    break;
                }
            }
            i += 1;
        }
        let name_of = |s: &Insn| kstr(self.m, p, s.aux as usize).filter(|k| is_ident(k) && !is_keyword(k));
        if let Some(last) = flushed {
            // a list: sized by its items, as it is printed. Keyed stores after
            // its last flush join while the hash part counts them
            let after: Vec<&Insn> = stores.iter().filter(|s| s.pc > last).collect();
            if after.is_empty() || made.op != op::NEWTABLE {
                return None;
            }
            let want = made.b() as u8;
            let total = after.len();
            let mut count = stores.len() - total;
            let (mut end, mut taken) = (last, 0);
            for s in after {
                if hash_class(count) == want {
                    break;
                }
                count += 1;
                taken += 1;
                end = s.pc;
            }
            // all of them its own: nothing ends it early (an end would make
            // it a variable)
            return (taken < total).then_some(end);
        }
        if made.op == op::DUPTABLE {
            let K::TableK(keys) = &p.k[kindex(p, made.d() as i64)] else { return None };
            let mut last = pc;
            for s in &stores {
                let key = match s.op {
                    op::SETTABLEKS => Some(s.aux as u64),
                    op::SETTABLE => self.flow.idx(s.pc).and_then(|i| self.settable_key(i)).map(|k| k as u64),
                    _ => None,
                };
                let open = key.is_some_and(|key| keys.iter().any(|(k, v)| *v < 0 && *k == key));
                if !open {
                    return Some(last);
                }
                last = s.pc;
            }
            return None;
        }
        let want = made.b() as u8;
        if want == 0 || stores.is_empty() {
            return None;
        }
        // `{}` then the stores: sized by the distinct `.name` fields stored
        // into that local anywhere in the function
        let web = self.flow.def_web(pc, r);
        let mut names: Vec<&str> = code
            .iter()
            .filter(|s| s.op == op::SETTABLEKS && s.a() != s.b() && web.is_some() && self.flow.use_web(s.pc, s.b()) == web)
            .filter_map(name_of)
            .collect();
        names.sort_unstable();
        names.dedup();
        if hash_class(names.len()) == want {
            return None;
        }
        // a constructor: sized by its keyed items, the [1], [2], ... in order
        // among them going to the array when there is nothing else
        let (mut records, mut general, mut index) = (0usize, 0usize, 0usize);
        let mut fits = None;
        for s in &stores {
            match s.op {
                op::SETTABLEKS if name_of(s).is_some() => records += 1,
                op::SETTABLEN if s.c() as usize == index => {
                    general += 1;
                    index += 1;
                }
                _ => general += 1,
            }
            let hash = if general == index { records } else { records + general };
            match hash_class(hash).cmp(&want) {
                std::cmp::Ordering::Equal => fits = Some(s.pc),
                std::cmp::Ordering::Greater => break,
                std::cmp::Ordering::Less => {}
            }
        }
        fits
    }

    /// `{[1] = a, [2] = b}`: a NEWTABLE with an array part and no hash part,
    /// filled by SETTABLEN inside that part before anything else uses it. The
    /// last of those stores, which the constructor takes, and whether as list
    /// items. Luau sizes `[n]` keys as an array only while they count 1, 2, 3
    /// from the first, and `{}` by the indexed stores after it, so with other
    /// keys, when `{}` and statements would not make this size either, they
    /// are list items with nil between: `{a, nil, c}` (stores a copied-in
    /// helper with constant arguments left gaps in).
    fn index_keys_end(&self, pc: i32, r: usize) -> Option<(i32, bool)> {
        let made = self.insn(pc);
        let array = made.aux as usize;
        if made.op != op::NEWTABLE || array == 0 || array > 256 || made.b() != 0 {
            return None;
        }
        let code = &self.p.code;
        let start = self.flow.idx(pc)? + 1;
        let mut i = start;
        let mut last = None;
        let mut keys: Vec<usize> = Vec::new();
        let mut low = usize::MAX; // the lowest register written since the last store
        while i < code.len() {
            let ins = code[i];
            // straight on: Luau sizes `{}` for the indexed stores after it too,
            // in branches as well, and those are statements
            if ins.jump_target().is_some() || self.has_label(ins.pc) {
                break;
            }
            if ins.op == op::SETTABLEN && ins.b() == r && ins.a() != r {
                if ins.c() as usize >= array {
                    return None; // past the array part: a statement's
                }
                // an item's value is made from its own register up; a write
                // below it was a local, and the store a statement
                if low < ins.a() {
                    break;
                }
                last = Some(ins.pc);
                keys.push(ins.c() as usize + 1);
                low = usize::MAX;
            } else {
                let (rd, wr, barrier) = self.effects_at(i);
                if let Some(w) = (0..self.p.maxstack).find(|x| wr.has(*x)) {
                    low = low.min(w);
                }
                if rd.has(r) || wr.has(r) || barrier {
                    break;
                }
            }
            i += 1;
        }
        let last = last?;
        if keys.len() == array && keys.iter().enumerate().all(|(n, k)| *k == n + 1) {
            return Some((last, false));
        }
        if !keys.windows(2).all(|w| w[0] < w[1]) {
            return None; // a key stored twice: no list says that
        }
        // as `{}` and statements, what size Luau gives it: one more for each
        // store of the next index, until the register holds something else
        let mut count = 0;
        for (k, ins) in code.iter().enumerate().skip(start) {
            if ins.op == op::SETTABLEN && ins.b() == r && ins.a() != r {
                if ins.c() as usize == count {
                    count += 1;
                }
                continue;
            }
            if ins.op == op::SETTABLE && ins.b() == r {
                return None; // `t[i]` in a loop may size it by its bound
            }
            if self.effects_at(k).1.has(r) {
                break;
            }
        }
        (count != array).then_some((last, true))
    }

    /// The string constant the key register of the SETTABLE at code index
    /// `at` holds, loaded since the last label: at -O0 Luau folds no
    /// constants, and stores a constructor's `k = v` that way.
    fn settable_key(&self, at: usize) -> Option<usize> {
        let code = &self.p.code;
        let key = code[at].c();
        for i in (0..at).rev() {
            let ins = code[i];
            if ins.jump_target().is_some() || self.has_label(code[i + 1].pc) {
                return None;
            }
            let (_, wr, _) = self.effects_at(i);
            if wr.has(key) {
                let k = match ins.op {
                    op::LOADK if ins.a() == key => ins.d() as usize,
                    op::LOADKX if ins.a() == key => ins.aux as usize,
                    _ => return None,
                };
                return matches!(self.p.k.get(k), Some(K::Str(_))).then_some(k);
            }
        }
        None
    }

    /// The SETTABLE at code index `at` stores a key the template DUPTABLE
    /// made at pc holds no value for: one of its constructor's items, at -O0.
    fn template_key_store(&self, pc: i32, at: usize) -> bool {
        let made = self.insn(pc);
        if made.op != op::DUPTABLE {
            return false;
        }
        let K::TableK(keys) = &self.p.k[kindex(self.p, made.d() as i64)] else { return false };
        self.settable_key(at).is_some_and(|key| keys.iter().any(|(k, v)| *v < 0 && *k == key as u64))
    }

    /// Keys DUPTABLE's template has no value for: the constructor's own, so
    /// what stands between it and their stores are those values, calls included.
    fn template_holes(&self, pc: i32) -> usize {
        let ins = self.insn(pc);
        if ins.op != op::DUPTABLE {
            return 0;
        }
        match &self.p.k[kindex(self.p, ins.d() as i64)] {
            K::TableK(v) => v.iter().filter(|(_, val)| *val < 0).count(),
            _ => 0,
        }
    }

    /// A table set up field by field, then used once (and dead after that
    /// use, on every path): still one value, its constructor.
    fn table_filled_then_read(&self, pc: i32, r: usize) -> bool {
        if self.shared(pc, r) || self.constructor_limit(pc, r).is_some() {
            return false;
        }
        let code = &self.p.code;
        let mut i = self.flow.idx(pc).unwrap() + 1;
        let mut left = self.template_holes(pc);
        let mut open = false; // the list ended in all of a call or `...`
        while i < code.len() {
            let ins = code[i];
            let q = ins.pc;
            let (qa, qb) = (ins.a(), ins.b());
            let joined = self.has_label(q) && self.first_jump[q as usize] <= pc;
            if (joined && left == 0) || self.is_loop_head(q) {
                return false; // another path joins while it is being built
            }
            let keyed = matches!(ins.op, op::SETTABLEKS | op::SETTABLEN)
                || ins.op == op::SETTABLE && ins.c() != r && self.template_key_store(pc, i);
            if keyed && qb == r && qa != r {
                if open {
                    return false; // a store of its own: the constructor ended
                }
                left = left.saturating_sub(1);
                i += 1;
                continue;
            }
            if ins.op == op::SETLIST && qa == r {
                open = ins.c() == 0;
                i += 1;
                continue;
            }
            if is_forgprep(ins.op) && qa == r {
                return true; // a loop's iterator: its header
            }
            let (rd, wr, barrier) = self.effects_at(i);
            if rd.has(r) {
                return !twice(&ins, r) && (wr.has(r) || !self.live_after(q, r));
            }
            let (s, c) = ins.successors();
            let inside = s[..c].iter().all(|t| *t == q + ins.n || (*t > q && self.first_jump.get(*t as usize).is_some_and(|f| *f > pc)));
            if (barrier && left == 0) || wr.has(r) || !inside {
                return false;
            }
            i += 1;
        }
        false
    }

    /// The block starting at target, when it is short, straight-line and
    /// flows on to where the current block exits anyway.
    fn shared_tail(&self, target: i32, end: i32) -> Option<i32> {
        let exit_pc = self.range_exit()?;
        if !(end <= target && target < exit_pc) {
            return None;
        }
        let mut count = 0;
        let mut q = target;
        while q < exit_pc {
            if !self.valid(q) {
                q += 1;
                continue;
            }
            let ins = self.insn(q);
            if matches!(ins.op, op::JUMP | op::JUMPBACK | op::JUMPX | op::NEWCLOSURE | op::DUPCLOSURE)
                || is_for(ins.op)
                || is_cond_jump(ins.op)
            {
                return None;
            }
            count += 1;
            if count > 12 {
                return None;
            }
            q += ins.n;
        }
        Some(exit_pc)
    }

    fn reset_defs(&mut self) {
        self.defs_left = self.ctx.defs.get(&self.idx).cloned().unwrap_or_default();
        self.defs_left.reverse();
    }

    /// Definitions the compiler dropped, now that the calls have said what
    /// their upvalues are. Last first: one may call an earlier one.
    fn fill_defs(&mut self, out: &mut Vec<String>) {
        for i in (0..out.len()).rev() {
            let Some(s) = out[i].trim_start().strip_prefix("--@def ") else { continue };
            let idx: usize = s.parse().expect("a function number");
            let ind = passes::indent_of(&out[i]);
            let nups = self.m.protos[idx].nups;
            let known = self.ctx.def_upvals.borrow().get(&idx).cloned();
            let caps = match known {
                Some(c) => Some(c),
                None if nups == 0 => Some(Vec::new()),
                None => None,
            };
            out[i] = match caps {
                Some(c) => {
                    let name = self.m.protos[idx].name.clone().unwrap_or_default();
                    let text = self.closure_text(idx, c, ind);
                    let body = text.strip_prefix("function").unwrap_or(&text);
                    format!("{}local function {name}{body}", sp(ind))
                }
                None => String::new(), // no call showed its upvalues
            };
        }
        out.retain(|l| !l.is_empty());
    }

    /// LUAU_WHY: one line per flag this function needed, and what was refused.
    fn report_flags(&self, lines: &[String]) {
        let own = sp((self.depth + 1) as usize * 4).len();
        let mut refused: Vec<(usize, String)> = self.refusals.borrow().clone();
        refused.sort_by_key(|r| std::cmp::Reverse(r.0));
        let name = self.p.name.as_deref().unwrap_or("(anon)");
        for l in lines {
            let s = l.trim_start();
            if s.starts_with("local jump_") && l.len() - s.len() >= own {
                let mut seen = HashSet::default();
                let list: Vec<&str> =
                    refused.iter().map(|r| r.1.as_str()).filter(|r| seen.insert(*r)).take(4).collect();
                eprintln!("FLAG\t{name}\t{}\t{}", s.split(' ').nth(1).unwrap_or(""), list.join(" | "));
            }
        }
    }

    fn reset_names(&mut self) {
        self.regname.iter_mut().for_each(|n| *n = None);
        self.declared.iter_mut().for_each(|d| *d = false);
        self.trail.clear();
        self.decl_log.clear();
    }

    fn decompile_flat(&mut self, base: usize) -> Vec<String> {
        let p = self.p;
        self.reset_names();
        if let Some(u) = self.flat_used.take() {
            self.used = u;
        }
        for (i, n) in self.param_names.clone().into_iter().enumerate() {
            let key = KW + self.flow.entry[i];
            self.regname[key as usize] = Some(n);
            self.declared[key as usize] = true;
        }
        self.pending.clear();
        self.loops.clear();
        self.exits.clear();
        self.consumed.iter_mut().for_each(|c| *c = 0);
        // a value read by a helper's copy was read once while the copy was a
        // call; this form never puts one back
        self.once_cache.clear();
        self.flat = true;
        self.base_indent = -1; // no renaming or hoisting: every name is fixed
        // every variable is declared up front (blocks are siblings, so no
        // block can declare one another reads); Luau allows 200 locals
        let mut all = self.flow.all_defs(&p.code);
        let nvars = all.iter().filter(|d| d.1 >= 0).map(|d| d.2).collect::<HashSet<u32>>().len();
        self.flat_regs = self.flat_regs || nvars > 150;
        let mut regs: Vec<String> = Vec::new();
        if self.flat_regs {
            for (i, n) in self.param_names.clone().into_iter().enumerate() {
                self.regname[i] = Some(n); // parameters, named by register here too
                self.declared[i] = true;
            }
            for r in p.numparams..p.maxstack.max(1) {
                let name: Rc<str> = Rc::from(format!("v{r}{}", self.suffix));
                self.used.insert(name.clone());
                self.regname[r] = Some(name.clone());
                self.declared[r] = true;
                regs.push(name.to_string());
            }
            for root in 0..self.flow.ndefs {
                if self.flow.byref_webs[root] {
                    let rr = all.iter().find(|d| d.2 as usize == root).map(|d| d.0).unwrap();
                    let name: Rc<str> = Rc::from(format!("v{rr}x{}{}", regs.len(), self.suffix));
                    self.used.insert(name.clone());
                    let key = KW as usize + root;
                    self.regname[key] = Some(name.clone());
                    self.declared[key] = true;
                    regs.push(name.to_string());
                }
            }
        } else {
            all.sort_by_key(|d| (d.0, d.1));
            for (r, dpc, root) in all {
                let key = KW as usize + root as usize;
                if self.declared[key] || self.regname[key].is_some() {
                    continue;
                }
                let mut name = format!("v{r}{}", self.suffix);
                let mut k = 2;
                while self.is_used(&name) {
                    name = format!("v{r}x{k}{}", self.suffix);
                    k += 1;
                }
                let name: Rc<str> = Rc::from(name);
                self.used.insert(name.clone());
                self.regname[key] = Some(name.clone());
                self.declared[key] = true;
                if dpc >= 0 || self.flow.used_web[root as usize] {
                    regs.push(name.to_string());
                }
            }
        }
        let mut leaders: BTreeSet<i32> = BTreeSet::new();
        leaders.insert(0);
        for ins in &p.code {
            let (pc, n, d) = (ins.pc, ins.n, ins.d());
            if is_cond_jump(ins.op)
                || matches!(ins.op, op::JUMP | op::JUMPBACK | op::FORNPREP | op::FORNLOOP | op::FORGLOOP)
                || is_forgprep(ins.op)
            {
                leaders.insert(pc + 1 + d);
                if ins.op == op::FORNPREP || is_forgprep(ins.op) {
                    leaders.insert(pc + n); // the loop body is jumped back to
                }
            } else if ins.op == op::JUMPX {
                leaders.insert(ins.jump_target().unwrap());
            } else if ins.op == op::LOADB && ins.c() != 0 {
                leaders.insert(pc + 1 + ins.c() as i32);
            }
        }
        let starts: Vec<i32> = leaders.into_iter().filter(|x| self.valid(*x)).collect();
        let mut out = Vec::new();
        let pad = sp(base);
        if !regs.is_empty() {
            out.push(format!("{pad}local {}", regs.join(", ")));
        }
        out.push(format!("{pad}local block = 0"));
        out.push(format!("{pad}while true do"));
        let (ind1, ind2) = (base + 4, base + 8);
        // Luau parses each elseif one level deeper, up to 1000 levels in all:
        // a long dispatcher is several chains in a row. A block that falls
        // through has set `block` to the next, which the next chain or the
        // next round finds.
        const CHAIN: usize = 150;
        for (i, &s) in starts.iter().enumerate() {
            let e = if i + 1 < starts.len() { starts[i + 1] } else { p.sizecode };
            if i > 0 && i % CHAIN == 0 {
                out.push(format!("{}end", sp(ind1)));
            }
            out.push(format!("{}{} block == {s} then", sp(ind1), if i % CHAIN == 0 { "if" } else { "elseif" }));
            let mut body = Vec::new();
            let mut pc = s;
            let mut ended = false;
            while pc < e && !ended {
                if !self.valid(pc) {
                    pc += 1;
                    continue;
                }
                let ins = self.insn(pc);
                let (next, done) = self.flat_stmt(&mut body, ind2, ins, e);
                pc = next;
                ended = done;
            }
            self.flush(&mut body, ind2, &[]);
            if !ended {
                if e < p.sizecode {
                    body.push(format!("{}block = {e}", sp(ind2)));
                } else {
                    body.push(format!("{}return", sp(ind2)));
                }
            }
            if body.is_empty() {
                out.push(format!("{}-- (empty)", sp(ind2)));
            } else {
                out.append(&mut body);
            }
        }
        out.push(format!("{}end", sp(ind1)));
        out.push(format!("{pad}end"));
        passes::wrap_mid_block_exits(out)
    }

    /// One instruction in dispatcher form. Returns (next pc, block ended).
    fn flat_stmt(&mut self, out: &mut Vec<String>, ind: usize, ins: Insn, end: i32) -> (i32, bool) {
        let (pc, a, b, c) = (ins.pc, ins.a(), ins.b(), ins.c());
        let mut d = ins.d();
        let nxt = pc + ins.n;
        self.cur_pc = pc;
        let pad = sp(ind);
        if is_cond_jump(ins.op) {
            if matches!(ins.op, op::JUMPIF | op::JUMPIFNOT)
                && self.pending.contains_key(&a)
                && self.live_at(pc + 1 + d, a)
            {
                let e = self.get(a);
                self.assign(out, ind, a, &e); // the target reads it too
            }
            let (cond, target) = self.cond_expr(pc);
            self.flush(out, ind, &[]);
            out.push(format!("{pad}if {} then block = {target} continue end", cond.borrow().text));
            return (nxt, false);
        }
        if matches!(ins.op, op::JUMP | op::JUMPBACK | op::JUMPX) {
            if ins.op == op::JUMPX {
                d = ins.jump_target().unwrap() - pc - 1;
            }
            self.flush(out, ind, &[]);
            out.push(format!("{pad}block = {} continue", pc + 1 + d));
            return (nxt, true);
        }
        if ins.op == op::LOADB && c != 0 {
            self.put(out, ind, a, ex_pure(if b != 0 { "true" } else { "false" }), false);
            self.flush(out, ind, &[]);
            out.push(format!("{pad}block = {} continue", pc + 1 + c as i32));
            return (nxt, true);
        }
        match ins.op {
            op::FORNPREP | op::FORNLOOP => {
                let (lim, step, idx) = (self.reg(a), self.reg(a + 1), self.reg(a + 2));
                let within = format!("({step} > 0 and {idx} <= {lim} or {step} <= 0 and {idx} >= {lim})");
                self.flush(out, ind, &[]);
                if ins.op == op::FORNPREP {
                    out.push(format!("{pad}if not {within} then block = {} continue end", pc + 1 + d));
                } else {
                    out.push(format!("{pad}{idx} = {idx} + {step}"));
                    out.push(format!("{pad}if {within} then block = {} continue end", pc + 1 + d));
                }
                return (nxt, false);
            }
            o if is_forgprep(o) => {
                self.flush(out, ind, &[]);
                let (gen, state, ctrl) = (self.reg(a), self.reg(a + 1), self.reg(a + 2));
                out.push(format!(
                    "{pad}if type({gen}) == \"table\" then {gen}, {state}, {ctrl} = next, {gen}, nil end"
                ));
                out.push(format!("{pad}block = {} continue", pc + 1 + d));
                return (nxt, true);
            }
            op::FORGLOOP => {
                self.flush(out, ind, &[]);
                let nvars = ((ins.aux & 0xFF) as usize).max(1);
                let (gen, state, ctrl) = (self.reg(a), self.reg(a + 1), self.reg(a + 2));
                let vars: Vec<Rc<str>> = (0..nvars).map(|i| self.reg(a + 3 + i)).collect();
                let list: Vec<&str> = vars.iter().map(|v| &**v).collect();
                out.push(format!("{pad}{} = {gen}({state}, {ctrl})", list.join(", ")));
                out.push(format!(
                    "{pad}if {} ~= nil then {ctrl} = {} block = {} continue end",
                    vars[0],
                    vars[0],
                    pc + 1 + d
                ));
                return (nxt, false);
            }
            op::RETURN => {
                self.stmt(out, ind, pc, end);
                if !out.last().is_some_and(|l| l.trim_start().starts_with("return")) {
                    out.push(format!("{pad}return"));
                }
                return (nxt, true);
            }
            _ => {}
        }
        (self.stmt(out, ind, pc, end), false)
    }

    /// Is the call at pc one of Luau's builtins that return one value? A
    /// FASTCALL that names it says so, and at -O2 the compiler never lets
    /// such a call return more: no parentheses are needed to keep it to one.
    fn builtin_one(&self, pc: i32) -> bool {
        let Some(at) = self.flow.idx(pc) else { return false };
        self.p.code[..at]
            .iter()
            .rev()
            .take(48)
            .find(|i| {
                matches!(i.op, op::FASTCALL | op::FASTCALL1 | op::FASTCALL2 | op::FASTCALL2K | op::FASTCALL3)
                    && i.pc + 1 + i.c() as i32 == pc
            })
            .is_some_and(|i| builtin_returns_one(i.a()))
    }

    /// effects() of instruction i, kept by the analysis or asked again.
    #[inline]
    fn effects_at(&self, i: usize) -> (RegSet, RegSet, bool) {
        if self.flow.has_effects() {
            self.flow.effects(i)
        } else {
            effects(&self.p.code[i], self.p.maxstack)
        }
    }

    fn read_once_from(&mut self, j: i32, r: usize) -> bool {
        let ins = self.insn(j);
        let (rd, wr, barrier) = effects(&ins, self.p.maxstack);
        if rd.has(r) {
            return !twice(&ins, r) && (wr.has(r) || !self.live_after(j, r));
        }
        if wr.has(r) || barrier {
            return false;
        }
        self.read_once(j, r)
    }

    /// Inside a value region: does the value written to r at pc go straight
    /// to the region's join, unread on the way?
    fn reaches_join_unread(&self, pc: i32, r: usize) -> bool {
        let Some(j) = self.region_join else { return false };
        let mut q = pc + self.insn(pc).n;
        let mut steps = 0;
        while steps < 64 {
            steps += 1;
            if q == j {
                return true;
            }
            if !self.valid(q) || q > j {
                return false;
            }
            let ins = self.insn(q);
            let (rd, wr) = full_effects(&ins, self.p.maxstack);
            if rd.has(r) || wr.has(r) {
                return false;
            }
            if matches!(ins.op, op::JUMP | op::JUMPX) {
                let t = ins.jump_target().unwrap();
                if t <= q {
                    return false;
                }
                q = t;
                continue;
            }
            if is_cond_jump(ins.op)
                || ins.op == op::RETURN
                || is_for(ins.op)
                || (ins.op == op::LOADB && ins.c() != 0)
            {
                return false;
            }
            q += ins.n;
        }
        false
    }

    // -- values computed through conditions
    /// A run of tests whose every path only computes one value into the same
    /// register, meeting at a join: one Luau if-expression
    /// (`if a then x elseif b then y else z`). Inlined validators and
    /// `a and b or c` chains of any shape compile to this.
    fn value_region(&mut self, pc: i32, end: i32) -> Option<i32> {
        if self.in_region || self.flat {
            return None;
        }
        let mut joins: Vec<i32> = Vec::new();
        let mut returns: Vec<i32> = Vec::new();
        let mut far = pc; // the furthest forward jump seen so far
        let mut q = pc;
        while q < end && joins.len() < 8 && q - pc < 300 {
            if !self.valid(q) {
                break;
            }
            let ins = self.insn(q);
            if is_for(ins.op) || self.is_loop_head(q) || self.is_repeat_head(q) {
                break;
            }
            if ins.op == op::RETURN {
                if ins.b() != 2 || q >= far {
                    if ins.b() == 2 {
                        returns.push(q);
                    }
                    break;
                }
                returns.push(q);
            }
            if let Some(t) = ins.jump_target() {
                if t > pc
                    && t <= end
                    && !joins.contains(&t)
                    && (is_cond_jump(ins.op) || matches!(ins.op, op::JUMP | op::JUMPX | op::LOADB))
                {
                    joins.push(t);
                    far = far.max(t);
                }
            }
            q += ins.n;
        }
        joins.sort_unstable();
        // `return a and b or c`: its jumps to the return were folded into
        // copies of it, so the last copy is where the paths meet
        let shared_return = match returns.as_slice() {
            [.., first, last] if self.insn(*first).a() == self.insn(*last).a() => Some(*last),
            _ => None,
        };
        joins.extend(shared_return.filter(|r| !joins.contains(r)));
        for j in joins {
            if self.label_pcs.range(pc + 1..j).next().is_some() {
                continue; // code only a goto reaches may sit between its paths
            }
            let saved = self.mark();
            let before: Vec<(usize, E)> = self.pending.iter().map(|(r, e)| (*r, e.clone())).collect();
            self.in_region = true;
            self.region_steps = 0;
            self.region_written = RegSet::EMPTY;
            self.region_join = Some(j);
            self.region_pc = pc;
            self.region_head = None;
            let got = self.walk_value(pc, j, &before);
            self.in_region = false;
            if let Ok((r, expr)) = got {
                // `local v` came first: each arm assigns it, as a statement
                let declared = self.region_def(pc, j, r).is_some_and(|w| self.declared[(KW + w) as usize]);
                if declared {
                    self.rollback(saved);
                    continue;
                }
                // a guard clause reads better as what it is, but `return x` needs
                // no MOVE: with one, the source was an expression
                let copied = self.p.code.windows(2).any(|w| {
                    (pc..=j).contains(&w[0].pc)
                        && (w[0].op == op::MOVE && w[0].a() == r)
                        && (w[1].op == op::RETURN && w[1].a() == r && w[1].b() == 2)
                });
                if self.insn(j).op == op::RETURN && expr.borrow().logic.is_none() && !copied {
                    self.rollback(saved);
                    continue;
                }
                let others = (0..self.p.maxstack).any(|x| {
                    x != r
                        && self.region_written.has(x)
                        && !before.iter().any(|b| b.0 == x)
                        && self.live_at(j, x)
                });
                if self.live_at(j, r) && !others {
                    let code = &self.p.code;
                    let from = self.flow.idx(pc).unwrap();
                    let maxstack = self.p.maxstack;
                    // the last write: earlier ones may be temporaries of a
                    // condition that happened to use this register
                    let value_def = code[from..]
                        .iter()
                        .take_while(|i| i.pc < j)
                        .filter(|i| full_effects(i, maxstack).1.has(r))
                        .last()
                        .map(|i| i.pc);
                    if let Some(dp) = value_def {
                        let k = self.flow.def_web(dp, r).map(|w| KW + w).unwrap_or_else(|| self.var_key(r));
                        self.set_def_key(&expr, r, k);
                    }
                    if self.in_value || self.read_once_from(j, r) {
                        self.pset(r, expr);
                    } else {
                        // used more than once after the join: a variable
                        let mut region_out = Vec::new();
                        self.cur_pc = value_def.unwrap_or(pc);
                        let ind = self.region_ind;
                        self.flush_impure(&mut region_out, ind, r);
                        self.assign(&mut region_out, ind, r, &expr);
                        self.region_lines = Some(region_out);
                    }
                    let inside = code[from..]
                        .iter()
                        .take_while(|i| i.pc < j)
                        .filter(|i| i.jump_target() == Some(j))
                        .count();
                    self.consume(j, inside as u32);
                    return Some(j);
                }
            }
            self.rollback(saved);
        }
        None
    }

    /// Follow the code from q to J; (register, value) when every path
    /// computes exactly one new value, in the same register.
    fn walk_value(&mut self, mut q: i32, j: i32, before: &[(usize, E)]) -> Result<(usize, E), RegionFail> {
        loop {
            self.region_steps += 1;
            if self.region_steps > 400 {
                return Err(RegionFail);
            }
            if q == j {
                // what waited before the first test may not run after it
                if let Some(head) = &self.region_head {
                    let moved = before.iter().any(|(r, e)| {
                        head.contains(r) && !e.borrow().pure_ && !self.pending.contains_key(r)
                    });
                    if moved {
                        return Err(RegionFail);
                    }
                }
                let mut new: Option<usize> = None;
                for (r, e) in &self.pending {
                    let same = before.iter().any(|b| b.0 == *r && Rc::ptr_eq(&b.1, e));
                    if !same {
                        if new.is_some() {
                            return Err(RegionFail);
                        }
                        new = Some(*r);
                    }
                }
                let r = new.ok_or(RegionFail)?;
                let e = self.get(r);
                return Ok((r, e));
            }
            if q > j || !self.valid(q) || self.is_loop_head(q) || self.is_repeat_head(q) {
                return Err(RegionFail);
            }
            if q != self.region_pc && self.label_pcs.contains(&q) {
                return Err(RegionFail); // a goto lands here: it stays a statement
            }
            if let Some((call, _)) = self.inline_at(q) {
                let mut scratch = Vec::new();
                let next = self.stmt(&mut scratch, 0, q, j);
                if !scratch.is_empty() {
                    return Err(RegionFail);
                }
                if let Some(r) = call.result {
                    self.region_written.add(r);
                }
                q = next;
                continue;
            }
            let ins = self.insn(q);
            let a = ins.a();
            if is_cond_jump(ins.op) {
                let tested = if matches!(ins.op, op::JUMPIF | op::JUMPIFNOT) {
                    self.pending.get(&a).cloned()
                } else {
                    None
                };
                // loaded before the compare: the jump keeps it as the value
                let loaded: Vec<(usize, E)> = self
                    .pending
                    .iter()
                    .filter(|(_, e)| matches!(e.borrow().text.as_str(), "true" | "false"))
                    .map(|(r, e)| (*r, e.clone()))
                    .collect();
                let (cond, t) = self.cond_expr(q);
                let f = q + ins.n;
                if t <= q || t > j {
                    return Err(RegionFail);
                }
                if self.region_head.is_none() {
                    self.region_head = Some(self.pending.keys().copied().collect());
                }
                if tested.is_some() && self.live_at(f, a) {
                    return Err(RegionFail); // the other path would read the tested value
                }
                let snap = self.mark();
                if let Some(v) = tested.as_ref().filter(|_| self.live_at(t, a)) {
                    self.pset(a, v.clone()); // the jump keeps it in its register
                }
                let (rt, vt) = self.walk_value(t, j, before)?;
                self.rollback(snap);
                let (rf, vf) = self.walk_value(f, j, before)?;
                if rt != rf {
                    return Err(RegionFail);
                }
                if let Some(v) = &tested {
                    if Rc::ptr_eq(&vt, v) {
                        let op_ = if ins.op == op::JUMPIF { Logic::Or } else { Logic::And };
                        return Ok((rt, mk_logic(op_, v, &vf)));
                    }
                    let id = v.borrow().id;
                    if vt.borrow().uses.contains(&id) || vf.borrow().uses.contains(&id) {
                        return Err(RegionFail);
                    }
                }
                // `B or Z` loads true, then skips Z when B holds; `B and Z` loads
                // false and skips it when B fails
                if is_boolean(&cond) && loaded.iter().any(|(r, e)| *r == rt && Rc::ptr_eq(e, &vt)) {
                    let v = if vt.borrow().text == "true" {
                        mk_logic(Logic::Or, &cond, &vf)
                    } else {
                        mk_logic(Logic::And, &negate(&cond), &vf)
                    };
                    let mut uses = vf.borrow().uses.clone();
                    uses.insert(vf.borrow().id);
                    v.borrow_mut().uses = uses;
                    return Ok((rt, v));
                }
                return Ok((rt, if_expr(&negate(&cond), &vf, &vt)));
            }
            if matches!(ins.op, op::JUMP | op::JUMPX) {
                let t = ins.jump_target().unwrap();
                if t <= q {
                    return Err(RegionFail);
                }
                q = t;
                continue;
            }
            if ins.op == op::LOADB && (ins.c() != 0 || self.loaded_for_compare(q)) {
                self.pset(a, ex_pure(if ins.b() != 0 { "true" } else { "false" }));
                self.region_written.add(a);
                q = q + 1 + ins.c() as i32;
                continue;
            }
            if ins.op == op::RETURN && self.valid(j) {
                let last = self.insn(j);
                if last.op == op::RETURN && (last.a(), last.b()) == (a, ins.b()) {
                    q = j; // a copy of the return the paths meet at
                    continue;
                }
            }
            if matches!(
                ins.op,
                op::RETURN | op::SETTABLEKS | op::SETTABLE | op::SETTABLEN | op::SETGLOBAL | op::SETUPVAL
                    | op::NEWCLOSURE | op::DUPCLOSURE | op::CAPTURE | op::SETLIST | op::NEWTABLE
                    | op::DUPTABLE
            ) || is_for(ins.op)
            {
                return Err(RegionFail);
            }
            let mut scratch = Vec::new();
            let wr = full_effects(&ins, self.p.maxstack).1;
            let q2 = self.stmt(&mut scratch, 0, q, j);
            if !scratch.is_empty() {
                return Err(RegionFail);
            }
            for w in 0..wr.0.len() {
                self.region_written.0[w] |= wr.0[w];
            }
            q = q2;
        }
    }

    /// The compiler's `B or Z`, with B a comparison: true goes in, and the jump
    /// that keeps it follows. A load that belongs to the pair `b = x == y`
    /// compiles to is not this: the pair stands by itself.
    fn loaded_for_compare(&self, q: i32) -> bool {
        let r = self.insn(q).a();
        let pair = |at: i32| self.valid(at) && self.insn(at).op == op::LOADB && self.insn(at).c() != 0;
        if q > 0 && pair(q - 1) {
            return false;
        }
        let mut x = q + 1;
        while self.valid(x) && x - q < 32 {
            let ins = self.insn(x);
            if is_cond_jump(ins.op) {
                return !matches!(ins.op, op::JUMPIF | op::JUMPIFNOT) && !pair(x + ins.n);
            }
            let (rd, wr) = full_effects(&ins, self.p.maxstack);
            if ins.jump_target().is_some() || ins.op == op::RETURN || rd.has(r) || wr.has(r) {
                return false;
            }
            x += ins.n;
        }
        false
    }

    /// The conditional jump ending the expression code at s, if the code up to
    /// it only computes values the condition consumes. Runs that code, so the
    /// caller rolls back; a value with jumps of its own is stepped over.
    fn cond_block(&mut self, s: i32, end: i32) -> Option<i32> {
        if self.is_loop_head(s) || self.is_repeat_head(s) || self.label_pcs.contains(&s) {
            return None; // a loop's own test, right after an `if`, or a jump's target
        }
        let mut scratch = Vec::new();
        let mut q = s;
        let mut joined = s; // where a value just stepped over came together
        while self.valid(q) {
            let ins = self.insn(q);
            if q != s && q != joined && self.has_label(q) {
                return None; // another path joins inside: not one block
            }
            if self.inline_at(q).is_some() {
                q = self.stmt(&mut scratch, 0, q, end);
                continue;
            }
            if is_cond_jump(ins.op) {
                match self.value_in_test(q, end, &mut scratch) {
                    Some(f) => {
                        q = f;
                        joined = f;
                        continue;
                    }
                    None => return Some(q),
                }
            }
            // (a LOADB that skips the next is a value made by a test; a plain
            // one, `true` as an argument, is a constant)
            if matches!(
                ins.op,
                op::JUMP | op::JUMPBACK | op::JUMPX | op::RETURN | op::NEWCLOSURE | op::DUPCLOSURE
                    | op::SETTABLEKS | op::SETTABLE | op::SETTABLEN | op::SETGLOBAL | op::SETUPVAL
                    | op::NEWTABLE | op::DUPTABLE | op::SETLIST | op::CAPTURE
            ) || (ins.op == op::LOADB && ins.c() != 0)
                || is_for(ins.op)
            {
                return None;
            }
            q = self.stmt(&mut scratch, 0, q, end);
            if !scratch.is_empty() {
                return None; // the code before a test was a statement
            }
        }
        None
    }

    /// The jump at q is part of a value inside a test, not a test of its own:
    /// where the value is complete. `a == (b == c)`, `f(x or y)`, `f(if c then x else y)`.
    fn value_in_test(&mut self, q: i32, end: i32, scratch: &mut Vec<String>) -> Option<i32> {
        let nxt = q + self.insn(q).n;
        if let Some(f) = self.bool_value(q, nxt, Some(&mut *scratch), 0, false) {
            return Some(f);
        }
        self.joined_value = None;
        if let Some(f) = self.and_or_value(q, nxt, end) {
            // read again after the test: a variable, which no test can
            // declare, so the statement ends what folds into the test
            if let Some(r) = self.joined_value.take() {
                if self.pending.contains_key(&r) && !self.read_once_from(f, r) {
                    let e = self.get(r);
                    self.flush_impure_for(scratch, 0, r, Some(&e));
                    self.assign(scratch, 0, r, &e);
                }
            }
            return Some(f);
        }
        self.region_ind = 0;
        self.region_lines = None;
        let f = self.value_region(q, end)?;
        self.region_lines.take().is_none().then_some(f)
    }

    /// Short-circuit conditions (`a and (b or not c)`) compile to a run of
    /// conditional jumps. Treat the run as a graph of tests, each with a
    /// taken and a fall-through exit, and merge a test into the one before
    /// it while they share an exit:
    ///     X(c) taken -> B, B(b) shares X's fall-through  =>  c and b
    ///     X(c) falls -> B, B(b) shares X's taken exit    =>  c or b
    /// (with `not b` when B's exits are the other way round). When the run
    /// reduces to one test whose fall-through is the code right after it,
    /// that test is the if's condition. Returns (taken condition, taken
    /// target, then-start).
    fn compound_condition(&mut self, pc: i32, end: i32) -> Option<(E, i32, i32)> {
        if self.flat {
            return None;
        }
        let mut blocks = vec![(pc, pc)]; // (start, jump pc)
        let mut q = pc + self.insn(pc).n;
        let scan = self.mark();
        self.cond_expr(pc);
        while q < end && blocks.len() < 16 {
            match self.cond_block(q, end) {
                Some(j) if j < end => {
                    self.cond_expr(j);
                    blocks.push((q, j));
                    q = j + self.insn(j).n;
                }
                _ => break,
            }
        }
        self.rollback(scan);
        self.cur_pc = pc;
        if blocks.len() < 2 {
            return None;
        }
        for count in (2..=blocks.len()).rev() {
            let saved = self.mark();
            if let Some(got) = self.reduce_tests(&blocks[..count], end) {
                return Some(got);
            }
            self.rollback(saved);
        }
        None
    }

    fn reduce_tests(&mut self, blocks: &[(i32, i32)], end: i32) -> Option<(E, i32, i32)> {
        struct Node {
            start: i32,
            c: E,
            t: i32,
            f: i32,
        }
        let mut nodes: Vec<Node> = Vec::with_capacity(blocks.len());
        for &(start, j) in blocks {
            let mut scratch = Vec::new();
            let mut q = start;
            while q < j {
                q = self.stmt(&mut scratch, 0, q, end);
            }
            if !scratch.is_empty() {
                return None; // the code before a test was a statement
            }
            let ji = self.insn(j);
            if matches!(ji.op, op::JUMPIF | op::JUMPIFNOT) && self.pending.contains_key(&ji.a()) {
                let (s, c) = ji.successors();
                if s[..c].iter().any(|t| self.live_at(*t, ji.a())) {
                    return None; // `x and v or w`: the tested value is the result
                }
            }
            let (cond, taken) = self.cond_expr(j);
            nodes.push(Node { start, c: cond, t: taken, f: self.past_dead(j + ji.n) });
        }
        let last = blocks[blocks.len() - 1].1;
        let region_end = self.past_dead(last + self.insn(last).n);
        let mut preds: HashMap<i32, i32> = HashMap::default();
        for n in &nodes {
            *preds.entry(n.t).or_insert(0) += 1;
            *preds.entry(n.f).or_insert(0) += 1;
        }
        let first = nodes[0].start;
        for n in &nodes {
            // an edge from outside the run makes a test shared: not mergeable
            let falls = i32::from(nodes.iter().any(|x| x.f == n.start));
            let inc = Self::counter(&self.incoming, n.start) as i32 + falls;
            if n.start != first && inc != preds.get(&n.start).copied().unwrap_or(0) {
                return None;
            }
        }
        let mut changed = true;
        while changed && nodes.len() > 1 {
            changed = false;
            'scan: for x in 0..nodes.len() {
                let (c, t, f) = (nodes[x].c.clone(), nodes[x].t, nodes[x].f);
                for slot_t in [true, false] {
                    let b = if slot_t { t } else { f };
                    let Some(bi) = nodes.iter().position(|n| n.start == b) else { continue };
                    if bi == x || preds.get(&b).copied().unwrap_or(0) != 1 {
                        continue;
                    }
                    let (bc, bt, bf) = (nodes[bi].c.clone(), nodes[bi].t, nodes[bi].f);
                    let new = if slot_t && bf == f {
                        Some((mk_logic(Logic::And, &c, &bc), bt, f, f))
                    } else if slot_t && bt == f {
                        Some((mk_logic(Logic::And, &c, &negate(&bc)), bf, f, f))
                    } else if !slot_t && bt == t {
                        Some((mk_logic(Logic::Or, &c, &bc), t, bf, t))
                    } else if !slot_t && bf == t {
                        Some((mk_logic(Logic::Or, &c, &negate(&bc)), t, bt, t))
                    } else {
                        None
                    };
                    let Some((expr, nt, nf, shared)) = new else { continue };
                    nodes[x].c = expr;
                    nodes[x].t = nt;
                    nodes[x].f = nf;
                    nodes.remove(bi);
                    *preds.entry(shared).or_insert(0) -= 1; // X and B both led there; now only X
                    changed = true;
                    break 'scan;
                }
            }
        }
        if nodes.len() != 1 {
            return None;
        }
        let n = &nodes[0];
        if n.f == region_end {
            return Some((n.c.clone(), n.t, region_end));
        }
        if n.t == region_end {
            return Some((negate(&n.c), n.f, region_end));
        }
        None
    }

    /// `b and true or false`: a jump over a dead one, which `or false` left
    /// (nothing jumps to it, and the jump before it does not fall in), is
    /// falling through to past both: where q's code goes on.
    fn past_dead(&self, q: i32) -> i32 {
        if !self.valid(q) || self.has_label(q) || self.insn(q).op != op::JUMP {
            return q;
        }
        let over = q + self.insn(q).n;
        if !self.valid(over) || self.has_label(over) || !matches!(self.insn(over).op, op::JUMP | op::JUMPX) {
            return q;
        }
        let beyond = over + self.insn(over).n;
        if self.insn(q).jump_target() == Some(beyond) {
            beyond
        } else {
            q
        }
    }

    /// break / continue when the target is the enclosing loop's edge.
    /// (FORNLOOP pc, pc after it) of the numeric loop prepared at pc.
    fn for_loop_end(&self, pc: i32) -> (i32, i32) {
        let prep = self.insn(pc);
        let body = pc + prep.n;
        let code = &self.p.code[self.flow.idx(pc).expect("no instruction at pc")..];
        // the compiler sends a jump that lands on a forward JUMP where that one
        // goes, so a body starting with one (`do break end`) is jumped past
        let land = |mut t: i32| {
            while self.valid(t) && self.insn(t).op == op::JUMP && self.insn(t).jump_target().is_some_and(|n| n > t) {
                t = self.insn(t).jump_target().unwrap();
            }
            t
        };
        let back = land(body);
        let close = code
            .iter()
            .find(|i| i.op == op::FORNLOOP && i.a() == prep.a() && i.jump_target().map(land) == Some(back));
        let close = close.expect("FORNPREP without its FORNLOOP");
        (close.pc, close.pc + close.n)
    }

    /// Where a jump to pc ends up: jumps to jumps go straight there.
    fn jump_end(&self, pc: i32) -> i32 {
        if self.valid(pc) {
            let ins = self.insn(pc);
            if matches!(ins.op, op::JUMP | op::JUMPX) {
                return ins.jump_target().unwrap();
            }
        }
        pc
    }

    /// Are both pcs the same return? The compiler copies a function's return
    /// over jumps to it, so one return may stand at several places.
    fn same_return(&self, a: i32, b: i32) -> bool {
        if !self.valid(a) || !self.valid(b) {
            return false;
        }
        let (x, y) = (self.insn(a), self.insn(b));
        x.op == op::RETURN && y.op == op::RETURN && (x.a(), x.b()) == (y.a(), y.b())
    }

    /// Does something in [from, to) jump to pc?
    fn jumps_to(&self, from: i32, to: i32, pc: i32) -> bool {
        let code = &self.p.code;
        let start = code.partition_point(|i| i.pc < from);
        code[start..].iter().take_while(|i| i.pc < to).any(|i| !is_for(i.op) && i.jump_target() == Some(pc))
    }

    /// Does the branch [from, to) reach the code at skipped only by not
    /// running at all, and leave for the return at r? A jump in it to skipped
    /// means an `if` that ended there: no else, just a return inside.
    fn jumps_over(&self, from: i32, to: i32, skipped: i32, r: i32) -> bool {
        let code = &self.p.code;
        let start = code.partition_point(|i| i.pc < from);
        let branch = code[start..].iter().take_while(|i| i.pc < to);
        let (mut leaves, mut over) = (false, false);
        for ins in branch {
            if let Some(t) = ins.jump_target() {
                if !is_for(ins.op) {
                    leaves |= t == skipped;
                    over |= t == r;
                }
            }
        }
        over && !leaves
    }

    /// The first jumped-to copy of the return at pc, in [from, end].
    fn return_copy(&self, pc: i32, from: i32, end: i32) -> Option<i32> {
        let code = &self.p.code;
        let start = code.partition_point(|i| i.pc < from);
        code[start..]
            .iter()
            .take_while(|i| i.pc <= end)
            .find(|i| self.has_label(i.pc) && self.same_return(i.pc, pc))
            .map(|i| i.pc)
    }

    /// Whether the rest of a loop's body after the test at pc reads as that
    /// test's own block (`if c then <rest> end`) rather than as what follows a
    /// `continue`: the two compile alike, lines included, except that a block
    /// starting on the test's own line was written there. Otherwise a short
    /// rest is a block and a long one follows a guard.
    fn short_tail(&self, pc: i32, nxt: i32, end: i32) -> bool {
        let line = |q: i32| self.p.lines.get(q as usize).copied();
        let (Some(at), Some(first)) = (line(pc), line(nxt)) else { return false };
        if first == at {
            return true;
        }
        let limit = 6;
        let mut seen: Vec<u32> = Vec::new();
        let mut q = nxt;
        while q < end {
            if self.valid(q) {
                if let Some(l) = line(q) {
                    if !seen.contains(&l) {
                        seen.push(l);
                        if seen.len() > limit {
                            return false;
                        }
                    }
                }
            }
            q += 1;
        }
        true
    }

    fn jump_word(&self, target: i32) -> Option<&'static str> {
        let (cont, brk) = *self.loops.last()?;
        if target == brk || target == self.jump_end(brk) || self.same_return(target, brk) {
            return Some("break");
        }
        if target == cont {
            return Some("continue");
        }
        None
    }

    fn last_insn_before(&self, target: i32) -> Option<i32> {
        let code = &self.p.code;
        let k = code.partition_point(|i| i.pc < target);
        if k > 0 {
            Some(code[k - 1].pc)
        } else {
            None
        }
    }

    fn reg_text(&mut self, r: usize) -> String {
        if self.pending.contains_key(&r) {
            return self.get(r).borrow().text.clone();
        }
        self.reg(r).to_string()
    }

    fn pending_regs_from(&self, first: usize) -> Option<usize> {
        self.pending.range(first..).next_back().map(|(r, _)| *r)
    }

    fn upval(&self, b: usize) -> String {
        match self.upvals.get(b) {
            Some(u) => u.to_string(),
            None => format!("upval{b}"),
        }
    }

    fn cond_stmt(&mut self, out: &mut Vec<String>, ind: usize, pc: i32, end: i32) -> i32 {
        let ins = self.insn(pc);
        let (a, d) = (ins.a(), ins.d());
        let mut nxt = pc + ins.n;
        let pad = sp(ind);
        if let Some(f) = self.bool_value(pc, nxt, Some(&mut *out), ind, false) {
            return f;
        }
        self.joined_value = None;
        if let Some(f) = self.and_or_value(pc, nxt, end) {
            // read more than once after the join: a variable
            if let Some(r) = self.joined_value.take() {
                if self.pending.contains_key(&r) && !self.read_once_from(f, r) {
                    let e = self.get(r);
                    self.flush_impure_for(out, ind, r, Some(&e));
                    self.assign(out, ind, r, &e);
                }
            }
            return f;
        }
        self.region_ind = ind;
        self.region_lines = None;
        if let Some(f) = self.value_region(pc, end) {
            if let Some(mut l) = self.region_lines.take() {
                out.append(&mut l);
            }
            return f;
        }
        if matches!(ins.op, op::JUMPIF | op::JUMPIFNOT)
            && self.pending.contains_key(&a)
            && self.live_at(pc + 1 + d, a)
        {
            // `x = a or b` that did not fold: a still flows to the target
            let e = self.get(a);
            self.assign(out, ind, a, &e);
        }
        let (mut cond, target);
        match self.compound_condition(pc, end) {
            Some((c, t, n)) => {
                cond = c;
                target = t;
                nxt = n;
            }
            None => {
                let (c, t) = self.cond_expr(pc);
                cond = c;
                target = t;
            }
        }
        if let Some(word) = self.jump_word(target) {
            if word == "continue" && nxt < end && (end == target || self.range_exit() == Some(target)) && self.short_tail(pc, nxt, end) {
                // `if c then <rest> end` closing the loop's body: the same
                // code as `if not c then continue end <rest>`
                self.flush(out, ind, &[]);
                out.push(format!("{pad}if {} then", negate(&cond).borrow().text));
                self.emit_range(out, ind + 4, nxt, end, None, Some(target));
                self.flush(out, ind + 4, &[]);
                out.push(format!("{pad}end"));
                return end;
            }
            self.flush(out, ind, &[]);
            out.push(format!("{pad}if {} then {word} end", cond.borrow().text));
            return nxt;
        }
        if nxt < target && target <= end {
            // `if a and b then`: every part jumps to the same place when it
            // fails. Pure expression code between the parts folds in.
            let mut enter = vec![negate(&cond)];
            let mut cur = nxt;
            loop {
                if self.is_loop_head(cur) || self.is_repeat_head(cur) || self.label_pcs.contains(&cur) {
                    break; // a loop's own test, right after this one, or a jump's target
                }
                // `x and false`: a constant the compiler folded, leaving a jump
                // straight to the else and a body nothing reaches
                let always = self.valid(cur) && {
                    let j = self.insn(cur);
                    matches!(j.op, op::JUMP | op::JUMPX) && j.jump_target() == Some(target)
                };
                if always {
                    enter.push(ex_pure("false"));
                    cur += self.insn(cur).n;
                    continue;
                }
                let saved = self.mark();
                let (mut q, mut scratch, mut pure_) = (cur, Vec::new(), true);
                while q < target && self.valid(q) {
                    let qi = self.insn(q);
                    if is_cond_jump(qi.op) && self.inline_at(q).is_none() {
                        match self.value_in_test(q, target, &mut scratch) {
                            Some(f) if scratch.is_empty() => {
                                q = f;
                                continue;
                            }
                            Some(_) => pure_ = false,
                            None => {}
                        }
                        break;
                    }
                    if matches!(
                        qi.op,
                        op::JUMP | op::JUMPBACK | op::JUMPX | op::RETURN | op::NEWCLOSURE | op::DUPCLOSURE
                            | op::SETTABLEKS | op::SETTABLE | op::SETTABLEN | op::SETGLOBAL
                            | op::SETUPVAL | op::NEWTABLE | op::DUPTABLE | op::SETLIST
                    ) || is_for(qi.op)
                    {
                        pure_ = false;
                        break;
                    }
                    q = self.stmt(&mut scratch, ind, q, end);
                    if !scratch.is_empty() {
                        pure_ = false;
                        break;
                    }
                }
                if pure_ && q < target && self.valid(q) && is_cond_jump(self.insn(q).op) {
                    let qi = self.insn(q);
                    let qn = q + qi.n;
                    let value_test = matches!(qi.op, op::JUMPIF | op::JUMPIFNOT)
                        && self.pending.contains_key(&qi.a())
                        && {
                            let (s, c) = qi.successors();
                            s[..c].iter().any(|t| self.live_at(*t, qi.a()))
                        };
                    if self.bool_value(q, qn, None, 0, true).is_none() && !value_test {
                        let (c2, t2) = self.cond_expr(q);
                        if t2 == target {
                            enter.push(negate(&c2));
                            cur = qn;
                            continue;
                        }
                        if qn == target && target < t2 && t2 <= end {
                            // if A and C2 then (body at t2) else (block at
                            // target, which then falls into t2): the block
                            // runs exactly when the chain fails
                            let mut every: Vec<String> = enter.iter().map(|e| wrap(e, 3)).collect();
                            every.push(wrap(&c2, 3));
                            self.flush(out, ind, &[]);
                            if self.jump_word(t2) == Some("break") {
                                // t2 leaves the loop: the chain is a break, and
                                // the block what follows it (falling out of an
                                // if would go round again; to a continue it is
                                // the same either way)
                                out.push(format!("{pad}if {} then break end", every.join(" and ")));
                                return target;
                            }
                            out.push(format!("{pad}if not ({}) then", every.join(" and ")));
                            // the block may end by jumping over an else
                            let blast = self.last_insn_before(t2);
                            let mut jt = None;
                            if let Some(bl) = blast {
                                if bl >= target {
                                    let bi = self.insn(bl);
                                    let cand = bl + 1 + bi.d();
                                    if bi.op == op::JUMP
                                        && t2 < cand
                                        && (cand <= end || Some(cand) == self.range_exit())
                                    {
                                        jt = Some(cand);
                                    }
                                }
                            }
                            let Some(jt) = jt else {
                                self.emit_range(out, ind + 4, target, t2, None, None);
                                self.flush(out, ind + 4, &[]);
                                out.push(format!("{pad}end"));
                                return t2;
                            };
                            let else_end = if jt <= end { jt } else { end };
                            self.emit_range(out, ind + 4, target, blast.unwrap(), None, Some(jt));
                            self.flush(out, ind + 4, &[]);
                            let mut body = Vec::new();
                            let ex_ = if else_end == end { self.range_exit() } else { Some(else_end) };
                            self.emit_range(&mut body, ind + 4, t2, else_end, None, ex_);
                            self.flush(&mut body, ind + 4, &[]);
                            let round = self.jump_word(else_end) == Some("continue");
                            out.extend(passes::merge_elseif(body, ind, round));
                            out.push(format!("{pad}end"));
                            return else_end;
                        }
                    }
                }
                self.rollback(saved);
                break;
            }
            if enter.len() > 1 {
                let text: Vec<String> = enter.iter().map(|e| wrap(e, 3)).collect();
                cond = negate(&ex(text.join(" and "), 2));
                nxt = cur;
            }
            // forward conditional: an if (maybe with else)
            let mut then_end = target;
            let mut else_end = None;
            let mut after = target; // where the then-branch continues
            if let Some(last) = self.last_insn_before(target) {
                if last >= nxt {
                    let li = self.insn(last);
                    let jt = last + 1 + li.d();
                    // at the end of a while body that is the way out, not
                    // the way over an else
                    let leaves_loop = jt == end && self.loops.last().is_some_and(|l| l.1 == jt);
                    // `if a then if b then x continue end end y`: the branch can
                    // reach what follows it, so that is no else
                    let early_exit = self.jump_word(jt).is_some() && self.jumps_to(nxt, last, target);
                    if li.op == op::JUMP && jt > target && jt <= end && !leaves_loop && !early_exit {
                        else_end = Some(jt);
                        then_end = last;
                        after = jt;
                    } else if li.op == op::JUMP && jt > target && Some(jt) == self.range_exit() && !early_exit {
                        // the branch leaves the enclosing block: everything
                        // after it in this block is the else
                        else_end = Some(end);
                        then_end = last;
                        after = jt;
                    } else if li.op == op::RETURN && li.b() == 1 {
                        // the jump over the else was to a bare return, so the
                        // compiler put the return here instead; jumps from
                        // inside the branch still go to the real one
                        let exit = self.range_exit().filter(|x| *x >= end && self.same_return(*x, last));
                        let copy = exit.or_else(|| self.return_copy(last, target, end));
                        if let Some(r) = copy.filter(|r| self.jumps_over(nxt, last, target, *r)) {
                            else_end = Some(if exit.is_some() { end } else { r });
                            then_end = last;
                            after = r;
                        }
                    }
                }
            }
            self.flush(out, ind, &[]);
            out.push(format!("{pad}if {} then", negate(&cond).borrow().text));
            self.emit_range(out, ind + 4, nxt, then_end, None, Some(after));
            self.flush(out, ind + 4, &[]);
            if let Some(ee) = else_end {
                let mut body = Vec::new();
                let ex_ = if ee == end { self.range_exit() } else { Some(ee) };
                self.emit_range(&mut body, ind + 4, target, ee, None, ex_);
                self.flush(&mut body, ind + 4, &[]);
                let round = self.jump_word(ee) == Some("continue");
                out.extend(passes::merge_elseif(body, ind, round));
                out.push(format!("{pad}end"));
                return ee;
            }
            out.push(format!("{pad}end"));
            return target;
        }
        if Some(target) == self.range_exit() && target >= end {
            // an inner if whose end coincides with the enclosing block's end:
            // the compiler jumps straight there, so the rest of this block
            // is the if's body
            self.flush(out, ind, &[]);
            out.push(format!("{pad}if {} then", negate(&cond).borrow().text));
            self.emit_range(out, ind + 4, nxt, end, None, Some(target));
            self.flush(out, ind + 4, &[]);
            out.push(format!("{pad}end"));
            return end;
        }
        if let Some(tail) = self.shared_tail(target, end) {
            // `if C then <shared tail> else <rest of block> end`: the tail
            // and the rest both end where this block exits
            self.flush(out, ind, &[]);
            out.push(format!("{pad}if {} then", cond.borrow().text));
            let saved = self.mark();
            self.emit_range(out, ind + 4, target, tail, None, Some(tail));
            self.flush(out, ind + 4, &[]);
            // the tail is written again after this, from the same state; but
            // the lines just written stay, and what they declared at the top
            // with them, under the same names
            let hoisted = self.hoisted_since(&saved);
            self.rollback(saved);
            for (k, name) in hoisted {
                self.keep_hoisted(k, name);
            }
            let mut body = Vec::new();
            let ex_ = self.range_exit();
            self.emit_range(&mut body, ind + 4, nxt, end, None, ex_);
            self.flush(&mut body, ind + 4, &[]);
            let round = self.jump_word(end) == Some("continue");
            out.extend(passes::merge_elseif(body, ind, round));
            out.push(format!("{pad}end"));
            return end;
        }
        self.flush(out, ind, &[]);
        self.add_goto(target);
        out.push(format!("{pad}if {} then goto L{target} end", cond.borrow().text));
        nxt
    }

    fn stmt(&mut self, out: &mut Vec<String>, ind: usize, pc: i32, end: i32) -> i32 {
        self.cur_pc = pc;
        let ins = self.insn(pc);
        let (o, a, b, c) = (ins.op, ins.a(), ins.b(), ins.c());
        let mut d = ins.d();
        let aux = ins.aux;
        let nxt = pc + ins.n;
        let (m, p) = (self.m, self.p);
        let pad = sp(ind);

        if let Some(next) = self.helper_call(out, ind, pc) {
            return next;
        }
        if o == op::LOADB && c == 0 && !self.flat && !self.in_region && self.loaded_for_compare(pc) {
            self.region_ind = ind;
            self.region_lines = None;
            if let Some(f) = self.value_region(pc, end) {
                if let Some(mut l) = self.region_lines.take() {
                    out.append(&mut l);
                }
                return f;
            }
            self.cur_pc = pc; // the attempt walked on
        }
        // ---- control flow -------------------------------------------------
        if is_cond_jump(o) && !self.flat {
            return self.cond_stmt(out, ind, pc, end);
        }
        if matches!(o, op::JUMP | op::JUMPBACK | op::JUMPX) {
            if o == op::JUMPX {
                d = ins.jump_target().unwrap() - pc - 1;
            }
            let target = pc + 1 + d;
            // `else continue end`: the jump to it went straight to the loop's end,
            // and what is left is an arm no path enters
            if self.nothing_reaches(nxt, target) {
                return target;
            }
            self.flush(out, ind, &[]);
            if let Some(l) = self.loops.last() {
                if target == l.0 && nxt == l.1 {
                    return nxt; // the loop's own closing jump
                }
            }
            if Some(target) == self.range_exit() && nxt >= end {
                return nxt; // leaves the block the way it would anyway
            }
            if let Some(word) = self.jump_word(target) {
                out.push(format!("{pad}{word}"));
            } else if target != nxt {
                self.add_goto(target);
                out.push(format!("{pad}goto L{target}"));
            }
            return nxt;
        }
        if o == op::FORNPREP {
            // not from this jump's target: when a jump follows the loop, the
            // compiler sends this one straight to that jump's destination
            let (edge, target) = self.for_loop_end(pc);
            self.flush(out, ind, &[a, a + 1, a + 2]); // the header takes those
            let var = self.name_for(a + 2);
            let key = self.var_key(a + 2);
            let was_declared = self.declared[key as usize];
            self.declare(key);
            let (s, l, st) = (self.reg_text(a + 2), self.reg_text(a), self.reg_text(a + 1));
            // a step left out is a 1 loaded all the same
            if st == "1" {
                out.push(format!("{pad}for {var} = {s}, {l} do"));
            } else {
                out.push(format!("{pad}for {var} = {s}, {l}, {st} do"));
            }
            self.loops.push((edge, target));
            self.emit_range(out, ind + 4, nxt, edge, None, None);
            self.flush(out, ind + 4, &[]);
            self.loops.pop();
            if !was_declared {
                self.cur_pc = pc;
                self.undeclare(key); // the loop variable ends with the loop
            }
            out.push(format!("{pad}end"));
            return target;
        }
        if is_forgprep(o) {
            let loop_pc = pc + 1 + d; // the FORGLOOP
            if self.valid(loop_pc) {
                let li = self.insn(loop_pc);
                let nvars = (li.aux & 0xFF) as usize;
                let gen = [self.reg_text(a), self.reg_text(a + 1), self.reg_text(a + 2)];
                let iter_call = self.iter_call.take();
                self.flush(out, ind, &[]);
                let mut vars: Vec<Rc<str>> = Vec::new();
                let mut fresh: Vec<u32> = Vec::new();
                self.key_pc = Some(loop_pc);
                for i in 0..nvars.max(1) {
                    let r = a + 3 + i;
                    vars.push(self.name_for(r));
                    let key = self.var_key(r);
                    if !self.declared[key as usize] {
                        fresh.push(key);
                    }
                    self.declare(key);
                }
                self.key_pc = None;
                // only nils at the end go; a lone call is one value, as the
                // register holds, not the three a call there would spread to
                let used = gen.iter().rposition(|g| g != "nil").map_or(1, |k| k + 1);
                let mut head = gen[..used].join(", ");
                if used == 1 && head.ends_with(')') {
                    head = format!("({head})");
                }
                let here = self.reg(a);
                if o == op::FORGPREP_INEXT && (gen[0] == "ipairs" || gen[0] == *here) {
                    head = format!("ipairs({})", gen[1]);
                } else if o == op::FORGPREP_NEXT && gen[0] == "next" {
                    head = format!("next, {}", gen[1]);
                } else if o == op::FORGPREP_NEXT && (gen[0] == "pairs" || gen[0] == *here) {
                    head = format!("pairs({})", gen[1]);
                }
                if let Some((ia, text)) = iter_call {
                    if ia == a {
                        head = text;
                    }
                }
                let list: Vec<&str> = vars.iter().map(|v| &**v).collect();
                out.push(format!("{pad}for {} in {head} do", list.join(", ")));
                self.loops.push((loop_pc, loop_pc + 2));
                self.emit_range(out, ind + 4, nxt, loop_pc, None, None);
                self.flush(out, ind + 4, &[]);
                self.loops.pop();
                for k in fresh {
                    self.undeclare(k); // loop variables end with the loop
                }
                out.push(format!("{pad}end"));
                return loop_pc + 2;
            }
        }
        if matches!(o, op::FORNLOOP | op::FORGLOOP) {
            return nxt;
        }

        // ---- plain statements ---------------------------------------------
        match o {
            op::NOP | op::BREAK | op::PREPVARARGS | op::COVERAGE | op::NATIVECALL | op::FASTCALL
            | op::FASTCALL1 | op::FASTCALL2 | op::FASTCALL2K | op::FASTCALL3 | op::FASTPCALL => {
                return nxt; // a fast path: the CALL that follows says it all
            }
            op::LOADNIL => self.put(out, ind, a, ex_pure("nil"), true),
            op::LOADB => {
                self.put(out, ind, a, ex_pure(if b != 0 { "true" } else { "false" }), true);
                if c != 0 {
                    self.flush(out, ind, &[]);
                    self.add_goto(pc + 1 + c as i32);
                    out.push(format!("{pad}goto L{}", pc + 1 + c as i32));
                }
            }
            op::LOADN => self.put(out, ind, a, ex_const(d.to_string()), true),
            op::LOADK => self.put(out, ind, a, ex_const(kexpr(m, p, d as i64)), true),
            op::LOADKX => self.put(out, ind, a, ex_const(kexpr(m, p, aux as i64)), true),
            op::MOVE => {
                self.declare_named(out, ind, b); // to where copied-in code keeps it, or an argument
                // read again after this copy (`own = slot and t[slot]`): the
                // value has to stay where it is, under its own name
                if self.pending.contains_key(&b) && !self.shared(pc, b) && self.live_after(pc, b) && b != a {
                    let e = self.get(b);
                    self.flush_impure_for(out, ind, b, Some(&e));
                    self.assign(out, ind, b, &e);
                }
                let shared = !self.pending.contains_key(&b) && self.shared(pc, b);
                let v = self.get(b);
                if shared {
                    v.borrow_mut().pure_ = false; // a call may change it
                    v.borrow_mut().snapshot = true;
                    v.borrow_mut().local_copy = true;
                }
                self.put(out, ind, a, v, true);
            }
            op::GETGLOBAL => {
                let t = match kstr(m, p, aux as usize) {
                    Some(s) if !s.is_empty() => s.to_string(),
                    _ => kexpr(m, p, aux as i64),
                };
                self.put(out, ind, a, ex(t, 100), true);
            }
            op::SETGLOBAL => {
                self.flush(out, ind, &[a]);
                let v = self.get(a);
                let name = kstr(m, p, aux as usize).unwrap_or("None");
                let rest = self.definition_rest(&v.borrow(), Some(name));
                match rest {
                    Some(rest) => out.push(format!("{pad}function {name}{rest}")),
                    None => out.push(format!("{pad}{name} = {}", v.borrow().text)),
                }
            }
            op::GETUPVAL => {
                let t = self.upval(b);
                let e = ex(t, 100);
                e.borrow_mut().snapshot = self.ctx.byref_ups.get(self.idx).is_some_and(|u| u.get(b).copied().unwrap_or(false));
                self.put(out, ind, a, e, true);
            }
            op::SETUPVAL => {
                self.flush(out, ind, &[a]);
                let v = self.get(a);
                out.push(format!("{pad}{} = {}", self.upval(b), v.borrow().text));
            }
            op::CLOSEUPVALS | op::CAPTURE => {}
            op::GETIMPORT => self.put(out, ind, a, ex_pure(import_expr(m, p, aux)), true),
            op::GETTABLE => {
                let obj = self.get(b);
                let key = self.get(c);
                let t = format!("{}[{}]", wrap(&obj, 100), key.borrow().text);
                self.put(out, ind, a, ex(t, 100), true);
            }
            op::SETTABLE => {
                let fold = a != b
                    && c != b
                    && self.pending.get(&b).is_some_and(|e| e.borrow().fields.is_some());
                if fold {
                    let table = self.pending[&b].clone();
                    // a template's key in a register: its `k = v` at -O0
                    let temp = self.pending.contains_key(&c);
                    let named = self.flow.idx(pc).and_then(|i| self.settable_key(i)).and_then(|k| kstr(m, p, k));
                    let key = self.get(c);
                    let val = self.get(a);
                    self.pset(b, table.clone());
                    let slot = named.filter(|_| temp).and_then(|n| {
                        let want = format!("{SLOT}{}", field_text(n));
                        table.borrow().fields.as_ref().and_then(|fs| fs.iter().position(|x| *x == want)).map(|at| (n, at))
                    });
                    match slot {
                        Some((n, at)) => self.fill_slot(&table, at, format!("{} = {}", field_text(n), val.borrow().text)),
                        None => {
                            let f = format!("[{}] = {}", key.borrow().text, val.borrow().text);
                            self.add_fields(&table, [f]);
                        }
                    }
                    self.finish_table(out, ind, b, &table, pc);
                } else {
                    self.flush(out, ind, &[a, b, c]);
                    let val = self.get(a);
                    let key = self.get(c);
                    let obj = self.get(b);
                    self.flush(out, ind, &[]);
                    out.push(format!("{pad}{}[{}] = {}", wrap(&obj, 100), key.borrow().text, val.borrow().text));
                }
            }
            op::GETTABLEKS => {
                let obj = self.get(b);
                let t = format!("{}{}", wrap(&obj, 100), index_suffix(kstr_or(m, p, aux as usize, "?")));
                self.put(out, ind, a, ex(t, 100), true);
            }
            _ if self.store_run(pc).len() > 1 && !self.pending_constructor(b) => {
                let run = self.store_run(pc);
                let mut targets = Vec::with_capacity(run.len());
                let mut values = Vec::with_capacity(run.len());
                for s in &run {
                    values.push(self.get(s.a()).borrow().text.clone());
                    let obj = self.get(s.b());
                    let obj = wrap(&obj, 100);
                    targets.push(match s.op {
                        op::SETTABLEKS => format!("{obj}{}", index_suffix(kstr_or(m, p, s.aux as usize, "?"))),
                        op::SETTABLEN => format!("{obj}[{}]", s.c() + 1),
                        _ => {
                            let key = self.get(s.c());
                            format!("{obj}[{}]", key.borrow().text)
                        }
                    });
                }
                self.flush(out, ind, &[]);
                out.push(format!("{pad}{} = {}", targets.join(", "), values.join(", ")));
                let last = run[run.len() - 1];
                self.skip_to = Some(last.pc + last.n);
            }
            op::SETTABLEKS => {
                let fold = a != b && self.pending.get(&b).is_some_and(|e| e.borrow().fields.is_some());
                if fold {
                    let key = kstr_or(m, p, aux as usize, "?");
                    let table = self.pending[&b].clone();
                    let val = self.get(a);
                    self.pset(b, table.clone());
                    let field = field_text(key);
                    let f = format!("{field} = {}", val.borrow().text);
                    let slot = format!("{SLOT}{field}");
                    let at = table.borrow().fields.as_ref().and_then(|fs| fs.iter().position(|x| *x == slot));
                    match at {
                        Some(at) => self.fill_slot(&table, at, f),
                        None => self.add_fields(&table, [f]),
                    }
                    self.finish_table(out, ind, b, &table, pc);
                } else {
                    self.flush(out, ind, &[a, b]);
                    let val = self.get(a);
                    let obj = self.get(b);
                    self.flush(out, ind, &[]);
                    let key = kstr_or(m, p, aux as usize, "?");
                    let k = index_suffix(key);
                    let target = wrap(&obj, 100);
                    // `function t.k()` needs a name chain before the key
                    let chain = target.split('.').all(|w| {
                        w.as_bytes().first().is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
                            && w.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                    });
                    let rest = self.definition_rest(&val.borrow(), Some(key)).filter(|_| chain && k.starts_with('.'));
                    match rest {
                        Some(rest) => out.push(format!("{pad}function {target}{k}{rest}")),
                        None => out.push(format!("{pad}{target}{k} = {}", val.borrow().text)),
                    }
                }
            }
            op::GETTABLEN => {
                let obj = self.get(b);
                let t = format!("{}[{}]", wrap(&obj, 100), c + 1);
                self.put(out, ind, a, ex(t, 100), true);
            }
            op::SETTABLEN => {
                let fold = a != b && self.pending.get(&b).is_some_and(|e| e.borrow().fields.is_some());
                if fold {
                    let table = self.pending[&b].clone();
                    let val = self.get(a);
                    self.pset(b, table.clone());
                    let slots = table.borrow().fields.as_ref().map_or(0, |fs| {
                        if fs.get(c).is_some_and(|x| x == ITEM) { fs.len() } else { 0 }
                    });
                    if slots > 0 {
                        // a list item; the last one is all of a call unless cut
                        let v = val.borrow();
                        let item = if c + 1 == slots && v.one { format!("({})", v.text) } else { v.text.clone() };
                        drop(v);
                        self.fill_slot(&table, c, item);
                    } else {
                        let f = format!("[{}] = {}", c + 1, val.borrow().text);
                        self.add_fields(&table, [f]);
                    }
                    self.finish_table(out, ind, b, &table, pc);
                } else {
                    self.flush(out, ind, &[a, b]);
                    let val = self.get(a);
                    let obj = self.get(b);
                    self.flush(out, ind, &[]);
                    out.push(format!("{pad}{}[{}] = {}", wrap(&obj, 100), c + 1, val.borrow().text));
                }
            }
            op::NEWCLOSURE | op::DUPCLOSURE => return self.closure(out, ind, pc),
            op::NAMECALL => {
                let obj = self.get(b);
                let method = kstr_or(m, p, aux as usize, "?");
                let f = ex(format!("{}:{method}", wrap(&obj, 100)), 100);
                let s = ex("<self>", 100);
                f.borrow_mut().method = true;
                s.borrow_mut().method = true;
                self.pset(a, f);
                self.pset(a + 1, s);
            }
            op::CALL | op::CALLFB => {
                self.declare_named(out, ind, a); // copied-in code returned it
                let f = self.get(a);
                let mut nargs = b as i32 - 1;
                let self_call = self.pending.get(&(a + 1)).is_some_and(|e| e.borrow().text == "<self>");
                if self_call {
                    self.ppop(a + 1);
                }
                let first = if self_call { a + 2 } else { a + 1 };
                if self_call && nargs > 0 {
                    nargs -= 1; // the object itself is the first argument
                }
                if nargs < 0 {
                    // up to the top of the stack: the last argument is multi-valued
                    let last = self.pending_regs_from(first).map_or(first as i32 - 1, |r| r as i32);
                    nargs = last - first as i32 + 1;
                }
                let args: Vec<E> = (first..first + nargs.max(0) as usize).map(|r| self.get(r)).collect();
                let texts = list_texts(&args, b != 0);
                let call = ex(format!("{}({})", wrap(&f, 100), texts.join(", ")), 100);
                call.borrow_mut().one = c == 2 && !self.builtin_one(pc);
                // above the arguments: the call overwrites it, so nothing reads
                // it, but it may have effects, and those come first
                let above: Vec<usize> = self.pending.range(a..).map(|(r, _)| *r).collect();
                for r in above {
                    if self.pending.get(&r).is_some_and(|e| e.borrow().pure_) {
                        self.ppop(r);
                    }
                }
                let mut keep: Vec<usize> = self
                    .pending
                    .iter()
                    .filter(|(r, e)| **r < a && (e.borrow().pure_ || self.flows_with_call(pc, **r)))
                    .map(|(r, _)| *r)
                    .collect();
                self.keep_older_effects(&mut keep);
                self.flush(out, ind, &keep);
                if c == 1 {
                    out.push(format!("{pad}{}", call.borrow().text));
                } else if c == 2 {
                    self.put(out, ind, a, call, true);
                } else if c == 0 {
                    call.borrow_mut().multret = true;
                    self.pset(a, call);
                } else if c == 4
                    && !self.flat
                    && self.valid(nxt)
                    && is_forgprep(self.insn(nxt).op)
                    && self.insn(nxt).a() == a
                {
                    self.iter_call = Some((a, call.borrow().text.clone())); // the loop header says it
                } else {
                    let text = call.borrow().text.clone();
                    self.results(out, ind, a, c - 1, &text);
                }
            }
            op::RETURN => {
                let count = if b == 0 {
                    self.pending_regs_from(a).map_or(0, |r| r - a + 1)
                } else {
                    b - 1
                };
                let vals: Vec<E> = (a..a + count).map(|r| self.get(r)).collect();
                let texts = list_texts(&vals, b != 0);
                self.flush(out, ind, &[]);
                let is_last = p.code.last().is_some_and(|l| l.pc == pc);
                if !(is_last && texts.is_empty()) {
                    let line = format!("{pad}return {}", texts.join(", "));
                    out.push(line.trim_end().to_string());
                }
            }
            op::ADD | op::SUB | op::MUL | op::DIV | op::MOD | op::POW | op::IDIV => {
                let (sym, prec) = binop(o);
                let l = self.get(b);
                let r = self.get(c);
                let t = format!("{} {sym} {}", wrap(&l, prec), wrap(&r, prec + 1));
                self.put(out, ind, a, ex(t, prec), true);
            }
            op::ADDK | op::SUBK | op::MULK | op::DIVK | op::MODK | op::POWK | op::IDIVK => {
                let (sym, prec) = binop(o);
                let l = self.get(b);
                // `x ^ -k` needs no parentheses: a negation binds there
                let need = if o == op::POWK { 8 } else { prec + 1 };
                let t = format!("{} {sym} {}", wrap(&l, prec), kwrap(kexpr(m, p, c as i64), need));
                self.put(out, ind, a, ex(t, prec), true);
            }
            op::SUBRK | op::DIVRK => {
                let (sym, prec) = binop(o);
                let r = self.get(c);
                let t = format!("{} {sym} {}", kwrap(kexpr(m, p, b as i64), prec), wrap(&r, prec + 1));
                self.put(out, ind, a, ex(t, prec), true);
            }
            op::AND | op::OR => {
                let l = self.get(b);
                let r = self.get(c);
                let (sym, prec) = if o == op::AND { ("and", 2) } else { ("or", 1) };
                let t = format!("{} {sym} {}", wrap(&l, prec), wrap(&r, prec + 1));
                self.put(out, ind, a, ex(t, prec), true);
            }
            op::ANDK | op::ORK => {
                let l = self.get(b);
                let (sym, prec) = if o == op::ANDK { ("and", 2) } else { ("or", 1) };
                let t = format!("{} {sym} {}", wrap(&l, prec), kexpr(m, p, c as i64));
                self.put(out, ind, a, ex(t, prec), true);
            }
            op::CONCAT => {
                let parts: Vec<String> = (b..c + 1).map(|r| { let v = self.get(r); wrap(&v, 6) }).collect();
                self.put(out, ind, a, ex(parts.join(" .. "), 5), true);
            }
            op::NOT => {
                let v = self.get(b);
                self.put(out, ind, a, not_of(&v), true);
            }
            op::MINUS => {
                let v = self.get(b);
                let v = wrap(&v, 8);
                let t = if v.starts_with('-') { format!("-({v})") } else { format!("-{v}") };
                self.put(out, ind, a, ex(t, 8), true);
            }
            op::LENGTH => {
                let v = self.get(b);
                self.put(out, ind, a, ex(format!("#{}", wrap(&v, 8)), 8), true);
            }
            op::NEWTABLE | op::DUPTABLE => {
                let table = ex("{}", 100);
                table.borrow_mut().constructor = true;
                table.borrow_mut().newtable = o == op::NEWTABLE;
                table.borrow_mut().fields = Some(if o == op::DUPTABLE {
                    template_slots(m, p, d as i64)
                } else {
                    Vec::new()
                });
                if let Some(w) = self.flow.def_web(pc, a) {
                    self.set_def_key(&table, a, KW + w);
                }
                // its items are made after it: a copy of a shared variable
                // made before, stored into it, was a local (Luau reads a
                // local key or value where it is stored, after any call)
                let read_by_stores = self.store_operands(pc, a);
                // and Luau computes a constructor's items above it: one waiting
                // below it that a store reads was a local
                let tight = self.tight;
                let early: Vec<usize> = self
                    .pending
                    .iter()
                    .filter(|(q, e)| **q != a && read_by_stores.has(**q) && (e.borrow().snapshot || (**q < a && !tight)))
                    .map(|(q, _)| *q)
                    .collect();
                for q in early {
                    let e = self.get(q);
                    self.assign(out, ind, q, &e);
                }
                let fills = self.table_fills(pc, a);
                let mut limit = self.constructor_limit(pc, a);
                if limit.is_none() {
                    if let Some((last, list)) = self.index_keys_end(pc, a) {
                        if list {
                            // each slot of its array part, a nil until stored
                            table.borrow_mut().fields = Some(vec![ITEM.to_string(); self.insn(pc).aux as usize]);
                        }
                        limit = Some(last);
                    }
                }
                if let Some(last) = limit {
                    // the stores up to `last` were its constructor's, as its
                    // size shows; the rest are statements
                    if last > pc {
                        table.borrow_mut().build_end = Some(last);
                        self.ppop(a);
                        self.pset(a, table);
                    } else {
                        let t = table.borrow().render();
                        self.put(out, ind, a, ex(t, 100), false);
                    }
                } else if o == op::NEWTABLE && fills.0 && !fills.1 && !self.item_of_constructor(pc, a, fills.2) {
                    // `local t = {}` then `t.k = v`: a constructor with keys
                    // would have compiled to DUPTABLE, so these are statements
                    // (put: what waited in the register, never read, goes first)
                    self.put(out, ind, a, ex("{}", 100), false);
                } else if self.read_once(pc, a) || self.table_filled_then_read(pc, a) {
                    self.ppop(a);
                    self.pset(a, table);
                } else if fills.2.is_some() && (o == op::DUPTABLE || fills.1) {
                    // a table used more than once: built as a constructor,
                    // then given its variable once the last field is in
                    table.borrow_mut().build_end = fills.2;
                    self.ppop(a);
                    self.pset(a, table);
                } else {
                    // a table used more than once: its template's constant
                    // fields still belong in the constructor
                    let t = table.borrow().render();
                    self.put(out, ind, a, ex(t, 100), false);
                }
            }
            op::SETLIST => {
                let fold = self.pending.get(&a).is_some_and(|e| e.borrow().fields.is_some())
                    && !(b <= a && a < b + c.saturating_sub(1));
                if fold {
                    let table = self.ppop(a).unwrap();
                    let mut count = c as i32 - 1;
                    if count < 0 {
                        count = self.pending_regs_from(b).map_or(0, |r| (r - b + 1) as i32);
                    }
                    let vals: Vec<E> = (b..b + count as usize).map(|r| self.get(r)).collect();
                    let texts = list_texts(&vals, c != 0);
                    self.add_fields(&table, texts);
                    self.pset(a, table.clone());
                    self.finish_table(out, ind, a, &table, pc);
                } else {
                    let mut count = c as i32 - 1;
                    if count < 0 {
                        count = self.pending_regs_from(b).map_or(0, |r| (r - b + 1) as i32);
                    }
                    let vals: Vec<E> = (b..b + count as usize).map(|r| self.get(r)).collect();
                    let mut texts = list_texts(&vals, c != 0);
                    self.flush(out, ind, &[]);
                    let tbl = self.reg(a);
                    // the last one is a list when C says so: all of it goes in
                    let open_end = if c == 0 { texts.pop() } else { None };
                    for (i, v) in texts.iter().enumerate() {
                        out.push(format!("{pad}{tbl}[{}] = {v}", aux as u64 + i as u64));
                    }
                    if let Some(list) = open_end {
                        let at = aux as u64 + texts.len() as u64;
                        out.push(format!("{pad}for i_, v_ in {{ {list} }} do"));
                        if at == 1 {
                            out.push(format!("{pad}    {tbl}[i_] = v_"));
                        } else {
                            out.push(format!("{pad}    {tbl}[i_ + {}] = v_", at - 1));
                        }
                        out.push(format!("{pad}end"));
                    }
                }
            }
            op::GETVARARGS if b > 2 => self.results(out, ind, a, b - 1, "..."),
            op::GETVARARGS => {
                let va = ex("...", 100);
                va.borrow_mut().one = b == 2; // `local x = ...` keeps the first value only
                self.put(out, ind, a, va, true);
            }
            _ => {
                self.flush(out, ind, &[]);
                out.push(format!("{pad}-- {} A={a} B={b} C={c} D={d}", op_name(o)));
            }
        }
        self.skip_to.take().unwrap_or(nxt)
    }

    /// `local a, b = <text>`: n values into the registers from a.
    fn results(&mut self, out: &mut Vec<String>, ind: usize, a: usize, n: usize, text: &str) {
        let names: Vec<Rc<str>> = (0..n).map(|i| self.name_for(a + i)).collect();
        let keys: Vec<u32> = (0..n).map(|i| self.var_key(a + i)).collect();
        let fresh = keys.iter().any(|k| !self.declared[*k as usize]);
        for k in keys {
            self.declare(k);
        }
        let list: Vec<&str> = names.iter().map(|v| &**v).collect();
        let lead = if fresh { "local " } else { "" };
        out.push(format!("{}{lead}{} = {text}", sp(ind), list.join(", ")));
    }

    /// A function's source, written to sit at indent ind.
    fn closure_text(&mut self, child: usize, caps: Vec<Rc<str>>, ind: usize) -> String {
        // `--info`: the body opens with the function's number and the names its
        // upvalues have here, for the comment above it once names are final
        let marker = INFO.load(std::sync::atomic::Ordering::Relaxed).then(|| {
            let names: Vec<&str> = caps.iter().map(|c| &**c).collect();
            format!("{}{}{child}|{}|", sp(ind + 4), passes::INFO_MARK, names.join("|"))
        });
        let key = (child, self.depth + 1, caps, self.ctx.helper_names.get());
        let cached = self.ctx.memo.borrow().get(&key).cloned();
        let result: Body = match cached {
            Some(b) => b,
            None => {
                let mut sub = Dec::new(self.ctx, child, key.2.clone(), self.depth + 1);
                let (params, body) = sub.decompile();
                let r: Body = Rc::new((params, body));
                self.ctx.memo.borrow_mut().insert(key, r.clone());
                r
            }
        };
        let (fparams, body) = (&result.0, &result.1);
        // the body sits one level under the line that writes the closure
        let shift = ind as i64 + 4 - 4 * (self.depth as i64 + 2);
        // with the type each parameter was annotated with, where recorded
        let types = &self.m.protos[child].param_types;
        let shown: Vec<String> = fparams
            .iter()
            .enumerate()
            .map(|(i, p)| match types.get(i).and_then(|t| crate::bytecode::type_name(self.m, *t)) {
                Some(t) if p != "..." => format!("{p}: {t}"),
                _ => p.clone(),
            })
            .collect();
        let mut text = format!("function({})\n", shown.join(", "));
        if let Some(marker) = &marker {
            text.push_str(marker);
            text.push('\n');
        }
        if shift != 0 {
            let mut first = true;
            for chunk in body {
                for l in chunk.split('\n') {
                    if !first {
                        text.push('\n');
                    }
                    first = false;
                    if shift > 0 {
                        text.push_str(sp(shift as usize));
                        text.push_str(l);
                    } else {
                        let cut = ((-shift) as usize).min(passes::indent_of(l));
                        text.push_str(&l[cut..]);
                    }
                }
            }
        } else {
            let mut first = true;
            for chunk in body {
                if !first {
                    text.push('\n');
                }
                first = false;
                text.push_str(chunk);
            }
        }
        text.push('\n');
        text.push_str(sp(ind));
        text.push_str("end");
        text
    }

    fn closure(&mut self, out: &mut Vec<String>, ind: usize, pc: i32) -> i32 {
        let ins = self.insn(pc);
        let (a, d) = (ins.a(), ins.d());
        let (m, p) = (self.m, self.p);
        if ins.op == op::DUPCLOSURE {
            if let Some(name) = self.shared_closures.get(&kindex(p, d as i64)).cloned() {
                self.put(out, ind, a, ex_pure(name.to_string()), true);
                return pc + ins.n;
            }
        }
        let child = if ins.op == op::NEWCLOSURE {
            p.children[if d < 0 { (p.children.len() as i32 + d) as usize } else { d as usize }]
        } else {
            match &p.k[kindex(p, d as i64)] {
                K::Closure(c) => *c,
                _ => panic!("DUPCLOSURE constant is not a closure"),
            }
        };
        let mut caps: Vec<Rc<str>> = Vec::new();
        let mut q = pc + ins.n;
        while self.valid(q) && self.insn(q).op == op::CAPTURE {
            let ci = self.insn(q);
            let (ctype, creg) = (ci.a(), ci.b());
            if ctype == 2 {
                caps.push(Rc::from(self.upval(creg)));
            } else {
                if creg == a {
                    // `local function f` that calls itself: name it now, so
                    // the body refers to it by that name
                    if let Some(hint) = m.protos[child].name.as_deref() {
                        let key = self.var_key(a);
                        let cur_ok = match &self.regname[key as usize] {
                            None => true,
                            Some(n) => is_generated_v(n, false),
                        };
                        // the name the definition will get, a number added if taken
                        if let Some(n) = self.free_name(hint).filter(|_| !self.declared[key as usize] && cur_ok) {
                            let h: Rc<str> = Rc::from(n.as_str());
                            self.set_name(key, h.clone());
                            self.add_used(&h);
                        }
                    }
                } else if self.pending.contains_key(&creg) {
                    let e = self.get(creg);
                    self.assign(out, ind, creg, &e);
                }
                self.key_pc = Some(q);
                caps.push(self.name_for(creg));
                self.key_pc = None;
            }
            q += 1;
        }
        let captured = caps.iter().map(|c| key(c.as_bytes())).collect();
        let names = self.ctx.helper_names.get();
        let mut text = self.closure_text(child, caps.clone(), ind);
        // a helper the body calls by name (its code put back as a call, not
        // an upvalue) has to be a local by then: one made here still waiting
        // to be written goes out first
        let called: Vec<usize> = {
            let helpers = self.ctx.helpers.borrow();
            let calls = |e: &E| {
                let c = e.borrow().closure;
                c.is_some_and(|c| helpers.iter().any(|h| h.0 == c && contains_ident(&text, &h.1)))
            };
            self.pending.iter().filter(|(_, e)| calls(e)).map(|(q, _)| *q).collect()
        };
        for q in called {
            let e = self.get(q);
            self.assign(out, ind, q, &e);
        }
        if self.ctx.helper_names.get() != names {
            text = self.closure_text(child, caps, ind); // under the name it got
        }
        let fname = m.protos[child].name.clone();
        let closure = ex_pure(text);
        closure.borrow_mut().hint = fname;
        closure.borrow_mut().closure = Some(child);
        closure.borrow_mut().names_read = captured;
        self.put(out, ind, a, closure, true);
        q
    }
}

/// At ins a value and a later read of state first meet: is the read written
/// out first? Operands go left to right, lists in register order, and a store
/// says where before what. A fast call's function is loaded last, after its
/// arguments, however it is written.
fn state_operand_first(ins: &Insn, val: &RegSet, st: &RegSet, prev: Option<Insn>) -> bool {
    let (a, b, c) = (ins.a(), ins.b(), ins.c());
    let first_of = |regs: &mut dyn Iterator<Item = usize>| {
        for x in regs {
            if st.has(x) {
                return true;
            }
            if val.has(x) {
                return false;
            }
        }
        true
    };
    match ins.op {
        op::ADD | op::SUB | op::MUL | op::DIV | op::MOD | op::POW | op::IDIV | op::AND | op::OR | op::GETTABLE => {
            first_of(&mut [b, c].into_iter())
        }
        op::CONCAT => first_of(&mut (b..=c)),
        op::CALL | op::CALLFB => {
            let fast = prev.is_some_and(|p| {
                p.a() == a && matches!(p.op, op::GETUPVAL | op::GETGLOBAL | op::GETIMPORT | op::GETTABLEKS)
            });
            let from = if fast { a + 1 } else { a };
            let end = if b == 0 { a + 256 } else { a + b };
            first_of(&mut (from..end))
        }
        op::RETURN => first_of(&mut (a..a + 256)),
        op::SETLIST => first_of(&mut std::iter::once(a).chain(b..b + 256)),
        op::SETTABLE => first_of(&mut [b, c, a].into_iter()),
        op::SETTABLEKS | op::SETTABLEN => first_of(&mut [b, a].into_iter()),
        op::JUMPIFEQ | op::JUMPIFLE | op::JUMPIFLT | op::JUMPIFNOTEQ | op::JUMPIFNOTLE | op::JUMPIFNOTLT => {
            // `x > y` is `y < x`: written with the one made first first
            let other = (ins.aux & 0xFF) as usize;
            first_of(&mut [a.min(other), a.max(other)].into_iter())
        }
        _ => true,
    }
}

/// NEWTABLE's B for a hash part of n keys: the power of two that holds them, as log2 + 1.
fn hash_class(n: usize) -> u8 {
    if n == 0 {
        return 0;
    }
    let mut log2 = 0;
    while (1usize << log2) < n {
        log2 += 1;
    }
    log2 as u8 + 1
}

fn binop(o: u8) -> (&'static str, i32) {
    match o {
        op::ADD | op::ADDK => ("+", 6),
        op::SUB | op::SUBK | op::SUBRK => ("-", 6),
        op::MUL | op::MULK => ("*", 7),
        op::DIV | op::DIVK | op::DIVRK => ("/", 7),
        op::MOD | op::MODK => ("%", 7),
        op::POW | op::POWK => ("^", 9),
        _ => ("//", 7),
    }
}

/// `^{base}local (?:function )?([A-Za-z0-9_, ]+?)(?: =|\(|$)`: how many names
/// the line declares at the function's top level.
/// The most locals alive at any point of a function. Statements are one
/// entry each: a closure's body is inside its entry and has its own limit.
fn peak_locals(body: &[String]) -> usize {
    let (mut alive, mut peak) = (0usize, 0usize);
    let mut scopes: Vec<(usize, usize)> = Vec::new(); // (indent, locals there)
    for chunk in body {
        let line = chunk.split('\n').next().unwrap_or("");
        let ind = passes::indent_of(line);
        while scopes.last().is_some_and(|s| s.0 > ind) {
            alive -= scopes.pop().unwrap().1;
        }
        let new = base_locals(line, &line[..ind]);
        if new > 0 {
            match scopes.last_mut() {
                Some(s) if s.0 == ind => s.1 += new,
                _ => scopes.push((ind, new)),
            }
            alive += new;
            peak = peak.max(alive);
        }
    }
    peak
}

/// The names a statement declares at the start of the body (`base` indent).
fn declared_names(chunk: &str, base: &str) -> Vec<String> {
    let line = chunk.split('\n').next().unwrap_or("");
    let Some(rest) = line.strip_prefix(base).and_then(|l| l.strip_prefix("local ")) else { return Vec::new() };
    if let Some(r) = rest.strip_prefix("function ") {
        let name: String = r.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        return if is_ident(&name) { vec![name] } else { Vec::new() };
    }
    let head = rest.split(" = ").next().unwrap_or(rest);
    head.split(',').map(|s| s.trim().to_string()).filter(|s| is_ident(s)).collect()
}

/// Luau's limit on locals in scope in one function, its parameters included.
const LOCAL_LIMIT: usize = 200;

/// Luau's limit on registers in one function.
const REG_LIMIT: usize = 255;

/// The most registers a line of the body needs, as near as its text tells:
/// the parameters and the locals in scope there, and the widest call on it
/// (one register for the function, one per argument).
fn register_pressure(body: &[String], given: usize) -> usize {
    let mut most = 0;
    scope_walk(body, |line, alive, held| most = most.max(given + alive + held + widest_call(line)));
    most
}

/// The most registers one call on the line takes: the function and each
/// argument at the call's own depth.
fn widest_call(line: &str) -> usize {
    let b = line.as_bytes();
    let mut best = 0;
    let mut opens: Vec<(usize, bool)> = Vec::new(); // (arguments so far, a call)
    let mut quote = 0u8;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == b'\\' {
                i += 1;
            } else if c == quote {
                quote = 0;
            }
        } else if matches!(c, b'"' | b'\'' | b'`') {
            quote = c;
        } else if matches!(c, b'(' | b'{' | b'[') {
            let call = c == b'(' && i > 0 && (b[i - 1].is_ascii_alphanumeric() || matches!(b[i - 1], b'_' | b')' | b']' | b'"'));
            let empty = b.get(i + 1) == Some(&b')');
            opens.push((usize::from(!empty), call));
        } else if c == b',' {
            if let Some(top) = opens.last_mut() {
                top.0 += 1;
            }
        } else if matches!(c, b')' | b'}' | b']') {
            if let Some((args, call)) = opens.pop() {
                if call {
                    best = best.max(args + 1);
                }
            }
        }
        i += 1;
    }
    best
}

/// The most locals in scope at once in a body, loop variables included, as
/// they count against LOCAL_LIMIT. A nested function's are its own: under its
/// own limit, they count for none of this one's.
fn locals_in_scope(body: &[String]) -> usize {
    let mut peak = 0;
    scope_walk(body, |_, alive, _| peak = peak.max(alive));
    peak
}

/// Each line of the function's own (not a nested function's), with the locals
/// in scope once it has run.
fn scope_walk(body: &[String], mut each: impl FnMut(&str, usize, usize)) {
    let (mut alive, mut held) = (0usize, 0usize);
    let mut scopes: Vec<(usize, usize, usize)> = Vec::new(); // (indent, locals there, loop registers)
    let mut inside: Option<usize> = None; // the indent of a nested function's header
    for line in body.iter().flat_map(|c| c.split('\n')).filter(|l| !l.trim().is_empty()) {
        let ind = passes::indent_of(line);
        match inside {
            Some(f) if ind > f => continue,
            Some(_) => inside = None,
            None => {}
        }
        while scopes.last().is_some_and(|s| s.0 > ind) {
            let s = scopes.pop().unwrap();
            alive -= s.1;
            held -= s.2;
        }
        // a function opens here whose body is the deeper lines that follow
        // (`local function f()` is itself one of this function's locals)
        let code = &line[ind..];
        if code.starts_with("function ") || code.starts_with("local function ") || code.contains("function(") || code.contains("= function") {
            inside = Some(ind);
        }
        // a loop's variables live in its body, one level in
        let looped = line[ind..].strip_prefix("for ").map_or(0, |rest| {
            let head = rest.split(" = ").next().unwrap_or(rest).split(" in ").next().unwrap_or(rest);
            head.split(',').count()
        });
        // and its own state besides: generator, state and control (and at
        // least two variables' registers), or limit and step; registers,
        // not locals
        let state = if looped == 0 {
            0
        } else if line[ind..].contains(" in ") {
            3 + 2usize.saturating_sub(looped)
        } else {
            2
        };
        for (at, new, regs) in [(ind, base_locals(line, &line[..ind]), 0), (ind + 1, looped, state)] {
            if new == 0 {
                continue;
            }
            match scopes.last_mut() {
                Some(s) if s.0 == at => {
                    s.1 += new;
                    s.2 += regs;
                }
                _ => scopes.push((at, new, regs)),
            }
            alive += new;
            held += regs;
        }
        each(line, alive, held);
    }
}

/// The most locals in scope at once anywhere in these lines, nested blocks
/// and functions included.
fn peak_lines(lines: &[String]) -> usize {
    let (mut alive, mut peak) = (0usize, 0usize);
    let mut scopes: Vec<(usize, usize)> = Vec::new();
    for line in lines.iter().filter(|l| !l.trim().is_empty()) {
        let ind = passes::indent_of(line);
        while scopes.last().is_some_and(|s| s.0 > ind) {
            alive -= scopes.pop().unwrap().1;
        }
        let new = base_locals(line, &line[..ind]);
        if new > 0 {
            match scopes.last_mut() {
                Some(s) if s.0 == ind => s.1 += new,
                _ => scopes.push((ind, new)),
            }
            alive += new;
            peak = peak.max(alive);
        }
    }
    peak
}

/// Lines at indentation `ind` and the blocks under them, as statements: one
/// starts at each line at `ind` but the ends and middles of blocks. As
/// ranges of the lines.
fn statements_at(lines: &[String], ind: usize) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (j, line) in lines.iter().enumerate() {
        let starts = !line.trim().is_empty() && passes::indent_of(line) == ind && {
            let s = line.trim_start();
            !(s.starts_with("end") || s.starts_with("else") || s.starts_with("until ") || s.starts_with('}') || s.starts_with(')'))
        };
        match out.last_mut() {
            Some(last) if !starts => last.1 = j + 1,
            _ => out.push((j, j + 1)),
        }
    }
    out
}

/// Which lines mention each name, as contains_ident finds them: read once for
/// a whole function, as `do` blocks put back inside change no mention.
struct Mentions<'a> {
    at: HashMap<&'a str, Vec<u32>>,
}

impl<'a> Mentions<'a> {
    fn new(lines: &'a [String]) -> Self {
        let mut at: HashMap<&'a str, Vec<u32>> = HashMap::default();
        for (j, l) in lines.iter().enumerate() {
            for w in ident_words(l) {
                let list = at.entry(w).or_default();
                if list.last() != Some(&(j as u32)) {
                    list.push(j as u32);
                }
            }
        }
        Mentions { at }
    }

    /// The last line in lo..hi that mentions name.
    fn last_in(&self, name: &str, lo: usize, hi: usize) -> Option<usize> {
        let list = self.at.get(name)?;
        let k = list.partition_point(|j| (*j as usize) < hi);
        let last = *list.get(k.checked_sub(1)?)? as usize;
        (last >= lo).then_some(last)
    }
}

/// scope_blocks at every depth: each block's body first, then the
/// statements at this indentation. `keep`: helpers whose copies are here,
/// called by name only once a later pass puts the calls back, so in use to
/// the end.
fn scope_nested(lines: &[String], ind: usize, keep: &[&str]) -> Vec<String> {
    let mentions = Mentions::new(lines);
    scope_level(lines, 0, ind, &mentions, false, keep)
}

/// scope_nested, and inside each block it makes the statements after the one
/// that holds it open are scoped again: deeper blocks, fewer locals at once.
fn scope_nested_deep(lines: &[String], ind: usize, keep: &[&str]) -> Vec<String> {
    let mentions = Mentions::new(lines);
    scope_level(lines, 0, ind, &mentions, true, keep)
}

/// One depth of scope_nested: these lines start at line `off` of the function.
fn scope_level(lines: &[String], off: usize, ind: usize, mentions: &Mentions, deep: bool, keep: &[&str]) -> Vec<String> {
    let ranges = statements_at(lines, ind);
    let mut chunks: Vec<String> = Vec::with_capacity(ranges.len());
    for &(a, b) in &ranges {
        let stmt = &lines[a..b];
        // each run of deeper lines is a block's body (or a function's)
        let mut done: Vec<String> = Vec::with_capacity(stmt.len());
        let mut i = 0;
        while i < stmt.len() {
            let deeper = |l: &String| l.trim().is_empty() || passes::indent_of(l) > ind;
            if i > 0 && deeper(&stmt[i]) {
                let start = i;
                while i < stmt.len() && deeper(&stmt[i]) {
                    i += 1;
                }
                let inner = &stmt[start..i];
                // a function's body (its header ends in `)`: lines are never
                // wrapped) has its own limit and was scoped on its own
                let opener = stmt[start - 1].trim_end();
                if opener.ends_with(')') && opener.contains("function") {
                    done.extend(inner.iter().cloned());
                    continue;
                }
                let inner_ind = inner.iter().find(|l| !l.trim().is_empty()).map_or(ind + 4, |l| passes::indent_of(l));
                done.extend(scope_level(inner, off + a + start, inner_ind, mentions, deep, keep));
            } else {
                done.push(stmt[i].clone());
                i += 1;
            }
        }
        chunks.push(done.join("\n"));
    }
    let spans: Vec<(usize, usize)> = ranges.iter().map(|&(a, b)| (off + a, off + b)).collect();
    scope_blocks(&chunks, &spans, mentions, deep, keep)
        .into_iter()
        .flat_map(|c| c.split('\n').map(String::from).collect::<Vec<_>>())
        .collect()
}

/// A body's top-level statements, a stretch of them whose locals nothing after
/// it mentions wrapped in `do ... end`, so they end there as in the source.
/// A local that lives to the end stays where it is. `spans` are the lines of
/// the function each statement came from, for `mentions`.
fn scope_blocks(body: &[String], spans: &[(usize, usize)], mentions: &Mentions, deep: bool, keep: &[&str]) -> Vec<String> {
    let n = body.len();
    let Some(first) = body.iter().find(|c| !c.trim().is_empty()) else { return body.to_vec() };
    let base = " ".repeat(passes::indent_of(first.split('\n').next().unwrap_or("")));
    let declared: Vec<Vec<String>> = body.iter().map(|c| declared_names(c, &base)).collect();
    if declared.iter().all(Vec::is_empty) {
        return body.to_vec();
    }
    // where each name is declared again
    let mut declares: HashMap<&str, Vec<usize>> = HashMap::default();
    for (j, names) in declared.iter().enumerate() {
        for name in names {
            declares.entry(name.as_str()).or_default().push(j);
        }
    }
    // the last statement mentioning a name, up to one declaring it again
    // (that one included: `local x = x + 1` reads it)
    let last_use = |name: &str, from: usize| -> usize {
        if keep.contains(&name) {
            return n - 1; // a helper whose calls come back later: the last statement
        }
        let again = declares.get(name).and_then(|d| d.iter().find(|j| **j > from).copied());
        let stop = again.map_or(n, |j| j + 1);
        if stop <= from + 1 {
            return from;
        }
        match mentions.last_in(name, spans[from + 1].0, spans[stop - 1].1) {
            Some(line) => spans.partition_point(|s| s.1 <= line),
            None => from,
        }
    };
    let mut out = Vec::with_capacity(n + 8);
    let mut i = 0;
    while i < n {
        if declared[i].is_empty() {
            out.push(body[i].clone());
            i += 1;
            continue;
        }
        let mut end = i;
        let mut k = i;
        while k <= end {
            for name in &declared[k] {
                end = end.max(last_use(name, k));
            }
            k += 1;
        }
        if end + 1 >= n {
            out.push(body[i].clone()); // lives to the end
            i += 1;
            continue;
        }
        out.push(format!("{base}do"));
        // deep: what follows the statement holding the block open may end
        // sooner, in blocks of its own (all of it ends here at the latest)
        let inner: Vec<String> = if deep && end > i + 1 {
            std::iter::once(body[i].clone())
                .chain(scope_blocks(&body[i + 1..=end], &spans[i + 1..=end], mentions, true, keep))
                .collect()
        } else {
            body[i..=end].to_vec()
        };
        for chunk in &inner {
            let lines: Vec<String> =
                chunk.split('\n').map(|l| if l.is_empty() { String::new() } else { format!("    {l}") }).collect();
            out.push(lines.join("\n"));
        }
        out.push(format!("{base}end"));
        i = end + 1;
    }
    out
}

fn base_locals(line: &str, base: &str) -> usize {
    let Some(rest) = line.strip_prefix(base).and_then(|l| l.strip_prefix("local ")) else {
        return 0;
    };
    fn scan(s: &str) -> Option<usize> {
        let b = s.as_bytes();
        let mut i = 0;
        while i < b.len() && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'_' | b',' | b' ')) {
            i += 1;
            if i == b.len() || b[i] == b'(' || s[i..].starts_with(" =") {
                return Some(s[..i].split(',').count());
            }
        }
        None
    }
    if let Some(r) = rest.strip_prefix("function ") {
        if let Some(c) = scan(r) {
            return c;
        }
    }
    scan(rest).unwrap_or(0)
}

/// Annotated disassembly of every function.
pub fn disassemble(m: &Module) -> String {
    let mut all = Vec::with_capacity(m.protos.len());
    for (idx, p) in m.protos.iter().enumerate() {
        let mut lines = vec![format!(
            "; function {idx} {} params={} upvals={} vararg={} maxstack={} line={}",
            p.name.as_deref().filter(|n| !n.is_empty()).unwrap_or("(anon)"),
            p.numparams,
            p.nups,
            p.vararg,
            p.maxstack,
            p.linedefined
        )];
        for ins in &p.code {
            let (pc, d) = (ins.pc, ins.d());
            let name = op_name(ins.op);
            let note = match ins.op {
                op::LOADK | op::DUPTABLE | op::DUPCLOSURE => kexpr(m, p, d as i64),
                op::LOADKX => kexpr(m, p, ins.aux as i64),
                op::GETGLOBAL | op::SETGLOBAL | op::GETTABLEKS | op::SETTABLEKS | op::NAMECALL => {
                    kexpr(m, p, ins.aux as i64)
                }
                op::GETIMPORT => import_expr(m, p, ins.aux),
                _ if name.starts_with("JUMP") || name.starts_with("FOR") => format!("-> {}", pc + 1 + d),
                _ => String::new(),
            };
            let at = p.lines.get(pc as usize).map_or(String::new(), |l| format!("{l:5}:"));
            let mut l = format!(
                "{at}  {pc:4} {name:<15} A={} B={} C={} D={d}",
                ins.a(),
                ins.b(),
                ins.c()
            );
            if ins.n == 2 {
                l.push_str(&format!(" AUX={:#x}", ins.aux));
            }
            if !note.is_empty() {
                l.push_str("  ; ");
                l.push_str(&note);
            }
            lines.push(l);
        }
        all.push(lines.join("\n"));
    }
    all.join("\n\n")
}

/// The builtins whose result count Luau's compiler knows to be one: all up to
/// the vector library but assert, math.frexp and math.modf, string.byte,
/// table.insert and table.unpack, select(_, ...) and the buffer writes.
fn builtin_returns_one(id: usize) -> bool {
    (2..=88).contains(&id) && ![14, 20, 41, 52, 53, 57, 67, 70, 73, 75, 77].contains(&id)
}
