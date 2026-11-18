//! Event interpreter (T1-7): fires parsed scenario events against game
//! state and returns the effects the loop must apply.
//!
//! Scope (from the FallRome corpus — 64 events): triggers
//! SCENARIOLOADED / UNITKILLED / CITYTAKEN / NEGOTIATION / RANDOMTURN,
//! actions TEXT / CREATEUNIT / CHANGEMONEY / PLAYCDTRACK / JUSTONCE /
//! DONTPLAYWONDERS. Matching follows Macro.txt: `ANYBODY` is a wildcard,
//! every other name must match exactly (case-insensitive).
//! `JUSTONCE` marks the event as fired (a one-shot latch).
//!
//! Flags (#328, decoded from civ2.bak — evidence §Event flags): FLAG sets
//! per-civ flag words, CHECKFLAG tests them once at each turn start, and
//! flags not set `Continuous` are cleared right after that pass. `@AND`
//! events need both halves active since the last turn start.

use std::collections::HashSet;

use tot_kit::{EventFile, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Macro.txt: create `count` units (default 1) at the FIRST valid
    /// location of `locations`; later entries are fallbacks tried in order
    /// (`Randomize` picks one at random — not modelled).
    CreateUnit {
        unit_name: String,
        owner: String,
        veteran: bool,
        count: u32,
        locations: Vec<(i32, i32)>,
    },
    /// Macro.txt: add `amount` to `receiver`'s treasury; a result below
    /// zero becomes zero.
    ChangeMoney {
        receiver: String,
        amount: i32,
    },
    /// `technology=` is a Rules @CIVILIZE index (FallRome: 91, 99).
    GiveTechnology {
        receiver: String,
        technology: i32,
    },
    /// Macro.txt TEXT: a pop-up box for the player. `audience` keeps the
    /// optional `No Broadcast` restriction (first line of the block).
    Text {
        text: String,
        audience: Audience,
    },
    PlayTrack(i32),
    DontPlayWonders,
}

/// Who sees a TEXT box (Macro.txt: "The optional No Broadcast parameter …
/// specifies that this message should be shown only to the triggering
/// civilization (\"who\")").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audience {
    Everyone,
    /// `No Broadcast`: only the triggering civilization — the trigger's
    /// aggressor (Macro.txt TriggerAttacker, "who") or its talker. `None`
    /// when the trigger names no such civilization (TURN, RANDOMTURN …):
    /// then nobody is shown the box (unverified against the original).
    Triggering(Option<String>),
}

/// One TEXT box an event fired, in firing order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventText {
    pub text: String,
    pub audience: Audience,
}

impl EventText {
    /// Whether the civilization named `civ` (Rules/save plural name) sees it.
    pub fn shown_to(&self, civ: &str) -> bool {
        match &self.audience {
            Audience::Everyone => true,
            Audience::Triggering(Some(who)) => who.trim().eq_ignore_ascii_case(civ.trim()),
            Audience::Triggering(None) => false,
        }
    }
}

/// Splits a TEXT block's optional leading `No Broadcast` line (the space is
/// required, Macro.txt) from its body.
pub fn text_block(raw: &str) -> (&str, bool) {
    let mut lines = raw.splitn(2, '\n');
    let first = lines.next().unwrap_or("");
    if first.trim().eq_ignore_ascii_case("no broadcast") {
        (lines.next().unwrap_or(""), true)
    } else {
        (raw, false)
    }
}

/// Context for one trigger evaluation.
#[derive(Debug, Clone, Default)]
pub struct Ctx {
    pub unit: Option<String>,
    pub attacker: Option<String>,
    pub defender: Option<String>,
    pub talker: Option<String>,
    pub listener: Option<String>,
    /// CITYTAKEN's `city=` (exact name match like every other name).
    pub city: Option<String>,
    /// Current turn, for TURN (`turn=N` or `Every`) and TURNINTERVAL.
    pub turn: Option<i32>,
    pub denominator: Option<i32>,
}

#[derive(Clone)]
pub struct EventEngine {
    pub file: EventFile,
    fired: HashSet<usize>,
    /// Every TEXT box fired so far, in order — also returned as effects.
    /// Sessions read new entries by index; a cloned engine (transactional
    /// rollback) carries its log with it.
    pub log: Vec<EventText>,
    /// Per-civ flag words, civs 0..=7 (civ2.bak: the @INITFLAG record's
    /// `+0x88 + civ*4`).
    pub flags: [u32; 8],
    /// Flags set with `Continuous`: ONE word for all civs (`+0xb4`). Every
    /// other flag is cleared after the turn-start CHECKFLAG pass.
    pub continuous: u32,
    /// @AND activation of each event's two halves since the last turn
    /// start (civ2.bak record bits 0x20000000 / 0x80000000).
    active: Vec<[bool; 2]>,
    /// Plural civ names by id 0..=7, so FLAG/CHECKFLAG `who=` can name a
    /// flag word. Refreshed by the world before each firing
    /// ([`crate::WorldState::fire_event`], the turn loop).
    pub civ_names: [String; 8],
    /// The file has `@INITFLAG` (kept apart: `file` is moved out while
    /// events run).
    init_flags: bool,
}

fn word_of(v: Option<&Value>) -> Option<&str> {
    v.and_then(|v| v.word())
}

/// ANYBODY / ANYONE match everything; otherwise names must match
/// **exactly**, case-insensitively — Macro.txt (shipped with the game):
/// "unit types, civilization names, and so on must match the
/// corresponding names in the rules.txt file exactly". (Prefix matching
/// would fire FallRome's "Theodoric II dies" event when Theodoric I dies.)
/// Action civ parameters may name the trigger's parties (Macro.txt:
/// TriggerAttacker / TriggerDefender); resolve them against the context.
/// Unresolvable names pass through unchanged.
fn resolve_civ(name: &str, ctx: &Ctx) -> String {
    let n = name.trim();
    let r = if n.eq_ignore_ascii_case("TRIGGERATTACKER") {
        ctx.attacker.as_deref()
    } else if n.eq_ignore_ascii_case("TRIGGERDEFENDER") {
        ctx.defender.as_deref()
    } else {
        None
    };
    r.unwrap_or(n).to_string()
}

fn matches(param: Option<&str>, actual: Option<&str>) -> bool {
    let Some(p) = param else { return true };
    if p.eq_ignore_ascii_case("ANYBODY") || p.eq_ignore_ascii_case("ANYONE") {
        return true;
    }
    let Some(a) = actual else { return false };
    p.trim().eq_ignore_ascii_case(a.trim())
}

impl EventEngine {
    pub fn new(file: EventFile) -> Self {
        let n = file.events.len();
        let init_flags = file.init_flags;
        EventEngine {
            file,
            fired: HashSet::new(),
            log: Vec::new(),
            flags: [0; 8],
            continuous: 0,
            active: vec![[false; 2]; n],
            civ_names: Default::default(),
            init_flags,
        }
    }

    /// Evaluate every event with this trigger keyword; returns effects.
    /// JUSTONCE events fire at most once; non-JUSTONCE events re-arm.
    pub fn fire(&mut self, trigger: &str, ctx: &Ctx) -> Vec<Effect> {
        let d = ctx.denominator;
        self.fire_with(trigger, ctx, &mut |denom| d == Some(denom))
    }

    /// [`EventEngine::fire`] with the RANDOMTURN roll supplied by the
    /// caller: `roll(n)` is asked once per RANDOMTURN event half, in file
    /// order, and returns whether its 1-in-`n` chance came up (Macro.txt:
    /// "a one in 40 chance every turn").
    pub fn fire_with(
        &mut self,
        trigger: &str,
        ctx: &Ctx,
        roll: &mut dyn FnMut(i32) -> bool,
    ) -> Vec<Effect> {
        let file = std::mem::take(&mut self.file);
        let mut effects = Vec::new();
        for (i, e) in file.events.iter().enumerate() {
            for half in 0..2 {
                if self.fired.contains(&i) {
                    break;
                }
                let t = match half {
                    0 => &e.trigger,
                    _ => match &e.and {
                        Some(t) => t,
                        None => break,
                    },
                };
                if t.keyword.eq_ignore_ascii_case(trigger) && trigger_matches(t, ctx, roll) {
                    self.activate(i, e, half, ctx, &mut effects);
                }
            }
        }
        self.file = file;
        effects
    }

    /// The start of a game turn, in the original's order (civ2.bak call
    /// sites 0x47451b…0x47456f): @AND activations lapse, then TURN,
    /// TURNINTERVAL and RANDOMTURN fire, then CHECKFLAG is evaluated and
    /// every non-continuous flag cleared.
    pub fn turn_start(&mut self, ctx: &Ctx, roll: &mut dyn FnMut(i32) -> bool) -> Vec<Effect> {
        self.lapse_activations();
        let mut effects = self.fire_with("TURN", ctx, roll);
        effects.extend(self.fire_with("TURNINTERVAL", ctx, roll));
        effects.extend(self.fire_with("RANDOMTURN", ctx, roll));
        effects.extend(self.check_flags(ctx));
        effects
    }

    /// civ2.bak 0x575d9a: an @AND half keeps its activation across turns
    /// only when its trigger is `Continuous`. The first half also carries
    /// the second half's Continuous bit (copied at @ENDIF, 0x4db290…).
    fn lapse_activations(&mut self) {
        for (i, e) in self.file.events.iter().enumerate() {
            let Some(second) = &e.and else { continue };
            let keep2 = second.has_word("CONTINUOUS");
            let keep1 = keep2 || e.trigger.has_word("CONTINUOUS");
            let a = &mut self.active[i];
            a[0] &= keep1;
            a[1] &= keep2;
        }
    }

    /// The @AND gate (civ2.bak 0x575cbe): a half that fires marks itself
    /// and passes only if the other half is marked. Two halves true in the
    /// same pass run the actions once (the second one passes).
    fn gate(&mut self, i: usize, e: &tot_kit::Event, half: usize) -> bool {
        if e.and.is_none() {
            return true;
        }
        self.active[i][half] = true;
        self.active[i][1 - half]
    }

    fn activate(
        &mut self,
        i: usize,
        e: &tot_kit::Event,
        half: usize,
        ctx: &Ctx,
        effects: &mut Vec<Effect>,
    ) {
        if self.gate(i, e, half) {
            self.run_actions(i, e, ctx, effects);
        }
    }

    /// Resolve a flag `who=` to civ ids: a plural name, TRIGGERATTACKER /
    /// TRIGGERDEFENDER via the context, `Everybody` (or no `who=` on FLAG)
    /// all eight civs. Anything else — Anybody, Somebody, unknown names —
    /// resolves to nothing (civ2.bak 0x575899).
    fn flag_civs(&self, who: Option<&str>, ctx: &Ctx) -> Vec<usize> {
        let Some(w) = who.map(str::trim) else {
            return (0..8).rev().collect();
        };
        if w.eq_ignore_ascii_case("EVERYBODY") {
            return (0..8).rev().collect();
        }
        let name = resolve_civ(w, ctx);
        self.civ_id(&name).into_iter().collect()
    }

    fn civ_id(&self, name: &str) -> Option<usize> {
        let n = name.trim();
        if n.is_empty() {
            return None;
        }
        self.civ_names
            .iter()
            .position(|c| !c.is_empty() && c.eq_ignore_ascii_case(n))
    }

    /// FLAG (civ2.bak 0x5757b7): set or clear one flag (`flag=N`) or a
    /// `mask=` of them. Setting with `Continuous` adds the bits to the
    /// global continuous word, setting without it removes them, and
    /// clearing removes them too.
    fn apply_flag(&mut self, a: &tot_kit::Command, continuous: bool, ctx: &Ctx) {
        if !self.init_flags {
            return;
        }
        let bits = match (a.param("mask"), a.param("flag").and_then(|v| v.number())) {
            (Some(m), _) => mask_value(m),
            (None, Some(n)) => Some(1u32 << (n as u32 & 31)),
            _ => None,
        };
        let (Some(bits), Some(on)) = (bits, state_on(a.param("state"))) else {
            return;
        };
        for c in self.flag_civs(word_of(a.param("who")), ctx) {
            if on {
                self.flags[c] |= bits;
            } else {
                self.flags[c] &= !bits;
            }
        }
        if on && continuous {
            self.continuous |= bits;
        } else {
            self.continuous &= !bits;
        }
    }

    /// The turn-start CHECKFLAG pass (civ2.bak 0x574c3e), then the decay of
    /// every flag not in the continuous word (0x5750e5).
    pub fn check_flags(&mut self, ctx: &Ctx) -> Vec<Effect> {
        let mut effects = Vec::new();
        if !self.init_flags {
            return effects;
        }
        let file = std::mem::take(&mut self.file);
        for (i, e) in file.events.iter().enumerate() {
            for half in 0..2 {
                if self.fired.contains(&i) {
                    break;
                }
                let t = match half {
                    0 => &e.trigger,
                    _ => match &e.and {
                        Some(t) => t,
                        None => break,
                    },
                };
                if !t.keyword.eq_ignore_ascii_case("CHECKFLAG") {
                    continue;
                }
                let Some(test) = FlagTest::of(t) else {
                    continue;
                };
                let who = word_of(t.param("who")).unwrap_or("").trim();
                if who.eq_ignore_ascii_case("SOMEBODY") {
                    // Each matching civ 1..=7 runs the event as the
                    // trigger's civilization (0x574e79…0x574f9f).
                    for c in 1..8 {
                        if self.fired.contains(&i) || !test.passes(self.flags[c]) {
                            continue;
                        }
                        let mut cx = ctx.clone();
                        cx.attacker = Some(self.civ_names[c].clone());
                        self.activate(i, e, half, &cx, &mut effects);
                    }
                } else if who.eq_ignore_ascii_case("EVERYBODY") {
                    let all = self.flags[1..].iter().fold(0, |m, f| m | f);
                    if test.passes(all) {
                        self.activate(i, e, half, ctx, &mut effects);
                    }
                } else if let Some(c) = self.civ_id(who).filter(|&c| c > 0) {
                    // A named civ (never the barbarians: 0x574d52).
                    if test.passes(self.flags[c]) {
                        let mut cx = ctx.clone();
                        cx.attacker = Some(self.civ_names[c].clone());
                        self.activate(i, e, half, &cx, &mut effects);
                    }
                }
            }
        }
        self.file = file;
        for f in &mut self.flags {
            *f &= self.continuous;
        }
        effects
    }

    fn run_actions(&mut self, i: usize, e: &tot_kit::Event, ctx: &Ctx, effects: &mut Vec<Effect>) {
        let justonce = e.actions.iter().any(|a| a.keyword == "JUSTONCE");
        let continuous = e.actions.iter().any(|a| a.keyword == "CONTINUOUS");
        for a in &e.actions {
            match a.keyword.as_str() {
                "TEXT" => {
                    if let Some(Value::Text(s)) = a.args.first() {
                        let (body, restricted) = text_block(s);
                        let audience = if restricted {
                            Audience::Triggering(
                                ctx.attacker.clone().or_else(|| ctx.talker.clone()),
                            )
                        } else {
                            Audience::Everyone
                        };
                        effects.push(Effect::Text {
                            text: body.to_string(),
                            audience: audience.clone(),
                        });
                        self.log.push(EventText {
                            text: body.to_string(),
                            audience,
                        });
                    }
                }
                "CREATEUNIT" => {
                    let unit = word_of(a.param("unit")).unwrap_or("").to_string();
                    let owner = resolve_civ(word_of(a.param("owner")).unwrap_or("ANYBODY"), ctx);
                    let veteran = word_of(a.param("veteran"))
                        .map(|v| v.eq_ignore_ascii_case("yes"))
                        .unwrap_or(false);
                    let count = a
                        .param("count")
                        .and_then(|v| v.number())
                        .map(|n| n.clamp(1, 255) as u32)
                        .unwrap_or(1);
                    if let Some(Value::Points(pts)) =
                        a.args.iter().find(|v| matches!(v, Value::Points(_)))
                    {
                        effects.push(Effect::CreateUnit {
                            unit_name: unit,
                            owner,
                            veteran,
                            count,
                            locations: pts.clone(),
                        });
                    }
                }
                "CHANGEMONEY" => {
                    if let (Some(n), Some(r)) = (
                        a.param("amount").and_then(|v| v.number()),
                        word_of(a.param("receiver")),
                    ) {
                        effects.push(Effect::ChangeMoney {
                            receiver: resolve_civ(r, ctx),
                            amount: n,
                        });
                    }
                }
                "PLAYCDTRACK" => {
                    if let Some(n) = a.args.first().and_then(|v| v.number()) {
                        effects.push(Effect::PlayTrack(n));
                    }
                }
                "DONTPLAYWONDERS" => effects.push(Effect::DontPlayWonders),
                "GIVETECHNOLOGY" => {
                    // Macro.txt: `technology=` (an index) and
                    // `receiver=`; the old `tech=` key never occurs.
                    if let (Some(t), Some(r)) = (
                        a.param("technology").and_then(|v| v.number()),
                        word_of(a.param("receiver")),
                    ) {
                        effects.push(Effect::GiveTechnology {
                            receiver: resolve_civ(r, ctx),
                            technology: t,
                        });
                    }
                }
                "FLAG" => self.apply_flag(a, continuous, ctx),
                _ => {} // JUSTONCE and unknown actions carry no effect
            }
        }
        if justonce {
            self.fired.insert(i);
        }
    }

    pub fn fired_count(&self) -> usize {
        self.fired.len()
    }
}

/// `state=` of FLAG / CHECKFLAG: On/Set or Off/Clear.
fn state_on(v: Option<&Value>) -> Option<bool> {
    let w = word_of(v)?.trim();
    if w.eq_ignore_ascii_case("ON") || w.eq_ignore_ascii_case("SET") {
        Some(true)
    } else if w.eq_ignore_ascii_case("OFF") || w.eq_ignore_ascii_case("CLEAR") {
        Some(false)
    } else {
        None
    }
}

/// A `mask=`: `0b…` binary, `0x…` hex or decimal (Macro.txt "reverse
/// programmer notation": the last digit is flag 0).
fn mask_value(v: &Value) -> Option<u32> {
    match v {
        Value::Number(n) => Some(*n as u32),
        Value::Word(w) => {
            let w = w.trim();
            let (digits, radix) = if let Some(b) = w.strip_prefix("0b").or(w.strip_prefix("0B")) {
                (b, 2)
            } else if let Some(h) = w.strip_prefix("0x").or(w.strip_prefix("0X")) {
                (h, 16)
            } else {
                (w, 10)
            };
            u32::from_str_radix(digits, radix).ok()
        }
        _ => None,
    }
}

/// A parsed CHECKFLAG condition (civ2.bak parse 0x4d6200…0x4d68e7, test
/// 0x574cf6…0x574e47).
struct FlagTest {
    mask: u32,
    on: bool,
    /// Matches needed: `Threshold`/`Count` (default 1), at least 1 and at
    /// most the number of flags in the mask. `Count` behaves exactly like
    /// `Threshold` (a minimum): the original's equality test falls through
    /// to the `>=` test on a miss.
    need: u32,
}

impl FlagTest {
    fn of(t: &tot_kit::Command) -> Option<FlagTest> {
        // technology= adds a known-advance test the engine does not
        // evaluate here; such a trigger never fires (unverified scope).
        if t.param("technology").is_some() {
            return None;
        }
        let mask = match (t.param("mask"), t.param("flag").and_then(|v| v.number())) {
            (Some(m), _) => mask_value(m)?,
            (None, Some(n)) if (0..32).contains(&n) => 1u32 << n,
            _ => return None,
        };
        let on = state_on(t.param("state")).unwrap_or(false);
        let n = t
            .param("threshold")
            .or(t.param("count"))
            .and_then(|v| v.number())
            .unwrap_or(1)
            .max(1) as u32;
        Some(FlagTest {
            mask,
            on,
            need: n.min(mask.count_ones()),
        })
    }

    fn passes(&self, word: u32) -> bool {
        let hits = (word & self.mask).count_ones();
        let matched = if self.on {
            hits
        } else {
            self.mask.count_ones() - hits
        };
        matched >= self.need
    }
}

/// Party and turn conditions of one trigger (half).
fn trigger_matches(t: &tot_kit::Command, ctx: &Ctx, roll: &mut dyn FnMut(i32) -> bool) -> bool {
    if !matches(word_of(t.param("unit")), ctx.unit.as_deref())
        || !matches(word_of(t.param("attacker")), ctx.attacker.as_deref())
        || !matches(word_of(t.param("defender")), ctx.defender.as_deref())
        || !matches(word_of(t.param("talker")), ctx.talker.as_deref())
        || !matches(word_of(t.param("listener")), ctx.listener.as_deref())
        || !matches(word_of(t.param("city")), ctx.city.as_deref())
    {
        return false;
    }
    if t.keyword.eq_ignore_ascii_case("RANDOMTURN") {
        let denom = t.param("denominator").and_then(|v| v.number()).unwrap_or(2);
        if !roll(denom) {
            return false;
        }
    }
    // TURN: turn=N or turn=Every (Macro.txt)
    if t.keyword.eq_ignore_ascii_case("TURN") {
        let every = word_of(t.param("turn")).is_some_and(|w| w.eq_ignore_ascii_case("every"));
        let n = t.param("turn").and_then(|v| v.number());
        if !(every || (n.is_some() && n == ctx.turn)) {
            return false;
        }
    }
    // TURNINTERVAL: modelled as `turn % interval == 0`. Macro.txt
    // counts from the last activation; the two agree when that
    // was a multiple of the interval (unverified otherwise).
    if t.keyword.eq_ignore_ascii_case("TURNINTERVAL") {
        let i = t.param("interval").and_then(|v| v.number()).unwrap_or(0);
        if i <= 0 || ctx.turn.is_none_or(|turn| turn % i != 0) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tot_kit::parse_events;

    fn fallrome_events() -> Option<EventFile> {
        let p = std::env::var_os("CIVTOT_HOME")?;
        let src =
            std::fs::read_to_string(std::path::Path::new(&p).join("Scenario/FallRome/Events.txt"))
                .ok()?;
        parse_events(&src).ok()
    }

    #[test]
    fn scenario_loaded_fires_track_only() {
        let f =
            parse_events("@IF\nSCENARIOLOADED\n@THEN\nPLAYCDTRACK\n11\nDONTPLAYWONDERS\n@ENDIF")
                .unwrap();
        let mut eng = EventEngine::new(f);
        let eff = eng.fire("SCENARIOLOADED", &Ctx::default());
        assert_eq!(eff, vec![Effect::PlayTrack(11), Effect::DontPlayWonders]);
    }

    #[test]
    fn theodoric_killed_creates_successor_once() {
        let Some(f) = fallrome_events() else {
            eprintln!("skipping: no FallRome Events.txt");
            return;
        };
        let mut eng = EventEngine::new(f);
        let ctx = Ctx {
            unit: Some("Theodoric I".into()),
            attacker: Some("Huns".into()),
            defender: Some("Visigoths".into()),
            ..Default::default()
        };
        let eff = eng.fire("UNITKILLED", &ctx);
        let creates: Vec<_> = eff
            .iter()
            .filter(|e| matches!(e, Effect::CreateUnit { .. }))
            .collect();
        // ONE create (Macro.txt: first valid location, the rest are
        // fallbacks) of one veteran Theodoric II, six candidate tiles.
        assert_eq!(creates.len(), 1, "{creates:?}");
        match creates[0] {
            Effect::CreateUnit {
                unit_name,
                owner,
                veteran,
                count,
                locations,
            } => {
                assert_eq!(
                    (unit_name.as_str(), owner.as_str()),
                    ("Theodoric II", "Visigoths")
                );
                assert!(*veteran);
                assert_eq!(*count, 1);
                assert_eq!(locations.len(), 6);
                assert_eq!(locations[0], (24, 60));
            }
            _ => unreachable!(),
        }
        assert!(eff
            .iter()
            .any(|e| matches!(e, Effect::Text { text, audience: Audience::Everyone } if text.contains("Theodoric II"))));
        // JUSTONCE: a second kill of the same king does not re-fire
        let again = eng.fire("UNITKILLED", &ctx);
        assert!(again.is_empty(), "JUSTONCE latch held");
    }

    /// Macro.txt: `No Broadcast` (first line of the block, space required)
    /// limits the box to the triggering civilization ("who").
    #[test]
    fn no_broadcast_text_goes_to_the_triggering_civ_only() {
        let f = parse_events(
            "@IF\nCITYTAKEN\ncity=Roma\nattacker=ANYBODY\ndefender=ANYBODY\n@THEN\nTEXT\nNo Broadcast\nRoma falls!\nENDTEXT\n@ENDIF\n\
             @IF\nTURN\nturn=3\n@THEN\nTEXT\nno broadcast\nquiet\nENDTEXT\nTEXT\nloud\nENDTEXT\n@ENDIF",
        )
        .unwrap();
        let mut eng = EventEngine::new(f);
        let ctx = Ctx {
            city: Some("Roma".into()),
            attacker: Some("Huns".into()),
            defender: Some("Romans".into()),
            ..Default::default()
        };
        let eff = eng.fire("CITYTAKEN", &ctx);
        assert_eq!(
            eff,
            vec![Effect::Text {
                text: "Roma falls!".into(),
                audience: Audience::Triggering(Some("Huns".into())),
            }]
        );
        let turn = Ctx {
            turn: Some(3),
            ..Default::default()
        };
        eng.fire("TURN", &turn);
        assert_eq!(eng.log.len(), 3);
        assert!(eng.log[0].shown_to("huns") && !eng.log[0].shown_to("Romans"));
        // No triggering civilization: a restricted box reaches nobody.
        assert_eq!(eng.log[1].text, "quiet");
        assert!(!eng.log[1].shown_to("Huns"));
        assert!(eng.log[2].shown_to("Romans"));
    }

    #[test]
    fn exact_and_anybody_matching() {
        let f = parse_events(
            "@IF\nUNITKILLED\nunit=Theodoric I\ndefender=west romans\nattacker=ANYBODY\n@THEN\nTEXT\nhit\nENDTEXT\n@ENDIF",
        )
        .unwrap();
        let mut eng = EventEngine::new(f);
        let ctx = Ctx {
            unit: Some("Theodoric I".into()),
            attacker: Some("Francs".into()),
            defender: Some("West Romans".into()),
            ..Default::default()
        };
        assert!(
            !eng.fire("UNITKILLED", &ctx).is_empty(),
            "exact (any case) + ANYBODY"
        );
        for unit in ["Theodoric II", "Theodoric"] {
            let wrong = Ctx {
                unit: Some(unit.into()),
                ..ctx.clone()
            };
            assert!(
                eng.fire("UNITKILLED", &wrong).is_empty(),
                "{unit} is not Theodoric I"
            );
        }
        let wrong = Ctx {
            defender: Some("West".into()),
            ..ctx
        };
        assert!(eng.fire("UNITKILLED", &wrong).is_empty(), "no prefix match");
    }

    #[test]
    fn receivers_and_trigger_parties_resolve() {
        let Some(f) = fallrome_events() else {
            eprintln!("skipping: no FallRome Events.txt");
            return;
        };
        let mut eng = EventEngine::new(f);
        // Theodoric II's death: CREATEUNIT owner=TRIGGERDEFENDER
        let ctx = Ctx {
            unit: Some("Theodoric II".into()),
            attacker: Some("Huns".into()),
            defender: Some("Visigoths".into()),
            ..Default::default()
        };
        let eff = eng.fire("UNITKILLED", &ctx);
        assert!(
            eff.iter()
                .any(|e| matches!(e, Effect::CreateUnit { unit_name, owner, .. }
                if unit_name == "Barbarian King" && owner == "Visigoths")),
            "{eff:?}"
        );
        // Tyras taken from the Huns: GIVETECHNOLOGY receiver=TRIGGERATTACKER
        // technology=99 (it never fired under the old `tech=` key)
        let ctx = Ctx {
            attacker: Some("Byzantines".into()),
            defender: Some("Huns".into()),
            city: Some("Tyras".into()),
            ..Default::default()
        };
        let other = Ctx {
            city: Some("Olbia".into()),
            ..ctx.clone()
        };
        assert!(
            !eng.fire("CITYTAKEN", &other)
                .iter()
                .any(|e| matches!(e, Effect::GiveTechnology { .. })),
            "city= must match: Olbia is not Tyras"
        );
        let eff = eng.fire("CITYTAKEN", &ctx);
        assert!(
            eff.contains(&Effect::GiveTechnology {
                receiver: "Byzantines".into(),
                technology: 99
            }),
            "{eff:?}"
        );
        // Merchant ransacked by the Anglo-Saxons: CHANGEMONEY receiver
        let ctx = Ctx {
            unit: Some("Merchant".into()),
            attacker: Some("Anglo-Saxons".into()),
            defender: Some("West Romans".into()),
            ..Default::default()
        };
        let eff = eng.fire("UNITKILLED", &ctx);
        assert!(eff.contains(&Effect::ChangeMoney {
            receiver: "Anglo-Saxons".into(),
            amount: 50
        }));
    }

    fn flag_engine(body: &str) -> EventEngine {
        let mut eng = EventEngine::new(parse_events(body).unwrap());
        eng.civ_names = std::array::from_fn(|c| {
            ["Barbarians", "Gauls", "Romans", "Egyptians", "", "", "", ""][c].to_string()
        });
        eng
    }

    fn money(effects: &[Effect]) -> Vec<(String, i32)> {
        effects
            .iter()
            .filter_map(|e| match e {
                Effect::ChangeMoney { receiver, amount } => Some((receiver.clone(), *amount)),
                _ => None,
            })
            .collect()
    }

    fn taken(attacker: &str, city: &str) -> Ctx {
        Ctx {
            attacker: Some(attacker.into()),
            defender: Some("Romans".into()),
            city: Some(city.into()),
            ..Default::default()
        }
    }

    const FLAGS: &str = "@BEGINEVENTS\n@INITFLAG\n\
        @IF\nCITYTAKEN\ncity=A\nattacker=Gauls\n@THEN\nFLAG\ncontinuous\nflag=1\nWho=Gauls\nstate=on\n@ENDIF\n\
        @IF\nCITYTAKEN\ncity=B\nattacker=Gauls\n@THEN\nFLAG\nflag=2\nWho=TriggerAttacker\nstate=on\n@ENDIF\n\
        @IF\nCITYTAKEN\ncity=C\nattacker=Gauls\n@THEN\nFLAG\ncontinuous\nflag=1\nWho=Gauls\nstate=off\n@ENDIF\n\
        @IF\nCheckflag\nWho=Gauls\nMask=0b0110\nThreshold=2\nState=On\n@THEN\nCHANGEMONEY\nreceiver=TriggerAttacker\namount=5\n@ENDIF\n\
        @IF\nCheckflag\nWho=Gauls\nflag=1\nState=On\n@THEN\nCHANGEMONEY\nreceiver=Gauls\namount=1\n@ENDIF\n\
        @ENDEVENTS\n";

    #[test]
    fn checkflag_runs_at_turn_start_and_non_continuous_flags_decay() {
        let mut eng = flag_engine(FLAGS);
        let turn = Ctx {
            turn: Some(1),
            ..Default::default()
        };
        let mut no = |_| false;
        eng.fire("CITYTAKEN", &taken("Gauls", "A"));
        eng.fire("CITYTAKEN", &taken("Gauls", "B"));
        assert_eq!(eng.flags[1], 0b110);
        assert_eq!(eng.continuous, 0b010);
        // Checked only at turn start, as the trigger civ (who=Gauls).
        assert_eq!(
            money(&eng.turn_start(&turn, &mut no)),
            [("Gauls".to_string(), 5), ("Gauls".to_string(), 1)]
        );
        // flag 2 was not continuous: it lapsed after that pass.
        assert_eq!(eng.flags[1], 0b010);
        assert_eq!(
            money(&eng.turn_start(&turn, &mut no)),
            [("Gauls".to_string(), 1)]
        );
        // Clearing a continuous flag drops it from the continuous word.
        eng.fire("CITYTAKEN", &taken("Gauls", "C"));
        assert_eq!((eng.flags[1], eng.continuous), (0, 0));
        assert!(money(&eng.turn_start(&turn, &mut no)).is_empty());
    }

    #[test]
    fn flags_need_initflag() {
        let mut eng = flag_engine(&FLAGS.replace("@INITFLAG\n", ""));
        eng.fire("CITYTAKEN", &taken("Gauls", "A"));
        assert_eq!(eng.flags, [0; 8]);
        let mut no = |_| false;
        assert!(eng.turn_start(&Ctx::default(), &mut no).is_empty());
    }

    #[test]
    fn somebody_everybody_and_count() {
        let mut eng = flag_engine(
            "@INITFLAG\n\
             @IF\nCHECKFLAG\nwho=Somebody\nmask=0b11\ncount=2\nstate=on\n@THEN\nCHANGEMONEY\nreceiver=TriggerAttacker\namount=1\n@ENDIF\n\
             @IF\nCHECKFLAG\nwho=Everybody\nmask=0b11\nthreshold=2\nstate=on\n@THEN\nCHANGEMONEY\nreceiver=Romans\namount=2\n@ENDIF\n\
             @IF\nCHECKFLAG\nwho=Barbarians\nflag=0\nstate=on\n@THEN\nCHANGEMONEY\nreceiver=Romans\namount=3\n@ENDIF\n\
             @IF\nCHECKFLAG\nwho=Romans\nmask=0b11\nthreshold=9\nstate=off\n@THEN\nCHANGEMONEY\nreceiver=Romans\namount=4\n@ENDIF\n",
        );
        eng.flags = [0b11, 0b11, 0b01, 0b10, 0, 0, 0, 0];
        let mut no = |_| false;
        let got = money(&eng.turn_start(&Ctx::default(), &mut no));
        // SOMEBODY: once per matching civ (Gauls only — Count acts as a
        // minimum and the barbarians never count); EVERYBODY: Romans'
        // and Egyptians' words OR to 0b11. who=Barbarians never fires.
        // Romans: threshold 9 clamps to 2 flags, not both off.
        assert_eq!(got, [("Gauls".to_string(), 1), ("Romans".to_string(), 2)]);
        assert_eq!(eng.flags, [0; 8], "nothing continuous");
    }

    #[test]
    fn and_events_need_both_halves_since_the_last_turn_start() {
        let mut eng = flag_engine(
            "@INITFLAG\n\
             @IF\nCHECKFLAG\nwho=Gauls\nflag=0\nstate=on\n@AND\nRANDOMTURN\ndenominator=3\n@THEN\nCHANGEMONEY\nreceiver=Gauls\namount=1\n@ENDIF\n\
             @IF\nCITYTAKEN\ncity=A\nattacker=Gauls\n@AND\nUNITKILLED\nunit=Hero\n@THEN\nJUSTONCE\nCHANGEMONEY\nreceiver=Gauls\namount=2\n@ENDIF\n",
        );
        let ctx = Ctx::default();
        // Roll succeeds, flag off: nothing.
        assert!(eng.turn_start(&ctx, &mut |_| true).is_empty());
        // Flag on, roll fails: nothing.
        eng.flags[1] = 1;
        assert!(eng.turn_start(&ctx, &mut |_| false).is_empty());
        // Both in the same turn start: RANDOMTURN runs first, CHECKFLAG passes.
        eng.flags[1] = 1;
        assert_eq!(money(&eng.turn_start(&ctx, &mut |_| true)).len(), 1);
        // Capture then kill in one turn: once (JUSTONCE).
        assert!(money(&eng.fire("CITYTAKEN", &taken("Gauls", "A"))).is_empty());
        let kill = Ctx {
            unit: Some("Hero".into()),
            ..Default::default()
        };
        assert_eq!(
            money(&eng.fire("UNITKILLED", &kill)),
            [("Gauls".to_string(), 2)]
        );
        assert!(eng.fire("UNITKILLED", &kill).is_empty());
        // Halves activated in different turns do not combine, unless the
        // earlier half is Continuous.
        for (continuous, expect) in [("", 0), ("Continuous\n", 1)] {
            let mut eng = flag_engine(&format!(
                "@IF\nCITYTAKEN\ncity=A\n{continuous}@AND\nUNITKILLED\nunit=Hero\n@THEN\n\
                 CHANGEMONEY\nreceiver=Gauls\namount=2\n@ENDIF\n"
            ));
            eng.fire("CITYTAKEN", &taken("Gauls", "A"));
            eng.turn_start(&ctx, &mut |_| false);
            assert_eq!(money(&eng.fire("UNITKILLED", &kill)).len(), expect);
        }
    }
}
