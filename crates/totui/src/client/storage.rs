//! Explicit engine-checkpoint slots. Loading never replaces a session on error.
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub fn nonce() -> Result<String, String> {
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).map_err(|e| format!("OS randomness unavailable: {e}"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub fn default_folder() -> PathBuf {
    if let Some(p) = std::env::var_os("TOTENGINE_SAVES") {
        return p.into();
    }
    #[cfg(windows)]
    let root = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")));
    root.unwrap_or_else(std::env::temp_dir)
        .join("totengine/saves")
}
fn slot(root: &Path, name: &str) -> Result<PathBuf, String> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Use 1–64 letters, numbers, - or _ for the checkpoint name".into());
    }
    Ok(root.join(format!("{name}.cth")))
}
pub fn list(root: &Path) -> Vec<String> {
    let mut names = fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|e| {
            let p = e.path();
            if e.file_type().ok()?.is_file() && p.extension()?.to_str()? == "cth" {
                p.file_stem()?.to_str().map(str::to_owned)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}
pub fn load(root: &Path, name: &str) -> Result<String, String> {
    let p = slot(root, name)?;
    if fs::symlink_metadata(&p)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Checkpoint slots cannot be symlinks".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(p)
        .map_err(|e| e.to_string())?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("Checkpoint exceeds 16 MiB".into());
    }
    String::from_utf8(bytes).map_err(|_| {
        "Not a supported UTF-8 engine checkpoint; original SAV import is not available".into()
    })
}
pub fn save(root: &Path, name: &str, text: &str) -> Result<(), String> {
    let target = slot(root, name)?;
    if text.len() > 16 * 1024 * 1024 {
        return Err("Checkpoint exceeds 16 MiB".into());
    }
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    temp.write_all(text.as_bytes())
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    temp.persist_noclobber(&target)
        .map(|_| ())
        .map_err(|e| format!("Could not create checkpoint; choose a new name: {e}"))
}

/// Host-side setup memory (#266 shortlist): the last successfully started
/// setup, so the ToT directory and the world choices do not have to be
/// re-entered on every launch. One file beside the saves folder serves both
/// clients; the seed is deliberately not remembered (it regenerates per
/// launch by design). Delete the file to reset.
#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupMemory {
    pub format: String,
    pub home: String,
    pub civs: u8,
    pub human: u8,
    pub difficulty: u8,
    /// Map-size index: 0 standard, 1 medium, 2 small.
    pub size: usize,
}

pub fn setup_memory_path(root: &Path) -> Option<PathBuf> {
    root.parent().map(|p| p.join("setup.json"))
}

/// Bounded read and strict parse; every failure means "no memory", never a
/// blocked startup.
pub fn load_setup_memory(root: &Path) -> Option<SetupMemory> {
    let path = setup_memory_path(root)?;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(4096 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 4096 {
        return None;
    }
    let m: SetupMemory = serde_json::from_slice(&bytes).ok()?;
    (m.format == "totengine-setup 1"
        && (2..=7).contains(&m.civs)
        && (1..=7).contains(&m.human)
        && m.difficulty <= 5
        && m.size <= 2
        && m.home.len() <= 4096)
        .then_some(m)
}

/// Best-effort; a failed write leaves the game untouched.
pub fn save_setup_memory(root: &Path, memory: &SetupMemory) -> Result<(), String> {
    let path = setup_memory_path(root).ok_or("no setup memory path")?;
    let parent = path.parent().ok_or("no parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    let text = serde_json::to_string(memory).map_err(|e| e.to_string())?;
    temp.write_all(text.as_bytes())
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    temp.persist(&path).map(|_| ()).map_err(|e| e.to_string())
}

/// Opt-in client enhancements (#284 item 6, the B layer of #266): QoL that
/// differs from the original, every option off by default. One file beside
/// the saves folder serves both clients; delete it to reset. Presentation and
/// confirmation preferences only: nothing here reaches the engine or a
/// checkpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enhancements {
    pub format: String,
    /// Show a turn's new notices in an overlay on the map.
    pub notice_overlay: bool,
    /// Ask before every move, not only attacks and captures.
    pub confirm_moves: bool,
    /// Describe the tile, city and units under a resting pointer on the map.
    /// Absent in files written before it existed, so it defaults off.
    #[serde(default)]
    pub map_tooltips: bool,
    /// While a unit is selected, show the route and turns to the tile under
    /// a resting pointer. Defaults off for older files.
    #[serde(default)]
    pub path_preview: bool,
    /// In the science advisor, estimate the turns to the goal and allow
    /// sorting the available advances (#315). Defaults off for older files.
    #[serde(default)]
    pub goal_estimates: bool,
}

pub const ENHANCEMENTS_FORMAT: &str = "totengine-enhancements 1";

pub fn enhancements_path(root: &Path) -> Option<PathBuf> {
    root.parent().map(|p| p.join("enhancements.json"))
}

/// Bounded read and strict parse; any failure means "all off".
pub fn load_enhancements(root: &Path) -> Enhancements {
    let read = || -> Option<Enhancements> {
        let mut bytes = Vec::new();
        fs::File::open(enhancements_path(root)?)
            .ok()?
            .take(4096 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() > 4096 {
            return None;
        }
        let e: Enhancements = serde_json::from_slice(&bytes).ok()?;
        (e.format == ENHANCEMENTS_FORMAT).then_some(e)
    };
    read().unwrap_or_else(|| Enhancements {
        format: ENHANCEMENTS_FORMAT.into(),
        ..Enhancements::default()
    })
}

/// Atomic write; a failure leaves the previous file in place.
pub fn save_enhancements(root: &Path, e: &Enhancements) -> Result<(), String> {
    let path = enhancements_path(root).ok_or("no enhancements path")?;
    let parent = path.parent().ok_or("no parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    let text = serde_json::to_string(e).map_err(|e| e.to_string())?;
    temp.write_all(text.as_bytes())
        .and_then(|_| temp.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    temp.persist(&path).map(|_| ()).map_err(|e| e.to_string())
}
