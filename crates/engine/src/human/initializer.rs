//! Versioned initialization assertions for experimental scenario checkpoints.
//! A recorded profile is an engine initialization policy, not native admission.
use super::*;
use crate::rng::GameRng;

/// Owned parsed rules/events plus a borrowed immutable input snapshot. This
/// is passed by an adapter after capturing all dependencies in temporary values.
pub struct ScenarioInput<'a> {
    pub path: &'a str,
    pub snapshot: &'a [u8],
    pub rules: Rules,
    pub events: Option<crate::events::EventEngine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioStartMode {
    SavedRound,
    FreshTurn,
}

/// A reconstruction recipe, not a native-admission capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioKnowledgeRecipe {
    EngineSightV1,
    SavedInitialThenEngineV1,
}

/// Captured before any human request or opponent work. All original bytes
/// remain in the private source snapshot/dependencies supplied by the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioInitializer {
    pub source: ScenarioOrigin,
    pub mode: ScenarioStartMode,
    pub knowledge_recipe: ScenarioKnowledgeRecipe,
    pub saved_turn: i16,
    pub multiplayer: u8,
    pub player_civ: u8,
    pub active_civ: u8,
    pub human: u8,
    pub phase: Phase,
    pub cursor: u8,
    pub policy: crate::campaign::AiPolicy,
    pub initial_world: u64,
    pub initial_knowledge: u64,
    pub initial_choice: u64,
    pub input_events: u64,
    pub initial_events: u64,
    pub initial_rng: [u32; 2],
    pub initial_rng_draws: [u64; 2],
    /// Canonical adapter manifest, captured with the bytes it describes.
    pub dependencies: String,
}

impl ScenarioInitializer {
    /// Version 1 remains explicitly experimental while native knowledge and
    /// the faithful entry predicates are incomplete.
    pub const PROFILE: &'static str = "experimental-scenario-v1";
    pub const SAVED_VIEW_PROFILE: &'static str = "experimental-saved-view-v1";

    pub fn schema(&self) -> u8 {
        match self.knowledge_recipe {
            ScenarioKnowledgeRecipe::EngineSightV1 => 1,
            ScenarioKnowledgeRecipe::SavedInitialThenEngineV1 => 2,
        }
    }
    pub fn profile(&self) -> &'static str {
        match self.knowledge_recipe {
            ScenarioKnowledgeRecipe::EngineSightV1 => Self::PROFILE,
            ScenarioKnowledgeRecipe::SavedInitialThenEngineV1 => Self::SAVED_VIEW_PROFILE,
        }
    }

    pub(super) fn capture(
        session: &HumanSession,
        saved: &tot_kit::SaveFile,
        dependencies: &str,
        input_events: u64,
    ) -> Self {
        Self {
            source: session.scenario_origin().unwrap().clone(),
            mode: if crate::scenario::resume_blocker(saved, &session.game.rules, session.human)
                .is_none()
            {
                ScenarioStartMode::SavedRound
            } else {
                ScenarioStartMode::FreshTurn
            },
            knowledge_recipe: session.knowledge_recipe,
            saved_turn: saved.turn,
            multiplayer: saved.multiplayer,
            player_civ: saved.player_civ,
            active_civ: saved.active_civ,
            human: session.human,
            phase: session.phase(),
            cursor: session.cursor,
            policy: session.policy,
            initial_world: session.game.world.checksum(),
            initial_knowledge: session.knowledge_fingerprint(),
            initial_choice: crate::player::fnv64(format!("{:?}", session.decision).as_bytes()),
            input_events,
            initial_events: session
                .game
                .events
                .as_ref()
                .map_or(0, crate::events::EventEngine::initializer_fingerprint),
            initial_rng: [
                session.game.world.rng.state(),
                session.game.world.combat_rng.state(),
            ],
            initial_rng_draws: [
                session.game.world.rng.draws(),
                session.game.world.combat_rng.draws(),
            ],
            dependencies: dependencies.into(),
        }
    }

    /// Canonical versioned assertions, checked before the operation journal.
    /// Dependencies are hex encoded metadata, not embedded resource bytes.
    pub fn descriptor(&self) -> String {
        let (mode, movement, legacy_knowledge) = match self.mode {
            ScenarioStartMode::SavedRound => (
                "saved-round",
                "saved-thirds-clamp-v1",
                "saved-exploration-engine-sight-v1",
            ),
            ScenarioStartMode::FreshTurn => ("fresh-turn", "fresh-engine-v1", "engine-sight-v1"),
        };
        let (schema, profile, knowledge) = match self.knowledge_recipe {
            ScenarioKnowledgeRecipe::EngineSightV1 => (1, Self::PROFILE, legacy_knowledge),
            ScenarioKnowledgeRecipe::SavedInitialThenEngineV1 => {
                (2, Self::SAVED_VIEW_PROFILE, "saved-initial-then-engine-v1")
            }
        };
        format!(
            "initializer {schema} profile {profile} source {} {} {:016x} mode {mode} saved {} {} {} {} human {} phase {:?} cursor {} policy {} seed {} movement {movement} knowledge {knowledge} world {:016x} memory {:016x} choice {:016x} events {:016x} {:016x} rng {:08x} {} {:08x} {} dependencies {} {:016x} {}",
            crate::scenario::hex(self.source.path.as_bytes()), self.source.len,
            self.source.fingerprint, self.saved_turn, self.multiplayer, self.player_civ,
            self.active_civ, self.human, self.phase, self.cursor, self.policy.id(), self.source.seed,
            self.initial_world, self.initial_knowledge, self.initial_choice,
            self.input_events, self.initial_events,
            self.initial_rng[0], self.initial_rng_draws[0], self.initial_rng[1], self.initial_rng_draws[1],
            self.dependencies.len(),
            crate::player::fnv64(self.dependencies.as_bytes()),
            crate::scenario::hex(self.dependencies.as_bytes()),
        )
    }

    pub(super) fn recipe_from_descriptor(line: &str) -> Result<ScenarioKnowledgeRecipe, String> {
        if line.starts_with(&format!("initializer 1 profile {} ", Self::PROFILE)) {
            Ok(ScenarioKnowledgeRecipe::EngineSightV1)
        } else if line.starts_with(&format!(
            "initializer 2 profile {} ",
            Self::SAVED_VIEW_PROFILE
        )) {
            Ok(ScenarioKnowledgeRecipe::SavedInitialThenEngineV1)
        } else {
            Err("unsupported scenario initializer version or profile".into())
        }
    }
}

#[cfg(test)]
mod tests;
