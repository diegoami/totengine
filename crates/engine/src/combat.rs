//! Combat resolution (T1-6), decoded from the original and replayed
//! bit-exactly against six oracle battles (evidence §T1-6 combat).
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
//! promotion: r = rand() % (A+D); winner promoted if r <= loser's strength
//!            (or its civ has wonder 7)          // VA 0x544554
//! ```
//!
//! Decoded here: the **round loop**, the **attack strength** with its
//! context modifiers ([`attack_in_context`]), and the **land defence
//! strength** ([`defense_strength`], including the ToTPP hooks that replace
//! the terrain and fortify/fortress/walls constants) and the **promotion
//! roll**. Oracle-proven: six battles bit-exact, live-frame A/D read in
//! battles 3–6, including a fortified defender (D 54 → 81) and a predicted
//! promotion (battle 6). Not implemented: the sea/air and
//! city-improvement special cases (coastal fortress, SAM, SDI, fighters vs
//! air, pikemen) — see evidence §T1-6 combat.

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

/// Context the combat function (VA 0x541425) applies to the attack
/// strength before the rounds start. Owners: 0 = barbarians.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackContext {
    /// Movement left and the game's one-move unit (byte @0x5f5eb8: 15 in
    /// FallRome). The attack is scaled by `min(left, unit) / unit` (only
    /// in a real attack — the AI odds probe skips it).
    pub moves_left: i32,
    pub move_unit: i32,
    pub attacker_barbarian: bool,
    pub defender_barbarian: bool,
    /// Attacker's / defender's civ has wonder 6 (Great Wall; FallRome:
    /// Hadrianus' Wall) — `0x444a61(civ, 6)`.
    pub attacker_great_wall: bool,
    pub defender_great_wall: bool,
    /// Barbarian-attacker branch only: defender civ is human, game
    /// difficulty (@0x607658), defender civ's city count, and whether the
    /// defended tile is a city with a Palace (improvement 1).
    pub defender_human: bool,
    pub difficulty: i32,
    pub defender_city_count: i32,
    pub defender_city_has_palace: bool,
    /// The defended tile holds a city (`0x65b470 >= 0`).
    pub defender_in_city: bool,
}

/// Final attack strength and whether the defender-favouring re-roll is on.
/// Mirrors VA 0x541590..0x541d82 for land combat. Integer rounding is the
/// original's (`/2`, `/4` truncate toward zero on non-negative values).
pub fn attack_in_context(base: i32, ctx: &AttackContext) -> (i32, bool) {
    let mut a = base;
    let mut reroll = false;
    if ctx.move_unit > 0 {
        let left = ctx.moves_left.min(ctx.move_unit);
        a = a * left / ctx.move_unit;
    }
    if ctx.attacker_barbarian {
        a = if ctx.defender_human {
            (ctx.difficulty + 1) * a / 4
        } else {
            a / 2
        };
        if ctx.defender_in_city {
            if ctx.defender_city_count <= 1 {
                a = 0;
            }
            if ctx.defender_city_has_palace {
                a >>= 1;
                reroll = true;
            }
            if ctx.defender_city_count < 8 {
                reroll = true;
            }
        }
        if ctx.defender_great_wall {
            a /= 2;
        }
    }
    if ctx.defender_barbarian && ctx.attacker_great_wall {
        a <<= 1;
    }
    (a, reroll)
}

/// Unit domain (Rules @UNITS column 3; unit type +9).
pub const DOMAIN_LAND: u8 = 0;
pub const DOMAIN_AIR: u8 = 1;
pub const DOMAIN_SEA: u8 = 2;

/// ToTPP's configurable defence multipliers, in halves (2 = ×1). The
/// stock game hard-codes these; ToTPP stores them at TOTPP.dll data
/// `+0x36b1c..` (read live: 3, 4, 6 — fortified ×1.5, fortress ×2,
/// walls ×3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefenseMultipliers {
    pub fortified: i32,
    pub fortress: i32,
    pub walls: i32,
    /// ToTPP options: air / sea defenders use terrain too (both on in the
    /// oracle install). Off → a flat ×1.
    pub air_uses_terrain: bool,
    pub sea_uses_terrain: bool,
}

impl Default for DefenseMultipliers {
    fn default() -> Self {
        DefenseMultipliers {
            fortified: 3,
            fortress: 4,
            walls: 6,
            air_uses_terrain: true,
            sea_uses_terrain: true,
        }
    }
}

/// Everything the defence-strength function reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefenseInput {
    /// Rules defence factor and domain of the defender's type.
    pub defense: u8,
    pub domain: u8,
    pub veteran: bool,
    /// Rules @TERRAIN defence column of the tile (in halves: Grassland 2,
    /// Forest 3, Hills 4, Mountains 6) and the tile's river bit (+1).
    pub terrain_defense: u8,
    pub river: bool,
    /// Defender's orders are "fortified" (unit +0x11 == 2).
    pub fortified: bool,
    /// Tile has a fortress (and no airbase): terrain flags `& 0x42 == 0x40`.
    pub fortress: bool,
    /// Defender stands in a city that has City Walls (improvement 8), or
    /// its civ has wonder 6 (Great Wall acts as walls everywhere).
    pub in_city: bool,
    pub city_walls_or_great_wall: bool,
    /// The attacker, if any: its domain and the "negates city walls" flag
    /// (unit-type flags bit 6).
    pub attacker: Option<(u8, bool)>,
}

/// The original's defence strength (VA 0x53f67a + the ToTPP hooks it
/// calls), for land, sea and air defenders outside the air-in-city
/// special case.
///
/// ```text
/// D = def·4 · (terrain_def + river)        // ToTPP 0x76c14f86/0x76c14f23
/// mult = 2 (halves)
/// fortified && land                   → mult = M.fortified
/// fortress tile, attacker not air and not negating walls → mult = M.fortress
/// city && (walls || great wall) && land && attacker land, not negating
///                                     → mult = M.walls
/// defender not land                   → mult = 2
/// D = D·mult >> 1 (if mult != 2);  veteran: D += D >> 1
/// ```
pub fn defense_strength(d: &DefenseInput, m: &DefenseMultipliers) -> i32 {
    let terrain = match d.domain {
        DOMAIN_LAND => true,
        DOMAIN_AIR => m.air_uses_terrain,
        DOMAIN_SEA => m.sea_uses_terrain,
        _ => false,
    };
    let factor = if terrain {
        d.terrain_defense as i32 + d.river as i32
    } else {
        2
    };
    let mut def = d.defense as i32 * factor * 4;
    let mut mult = 2;
    if d.fortified && d.domain == DOMAIN_LAND {
        mult = m.fortified;
    }
    let attacker_ok = |need_land: bool| match d.attacker {
        None => true,
        Some((dom, negates)) => {
            !negates
                && if need_land {
                    dom == DOMAIN_LAND
                } else {
                    dom != DOMAIN_AIR
                }
        }
    };
    if d.fortress && attacker_ok(false) {
        mult = m.fortress;
    }
    if d.in_city && d.city_walls_or_great_wall && d.domain == DOMAIN_LAND && attacker_ok(true) {
        mult = m.walls;
    }
    if d.domain != DOMAIN_LAND {
        mult = 2;
    }
    if mult != 2 {
        def = (def * mult) >> 1;
    }
    if d.veteran {
        def += def >> 1;
    }
    def
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
    /// The unit's civ has wonder 7 (Sun Tzu; FallRome: Warrior Culture):
    /// a won battle always passes the promotion roll.
    pub wonder7: bool,
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
    /// The winner passed the post-battle promotion roll. Apply it as the
    /// original's `0x53ff5e` does: set attr `0x2000` unless the unit is
    /// already a veteran or its type is one-shot (flags bit 12, missiles).
    pub winner_promoted: bool,
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
    // Promotion roll (VA 0x544554 / 0x5445dc): one draw, always taken when
    // A + D > 1. The winner is promoted if the roll lands at or below the
    // LOSER's strength, so an underdog win promotes more often.
    let attacker_won = def.damage >= def.hp;
    let r = rng.roll(att.strength + def.strength);
    let (loser_strength, wonder) = if attacker_won {
        (def.strength, att.wonder7)
    } else {
        (att.strength, def.wonder7)
    };
    Outcome {
        attacker_damage: att.damage,
        defender_damage: def.damage,
        attacker_won,
        rounds,
        winner_promoted: r <= loser_strength || wonder,
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

    /// The oracle setting: WRE (owns wonder 6, Hadrianus' Wall) attacks
    /// barbarians, full movement (FallRome move unit 15).
    fn vs_barbarian() -> AttackContext {
        AttackContext {
            moves_left: 15,
            move_unit: 15,
            attacker_barbarian: false,
            defender_barbarian: true,
            attacker_great_wall: true,
            defender_great_wall: false,
            defender_human: false,
            difficulty: 0,
            defender_city_count: 0,
            defender_city_has_palace: false,
            defender_in_city: false,
        }
    }
    /// B. Horsemen (def 3, veteran) on Grassland/Plains (2) + river, as
    /// on all three defended tiles (map: ROMFALL.scn).
    fn horsemen_input(fortified: bool) -> DefenseInput {
        DefenseInput {
            defense: 3,
            domain: DOMAIN_LAND,
            veteran: true,
            terrain_defense: 2,
            river: true,
            fortified,
            fortress: false,
            in_city: false,
            city_walls_or_great_wall: false,
            attacker: Some((DOMAIN_LAND, false)),
        }
    }
    fn legion(damage: i32) -> Fighter {
        // WRE Legion 4a veteran → 48, ×2 Great Wall vs barbarians = 96
        // (read live from the combat frame's [ebp-0xb0] in battles 3–5).
        let (a, reroll) = attack_in_context(attack_strength(4, ATTR_VETERAN), &vs_barbarian());
        assert!(!reroll);
        Fighter {
            strength: a,
            hp: 10,
            firepower: 1,
            damage,
            wonder7: false,
        }
    }
    fn horsemen() -> Fighter {
        // D read live from [ebp-0x60]: 54.
        Fighter {
            strength: defense_strength(&horsemen_input(false), &DefenseMultipliers::default()),
            hp: 10,
            firepower: 1,
            damage: 0,
            wonder7: false,
        }
    }

    #[test]
    fn strengths_match_live_frames() {
        assert_eq!(legion(0).strength, 96);
        assert_eq!(horsemen().strength, 54);
        let m = DefenseMultipliers::default();
        // Battle 5: the defender's orders were set to fortified → 81 live.
        assert_eq!(defense_strength(&horsemen_input(true), &m), 81);
        // Battle 1: Roman Inf 2a, not veteran: 16 ×2 = 32.
        assert_eq!(
            attack_in_context(attack_strength(2, 0), &vs_barbarian()).0,
            32
        );
    }

    #[test]
    fn defense_multiplier_precedence() {
        let m = DefenseMultipliers::default();
        let mut d = horsemen_input(true);
        d.veteran = false; // base 3·4·3 = 36
        d.fortress = true; // fortress replaces fortified, does not stack
        assert_eq!(defense_strength(&d, &m), 72);
        d.in_city = true;
        d.city_walls_or_great_wall = true;
        assert_eq!(defense_strength(&d, &m), 108);
        d.attacker = Some((DOMAIN_LAND, true)); // negates walls (and fortress)
        assert_eq!(defense_strength(&d, &m), 54); // only fortified remains
        d.domain = DOMAIN_SEA; // non-land: no multiplier at all
        assert_eq!(defense_strength(&d, &m), 36);
    }

    #[test]
    fn partial_move_and_barbarian_attacker() {
        let mut c = vs_barbarian();
        c.moves_left = 5; // a third of a move
        assert_eq!(attack_in_context(48, &c).0, 32); // 48·5/15 = 16, ×2
        let c = AttackContext {
            attacker_barbarian: true,
            defender_barbarian: false,
            attacker_great_wall: false,
            defender_human: true,
            difficulty: 2,
            defender_in_city: true,
            defender_city_count: 5,
            ..vs_barbarian()
        };
        // 48·3/4 = 36; fewer than 8 cities → re-roll on.
        assert_eq!(attack_in_context(48, &c), (36, true));
        let c = AttackContext {
            defender_city_count: 1,
            ..c
        };
        assert_eq!(attack_in_context(48, &c).0, 0); // last city is immune
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
            wonder7: false,
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

    /// Oracle battle 5: the CRT seeded to 0x0badf00d by us, the defending
    /// Horsemen's orders set to fortified (D 81 read live), the Legion at
    /// damage 6 attacks N and dies; the defender ends at damage 2. Polled
    /// live roll (32,50) is round 6.
    #[test]
    fn oracle_battle_5_fortified_defender() {
        let mut r = CrtRand::new(0x0bad_f00d);
        let def = Fighter {
            strength: defense_strength(&horsemen_input(true), &DefenseMultipliers::default()),
            ..horsemen()
        };
        let o = resolve(&mut r, legion(6), def, false);
        assert_eq!((o.attacker_damage, o.defender_damage), (10, 2));
        assert!(!o.attacker_won);
        assert_eq!(o.rounds, 6);
        assert_eq!(r.draws(), 13);
        assert_eq!(r.state(), 0x8847_66e8);
    }

    /// Oracle battle 6 — promotion, predicted: a veteran WRE Roman Inf
    /// (A 24; no Great Wall bonus, the defender is the Anglo-Saxons, not
    /// barbarians) attacks E into B. Horsemen whose veteran bit we cleared
    /// (Grassland, no river: D 24). Seed 0x1008 was chosen so the defender
    /// wins AND passes the promotion roll; the original set its attr
    /// 0 → 0x2000 and ended at exactly this CRT state.
    #[test]
    fn oracle_battle_6_defender_promoted() {
        let a = attack_in_context(
            attack_strength(2, ATTR_VETERAN),
            &AttackContext {
                defender_barbarian: false,
                ..vs_barbarian()
            },
        )
        .0;
        let d = defense_strength(
            &DefenseInput {
                veteran: false,
                river: false,
                ..horsemen_input(false)
            },
            &DefenseMultipliers::default(),
        );
        assert_eq!((a, d), (24, 24)); // read live from the frame
        let mut r = CrtRand::new(0x1008);
        let o = resolve(
            &mut r,
            Fighter {
                strength: a,
                ..horsemen()
            },
            Fighter {
                strength: d,
                ..horsemen()
            },
            false,
        );
        assert_eq!((o.attacker_damage, o.defender_damage), (10, 8));
        assert!(!o.attacker_won);
        assert!(o.winner_promoted);
        assert_eq!(o.rounds, 18);
        assert_eq!(r.draws(), 37);
        assert_eq!(r.state(), 0x1872_bf7f);
    }

    #[test]
    fn strength_one_rolls_nothing() {
        let mut r = CrtRand::new(7);
        let att = Fighter {
            strength: 1,
            hp: 10,
            firepower: 1,
            damage: 9,
            wonder7: false,
        };
        let def = Fighter {
            strength: 1,
            hp: 10,
            firepower: 1,
            damage: 0,
            wonder7: false,
        };
        let o = resolve(&mut r, att, def, false);
        // 0 > 0 is false: the defender wins the only round; only the
        // trailing draw is consumed.
        assert_eq!(o.attacker_damage, 10);
        assert_eq!(r.draws(), 1);
    }
}
