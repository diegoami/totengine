//! Player-facing contact projection for #199.
use super::{Decision, HumanSession};
use crate::WorldState;

const CONTACT: u32 = 1;
const PEACE: u32 = 4;
const ALLIANCE: u32 = 8;
const WAR: u32 = 0x2000;

/// Engine responder used for the bounded, unconditional peace proposal.
pub const PEACE_POLICY: &str = "engine-peace-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalksStage {
    Topics,
    Proposal,
    Accepted,
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TalksReply {
    PreviewPeace,
    Back,
    OfferPeace,
    Leave,
}

impl TalksReply {
    pub(super) fn token(self) -> &'static str {
        match self {
            Self::PreviewPeace => "preview",
            Self::Back => "back",
            Self::OfferPeace => "offer",
            Self::Leave => "leave",
        }
    }

    pub(super) fn from_token(token: &str) -> Option<Self> {
        Some(match token {
            "preview" => Self::PreviewPeace,
            "back" => Self::Back,
            "offer" => Self::OfferPeace,
            "leave" => Self::Leave,
            _ => return None,
        })
    }
}

impl HumanSession {
    pub(super) fn check_peace_contact(&self, other: u8) -> Result<(), &'static str> {
        let world = &self.game.world;
        if !known_contacts(world, self.human)
            .iter()
            .any(|c| c.civilization == other && c.peace_talks_available)
        {
            return Err("Peace talks are unavailable for this contact");
        }
        Ok(())
    }

    pub(super) fn talks_decision(&mut self, other: u8, stage: TalksStage) -> Decision {
        Decision::Talks {
            id: self.allocate_decision(),
            other,
            stage,
        }
    }

    pub(super) fn offer_peace(&mut self, other: u8) -> Result<TalksStage, &'static str> {
        self.check_peace_contact(other)?;
        let world = &mut self.game.world;
        let human = usize::from(self.human);
        let opponent = usize::from(other);
        // Engine policy v1 uses the counterpart's recorded attitude and
        // requires reciprocal contact at war. Neither input is exposed.
        let reciprocal = world.diplomacy[opponent][human];
        if reciprocal & (CONTACT | WAR | PEACE | ALLIANCE) != (CONTACT | WAR)
            || world.ai_state.attitudes[opponent][human] > 50
        {
            return Ok(TalksStage::Refused);
        }
        world.treaty_set(self.human, other, 0x4000 | PEACE);
        Ok(TalksStage::Accepted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactRelationship {
    Unspecified,
    Peace,
    Alliance,
    War,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownContact {
    pub civilization: u8,
    pub name: String,
    pub relationship: ContactRelationship,
    pub peace_talks_available: bool,
}

/// The viewer's contact bit is the disclosure gate. The engine rechecks
/// availability when opening talks or sending a proposal.
pub(super) fn known_contacts(world: &WorldState, viewer: u8) -> Vec<KnownContact> {
    let Some(outgoing) = world.diplomacy.get(usize::from(viewer)) else {
        return Vec::new();
    };
    (1..8)
        .filter(|&other| other != viewer && outgoing[usize::from(other)] & CONTACT != 0)
        .filter_map(|civilization| {
            let flags = outgoing[usize::from(civilization)];
            let relationship = match flags & (PEACE | ALLIANCE | WAR) {
                PEACE => ContactRelationship::Peace,
                WAR => ContactRelationship::War,
                bits if bits == PEACE | ALLIANCE => ContactRelationship::Alliance,
                _ => ContactRelationship::Unspecified,
            };
            let name = world.civ_name(civilization);
            (!name.is_empty()).then_some(KnownContact {
                civilization,
                name,
                relationship,
                peace_talks_available: relationship == ContactRelationship::War
                    && world.human_mask & (1 << civilization) == 0
                    && world.living_civs().contains(&civilization),
            })
        })
        .collect()
}
