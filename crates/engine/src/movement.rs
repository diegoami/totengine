//! Movement kernel (T1-3): terrain costs, roads/railroads, domains, zone
//! of control.
//!
//! Movement is tracked in **thirds of a movement point** (`moves_left`):
//! a unit with rate 1 starts each turn with 3. Terrain cost `c` costs
//! `3*c` thirds; a road tile costs `1` third (three road tiles per point,
//! `@COSMIC` road multiplier 3); railroad costs 0; and a unit with any
//! movement left can always enter one tile (classic "always move at least
//! once" rule). Zone of control follows classic Civ2: moving from a tile
//! adjacent to an enemy into another such tile consumes all remaining
//! movement.
//!
//! Rules source: the *scenario's* Rules.txt (never the global one —
//! scenarios override @UNITS/@TERRAIN; issue #30). Column identifications
//! are evidence-backed in `tot_kit::rules` (`unit_move` field 3,
//! `terrain_move_cost` field 1, `road_multiplier` @COSMIC).

use tot_kit::Rules;

use crate::WorldState;

/// Outcome of a single [`WorldState::move_unit`] attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveOutcome {
    pub moved: bool,
    /// Cost charged, in thirds of a movement point.
    pub cost_thirds: u32,
    /// Movement was terminated by zone of control.
    pub zoc_ended: bool,
    /// The destination held another civ's units: the move became an
    /// attack (T1-6, [`WorldState::attack`]); the unit did not move.
    pub battle: Option<crate::combat::Battle>,
    /// The unit entered a goody hut ([`crate::MapState::hut_at`]). The hut
    /// is gone: the tile is now owned. Its outcome (gold, tech, units,
    /// barbarians) is not decoded yet and is not applied.
    pub hut: bool,
}

/// Ocean terrain id in every Rules.txt observed (id 10 — the #44 lesson:
/// an off-by-one parser shifts this; `Rules::terrain_semantic_ok` guards).
pub const OCEAN_TERRAIN: u8 = 10;

/// Neighbouring tiles on the logical diamond grid (x + y is even for every
/// tile): the 8 offsets (±1, ±1), (±2, 0), (0, ±2). Oracle battles used
/// three of them: NE (21,31)→(22,30), N (21,31)→(21,29), E (24,34)→(26,34).
/// East-west map wrap is not modelled: a step across the seam is refused.
pub fn is_adjacent(a: (u16, u16), b: (u16, u16)) -> bool {
    let dx = (a.0 as i32 - b.0 as i32).abs();
    let dy = (a.1 as i32 - b.1 as i32).abs();
    matches!((dx, dy), (1, 1) | (2, 0) | (0, 2))
}

impl WorldState {
    /// Reset every unit's movement for a new turn (`rate * 3` thirds).
    /// Unknown unit types get 0 (they cannot move until the rules table
    /// is fixed — loud in tests via the boot summary).
    pub fn start_turn(&mut self, rules: &Rules) {
        for u in &mut self.units {
            u.moves_left = rules.unit_move(u.type_id as usize).unwrap_or(0) * 3;
        }
    }

    /// Move `unit_idx` to logical tile (x, y). Returns what happened; the
    /// unit stays put when the move is illegal (domain, impassable
    /// destination, or no movement left).
    pub fn move_unit(&mut self, rules: &Rules, unit_idx: usize, x: u16, y: u16) -> MoveOutcome {
        let none = MoveOutcome {
            moved: false,
            cost_thirds: 0,
            zoc_ended: false,
            battle: None,
            hut: false,
        };
        let Some(unit) = self.units.get(unit_idx) else {
            return none;
        };
        // one step per request: the destination must be a neighbour
        // (reviewer finding #62 — no teleporting moves or attacks)
        if !is_adjacent((unit.x, unit.y), (x, y)) || unit.moves_left == 0 {
            return none;
        }
        let owner = unit.owner;
        if self
            .units
            .iter()
            .any(|u| (u.x, u.y) == (x, y) && u.owner != owner)
        {
            return MoveOutcome {
                battle: self.attack(rules, unit_idx, x, y),
                ..none
            };
        }
        let domain = rules.unit_domain(unit.type_id as usize).unwrap_or(0);
        let Some(dest_terrain) = self.map.terrain_at(x, y) else {
            return none; // outside the decodable grid
        };
        let is_ocean = dest_terrain == OCEAN_TERRAIN;
        // Domain gating: sea units need ocean, land units need land.
        // (Cities are land for this purpose and handled by city logic
        // later; air units ignore terrain.)
        match domain {
            2 if !is_ocean => return none,
            0 if is_ocean => return none,
            _ => {}
        }
        let Some(dest_idx) = self.map.stored_index(x, y) else {
            return none;
        };
        let base_cost = rules
            .terrain_move_cost(dest_terrain as usize)
            .unwrap_or(1)
            .max(1);
        let mut cost = base_cost * 3; // thirds
        if self.map.rail[dest_idx] {
            cost = 0;
        } else if self.map.road[dest_idx] {
            // road multiplier: M road tiles per movement point
            let mult = rules.road_multiplier().unwrap_or(3).max(1);
            cost = (3 / mult).max(1);
        }
        // classic rule: any movement left ⇒ at least one tile
        let charge = cost.min(unit.moves_left);
        // ZOC: leaving an enemy-adjacent tile into another enemy-adjacent
        // tile consumes all remaining movement (classic Civ2).
        let origin_zoc = self.is_zoc(rules, unit.owner, unit.x, unit.y);
        let dest_zoc = self.is_zoc(rules, unit.owner, x, y);
        let zoc_ended = origin_zoc && dest_zoc;
        let Some(unit) = self.units.get_mut(unit_idx) else {
            return none;
        };
        unit.x = x;
        unit.y = y;
        let civ = unit.owner;
        unit.moves_left = if zoc_ended {
            0
        } else {
            unit.moves_left.saturating_sub(charge)
        };
        let hut = self.map.hut_at(x, y);
        self.map.owner[dest_idx] = civ & 0x0F;
        MoveOutcome {
            moved: true,
            cost_thirds: charge,
            zoc_ended,
            battle: None,
            hut,
        }
    }

    /// True when (x, y) is adjacent ([`is_adjacent`]) to any unit of
    /// another civilization. `owner` 0 (barbarians) is hostile to everyone;
    /// everyone is hostile to owner 0.
    fn is_zoc(&self, _rules: &Rules, owner: u8, x: u16, y: u16) -> bool {
        self.units
            .iter()
            .any(|u| u.owner != owner && is_adjacent((u.x, u.y), (x, y)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Lcg64;
    use crate::Unit;

    fn rules_text() -> &'static str {
        "@COSMIC\r\n3       ; Road movement multiplier\r\n\
         @TERRAIN\r\nDesert,  1, 2,  0,1,0,  no, 0, 0, 0,  no, 0, 0, 0,  no, no\r\n\
         Plains,  1, 2,  1,1,0,  yes, 1, 5, 1,  For, 0,15, 0,  Grs, no\r\n\
         Grassland,  1, 2,  2,1,0,  yes, 1, 5, 2,  For, 0,10, 0,  Hil, no\r\n\
         Forest,  2, 3,  1,2,0,  Pln, 0, 5, 5,  no,  0, 5, 0,  Grs, no\r\n\
         Hills,   2, 4,  1,0,0,  yes, 1,10, 0,  yes, 3,10, 1,  Pln, no\r\n\
         Mountains,3,6,  0,1,0,  no,  0, 0, 0,  yes, 1,10, 6,  Hil, no\r\n\
         Tundra,  1, 2,  1,0,0,  yes, 1,10, 1,  no,  0, 0, 0,  Drt, no\r\n\
         Glacier, 2, 2,  0,0,0,  no,  0, 0, 0,  yes, 1,15, 3,  Tun, no\r\n\
         Swamp,   2, 3,  1,0,0,  Grs, 0,15, 6,  For, 0,15, 0,  Pln, no\r\n\
         Jungle,  2, 3,  1,0,0,  Grs, 0,15, 6,  For, 0,15, 0,  Pln, no\r\n\
         Ocean,   1, 2,  1,0,2,  no,  0, 0, 0,  no,  0, 0, 0,  no, no\r\n\
         @UNITS\r\n\
         Settlers,     Exp, 0,  1.,0,  0a,1d,  2h,1f,  4,0,  5, nil, 000000000000000\r\n\
         Knights,      Ldr, 0,  2.,0,  4a,2d,  1h,1f,  4,0,  0, Chi, 000000000000000\r\n\
         Trireme,      Nav, 2,  3.,0,  1a,1d,  1h,1f,  4,2,  4, Map, 000000000100000\r\n"
    }

    fn world_with_map() -> WorldState {
        // 4x4 stored grid. Logical (x, y) maps to stored y*4 + x/2 (the
        // game's accessor, evidence §Map tile framing).
        //   tile (2,0) [stored 1]  = ocean
        //   stored row 2           = road strip (logical y=2)
        let surface = 16usize;
        let mut terrain = vec![1u8; surface];
        let mut road = vec![false; surface];
        terrain[1] = OCEAN_TERRAIN;
        for r in road.iter_mut().take(12).skip(8) {
            *r = true;
        }
        WorldState {
            turn: 1,
            year: -4000,
            difficulty: 4,
            rivals: 1,
            seed_word: 0x0200,
            game_flags: 0x10,
            human_mask: 0x02,
            wonders: [-1; 28],
            treasury: [0; 8],
            civs: vec![],
            units: vec![
                Unit {
                    type_id: 1, // Knights, move 2
                    owner: 1,
                    x: 5,
                    y: 1,
                    is_leader: false,
                    moves_left: 6,
                    veteran: false,
                    damage: 0,
                    orders: 0xff,
                    stack_pos: 0,
                    home: 0xff,
                },
                Unit {
                    type_id: 2, // Trireme, sea on (2,0)=ocean
                    owner: 1,
                    x: 2,
                    y: 0,
                    is_leader: false,
                    moves_left: 9,
                    veteran: false,
                    damage: 0,
                    orders: 0xff,
                    stack_pos: 0,
                    home: 0xff,
                },
                Unit {
                    type_id: 1,
                    owner: 2, // enemy knight for ZOC
                    x: 7,
                    y: 3,
                    is_leader: false,
                    moves_left: 6,
                    veteran: false,
                    damage: 0,
                    orders: 0xff,
                    stack_pos: 0,
                    home: 0xff,
                },
            ],
            cities: vec![],
            map: crate::MapState {
                stored_w: 4,
                stored_h: 4,
                terrain,
                special: vec![false; surface],
                river: vec![false; surface],
                road,
                rail: vec![false; surface],
                owner: vec![0xF; surface],
                resource_seed: 0,
                huts_enabled: false,
            },
            rng: Lcg64::new(1),
            combat_rng: crate::combat::CrtRand::new(1),
            rng_seed: 1,
        }
    }

    #[test]
    fn start_turn_sets_moves_from_rules() {
        let rules = tot_kit::parse_rules(rules_text());
        let mut w = world_with_map();
        w.start_turn(&rules);
        assert_eq!(w.units[0].moves_left, 6, "Knights move 2 ⇒ 6 thirds");
        assert_eq!(w.units[1].moves_left, 9, "Trireme move 3 ⇒ 9 thirds");
    }

    #[test]
    fn plains_cost_one_point_three_thirds() {
        let rules = tot_kit::parse_rules(rules_text());
        let mut w = world_with_map();
        w.start_turn(&rules);
        // (5,1) → (6,0) NE: plains, no road (road only on stored row 2)
        let out = w.move_unit(&rules, 0, 6, 0);
        assert!(out.moved);
        assert_eq!(out.cost_thirds, 3);
        assert_eq!(w.units[0].moves_left, 3);
    }

    #[test]
    fn road_costs_one_third() {
        let rules = tot_kit::parse_rules(rules_text());
        let mut w = world_with_map();
        w.start_turn(&rules);
        let out = w.move_unit(&rules, 0, 6, 2); // SE, stored row 2 road
        assert!(out.moved);
        assert_eq!(out.cost_thirds, 1);
    }

    #[test]
    fn terrain_cost_scales_and_min_charge() {
        let rules = tot_kit::parse_rules(rules_text());
        let mut w = world_with_map();
        // forest (cost 2 ⇒ 6 thirds) at (6,0); unit has 6 ⇒ lands with 0
        let idx = w.map.stored_index(6, 0).unwrap();
        w.map.terrain[idx] = 3; // Forest
        let out = w.move_unit(&rules, 0, 6, 0);
        assert!(out.moved);
        assert_eq!(out.cost_thirds, 6);
        assert_eq!(w.units[0].moves_left, 0);
    }

    #[test]
    fn entering_a_hut_takes_it() {
        let rules = tot_kit::parse_rules(rules_text());
        let mut w = world_with_map();
        w.start_turn(&rules);
        w.map.huts_enabled = true;
        // pick the first seed that puts a hut on (6,0), NE of the knight
        w.map.resource_seed = (0..u16::MAX)
            .find(|&s| crate::placement::hut_hash(s, 6, 0))
            .unwrap();
        assert!(w.map.hut_at(6, 0));
        let out = w.move_unit(&rules, 0, 6, 0);
        assert!(out.moved && out.hut);
        let i = w.map.stored_index(6, 0).unwrap();
        assert_eq!(w.map.owner[i], 1, "entering makes the tile the civ's");
        assert!(!w.map.hut_at(6, 0), "an owned tile holds no hut");
        // back and forth: the hut does not come back
        w.units[0].moves_left = 6;
        assert!(!w.move_unit(&rules, 0, 5, 1).hut);
        assert!(!w.move_unit(&rules, 0, 6, 0).hut);
    }

    #[test]
    fn domain_gating() {
        let rules = tot_kit::parse_rules(rules_text());
        let mut w = world_with_map();
        w.start_turn(&rules);
        // land unit at (3,1) onto ocean (2,0), NW: refused
        w.units[0].x = 3;
        assert!(!w.move_unit(&rules, 0, 2, 0).moved);
        // sea unit (2,0) onto land (4,0), E: refused
        assert!(!w.move_unit(&rules, 1, 4, 0).moved);
    }

    #[test]
    fn zoc_ends_movement_when_crossing() {
        let rules = tot_kit::parse_rules(rules_text());
        let mut w = world_with_map();
        w.start_turn(&rules);
        // enemy knight at (7,3). Put our knight adjacent at (5,3) (E of
        // it is the enemy) and move NE to (6,2) — also adjacent ⇒ ZOC end.
        w.units[0].x = 5;
        w.units[0].y = 3;
        let out = w.move_unit(&rules, 0, 6, 2);
        assert!(out.moved);
        assert!(out.zoc_ended);
        assert_eq!(w.units[0].moves_left, 0);
    }

    #[test]
    fn no_moves_left_means_no_move() {
        let rules = tot_kit::parse_rules(rules_text());
        let mut w = world_with_map();
        w.start_turn(&rules);
        w.units[0].moves_left = 0;
        assert!(!w.move_unit(&rules, 0, 6, 0).moved);
    }

    /// Reviewer finding #62: one request moves or attacks one step only.
    #[test]
    fn non_adjacent_destinations_are_refused() {
        let rules = tot_kit::parse_rules(rules_text());
        let mut w = world_with_map();
        w.start_turn(&rules);
        // empty land two steps away (E of E)
        let out = w.move_unit(&rules, 0, 9, 1);
        assert!(!out.moved && out.battle.is_none());
        assert_eq!(
            (w.units[0].x, w.units[0].y, w.units[0].moves_left),
            (5, 1, 6)
        );
        // the enemy at (7,3) is two steps away: no battle, either entry point
        assert!(w.move_unit(&rules, 0, 7, 3).battle.is_none());
        assert!(w.attack(&rules, 0, 7, 3).is_none());
        assert_eq!(w.units.len(), 3);
        // the eight diamond neighbours, and nothing else
        let n: Vec<_> = (0..12u16)
            .flat_map(|x| (0..8u16).map(move |y| (x, y)))
            .filter(|&t| is_adjacent((5, 3), t))
            .collect();
        assert_eq!(
            n,
            [
                (3, 3),
                (4, 2),
                (4, 4),
                (5, 1),
                (5, 5),
                (6, 2),
                (6, 4),
                (7, 3)
            ]
        );
    }
}
