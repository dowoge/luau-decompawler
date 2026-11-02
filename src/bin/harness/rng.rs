//! The random numbers behind the generated programs: a Mersenne Twister, seeded
//! and drawn exactly as the tests always have been, so a seed keeps naming the
//! same program and a failure stays reproducible.

const N: usize = 624;
const M: usize = 397;

pub struct Rng {
    state: [u32; N],
    at: usize,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let low = seed as u32;
        let high = (seed >> 32) as u32;
        let mut r = Rng { state: [0; N], at: N };
        if high == 0 {
            r.seed_by(&[low]);
        } else {
            r.seed_by(&[low, high]);
        }
        r
    }

    fn seed_by(&mut self, key: &[u32]) {
        let s = &mut self.state;
        s[0] = 19650218;
        for i in 1..N {
            s[i] = 1812433253u32.wrapping_mul(s[i - 1] ^ (s[i - 1] >> 30)).wrapping_add(i as u32);
        }
        let (mut i, mut j) = (1, 0);
        for _ in 0..N.max(key.len()) {
            s[i] = (s[i] ^ (s[i - 1] ^ (s[i - 1] >> 30)).wrapping_mul(1664525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                s[0] = s[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..N - 1 {
            s[i] = (s[i] ^ (s[i - 1] ^ (s[i - 1] >> 30)).wrapping_mul(1566083941)).wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                s[0] = s[N - 1];
                i = 1;
            }
        }
        s[0] = 0x8000_0000;
    }

    fn next_u32(&mut self) -> u32 {
        if self.at >= N {
            let s = &mut self.state;
            for k in 0..N {
                let y = (s[k] & 0x8000_0000) | (s[(k + 1) % N] & 0x7fff_ffff);
                let odd = if y & 1 != 0 { 0x9908_b0df } else { 0 };
                s[k] = s[(k + M) % N] ^ (y >> 1) ^ odd;
            }
            self.at = 0;
        }
        let mut y = self.state[self.at];
        self.at += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// In [0, 1).
    pub fn float(&mut self) -> f64 {
        let a = (self.next_u32() >> 5) as f64;
        let b = (self.next_u32() >> 6) as f64;
        (a * 67108864.0 + b) * (1.0 / 9007199254740992.0)
    }

    /// In [0, n).
    pub fn below(&mut self, n: usize) -> usize {
        let bits = usize::BITS - n.leading_zeros();
        loop {
            let r = (self.next_u32() >> (32 - bits)) as usize;
            if r < n {
                return r;
            }
        }
    }

    /// In [lo, hi).
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo) as usize) as i64
    }

    pub fn pick<T: Clone>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())].clone()
    }

    /// Two different items.
    pub fn two<T: Clone>(&mut self, items: &[T]) -> (T, T) {
        let n = items.len();
        // the small case swaps through a copy, the large one retries
        if n <= 21 {
            let mut pool = items.to_vec();
            let j = self.below(n);
            let first = pool[j].clone();
            pool[j] = pool[n - 1].clone();
            let k = self.below(n - 1);
            return (first, pool[k].clone());
        }
        let j = self.below(n);
        let mut k = self.below(n);
        while k == j {
            k = self.below(n);
        }
        (items[j].clone(), items[k].clone())
    }
}
