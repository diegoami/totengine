//! Bounded knowledge writes from native unit-update loops, not runtime entry.
use super::{explored_by_unit, nearby, surface, Scout};
use std::collections::BTreeMap;
use tot_kit::{Rules, SaveFile};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Sight {
    domain: u8,
    extended: bool,
    armed: bool,
    role: u8,
    submarine: bool,
    detects_submarines: bool,
}

impl Sight {
    fn reaches(self, ocean: bool) -> bool {
        self.extended && (self.domain == 1 || ocean == (self.domain == 2))
    }
}

/// Return a snapshot with only unit knowledge writes applied, leaving the
/// input unchanged. Re-derived from civ2.bak 0x41f8a7..0x4205a7.
///
/// This intentionally narrow kernel accepts one classic surface, ordinary
/// non-submarine awake units, coherent stacks/occupancy, and existing foreign
/// treaty bits 1/4/8. It rejects unsupported contexts before producing output.
/// It does not run movement, first contact, AI sighting bookkeeping, reset
/// visibility, or update city-phase memory. Clients do not call this kernel;
/// it does not activate a faithful import profile or change old checkpoints.
pub fn unit_knowledge_after(
    save: &SaveFile,
    rules: &Rules,
    scout_slot: usize,
) -> Result<SaveFile, String> {
    observe(save, rules, scout_slot, true, false, false, |_, error| {
        Err(error)
    })
    .map_err(|e| e.to_string())
}

/// A native contact-call boundary, before any conversation is executed.
/// Existing contacts can reach it too; this does not mean a newly met civ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitObservationError {
    Unsupported(String),
    ContactRequired {
        observer: u8,
        other: u8,
        x: u16,
        y: u16,
        map: u8,
    },
}

impl From<String> for UnitObservationError {
    fn from(s: String) -> Self {
        Self::Unsupported(s)
    }
}

impl From<&str> for UnitObservationError {
    fn from(s: &str) -> Self {
        Self::Unsupported(s.into())
    }
}

impl std::fmt::Display for UnitObservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(s) => f.write_str(s),
            Self::ContactRequired {
                observer,
                other,
                x,
                y,
                map,
            } => write!(
                f,
                "native contact required: civ {observer} with {other} at ({x},{y}) on map {map}"
            ),
        }
    }
}

impl std::error::Error for UnitObservationError {}

/// Normal classic single-surface unit observation, with coherent stacks,
/// ordinary owners and bounded city sizes, including native sentry/goto wakes.
/// `active_contacts` is the original updater's second argument (0/1).
/// An attempted conversation returns ContactRequired without a partial result.
/// No contact outcome, movement or AI sighting bookkeeping is implemented.
pub fn unit_observation_after(
    save: &SaveFile,
    rules: &Rules,
    scout_slot: usize,
    active_contacts: bool,
) -> Result<SaveFile, UnitObservationError> {
    observe(
        save,
        rules,
        scout_slot,
        false,
        active_contacts,
        false,
        |_, error| Err(error),
    )
}

/// Compose the native stack reset and observation kernels atomically. The
/// input must already contain coherent unit positions/occupancy; this does
/// not implement a movement operation or its other intermediate side effects.
pub fn unit_observation_after_reset(
    save: &SaveFile,
    rules: &Rules,
    scout_slot: usize,
    active_contacts: bool,
) -> Result<SaveFile, UnitObservationError> {
    let reset = clear_unit_stack_visibility(save, scout_slot)?;
    unit_observation_after(&reset, rules, scout_slot, active_contacts)
}

pub(crate) fn unit_observation_with_remote_barbarians(
    save: &SaveFile,
    rules: &Rules,
    scout_slot: usize,
) -> Result<SaveFile, UnitObservationError> {
    observe(save, rules, scout_slot, false, true, true, |_, error| {
        Err(error)
    })
}

pub(crate) fn unit_observation_with_contacts(
    save: &SaveFile,
    rules: &Rules,
    scout_slot: usize,
    allow_remote_barbarians: bool,
    contact: impl FnMut(&mut SaveFile, UnitObservationError) -> Result<(), UnitObservationError>,
) -> Result<SaveFile, UnitObservationError> {
    observe(
        save,
        rules,
        scout_slot,
        false,
        true,
        allow_remote_barbarians,
        contact,
    )
}

fn observe(
    save: &SaveFile,
    rules: &Rules,
    scout_slot: usize,
    strict: bool,
    active_contacts: bool,
    allow_remote_barbarians: bool,
    mut encounter: impl FnMut(&mut SaveFile, UnitObservationError) -> Result<(), UnitObservationError>,
) -> Result<SaveFile, UnitObservationError> {
    let map = surface(save)?;
    if save.multiplayer != 0 {
        return Err("unit knowledge update requires single player".into());
    }
    let n = save.map_header.surface();
    let index = |x: u16, y: u16| {
        save.map_header
            .stored_index(usize::from(x), usize::from(y))
            .filter(|_| (u32::from(x) + u32::from(y)) % 2 == 0)
            .ok_or_else(|| "unit knowledge record has invalid coordinates".to_string())
    };
    let scout = save
        .units
        .get(scout_slot)
        .filter(|u| u.id() != 0 && (1..8).contains(&u.owner()))
        .ok_or("unit knowledge scout requires a live player unit")?;
    let neighborhood = nearby(save, scout.x(), scout.y())?;
    let mut stacks: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut sights = vec![None; save.units.len()];
    for (slot, u) in save.units.iter().enumerate().filter(|(_, u)| u.id() != 0) {
        let tile = index(u.x(), u.y())?;
        let remote_barbarian = allow_remote_barbarians
            && u.owner() == 0
            && map.terrain(tile) == 10
            && neighborhood.iter().all(|&(_, near)| near != tile);
        if u.raw[4..6] != [0, 0] || (!(1..8).contains(&u.owner()) && !remote_barbarian) {
            return Err("unit knowledge requires unflagged surface units from civs 1..7".into());
        }
        if strict && u.orders() != 255 {
            return Err("knowledge-only update requires awake units".into());
        }
        let id = usize::from(u.type_id());
        let domain = rules.unit_domain(id).filter(|&d| d <= 3);
        let flags = rules.unit_flags(id).filter(|&f| !strict || f & 8 == 0);
        let attack = rules.unit_attack(id).filter(|&a| a <= 127);
        // The earlier knowledge-only API never needed an order-wake role.
        let role = if strict {
            Some(0)
        } else {
            rules.unit_role(id).filter(|&r| r <= 127)
        };
        let (Some(domain), Some(flags), Some(attack), Some(role)) = (domain, flags, attack, role)
        else {
            return Err("unit knowledge requires supported decoded unit definitions".into());
        };
        sights[slot] = Some(Sight {
            domain,
            extended: flags & 1 != 0,
            armed: attack != 0,
            role,
            submarine: flags & 8 != 0,
            detects_submarines: !strict && flags & 0x4000 != 0,
        });
        stacks.entry(tile).or_default().push(slot);
    }
    let Some(sight) = sights.get(scout_slot).copied().flatten() else {
        return Err("unit knowledge scout slot is not a live unit".into());
    };
    let owner = scout.owner();
    let origin = index(scout.x(), scout.y())?;
    let mut cities = BTreeMap::new();
    for (slot, c) in save
        .cities
        .iter()
        .enumerate()
        .filter(|(_, c)| c.city_id() != 0)
    {
        if c.raw[4..6] != [0, 0] || !(1..8).contains(&c.owner()) || !(1..128).contains(&c.size()) {
            return Err("unit knowledge requires ordinary surface city owners and sizes".into());
        }
        if cities.insert(index(c.x(), c.y())?, slot).is_some() {
            return Err("unit knowledge has overlapping cities".into());
        }
    }
    // Require a complete native stack chain, not just matching coordinates.
    // Otherwise native traversal might reveal fewer units than our grouping.
    for (&i, stack) in &mut stacks {
        let first = &save.units[stack[0]];
        if stack.iter().any(|&s| {
            save.units[s].owner() != first.owner() || (strict && sights[s] != sights[stack[0]])
        }) {
            return Err("unit knowledge requires coherent same-owner stacks".into());
        }
        *stack = ordered_stack(save, stack)?;
        if let Some(&c) = cities.get(&i) {
            if save.cities[c].owner() as u8 != first.owner() {
                return Err("unit knowledge has mixed city/unit owners".into());
            }
        }
    }
    for i in 0..n {
        let units = stacks.get(&i);
        let city = cities.get(&i).map(|&c| &save.cities[c]);
        if (map.improvements(i) & 1 != 0) != units.is_some()
            || (map.improvements(i) & 0xc2 == 2) != city.is_some()
            || (map.improvements(i) & 2 != 0 && city.is_none())
        {
            return Err("unit knowledge occupancy and live records disagree".into());
        }
        if let Some(civ) = city
            .map(|c| c.owner() as u8)
            .or_else(|| units.map(|s| save.units[s[0]].owner()))
        {
            if map.tile_fields[6 * i + 5] >> 4 != civ {
                return Err("unit knowledge tile owner and live records disagree".into());
            }
            if strict
                && civ != owner
                && (save.diplomacy[civ as usize][owner as usize] & 13 != 13
                    || save.diplomacy[owner as usize][civ as usize] & 13 != 13)
            {
                return Err("unit knowledge requires established treaty bits 1/4/8".into());
            }
        }
    }
    let mut out = save.clone();
    for i in explored_by_unit(
        save,
        Scout {
            x: scout.x(),
            y: scout.y(),
            domain: sight.domain,
            extended: sight.extended,
        },
    )? {
        out.maps[0].tile_fields[6 * i + 4] |= 1 << owner;
        out.maps[0].visibility[(owner as usize - 1) * n + i] = map.improvements(i);
    }
    let origin_city = cities.contains_key(&origin);
    let origin_ocean = map.terrain(origin) == 10;
    for (k, i) in neighborhood {
        if k == 20 {
            continue;
        }
        let city = cities.get(&i).copied();
        let stack = stacks.get(&i);
        let other = city
            .map(|c| save.cities[c].owner() as u8)
            .or_else(|| stack.map(|s| save.units[s[0]].owner()));
        let Some(other) = other.filter(|&civ| civ != owner) else {
            continue;
        };
        let ocean = map.terrain(i) == 10;
        let observing = k < 8 || sight.reaches(ocean);
        if observing {
            if let Some(c) = city {
                out.cities[c].raw[14] |= 1 << owner;
                out.cities[c].raw[15 + owner as usize] = save.cities[c].size() as u8;
            }
        }
        if k < 8 {
            // Native city observation precedes the submarine land-neighbor
            // early exit. A submarine does not reveal itself to a city here.
            if city.is_some() && !sight.submarine {
                reveal_stack(&mut out, &stacks[&origin], other);
            }
            if sight.submarine && !ocean {
                continue;
            }
            if let Some(stack) = stack {
                let head = stack[0];
                let neighbor = sights[head].unwrap();
                if !sight.submarine || neighbor.detects_submarines {
                    if !origin_city {
                        reveal_stack(&mut out, &stacks[&origin], other);
                    }
                    if !strict {
                        if out.diplomacy[other as usize][owner as usize] & 8 == 0 {
                            wake_sentries(&mut out, stack, &sights, ocean);
                        }
                        let other_human = save.human_mask & (1 << other) != 0;
                        let wake_other = if other_human {
                            ocean == origin_ocean
                        } else {
                            ocean || !origin_ocean
                        };
                        if wake_other {
                            wake_goto(&mut out, head, neighbor);
                        }
                    }
                }
                // Native 0x56b64b(head, 2) counts the entire stack. Its >1
                // exception is distinct from the detector flag check.
                if !neighbor.submarine || stack.len() > 1 || sight.detects_submarines {
                    if city.is_none() {
                        reveal_stack(&mut out, stack, owner);
                    }
                    if !strict {
                        let own_human = save.human_mask & (1 << owner) != 0;
                        let wake_scout = if own_human {
                            ocean == origin_ocean || sight.domain == 1
                        } else {
                            origin_ocean || !ocean || sight.domain == 1
                        };
                        if wake_scout {
                            wake_goto(&mut out, scout_slot, sight);
                        }
                    }
                }
            }
        } else {
            if let Some(stack) = stack {
                let neighbor = sights[stack[0]].unwrap();
                if observing && !neighbor.submarine && city.is_none() {
                    reveal_stack(&mut out, stack, owner);
                }
                if !sight.submarine && neighbor.reaches(origin_ocean) {
                    if !strict {
                        wake_goto(&mut out, stack[0], neighbor);
                        wake_sentries(&mut out, stack, &sights, ocean);
                    }
                    if !origin_city {
                        reveal_stack(&mut out, &stacks[&origin], other);
                    }
                }
            }
            if !sight.submarine
                && city.is_some()
                && map.visible_to(origin) & (1 << other) != 0
                && sight.armed
            {
                reveal_stack(&mut out, &stacks[&origin], other);
            }
        }
        if !strict && active_contacts && k < 8 && map.terrain(origin) != 10 {
            let contact = if let Some(stack) = stack {
                sight.domain == 0
                    && map.terrain(i) != 10
                    && (sights[stack[0]].unwrap().domain != 1 || city.is_some())
                    && scout.type_id() != 9
            } else {
                city.is_some() && out.diplomacy[owner as usize][other as usize] & 1 == 0
            };
            if contact {
                encounter(
                    &mut out,
                    UnitObservationError::ContactRequired {
                        observer: owner,
                        other,
                        x: (i % save.map_header.stored_w() * 2
                            + (i / save.map_header.stored_w()) % 2)
                            as u16,
                        y: (i / save.map_header.stored_w()) as u16,
                        map: 0,
                    },
                )?;
            }
        }
    }
    Ok(out)
}

fn reveal_stack(save: &mut SaveFile, stack: &[usize], civ: u8) {
    for &slot in stack {
        save.units[slot].raw[11] |= 1 << civ;
    }
}

fn wake_goto(save: &mut SaveFile, slot: usize, sight: Sight) {
    if save.units[slot].orders() & 15 == 12 && sight.role != 7 {
        save.units[slot].raw[17] = 255;
    }
}

fn wake_sentries(save: &mut SaveFile, stack: &[usize], sights: &[Option<Sight>], ocean: bool) {
    for &slot in stack {
        if save.units[slot].orders() == 3 && (sights[slot].unwrap().domain != 0 || !ocean) {
            save.units[slot].raw[17] = 255;
        }
    }
}

fn ordered_stack(save: &SaveFile, members: &[usize]) -> Result<Vec<usize>, String> {
    let heads: Vec<_> = members
        .iter()
        .copied()
        .filter(|&s| save.units[s].stack_prev() == -1)
        .collect();
    if heads.len() != 1 || heads[0] > i16::MAX as usize {
        return Err("unit knowledge requires one stack head".into());
    }
    let mut visited = Vec::new();
    let mut next = heads[0] as i16;
    let mut prev = -1;
    while next != -1 {
        let slot = next as usize;
        if !members.contains(&slot)
            || visited.contains(&slot)
            || save.units[slot].stack_prev() != prev
        {
            return Err("unit knowledge has a broken stack chain".into());
        }
        visited.push(slot);
        prev = next;
        next = save.units[slot].stack_next();
    }
    if visited.len() != members.len() {
        return Err("unit knowledge has a disconnected stack".into());
    }
    Ok(visited)
}

/// Native 0x56adfe: clear +11 for the complete selected stack, from any member.
/// Only this reset is performed; movement and subsequent observation are separate.
pub fn clear_unit_stack_visibility(save: &SaveFile, slot: usize) -> Result<SaveFile, String> {
    surface(save)?;
    let u = save
        .units
        .get(slot)
        .filter(|u| u.id() != 0)
        .ok_or("visibility reset needs a live unit")?;
    if save.multiplayer != 0 || u.raw[4..6] != [0, 0] || !(1..8).contains(&u.owner()) {
        return Err("visibility reset requires ordinary single-player surface units".into());
    }
    let members: Vec<_> = save
        .units
        .iter()
        .enumerate()
        .filter(|(_, v)| v.id() != 0 && v.raw[4..6] == [0, 0] && v.x() == u.x() && v.y() == u.y())
        .map(|(s, _)| s)
        .collect();
    if members.iter().any(|&s| save.units[s].owner() != u.owner()) {
        return Err("visibility reset has mixed stack owners".into());
    }
    let stack = ordered_stack(save, &members)?;
    let mut out = save.clone();
    for s in stack {
        out.units[s].raw[11] = 0;
    }
    Ok(out)
}
