//! Player-facing contact projection for #199.
use crate::WorldState;

const CONTACT: u32 = 1;
const PEACE: u32 = 4;
const ALLIANCE: u32 = 8;
const WAR: u32 = 0x2000;

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
}

/// The viewer's contact bit is the disclosure gate. Treaty mutations and
/// conversation availability belong to the authoritative decision path.
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
            })
        })
        .collect()
}
