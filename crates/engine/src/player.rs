//! The player interface (RG-8, v0.5.0; decision 0003).
//!
//! Every civ is driven by a [`Player`]. The player receives an
//! [`Observation`] (what its civ can see) and returns [`Order`]s. The
//! [`Game`] validates and applies each order and records it, with its
//! result, in an order log; a game replays bit-exact from its save, seed
//! and log ([`Game::replay`]).
//!
//! Fog is the engine's own rule, not decoded from the original. A civ
//! currently sees the tiles within distance 1 of its units and within the
//! 21-tile radius of its cities, and seen tiles stay seen (tile byte +4,
//! bit c).

use std::fmt;

use tot_kit::Rules;

use crate::events::EventEngine;
use crate::placement::diamond_distance;
use crate::rng::{GameRng, Lcg64};
use crate::yields::{CityOutput, RADIUS_DX, RADIUS_DY};
use crate::WorldState;

/// Orders a player may give (decision 0003). `FoundCity` and
/// `SetResearch` come with AI v0 (#75).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Move unit `unit` (index in [`WorldState::units`]) to a diamond
    /// neighbour; onto foreign units this is an attack.
    Move { unit: usize, to: (u16, u16) },
    /// Set city `city`'s build item: unit type ≥ 0, improvement `-id` < 0.
    SetProduction { city: usize, item: i8 },
    /// End this civ's turn.
    EndTurn,
}

/// What applying an [`Order`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderResult {
    Moved,
    /// The move was an attack; `won` = the attacker won.
    Attacked {
        won: bool,
    },
    ProductionSet,
    TurnEnded,
    /// Nothing changed; the reason says why.
    Rejected(&'static str),
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Order::Move { unit, to } => write!(f, "move {unit} {} {}", to.0, to.1),
            Order::SetProduction { city, item } => write!(f, "prod {city} {item}"),
            Order::EndTurn => write!(f, "end"),
        }
    }
}

impl fmt::Display for OrderResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderResult::Moved => write!(f, "moved"),
            OrderResult::Attacked { won } => {
                write!(f, "attack {}", if *won { "won" } else { "lost" })
            }
            OrderResult::ProductionSet => write!(f, "prod-set"),
            OrderResult::TurnEnded => write!(f, "ended"),
            OrderResult::Rejected(why) => write!(f, "rejected {}", why.replace(' ', "_")),
        }
    }
}

impl Order {
    fn parse(s: &str) -> Option<Order> {
        let f: Vec<&str> = s.split_whitespace().collect();
        match f.as_slice() {
            ["move", u, x, y] => Some(Order::Move {
                unit: u.parse().ok()?,
                to: (x.parse().ok()?, y.parse().ok()?),
            }),
            ["prod", c, i] => Some(Order::SetProduction {
                city: c.parse().ok()?,
                item: i.parse().ok()?,
            }),
            ["end"] => Some(Order::EndTurn),
            _ => None,
        }
    }
}

/// One unit as an observer sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitView {
    pub index: usize,
    pub type_id: u8,
    pub owner: u8,
    pub x: u16,
    pub y: u16,
    /// Own units only: moves left (thirds); 0 for foreign units.
    pub moves_left: u32,
    pub veteran: bool,
}

/// One city as an observer sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CityView {
    pub index: usize,
    pub name: String,
    pub owner: u8,
    pub x: u16,
    pub y: u16,
    pub size: u16,
    /// Own cities only: build item, shield box and output.
    pub building: Option<i8>,
    pub shield_box: Option<u16>,
    pub output: Option<CityOutput>,
}

/// What civ `civ` knows at the moment of the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub civ: u8,
    pub turn: i16,
    pub treasury: i32,
    pub government: u8,
    pub techs: [u8; 13],
    /// Logical x range and height of the map.
    pub w2: u16,
    pub h: u16,
    /// Per stored tile: `Some(terrain)` where the civ has seen it.
    pub seen_terrain: Vec<Option<u8>>,
    pub units: Vec<UnitView>,
    pub cities: Vec<CityView>,
    /// The result of this player's previous order this turn, if any.
    pub last_result: Option<OrderResult>,
}

/// A player: given what its civ sees, the next order. The driver calls
/// [`Player::act`] until it returns [`Order::EndTurn`] (or an engine
/// action cap is reached).
pub trait Player {
    fn act(&mut self, obs: &Observation) -> Order;
}

/// Engine guard against a player that never ends its turn.
pub const ACTION_CAP: usize = 200;

impl WorldState {
    /// Mark the tiles a civ currently sees as seen: distance ≤ 1 around its
    /// units, the city radius around its cities.
    pub fn reveal(&mut self, civ: u8) {
        let bit = 1u8 << (civ & 7);
        let mut spots: Vec<(i32, i32)> = Vec::new();
        for u in self.units.iter().filter(|u| u.id != 0 && u.owner == civ) {
            spots.push((u.x as i32, u.y as i32));
            for (dx, dy) in NEIGHBOURS {
                spots.push((u.x as i32 + dx, u.y as i32 + dy));
            }
        }
        for c in self.cities.iter().filter(|c| (c.owner & 0x0F) as u8 == civ) {
            for k in 0..21 {
                spots.push((c.x as i32 + RADIUS_DX[k], c.y as i32 + RADIUS_DY[k]));
            }
        }
        let w2 = 2 * self.map.stored_w as i32;
        for (x, y) in spots {
            if y < 0 || w2 == 0 {
                continue;
            }
            if let Some(i) = self.map.stored_index(x.rem_euclid(w2) as u16, y as u16) {
                self.map.visible[i] |= bit;
            }
        }
    }

    fn sees_now(&self, civ: u8, x: u16, y: u16) -> bool {
        let w2 = 2 * self.map.stored_w as i32;
        let d =
            |a: (u16, u16)| diamond_distance(w2, false, a.0 as i32, a.1 as i32, x as i32, y as i32);
        self.units
            .iter()
            .any(|u| u.id != 0 && u.owner == civ && d((u.x, u.y)) <= 1)
            || self.cities.iter().any(|c| {
                (c.owner & 0x0F) as u8 == civ
                    && (0..21).any(|k| {
                        (c.x as i32 + RADIUS_DX[k]).rem_euclid(w2.max(1)) == x as i32
                            && c.y as i32 + RADIUS_DY[k] == y as i32
                    })
            })
    }

    /// The observation of `civ` (decision 0003).
    pub fn observe(&self, rules: &Rules, civ: u8, last: Option<OrderResult>) -> Observation {
        let bit = 1u8 << (civ & 7);
        let c = civ as usize;
        let units = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.id != 0 && (u.owner == civ || self.sees_now(civ, u.x, u.y)))
            .map(|(i, u)| UnitView {
                index: i,
                type_id: u.type_id,
                owner: u.owner,
                x: u.x,
                y: u.y,
                moves_left: if u.owner == civ { u.moves_left } else { 0 },
                veteran: u.veteran,
            })
            .collect();
        let cities = self
            .cities
            .iter()
            .enumerate()
            .filter(|(_, k)| {
                let own = (k.owner & 0x0F) as u8 == civ;
                own || self
                    .map
                    .stored_index(k.x, k.y)
                    .is_some_and(|i| self.map.visible[i] & bit != 0)
            })
            .map(|(i, k)| {
                let own = (k.owner & 0x0F) as u8 == civ;
                CityView {
                    index: i,
                    name: k.name.clone(),
                    owner: (k.owner & 0x0F) as u8,
                    x: k.x,
                    y: k.y,
                    size: k.size,
                    building: own.then_some(k.building),
                    shield_box: own.then_some(k.shield_box),
                    output: own.then(|| self.city_output(rules, i)),
                }
            })
            .collect();
        Observation {
            civ,
            turn: self.turn,
            treasury: self.treasury.get(c).copied().unwrap_or(0),
            government: self.governments.get(c).copied().unwrap_or(0),
            techs: self.techs.get(c).copied().unwrap_or([0; 13]),
            w2: (2 * self.map.stored_w) as u16,
            h: self.map.stored_h as u16,
            seen_terrain: self
                .map
                .terrain
                .iter()
                .zip(&self.map.visible)
                .map(|(&t, &v)| (v & bit != 0).then_some(t))
                .collect(),
            units,
            cities,
            last_result: last,
        }
    }

    fn knows(&self, rules: &Rules, civ: u8, raw: Option<&str>) -> bool {
        match raw {
            None | Some("nil") => true,
            Some("no") => false,
            Some(abbrev) => {
                let tree = crate::research::TechTree::from_rules(rules);
                tree.by_abbrev(abbrev).is_some_and(|t| {
                    self.techs
                        .get(civ as usize)
                        .and_then(|b| b.get(t >> 3))
                        .is_some_and(|b| b >> (t & 7) & 1 != 0)
                })
            }
        }
    }

    /// Validate and apply one order of `civ` (decision 0003).
    pub fn apply_order(
        &mut self,
        rules: &Rules,
        events: Option<&mut EventEngine>,
        civ: u8,
        order: Order,
    ) -> OrderResult {
        match order {
            Order::EndTurn => OrderResult::TurnEnded,
            Order::SetProduction { city, item } => {
                let Some(c) = self.cities.get(city) else {
                    return OrderResult::Rejected("no such city");
                };
                if (c.owner & 0x0F) as u8 != civ {
                    return OrderResult::Rejected("not your city");
                }
                let ok = if item >= 0 {
                    let t = item as usize;
                    t < rules.unit_names().len() && self.knows(rules, civ, rules.unit_prereq_raw(t))
                } else {
                    let id = -(item as i32) as usize;
                    (1..40).contains(&id)
                        && rules.improvement_cost(id).is_some()
                        && !c.has_building(id)
                        && self.knows(rules, civ, rules.improvement_prereq_raw(id))
                };
                if !ok {
                    return OrderResult::Rejected("cannot build");
                }
                self.cities[city].building = item;
                OrderResult::ProductionSet
            }
            Order::Move { unit, to } => {
                let Some(u) = self.units.get(unit) else {
                    return OrderResult::Rejected("no such unit");
                };
                if u.id == 0 || u.owner != civ {
                    return OrderResult::Rejected("not your unit");
                }
                if u.moves_left == 0 {
                    return OrderResult::Rejected("no moves left");
                }
                if !crate::movement::is_adjacent((u.x, u.y), to) {
                    return OrderResult::Rejected("not adjacent");
                }
                let out = match events {
                    Some(ev) => self.move_unit_with_events(rules, ev, unit, to.0, to.1).0,
                    None => self.move_unit(rules, unit, to.0, to.1),
                };
                self.reveal(civ);
                match (out.moved, &out.battle) {
                    (_, Some(b)) => OrderResult::Attacked {
                        won: b.outcome.attacker_won,
                    },
                    (true, None) => OrderResult::Moved,
                    (false, None) => OrderResult::Rejected("move refused"),
                }
            }
        }
    }

    /// Civs that still have a unit or a city (barbarians excluded).
    pub fn living_civs(&self) -> Vec<u8> {
        (1..8u8)
            .filter(|&c| {
                self.units.iter().any(|u| u.id != 0 && u.owner == c)
                    || self.cities.iter().any(|k| (k.owner & 0x0F) as u8 == c)
            })
            .collect()
    }
}

const NEIGHBOURS: [(i32, i32); 8] = [
    (1, -1),
    (2, 0),
    (1, 1),
    (0, 2),
    (-1, 1),
    (-2, 0),
    (-1, -1),
    (0, -2),
];

/// FNV-1a 64-bit, for the save fingerprint in the log header.
fn fnv64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// A game in progress: the world, its rules and events, a player per civ
/// and the order log (decision 0003).
pub struct Game {
    pub world: WorldState,
    pub rules: Rules,
    pub events: Option<EventEngine>,
    /// Index = civ; `None` = no player (the civ only sits through turns).
    pub players: Vec<Option<Box<dyn Player>>>,
    log: String,
}

impl Game {
    /// Boot a game from save bytes; the log header records the seed and a
    /// fingerprint of the save.
    pub fn new(
        save_name: &str,
        save: &[u8],
        seed: u64,
        rules: Rules,
        events: Option<EventEngine>,
    ) -> Result<Game, String> {
        let sav = tot_kit::parse_sav(save).map_err(|e| e.to_string())?;
        let mut world = crate::boot(&sav, seed);
        world.start_turn(&rules);
        for c in 1..8 {
            world.reveal(c);
        }
        let log = format!(
            "totengine-orderlog 1\nseed {seed}\nsave {} {} {:016x}\n",
            save_name.replace(' ', "_"),
            save.len(),
            fnv64(save)
        );
        Ok(Game {
            world,
            rules,
            events,
            players: (0..8).map(|_| None).collect(),
            log,
        })
    }

    /// The order log so far.
    pub fn log(&self) -> &str {
        &self.log
    }

    fn submit(&mut self, civ: u8, order: Order) -> OrderResult {
        let result = self
            .world
            .apply_order(&self.rules, self.events.as_mut(), civ, order);
        self.log.push_str(&format!(
            "T {} C {civ} {order} = {result}\n",
            self.world.turn
        ));
        result
    }

    fn close_turn(&mut self) {
        self.world.run_turn(&self.rules, self.events.as_mut());
        for c in 1..8 {
            self.world.reveal(c);
        }
        self.log.push_str(&format!(
            "R {} {:016x}\n",
            self.world.turn,
            self.world.checksum()
        ));
    }

    /// One full turn: every living civ with a player acts until it ends its
    /// turn (or [`ACTION_CAP`]), in civ order; then the world turn runs.
    pub fn play_turn(&mut self) {
        for civ in self.world.living_civs() {
            let Some(mut player) = self.players.get_mut(civ as usize).and_then(Option::take) else {
                continue;
            };
            let mut last = None;
            for _ in 0..ACTION_CAP {
                let obs = self.world.observe(&self.rules, civ, last.take());
                let order = player.act(&obs);
                let result = self.submit(civ, order);
                if order == Order::EndTurn {
                    break;
                }
                last = Some(result);
            }
            self.players[civ as usize] = Some(player);
        }
        self.close_turn();
    }

    /// Replay a log against a fresh boot of the same save and seed: every
    /// order must give its logged result, and every turn its logged
    /// checksum. The error names the first divergent line.
    pub fn replay(
        save_name: &str,
        save: &[u8],
        rules: Rules,
        events: Option<EventEngine>,
        log: &str,
    ) -> Result<Game, String> {
        let mut lines = log.lines().enumerate();
        let hdr: Vec<&str> = (&mut lines).take(3).map(|(_, l)| l).collect();
        if hdr.first() != Some(&"totengine-orderlog 1") {
            return Err("line 1: not an order log".into());
        }
        let seed: u64 = hdr
            .get(1)
            .and_then(|l| l.strip_prefix("seed "))
            .and_then(|v| v.parse().ok())
            .ok_or("line 2: bad seed")?;
        let mut g = Game::new(save_name, save, seed, rules, events)?;
        if g.log.lines().nth(2) != hdr.get(2).copied() {
            return Err("line 3: the save differs from the logged one".into());
        }
        for (n, line) in lines {
            let at = n + 1;
            if let Some(rest) = line.strip_prefix("R ") {
                g.close_turn();
                let want = rest.split_whitespace().nth(1).unwrap_or("");
                let got = format!("{:016x}", g.world.checksum());
                if want != got {
                    return Err(format!("line {at}: checksum {got}, log says {want}"));
                }
                continue;
            }
            let (head, want) = line
                .split_once(" = ")
                .ok_or_else(|| format!("line {at}: malformed"))?;
            let f: Vec<&str> = head.splitn(5, ' ').collect();
            let (civ, order) = match f.as_slice() {
                ["T", _, "C", c, rest] => (
                    c.parse::<u8>().map_err(|_| format!("line {at}: bad civ"))?,
                    Order::parse(rest).ok_or_else(|| format!("line {at}: bad order"))?,
                ),
                _ => return Err(format!("line {at}: malformed")),
            };
            let got = g.submit(civ, order).to_string();
            if got != want {
                return Err(format!("line {at}: result {got}, log says {want}"));
            }
        }
        Ok(g)
    }
}

/// Test player: random orders, legal and not, to exercise validation,
/// logging and replay (decision 0003). Its randomness is its own: seeded
/// from the game seed and the civ, never the world's generators.
pub struct RandomPlayer {
    rng: Lcg64,
    actions: u32,
    turn: i16,
}

impl RandomPlayer {
    pub fn new(seed: u64, civ: u8) -> Self {
        RandomPlayer {
            rng: Lcg64::new((seed as u32) ^ (0x9e37_79b9u32.wrapping_mul(civ as u32 + 1))),
            actions: 0,
            turn: i16::MIN,
        }
    }

    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.rng.next_u32() as usize) % n
        }
    }
}

impl Player for RandomPlayer {
    fn act(&mut self, obs: &Observation) -> Order {
        if obs.turn != self.turn {
            self.turn = obs.turn;
            self.actions = 0;
        }
        self.actions += 1;
        if self.actions > 12 {
            return Order::EndTurn;
        }
        let own_units: Vec<&UnitView> = obs.units.iter().filter(|u| u.owner == obs.civ).collect();
        let own_cities: Vec<&CityView> = obs.cities.iter().filter(|c| c.owner == obs.civ).collect();
        match self.below(10) {
            0 if !own_cities.is_empty() => {
                let c = own_cities[self.below(own_cities.len())].index;
                let item = if self.below(2) == 0 {
                    self.below(10) as i8
                } else {
                    -(1 + self.below(20) as i8)
                };
                Order::SetProduction { city: c, item }
            }
            _ if !own_units.is_empty() => {
                let u = own_units[self.below(own_units.len())];
                let (dx, dy) = NEIGHBOURS[self.below(8)];
                let x = (u.x as i32 + dx).rem_euclid(obs.w2.max(1) as i32) as u16;
                let y = (u.y as i32 + dy).max(0) as u16;
                Order::Move {
                    unit: u.index,
                    to: (x, y),
                }
            }
            _ => Order::EndTurn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_support::synthetic_save_bytes;

    const RULES: &str = "@COSMIC\n3 ; road\n2 ; trireme\n2 ; food\n10 ; food rows\n\
10 ; shield rows\n@UNITS\nSettlers, nil, 0, 1., 0, 0a, 1d, 1h, 1f, 3, 0, 5, nil, 000000000000000\n\
Warriors, nil, 0, 1., 0, 1a, 1d, 1h, 1f, 1, 0, 1, nil, 000000000000000\n\
Knights, nil, 0, 2., 0, 4a, 2d, 1h, 1f, 4, 0, 1, Chv, 000000000000000\n\
@IMPROVE\nNothing, 0, 0, nil\nPalace, 10, 0, nil\nBarracks, 4, 1, nil\n\
@TERRAIN\nDesert, 1, 2, 0, 1, 0\nPlains, 1, 2, 1, 1, 0\nGrassland, 1, 2, 2, 0, 0\n";

    fn game(seed: u64) -> Game {
        let rules = tot_kit::parse_rules(RULES);
        let mut g = Game::new("synthetic.sav", &synthetic_save_bytes(), seed, rules, None).unwrap();
        for civ in 1..8 {
            g.players[civ] = Some(Box::new(RandomPlayer::new(seed, civ as u8)));
        }
        g
    }

    /// RG-8 acceptance (CI half): random play logs orders and results,
    /// and the log replays bit-exact on a fresh boot.
    #[test]
    fn random_play_replays_bit_exact() {
        let mut g = game(11);
        for _ in 0..20 {
            g.play_turn();
        }
        let log = g.log().to_string();
        assert!(log.lines().filter(|l| l.starts_with("R ")).count() == 20);
        assert!(log.contains("= moved"), "some moves happen");
        assert!(log.contains("= rejected"), "some orders are rejected");
        let rules = tot_kit::parse_rules(RULES);
        let r = Game::replay("synthetic.sav", &synthetic_save_bytes(), rules, None, &log).unwrap();
        assert_eq!(r.world.checksum(), g.world.checksum());
        assert_eq!(r.log(), log);
    }

    /// A tampered result line is caught at that line; a wrong seed or save
    /// is caught at the header.
    #[test]
    fn tampering_is_detected_at_the_line() {
        let mut g = game(5);
        for _ in 0..3 {
            g.play_turn();
        }
        let log = g.log().to_string();
        let rules = || tot_kit::parse_rules(RULES);
        let (n, line) = log
            .lines()
            .enumerate()
            .find(|(_, l)| l.ends_with("= moved"))
            .expect("a move");
        let bad = log.replacen(line, &line.replace("= moved", "= ended"), 1);
        let err = Game::replay(
            "synthetic.sav",
            &synthetic_save_bytes(),
            rules(),
            None,
            &bad,
        )
        .err()
        .expect("tampering detected");
        assert!(err.starts_with(&format!("line {}:", n + 1)), "{err}");
        let mut other = synthetic_save_bytes();
        *other.last_mut().unwrap() ^= 1;
        assert!(Game::replay("synthetic.sav", &other, rules(), None, &log).is_err());
    }

    /// Orders are validated: foreign units and cities, non-neighbours and
    /// unknown prerequisites are rejected without changing the world.
    #[test]
    fn invalid_orders_change_nothing() {
        let mut g = game(1);
        let before = g.world.checksum();
        let foreign_unit = g
            .world
            .units
            .iter()
            .position(|u| u.owner != 1 && u.id != 0)
            .unwrap();
        let r = g.world.apply_order(
            &g.rules,
            None,
            1,
            Order::Move {
                unit: foreign_unit,
                to: (0, 0),
            },
        );
        assert_eq!(r, OrderResult::Rejected("not your unit"));
        let own = g.world.units.iter().position(|u| u.owner == 1).unwrap();
        let (x, y) = (g.world.units[own].x, g.world.units[own].y);
        let r = g.world.apply_order(
            &g.rules,
            None,
            1,
            Order::Move {
                unit: own,
                to: (x + 4, y),
            },
        );
        assert_eq!(r, OrderResult::Rejected("not adjacent"));
        let city = g.world.cities.iter().position(|c| c.owner == 1).unwrap();
        // Knights need Chivalry, which civ 1 lacks
        let r = g
            .world
            .apply_order(&g.rules, None, 1, Order::SetProduction { city, item: 2 });
        assert_eq!(r, OrderResult::Rejected("cannot build"));
        assert_eq!(g.world.checksum(), before);
        let r = g
            .world
            .apply_order(&g.rules, None, 1, Order::SetProduction { city, item: -2 });
        assert_eq!(r, OrderResult::ProductionSet);
        assert_eq!(g.world.cities[city].building, -2);
    }

    /// The observation hides foreign units out of sight and shows only seen
    /// terrain.
    #[test]
    fn observation_respects_fog() {
        let g = game(1);
        let obs = g.world.observe(&g.rules, 1, None);
        assert!(obs
            .units
            .iter()
            .all(|u| u.owner == 1 || g.world.sees_now(1, u.x, u.y)));
        assert!(obs
            .units
            .iter()
            .filter(|u| u.owner != 1)
            .all(|u| u.moves_left == 0));
        assert!(obs.seen_terrain.iter().any(Option::is_some));
        assert!(obs
            .cities
            .iter()
            .filter(|c| c.owner != 1)
            .all(|c| c.output.is_none()));
    }
}
