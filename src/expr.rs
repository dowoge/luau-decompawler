//! Expression values: source text plus just enough structure to negate and
//! merge conditions exactly.
//!
//! Exprs are shared and mutable on purpose. A table constructor stays one
//! object while its fields arrive, and code compares values by identity
//! ("is this the value that was tested?"), so an `E` is a handle.

use std::cell::{Cell, RefCell};
use crate::hash::HashSet;
use std::rc::Rc;

pub type E = Rc<RefCell<X>>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Logic {
    And,
    Or,
}

pub struct Cmp {
    pub lhs: E,
    pub sym: &'static str,
    pub rhs: E,
    pub negated: bool,
}

pub struct X {
    pub id: u64,
    pub text: String,
    /// Binding strength, for parentheses.
    pub prec: i32,
    /// No side effects, unaffected by any call.
    pub pure_: bool,
    /// A natural local name for this value.
    pub hint: Option<String>,
    /// Table constructor fields, while still being built.
    pub fields: Option<Vec<String>>,
    /// A method lookup (or its `self` slot), waiting for its call.
    pub method: bool,
    /// `lhs sym rhs`: what it means, not how it is written.
    pub cmp: Option<Rc<Cmp>>,
    pub logic: Option<(Logic, E, E)>,
    pub logic_neg: Option<E>,
    /// Cut to one value: `(f())` where a list would expand it.
    pub one: bool,
    /// All its values go on: never a single assignment.
    pub multret: bool,
    /// (register, variable) this value was computed for.
    pub def_key: Option<(usize, u32)>,
    /// A constructor is complete after this pc.
    pub build_end: Option<i32>,
    pub ifparts: Option<Vec<(E, E)>>,
    pub ifelse: Option<E>,
    /// Identities of the values this one was assembled from.
    pub uses: HashSet<u64>,
    /// The function this value creates.
    pub closure: Option<usize>,
    /// A copy of a variable a closure shares: a call may change the variable,
    /// and Luau reads a local operand where it is used, so the copy is a
    /// local of its own before any call.
    pub snapshot: bool,
    /// A table constructor.
    pub constructor: bool,
    /// A snapshot of a local register (not of an upvalue): printed as the
    /// local, Luau would read it where it is used.
    pub local_copy: bool,
    /// The names a constructor's fields or a closure's captures read: all a
    /// constructor or closure reads when it is made (a body runs later).
    pub names_read: HashSet<u64>,
}

thread_local! {
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
}

pub fn ex(text: impl Into<String>, prec: i32) -> E {
    let id = NEXT_ID.with(|c| {
        let v = c.get();
        c.set(v + 1);
        v
    });
    Rc::new(RefCell::new(X {
        id,
        text: text.into(),
        prec,
        pure_: false,
        hint: None,
        fields: None,
        method: false,
        cmp: None,
        logic: None,
        logic_neg: None,
        one: false,
        multret: false,
        def_key: None,
        build_end: None,
        ifparts: None,
        ifelse: None,
        uses: HashSet::default(),
        closure: None,
        snapshot: false,
        constructor: false,
        local_copy: false,
        names_read: HashSet::default(),
    }))
}

/// How tightly a constant's text binds: `0/0` is a division, `-4` a negation.
pub fn const_prec(t: &str) -> i32 {
    if t == "0/0" {
        7
    } else if t.starts_with('-') {
        8
    } else {
        100
    }
}

/// A constant, as a value.
pub fn ex_const(text: String) -> E {
    let prec = const_prec(&text);
    let e = ex(text, prec);
    e.borrow_mut().pure_ = true;
    e
}

/// A constant's text as an operand that must bind at least `need`.
pub fn kwrap(t: String, need: i32) -> String {
    if const_prec(&t) < need {
        format!("({t})")
    } else {
        t
    }
}

pub fn ex_pure(text: impl Into<String>) -> E {
    let e = ex(text, 100);
    e.borrow_mut().pure_ = true;
    e
}

/// A template key no store has given its value yet, in its place among the
/// fields: Luau makes a template's keys in order, so where a key is decides
/// the table's layout.
pub const SLOT: char = '\u{1}';

/// A list item no store has given its value yet: nil in its place.
pub const ITEM: &str = "\u{2}";

impl X {
    pub fn render(&self) -> String {
        match &self.fields {
            None => self.text.clone(),
            Some(f) if f.is_empty() => "{}".to_string(),
            Some(f) => {
                // a slot never filled stays in the template, with nothing in it
                let shown: Vec<String> = f
                    .iter()
                    .map(|x| match x.strip_prefix(SLOT) {
                        Some(k) => format!("{k} = nil"),
                        None if x == ITEM => "nil".to_string(),
                        None => x.clone(),
                    })
                    .collect();
                format!("{{{}}}", shown.join(", "))
            }
        }
    }
}

pub fn wrap(e: &E, prec: i32) -> String {
    let x = e.borrow();
    let t = &x.text;
    if prec >= 100
        && (t.starts_with(['"', '\'', '`', '['])
            || t.starts_with('{')
            || t.starts_with("function")
            || matches!(t.as_str(), "..." | "nil" | "true" | "false")
            || t.as_bytes().first().is_some_and(u8::is_ascii_digit)
            || t.starts_with('-'))
    {
        return format!("({t})"); // a literal cannot be called or indexed bare
    }
    if x.prec >= prec {
        t.clone()
    } else {
        format!("({t})")
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

pub fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

pub fn is_ident(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty() && is_ident_start(b[0]) && b.iter().all(|&c| is_ident_byte(c))
}

/// Literals and SHOUTED_CONSTANTS: what people write on the right of a comparison.
fn const_like(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() {
        return false;
    }
    if matches!(s, "math.huge" | "-math.huge" | "nil" | "true" | "false") {
        return true;
    }
    let i = usize::from(b[0] == b'-');
    if i < b.len()
        && b[i].is_ascii_digit()
        && b[i + 1..].iter().all(|c| c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'+' | b'-'))
    {
        return true;
    }
    if b.len() >= 2 && b[0] == b'"' && b[b.len() - 1] == b'"' && !b[1..b.len() - 1].contains(&b'"') {
        return true;
    }
    // ([A-Za-z_][A-Za-z0-9_]*\.)*[A-Z][A-Z0-9_]+
    let (prefix, tail) = match s.rfind('.') {
        Some(p) => (&s[..p], &s[p + 1..]),
        None => ("", s),
    };
    let t = tail.as_bytes();
    if t.len() < 2 || !t[0].is_ascii_uppercase() {
        return false;
    }
    if !t[1..].iter().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == b'_') {
        return false;
    }
    prefix.is_empty() || prefix.split('.').all(is_ident)
}

pub fn mirror(s: &'static str) -> &'static str {
    match s {
        "<" => ">",
        "<=" => ">=",
        ">" => "<",
        ">=" => "<=",
        other => other,
    }
}

/// `lhs sym rhs` (or its negation). Written constant-last, as people do.
pub fn make_cmp(lhs: &E, sym: &'static str, rhs: &E, negated: bool) -> E {
    let (mut a, mut b, mut s) = (lhs.clone(), rhs.clone(), sym);
    let asked = negated;
    let mut neg = negated;
    if const_like(&a.borrow().text) && !const_like(&b.borrow().text) {
        std::mem::swap(&mut a, &mut b);
        s = mirror(s);
    }
    if neg && (s == "==" || s == "~=") {
        s = if s == "==" { "~=" } else { "==" };
        neg = false;
    }
    let text = format!("{} {} {}", wrap(&a, 4), s, wrap(&b, 4));
    let e = if neg { ex(format!("not ({text})"), 8) } else { ex(text, 3) };
    e.borrow_mut().cmp = Some(Rc::new(Cmp { lhs: lhs.clone(), sym, rhs: rhs.clone(), negated: asked }));
    e
}

/// `left and right` / `left or right`, remembering its parts.
pub fn mk_logic(op: Logic, left: &E, right: &E) -> E {
    let same = matches!(&right.borrow().logic, Some((o, _, _)) if *o == op);
    let e = if op == Logic::And {
        // `a and (b and c)` is `a and b and c`: same value, same order
        let r = if same { right.borrow().text.clone() } else { wrap(right, 3) };
        ex(format!("{} and {}", wrap(left, 2), r), 2)
    } else {
        let r = if same { right.borrow().text.clone() } else { wrap(right, 2) };
        ex(format!("{} or {}", wrap(left, 1), r), 1)
    };
    e.borrow_mut().logic = Some((op, left.clone(), right.clone()));
    e
}

/// A literal that is always true when tested: not nil, not false.
/// A literal that is true however it is used: `true`, a number, a string. The
/// whole text: `1 .. s == t` starts with a digit and may be false.
pub fn truthy_constant(e: &E) -> bool {
    let x = e.borrow();
    let t = x.text.trim_start_matches('-');
    let number = t.parse::<f64>().is_ok()
        || t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).is_some_and(|h| !h.is_empty() && h.bytes().all(|c| c.is_ascii_hexdigit()));
    let string = t.len() >= 2 && t.starts_with('"') && t.ends_with('"') && {
        // one literal: no quote inside but escaped ones
        let inner = &t.as_bytes()[1..t.len() - 1];
        let mut i = 0;
        let mut one = true;
        while i < inner.len() {
            if inner[i] == b'\\' {
                i += 1;
            } else if inner[i] == b'"' {
                one = false;
                break;
            }
            i += 1;
        }
        one && !(t.len() >= 3 && t.as_bytes()[t.len() - 2] == b'\\' && t.as_bytes()[t.len() - 3] != b'\\')
    };
    x.text == "true" || ((number || string) && !t.is_empty())
}

pub fn is_boolean(e: &E) -> bool {
    let x = e.borrow();
    if x.cmp.is_some() || x.text == "true" || x.text == "false" || (x.text.starts_with("not ") && x.prec == 8) {
        return true;
    }
    match &x.logic {
        Some((_, l, r)) => is_boolean(l) && is_boolean(r),
        None => false,
    }
}

fn balanced_parens(s: &str) -> bool {
    let mut depth = 0i32;
    for c in s.bytes() {
        if c == b'(' {
            depth += 1;
        } else if c == b')' {
            depth -= 1;
            if depth < 0 {
                return false;
            }
        }
    }
    depth == 0
}

/// `not v`; negating it gives v back.
pub fn not_of(v: &E) -> E {
    let e = ex(format!("not {}", wrap(v, 8)), 8);
    e.borrow_mut().logic_neg = Some(v.clone());
    e
}

pub fn negate(e: &E) -> E {
    if let Some(c) = e.borrow().cmp.clone() {
        return make_cmp(&c.lhs, c.sym, &c.rhs, !c.negated);
    }
    let logic = e.borrow().logic.clone();
    if let Some((op, l, r)) = logic {
        // De Morgan, when it leaves fewer `not`s than wrapping the whole
        let flipped = if op == Logic::And { Logic::Or } else { Logic::And };
        let dm = mk_logic(flipped, &negate(&l), &negate(&r));
        let plain = ex(format!("not ({})", e.borrow().text), 8);
        plain.borrow_mut().logic_neg = Some(e.clone());
        let fewer = dm.borrow().text.matches("not ").count() < plain.borrow().text.matches("not ").count();
        return if fewer { dm } else { plain };
    }
    if let Some(n) = e.borrow().logic_neg.clone() {
        return n;
    }
    let (t, prec) = {
        let x = e.borrow();
        (x.text.clone(), x.prec)
    };
    if t.starts_with("not ") && prec == 8 {
        let inner = &t[4..];
        if inner.len() >= 2 && inner.starts_with('(') && inner.ends_with(')') && balanced_parens(&inner[1..inner.len() - 1]) {
            return ex(&inner[1..inner.len() - 1], 0); // no telling how it binds
        }
        return ex(inner, 100);
    }
    // `x == y` as a whole, maybe in parentheses: not one inside a call, an
    // index or a string (`not f(x == nil)` is no `f(x ~= nil)`)
    if t.bytes().filter(|c| *c == b' ').count() == 2 {
        let whole = t.len() >= 2 && t.starts_with('(') && t.ends_with(')') && balanced_parens(&t[1..t.len() - 1]);
        let core = if whole { &t[1..t.len() - 1] } else { &t[..] };
        for (a, b) in [(" == ", " ~= "), (" ~= ", " == ")] {
            if let Some(at) = outside_brackets(core, a) {
                let flipped = format!("{}{b}{}", &core[..at], &core[at + a.len()..]);
                return ex(if whole { format!("({flipped})") } else { flipped }, 3);
            }
        }
    }
    ex(format!("not {}", wrap(e, 8)), 8)
}

/// Where `op` stands in t outside brackets and strings, if there it does
/// exactly once.
fn outside_brackets(t: &str, op: &str) -> Option<usize> {
    let b = t.as_bytes();
    let (mut depth, mut quote, mut found) = (0i32, 0u8, None);
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
        } else if depth == 0 && t[i..].starts_with(op) {
            if found.is_some() {
                return None;
            }
            found = Some(i);
        }
        i += 1;
    }
    found
}

/// `if c then a else b` when a or b repeats the other's last part, which is
/// what `and`/`or` chains compile to: their paths share a tail, and following
/// each path reaches it twice. All four identities hold exactly, values and
/// order of evaluation included (c decides, then y, then the shared tail).
fn shared_logic(c: &E, a: &E, b: &E) -> Option<E> {
    let tail_of = |v: &E, other: &E| -> Option<(Logic, E)> {
        let (op, y, r) = v.borrow().logic.clone()?;
        let same = r.borrow().text == other.borrow().text;
        same.then_some((op, y))
    };
    if let Some((op, y)) = tail_of(a, b) {
        return Some(match op {
            // if c then y or b else b  ==  c and y or b
            Logic::Or => mk_logic(Logic::Or, &mk_logic(Logic::And, c, &y), b),
            // if c then y and b else b  ==  (not c or y) and b
            Logic::And => mk_logic(Logic::And, &mk_logic(Logic::Or, &negate(c), &y), b),
        });
    }
    if let Some((op, y)) = tail_of(b, a) {
        return Some(match op {
            // if c then a else y or a  ==  not c and y or a
            Logic::Or => mk_logic(Logic::Or, &mk_logic(Logic::And, &negate(c), &y), a),
            // if c then a else y and a  ==  (c or y) and a
            Logic::And => mk_logic(Logic::And, &mk_logic(Logic::Or, c, &y), a),
        });
    }
    None
}

/// `if cond then a else b` as one expression; an else that is itself an
/// if-expression continues as elseif. Exact simplifications only.
pub fn if_expr(cond: &E, a: &E, b: &E) -> E {
    let e;
    if let Some(v) = shared_logic(cond, a, b) {
        let mut uses: HashSet<u64> = a.borrow().uses.clone();
        uses.extend(b.borrow().uses.iter().copied());
        uses.insert(a.borrow().id);
        uses.insert(b.borrow().id);
        v.borrow_mut().uses = uses;
        return v;
    }
    // if c then (if d then x else y) else y  ==  if c and d then x else y
    let inner = {
        let x = a.borrow();
        match (&x.ifparts, &x.ifelse) {
            (Some(parts), Some(tail)) if parts.len() == 1 && tail.borrow().text == b.borrow().text => {
                Some((parts[0].0.clone(), parts[0].1.clone()))
            }
            _ => None,
        }
    };
    if let Some((d, x)) = inner {
        return if_expr(&mk_logic(Logic::And, cond, &d), &x, b);
    }
    let ta = a.borrow().text.clone();
    let tb = b.borrow().text.clone();
    if ta == "true" && tb == "false" && is_boolean(cond) {
        let c = cond.borrow();
        e = ex(c.text.clone(), c.prec);
        let mut x = e.borrow_mut();
        x.cmp = c.cmp.clone();
        x.logic = c.logic.clone();
    } else if ta == "false" && tb == "true" && is_boolean(cond) {
        e = negate(cond);
    } else {
        fn val(v: &E) -> String {
            let x = v.borrow();
            if x.text.starts_with("if ") {
                format!("({})", x.text)
            } else {
                x.text.clone()
            }
        }
        let mut parts: Vec<(E, E)> = vec![(cond.clone(), a.clone())];
        let mut tail = b.clone();
        loop {
            let next = {
                let t = tail.borrow();
                match &t.ifparts {
                    Some(p) if !p.is_empty() => {
                        parts.extend(p.iter().cloned());
                        t.ifelse.clone()
                    }
                    _ => None,
                }
            };
            match next {
                Some(n) => tail = n,
                None => break,
            }
        }
        // same value in adjacent branches: one branch, conditions or'ed
        let mut merged: Vec<(E, E)> = Vec::with_capacity(parts.len());
        for (c, v) in parts {
            let same = merged.last().is_some_and(|m| m.1.borrow().text == v.borrow().text);
            if same {
                let last = merged.last_mut().unwrap();
                last.0 = mk_logic(Logic::Or, &last.0, &c);
            } else {
                merged.push((c, v));
            }
        }
        let mut parts = merged;
        // a last branch giving the else value: drop it (when its test has no call)
        while let Some(last) = parts.last() {
            if last.1.borrow().text == tail.borrow().text && !last.0.borrow().text.contains('(') {
                parts.pop();
            } else {
                break;
            }
        }
        // if A then x elseif B then y else x  ->  if not A and B then y else x
        if parts.len() == 2 && parts[0].1.borrow().text == tail.borrow().text {
            let c = mk_logic(Logic::And, &negate(&parts[0].0), &parts[1].0);
            let v = parts[1].1.clone();
            parts = vec![(c, v)];
        }
        if parts.is_empty() {
            return tail;
        }
        if parts.len() == 1
            && parts[0].1.borrow().text == "true"
            && tail.borrow().text == "false"
            && is_boolean(&parts[0].0)
        {
            return parts[0].0.clone();
        }
        let mut text = format!("if {} then {}", parts[0].0.borrow().text, val(&parts[0].1));
        for (c, v) in &parts[1..] {
            text.push_str(" elseif ");
            text.push_str(&c.borrow().text);
            text.push_str(" then ");
            text.push_str(&val(v));
        }
        text.push_str(" else ");
        text.push_str(&val(&tail));
        e = ex(text, 0);
        let mut x = e.borrow_mut();
        x.ifparts = Some(parts);
        x.ifelse = Some(tail);
    }
    let mut uses: HashSet<u64> = a.borrow().uses.clone();
    uses.extend(b.borrow().uses.iter().copied());
    uses.insert(a.borrow().id);
    uses.insert(b.borrow().id);
    e.borrow_mut().uses = uses;
    e
}

/// Text of an argument/return/constructor list. In a list of fixed length
/// the last value was cut to one value; a call or `...` there would expand
/// again, so it is parenthesized.
pub fn list_texts(exprs: &[E], fixed: bool) -> Vec<String> {
    let mut texts: Vec<String> = exprs.iter().map(|e| e.borrow().text.clone()).collect();
    if fixed {
        if let Some(last) = exprs.last() {
            if last.borrow().one {
                let t = texts.pop().unwrap();
                texts.push(format!("({t})"));
            }
        }
    }
    texts
}

// ------------------------------------------------------------- formatting

pub fn lua_string(b: &[u8]) -> String {
    let mut out = String::with_capacity(b.len() + 2);
    out.push('"');
    for &c in b {
        match c {
            0x5C => out.push_str("\\\\"),
            0x22 => out.push_str("\\\""),
            0x0A => out.push_str("\\n"),
            0x0D => out.push_str("\\r"),
            0x09 => out.push_str("\\t"),
            32..=126 => out.push(c as char),
            _ => {
                out.push('\\');
                out.push_str(&c.to_string());
            }
        }
    }
    out.push('"');
    out
}

/// Shortest round-trip digits, laid out the way `repr(float)` does.
fn float_repr(v: f64) -> String {
    // Shortest length, then the digits by exact rounding: when v is halfway
    // between two shortest forms, ties go to even. Next to a power of two the
    // spacing below v is half that above, so the even form may not read back.
    let shortest = format!("{v:e}");
    let (m, _) = shortest.split_once('e').unwrap();
    let ndigits = m.bytes().filter(u8::is_ascii_digit).count();
    let rounded = format!("{:.*e}", ndigits - 1, v);
    let s = if rounded.parse::<f64>() == Ok(v) { rounded } else { shortest };
    let (mant, exp) = s.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let (sign, mant) = match mant.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mant),
    };
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    if (-4..16).contains(&exp) {
        if exp >= 0 {
            let int_len = exp as usize + 1;
            if digits.len() <= int_len {
                let zeros = "0".repeat(int_len - digits.len());
                format!("{sign}{digits}{zeros}.0")
            } else {
                format!("{sign}{}.{}", &digits[..int_len], &digits[int_len..])
            }
        } else {
            let zeros = "0".repeat((-exp - 1) as usize);
            format!("{sign}0.{zeros}{digits}")
        }
    } else {
        let frac = if digits.len() > 1 { format!(".{}", &digits[1..]) } else { String::new() };
        let e_sign = if exp < 0 { '-' } else { '+' };
        format!("{sign}{}{frac}e{e_sign}{:02}", &digits[..1], exp.abs())
    }
}

pub fn fmt_number(v: f64) -> String {
    if v.is_nan() {
        // 0/0 folds to the default NaN, its sign bit set where Luau compiles
        // (x86); the other sign is its negation
        return if v.is_sign_negative() { "0/0" } else { "-(0/0)" }.to_string();
    }
    if v == 0.0 && v.is_sign_negative() {
        return "-0".to_string(); // negative zero prints differently
    }
    if v.is_infinite() {
        return if v > 0.0 { "math.huge".to_string() } else { "-math.huge".to_string() };
    }
    if v == v.trunc() && v.abs() < 1e15 {
        return (v as i64).to_string();
    }
    float_repr(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_shortest() {
        assert_eq!(fmt_number(0.5), "0.5");
        assert_eq!(fmt_number(-4.0), "-4");
        assert_eq!(fmt_number(-0.0), "-0");
        assert_eq!(fmt_number(1e15), "1000000000000000.0");
        assert_eq!(fmt_number(9007199254740992.0), "9007199254740992.0");
        assert_eq!(fmt_number(1e16), "1e+16");
        assert_eq!(fmt_number(1.5e300), "1.5e+300");
        assert_eq!(fmt_number(0.0001), "0.0001");
        assert_eq!(fmt_number(1e-5), "1e-05");
        assert_eq!(fmt_number(f64::INFINITY), "math.huge");
        assert_eq!(fmt_number(-f64::NAN), "0/0");
        assert_eq!(fmt_number(f64::NAN.copysign(1.0)), "-(0/0)");
    }

    #[test]
    fn ties_go_to_even_when_that_reads_back() {
        assert_eq!(fmt_number(84.6878967285156250), "84.68789672851562");
        assert_eq!(fmt_number(123.220001220703125), "123.22000122070312");
    }

    #[test]
    fn next_to_a_power_of_two_the_even_form_may_not_read_back() {
        assert_eq!(fmt_number(2f64.powi(-24)), "5.960464477539063e-08");
    }

    #[test]
    fn comparisons_are_written_constant_last() {
        let (x, zero) = (ex_pure("x"), ex_pure("0"));
        let e = make_cmp(&zero, "<", &x, false);
        assert_eq!(e.borrow().text, "x > 0");
        assert_eq!(negate(&e).borrow().text, "not (x > 0)");
        assert_eq!(negate(&negate(&e)).borrow().text, "x > 0");
    }

    #[test]
    fn undoing_not_keeps_what_binds_loosely_together() {
        let (a, b) = (ex_pure("a"), ex_pure("b"));
        let either = mk_logic(Logic::Or, &a, &b);
        let back = negate(&not_of(&either));
        assert_eq!(mk_logic(Logic::And, &back, &ex_pure("c")).borrow().text, "(a or b) and c");
    }

    #[test]
    fn only_a_whole_equality_flips() {
        assert_eq!(negate(&ex("f(x == nil)", 100)).borrow().text, "not f(x == nil)");
        assert_eq!(negate(&ex("t[a == b]", 100)).borrow().text, "not t[a == b]");
        assert_eq!(negate(&ex("\"a == b\"", 100)).borrow().text, "not \"a == b\"");
        assert_eq!(negate(&ex("f(a) == b", 3)).borrow().text, "f(a) ~= b");
        assert_eq!(negate(&ex("(t.k ~= nil)", 0)).borrow().text, "(t.k == nil)");
    }

    #[test]
    fn only_whole_literals_are_truthy_constants() {
        for yes in ["true", "1", "-2.5", "0x1F", "\"a\"", "\"a \\\" b\"", "0"] {
            assert!(truthy_constant(&ex(yes, 100)), "{yes}");
        }
        for no in ["1 .. s == t", "\"a\" == x", "\"a\" .. \"b\"", "1 + x", "false", "nil", "x"] {
            assert!(!truthy_constant(&ex(no, 0)), "{no}");
        }
    }

    #[test]
    fn a_negated_equality_negates_back() {
        let (x, two) = (ex_pure("x"), ex_pure("2"));
        let e = make_cmp(&x, "==", &two, false);
        assert_eq!(negate(&e).borrow().text, "x ~= 2");
        assert_eq!(negate(&negate(&e)).borrow().text, "x == 2");
    }
}
