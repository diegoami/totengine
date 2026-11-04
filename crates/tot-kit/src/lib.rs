//! `tot-kit` — Civilization II: Test of Time data-format readers and writers.
//!
//! Every reader documents what is *proven* (verified against the local install
//! and recorded in `docs/evidence/`) versus what is still open. Writers must
//! round-trip through the original game before they count as done (T1-8).
//!
//! Provenance: layout findings live in `docs/evidence/README.md`; the external
//! structural spec is Mercator's Apolyton thread (see `docs/plan.md` §7).

mod events;
mod mp;
mod rules;
mod sav;

pub use events::{parse_events, Command, Event, EventError, EventFile, Value};
pub use mp::{parse_mp, parse_mp_file, MpFormat, MpMap};
pub use rules::{emit_rules, parse_rules, Rules, Section, TechDef, WonderDef};
pub use sav::{
    civ_names_from, emit_sav, latin1, parse_sav, parse_sav_file, CityRecord, CivNames, MapBlock,
    MapHeader, ResearchSettings, ResearchState, SavEditor, SavError, SaveFile, UnitRecord, MAGIC,
    OFF_CIV_NAMES, OFF_CIV_PROPS, OFF_GAME, OFF_MENU, OFF_TECH, OFF_TRANSPORTERS,
    OFF_TRANSPORT_REL, OFF_UNKNOWN_288, OFF_WONDERS,
};
