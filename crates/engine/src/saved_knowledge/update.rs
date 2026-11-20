//! Bounded knowledge writes from native unit-update loops, not runtime entry.
use super::{explored_by_unit, nearby, surface, Scout};
use std::collections::BTreeMap;
use tot_kit::{Rules, SaveFile};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Sight {
    domain: u8,
    extended: bool,
    armed: bool,
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
/// non-invisible awake units, coherent stacks/occupancy, and existing foreign
/// treaty bits 1/4/8. It rejects unsupported contexts before producing output.
/// It does not run movement, first contact, AI sighting bookkeeping, reset
/// visibility, or update city-phase memory. Clients do not call this kernel;
/// it does not activate a faithful import profile or change old checkpoints.
pub fn unit_knowledge_after(
    save: &SaveFile,
    rules: &Rules,
    scout_slot: usize,
) -> Result<SaveFile, String> {
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
    let mut stacks: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut sights = vec![None; save.units.len()];
    for (slot, u) in save.units.iter().enumerate().filter(|(_, u)| u.id() != 0) {
        if u.raw[4..6] != [0, 0] || !(1..8).contains(&u.owner()) || u.orders() != 255 {
            return Err(
                "unit knowledge requires unflagged awake surface units from civs 1..7".into(),
            );
        }
        let id = usize::from(u.type_id());
        let domain = rules.unit_domain(id).filter(|&d| d <= 3);
        let flags = rules.unit_flags(id).filter(|&f| f & 8 == 0);
        let attack = rules.unit_attack(id).filter(|&a| a <= 127);
        let (Some(domain), Some(flags), Some(attack)) = (domain, flags, attack) else {
            return Err("unit knowledge requires decoded non-invisible unit definitions".into());
        };
        sights[slot] = Some(Sight {
            domain,
            extended: flags & 1 != 0,
            armed: attack != 0,
        });
        stacks.entry(index(u.x(), u.y())?).or_default().push(slot);
    }
    let Some(sight) = sights.get(scout_slot).copied().flatten() else {
        return Err("unit knowledge scout slot is not a live unit".into());
    };
    let scout = &save.units[scout_slot];
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
    for (&i, stack) in &stacks {
        let first = &save.units[stack[0]];
        let heads: Vec<_> = stack
            .iter()
            .copied()
            .filter(|&s| save.units[s].stack_prev() == -1)
            .collect();
        if heads.len() != 1
            || stack
                .iter()
                .any(|&s| save.units[s].owner() != first.owner() || sights[s] != sights[stack[0]])
        {
            return Err("unit knowledge requires coherent same-owner stacks".into());
        }
        let mut visited = Vec::new();
        let mut next = heads[0] as i16;
        let mut prev = -1;
        while next != -1 {
            let slot = next as usize;
            if !stack.contains(&slot)
                || visited.contains(&slot)
                || save.units[slot].stack_prev() != prev
            {
                return Err("unit knowledge has a broken stack chain".into());
            }
            visited.push(slot);
            prev = next;
            next = save.units[slot].stack_next();
        }
        if visited.len() != stack.len() {
            return Err("unit knowledge has a disconnected stack".into());
        }
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
            if civ != owner
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
    for (k, i) in nearby(save, scout.x(), scout.y())? {
        if k == 20 {
            continue;
        }
        let city = cities.get(&i).copied();
        let stack = stacks.get(&i);
        let other = city
            .map(|c| save.cities[c].owner() as u8)
            .or_else(|| stack.map(|s| save.units[s[0]].owner()));
        if other == Some(owner) {
            continue;
        }
        let observing = k < 8 || sight.reaches(map.terrain(i) == 10);
        if observing {
            if let Some(c) = city {
                out.cities[c].raw[14] |= 1 << owner;
                out.cities[c].raw[15 + owner as usize] = save.cities[c].size() as u8;
            } else if let Some(stack) = stack {
                for &s in stack {
                    out.units[s].raw[11] |= 1 << owner;
                }
            }
        }
        let Some(other) = other else {
            continue;
        };
        let reciprocal_unit =
            stack.is_some_and(|s| sights[s[0]].unwrap().reaches(map.terrain(origin) == 10));
        let reciprocal_city =
            city.is_some() && map.visible_to(origin) & (1 << other) != 0 && sight.armed;
        if (k < 8 && (city.is_some() || !origin_city))
            || (k >= 8 && ((!origin_city && reciprocal_unit) || reciprocal_city))
        {
            for &s in &stacks[&origin] {
                out.units[s].raw[11] |= 1 << other;
            }
        }
    }
    Ok(out)
}
