//! Native saved-round/research call boundaries (#197), not session admission.
use tot_kit::{ResearchState, Rules, SaveFile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundEntry {
    TurnStart,
    RestartSetup,
    HumanPreparation,
    OpponentDispatch,
}

/// The civ-loop guard at 0x474b62. `resuming` means the native resume-civ
/// byte is nonzero; the outer loop has already selected the current civ.
/// Stops precede turn-start, restart UI setup and human/AI preparation.
pub fn round_entry(resuming: bool, restart: bool, human: bool) -> RoundEntry {
    if !resuming {
        RoundEntry::TurnStart
    } else if restart {
        RoundEntry::RestartSetup
    } else if human {
        RoundEntry::HumanPreparation
    } else {
        RoundEntry::OpponentDispatch
    }
}

/// Conservative context check, unused by current factories/restore. Identifies
/// a measured entry boundary, never a claim that all pending UI state is mapped.
/// This selects the prospective clean/unforced entry policy; it does not infer
/// a serialized native restart flag or restore external process/UI state.
pub fn saved_human_entry(save: &SaveFile, rules: &Rules, human: u8) -> Result<RoundEntry, String> {
    if !(1..8).contains(&human) {
        return Err("saved phase requires a human civilization from 1 to 7".into());
    }
    if save.version != 0x32
        || save.game_mode != 0
        || save.map_header.map_count() != 1
        || save.maps.len() != 1
    {
        return Err("saved phase requires classic Original with one surface context".into());
    }
    if let Some(reason) = super::resume_blocker(save, rules, human) {
        return Err(format!("saved phase: {reason}"));
    }
    if rules.road_multiplier() != Some(3) {
        return Err("saved phase requires an explicitly decoded road multiplier of 3".into());
    }
    if save.active_civ != human || save.civs_mask & (1 << human) == 0 {
        return Err("saved phase requires the live active civilization to match the player".into());
    }
    let alive = save.units.iter().any(|u| u.id() != 0 && u.owner() == human)
        || save
            .cities
            .iter()
            .any(|c| c.city_id() != 0 && c.owner() == u16::from(human));
    if !alive {
        return Err("saved phase player has no live units or cities".into());
    }
    if !(-1..100).contains(&save.research[usize::from(human)].target) {
        return Err("saved phase has an unsupported research target".into());
    }
    Ok(round_entry(true, false, true))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScienceBoundary {
    Return,
    ProjectCost,
    AdvisorBeforeChoice,
    ChooseProject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScienceEntry {
    pub progress: i16,
    pub boundary: ScienceBoundary,
}

/// Original 0x4a1ebf up to its first cost/advice/selection boundary. It does
/// not complete research, choose a project or assert that a modal opens.
/// All original state except progress remains untouched before these stops.
pub fn science_entry(
    state: ResearchState,
    science: i32,
    human: bool,
    advice_enabled: bool,
) -> ScienceEntry {
    if science <= 0 {
        return ScienceEntry {
            progress: state.progress,
            boundary: ScienceBoundary::Return,
        };
    }
    let progress = state.progress.wrapping_add(science as i16);
    let boundary = if state.target >= 0 {
        ScienceBoundary::ProjectCost
    } else if progress == 0 {
        ScienceBoundary::Return
    } else if advice_enabled && human && state.acquired <= 1 {
        ScienceBoundary::AdvisorBeforeChoice
    } else {
        ScienceBoundary::ChooseProject
    };
    ScienceEntry { progress, boundary }
}

#[cfg(test)]
mod tests;
