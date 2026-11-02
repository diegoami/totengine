//! Seeded RNG abstraction for the turn kernel (T1-2).
//!
//! The original game's PRNG identity is **unknown** — its `.text` contains
//! none of the classic LCG constants (the ones found are in ToTPP's
//! appended code, not the original game; see `docs/evidence/README.md`
//! §"RNG / seed state"). [`Lcg64`] is therefore an explicit *placeholder*:
//! deterministic and seedable, to be swapped for the original generator
//! when T1-6 pins it. Everything downstream depends only on [`GameRng`],
//! so the swap is a one-site change.

/// The turn kernel's randomness source. Deterministic given the seed and
/// the sequence of calls — the contract T1-6 must eventually satisfy with
/// the original game's generator.
pub trait GameRng {
    /// Next raw 32-bit draw.
    fn next_u32(&mut self) -> u32;
    /// Uniform draw in `0..n` (n > 0). Rounding via rejection-free
    /// multiply-shift: good enough for gameplay parity work; documented
    /// so a future generator can match this exact mapping or replace it.
    fn next_below(&mut self, n: u32) -> u32 {
        if n <= 1 {
            return 0;
        }
        ((self.next_u32() as u64 * n as u64) >> 32) as u32
    }
    /// Number of `next_u32` draws so far — lets tests assert that two
    /// runs consume randomness identically without peeking at values.
    fn draws(&self) -> u64;
}

/// Placeholder generator: Knuth's MMIX 64-bit LCG
/// (`x = 6364136223846793005*x + 1442695040888963407`), high 32 bits out.
/// Chosen for std-only simplicity and well-understood behavior; **not**
/// the original game's PRNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lcg64 {
    state: u64,
    draws: u64,
}

impl Lcg64 {
    pub fn new(seed: u64) -> Self {
        Lcg64 {
            state: seed,
            draws: 0,
        }
    }
}

impl GameRng for Lcg64 {
    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.draws += 1;
        (self.state >> 32) as u32
    }
    fn draws(&self) -> u64 {
        self.draws
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_given_seed() {
        let mut a = Lcg64::new(42);
        let mut b = Lcg64::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        let mut c = Lcg64::new(43);
        assert_ne!(a.next_u32(), c.next_u32());
    }

    #[test]
    fn next_below_stays_in_range() {
        let mut r = Lcg64::new(7);
        for _ in 0..1000 {
            assert!(r.next_below(10) < 10);
        }
        assert_eq!(r.next_below(1), 0);
        assert!(r.draws() > 0);
    }
}
