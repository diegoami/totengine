//! Original research selection loop (`civ2.bak` VA 0x49f98b).
//!
//! This is a decision kernel, not yet the campaign research policy. Callers
//! supply eligibility and context for the value routine at 0x49c95d.
//! Parsed Rules/world mechanics adapters are available; AI v1 and v0.9
//! checkpoints keep their policy until the observation adapter and policy
//! identity are implemented.
//! Evidence: `docs/evidence/ai-research-choice.json` and
//! `docs/evidence/ai-research-value.json`, plus the saved-input adapters in
//! `docs/evidence/ai-research-adapter.json`, with their native probes.

use crate::combat::CrtRand;

mod adapter;

/// Signed byte fields used by native advance valuation.
#[derive(Debug, Clone, Copy)]
pub struct AdvanceValue {
    pub base: i8,
    pub modifier: i8,
    /// -1 = nil, -2 = no; nonnegative values are advance IDs.
    pub prerequisites: [i8; 2],
}

pub struct ValueRules {
    pub advances: [AdvanceValue; 100],
    pub unit_prerequisites: [i8; 80],
    pub improvement_prerequisites: [i8; 68],
    pub wonder_expiry: [i8; 28],
    pub growth_limits: [u8; 2],
}

#[derive(Debug, Clone, Copy)]
pub struct ValueCity {
    pub owner: u8,
    pub size: i8,
    pub live: bool,
}

/// Inputs read by the native advance value routine. This is a mechanics
/// boundary, not a player observation: foreign knowledge/contact information
/// must not be exposed to a player implicitly by constructing this context.
pub struct ValueContext {
    pub civ: u8,
    pub human_mask: u8,
    /// Native word 0x607640 (save @660), exposed by ResearchSettings::flags;
    /// distinct from WorldState::game_flags (save @652).
    pub game_flags: u16,
    pub scenario_flags: u16,
    pub leader_id: i16,
    pub leader_modifier: i8,
    pub acquired: [u8; 8],
    pub reputations: [u8; 8],
    /// This civilization's treaty row.
    pub treaties: [u32; 8],
    pub city_landmasses: [[u8; 256]; 8],
    /// Refreshed cache, consistent with per-civ packed knowledge. Stale cache
    /// differences are outside this context's contract.
    pub tech_owner_masks: [u8; 100],
    /// Native signed count at civ +114 (statistics writer 0x5072f0).
    pub sea_units: i16,
    pub cities: Vec<ValueCity>,
    /// City index; negative means no host.
    pub wonder_cities: [i16; 28],
}

impl ValueRules {
    /// The original's recursive dependency test, with a visited set so a
    /// malformed cyclic scenario cannot exhaust the engine stack.
    fn requires(&self, root: i8, target: usize) -> bool {
        let mut pending = vec![root];
        let mut visited = [false; 100];
        while let Some(id) = pending.pop() {
            let Ok(id) = usize::try_from(id) else {
                continue;
            };
            if id >= 100 || visited[id] {
                continue;
            }
            if id == target {
                return true;
            }
            visited[id] = true;
            pending.extend(self.advances[id].prerequisites);
        }
        false
    }
}

impl ValueContext {
    fn knows(&self, id: i8, civ: usize) -> bool {
        if id < 0 {
            return id != -2;
        }
        if id == 89 || civ == 0 {
            return false;
        }
        usize::try_from(id)
            .ok()
            .and_then(|id| self.tech_owner_masks.get(id))
            .is_some_and(|mask| mask & (1 << civ) != 0)
    }

    fn wonder_owner(&self, rules: &ValueRules, wonder: usize) -> Option<u8> {
        let expiry = rules.wonder_expiry[wonder];
        if expiry >= 0 && (1..8).any(|c| self.knows(expiry, c)) {
            return None;
        }
        usize::try_from(self.wonder_cities[wonder])
            .ok()
            .and_then(|i| self.cities.get(i))
            .map(|c| c.owner)
    }
}

/// Complete original advance valuation (0x49c95d), from supplied parsed Rules
/// and native-equivalent caches. Invalid civ/advance indices return `None`.
pub fn value(rules: &ValueRules, state: &ValueContext, advance: usize) -> Option<i32> {
    let civ = usize::from(state.civ);
    let tech = rules.advances.get(advance)?;
    if !(1..8).contains(&civ) {
        return None;
    }
    let human = state.human_mask & (1 << civ) != 0;
    let mut modifier = i32::from(state.leader_modifier);
    if !human && modifier >= 0 {
        for other in 1..8 {
            if state.human_mask & (1 << other) == 0
                || state.treaties[other] & 8192 == 0
                || state.acquired[other] <= state.acquired[civ]
            {
                continue;
            }
            for land in 1..255 {
                if state.city_landmasses[civ][land] != 0 && state.city_landmasses[other][land] != 0
                {
                    if modifier >= 0 {
                        modifier -= 1;
                    }
                    if state.reputations[other] > 6 {
                        modifier = -1;
                    }
                }
            }
        }
    }
    let mut result = i32::from(tech.base) + i32::from(tech.modifier) * modifier;
    let requires_unit = |id| rules.requires(rules.unit_prerequisites[id], advance);
    let mut military = if requires_unit(32) {
        3
    } else if requires_unit(33) || requires_unit(35) {
        2
    } else if requires_unit(43) {
        1
    } else {
        0
    };
    if state.game_flags & 4 != 0 {
        military *= 2;
    }
    // Native 0x49cc67 reads the OWN landmass cache again inside the foreign
    // loop. Thus the "isolated" flag never becomes true; retain that behavior.
    let has_land = state.city_landmasses[civ][1..255].iter().any(|&n| n != 0);
    if military != 0 && (!human || !has_land || state.game_flags & 4 != 0) {
        result += 1;
    }
    if rules.improvement_prerequisites[34] == advance as i8 {
        result += i32::from(state.sea_units) / 4;
    }
    if !human {
        if state.tech_owner_masks[76] & state.human_mask != 0 {
            if rules.requires(32, advance) {
                result += 2;
            }
            for &prerequisite in &rules.improvement_prerequisites[36..=38] {
                if prerequisite == advance as i8 {
                    result += 3;
                }
            }
        }
        let nuclear = rules.unit_prerequisites[28];
        if (0..100).contains(&nuclear)
            && state.tech_owner_masks[nuclear as usize] & state.human_mask != 0
            && rules.requires(nuclear, advance)
        {
            result += 2 + 3 * i32::from(nuclear == advance as i8);
        }
        for wonder in 0..28 {
            if rules.wonder_expiry[wonder] != advance as i8 {
                continue;
            }
            if let Some(owner) = state.wonder_owner(rules, wonder) {
                let disabled =
                    wonder == 20 && state.game_flags & 128 != 0 && state.scenario_flags & 1 != 0;
                if owner == state.civ && advance != 37 && !disabled {
                    result -= 2;
                }
                if owner < 8 && state.human_mask & (1 << owner) != 0 {
                    result += 2;
                }
            }
        }
    }
    for (id, limit) in [(9, rules.growth_limits[0]), (23, rules.growth_limits[1])] {
        if rules.improvement_prerequisites[id] == advance as i8 {
            // If both improvements share an advance, native selects the first
            // limit and adds the bonus only once.
            let largest = state
                .cities
                .iter()
                .filter(|c| c.live && c.owner == state.civ)
                .map(|c| i32::from(c.size))
                .max()
                .unwrap_or(0)
                .max(0);
            if i32::from(limit) <= largest {
                result += 2;
            }
            break;
        }
    }
    if state.leader_modifier >= 0 && state.knows(85, civ) {
        if rules.requires(16, advance) {
            result += i32::from(state.leader_modifier) + 1;
        }
        if advance == 16 {
            result += 2;
        }
    }
    if state.knows(38, civ) && rules.requires(37, advance) {
        result += 1 + i32::from(state.knows(57, civ));
    }
    let fixed: &[(usize, i32)] = match state.leader_id {
        0 => &[(39, 2), (8, 2), (86, 2), (55, -1)],
        1 => &[(12, 1)],
        2 => &[(6, 1), (82, 1), (60, 1)],
        3 => &[(47, 2)],
        4 => &[(21, 2), (15, -1), (73, 1), (16, 1), (42, 1)],
        5 => &[(64, 1), (8, 1), (1, 1), (46, 1), (55, -1), (60, 2)],
        6 | 18 | 19 => &[(64, 2), (36, 1), (56, 1), (9, 1), (55, -1)],
        7 => &[(15, 2), (60, 1), (34, 1)],
        8 => &[(64, 2), (36, 1), (56, 1), (9, 1), (8, -1), (39, -1)],
        9 => &[(42, 1), (81, 1), (17, 1)],
        10 => &[(35, -2), (55, -1)],
        11 => &[(35, 1)],
        12 => &[(55, 1)],
        15 => &[(79, 1), (52, 1)],
        16 => &[(46, 1)],
        17 => &[(55, 1)],
        20 => &[(36, 2)],
        _ => &[],
    };
    result += fixed
        .iter()
        .filter(|&&(id, _)| id == advance)
        .map(|&(_, n)| n)
        .sum::<i32>();
    if (state.leader_id == 12 && military != 0) || (state.leader_id == 17 && military > 1) {
        result += 1;
    }
    if state.tech_owner_masks[advance] == 0 {
        result += 1;
    }
    for (id, tech) in rules.advances.iter().enumerate() {
        if tech.prerequisites.contains(&(advance as i8)) && state.knows(id as i8, civ) {
            result -= 1;
        }
    }
    Some(result.max(1))
}

/// One slot in the original's 100-entry advance table.
#[derive(Debug, Clone, Copy, Default)]
pub struct Candidate {
    /// Prerequisite/ownership/enablement checks and the 0x4a2098 group gate
    /// passed, and the two prerequisites are not both `no` (-2).
    pub eligible: bool,
    /// Context-dependent result of the separate native advance value routine.
    pub value: i32,
}

/// Context used by the original selector, including its human advisor branch.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    pub human: bool,
    pub difficulty: u8,
    pub acquired: u8,
}

/// Select in ascending advance ID order, returning `None` for no candidate.
/// AI candidates with value <= 1 score zero without drawing. Otherwise they
/// score `CRT rand() % value`. Strict comparison retains the first score tie.
/// The signed decrement wraps as in x86: the artificial `i32::MIN` boundary
/// also draws. Real native valuations are clamped to at least one.
/// A human recommendation instead adds `rand() % 3 - 1` to every candidate's
/// value, and (above Chieftain) omits later IDs congruent to acquired mod 3.
pub fn choose(candidates: &[Candidate; 100], context: Context, rng: &mut CrtRand) -> Option<usize> {
    let mut best_score = -1;
    let mut best = None;
    let mut admitted = false;
    for (id, candidate) in candidates.iter().enumerate() {
        if !candidate.eligible {
            continue;
        }
        if context.human
            && admitted
            && context.difficulty != 0
            && (id as i32 - i32::from(context.acquired)) % 3 == 0
        {
            continue;
        }
        admitted = true;
        let score = if context.human {
            candidate
                .value
                .wrapping_add((rng.rand() % 3) as i32)
                .wrapping_sub(1)
        } else if candidate.value.wrapping_sub(1) > 0 {
            rng.rand() as i32 % candidate.value
        } else {
            0
        };
        if score > best_score {
            best_score = score;
            best = Some(id);
        }
    }
    best
}
