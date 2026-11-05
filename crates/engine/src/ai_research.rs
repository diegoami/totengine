//! Original research selection loop (`civ2.bak` VA 0x49f98b).
//!
//! This is a decision kernel, not yet the campaign research policy. Callers
//! supply eligibility and the value returned by 0x49c95d; that separate value
//! routine still needs decoding. AI v1 and v0.9 checkpoints keep their policy.
//! Evidence: `docs/evidence/ai-research-choice.json` and the native probe.

use crate::combat::CrtRand;

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
