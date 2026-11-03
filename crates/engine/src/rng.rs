//! Seeded RNG for the turn kernel (T1-2/T1-6).
//!
//! **The original game's PRNG, runtime-verified (2026-11-03)**: a
//! Numerical-Recipes LCG `x = x·1664525 + 1013904223 (mod 2³²)` on the
//! global at `0x5dd53c` (constants adjacent in `.text`; the post-step
//! value is used as a float in [0,1]). Verification: writing `state = 1`
//! into the live game's global and later reading `0x0438e694` — exactly
//! LCG step 9 from 1 — proves game rolls flow through this recurrence
//! (evidence §T1-6).
//!
//! The **distribution** is also decoded from the binary: every gameplay
//! roll goes through one function, `rand_int(lo, hi)` at VA `0x554aa0`,
//! which is
//!
//! ```text
//! span = hi - lo
//! return lo + trunc( (state / 4294967295.0) * (span + 0.999999) )
//! ```
//!
//! (`state` = the post-step LCG value; `0.999999` is a `double` constant
//! at VA `0x5c62f0`; the truncation is the float→int helper at VA
//! `0x5b4f70`). It is **inclusive** of `hi`. A degenerate range `lo == hi`
//! still burns one draw and returns `lo`. [`GameRng::next_between`]
//! implements exactly this, and [`GameRng::next_below`] is `0..n-1`, so
//! engine draws match the original draw-for-draw.
//!
//! Callers that need to swap contexts (the original saves/restores the
//! global around per-context roll batches — contexts keep their own
//! state at `ctx+0x100`; the exchange primitive is VA `0x554a00`) can do
//! so via [`GameRng::set_state`] / [`GameRng::state`].

/// The turn kernel's randomness source. Deterministic given the seed and
/// the sequence of calls.
pub trait GameRng {
    /// Next raw 32-bit draw (the post-step LCG state).
    fn next_u32(&mut self) -> u32;
    /// The original's unit float: `state / 4294967295.0`, in `[0, 1]`.
    fn next_unit(&mut self) -> f64 {
        self.next_u32() as f64 / u32::MAX as f64
    }
    /// The original's `rand_int(lo, hi)` — **inclusive**, and exactly the
    /// mapping at VA 0x554aa0. A degenerate range keeps the "burn one
    /// draw" behaviour of the original.
    fn next_between(&mut self, lo: i32, hi: i32) -> i32 {
        if lo == hi {
            self.next_u32();
            return lo;
        }
        let (lo, hi) = if hi < lo { (hi, lo) } else { (lo, hi) };
        let span = (hi as i64 - lo as i64) as f64;
        // 0.999999 is the game's own double constant (VA 0x5c62f0).
        let f = self.next_unit() * (span + 0.999_999);
        lo + f as i32
    }
    /// Uniform draw in `0..n` (n > 0), i.e. `next_between(0, n-1)` —
    /// draw-for-draw identical to the original.
    fn next_below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        self.next_between(0, n as i32 - 1) as u32
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
        OrigRng {
            state: seed,
            draws: 0,
        }
    }
    /// The original's context-swap primitive (VA 0x554a00): store `v` in
    /// the slot and return the value that was there. Used around
    /// per-context roll batches.
    pub fn exchange_state(&mut self, v: u32) -> u32 {
        std::mem::replace(&mut self.state, v)
    }
}

impl GameRng for OrigRng {
    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
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

    /// `rand_int` is inclusive at both ends and reproduces the binary's
    /// arithmetic exactly for known states.
    #[test]
    fn next_between_matches_binary_formula() {
        let mut r = OrigRng::new(1);
        // Independent restatement of VA 0x554aa0 for the same state
        // stream: lo + trunc(unit * (span + 0.999999)).
        let mut s = 1u32;
        let mut expect = |lo: i32, hi: i32| {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            let span = (hi - lo) as f64;
            lo + ((s as f64 / u32::MAX as f64) * (span + 0.999_999)) as i32
        };
        for (lo, hi) in [(0, 0), (0, 1), (0, 9), (1, 6), (3, 3), (0, 100)] {
            assert_eq!(r.next_between(lo, hi), expect(lo, hi), "range {lo}..={hi}");
        }
    }

    /// A degenerate range still consumes a draw (the original's quirk) —
    /// callers depend on the draw count, not just the value.
    #[test]
    fn degenerate_range_burns_a_draw() {
        let mut r = OrigRng::new(1);
        assert_eq!(r.next_between(4, 4), 4);
        assert_eq!(r.draws(), 1);
    }

    /// Swapping arguments is allowed (the original normalises the pair).
    #[test]
    fn reversed_bounds_are_normalised() {
        let mut a = OrigRng::new(99);
        let mut b = OrigRng::new(99);
        assert_eq!(a.next_between(7, 2), b.next_between(2, 7));
    }
}
