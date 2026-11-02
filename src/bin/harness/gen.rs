//! Random Luau programs: deterministic, always terminating, printing the values
//! they compute. Every expression has a type and never errors at run time, and
//! loops are bounded.

use crate::luau::Args;
use crate::rng::Rng;

type Ty = &'static str;
type Var = (String, Ty);

#[derive(Clone)]
struct Helper {
    name: String,
    params: Vec<Ty>,
    ret: Ty,
}

struct Gen {
    r: Rng,
    size: usize,
    n: u64,
    lines: Vec<String>,
    helpers: Vec<Helper>,
}

/// What a generated program is, from its seed and size.
pub fn program(seed: u64, size: usize) -> String {
    Gen { r: Rng::new(seed), size, n: 0, lines: Vec::new(), helpers: Vec::new() }.program()
}

pub fn main(args: &Args) -> u8 {
    let (Some(seed), Some(size)) = (args.0.first(), args.0.get(1)) else {
        eprintln!("harness gen SEED SIZE");
        return 2;
    };
    print!("{}", program(seed.parse().expect("a seed"), size.parse().expect("a size")));
    0
}

fn of(sc: &[Var], t: Ty) -> Vec<String> {
    sc.iter().filter(|e| e.1 == t).map(|e| e.0.clone()).collect()
}

/// Our own variables: not a loop's, which must not be assigned.
fn own_num(v: &str) -> bool {
    !v.starts_with(['i', 'c'])
}

fn with(sc: &[Var], more: &[(&String, Ty)]) -> Vec<Var> {
    let mut s = sc.to_vec();
    s.extend(more.iter().map(|(v, t)| ((*v).clone(), *t)));
    s
}

fn lit(t: Ty) -> &'static str {
    match t {
        "num" => "1",
        "bool" => "true",
        _ => "\"z\"",
    }
}

/// One line. Its parts are computed first, in the order they are written.
macro_rules! put {
    ($s:ident, $ind:expr, $($fmt:tt)*) => {{
        let text = format!($($fmt)*);
        $s.emit($ind, text);
    }};
}

impl Gen {
    fn fresh(&mut self, base: &str) -> String {
        self.n += 1;
        format!("{base}{}", self.n)
    }

    fn emit(&mut self, ind: usize, text: String) {
        self.lines.push("    ".repeat(ind) + &text);
    }

    fn helpers_of(&self, t: Ty) -> Vec<Helper> {
        self.helpers.iter().filter(|h| h.ret == t).cloned().collect()
    }

    // -- expressions
    fn num(&mut self, sc: &[Var], d: u32) -> String {
        let nums = of(sc, "num");
        let c = self.r.float();
        if d > 3 || c < 0.25 {
            if !nums.is_empty() && self.r.float() < 0.7 {
                return self.r.pick(&nums);
            }
            return self.r.pick(&["0", "1", "2", "3", "5", "7", "10", "-1", "-4", "100", "0.5", "2.25"]).to_string();
        }
        let e = d + 1;
        match self.r.below(14) {
            0 => return format!("{} + {}", self.num(sc, e), self.num(sc, e)),
            1 => return format!("{} - {}", self.num(sc, e), self.num(sc, e)),
            2 => return format!("({}) * {}", self.num(sc, e), self.r.pick(&["2", "3", "-1", "0.5"])),
            3 => return format!("({}) // {}", self.num(sc, e), self.r.pick(&["2", "3", "4"])),
            4 => return format!("({}) % {}", self.num(sc, e), self.r.pick(&["3", "5", "7"])),
            5 => return format!("math.{}({}, {})", self.r.pick(&["max", "min"]), self.num(sc, e), self.num(sc, e)),
            6 => return format!("math.abs({})", self.num(sc, e)),
            7 => return format!("(if {} then {} else {})", self.boolean(sc, e), self.num(sc, e), self.num(sc, e)),
            8 => return format!("({} and {} or {})", self.boolean(sc, e), self.num(sc, e), self.num(sc, e)),
            9 => {
                let arrs = of(sc, "arr");
                if !arrs.is_empty() {
                    return format!("#{}", self.r.pick(&arrs));
                }
            }
            10 => {
                let objs = of(sc, "obj");
                if !objs.is_empty() {
                    let o = self.r.pick(&objs);
                    return self.r.pick(&[format!("{o}.x"), format!("{o}.y"), format!("{o}:get()")]);
                }
            }
            11 => {
                let hs = self.helpers_of("num");
                if !hs.is_empty() {
                    let h = self.r.pick(&hs);
                    return self.call(&h, sc, d);
                }
            }
            12 => return format!("-{}", self.atom_num(sc)),
            13 => return format!("string.len({})", self.string(sc, e)),
            _ => {}
        }
        self.atom_num(sc)
    }

    fn atom_num(&mut self, sc: &[Var]) -> String {
        let nums = of(sc, "num");
        if nums.is_empty() {
            self.r.below(10).to_string()
        } else {
            self.r.pick(&nums)
        }
    }

    fn boolean(&mut self, sc: &[Var], d: u32) -> String {
        let bools = of(sc, "bool");
        let c = self.r.float();
        if d > 3 || c < 0.2 {
            if !bools.is_empty() && self.r.float() < 0.7 {
                return self.r.pick(&bools);
            }
            return self.r.pick(&["true", "false"]).to_string();
        }
        let e = d + 1;
        match self.r.below(8) {
            0 => {
                return format!(
                    "{} {} {}",
                    self.num(sc, e),
                    self.r.pick(&["<", "<=", ">", ">=", "==", "~="]),
                    self.num(sc, e)
                )
            }
            1 => return format!("not ({})", self.boolean(sc, e)),
            2 => return format!("({} and {})", self.boolean(sc, e), self.boolean(sc, e)),
            3 => return format!("({} or {})", self.boolean(sc, e), self.boolean(sc, e)),
            4 => return format!("{} == {}", self.string(sc, e), self.string(sc, e)),
            5 => {
                let hs = self.helpers_of("bool");
                if !hs.is_empty() {
                    let h = self.r.pick(&hs);
                    return self.call(&h, sc, d);
                }
            }
            6 => {
                let maybes = of(sc, "maybe");
                if !maybes.is_empty() {
                    return format!("{} {} nil", self.r.pick(&maybes), self.r.pick(&["==", "~="]));
                }
            }
            _ => {}
        }
        format!("{} > {}", self.num(sc, e), self.num(sc, e))
    }

    fn string(&mut self, sc: &[Var], d: u32) -> String {
        let strs = of(sc, "str");
        let c = self.r.float();
        if d > 3 || c < 0.3 {
            if !strs.is_empty() && self.r.float() < 0.6 {
                return self.r.pick(&strs);
            }
            return format!("\"{}\"", self.r.pick(&["a", "bc", "x y", "", "hello", "%", "{q}"]));
        }
        let e = d + 1;
        match self.r.below(5) {
            0 => format!("{} .. {}", self.string(sc, e), self.string(sc, e)),
            1 => format!("`<{{{}}}:{{{}}}>`", self.num(sc, e), self.string(sc, e)),
            2 => format!("tostring({})", self.num(sc, e)),
            3 => format!("string.rep({}, {})", self.string(sc, d + 2), self.r.pick(&["0", "1", "2"])),
            _ => format!("(if {} then {} else {})", self.boolean(sc, e), self.string(sc, e), self.string(sc, e)),
        }
    }

    fn expr(&mut self, t: Ty, sc: &[Var]) -> String {
        match t {
            "num" => self.num(sc, 0),
            "bool" => self.boolean(sc, 0),
            _ => self.string(sc, 0),
        }
    }

    fn call(&mut self, h: &Helper, sc: &[Var], d: u32) -> String {
        let args: Vec<String> =
            h.params.iter().map(|t| if d < 3 { self.expr(t, sc) } else { lit(t).to_string() }).collect();
        format!("{}({})", h.name, args.join(", "))
    }

    // -- statements
    fn block(&mut self, ind: usize, sc: &[Var], budget: usize, in_loop: bool, in_func: Option<Ty>) {
        let mut sc = sc.to_vec();
        for _ in 0..budget {
            self.stmt(ind, &mut sc, in_loop, in_func);
        }
    }

    fn count(&mut self, lo: i64, hi: i64) -> usize {
        self.r.range(lo, hi) as usize
    }

    fn stmt(&mut self, ind: usize, sc: &mut Vec<Var>, in_loop: bool, in_func: Option<Ty>) {
        let k = self.r.below(26);
        let t = self.r.pick(&["num", "num", "bool", "str"]);
        let deep = ind + 1;
        if k <= 2 {
            let v = self.fresh("v");
            put!(self, ind, "local {v} = {}", self.expr(t, sc));
            sc.push((v, t));
        } else if k == 3 {
            let own: Vec<Var> = sc
                .iter()
                .filter(|(v, ty)| matches!(*ty, "num" | "bool" | "str") && own_num(v))
                .cloned()
                .collect();
            if !own.is_empty() {
                let (v, ty) = self.r.pick(&own);
                if ty == "num" && self.r.float() < 0.5 {
                    put!(self, ind, "{v} {} {}", self.r.pick(&["+=", "-=", "*="]), self.atom_num(sc));
                } else if ty == "str" && self.r.float() < 0.3 {
                    put!(self, ind, "{v} ..= {}", self.string(sc, 0));
                } else {
                    put!(self, ind, "{v} = {}", self.expr(ty, sc));
                }
            }
        } else if k <= 5 {
            put!(self, ind, "if {} then", self.boolean(sc, 0));
            let n = self.count(1, 4);
            self.block(deep, sc, n, in_loop, in_func);
            for _ in 0..self.r.below(3) {
                put!(self, ind, "elseif {} then", self.boolean(sc, 0));
                let n = self.count(1, 3);
                self.block(deep, sc, n, in_loop, in_func);
            }
            if self.r.float() < 0.5 {
                put!(self, ind, "else");
                let n = self.count(1, 3);
                self.block(deep, sc, n, in_loop, in_func);
            }
            put!(self, ind, "end");
        } else if k == 6 && ind < 4 {
            let i = self.fresh("i");
            put!(
                self,
                ind,
                "for {i} = {}, {}{} do",
                self.r.pick(&["1", "0", "3"]),
                self.r.pick(&["3", "4", "1"]),
                self.r.pick(&["", ", -1", ", 2"])
            );
            let body = with(sc, &[(&i, "num")]);
            let n = self.count(1, 4);
            self.block(deep, &body, n, true, in_func);
            put!(self, ind, "end");
        } else if k == 7 && ind < 4 {
            let arrs = of(sc, "arr");
            if !arrs.is_empty() {
                let (i, x) = (self.fresh("i"), self.fresh("x"));
                let wrapped = self.r.below(2) == 1;
                let a = self.r.pick(&arrs);
                let form = if wrapped { format!("ipairs({a})") } else { a };
                put!(self, ind, "for {i}, {x} in {form} do");
                let body = with(sc, &[(&i, "num"), (&x, "num")]);
                let n = self.count(1, 4);
                self.block(deep, &body, n, true, in_func);
                put!(self, ind, "end");
            }
        } else if k == 8 && ind < 4 {
            let c = self.fresh("c");
            put!(self, ind, "local {c} = 0");
            let body = with(sc, &[(&c, "num")]);
            if self.r.float() < 0.5 {
                put!(self, ind, "while {c} < {} do", self.r.pick(&["2", "3", "4"]));
                put!(self, deep, "{c} += 1");
                let n = self.count(1, 3);
                self.block(deep, &body, n, true, in_func);
                put!(self, ind, "end");
            } else {
                put!(self, ind, "repeat");
                put!(self, deep, "{c} += 1");
                let n = self.count(1, 3);
                self.block(deep, &body, n, true, in_func);
                put!(self, ind, "until {c} >= {}", self.r.pick(&["2", "3"]));
            }
        } else if k == 9 && in_loop {
            put!(self, ind, "if {} then {} end", self.boolean(sc, 0), self.r.pick(&["break", "continue"]));
        } else if k == 10 && in_func.is_some() {
            let ret = in_func.unwrap_or("num");
            put!(self, ind, "if {} then return {} end", self.boolean(sc, 0), self.expr(ret, sc));
        } else if k == 11 && !in_loop {
            // growing an array inside a loop may never end
            let arrs = of(sc, "arr");
            if !arrs.is_empty() {
                let a = self.r.pick(&arrs);
                let insert = format!("table.insert({a}, {})", self.num(sc, 0));
                let store = format!("{a}[{}] = {}", self.r.range(1, 4), self.num(sc, 0));
                let line = self.r.pick(&[insert, store]);
                self.emit(ind, line);
            }
        } else if k == 12 {
            let objs = of(sc, "obj");
            if !objs.is_empty() {
                let o = self.r.pick(&objs);
                let store = format!("{o}.x = {}", self.num(sc, 0));
                let bump = format!("{o}:bump({})", self.num(sc, 0));
                let line = self.r.pick(&[store, bump]);
                self.emit(ind, line);
            }
        } else if k == 13 {
            let v = self.fresh("v");
            put!(self, ind, "local {v} = if {} then {} else nil", self.boolean(sc, 0), self.num(sc, 0));
            sc.push((v.clone(), "maybe"));
            put!(self, ind, "print({v} or {})", self.num(sc, 0));
        } else if k == 14 {
            let ok = self.fresh("ok");
            put!(self, ind, "local {ok} = pcall(function()");
            put!(self, deep, "if {} then error(\"boom\") end", self.boolean(sc, 0));
            put!(self, ind, "end)");
            put!(self, ind, "print({ok})");
        } else if k == 15 {
            // several targets at once: values in temporaries
            let own: Vec<String> = of(sc, "num").into_iter().filter(|v| own_num(v)).collect();
            if own.len() >= 2 {
                let (a, b) = self.r.two(&own);
                let c = self.r.pick(&own);
                let swap = format!("{a}, {b} = {b}, {a}");
                let add = format!("{a}, {b} = {b}, {a} + {}", self.atom_num(sc));
                let both = format!("{a}, {b} = {b} * 2, {c} - 1");
                let line = self.r.pick(&[swap, add, both]);
                self.emit(ind, line);
                put!(self, ind, "print({a}, {b})");
            }
        } else if k == 16 {
            // a table a closure captures, built with calls inside
            let (v, h) = (self.fresh("t"), self.fresh("h"));
            put!(
                self,
                ind,
                "local {v} = {{{}, tostring({}), k = {}}}",
                self.num(sc, 0),
                self.num(sc, 0),
                self.num(sc, 0)
            );
            put!(self, ind, "local function {h}() return #{v}, {v}.k end");
            put!(self, ind, "print({h}())");
        } else if k == 17 {
            // all of a call's results, or of ...
            let v = self.fresh("t");
            put!(
                self,
                ind,
                "local {v} = {{{}, select(1, {}, {})}}",
                self.num(sc, 0),
                self.num(sc, 0),
                self.num(sc, 0)
            );
            put!(self, ind, "print(#{v})");
            let (f, x, y) = (self.fresh("f"), self.fresh("x"), self.fresh("x"));
            put!(self, ind, "local function {f}(...) local {x}, {y} = ... return {y}, {x}, select('#', ...) end");
            put!(self, ind, "print({f}({}, {}, {}))", self.num(sc, 0), self.num(sc, 0), self.num(sc, 0));
        } else if k == 18 {
            // a variable a closure changes, read around the call
            let (v, h) = (self.fresh("n"), self.fresh("h"));
            put!(self, ind, "local {v} = {}", self.num(sc, 0));
            put!(self, ind, "local function {h}() {v} += 1 return {v} end");
            let a = self.fresh("v");
            put!(self, ind, "local {a} = {v}");
            put!(self, ind, "print({a}, {h}(), {v}, {a} + {v})");
            sc.push((a, "num"));
        } else if k == 19 && ind < 4 {
            // order: both sides are calls
            self.string(sc, 0);
            let log = self.fresh("o");
            put!(self, ind, "local {log} = {{}}");
            let w = format!("(function(v) table.insert({log}, v) return v end)");
            let (x, y) = (self.num(sc, 0), self.num(sc, 0));
            let op = self.r.pick(&[">", ">=", "<", "<=", "==", "+", "-"]);
            put!(self, ind, "print({w}({x}) {op} {w}({y}), #{log}, {log}[1])");
        } else if k == 20 {
            // a value from before a branch, read in its arms
            let v = self.fresh("v");
            put!(self, ind, "local {v} = math.abs({})", self.num(sc, 0));
            put!(self, ind, "if {} then", self.boolean(sc, 0));
            put!(self, deep, "{v} = -{v}");
            put!(self, ind, "end");
            put!(self, ind, "print({v})");
            sc.push((v, "num"));
        } else if k == 21 && ind < 4 {
            // chains, with variables tested and then used
            let (a, b) = (self.fresh("v"), self.fresh("v"));
            put!(self, ind, "local {a} = if {} then {} else nil", self.boolean(sc, 0), self.num(sc, 0));
            put!(self, ind, "local {b} = if {} then {} else nil", self.boolean(sc, 0), self.num(sc, 0));
            put!(self, ind, "if {a} and {b} then");
            put!(self, deep, "print({a} + {b})");
            put!(self, ind, "else");
            put!(self, deep, "print({a} or {b} or {}, ({a} or {b}) and {})", self.num(sc, 0), self.num(sc, 0));
            put!(self, ind, "end");
        } else if k == 22 && in_func.is_none() && !in_loop {
            // a loop, then an else
            let i = self.fresh("i");
            put!(self, ind, "if {} then", self.boolean(sc, 0));
            put!(self, deep, "for {i} = 1, 2 do print({i}) end");
            put!(self, ind, "elseif {} then", self.boolean(sc, 0));
            put!(self, deep, "print({})", self.num(sc, 0));
            put!(self, ind, "end");
        } else if k == 23 {
            // a loop header with a computed iterator
            let arrs = of(sc, "arr");
            if !arrs.is_empty() && ind < 4 {
                let (i, x) = (self.fresh("i"), self.fresh("x"));
                let a = self.r.pick(&arrs);
                put!(
                    self,
                    ind,
                    "for {i}, {x} in ipairs(if {} then {a} else {{{}}}) do",
                    self.boolean(sc, 0),
                    self.num(sc, 0)
                );
                let body = with(sc, &[(&i, "num"), (&x, "num")]);
                self.block(deep, &body, 1, true, in_func);
                put!(self, ind, "end");
            }
        } else {
            let vals: Vec<String> =
                sc.iter().filter(|e| matches!(e.1, "num" | "bool" | "str")).map(|e| e.0.clone()).collect();
            let mut picks = Vec::new();
            if !vals.is_empty() {
                for _ in 0..vals.len().min(3) {
                    picks.push(self.r.pick(&vals));
                }
            }
            picks.push(self.expr(t, sc));
            put!(self, ind, "print({})", picks.join(", "));
        }
    }

    /// A small local function, which -O2 inlines, or a bigger one.
    fn helper(&mut self) {
        let name = self.fresh("f");
        let n = self.count(1, 4);
        let ptypes: Vec<Ty> = (0..n).map(|_| self.r.pick(&["num", "num", "bool", "str"])).collect();
        let params: Vec<Var> = ptypes.iter().map(|t| (self.fresh("p"), *t)).collect();
        let rt = self.r.pick(&["num", "num", "bool", "str"]);
        let list: Vec<&str> = params.iter().map(|p| p.0.as_str()).collect();
        put!(self, 0, "local function {name}({})", list.join(", "));
        if self.r.float() < 0.5 {
            put!(self, 1, "return {}", self.expr(rt, &params));
        } else {
            for _ in 0..self.count(1, 3) {
                put!(self, 1, "if {} then", self.boolean(&params, 0));
                put!(self, 2, "return {}", self.expr(rt, &params));
                put!(self, 1, "end");
            }
            if self.r.float() < 0.5 {
                let n = self.count(1, 3);
                self.block(1, &params, n, false, Some(rt));
            }
            put!(self, 1, "return {}", self.expr(rt, &params));
        }
        put!(self, 0, "end");
        self.helpers.push(Helper { name, params: ptypes, ret: rt });
    }

    fn program(mut self) -> String {
        for _ in 0..self.count(1, 4) {
            self.helper();
        }
        let a = self.fresh("arr");
        let n = self.count(1, 5);
        let items: Vec<String> = (0..n).map(|_| self.r.range(-5, 9).to_string()).collect();
        put!(self, 0, "local {a} = {{{}}}", items.join(", "));
        let o = self.fresh("obj");
        put!(self, 0, "local {o} = {{x = {}, y = {}}}", self.r.below(9), self.r.below(9));
        put!(self, 0, "function {o}:get() return self.x * 2 + self.y end");
        put!(self, 0, "function {o}:bump(n) self.x += n end");
        let sc: Vec<Var> = vec![(a.clone(), "arr"), (o.clone(), "obj")];
        let mk = self.fresh("mk");
        put!(self, 0, "local function {mk}(n)");
        put!(self, 1, "local count = 0");
        put!(self, 1, "return function(x) count += 1 return x + n * count end");
        put!(self, 0, "end");
        let adder = self.fresh("add");
        put!(self, 0, "local {adder} = {mk}({})", self.r.range(1, 5));
        let size = self.size;
        self.block(0, &sc, size, false, None);
        put!(self, 0, "print({adder}(1), {adder}(2), {o}.x, {o}.y, #{a})");
        put!(self, 0, "for i, x in {a} do print(i, x) end");
        self.lines.join("\n") + "\n"
    }
}
