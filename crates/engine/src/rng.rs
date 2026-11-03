//! Seeded RNG for the turn kernel (T1-2/T1-6).
//!
//! **The original game's PRNG, runtime-verified (2026-11-03)**: a
//! Numerical-Recipes LCG `x = x·1664525 + 1013904223 (mod 2³²)` on the
//! global at `0x5dd53c` (constants adjacent in `.text` at RVA 0x554a4f;
//! the post-step value is used as a float in [0,1) via the
//! `0x4F000000`-exponent trick). Verification: writing `state = 1` into
//! the live game's global and later reading `0x0438e694` — exactly LCG
//! step 9 from 1 — proves game rolls flow through this recurrence
//! (evidence §T1-6).
//!
//! Callers that need to swap contexts (the original saves/restores the
//! global around per-context roll batches — contexts keep their own
//! state at `ctx+0x100`) can do so via [`GameRng::set_state`].

/// The turn kernel's randomness source. Deterministic given the seed and
/// the sequence of calls.
pub trait GameRng {
    /// Next raw 32-bit draw.
    fn next_u32(&mut self) -> u32;
    /// Uniform draw in `0..n` (n > 0). Rounding via rejection-free
    /// multiply-shift; documented so parity work can match this mapping.
    fn next_below(&mut self, n: u32) -> u32 {
        if n <= 1 {
            return 0;
        }
        ((self.next_u32() as u64 * n as u64) >> 32) as u32
    }
    /// Number of `next_u32` draws so far — lets tests assert that two
    /// runs consume randomness identically.
    fn draws(&self) -> u64;
    /// Direct state access for context swapping (mirrors the original's
    /// save/swap/roll/restore pattern around `0x5dd53c`).
    fn set_state(&mut self, state: u32);
    fn state(&self) -> u32;
}

/// The original game's generator: Numerical-Recipes 32-bit LCG
/// (`x = x·1664525 + 1013904223 mod 2³²`), raw state out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrigRng {
    state: u32,
    draws: u64,
}

impl OrigRng {
    pub fn new(seed: u32) -> Self {
        OrigRng { state: seed, draws: 0 }
    }
}

impl GameRng for OrigRng {
    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        self.draws += 1;
        self.state
    }
    fn draws(&self) -> u64 {
        self.draws
    }
    fn set_state(&mut self, state: u32) {
        self.state = state;
    }
    fn state(&self) -> u32 {
        self.state
    }
}

/// Compatibility alias for the pre-T1-6 placeholder name.
pub type Lcg64 = OrigRng;

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact oracle sequence: state 1 advanced 9 times = 0x0438e694
    /// (read from the live game after seeding 0x5dd53c = 1).
    #[test]
    fn reproduces_oracle_step9() {
        let mut r = OrigRng::new(1);
        for _ in 0..9 {
            r.next_u32();
        }
        assert_eq!(r.state(), 0x0438e694);
    }

    #[test]
    fn deterministic_given_seed() {
        let mut a = OrigRng::new(12345);
        let mut b = OrigRng::new(12345);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        assert_eq!(a.draws(), 100);
    }

    #[test]
    fn next_below_stays_in_range() {
        let mut r = OrigRng::new(7);
        for _ in 0..1000 {
            assert!(r.next_below(25) < 25);
        }
    }
}
