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


/// Canonical Huffman over bytes, after LZ77: the 256 code lengths (four bits
/// each) up front, then the byte count, then the codes, most significant bit
/// first. Decompiled Lua is mostly names and constants, and LZ77 alone keeps
/// it at five eighths; with this, half.
mod huff {
    const MAX: usize = 15;
    const FAST: usize = 11;

    /// Code lengths of at most MAX bits for these counts (0: unused).
    fn lengths(freq: &[u64; 256]) -> [u8; 256] {
        let mut f = *freq;
        loop {
            let mut used: Vec<usize> = (0..256).filter(|s| f[*s] > 0).collect();
            let mut len = [0u8; 256];
            match used.len() {
                0 => return len,
                1 => {
                    len[used[0]] = 1;
                    return len;
                }
                _ => {}
            }
            used.sort_by_key(|s| (f[*s], *s));
            // two queues: leaves by weight, and merged nodes in the order made
            let n = used.len();
            let mut weight: Vec<u64> = used.iter().map(|s| f[*s]).collect();
            let mut parent: Vec<usize> = vec![usize::MAX; 2 * n - 1];
            let (mut leaf, mut node) = (0, n);
            let take = |weight: &Vec<u64>, leaf: &mut usize, node: &mut usize, made: usize| -> usize {
                if *leaf < n && (*node >= made || weight[*leaf] <= weight[*node]) {
                    *leaf += 1;
                    *leaf - 1
                } else {
                    *node += 1;
                    *node - 1
                }
            };
            for made in n..2 * n - 1 {
                let a = take(&weight, &mut leaf, &mut node, made);
                let b = take(&weight, &mut leaf, &mut node, made);
                weight.push(weight[a] + weight[b]);
                parent[a] = made;
                parent[b] = made;
            }
            // depths, from the root down: a parent is made after its children
            let mut depth = vec![0usize; 2 * n - 1];
            for k in (0..2 * n - 2).rev() {
                depth[k] = depth[parent[k]] + 1;
            }
            if depth[..n].iter().all(|d| *d <= MAX) {
                for (k, s) in used.iter().enumerate() {
                    len[*s] = depth[k] as u8;
                }
                return len;
            }
            // too deep: flatten the counts and build again
            for x in f.iter_mut().filter(|x| **x > 0) {
                *x = (*x >> 1).max(1);
            }
        }
    }

    /// Canonical codes: by length, then by byte.
    fn codes(len: &[u8; 256]) -> [u16; 256] {
        let mut count = [0u16; MAX + 1];
        for l in len.iter().filter(|l| **l > 0) {
            count[*l as usize] += 1;
        }
        let mut next = [0u16; MAX + 2];
        let mut code = 0u16;
        for l in 1..=MAX {
            code = (code + count[l - 1]) << 1;
            next[l] = code;
        }
        let mut out = [0u16; 256];
        for s in 0..256 {
            let l = len[s] as usize;
            if l > 0 {
                out[s] = next[l];
                next[l] += 1;
            }
        }
        out
    }

    pub fn encode(data: &[u8]) -> Vec<u8> {
        let mut freq = [0u64; 256];
        for b in data {
            freq[*b as usize] += 1;
        }
        let len = lengths(&freq);
        let code = codes(&len);
        let mut out = Vec::with_capacity(132 + data.len() * 5 / 8);
        for pair in len.chunks(2) {
            out.push(pair[0] << 4 | pair[1]);
        }
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        let (mut acc, mut bits) = (0u64, 0u32);
        for b in data {
            let l = len[*b as usize] as u32;
            acc = (acc << l) | code[*b as usize] as u64;
            bits += l;
            while bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
            }
        }
        if bits > 0 {
            out.push((acc << (8 - bits)) as u8);
        }
        out
    }

    pub fn decode(p: &[u8]) -> Option<Vec<u8>> {
        let mut len = [0u8; 256];
        for (k, b) in p.get(..128)?.iter().enumerate() {
            len[2 * k] = b >> 4;
            len[2 * k + 1] = b & 15;
        }
        let n = u32::from_le_bytes(p.get(128..132)?.try_into().ok()?) as usize;
        let code = codes(&len);
        // codes up to FAST bits by a table; longer ones by their length
        let mut fast = vec![(0u8, 0u8); 1 << FAST];
        let mut by_len: Vec<(u8, u16, u8)> = Vec::new(); // (length, code, byte)
        for s in 0..256 {
            let l = len[s] as usize;
            if l == 0 {
                continue;
            }
            if l <= FAST {
                let from = (code[s] as usize) << (FAST - l);
                for e in &mut fast[from..from + (1 << (FAST - l))] {
                    *e = (s as u8, l as u8);
                }
            } else {
                by_len.push((l as u8, code[s], s as u8));
            }
        }
        let mut out = Vec::with_capacity(n);
        let bytes = &p[132..];
        let (mut acc, mut bits, mut at) = (0u64, 0u32, 0usize);
        while out.len() < n {
            while bits <= 56 && at < bytes.len() {
                acc = (acc << 8) | bytes[at] as u64;
                at += 1;
                bits += 8;
            }
            // the next FAST bits, zeros past the end
            let peek = if bits as usize >= FAST {
                (acc >> (bits as usize - FAST)) as usize & ((1 << FAST) - 1)
            } else {
                ((acc << (FAST - bits as usize)) as usize) & ((1 << FAST) - 1)
            };
            let (sym, l) = fast[peek];
            if l > 0 {
                if l as u32 > bits {
                    return None;
                }
                bits -= l as u32;
                out.push(sym);
                continue;
            }
            let mut found = None;
            for &(l, c, s) in &by_len {
                if l as u32 <= bits && (acc >> (bits - l as u32)) as u16 & ((1u32 << l) - 1) as u16 == c {
                    found = Some((l, s));
                    break;
                }
            }
            let (l, s) = found?;
            bits -= l as u32;
            out.push(s);
        }
        Some(out)
    }
}

/// A text kept packed.
pub struct Packed {
    len: usize,
    bytes: Vec<u8>,
}

impl Packed {
    pub fn new(text: &str) -> Packed {
        Packed { len: text.len(), bytes: huff::encode(&pack(text.as_bytes())) }
    }

    /// (unpacked, packed) sizes.
    pub fn sizes(&self) -> (usize, usize) {
        (self.len, self.bytes.len())
    }

    /// The text again.
    pub fn text(&self) -> String {
        let lz = huff::decode(&self.bytes).expect("a text packed here decodes");
        let bytes = unpack(&lz, self.len).expect("a text packed here unpacks");
        String::from_utf8(bytes).expect("a text packed here is text")
    }

    /// A copy, its bytes where new blocks come from now (out of a job).
    pub fn copy(&self) -> Packed {
        Packed { len: self.len, bytes: self.bytes.as_slice().to_vec() }
    }
}

#[cfg(test)]
mod tests {
    use super::{huff, pack, unpack, Packed};

    #[test]
    fn huffman_codes_come_back() {
        for data in [
            Vec::new(),
            vec![7u8],
            vec![7u8; 1000],
            (0..=255u8).collect::<Vec<u8>>(),
            (0..100_000u32).map(|i| (i % 7 + (i % 13) * (i % 3)) as u8).collect(),
            // counts of 1, 2, 4, 8, ... deepen the tree past fifteen bits
            (0..20u32).flat_map(|k| std::iter::repeat(k as u8).take(1 << k.min(19))).collect(),
        ] {
            let e = huff::encode(&data);
            assert_eq!(huff::decode(&e).as_deref(), Some(&data[..]), "{} bytes", data.len());
        }
    }

    #[test]
    fn packed_texts_come_back() {
        let lua = "local function f(x)\n    return x + 1\nend\n".repeat(50) + "print(\"ü\")";
        assert_eq!(Packed::new(&lua).text(), lua);
        assert_eq!(Packed::new("").text(), "");
    }

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
