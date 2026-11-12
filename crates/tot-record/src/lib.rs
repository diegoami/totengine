//! Session recording bundles, format 1 (#206; design in docs/recording-bundles.md).
//!
//! A bundle is one UTF-8 `.totrec` file: a manifest of identities and labels,
//! then length- and fingerprint-checked sections, then an `end` trailer.
//!
//! - `gameplay`: the canonical engine checkpoint (campaign or human). It is the
//!   only reproducible content; it reconstructs the game by replay.
//! - `observer`: derived observer stream v1 frames, labelled with their
//!   perspective. Verification regenerates them from the gameplay section.
//! - `presentation`: notes, markers and camera/view changes. Inert: it is
//!   validated but never executed and never affects gameplay.
//!
//! Original resources are never embedded. Rules and resources are bound by
//! identity (length + FNV-1a of the canonical text), so a mismatch yields a
//! diagnostic instead of a silently different game.
//!
//! Bundles are written atomically and never overwrite an existing file. A
//! [`Journal`] written during a run makes an interrupted recording
//! recoverable up to its last complete checkpoint record.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

use engine::campaign::{AiPolicy, Campaign, Outcome};
use engine::human::HumanSession;
use engine::newgame::NewGameConfig;
use engine::observer::Recorder;
use tot_kit::Rules;

pub const FORMAT: &str = "totengine-recording 1";
pub const JOURNAL_FORMAT: &str = "totengine-recording-journal 1";
/// Upper bound for a bundle or journal file.
pub const MAX_BYTES: usize = 64 * 1024 * 1024;

/// FNV-1a 64, the fingerprint used by engine checkpoints.
pub fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

/// `<len> <fnv>` identity of a text.
pub fn identity(text: &str) -> String {
    format!("{} {:016x}", text.len(), fingerprint(text.as_bytes()))
}

/// Identity of a Rules set, as human checkpoints bind it (canonical emission).
pub fn rules_identity(rules: &Rules) -> String {
    identity(&tot_kit::emit_rules(rules))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An all-AI generated campaign (`totengine-campaign` checkpoint).
    Campaign,
    /// A human session against built-in opponents (`totengine-human` checkpoint).
    Human,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Campaign => "campaign",
            Kind::Human => "human",
        }
    }
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "campaign" => Ok(Kind::Campaign),
            "human" => Ok(Kind::Human),
            _ => Err(format!("unknown recording kind {s:?}")),
        }
    }
}

/// Whether the bundle can reconstruct the game or only shows recorded views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Content {
    Reproducible,
    ViewOnly,
}

impl Content {
    fn as_str(self) -> &'static str {
        match self {
            Content::Reproducible => "reproducible",
            Content::ViewOnly => "view-only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The requested number of rounds completed.
    TurnLimit,
    OneCivilizationRemaining(u8),
    NoCivilizationsRemaining,
    /// The user or caller stopped the recording.
    Stopped,
    /// Recovered from a journal after an interrupted write.
    Interrupted,
}

impl StopReason {
    fn encode(self) -> String {
        match self {
            StopReason::TurnLimit => "turn-limit".into(),
            StopReason::OneCivilizationRemaining(c) => format!("one-civilization {c}"),
            StopReason::NoCivilizationsRemaining => "no-civilizations".into(),
            StopReason::Stopped => "stopped".into(),
            StopReason::Interrupted => "interrupted".into(),
        }
    }
    fn parse(s: &str) -> Result<Self, String> {
        Ok(match s {
            "turn-limit" => StopReason::TurnLimit,
            "no-civilizations" => StopReason::NoCivilizationsRemaining,
            "stopped" => StopReason::Stopped,
            "interrupted" => StopReason::Interrupted,
            _ => match s.strip_prefix("one-civilization ") {
                Some(c) => StopReason::OneCivilizationRemaining(
                    c.parse()
                        .map_err(|_| format!("invalid stop reason {s:?}"))?,
                ),
                None => return Err(format!("invalid stop reason {s:?}")),
            },
        })
    }
}

impl From<Outcome> for StopReason {
    fn from(outcome: Outcome) -> Self {
        match outcome {
            Outcome::TurnLimit => StopReason::TurnLimit,
            Outcome::OneCivilizationRemaining(c) => StopReason::OneCivilizationRemaining(c),
            Outcome::NoCivilizationsRemaining => StopReason::NoCivilizationsRemaining,
        }
    }
}

/// Which views an observer section contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Perspective {
    /// Spectator plus every civ's fogged view (full state).
    AllViews,
    /// The full-state spectator view only.
    Spectator,
    /// One civilization's fogged view only.
    Civ(u8),
}

impl Perspective {
    pub fn encode(self) -> String {
        match self {
            Perspective::AllViews => "all-views".into(),
            Perspective::Spectator => "spectator".into(),
            Perspective::Civ(c) => format!("civ-{c}"),
        }
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "all-views" => Ok(Perspective::AllViews),
            "spectator" => Ok(Perspective::Spectator),
            _ => match s.strip_prefix("civ-").and_then(|c| c.parse().ok()) {
                Some(c @ 1..=7) => Ok(Perspective::Civ(c)),
                _ => Err(format!("invalid perspective {s:?}")),
            },
        }
    }
    /// Whether the content shows more than one player's observation.
    pub fn full_state(self) -> bool {
        !matches!(self, Perspective::Civ(_))
    }
    fn keep(self, frame: &mut engine::observer::Snapshot) {
        match self {
            Perspective::AllViews => {}
            Perspective::Spectator => frame.views.retain(|v| v.civ.is_none()),
            Perspective::Civ(c) => frame.views.retain(|v| v.civ == Some(c)),
        }
    }
}

/// Derived observer frames; frame 0 is the state at `from_round`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observer {
    pub perspective: Perspective,
    pub from_round: u32,
    /// Observer stream v1 NDJSON, one frame per line.
    pub frames: String,
}

/// The identities and labels a bundle declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub kind: Kind,
    pub engine: String,
    /// `NewGameConfig::descriptor()`.
    pub setup: String,
    /// Opponent / all-civ policy identity.
    pub policy: String,
    /// The human civilization for human recordings.
    pub human: Option<u8>,
    /// `rules_identity` of the Rules the run used.
    pub rules: String,
    /// The resource identity text a human session bound (no paths, no assets).
    pub resources: Option<String>,
    /// Rounds completed (campaign) or the world turn counter (human).
    pub progress: u32,
    pub stop: StopReason,
    /// Free text naming what produced the recording, e.g. `totctl campaign`.
    pub producer: String,
}

/// A recording bundle in memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub manifest: Manifest,
    /// The canonical checkpoint text; `None` for a view-only bundle.
    pub gameplay: Option<String>,
    pub observer: Option<Observer>,
    /// Inert presentation NDJSON (notes, markers, camera/view changes).
    pub presentation: Option<String>,
}

fn single_line(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.contains('\n') || value.contains('\r') {
        Err(format!("{field} must be one non-empty line"))
    } else {
        Ok(())
    }
}

/// Validate presentation NDJSON: one object per line with a non-negative
/// integer `round` no later than `max_round` and a string `kind`.
pub fn validate_presentation(text: &str, max_round: u32) -> Result<(), String> {
    for (n, line) in text.lines().enumerate() {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("presentation line {}: {e}", n + 1))?;
        let round = value
            .get("round")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("presentation line {}: missing round", n + 1))?;
        if round > u64::from(max_round) {
            return Err(format!(
                "presentation line {}: round {round} is after the recording ends",
                n + 1
            ));
        }
        if !value.get("kind").is_some_and(serde_json::Value::is_string) {
            return Err(format!("presentation line {}: missing kind", n + 1));
        }
    }
    if !text.is_empty() && !text.ends_with('\n') {
        return Err("presentation must end with a newline".into());
    }
    Ok(())
}

impl Manifest {
    fn header(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "engine {}", self.engine);
        let _ = writeln!(out, "kind {}", self.kind.as_str());
        let _ = writeln!(out, "setup {}", self.setup);
        let _ = writeln!(out, "policy {}", self.policy);
        let _ = writeln!(
            out,
            "human {}",
            self.human.map_or("none".into(), |h| h.to_string())
        );
        let _ = writeln!(out, "rules {}", self.rules);
        let _ = writeln!(out, "producer {}", self.producer);
        out
    }

    fn check(&self) -> Result<(), String> {
        single_line("engine", &self.engine)?;
        single_line("setup", &self.setup)?;
        single_line("policy", &self.policy)?;
        single_line("rules", &self.rules)?;
        single_line("producer", &self.producer)?;
        NewGameConfig::from_descriptor(&self.setup)?;
        match (self.kind, self.human, &self.resources) {
            (Kind::Campaign, None, None) => Ok(()),
            (Kind::Human, Some(_), Some(_)) => Ok(()),
            (Kind::Campaign, _, _) => Err("campaign recordings have no human or resources".into()),
            (Kind::Human, _, _) => Err("human recordings need a human civ and resources".into()),
        }
    }
}

/// Reads `key value` lines in a fixed order.
struct Lines<'a> {
    rest: &'a str,
}

impl<'a> Lines<'a> {
    fn line(&mut self) -> Result<&'a str, String> {
        let (line, rest) = self
            .rest
            .split_once('\n')
            .ok_or("truncated recording (missing line)")?;
        self.rest = rest;
        Ok(line)
    }
    fn field(&mut self, key: &str) -> Result<&'a str, String> {
        let line = self.line()?;
        line.strip_prefix(key)
            .and_then(|v| v.strip_prefix(' '))
            .ok_or_else(|| format!("expected `{key}`, found {line:?}"))
    }
    fn bytes(&mut self, len: usize) -> Result<&'a str, String> {
        if self.rest.len() < len + 1 || !self.rest.is_char_boundary(len) {
            return Err("truncated recording section".into());
        }
        let (body, rest) = self.rest.split_at(len);
        self.rest = rest
            .strip_prefix('\n')
            .ok_or("section is not newline-terminated")?;
        Ok(body)
    }
}

fn parse_manifest_head(lines: &mut Lines) -> Result<Manifest, String> {
    let engine = lines.field("engine")?.to_string();
    let kind = Kind::parse(lines.field("kind")?)?;
    let setup = lines.field("setup")?.to_string();
    let policy = lines.field("policy")?.to_string();
    let human = match lines.field("human")? {
        "none" => None,
        h => Some(h.parse().map_err(|_| format!("invalid human civ {h:?}"))?),
    };
    let rules = lines.field("rules")?.to_string();
    let producer = lines.field("producer")?.to_string();
    Ok(Manifest {
        kind,
        engine,
        setup,
        policy,
        human,
        rules,
        resources: None,
        progress: 0,
        stop: StopReason::Stopped,
        producer,
    })
}

fn section(out: &mut String, name: &str, label: &str, body: &str) {
    let _ = writeln!(out, "section {name} {label} {}", identity(body));
    out.push_str(body);
    out.push('\n');
}

impl Bundle {
    pub fn content(&self) -> Content {
        if self.gameplay.is_some() {
            Content::Reproducible
        } else {
            Content::ViewOnly
        }
    }

    /// Structural validation (no replay): identities, labels, presentation.
    pub fn check(&self) -> Result<(), String> {
        self.manifest.check()?;
        if self.gameplay.is_none() && self.observer.is_none() {
            return Err("a recording needs gameplay or observer content".into());
        }
        if let Some(observer) = &self.observer {
            if observer.from_round > self.manifest.progress {
                return Err("observer frames start after the recording ends".into());
            }
            if observer.frames.is_empty() || !observer.frames.ends_with('\n') {
                return Err("observer section must hold newline-terminated frames".into());
            }
        }
        if let Some(text) = &self.presentation {
            validate_presentation(text, self.manifest.progress)?;
        }
        Ok(())
    }

    pub fn encode(&self) -> Result<String, String> {
        self.check()?;
        let m = &self.manifest;
        let mut out = format!("{FORMAT}\n");
        out.push_str(&m.header());
        let _ = writeln!(out, "content {}", self.content().as_str());
        let _ = writeln!(out, "progress {}", m.progress);
        let _ = writeln!(out, "stop {}", m.stop.encode());
        let mut count = 0;
        if let Some(resources) = &m.resources {
            section(&mut out, "resources", "identity", resources);
            count += 1;
        }
        if let Some(gameplay) = &self.gameplay {
            section(&mut out, "gameplay", "checkpoint", gameplay);
            count += 1;
        }
        if let Some(observer) = &self.observer {
            let label = format!("{}@{}", observer.perspective.encode(), observer.from_round);
            section(&mut out, "observer", &label, &observer.frames);
            count += 1;
        }
        if let Some(presentation) = &self.presentation {
            section(&mut out, "presentation", "inert", presentation);
            count += 1;
        }
        let _ = writeln!(out, "end {count}");
        if out.len() > MAX_BYTES {
            return Err(format!("recording exceeds {MAX_BYTES} bytes"));
        }
        Ok(out)
    }

    pub fn decode(text: &str) -> Result<Self, String> {
        if text.len() > MAX_BYTES {
            return Err(format!("recording exceeds {MAX_BYTES} bytes"));
        }
        let mut lines = Lines { rest: text };
        let format = lines.line()?;
        if format != FORMAT {
            return Err(if format.starts_with("totengine-recording ") {
                format!("unsupported recording format {format:?}; this build reads {FORMAT:?}")
            } else {
                "not a ToTEngine recording".into()
            });
        }
        let mut manifest = parse_manifest_head(&mut lines)?;
        let content = lines.field("content")?;
        manifest.progress = lines
            .field("progress")?
            .parse()
            .map_err(|_| "invalid progress")?;
        manifest.stop = StopReason::parse(lines.field("stop")?)?;
        let mut bundle = Bundle {
            manifest,
            gameplay: None,
            observer: None,
            presentation: None,
        };
        let mut count = 0usize;
        let mut order = 0;
        loop {
            let line = lines.line()?;
            if let Some(n) = line.strip_prefix("end ") {
                if n.parse() != Ok(count) || !lines.rest.is_empty() {
                    return Err("recording trailer does not match its sections".into());
                }
                break;
            }
            let fields: Vec<&str> = line
                .strip_prefix("section ")
                .ok_or_else(|| format!("expected a section, found {line:?}"))?
                .split(' ')
                .collect();
            let [name, label, len, fnv] = fields[..] else {
                return Err(format!("malformed section header {line:?}"));
            };
            let len: usize = len.parse().map_err(|_| "invalid section length")?;
            let body = lines.bytes(len)?;
            if format!("{:016x}", fingerprint(body.as_bytes())) != fnv {
                return Err(format!("section {name} is corrupt (fingerprint differs)"));
            }
            let rank = match name {
                "resources" => 1,
                "gameplay" => 2,
                "observer" => 3,
                "presentation" => 4,
                _ => return Err(format!("unknown section {name:?}")),
            };
            if rank <= order {
                return Err(format!("section {name} is duplicated or out of order"));
            }
            order = rank;
            count += 1;
            match (name, label) {
                ("resources", "identity") => bundle.manifest.resources = Some(body.into()),
                ("gameplay", "checkpoint") => bundle.gameplay = Some(body.into()),
                ("observer", label) => {
                    let (perspective, from) = label
                        .split_once('@')
                        .ok_or("observer label needs perspective@round")?;
                    bundle.observer = Some(Observer {
                        perspective: Perspective::parse(perspective)?,
                        from_round: from.parse().map_err(|_| "invalid observer start")?,
                        frames: body.into(),
                    });
                }
                ("presentation", "inert") => bundle.presentation = Some(body.into()),
                _ => return Err(format!("section {name} has unknown label {label:?}")),
            }
        }
        if content != bundle.content().as_str() {
            return Err(format!(
                "declared content {content:?} does not match the sections present"
            ));
        }
        bundle.check()?;
        Ok(bundle)
    }

    /// A campaign bundle from a run. `observer` frames, if any, must come from
    /// the same run (they are regenerated and compared by [`Bundle::verify`]).
    pub fn campaign(
        campaign: &Campaign,
        stop: StopReason,
        producer: &str,
        observer: Option<Observer>,
    ) -> Self {
        Bundle {
            manifest: Manifest {
                kind: Kind::Campaign,
                engine: env!("CARGO_PKG_VERSION").into(),
                setup: campaign.config().descriptor(),
                policy: campaign.policy().id().into(),
                human: None,
                rules: rules_identity(&campaign.game.rules),
                resources: None,
                progress: campaign.rounds(),
                stop,
                producer: producer.into(),
            },
            gameplay: Some(campaign.checkpoint()),
            observer,
            presentation: None,
        }
    }

    /// A human bundle from a session checkpoint. The checkpoint is restored
    /// (replayed) first, so an invalid one is never written.
    pub fn human(
        checkpoint: &str,
        rules: &Rules,
        resources: &str,
        stop: StopReason,
        producer: &str,
    ) -> Result<Self, String> {
        let session = HumanSession::restore(rules.clone(), resources, checkpoint)?;
        let view = session.view();
        let setup = checkpoint
            .lines()
            .nth(2)
            .and_then(|l| l.strip_prefix("setup "))
            .ok_or("human checkpoint has no setup line")?;
        Ok(Bundle {
            manifest: Manifest {
                kind: Kind::Human,
                engine: env!("CARGO_PKG_VERSION").into(),
                setup: setup.into(),
                policy: "builtin-1".into(),
                human: Some(view.observation.civ),
                rules: rules_identity(rules),
                resources: Some(resources.into()),
                progress: u32::try_from(view.observation.turn).unwrap_or(0),
                stop,
                producer: producer.into(),
            },
            gameplay: Some(checkpoint.into()),
            observer: None,
            presentation: None,
        })
    }

    /// Replay the gameplay section against `rules` and check every derived
    /// claim: identities, progress, stop reason and observer frames.
    pub fn verify(&self, rules: &Rules) -> Result<Verified, String> {
        self.check()?;
        let m = &self.manifest;
        if m.engine != env!("CARGO_PKG_VERSION") {
            return Err(format!(
                "recorded with engine {}, this build is {}; replay needs the same engine",
                m.engine,
                env!("CARGO_PKG_VERSION")
            ));
        }
        let actual = rules_identity(rules);
        if m.rules != actual {
            return Err(format!(
                "Rules identity differs: recording needs {}, supplied Rules are {actual}",
                m.rules
            ));
        }
        let Some(gameplay) = &self.gameplay else {
            return Ok(Verified {
                content: Content::ViewOnly,
                fingerprint: None,
                frames: self
                    .observer
                    .as_ref()
                    .map_or(0, |o| o.frames.lines().count()),
            });
        };
        let fingerprint = match m.kind {
            Kind::Campaign => {
                let campaign = Campaign::restore(rules.clone(), gameplay)?;
                if campaign.config().descriptor() != m.setup
                    || campaign.policy().id() != m.policy
                    || campaign.rounds() != m.progress
                {
                    return Err("manifest setup/policy/progress differ from the gameplay".into());
                }
                let expected = match m.stop {
                    StopReason::Stopped | StopReason::Interrupted => None,
                    _ => Some(StopReason::from(campaign.outcome())),
                };
                if expected.is_some_and(|e| e != m.stop) {
                    return Err(format!(
                        "declared stop reason {} differs from the replayed outcome",
                        m.stop.encode()
                    ));
                }
                if let Some(observer) = &self.observer {
                    let regenerated = campaign_frames(
                        rules,
                        campaign.config(),
                        campaign.policy(),
                        observer.from_round,
                        m.progress,
                        observer.perspective,
                    )?;
                    if regenerated != observer.frames {
                        return Err("observer frames differ from the replayed run".into());
                    }
                }
                campaign.game.world.checksum()
            }
            Kind::Human => {
                let resources = m.resources.as_deref().unwrap_or_default();
                let session = HumanSession::restore(rules.clone(), resources, gameplay)?;
                let view = session.view();
                if Some(view.observation.civ) != m.human
                    || u32::try_from(view.observation.turn).ok() != Some(m.progress)
                {
                    return Err("manifest human/progress differ from the gameplay".into());
                }
                if self.observer.is_some() {
                    return Err("observer frames for human recordings are not supported".into());
                }
                let world = gameplay
                    .lines()
                    .find_map(|l| l.strip_prefix("world "))
                    .and_then(|w| u64::from_str_radix(w, 16).ok())
                    .ok_or("human checkpoint has no world fingerprint")?;
                world
            }
        };
        Ok(Verified {
            content: Content::Reproducible,
            fingerprint: Some(fingerprint),
            frames: self
                .observer
                .as_ref()
                .map_or(0, |o| o.frames.lines().count()),
        })
    }
}

/// What verification established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub content: Content,
    /// Final world checksum after replay; `None` for view-only bundles.
    pub fingerprint: Option<u64>,
    pub frames: usize,
}

/// Observer stream v1 frames of a campaign from `from_round` to `to_round`
/// (inclusive), regenerated by replaying from setup.
pub fn campaign_frames(
    rules: &Rules,
    config: NewGameConfig,
    policy: AiPolicy,
    from_round: u32,
    to_round: u32,
    perspective: Perspective,
) -> Result<String, String> {
    let mut campaign = Campaign::with_policy(config, rules.clone(), policy)?;
    campaign.advance(from_round)?;
    if campaign.rounds() != from_round {
        return Err("observer start is after the campaign ended".into());
    }
    let mut recorder = Recorder::default();
    let mut out = String::new();
    let mut push = |c: &Campaign, out: &mut String| {
        let mut frame = recorder.capture(&c.game);
        perspective.keep(&mut frame);
        out.push_str(&frame.to_json());
        out.push('\n');
    };
    push(&campaign, &mut out);
    while campaign.rounds() < to_round {
        let before = campaign.rounds();
        campaign.advance(1)?;
        if campaign.rounds() == before {
            break;
        }
        push(&campaign, &mut out);
    }
    Ok(out)
}

/// Write `text` to `path` atomically; never replaces an existing file.
pub fn write_new(path: &Path, text: &str) -> Result<(), String> {
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let mut temp = tempfile::Builder::new()
        .prefix(".totrec-partial-")
        .tempfile_in(dir)
        .map_err(|e| format!("create temporary file in {}: {e}", dir.display()))?;
    temp.write_all(text.as_bytes())
        .and_then(|()| temp.as_file().sync_all())
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    temp.persist_noclobber(path)
        .map(|_| ())
        .map_err(|e| format!("create {}: {}", path.display(), e.error))
}

/// Read a recording or journal file with the size limit.
pub fn read_limited(path: &Path) -> Result<String, String> {
    use std::io::Read as _;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    if bytes.len() > MAX_BYTES {
        return Err(format!("{} exceeds {MAX_BYTES} bytes", path.display()));
    }
    String::from_utf8(bytes).map_err(|_| format!("{} is not UTF-8 text", path.display()))
}

/// An append-only journal kept while a run is recorded. Each record is a
/// complete checkpoint, framed with its length and fingerprint and synced to
/// disk, so an interruption loses at most the work since the last record.
pub struct Journal {
    file: std::fs::File,
}

impl Journal {
    /// Create a new journal; fails if the file already exists (an earlier
    /// interrupted recording must be recovered or removed first).
    pub fn create(path: &Path, manifest: &Manifest) -> Result<Self, String> {
        manifest.check()?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| format!("create journal {}: {e}", path.display()))?;
        let mut head = format!("{JOURNAL_FORMAT}\n{}", manifest.header());
        if let Some(resources) = &manifest.resources {
            section(&mut head, "resources", "identity", resources);
        }
        file.write_all(head.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|e| format!("write journal {}: {e}", path.display()))?;
        Ok(Journal { file })
    }

    /// Append one complete checkpoint at `progress`.
    pub fn record(&mut self, progress: u32, checkpoint: &str) -> Result<(), String> {
        let mut text = format!("record {progress} {}\n", identity(checkpoint));
        text.push_str(checkpoint);
        text.push('\n');
        self.file
            .write_all(text.as_bytes())
            .and_then(|()| self.file.sync_data())
            .map_err(|e| format!("write journal: {e}"))
    }
}

/// What recovery found in a journal.
#[derive(Debug)]
pub struct Recovered {
    pub bundle: Bundle,
    /// Complete records found; the bundle uses the last one.
    pub records: usize,
    /// Bytes after the last complete record (a torn write), discarded.
    pub discarded_bytes: usize,
}

/// Rebuild a bundle from a journal's last complete record. The result is
/// marked `interrupted`; its gameplay still replays like any checkpoint.
pub fn recover(journal: &str) -> Result<Recovered, String> {
    let mut lines = Lines { rest: journal };
    if lines.line()? != JOURNAL_FORMAT {
        return Err("not a ToTEngine recording journal".into());
    }
    let mut manifest = parse_manifest_head(&mut lines)?;
    if manifest.kind == Kind::Human {
        let header = lines.line()?;
        let fields: Vec<&str> = header.split(' ').collect();
        let ["section", "resources", "identity", len, fnv] = fields[..] else {
            return Err("journal is missing its resources identity".into());
        };
        let body = lines.bytes(len.parse().map_err(|_| "invalid length")?)?;
        if format!("{:016x}", fingerprint(body.as_bytes())) != fnv {
            return Err("journal resources identity is corrupt".into());
        }
        manifest.resources = Some(body.into());
    }
    let mut last: Option<(u32, &str)> = None;
    let mut records = 0;
    let mut good = lines.rest;
    loop {
        let mut attempt = Lines { rest: good };
        let Ok(header) = attempt.line() else { break };
        let fields: Vec<&str> = header.split(' ').collect();
        let ["record", progress, len, fnv] = fields[..] else {
            break;
        };
        let (Ok(progress), Ok(len)) = (progress.parse::<u32>(), len.parse::<usize>()) else {
            break;
        };
        let Ok(body) = attempt.bytes(len) else { break };
        if format!("{:016x}", fingerprint(body.as_bytes())) != fnv {
            break;
        }
        last = Some((progress, body));
        records += 1;
        good = attempt.rest;
    }
    let (progress, checkpoint) = last.ok_or("journal holds no complete checkpoint record")?;
    manifest.progress = progress;
    manifest.stop = StopReason::Interrupted;
    let bundle = Bundle {
        manifest,
        gameplay: Some(checkpoint.into()),
        observer: None,
        presentation: None,
    };
    bundle.check()?;
    Ok(Recovered {
        bundle,
        records,
        discarded_bytes: good.len(),
    })
}

#[cfg(test)]
mod tests;
