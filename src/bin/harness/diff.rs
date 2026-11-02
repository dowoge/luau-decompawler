//! Longest-matching-block sequence comparison (Ratcliff and Obershelp): find the
//! longest common run, then recurse on what lies left and right of it.

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tag {
    Equal,
    Replace,
    Delete,
    Insert,
}

pub struct Matcher<'a, T> {
    a: &'a [T],
    b: &'a [T],
    /// Where each item of b occurs, in order.
    places: HashMap<&'a T, Vec<usize>>,
}

impl<'a, T: Eq + Hash> Matcher<'a, T> {
    pub fn new(a: &'a [T], b: &'a [T]) -> Matcher<'a, T> {
        let mut places: HashMap<&'a T, Vec<usize>> = HashMap::new();
        for (j, x) in b.iter().enumerate() {
            places.entry(x).or_default().push(j);
        }
        Matcher { a, b, places }
    }

    /// (start in a, start in b, length). Of the longest runs, the one starting
    /// earliest in a, and of those the one starting earliest in b.
    fn longest(&self, alo: usize, ahi: usize, blo: usize, bhi: usize, runs: &mut [Vec<u32>; 2]) -> (usize, usize, usize) {
        let (mut besti, mut bestj, mut bestk) = (alo, blo, 0usize);
        // runs[0][j + 1] is the length of the run ending at b[j] and the item
        // of a before this one; touched says what to clear after each item
        let mut touched: Vec<usize> = Vec::new();
        let mut before: Vec<usize> = Vec::new();
        let (mut prev, mut next) = (0, 1);
        for i in alo..ahi {
            if let Some(js) = self.places.get(&self.a[i]) {
                for &j in js {
                    if j < blo {
                        continue;
                    }
                    if j >= bhi {
                        break;
                    }
                    let k = runs[prev][j] as usize + 1;
                    runs[next][j + 1] = k as u32;
                    touched.push(j + 1);
                    if k > bestk {
                        (besti, bestj, bestk) = (i + 1 - k, j + 1 - k, k);
                    }
                }
            }
            for &t in &before {
                runs[prev][t] = 0;
            }
            std::mem::swap(&mut before, &mut touched);
            touched.clear();
            std::mem::swap(&mut prev, &mut next);
        }
        for &t in &before {
            runs[prev][t] = 0;
        }
        (besti, bestj, bestk)
    }

    /// The matching runs in order, adjacent ones joined, ending with an empty one.
    pub fn blocks(&self) -> Vec<(usize, usize, usize)> {
        let (la, lb) = (self.a.len(), self.b.len());
        let mut runs = [vec![0u32; lb + 2], vec![0u32; lb + 2]];
        let mut found = Vec::new();
        let mut queue = vec![(0, la, 0, lb)];
        while let Some((alo, ahi, blo, bhi)) = queue.pop() {
            let (i, j, k) = self.longest(alo, ahi, blo, bhi, &mut runs);
            if k > 0 {
                found.push((i, j, k));
                if alo < i && blo < j {
                    queue.push((alo, i, blo, j));
                }
                if i + k < ahi && j + k < bhi {
                    queue.push((i + k, ahi, j + k, bhi));
                }
            }
        }
        found.sort_unstable();
        let mut joined: Vec<(usize, usize, usize)> = Vec::with_capacity(found.len() + 1);
        for (i, j, k) in found {
            match joined.last_mut() {
                Some(l) if l.0 + l.2 == i && l.1 + l.2 == j => l.2 += k,
                _ => joined.push((i, j, k)),
            }
        }
        joined.push((la, lb, 0));
        joined
    }

    /// (what, a from, a to, b from, b to): the steps from a to b.
    pub fn opcodes(&self) -> Vec<(Tag, usize, usize, usize, usize)> {
        let (mut i, mut j) = (0, 0);
        let mut out = Vec::new();
        for (ai, bj, size) in self.blocks() {
            let tag = match (i < ai, j < bj) {
                (true, true) => Some(Tag::Replace),
                (true, false) => Some(Tag::Delete),
                (false, true) => Some(Tag::Insert),
                (false, false) => None,
            };
            if let Some(tag) = tag {
                out.push((tag, i, ai, j, bj));
            }
            (i, j) = (ai + size, bj + size);
            if size > 0 {
                out.push((Tag::Equal, ai, i, bj, j));
            }
        }
        out
    }

    fn of(&self, matches: usize) -> f64 {
        let total = self.a.len() + self.b.len();
        if total == 0 {
            1.0
        } else {
            2.0 * matches as f64 / total as f64
        }
    }

    pub fn ratio(&self) -> f64 {
        self.of(self.blocks().iter().map(|b| b.2).sum())
    }

    /// An upper bound on ratio: the items in common, in any order.
    pub fn quick_ratio(&self) -> f64 {
        let mut left: HashMap<&T, usize> = self.places.iter().map(|(k, v)| (*k, v.len())).collect();
        let mut matches = 0;
        for x in self.a {
            if let Some(n) = left.get_mut(x) {
                if *n > 0 {
                    *n -= 1;
                    matches += 1;
                }
            }
        }
        self.of(matches)
    }

    /// A cheaper upper bound: the lengths alone.
    pub fn real_quick_ratio(&self) -> f64 {
        self.of(self.a.len().min(self.b.len()))
    }
}
