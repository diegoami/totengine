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
//! promotion (battle 6). Full defence strength now matches 1,536 original
//! executions including walls, fortresses and air interception in cities.
//! Later combat-only modifiers (coastal fortress, SAM/SDI, AEGIS
//! round strength and naval firepower) still have explicit limits.

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
    pub flags: u32,
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
    /// The attacker, if any: its domain and full Rules flags (walls and
    /// air-interception conditions).
    pub attacker: Option<(u8, u32)>,
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
        Some((dom, flags)) => {
            flags & FLAG_NEGATES_WALLS == 0
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
    let intercept = d.in_city
        && d.domain == DOMAIN_AIR
        && d.flags & FLAG_CAN_ATTACK_AIR != 0
        && d.attacker
            .is_some_and(|(domain, flags)| domain == DOMAIN_AIR && flags & FLAG_ONE_SHOT == 0);
    if intercept {
        mult *= if d
            .attacker
            .is_some_and(|(_, flags)| flags & FLAG_CAN_ATTACK_AIR != 0)
        {
            2
        } else {
            4
        };
    } else {
        if d.in_city && d.city_walls_or_great_wall && d.domain == DOMAIN_LAND && attacker_ok(true) {
            mult = m.walls;
        }
        if d.domain != DOMAIN_LAND {
            mult = 2;
        }
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

/// Unit-type flag bits (Rules @UNITS field 13, read right to left).
pub const FLAG_CAN_ATTACK_AIR: u32 = 1 << 4;
pub const FLAG_NEGATES_WALLS: u32 = 1 << 6;
pub const FLAG_PIKEMEN: u32 = 1 << 10;
pub const FLAG_ONE_SHOT: u32 = 1 << 12;
pub const FLAG_AEGIS: u32 = 1 << 13;

/// Inputs to the original pikemen and barbarian Archer/Legion branches.
#[derive(Debug, Clone, Copy)]
pub struct LandDefenseContext {
    pub attacker_domain: u8,
    pub attacker_move: u32,
    pub attacker_max_hp: i32,
    pub defender_flags: u32,
    pub barbarian_type: Option<u8>,
    pub humans_know_iron_working: bool,
}

pub fn land_defense_bonus(mut defense: i32, c: &LandDefenseContext) -> i32 {
    if c.defender_flags & FLAG_PIKEMEN != 0
        && c.attacker_domain == DOMAIN_LAND
        && c.attacker_move == 2
        && c.attacker_max_hp == 10
    {
        defense += defense >> 1;
    }
    if c.barbarian_type
        .is_some_and(|t| t == 4 || (t == 5 && !c.humans_know_iron_working))
    {
        defense = (defense / 2).max(1);
    }
    defense
}

/// Game-flags bit: damage persists between battles (save @652).
pub const GAME_FLAG_PERSISTENT_DAMAGE: u16 = 0x10;

/// What one attack did, for the turn report and the event engine
/// (UNITKILLED).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Battle {
    pub attacker: crate::Unit,
    pub defender: crate::Unit,
    pub attack: i32,
    pub defense: i32,
    pub outcome: Outcome,
    /// The unit that died (a copy of its last state).
    pub killed: crate::Unit,
}

impl Battle {
    /// The UNITKILLED trigger context for this battle, per the game's own
    /// Macro.txt: "Attacker is the civilization that killed the unit,
    /// defender is the one who owned the unit" — so when the attacker
    /// dies, `attacker` is the *defending* civ. `unit` is the killed
    /// unit's Rules name. (`Defender Only` is not modelled; FallRome does
    /// not use it.)
    pub fn unitkilled_ctx(&self, w: &crate::WorldState, rules: &Rules) -> crate::events::Ctx {
        let winner = if self.outcome.attacker_won {
            &self.attacker
        } else {
            &self.defender
        };
        crate::events::Ctx {
            unit: rules
                .unit_names()
                .get(self.killed.type_id as usize)
                .map(|s| s.to_string()),
            attacker: Some(w.civ_name(winner.owner)),
            defender: Some(w.civ_name(self.killed.owner)),
            ..Default::default()
        }
    }
}

impl crate::WorldState {
    /// Raw ownership only. Combat uses `has_active_wonder` with Rules so
    /// globally obsolete or destroyed wonders do not grant bonuses.
    pub fn civ_has_wonder(&self, civ: u8, w: usize) -> bool {
        let Some(&city) = self.wonders.get(w) else {
            return false;
        };
        city >= 0
            && self
                .cities
                .get(city as usize)
                .is_some_and(|c| c.city_id != 0 && (c.owner & 0x0f) as u8 == civ)
    }

    fn city_at(&self, x: u16, y: u16) -> Option<usize> {
        self.cities
            .iter()
            .position(|c| c.city_id != 0 && c.map_id == 0 && (c.x, c.y) == (x, y))
    }

    fn defense_of(&self, rules: &Rules, d: &crate::Unit, att: Option<&crate::Unit>) -> i32 {
        let t = d.type_id as usize;
        let terrain = self.map.terrain_at(d.x, d.y).unwrap_or(0);
        let river = self
            .map
            .stored_index(d.x, d.y)
            .is_some_and(|i| self.map.river[i]);
        let in_city = self.city_at(d.x, d.y).is_some();
        let input = DefenseInput {
            defense: rules.unit_defense(t).unwrap_or(0) as u8,
            flags: rules.unit_flags(t).unwrap_or(0),
            domain: rules.unit_domain(t).unwrap_or(0),
            veteran: d.veteran,
            terrain_defense: rules.terrain_defense(terrain as usize).unwrap_or(2) as u8,
            river,
            fortified: d.orders == 2,
            fortress: self
                .map
                .stored_index(d.x, d.y)
                .is_some_and(|i| self.map.improvements[i] & 0x42 == 0x40),
            in_city,
            city_walls_or_great_wall: self.has_active_wonder(rules, d.owner, 6)
                || self
                    .city_at(d.x, d.y)
                    .is_some_and(|i| self.cities[i].has_building(8)),
            attacker: att.map(|a| {
                let at = a.type_id as usize;
                (
                    rules.unit_domain(at).unwrap_or(0),
                    rules.unit_flags(at).unwrap_or(0),
                )
            }),
        };
        defense_strength(&input, &DefenseMultipliers::default())
    }

    fn hp_max(rules: &Rules, u: &crate::Unit) -> i32 {
        rules.unit_hits(u.type_id as usize).unwrap_or(1) as i32 * 10
    }

    /// The original's defender choice (VA 0x53fa30): every unit on the
    /// tile is rated in tile-list order ([`crate::Unit::stack_pos`]); the
    /// LAST unit with `rating >= best` defends. Land units on an ocean tile
    /// are skipped.
    fn select_defender(&self, rules: &Rules, att: &crate::Unit, x: u16, y: u16) -> Option<usize> {
        let ocean = self.map.terrain_at(x, y) == Some(crate::movement::OCEAN_TERRAIN);
        let att_domain = rules.unit_domain(att.type_id as usize).unwrap_or(0);
        let att_flags = rules.unit_flags(att.type_id as usize).unwrap_or(0);
        let in_city = self.city_at(x, y).is_some();
        let mut best = 0;
        let mut pick = None;
        let mut tile: Vec<usize> = (0..self.units.len())
            .filter(|&i| {
                let u = &self.units[i];
                u.id != 0 && (u.x, u.y) == (x, y) && u.owner != att.owner
            })
            .collect();
        tile.sort_by_key(|&i| self.units[i].stack_pos); // stable: index order within a position
        for i in tile {
            let u = &self.units[i];
            let t = u.type_id as usize;
            let domain = rules.unit_domain(t).unwrap_or(0);
            if ocean && domain == DOMAIN_LAND {
                continue;
            }
            let flags = rules.unit_flags(t).unwrap_or(0);
            let mut r = self.defense_of(rules, u, Some(att));
            if self.game_flags & GAME_FLAG_PERSISTENT_DAMAGE != 0 {
                let max = Self::hp_max(rules, u);
                r = r * (max - u.damage as i32) / max;
            }
            if flags & FLAG_PIKEMEN != 0 {
                r += 1;
            }
            if flags & FLAG_AEGIS != 0 && att_domain == DOMAIN_AIR {
                r *= if att_flags & FLAG_ONE_SHOT != 0 { 5 } else { 3 };
            }
            if flags & FLAG_CAN_ATTACK_AIR != 0
                && att_domain == DOMAIN_AIR
                && att_flags & FLAG_CAN_ATTACK_AIR != 0
            {
                r <<= 1;
            }
            if domain == DOMAIN_SEA && in_city {
                r = if att_domain == DOMAIN_AIR
                    && !self
                        .city_at(x, y)
                        .is_some_and(|i| self.cities[i].has_building(27))
                {
                    r << 1
                } else {
                    r / 2
                };
            }
            if r >= best {
                best = r;
                pick = Some(i);
            }
        }
        pick
    }

    /// `attacker_idx` attacks the enemy-held neighbouring tile (x, y).
    /// Returns `None` when the tile is not adjacent, holds nothing to
    /// attack, or the unit cannot attack (no
    /// movement, attack factor 0 — the latter is an engine rule, not
    /// decoded). The loser leaves a reusable dead slot; all other indices stay
    /// stable; only the defender dies, its stack-mates survive (observed
    /// in all FallRome battles, open field). The attacker stays on its
    /// tile and spends one movement point (oracle: moves spent 0 → 15).
    pub fn attack(&mut self, rules: &Rules, attacker_idx: usize, x: u16, y: u16) -> Option<Battle> {
        let att = self.units.get(attacker_idx)?.clone();
        if att.id == 0
            || att.moves_left == 0
            || !crate::movement::is_adjacent((att.x, att.y), (x, y))
        {
            return None;
        }
        let at = att.type_id as usize;
        let attack = rules.unit_attack(at).unwrap_or(0);
        if attack == 0 {
            return None;
        }
        let def_idx = self.select_defender(rules, &att, x, y)?;
        if self.game_flags & GAME_FLAG_PERSISTENT_DAMAGE == 0 {
            self.units[attacker_idx].damage = 0;
            self.units[def_idx].damage = 0;
        }
        let (att, def) = (
            self.units[attacker_idx].clone(),
            self.units[def_idx].clone(),
        );
        let civ_cities = self
            .cities
            .iter()
            .filter(|c| c.city_id != 0 && (c.owner & 0x0f) as u8 == def.owner)
            .count() as i32;
        let ctx = AttackContext {
            moves_left: att.moves_left as i32,
            move_unit: 3, // engine movement is in thirds (T1-3 caveat)
            attacker_barbarian: att.owner == 0,
            defender_barbarian: def.owner == 0,
            attacker_great_wall: self.has_active_wonder(rules, att.owner, 6),
            defender_great_wall: self.has_active_wonder(rules, def.owner, 6),
            defender_human: self.human_mask & (1 << (def.owner & 7)) != 0,
            difficulty: self.difficulty as i32,
            defender_city_count: civ_cities,
            defender_city_has_palace: self
                .city_at(x, y)
                .is_some_and(|i| self.cities[i].has_building(1)),
            defender_in_city: self.city_at(x, y).is_some(),
        };
        let attr = if att.veteran { ATTR_VETERAN } else { 0 };
        let (a, reroll) = attack_in_context(attack_strength(attack as u8, attr), &ctx);
        let d = land_defense_bonus(
            self.defense_of(rules, &def, Some(&att)),
            &LandDefenseContext {
                attacker_domain: rules.unit_domain(at).unwrap_or(0),
                attacker_move: self.movement_allowance(rules, at, att.owner, att.damage) / 3,
                attacker_max_hp: Self::hp_max(rules, &att),
                defender_flags: rules.unit_flags(def.type_id as usize).unwrap_or(0),
                barbarian_type: (def.owner == 0).then_some(def.type_id),
                humans_know_iron_working: (1..8).any(|c| {
                    self.human_mask & (1 << c) != 0 && self.techs[c][39 / 8] & (1 << (39 % 8)) != 0
                }),
            },
        );
        let fighter = |u: &crate::Unit, strength: i32| Fighter {
            strength,
            hp: Self::hp_max(rules, u),
            firepower: rules.unit_firepower(u.type_id as usize).unwrap_or(1) as i32,
            damage: u.damage as i32,
            wonder7: self.has_active_wonder(rules, u.owner, 7),
        };
        let (fa, fd) = (fighter(&att, a), fighter(&def, d));
        let o = resolve(&mut self.combat_rng, fa, fd, reroll);
        let one_shot = |u: &crate::Unit| {
            rules.unit_flags(u.type_id as usize).unwrap_or(0) & FLAG_ONE_SHOT != 0
        };
        let att_spent = self
            .movement_allowance(rules, at, att.owner, att.damage)
            .saturating_sub(att.moves_left)
            .saturating_add(3);
        let def_spent = self
            .movement_allowance(rules, def.type_id as usize, def.owner, def.damage)
            .saturating_sub(def.moves_left);
        let att_damage = o.attacker_damage.clamp(0, 255) as u8;
        let def_damage = o.defender_damage.clamp(0, 255) as u8;
        let att_left = self
            .movement_allowance(rules, at, att.owner, att_damage)
            .saturating_sub(att_spent);
        let def_left = self
            .movement_allowance(rules, def.type_id as usize, def.owner, def_damage)
            .saturating_sub(def_spent);
        self.units[attacker_idx].damage = att_damage;
        self.units[attacker_idx].moves_left = att_left;
        self.units[def_idx].damage = def_damage;
        self.units[def_idx].moves_left = def_left;
        let (winner, loser) = if o.attacker_won {
            (attacker_idx, def_idx)
        } else {
            (def_idx, attacker_idx)
        };
        if o.winner_promoted && !one_shot(&self.units[winner]) {
            self.units[winner].veteran = true;
        }
        let killed = self.units[loser].clone();
        self.remove_land_unit(loser);
        let removed_coordinate = if loser == attacker_idx {
            -200_i16
        } else {
            -100_i16
        } as u16;
        self.units[loser].x = removed_coordinate;
        self.units[loser].y = removed_coordinate;
        Some(Battle {
            attacker: att,
            defender: def,
            attack: a,
            defense: d,
            outcome: o,
            killed,
        })
    }
}

use tot_kit::Rules;

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

    #[test]
    fn original_land_defense_modifiers() {
        for line in include_str!("../../../docs/evidence/combat-land-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            let context = LandDefenseContext {
                attacker_domain: v[1] as u8,
                attacker_move: v[2] as u32,
                attacker_max_hp: v[3],
                defender_flags: v[4] as u32,
                barbarian_type: (v[5] != 0).then_some(v[6] as u8),
                humans_know_iron_working: v[7] != 0,
            };
            assert_eq!(land_defense_bonus(v[0], &context), v[8], "{line}");
            if v[0] == 24 && v[1] == 0 && v[2] == 2 && v[3] == 10 && v[4] == 1024 {
                let mut text = String::from(
                    "@UNITS\nAttacker,nil,0,2.,0,2a,1d,1h,1f,1,0,0,nil,000000000000000\n",
                );
                for _ in 1..6 {
                    text.push_str("Defender,nil,0,1.,0,1a,3d,1h,1f,1,0,1,nil,000010000000000\n");
                }
                text.push_str("@TERRAIN\nDesert,1,2,0,1,0\nPlains,1,2,1,1,0\n");
                let rules = tot_kit::parse_rules(&text);
                let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
                w.cities.clear();
                w.wonders = [-1; 28];
                w.techs = [[0; 13]; 8];
                w.human_mask = 2;
                if v[7] != 0 {
                    w.techs[1][39 / 8] |= 1 << (39 % 8);
                }
                let unit = w.units[0].clone();
                w.units = vec![unit.clone(), unit];
                for (i, u) in w.units.iter_mut().enumerate() {
                    u.id = i as u32 + 1;
                    u.x = 2 * i as u16;
                    u.y = 0;
                    u.type_id = if i == 0 { 0 } else { v[6] as u8 };
                    u.owner = if i == 0 {
                        1
                    } else if v[5] != 0 {
                        0
                    } else {
                        2
                    };
                    u.orders = 255;
                    u.veteran = false;
                    u.damage = 0;
                    u.moves_left = 6;
                    let tile = w.map.stored_index(u.x, u.y).unwrap();
                    w.map.terrain[tile] = 1;
                    w.map.river[tile] = false;
                }
                assert_eq!(
                    w.attack(&rules, 0, 2, 0).unwrap().defense,
                    v[8],
                    "world modifier {line}"
                );
            }
        }
    }

    #[test]
    fn original_full_defense_and_world_adapter_vectors() {
        for line in include_str!("../../../docs/evidence/combat-defense-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect();
            let input = DefenseInput {
                defense: v[1] as u8,
                flags: v[11] as u32,
                domain: v[0] as u8,
                veteran: v[2] != 0,
                terrain_defense: v[3] as u8,
                river: v[4] != 0,
                fortified: v[5] != 0,
                fortress: v[6] != 0,
                in_city: v[7] != 0,
                city_walls_or_great_wall: v[8] != 0,
                attacker: (v[9] >= 0).then_some((v[9] as u8, v[10] as u32)),
            };
            assert_eq!(
                defense_strength(&input, &DefenseMultipliers::default()),
                v[12],
                "{line}"
            );
            let rules = tot_kit::parse_rules(&format!(
                "@UNITS\nAttacker,nil,{},1.,0,2a,1d,1h,1f,1,0,0,nil,{:015b}\nDefender,nil,{},1.,0,1a,{}d,1h,1f,1,0,1,nil,{:015b}\n@TERRAIN\nDesert,1,2,0,1,0\nPlains,1,{},1,1,0\n",
                v[9].max(0),v[10],v[0],v[1],v[11],v[3]));
            let mut w = crate::boot(&crate::tests_support::synthetic_save(), 1);
            w.wonders = [-1; 28];
            let mut city = w.cities[0].clone();
            w.cities.clear();
            if v[7] != 0 {
                city.x = 2;
                city.y = 0;
                city.city_id = 1;
                city.map_id = 0;
                city.owner = 2;
                city.buildings = [0; 5];
                if v[8] != 0 {
                    city.buildings[1] = 1;
                }
                w.cities.push(city);
            }
            let template = w.units[0].clone();
            w.units = vec![template.clone(), template];
            for (k, u) in w.units.iter_mut().enumerate() {
                u.id = k as u32 + 1;
                u.type_id = k as u8;
                u.owner = k as u8 + 1;
                u.x = 2 * k as u16;
                u.y = 0;
                u.veteran = k == 1 && v[2] != 0;
                u.orders = if k == 1 && v[5] != 0 { 2 } else { 255 };
            }
            let tile = w.map.stored_index(2, 0).unwrap();
            w.map.terrain[tile] = 1;
            w.map.river[tile] = v[4] != 0;
            w.map.improvements[tile] = if v[7] != 0 {
                2
            } else if v[6] != 0 {
                64
            } else {
                0
            };
            assert_eq!(
                w.defense_of(&rules, &w.units[1], (v[9] >= 0).then_some(&w.units[0])),
                v[12],
                "world adapter {line}"
            );
        }
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
            flags: 0,
            domain: DOMAIN_LAND,
            veteran: true,
            terrain_defense: 2,
            river: true,
            fortified,
            fortress: false,
            in_city: false,
            city_walls_or_great_wall: false,
            attacker: Some((DOMAIN_LAND, 0)),
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
        d.attacker = Some((DOMAIN_LAND, FLAG_NEGATES_WALLS)); // negates walls (and fortress)
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
