//! HashMap and HashSet with a quicker hash. The standard one defends against
//! keys chosen to collide, which costs most of what hashing costs here; the
//! worst such keys could do to this program is slow it down.

use std::hash::{BuildHasherDefault, Hasher};

pub type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<Quick>>;
pub type HashSet<K> = std::collections::HashSet<K, BuildHasherDefault<Quick>>;

const MIX: u64 = 0xf135_7aea_2e62_a9c5;

/// A word as a number, for what only asks whether it has seen a word: no copy
/// to make, no text to compare, and two words alike one time in 2^64.
pub fn key(word: &[u8]) -> u64 {
    let mut h = Quick::default();
    h.write(word);
    h.0
}

#[derive(Default, Clone, Copy)]
pub struct Quick(u64);

impl Quick {
    #[inline]
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(MIX);
    }
}

#[inline]
fn u32_at(b: &[u8], at: usize) -> u64 {
    u64::from(u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]))
}

impl Hasher for Quick {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut words = bytes.chunks_exact(8);
        for w in &mut words {
            self.add(u64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]));
        }
        // the rest in two reads that may overlap: what is read depends on the
        // length, and str hashing ends with a marker byte anyway
        let r = words.remainder();
        let n = r.len();
        if n >= 4 {
            self.add(u32_at(r, 0) | u32_at(r, n - 4) << 32);
        } else if n > 0 {
            self.add(u64::from(r[0]) | u64::from(r[n / 2]) << 8 | u64::from(r[n - 1]) << 16);
        }
    }

    #[inline]
    fn write_u8(&mut self, n: u8) {
        self.add(u64::from(n));
    }

    #[inline]
    fn write_u32(&mut self, n: u32) {
        self.add(u64::from(n));
    }

    #[inline]
    fn write_u64(&mut self, n: u64) {
        self.add(n);
    }

    #[inline]
    fn write_usize(&mut self, n: usize) {
        self.add(n as u64);
    }

    /// The table takes its position from the low bits and its tag from the top
    /// ones, and multiplying leaves the low bits the weakest.
    #[inline]
    fn finish(&self) -> u64 {
        self.0.rotate_left(26)
    }
}
