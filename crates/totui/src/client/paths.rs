//! Path preview (#284 item 3, opt-in under Settings): while a unit is selected
//! for moving, a resting pointer on a known tile shows the cheapest route
//! there and the turns it takes. The search uses only the civilization's
//! filtered view: tiles never seen are impassable, and only foreign units it
//! can see block the way or exert zone of control. It mirrors the engine's
//! step rules (`engine::movement::move_unit`): terrain, road and railroad cost
//! of the tile entered, land/sea domains, no east-west wrap, "any movement left
//! enters one tile", and zone of control ending the turn. It is an estimate:
//! damage, expired wonders and anything unseen can change the real moves.
//! Nothing here reaches the engine; there is no goto order yet.
use super::*;
use engine::movement::{movement_allowance, MovementInput, OCEAN_TERRAIN};
use engine::player::{Observation, UnitView};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Routes longer than this many turn ends are not offered.
const MAX_TURNS: u16 = 99;

/// The best arrival at one stored tile: turn ends before arriving, moves left
/// (thirds) after entering, and the stored tile it was entered from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Reach {
    pub turns: u16,
    pub left: u32,
    pub parent: Option<usize>,
}

/// Every known tile the unit can reach, by stored index.
pub(super) struct Plan {
    pub origin: usize,
    pub reach: Vec<Option<Reach>>,
}

/// Stored index of logical `(x, y)`, and back.
pub(super) fn stored(o: &Observation, x: u16, y: u16) -> usize {
    usize::from(y) * (usize::from(o.w2) / 2) + usize::from(x) / 2
}
pub(super) fn logical(o: &Observation, i: usize) -> (u16, u16) {
    let y = (i / (usize::from(o.w2) / 2)) as u16;
    (((i % (usize::from(o.w2) / 2)) * 2) as u16 + (y & 1), y)
}
fn neighbours(o: &Observation, x: u16, y: u16) -> impl Iterator<Item = (u16, u16)> + '_ {
    [
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
        (2, 0),
        (-2, 0),
        (0, 2),
        (0, -2),
    ]
    .into_iter()
    .filter_map(move |(dx, dy): (i32, i32)| {
        let (nx, ny) = (i32::from(x) + dx, i32::from(y) + dy);
        (nx >= 0 && ny >= 0 && nx < i32::from(o.w2) && ny < i32::from(o.h))
            .then_some((nx as u16, ny as u16))
    })
}

/// The unit's full allowance at the start of a later turn, in thirds.
fn full_moves(rules: &tot_kit::Rules, o: &Observation, u: &UnitView) -> u32 {
    let t = usize::from(u.type_id);
    let domain = rules.unit_domain(t).unwrap_or(0);
    let tech = |n: usize| o.techs.get(n / 8).is_some_and(|b| b & (1 << (n % 8)) != 0);
    movement_allowance(&MovementInput {
        moves: rules.unit_move(t).unwrap_or(0),
        domain,
        flags: rules.unit_flags(t).unwrap_or(0),
        max_hp: rules.unit_hits(t).unwrap_or(1) as i32 * 10,
        damage: 0,
        persistent_damage: false,
        nuclear_power: domain == 2 && tech(59),
        lighthouse: domain == 2 && o.own_wonders[3].is_some(),
        magellan: domain == 2 && o.own_wonders[12].is_some(),
    })
}

/// Cheapest known routes from `u`, ranked by fewest turn ends, then most
/// moves left on arrival.
pub(super) fn plan(rules: &tot_kit::Rules, o: &Observation, u: &UnitView) -> Plan {
    let tiles = o.seen.len();
    let origin = stored(o, u.x, u.y);
    let mut reach = vec![None; tiles];
    let domain = rules.unit_domain(usize::from(u.type_id)).unwrap_or(0);
    let full = full_moves(rules, o, u);
    let road = 3 / rules.road_multiplier().unwrap_or(3).max(1);
    let mut blocked = vec![false; tiles];
    let mut zoc = vec![false; tiles];
    for f in o.units.iter().filter(|f| f.owner != o.civ) {
        if let Some(b) = blocked.get_mut(stored(o, f.x, f.y)) {
            *b = true;
        }
        for (nx, ny) in neighbours(o, f.x, f.y) {
            zoc[stored(o, nx, ny)] = true;
        }
    }
    for c in o.cities.iter().filter(|c| c.owner != o.civ) {
        if let Some(b) = blocked.get_mut(stored(o, c.x, c.y)) {
            *b = true;
        }
    }
    let cost = |i: usize| -> Option<u32> {
        let tile = o.seen.get(i).copied().flatten()?;
        let ocean = tile.terrain == OCEAN_TERRAIN;
        if blocked[i] || (domain == 2 && !ocean) || (domain == 0 && ocean) {
            return None;
        }
        Some(if tile.rail {
            0
        } else if tile.road {
            road.max(1)
        } else {
            rules
                .terrain_move_cost(usize::from(tile.terrain))
                .unwrap_or(1)
                .max(1)
                * 3
        })
    };
    if origin >= tiles {
        return Plan { origin, reach };
    }
    reach[origin] = Some(Reach {
        turns: 0,
        left: u.moves_left,
        parent: None,
    });
    let mut queue = BinaryHeap::new();
    queue.push(Reverse((0u16, Reverse(u.moves_left), origin)));
    while let Some(Reverse((turns, Reverse(left), at))) = queue.pop() {
        if reach[at].is_some_and(|r| (r.turns, r.left) != (turns, left)) {
            continue;
        }
        let (mut t, mut l) = (turns, left);
        if l == 0 {
            if full == 0 || t >= MAX_TURNS {
                continue;
            }
            t += 1;
            l = full;
        }
        let (x, y) = logical(o, at);
        for (nx, ny) in neighbours(o, x, y) {
            let next = stored(o, nx, ny);
            let Some(c) = cost(next) else { continue };
            let after = if zoc[at] && zoc[next] {
                0
            } else {
                l - c.min(l)
            };
            let better =
                reach[next].is_none_or(|r: Reach| (t, Reverse(after)) < (r.turns, Reverse(r.left)));
            if better {
                reach[next] = Some(Reach {
                    turns: t,
                    left: after,
                    parent: Some(at),
                });
                queue.push(Reverse((t, Reverse(after), next)));
            }
        }
    }
    Plan { origin, reach }
}

/// The tip line for a route to `(x, y)`; `None` on the unit's own tile.
pub(super) fn path_line(p: &Plan, o: &Observation, x: u16, y: u16) -> Option<String> {
    let at = stored(o, x, y);
    if at == p.origin {
        return None;
    }
    Some(match p.reach.get(at).copied().flatten() {
        None => "Path: no known route".into(),
        Some(Reach { turns: 0, .. }) => "Path: arrives this turn".into(),
        Some(Reach { turns: 1, .. }) => "Path: arrives in 1 turn".into(),
        Some(r) => format!("Path: arrives in {} turns", r.turns),
    })
}

/// Marker for one step of a route at screen `(px, py)`: a dot, and where the
/// unit's moves run out, the count of turns ended there.
fn step_draws(px: f64, py: f64, stop: Option<u16>) -> Vec<Draw> {
    let mut d = vec![
        rect(px - 4., py - 4., 8., 8., style::INSET),
        rect(px - 3., py - 3., 6., 6., style::GOLD),
    ];
    if let Some(n) = stop {
        let label = n.to_string();
        let w = (label.len() as f64 * f64::from(style::TYPE_SMALL) * 0.58 + 6.).ceil();
        d.push(rect(px + 5., py - 17., w, 17., style::INSET));
        d.push(text(
            px + 8.,
            py - 16.,
            &label,
            style::TYPE_SMALL,
            style::TEXT_BRIGHT,
        ));
    }
    d
}

/// The destination marker drawn with a tip.
pub(super) fn target_draws(px: f64, py: f64) -> Vec<Draw> {
    vec![
        rect(px - 7., py - 7., 14., 14., style::INSET),
        rect(px - 6., py - 6., 12., 12., style::TEXT_BRIGHT),
        rect(px - 3., py - 3., 6., 6., style::INSET),
    ]
}

/// Builds the shared route tree lazily: only tiles on the way to a tip's
/// target are added, each once.
pub(super) struct RouteTree<'a> {
    pub plan: &'a Plan,
    pub steps: Vec<RouteStep>,
    index: BTreeMap<usize, usize>,
}
impl<'a> RouteTree<'a> {
    pub fn new(plan: &'a Plan) -> Self {
        Self {
            plan,
            steps: Vec::new(),
            index: BTreeMap::new(),
        }
    }
    /// Route index of the step before stored tile `target`, adding its
    /// chain back to the origin. `screen` maps a stored tile to its on-map
    /// centre, `None` when off the map.
    pub fn before(
        &mut self,
        target: usize,
        screen: impl Fn(usize) -> Option<(f64, f64)>,
    ) -> Option<usize> {
        let first = self.plan.reach.get(target).copied().flatten()?.parent?;
        let mut chain = Vec::new();
        let mut at = Some(first);
        while let Some(i) = at {
            if self.index.contains_key(&i) || chain.len() > self.plan.reach.len() {
                break;
            }
            chain.push(i);
            at = self.plan.reach[i].and_then(|r| r.parent);
        }
        for &i in chain.iter().rev() {
            let r = self.plan.reach[i]?;
            let parent = r.parent.and_then(|p| self.index.get(&p).copied());
            let draws = match screen(i) {
                Some((px, py)) if i != self.plan.origin => {
                    step_draws(px, py, (r.left == 0).then_some(r.turns + 1))
                }
                _ => Vec::new(),
            };
            self.index.insert(i, self.steps.len());
            self.steps.push(RouteStep { parent, draws });
        }
        self.index.get(&first).copied()
    }
}
