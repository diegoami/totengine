//! Classic parsed Rules and world-state adapters for the mechanics boundary.
use super::{value, AdvanceValue, Candidate, ValueCity, ValueContext, ValueRules};
use crate::WorldState;
use tot_kit::Rules;

impl ValueRules {
    /// Parse classic scoring tables, rejecting unresolved references and
    /// unsupported numeric fields instead of silently substituting priorities.
    /// Unused advance/unit slots are disabled. Original text annotations after
    /// the fixed unit/wonder tables do not become extra definitions.
    pub fn from_rules(rules: &Rules) -> Result<Self, String> {
        let techs = rules.section("CIVILIZE").ok_or("missing CIVILIZE")?;
        if techs.rows.is_empty() || techs.rows.len() > 100 {
            return Err("research scoring requires 1..100 advances".into());
        }
        let reference = |raw: Option<&str>| -> Result<i8, String> {
            match raw.map(str::trim) {
                Some(raw) if raw.eq_ignore_ascii_case("nil") => Ok(-1),
                Some(raw) if raw.eq_ignore_ascii_case("no") => Ok(-2),
                // FallRome annotates disabled slots with *Ind / *E2 in
                // comments while prerequisites still use Ind / E2. The native
                // resolver uses case-insensitive fixed slot identifiers.
                Some(abbrev) => techs
                    .comments
                    .iter()
                    .position(|comment| {
                        comment
                            .trim()
                            .trim_start_matches('*')
                            .eq_ignore_ascii_case(abbrev)
                    })
                    .and_then(|id| i8::try_from(id).ok())
                    .ok_or_else(|| format!("unresolved research scoring reference: {abbrev}")),
                None => Err("missing research scoring reference".into()),
            }
        };
        let mut result = Self {
            advances: [AdvanceValue {
                base: 0,
                modifier: 0,
                prerequisites: [-2; 2],
            }; 100],
            unit_prerequisites: [-2; 80],
            improvement_prerequisites: [-2; 68],
            wonder_expiry: [-1; 28],
            growth_limits: [0; 2],
        };
        for (id, row) in techs.rows.iter().enumerate() {
            let number = |field: usize| {
                row.get(field)
                    .and_then(|n| n.trim().parse::<i8>().ok())
                    .ok_or_else(|| format!("invalid signed scoring field {field} for advance {id}"))
            };
            result.advances[id] = AdvanceValue {
                base: number(1)?,
                modifier: number(2)?,
                prerequisites: [
                    reference(row.get(3).map(String::as_str))?,
                    reference(row.get(4).map(String::as_str))?,
                ],
            };
        }
        let units = rules.section("UNITS").ok_or("missing UNITS")?;
        for (id, row) in units.rows.iter().enumerate() {
            if rules.unit_domain(id).is_none() || rules.unit_role(id).is_none() {
                // A text-only footer is retained by the permissive Rules reader.
                if row.len() != 1 || id + 1 != units.rows.len() {
                    return Err(format!("invalid research scoring unit {id}"));
                }
                continue;
            }
            if id >= 80 {
                return Err("research scoring supports at most 80 unit types".into());
            }
            result.unit_prerequisites[id] = reference(rules.unit_prereq_raw(id))?;
        }
        let improvements = rules.section("IMPROVE").ok_or("missing IMPROVE")?;
        if improvements.rows.len() != 68 {
            return Err("research scoring requires the classic 68 improvement rows".into());
        }
        for (id, prerequisite) in result.improvement_prerequisites.iter_mut().enumerate() {
            *prerequisite = reference(rules.improvement_prereq_raw(id))?;
        }
        let expiry = rules.section("ENDWONDER").ok_or("missing ENDWONDER")?;
        if expiry.rows.len() < 28 {
            return Err("research scoring requires 28 wonder expiry rows".into());
        }
        for (id, row) in expiry.rows.iter().take(28).enumerate() {
            result.wonder_expiry[id] = reference(row.first().map(String::as_str))?;
        }
        for (i, row) in [9, 10].into_iter().enumerate() {
            result.growth_limits[i] = rules
                .cosmic_row(row)
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| format!("invalid research growth limit COSMIC {row}"))?;
        }
        Ok(result)
    }
}

impl ValueContext {
    /// Build a privileged mechanics context, with current knowledge masks and
    /// a freshly counted naval roster. Other AI caches retain the world state.
    /// This does not extend the player observation or mutate world RNG/state.
    /// Leader traits come from Rules; saved custom human-leader overrides are
    /// outside this adapter's contract.
    pub fn from_world(world: &WorldState, rules: &Rules, civ: u8) -> Result<Self, String> {
        if !(1..8).contains(&civ) {
            return Err("invalid research scoring civilization".into());
        }
        let c = usize::from(civ);
        let leader_id = world.ai_state.leader_ids[c];
        let leader = usize::try_from(leader_id)
            .ok()
            .filter(|&id| id < 21)
            .ok_or("research scoring requires a classic leader index 0..20")?;
        let leader_modifier = rules
            .section("LEADERS")
            .and_then(|s| s.rows.get(leader))
            .and_then(|row| row.get(9))
            .and_then(|n| n.trim().parse::<i8>().ok())
            .ok_or("missing or invalid LEADERS civilized modifier")?;
        let scenario_flags = world
            .ai_state
            .scenario_flags
            .ok_or("research scoring requires located scenario flags")?;
        let mut sea_units = 0i16;
        for unit in &world.units {
            if unit.id != 0
                && unit.owner == civ
                && rules.unit_domain(unit.type_id as usize) == Some(2)
            {
                sea_units = sea_units.wrapping_add(1);
            }
        }
        Ok(Self {
            civ,
            human_mask: world.human_mask,
            game_flags: world.research_settings.flags,
            scenario_flags,
            leader_id,
            leader_modifier,
            acquired: world.research.map(|s| s.acquired),
            reputations: world.ai_state.reputation,
            treaties: world.diplomacy[c],
            city_landmasses: world.ai_state.city_landmasses,
            tech_owner_masks: std::array::from_fn(|tech| {
                world.techs.iter().enumerate().fold(0, |mask, (c, bits)| {
                    mask | (u8::from(bits[tech / 8] & (1 << (tech % 8)) != 0) << c)
                })
            }),
            sea_units,
            cities: world
                .cities
                .iter()
                .map(|city| ValueCity {
                    owner: city.owner as u8,
                    size: city.size as u8 as i8,
                    live: city.city_id != 0,
                })
                .collect(),
            wonder_cities: world.wonders,
        })
    }
}

impl WorldState {
    /// Original-selector candidate values and gates from privileged mechanics
    /// inputs. This does not grant these inputs to a player observation.
    /// Unlike the broader player research gate, the original selector requires
    /// a zero leader-group flag (0x4a2098), excluding flag 1 as well as flag 2.
    pub fn native_research_candidates(
        &self,
        rules: &Rules,
        civ: u8,
    ) -> Result<[Candidate; 100], String> {
        let tables = ValueRules::from_rules(rules)?;
        let context = ValueContext::from_world(self, rules, civ)?;
        if let Some(groups) = rules.section("CIVILIZE2") {
            for row in groups.rows.iter().take(100) {
                let group = row
                    .first()
                    .and_then(|n| n.trim().parse::<i32>().ok())
                    .ok_or("invalid research scoring technology group")?;
                if !(-1..8).contains(&group) {
                    return Err("unsupported research scoring technology group".into());
                }
            }
        }
        Ok(std::array::from_fn(|id| {
            let tech = tables.advances[id];
            Candidate {
                eligible: !context.knows(id as i8, civ as usize)
                    && tech
                        .prerequisites
                        .iter()
                        .all(|&p| context.knows(p, civ as usize))
                    && self.tech_group_flag(rules, civ, id) == 0,
                value: value(&tables, &context, id).expect("validated scoring context"),
            }
        }))
    }
}
