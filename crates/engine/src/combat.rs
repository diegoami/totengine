//! Combat resolution (T1-6), decoded from the original and replayed
//! bit-exactly against three oracle battles (evidence §T1-6 combat).
//!
//! **Combat does not use the `rand_int` generator in [`crate::rng`].** Its
//! rolls come from the statically linked MSVC CRT `rand()` at VA
//! `0x5b11d0` (`srand` at `0x5b11b0`), state at VA `0x5e2590`:
//!
//! ```text
//! state = state·214013 + 2531011 (mod 2³²);  return (state >> 16) & 0x7fff
//! ```
//!
//! The round loop lives in the combat function at VA `0x541425`
//! (`combat(attacker, dir, real)`; `real = 0` is the AI's odds probe,
//! which returns `min(A·8 / (D+1), 0x400)` and rolls nothing):
//!
//! ```text
//! while attacker alive && defender alive:          // hp·10 − damage > 0
//!     a = A > 1 ? rand() % A : 0                   // VA 0x543b3d
//!     d = D > 1 ? rand() % D : 0                   // VA 0x543b6a
//!     attacker_wins = a > d                        // strict
//!     if reroll && attacker_wins:                  // VA 0x543bdc
//!         if (A>1 ? rand()%A : 0) < (D>1 ? rand()%D : 0): attacker_wins = false
//!     if attacker_wins: defender.damage += attacker_fp
//!     else:             attacker.damage += defender_fp
//! one further rand() after the last round (purpose not yet decoded)
//! ```
//!
//! What is decoded here is the **round loop** and the **attack strength**.
//! The defence strength (`0x53f67a`) is only partly decoded — it calls
//! ToTPP hooks for terrain/fortification — so it is an input, not computed.

/// The MSVC CRT `rand()` the original uses for combat (VA 0x5b11d0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrtRand {
    state: u32,
    draws: u64,
}

impl CrtRand {
    /// `srand(seed)` (VA 0x5b11b0) — stores the seed verbatim.
    pub fn new(seed: u32) -> Self {
        CrtRand {
            state: seed,
            draws: 0,
        }
    }
    /// One `rand()`: 15-bit result, `0..=0x7fff`.
    pub fn rand(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(214013).wrapping_add(2531011);
        self.draws += 1;
        (self.state >> 16) & 0x7fff
    }
    pub fn state(&self) -> u32 {
        self.state
    }
    pub fn draws(&self) -> u64 {
        self.draws
    }
    /// `rand() % n` as the combat loop takes it: skipped (0, no draw) when
    /// `n <= 1`.
    fn roll(&mut self, n: i32) -> i32 {
        if n - 1 > 0 {
            (self.rand() as i32) % n
        } else {
            0
        }
    }
}

/// Unit attribute bit (record word at +6) that marks a veteran.
pub const ATTR_VETERAN: u16 = 0x2000;

/// The original's attack strength (VA 0x53f604): `attack·8`, ×1.5 for a
/// veteran (attr `0x2000`), ×1.5 again when attr bit `0x10` is set (its
/// meaning is still open). Integer halving as in the original
/// (`x += x >> 1`).
///
/// The combat function then applies context modifiers that are **not**
/// in here: partial-move scaling (`A·moves_left / move_unit`), and when the
/// defender is a barbarian, ×2 if the attacker's civ has wonder 6 (the
/// case in all three oracle battles: A = 48 → 96).
pub fn attack_strength(attack: u8, attr: u16) -> i32 {
    let mut a = attack as i32 * 8;
    if attr & ATTR_VETERAN != 0 {
        a += a >> 1;
    }
    if attr & 0x10 != 0 {
        a += a >> 1;
    }
    a
}

/// One side of a battle, as the round loop sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fighter {
    /// Final strength (after every modifier).
    pub strength: i32,
    /// Hit points ×10, i.e. the Rules `h` column times 10 (record +0x0e of
    /// the unit type).
    pub hp: i32,
    /// Firepower: damage dealt per won round.
    pub firepower: i32,
    /// Damage already taken (unit record +0x0c).
    pub damage: i32,
}

impl Fighter {
    fn alive(&self) -> bool {
        self.hp - self.damage > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub attacker_damage: i32,
    pub defender_damage: i32,
    pub attacker_won: bool,
    pub rounds: u32,
}

/// Fight to the death exactly as the original's round loop does, including
/// the trailing draw. `reroll` is the defender-favouring second roll the
/// original enables in a few barbarian/difficulty cases.
pub fn resolve(rng: &mut CrtRand, mut att: Fighter, mut def: Fighter, reroll: bool) -> Outcome {
    let mut rounds = 0;
    while att.alive() && def.alive() {
        let a = rng.roll(att.strength);
        let d = rng.roll(def.strength);
        let mut attacker_wins = a > d;
        if reroll && attacker_wins {
            let a2 = rng.roll(att.strength);
            let d2 = rng.roll(def.strength);
            if a2 < d2 {
                attacker_wins = false;
            }
        }
        if attacker_wins {
            def.damage += att.firepower;
        } else {
            att.damage += def.firepower;
        }
        rounds += 1;
    }
    // Observed in every oracle battle: exactly one more rand() after the
    // last round, regardless of who won (both winners were already
    // veterans, so it is not only a promotion roll).
    rng.rand();
    Outcome {
        attacker_damage: att.damage,
        defender_damage: def.damage,
        attacker_won: def.damage >= def.hp,
        rounds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The CRT recurrence as the binary builds it (lea/shl chain at
    /// 0x5b11d6..0x5b11f0) — restated independently of `wrapping_mul`.
    #[test]
    fn crt_multiplier_matches_the_lea_chain() {
        let x: u32 = 0x3a69_6ba7;
        let e = x.wrapping_mul(5).wrapping_mul(5).wrapping_add(x); // 26x
        let e = x.wrapping_add(e.wrapping_mul(8)); // 209x
        let e = (e << 8).wrapping_sub(x); // 53503x
        let e = x.wrapping_add(e.wrapping_mul(4)).wrapping_add(0x269ec3);
        let mut r = CrtRand::new(x);
        r.rand();
        assert_eq!(r.state(), e);
    }

    #[test]
    fn attack_strength_matches_decode() {
        // WRE Legion 4a, veteran: 32 + 16.
        assert_eq!(attack_strength(4, ATTR_VETERAN), 48);
        assert_eq!(attack_strength(2, 0), 16);
    }

    fn legion(damage: i32) -> Fighter {
        // 48 ×2 (wonder 6 vs barbarian defender) — read live from the
        // combat frame's [ebp-0xb0] during oracle battle 3.
        Fighter {
            strength: 96,
            hp: 10,
            firepower: 1,
            damage,
        }
    }
    fn horsemen() -> Fighter {
        // D read live from [ebp-0x60]: def 3 ·4 ·(terrain+river 3) ·1.5 vet.
        Fighter {
            strength: 54,
            hp: 10,
            firepower: 1,
            damage: 0,
        }
    }

    /// Oracle battle 3 (FallRome, Dec 429): veteran WRE Legion (damage 1)
    /// attacks B. Horsemen NE. CRT state before/after read from 0x5e2590;
    /// unit records before/after read from the live unit array. Two
    /// mid-battle rolls polled from the live stack frame — (54,13) and
    /// (73,45) — are rounds 6 and 13 here.
    #[test]
    fn oracle_battle_3_is_bit_exact() {
        let mut r = CrtRand::new(0xa925_748c);
        let o = resolve(&mut r, legion(1), horsemen(), false);
        assert_eq!((o.attacker_damage, o.defender_damage), (4, 10));
        assert!(o.attacker_won);
        assert_eq!(o.rounds, 13);
        assert_eq!(r.draws(), 27);
        assert_eq!(r.state(), 0xb876_62c9);
    }

    /// Oracle battle 2: same pair, undamaged Legion.
    #[test]
    fn oracle_battle_2_is_bit_exact() {
        let mut r = CrtRand::new(0x3a69_6ba7);
        let o = resolve(&mut r, legion(0), horsemen(), false);
        assert_eq!((o.attacker_damage, o.defender_damage), (1, 10));
        assert_eq!(o.rounds, 11);
        assert_eq!(r.draws(), 23);
        assert_eq!(r.state(), 0xa925_748c);
    }

    /// Oracle battle 1: non-veteran WRE Roman Inf (2a → 16 ×2 = 32) loses
    /// to B. Horsemen, which ends with damage 5. The pre-battle state was
    /// not captured; it is the unique state 31 draws before battle 2's
    /// start that reproduces the outcome (searched over 400 back-steps).
    #[test]
    fn oracle_battle_1_back_solved() {
        let inv = 0xb9b3_3155u32; // 214013⁻¹ mod 2³²
        assert_eq!(214013u32.wrapping_mul(inv), 1);
        let mut x = 0x3a69_6ba7u32;
        for _ in 0..31 {
            x = x.wrapping_sub(2531011).wrapping_mul(inv);
        }
        let mut r = CrtRand::new(x);
        let att = Fighter {
            strength: 32,
            hp: 10,
            firepower: 1,
            damage: 0,
        };
        let o = resolve(&mut r, att, horsemen(), false);
        assert_eq!((o.attacker_damage, o.defender_damage), (10, 5));
        assert!(!o.attacker_won);
        assert_eq!(o.rounds, 15);
        assert_eq!(r.state(), 0x3a69_6ba7);
    }

    /// Oracle battle 4 — the predictive one: the CRT slot was SEEDED to
    /// 0x12345678 by us (`rngslot.py set 0x12345678 --crt`), then the
    /// Legion (damage 4) attacked a different Horsemen (N, tile (21,29)).
    /// The original's result and end state equal this prediction; polled
    /// live rolls (22,27) and (67,21) are rounds 3 and 7.
    #[test]
    fn oracle_battle_4_chosen_seed_prediction() {
        let mut r = CrtRand::new(0x1234_5678);
        let o = resolve(&mut r, legion(4), horsemen(), false);
        assert_eq!((o.attacker_damage, o.defender_damage), (6, 10));
        assert_eq!(o.rounds, 12);
        assert_eq!(r.draws(), 25);
        assert_eq!(r.state(), 0x65e8_1e53);
    }

    #[test]
    fn strength_one_rolls_nothing() {
        let mut r = CrtRand::new(7);
        let att = Fighter {
            strength: 1,
            hp: 10,
            firepower: 1,
            damage: 9,
        };
        let def = Fighter {
            strength: 1,
            hp: 10,
            firepower: 1,
            damage: 0,
        };
        let o = resolve(&mut r, att, def, false);
        // 0 > 0 is false: the defender wins the only round; only the
        // trailing draw is consumed.
        assert_eq!(o.attacker_damage, 10);
        assert_eq!(r.draws(), 1);
    }
}
