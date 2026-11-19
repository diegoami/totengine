//! Applying event effects to the world (T1-7): the half of the
//! interpreter that changes state. [`crate::events::EventEngine::fire`]
//! decides *which* effects an event produces; [`WorldState::apply_effects`]
//! carries them out.
//!
//! Semantics come from the game's own `Macro.txt`:
//! - CHANGEMONEY adds `amount` to the receiver's treasury; a result below
//!   zero becomes zero. The treasury field is oracle-proven (save i32@10
//!   of each civ-properties block; evidence §T1-7 treasury).
//! - CREATEUNIT creates `count` units at the first valid location and tries
//!   the others in order. **What counts as valid is not oracle-checked**:
//!   the engine requires a tile on the map, terrain matching the unit's
//!   domain (land not on ocean, sea only on ocean), and no other civ's
//!   units on it. Macro.txt only says "invalid for any reason".
//! - GIVETECHNOLOGY grants the advance through the research-completion
//!   routine (evidence §GIVETECHNOLOGY executor).
//! - TEXT, PLAYCDTRACK and DONTPLAYWONDERS do not touch game state; FLAG
//!   changes only the event engine's own flag words.

use tot_kit::Rules;

use crate::events::{Ctx, Effect, EventEngine};
use crate::movement::OCEAN_TERRAIN;
use crate::{Unit, WorldState};

/// What [`WorldState::apply_effects`] did with one effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// Treasury of civ `civ` is now `treasury`.
    Money { civ: u8, treasury: i32 },
    /// `count` units created at `at`; their indices start at `first`.
    Created {
        first: usize,
        count: u32,
        at: (u16, u16),
    },
    /// Civ `civ` received advance `advance` (GIVETECHNOLOGY).
    Advance { civ: u8, advance: usize },
    /// Civ `civ` lost these advances, dependants first (TAKETECHNOLOGY).
    Taken { civ: u8, advances: Vec<usize> },
    /// `who` and `whom` are at war (MAKEAGGRESSION). `consequences` marks
    /// the original's treaty-break call for a human party (0x44af04),
    /// which is not modelled.
    War {
        who: u8,
        whom: u8,
        consequences: bool,
    },
    /// CHANGETERRAIN changed `tiles` tiles, destroying `cities` cities and
    /// removing `units` units on them.
    Terrain {
        tiles: usize,
        cities: usize,
        units: usize,
    },
    /// The effect changes nothing in the modelled state (text, music).
    NoState,
    /// The effect could not be applied; the reason says why.
    Skipped(String),
}

impl WorldState {
    /// Civ id for an event civ name: the plural, case-insensitive
    /// ("Barbarians" = 0, the engine's label; see [`WorldState::civ_name`]).
    pub fn civ_by_name(&self, name: &str) -> Option<u8> {
        let n = name.trim();
        if n.eq_ignore_ascii_case("Barbarians") {
            return Some(0);
        }
        self.civs
            .iter()
            .find(|c| c.plural.eq_ignore_ascii_case(n))
            .map(|c| c.id)
    }

    /// RECEIVEDTECHNOLOGY inputs: per civ, the advances it knows (native
    /// `0x49c820`: a living civ 1..=7, never advance 89), whether it lives,
    /// and its Future Technology count.
    pub fn event_advances(&self) -> ([[bool; 100]; 8], [bool; 8], [i32; 8]) {
        let mut known = [[false; 100]; 8];
        let mut alive = [false; 8];
        let mut future = [0; 8];
        for c in 1..8 {
            alive[c] = self.civ_alive(c as u8);
            future[c] = i32::from(self.research[c].future);
            if alive[c] {
                for (a, k) in known[c].iter_mut().enumerate() {
                    *k = a != 89 && self.techs[c][a / 8] & (1 << (a % 8)) != 0;
                }
            }
        }
        (known, alive, future)
    }

    /// Plural names of civs 0..=7 for flag `who=` resolution.
    pub fn event_civ_names(&self) -> [String; 8] {
        std::array::from_fn(|c| self.civ_name(c as u8))
    }

    /// Apply effects in order; one [`Applied`] per effect.
    pub fn apply_effects(&mut self, rules: &Rules, effects: &[Effect]) -> Vec<Applied> {
        effects
            .iter()
            .map(|e| self.apply_effect(rules, e))
            .collect()
    }

    /// Fire a trigger and apply what it produces: the turn-loop entry
    /// point (UNITKILLED after a battle, CITYTAKEN, SCENARIOLOADED, ...).
    pub fn fire_event(
        &mut self,
        rules: &Rules,
        events: &mut EventEngine,
        trigger: &str,
        ctx: &Ctx,
    ) -> Vec<(Effect, Applied)> {
        events.civ_names = self.event_civ_names();
        let effects = events.fire(trigger, ctx);
        let applied = self.apply_effects(rules, &effects);
        effects.into_iter().zip(applied).collect()
    }

    /// [`WorldState::move_unit`] plus the event hook: when the move was an
    /// attack, the kill fires UNITKILLED (context from
    /// [`crate::combat::Battle::unitkilled_ctx`]) and its effects are
    /// applied.
    pub fn move_unit_with_events(
        &mut self,
        rules: &Rules,
        events: &mut EventEngine,
        unit_idx: usize,
        x: u16,
        y: u16,
    ) -> (crate::movement::MoveOutcome, Vec<(Effect, Applied)>) {
        let out = self.move_unit(rules, unit_idx, x, y);
        let fired = match &out.battle {
            Some(b) => {
                let ctx = b.unitkilled_ctx(self, rules);
                self.fire_event(rules, events, "UNITKILLED", &ctx)
            }
            None => Vec::new(),
        };
        (out, fired)
    }

    fn apply_effect(&mut self, rules: &Rules, e: &Effect) -> Applied {
        match e {
            Effect::ChangeMoney { receiver, amount } => {
                let Some(civ) = self.civ_by_name(receiver) else {
                    return Applied::Skipped(format!("unknown civ {receiver:?}"));
                };
                let t = &mut self.treasury[civ as usize];
                *t = t.saturating_add(*amount).max(0);
                Applied::Money { civ, treasury: *t }
            }
            // Handler 0x4d2102: each unit scans the locations (from a CRT
            // `rand() % n` start under Randomize, no wraparound) for the
            // first valid one; none ends the action. InCapital then moves
            // it to the owner's first Palace city (a sea unit only when
            // that city is coastal).
            Effect::CreateUnit {
                unit_name,
                owner,
                veteran,
                count,
                locations,
                randomize,
                in_capital,
                home,
            } => {
                let Some(civ) = self.civ_by_name(owner) else {
                    return Applied::Skipped(format!("unknown civ {owner:?}"));
                };
                if civ != 0 && !self.civ_alive(civ) {
                    return Applied::Skipped(format!("{owner:?} is not alive"));
                }
                let Some(type_id) = rules
                    .unit_names()
                    .iter()
                    .position(|n| n.trim().eq_ignore_ascii_case(unit_name.trim()))
                else {
                    return Applied::Skipped(format!("unknown unit {unit_name:?}"));
                };
                let domain = rules.unit_domain(type_id).unwrap_or(0);
                let locs: Vec<Option<(u16, u16)>> = locations
                    .iter()
                    .take(10)
                    .map(|&(x, y)| Some((u16::try_from(x).ok()?, u16::try_from(y).ok()?)))
                    .collect();
                let n = locs.len();
                let capital = in_capital
                    .then(|| {
                        self.cities.iter().position(|c| {
                            c.city_id != 0
                                && (c.owner & 0x0F) as u8 == civ
                                && c.has_building(crate::founding::PALACE)
                        })
                    })
                    .flatten();
                // The first city of that name; only the owner's can be home.
                let home_city = home
                    .as_ref()
                    .and_then(|h| {
                        self.cities
                            .iter()
                            .position(|c| c.city_id != 0 && c.name.trim().eq_ignore_ascii_case(h))
                    })
                    .filter(|&k| (self.cities[k].owner & 0x0F) as u8 == civ)
                    .map_or(0xff, |k| k as u8);
                let mut created = Vec::new();
                let mut first_at = None;
                for _ in 0..*count {
                    let start = if *randomize && n > 1 {
                        self.combat_rng.rand() as usize % n
                    } else {
                        0
                    };
                    let Some(mut at) = (start..n)
                        .find_map(|k| locs[k].filter(|&(x, y)| self.can_place(civ, domain, x, y)))
                    else {
                        break;
                    };
                    if let Some(c) = capital.map(|k| &self.cities[k]) {
                        if domain != 2 || c.flags & 0x80 != 0 {
                            at = (c.x, c.y);
                        }
                    }
                    first_at.get_or_insert(at);
                    if let Some(index) = self.allocate_unit(
                        rules,
                        Unit {
                            type_id: type_id as u8,
                            owner: civ,
                            x: at.0,
                            y: at.1,
                            is_leader: false,
                            moves_left: self.movement_allowance(rules, type_id, civ, 0),
                            veteran: *veteran,
                            damage: 0,
                            orders: 0xff,
                            stack_pos: 0,
                            home: home_city,
                            id: 0,
                        },
                    ) {
                        created.push(index);
                    }
                }
                match (created.first().copied(), first_at) {
                    (Some(first), Some(at)) => Applied::Created {
                        first,
                        count: created.len() as u32,
                        at,
                    },
                    (None, Some(_)) => Applied::Skipped("unit capacity exhausted".into()),
                    _ => Applied::Skipped("no valid location".into()),
                }
            }
            // Original executor 0x4d201e: the receiver (trigger
            // parties resolved first) must be a living civ 1..=7, then the
            // advance goes through the same grant as research completion,
            // 0x49dedf(civ, tech, 0, 0, 0) — modelled by
            // `grant_advance_with_rules` (bit/count/target, Leonardo, Library).
            Effect::GiveTechnology {
                receiver,
                technology,
            } => {
                let Some(civ) = self.civ_by_name(receiver) else {
                    return Applied::Skipped(format!("unknown civ {receiver:?}"));
                };
                if !(1..8).contains(&civ) || !self.civ_alive(civ) {
                    return Applied::Skipped(format!("{receiver:?} is not a living civilization"));
                }
                let Ok(advance) = usize::try_from(*technology) else {
                    return Applied::Skipped(format!("invalid advance {technology}"));
                };
                if advance >= rules.tech_defs().len().min(100) {
                    return Applied::Skipped(format!("invalid advance {technology}"));
                }
                if self.grant_advance_with_rules(rules, civ, advance) {
                    Applied::Advance { civ, advance }
                } else {
                    Applied::Skipped("advance already known".into())
                }
            }
            // Handler 0x567ab4 → 0x4a2363: a living civ 1..=7 loses the
            // advance bit only (count, target and who-knows stay); with
            // Collapse, known advances that depend on it go first.
            Effect::TakeTechnology {
                whom,
                technology,
                collapse,
            } => {
                let Some(civ) = self.civ_by_name(whom) else {
                    return Applied::Skipped(format!("unknown civ {whom:?}"));
                };
                if !(1..8).contains(&civ) || !self.civ_alive(civ) {
                    return Applied::Skipped(format!("{whom:?} is not a living civilization"));
                }
                let Some(advance) = usize::try_from(*technology)
                    .ok()
                    .filter(|&a| a < rules.tech_defs().len().min(100))
                else {
                    return Applied::Skipped(format!("invalid advance {technology}"));
                };
                let prereqs: Vec<[Option<usize>; 2]> = if *collapse {
                    crate::research::TechTree::from_rules(rules)
                        .techs
                        .iter()
                        .map(|t| t.prereqs)
                        .collect()
                } else {
                    Vec::new()
                };
                let mut advances = Vec::new();
                self.take_advance(&prereqs, civ, advance, *collapse, &mut advances);
                if advances.is_empty() {
                    Applied::Skipped("advance not known".into())
                } else {
                    Applied::Taken { civ, advances }
                }
            }
            Effect::MakeAggression { who, whom } => {
                let (Some(a), Some(b)) = (self.civ_by_name(who), self.civ_by_name(whom)) else {
                    return Applied::Skipped(format!("unknown civ {who:?}/{whom:?}"));
                };
                if !(1..8).contains(&a) || !(1..8).contains(&b) || a == b {
                    return Applied::Skipped("needs two civilizations 1..=7".into());
                }
                if !self.civ_alive(a) || !self.civ_alive(b) {
                    return Applied::Skipped("both civilizations must be alive".into());
                }
                let consequences = self.make_aggression(a, b);
                Applied::War {
                    who: a,
                    whom: b,
                    consequences,
                }
            }
            Effect::ChangeTerrain {
                terrain,
                rect,
                map,
                exceptions,
            } => {
                if *map != 0 {
                    return Applied::Skipped(format!("map {map} is not modelled"));
                }
                let Some(t) = u8::try_from(*terrain).ok().filter(|&t| t <= 10) else {
                    return Applied::Skipped(format!("invalid terrain {terrain}"));
                };
                self.change_terrain(rules, t, rect, *exceptions)
            }
            Effect::Text { .. } | Effect::PlayTrack(_) | Effect::DontPlayWonders => {
                Applied::NoState
            }
        }
    }

    /// `0x4a2363`: dependants first (with `collapse`), then the bit.
    fn take_advance(
        &mut self,
        prereqs: &[[Option<usize>; 2]],
        civ: u8,
        advance: usize,
        collapse: bool,
        taken: &mut Vec<usize>,
    ) {
        let c = civ as usize;
        let knows = |w: &WorldState, a: usize| w.techs[c][a / 8] & (1 << (a % 8)) != 0;
        if collapse {
            for t in 0..prereqs.len().min(100) {
                if t != advance && knows(self, t) && depends_on(prereqs, t, advance) {
                    self.take_advance(prereqs, civ, t, collapse, taken);
                }
            }
        }
        if knows(self, advance) {
            self.techs[c][advance / 8] &= !(1 << (advance % 8));
            taken.push(advance);
        }
    }

    /// Treaty clear helper `0x454500`: both directions, with its cascades
    /// (alliance 4 → 8, war 0x2000 → 0x1800, contact 1 → 0x2000).
    fn treaty_clear(&mut self, a: u8, b: u8, mask: u32) {
        if mask & 4 != 0 {
            self.treaty_clear(a, b, 8);
        }
        if mask & 0x2000 != 0 {
            self.treaty_clear(a, b, 0x1800);
        }
        if mask & 1 != 0 {
            self.treaty_clear(a, b, 0x2000);
        }
        self.diplomacy[a as usize][b as usize] &= !mask;
        self.diplomacy[b as usize][a as usize] &= !mask;
    }

    /// Treaty set helper `0x4545d2`: both directions; 8 also sets 4, any
    /// of 0xe clears 0x2a60, war 0x2000 clears 0xe and adds 0x200000.
    fn treaty_set(&mut self, a: u8, b: u8, mut mask: u32) {
        if mask & 8 != 0 {
            self.treaty_set(a, b, 4);
        }
        if mask & 0xe != 0 {
            self.treaty_clear(a, b, 0x2a60);
        }
        if mask & 0x2000 != 0 {
            self.treaty_clear(a, b, 0xe);
            mask |= 0x200000;
        }
        self.diplomacy[a as usize][b as usize] |= mask;
        self.diplomacy[b as usize][a as usize] |= mask;
    }

    /// MAKEAGGRESSION (`0x53ac50`). Returns whether the original would
    /// also call its treaty-break consequences (`0x44af04`, not modelled):
    /// `whom` held a 0x6 treaty with `who` and either side is human.
    fn make_aggression(&mut self, who: u8, whom: u8) -> bool {
        let (a, b) = (who as usize, whom as usize);
        self.treaty_clear(who, whom, 0x60);
        let consequences =
            self.diplomacy[b][a] & 6 != 0 && self.human_mask & ((1 << who) | (1 << whom)) != 0;
        self.treaty_clear(who, whom, 0xc);
        if self.diplomacy[a][b] & 1 != 0 {
            self.treaty_set(who, whom, 0x2000);
            // 0x4546dc: attitude bytes (civ +0x48) set to 100 both ways.
            self.ai_state.attitudes[a][b] = 100;
            self.ai_state.attitudes[b][a] = 100;
            if self.diplomacy[b][a] & 0x1000 == 0 && self.diplomacy[a][b] & 0x1000 == 0 {
                self.diplomacy[a][b] |= 0x1000;
            }
        }
        consequences
    }

    /// CHANGETERRAIN (`0x4d2a3a`): x from corner 1 to corner 2, y from
    /// corner 1 to corner 4, inclusive. Tiles whose terrain is in the
    /// exception mask are skipped. First every city there is destroyed and
    /// every unit removed; then the terrain is set and the rest of tile
    /// bytes 0–3 cleared (river, resource, improvements, landmass).
    fn change_terrain(
        &mut self,
        rules: &Rules,
        terrain: u8,
        rect: &[(i32, i32)],
        exceptions: u16,
    ) -> Applied {
        let (x1, y1, x2, y4) = (rect[0].0, rect[0].1, rect[1].0, rect[3].1);
        let mut tiles = Vec::new();
        for y in y1..=y4 {
            for x in x1..=x2 {
                let (Ok(xu), Ok(yu)) = (u16::try_from(x), u16::try_from(y)) else {
                    continue;
                };
                let Some(i) = self.map.stored_index(xu, yu) else {
                    continue;
                };
                if exceptions & (1 << (self.map.terrain[i] & 0x0F)) == 0 && !tiles.contains(&i) {
                    tiles.push(i);
                }
            }
        }
        let on = |w: &WorldState, x: u16, y: u16| {
            w.map.stored_index(x, y).is_some_and(|i| tiles.contains(&i))
        };
        let doomed: Vec<usize> = (0..self.cities.len())
            .filter(|&k| {
                let c = &self.cities[k];
                c.city_id != 0 && c.map_id == 0 && on(self, c.x, c.y)
            })
            .collect();
        let cities = doomed
            .into_iter()
            .filter(|&k| self.destroy_surface_city(rules, k))
            .count();
        let mut units = 0;
        for k in (0..self.units.len()).rev() {
            let u = &self.units[k];
            if u.id != 0 && on(self, u.x, u.y) {
                self.remove_land_unit(k);
                units += 1;
            }
        }
        for &i in &tiles {
            let m = &mut self.map;
            m.terrain[i] = terrain;
            m.river[i] = false;
            m.special[i] = false;
            m.improvements[i] = 0;
            m.road[i] = false;
            m.rail[i] = false;
            m.site_bits[i] = 0;
            m.landmass[i] = 0;
            // Remembered improvements of civs that see the tile
            // (0x57027a; inferred: refreshed from the cleared byte).
            for c in 1..8 {
                if m.visible[i] & (1 << c) != 0 {
                    if let Some(k) = m.known_improvements[c - 1].get_mut(i) {
                        *k = 0;
                    }
                }
            }
        }
        Applied::Terrain {
            tiles: tiles.len(),
            cities,
            units,
        }
    }

    /// CREATEUNIT placement (`0x57054f`): on the map; a sea unit on ocean
    /// or in the owner's coastal city (city flag 0x80, inferred); others
    /// not on ocean unless air, and not in another civ's city; no other
    /// civ's units. The impassable-terrain test is not modelled.
    fn can_place(&self, civ: u8, domain: u8, x: u16, y: u16) -> bool {
        let Some(t) = self.map.terrain_at(x, y) else {
            return false;
        };
        let ocean = t == OCEAN_TERRAIN;
        let city = self
            .cities
            .iter()
            .find(|c| c.city_id != 0 && c.map_id == 0 && (c.x, c.y) == (x, y));
        let own_city = city.is_some_and(|c| (c.owner & 0x0F) as u8 == civ);
        let terrain_ok = match domain {
            2 => ocean || (own_city && city.is_some_and(|c| c.flags & 0x80 != 0)),
            1 => city.is_none() || own_city,
            _ => !ocean && (city.is_none() || own_city),
        };
        terrain_ok
            && !self
                .units
                .iter()
                .any(|u| (u.x, u.y) == (x, y) && u.owner != civ)
    }
}

/// `0x49c8d4`: advance `t` needs `of` somewhere in its prerequisite tree.
fn depends_on(prereqs: &[[Option<usize>; 2]], t: usize, of: usize) -> bool {
    let mut stack = vec![t];
    let mut seen = vec![false; prereqs.len()];
    while let Some(a) = stack.pop() {
        for p in prereqs.get(a).into_iter().flatten().flatten() {
            if *p == of {
                return true;
            }
            if *p < seen.len() && !seen[*p] {
                seen[*p] = true;
                stack.push(*p);
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::synthetic_save;

    const RULES: &str = "@COSMIC\n3 ; road\n2 ; trireme\n2 ; food\n10 ; food rows\n\
10 ; shield rows\n@UNITS\nSettlers, nil, 0, 1., 0, 0a, 1d, 1h, 1f, 3, 0, 5, nil, 000000000000000\n\
Warriors, nil, 0, 1., 0, 1a, 1d, 1h, 1f, 1, 0, 1, nil, 000000000000000\n\
@IMPROVE\nNothing, 0, 0, nil\nPalace, 10, 0, nil\n\
@CIVILIZE\nAlphabet, 5, 1, nil, nil, 0, 3 ; Alp\nCode of Laws, 4, 1, Alp, nil, 0, 2 ; CoL\n\
Writing, 4, 2, Alp, nil, 0, 3 ; Wri\nMonarchy, 5, 1, CoL, nil, 0, 2 ; Mon\n\
Bronze Working, 6, -1, nil, nil, 0, 4 ; Bro\n\
@TERRAIN\nDesert, 1, 2, 0, 1, 0\nPlains, 1, 2, 1, 1, 0\nGrassland, 1, 2, 2, 0, 0\n";

    /// Synthetic save: civ 1 "Testers" (cities at (k+1, k+2), a Settlers
    /// at (3,4)), civ 2 renamed "Others" (a Warriors at (5,6)).
    fn world() -> (WorldState, Rules) {
        let mut w = crate::boot(&synthetic_save(), 1);
        if let Some(c) = w.civs.iter_mut().find(|c| c.id == 2) {
            c.plural = "Others".into();
        }
        (w, tot_kit::parse_rules(RULES))
    }

    #[test]
    fn take_technology_with_and_without_collapse() {
        let (mut w, rules) = world();
        for a in 0..5 {
            w.techs[1][0] |= 1 << a;
        }
        let acquired = w.research[1].acquired;
        let take = |collapse| Effect::TakeTechnology {
            whom: "Testers".into(),
            technology: 0,
            collapse,
        };
        let mut plain = w.clone();
        assert_eq!(
            plain.apply_effect(&rules, &take(false)),
            Applied::Taken {
                civ: 1,
                advances: vec![0]
            }
        );
        // Dependants first, in index order with depth first: Monarchy
        // (via Code of Laws), Code of Laws, Writing, then Alphabet.
        assert_eq!(
            w.apply_effect(&rules, &take(true)),
            Applied::Taken {
                civ: 1,
                advances: vec![3, 1, 2, 0]
            }
        );
        assert_eq!(w.techs[1][0], 1 << 4, "Bronze Working stays");
        assert_eq!(w.research[1].acquired, acquired, "the count is untouched");
        assert!(matches!(
            w.apply_effect(&rules, &take(false)),
            Applied::Skipped(_)
        ));
    }

    #[test]
    fn make_aggression_follows_the_treaty_helpers() {
        let (mut w, rules) = world();
        w.human_mask = 0;
        // contact + peace + alliance both ways, a 0x40 bit on one side.
        w.diplomacy[1][2] = 1 | 4 | 8 | 0x40;
        w.diplomacy[2][1] = 1 | 4 | 8;
        let war = Effect::MakeAggression {
            who: "Testers".into(),
            whom: "Others".into(),
        };
        let mut human = w.clone();
        assert_eq!(
            w.apply_effect(&rules, &war),
            Applied::War {
                who: 1,
                whom: 2,
                consequences: false
            }
        );
        assert_eq!(w.diplomacy[1][2], 1 | 0x2000 | 0x200000 | 0x1000);
        assert_eq!(w.diplomacy[2][1], 1 | 0x2000 | 0x200000);
        assert_eq!(
            (w.ai_state.attitudes[1][2], w.ai_state.attitudes[2][1]),
            (100, 100)
        );
        // A human party with a 0x6 treaty: the original's treaty-break
        // consequences would run (not modelled, reported).
        human.human_mask = 1 << 2;
        assert!(matches!(
            human.apply_effect(&rules, &war),
            Applied::War {
                consequences: true,
                ..
            }
        ));
        // Without contact, treaties still end but no war is declared.
        let (mut w, _) = world();
        w.diplomacy[1][2] = 4;
        w.diplomacy[2][1] = 4;
        w.apply_effect(&rules, &war);
        assert_eq!((w.diplomacy[1][2], w.diplomacy[2][1]), (0, 0));
    }

    /// City0 at (2,0), the Settlers at (4,2), the Others' Warriors at
    /// (4,0), City2 (with a Palace) at (6,2): all on the 8×4 map.
    fn placed() -> (WorldState, Rules) {
        let (mut w, rules) = world();
        (w.cities[0].x, w.cities[0].y, w.cities[0].city_id) = (2, 0, 1);
        (w.cities[2].x, w.cities[2].y, w.cities[2].city_id) = (6, 2, 3);
        w.cities[1].city_id = 2;
        w.cities[2].buildings[0] |= 1 << crate::founding::PALACE;
        (w.units[0].x, w.units[0].y, w.units[0].home) = (4, 2, 0xff);
        (w.units[1].x, w.units[1].y) = (4, 0);
        (w, rules)
    }

    #[test]
    fn change_terrain_clears_tiles_cities_and_units() {
        let (mut w, rules) = placed();
        let at = |w: &WorldState, x, y| w.map.stored_index(x, y).unwrap();
        let (city, unit) = (at(&w, 2, 0), at(&w, 4, 2));
        w.map.road[city] = true;
        let change = |rect: Vec<(i32, i32)>, exceptions| Effect::ChangeTerrain {
            terrain: 0,
            rect,
            map: 0,
            exceptions,
        };
        // The tile's own terrain excepted: nothing happens.
        let before = w.map.terrain.clone();
        let own = 1 << w.map.terrain[city];
        assert_eq!(
            w.apply_effect(&rules, &change(vec![(2, 0); 4], own)),
            Applied::Terrain {
                tiles: 0,
                cities: 0,
                units: 0
            }
        );
        assert_eq!(w.map.terrain, before);
        // x 2..=2 from corners 1-2, y 0..=2 from corners 1 and 4.
        let rect = vec![(2, 0), (2, 9), (7, 7), (9, 2)];
        let Applied::Terrain { tiles, cities, .. } = w.apply_effect(&rules, &change(rect, 0))
        else {
            panic!()
        };
        assert_eq!((tiles, cities), (3, 1));
        assert_eq!(w.cities[0].city_id, 0);
        assert_eq!(w.map.terrain[city], 0);
        assert!(!w.map.road[city]);
        assert_eq!(w.map.improvements[city], 0);
        assert!(w.units.iter().any(|u| u.id != 0 && (u.x, u.y) == (4, 2)));
        let Applied::Terrain { units, .. } = w.apply_effect(&rules, &change(vec![(4, 2); 4], 0))
        else {
            panic!()
        };
        assert_eq!(units, 1);
        assert!(!w.units.iter().any(|u| u.id != 0 && (u.x, u.y) == (4, 2)));
        assert_eq!(w.map.terrain[unit], 0);
    }

    #[test]
    fn create_unit_in_capital_randomize_and_home() {
        let (mut w, rules) = placed();
        let create = |randomize, in_capital, count| Effect::CreateUnit {
            unit_name: "Warriors".into(),
            owner: "Testers".into(),
            veteran: true,
            count,
            locations: vec![(0, 0), (0, 2), (2, 2)],
            randomize,
            in_capital,
            home: Some("City1".into()),
        };
        let Applied::Created { first, at, .. } = w.apply_effect(&rules, &create(false, true, 1))
        else {
            panic!()
        };
        assert_eq!(at, (6, 2), "City2, the Palace city");
        assert_eq!((w.units[first].home, w.units[first].veteran), (1, true));
        // Randomize: one CRT draw per unit, scanning on from the start.
        let draws = w.combat_rng.draws();
        let Applied::Created { count, .. } = w.apply_effect(&rules, &create(true, false, 3)) else {
            panic!()
        };
        assert_eq!((count, w.combat_rng.draws() - draws), (3, 3));
        // Another civ's units or city block a tile.
        let others = |locations| Effect::CreateUnit {
            unit_name: "Warriors".into(),
            owner: "Others".into(),
            veteran: false,
            count: 1,
            locations,
            randomize: false,
            in_capital: false,
            home: Some("City1".into()),
        };
        assert!(matches!(
            w.apply_effect(&rules, &others(vec![(4, 2), (2, 0)])),
            Applied::Skipped(_)
        ));
        let Applied::Created { first, at, .. } =
            w.apply_effect(&rules, &others(vec![(4, 2), (4, 0)]))
        else {
            panic!()
        };
        assert_eq!(at, (4, 0));
        assert_eq!(w.units[first].home, 0xff, "City1 is not theirs");
    }
}
