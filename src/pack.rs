//! Texts held between stages, packed: a place's decompiled scripts are tens
//! of megabytes of Lua, kept from the first stage to the last, and Lua packs
//! to a fraction of that. LZ77 with a 64 KB window, in sequences of
//! `token, literals, offset, more length`: the token's high half is the count
//! of literals, its low half the match length less four, 15 meaning more
//! bytes follow (each adding up to 255). The last sequence has no match.

const MIN: usize = 4;
const WINDOW: usize = 0xFFFF;
const HASH_BITS: u32 = 14;

#[inline]
fn hash(b: &[u8], i: usize) -> usize {
    let v = u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    (v.wrapping_mul(2654435761) >> (32 - HASH_BITS)) as usize
}

fn length(out: &mut Vec<u8>, mut n: usize) {
    while n >= 255 {
        out.push(255);
        n -= 255;
    }
    out.push(n as u8);
}

fn sequence(out: &mut Vec<u8>, literals: &[u8], offset: usize, matched: usize) {
    let lit = literals.len();
    let m = matched.saturating_sub(MIN);
    let token = ((lit.min(15) as u8) << 4) | if matched == 0 { 0 } else { m.min(15) as u8 };
    out.push(token);
    if lit >= 15 {
        length(out, lit - 15);
    }
    out.extend_from_slice(literals);
    if matched == 0 {
        return;
    }
    out.extend_from_slice(&(offset as u16).to_le_bytes());
    if m >= 15 {
        length(out, m - 15);
    }
}

/// The bytes packed.
pub fn pack(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 3 + 16);
    let mut table = vec![usize::MAX; 1 << HASH_BITS];
    let (mut i, mut anchor) = (0, 0);
    while i + MIN <= data.len() {
        let h = hash(data, i);
        let cand = table[h];
        table[h] = i;
        if cand != usize::MAX && i - cand <= WINDOW && data[cand..cand + MIN] == data[i..i + MIN] {
            let mut n = MIN;
            while i + n < data.len() && data[cand + n] == data[i + n] {
                n += 1;
            }
            sequence(&mut out, &data[anchor..i], i - cand, n);
            // the positions the match covers, for later matches to find
            let end = i + n;
            let mut k = i + 1;
            while k + MIN <= data.len() && k < end {
                table[hash(data, k)] = k;
                k += 1;
            }
            i = end;
            anchor = i;
        } else {
            i += 1;
        }
    }
    sequence(&mut out, &data[anchor..], 0, 0);
    out
}

fn read_length(p: &[u8], at: &mut usize, mut n: usize) -> Option<usize> {
    loop {
        let b = *p.get(*at)?;
        *at += 1;
        n += b as usize;
        if b != 255 {
            return Some(n);
        }
    }
}

/// What `pack` was given, `len` bytes of it; None if the bytes are not
/// something it made.
pub fn unpack(p: &[u8], len: usize) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(len);
    let mut at = 0;
    while at < p.len() {
        let token = p[at];
        at += 1;
        let mut lit = (token >> 4) as usize;
        if lit == 15 {
            lit = read_length(p, &mut at, lit)?;
        }
        out.extend_from_slice(p.get(at..at + lit)?);
        at += lit;
        if at == p.len() {
            break; // the last sequence: literals only
        }
        let offset = u16::from_le_bytes([*p.get(at)?, *p.get(at + 1)?]) as usize;
        at += 2;
        let mut m = (token & 15) as usize;
        if m == 15 {
            m = read_length(p, &mut at, m)?;
        }
        let n = m + MIN;
        if offset == 0 || offset > out.len() {
            return None;
        }
        let from = out.len() - offset;
        for k in 0..n {
            let b = out[from + k]; // a match may run into what it is copying
            out.push(b);
        }
    }
    (out.len() == len).then_some(out)
}

/// A text kept packed.
pub struct Packed {
    len: usize,
    bytes: Vec<u8>,
}

impl Packed {
    pub fn new(text: &str) -> Packed {
        Packed { len: text.len(), bytes: pack(text.as_bytes()) }
    }

    /// (unpacked, packed) sizes.
    pub fn sizes(&self) -> (usize, usize) {
        (self.len, self.bytes.len())
    }

    /// The text again.
    pub fn text(&self) -> String {
        let bytes = unpack(&self.bytes, self.len).expect("a text packed here unpacks");
        String::from_utf8(bytes).expect("a text packed here is text")
    }

    /// A copy, its bytes where new blocks come from now (out of a job).
    pub fn copy(&self) -> Packed {
        Packed { len: self.len, bytes: self.bytes.as_slice().to_vec() }
    }
}

#[cfg(test)]
mod tests {
    use super::{pack, unpack};

    fn round(data: &[u8]) {
        let p = pack(data);
        assert_eq!(unpack(&p, data.len()).as_deref(), Some(data), "{} bytes", data.len());
    }

    #[test]
    fn texts_come_back_as_they_were() {
        round(b"");
        round(b"a");
        round(b"abcd");
        round(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let lua = "local function f(x)\n    return x + 1\nend\n".repeat(400);
        round(lua.as_bytes());
        // long literal runs, long matches, offsets near the window
        let mut s = String::new();
        let mut v: u64 = 12345;
        for i in 0..200_000 {
            v = v.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            s.push((b'a' + (v >> 59) as u8 % 26) as char);
            if i % 997 == 0 {
                s.push_str(&"x".repeat(300));
            }
        }
        round(s.as_bytes());
        let mut far = s.clone();
        far.push_str(&s[s.len() - 70_000..s.len() - 60_000]);
        round(far.as_bytes());
    }

    #[test]
    fn what_was_not_packed_here_is_refused() {
        assert_eq!(unpack(&[0x10], 5), None);
        assert_eq!(unpack(&[0x04, 1, 0], 4), None);
    }
}
