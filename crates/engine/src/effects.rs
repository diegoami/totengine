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
//! - GIVETECHNOLOGY needs per-civ techs, which the world state does not
//!   model yet. Those effects are reported back as not applied.
//! - TEXT, PLAYCDTRACK and DONTPLAYWONDERS do not touch game state.

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
            Effect::CreateUnit {
                unit_name,
                owner,
                veteran,
                count,
                locations,
            } => {
                let Some(civ) = self.civ_by_name(owner) else {
                    return Applied::Skipped(format!("unknown civ {owner:?}"));
                };
                let Some(type_id) = rules
                    .unit_names()
                    .iter()
                    .position(|n| n.trim().eq_ignore_ascii_case(unit_name.trim()))
                else {
                    return Applied::Skipped(format!("unknown unit {unit_name:?}"));
                };
                let domain = rules.unit_domain(type_id).unwrap_or(0);
                let Some(at) = locations.iter().find_map(|&(x, y)| {
                    let (x, y) = (u16::try_from(x).ok()?, u16::try_from(y).ok()?);
                    self.can_place(civ, domain, x, y).then_some((x, y))
                }) else {
                    return Applied::Skipped("no valid location".into());
                };
                let mut created = Vec::new();
                for _ in 0..*count {
                    if let Some(index) = self.allocate_unit(
                        rules,
                        Unit {
                            type_id: type_id as u8,
                            owner: civ,
                            x: at.0,
                            y: at.1,
                            is_leader: false,
                            moves_left: rules.unit_move(type_id).unwrap_or(0) * 3,
                            veteran: *veteran,
                            damage: 0,
                            orders: 0xff,
                            stack_pos: 0,
                            home: 0xff,
                            id: 0,
                        },
                    ) {
                        created.push(index);
                    }
                }
                if let Some(first) = created.first().copied() {
                    Applied::Created {
                        first,
                        count: created.len() as u32,
                        at,
                    }
                } else {
                    Applied::Skipped("unit capacity exhausted".into())
                }
            }
            Effect::GiveTechnology { .. } => {
                Applied::Skipped("per-civ techs are not modelled yet".into())
            }
            Effect::Text(_) | Effect::PlayTrack(_) | Effect::DontPlayWonders => Applied::NoState,
        }
    }

    /// Unverified placement rule (module docs): on the map, terrain fits
    /// the domain (0 land, 2 sea; air anywhere), no other civ's units.
    fn can_place(&self, civ: u8, domain: u8, x: u16, y: u16) -> bool {
        let Some(t) = self.map.terrain_at(x, y) else {
            return false;
        };
        let ocean = t == OCEAN_TERRAIN;
        let terrain_ok = match domain {
            0 => !ocean,
            2 => ocean,
            _ => true,
        };
        terrain_ok
            && !self
                .units
                .iter()
                .any(|u| (u.x, u.y) == (x, y) && u.owner != civ)
    }
}
