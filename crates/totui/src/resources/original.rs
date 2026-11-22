//! Closed reference dependency set for future classic admission (#197).
//! This adapter captures inputs; it does not construct or admit a session.
use super::*;
use sha2::{Digest, Sha256};
use std::borrow::Cow;

mod inspection;

#[derive(Clone)]
struct Pin {
    role: &'static str,
    file: &'static str,
    bytes: usize,
    sha256: Cow<'static, str>,
}

// Derived identities, never source assets. Re-derived by
// tools/import_dependencies_check.py; only this reference set is accepted.
const PINS: [Pin; 12] = [
    Pin {
        role: "rules",
        file: "Rules.txt",
        bytes: 46102,
        sha256: Cow::Borrowed("640ae61d6c6e871131a01647099983913e1a1bb98ce1e6503be5b9bc9da8adef"),
    },
    Pin {
        role: "events",
        file: "Events.txt",
        bytes: 604,
        sha256: Cow::Borrowed("742d7475e6fe3431a4e4d08494455f4696b82cd3f53b2805c05eeb6ecc92f312"),
    },
    Pin {
        role: "labels",
        file: "Labels.txt",
        bytes: 13873,
        sha256: Cow::Borrowed("793a7148ad0f6f983646a86baf52b557c20914ea0ab306a8ed41f6e39fc801cf"),
    },
    Pin {
        role: "describe",
        file: "Describe.txt",
        bytes: 213306,
        sha256: Cow::Borrowed("c933888c405a476cbb303494857fac657563112dd7f2f319f90064c7c6f22450"),
    },
    Pin {
        role: "terrain1",
        file: "Terrain1.bmp",
        bytes: 614454,
        sha256: Cow::Borrowed("5b591d8811069228e0f74b073ef42fd3f1db1d15357545016a9bda3a7b521734"),
    },
    Pin {
        role: "terrain2",
        file: "Terrain2.bmp",
        bytes: 614454,
        sha256: Cow::Borrowed("3f65949a078451711512b380ac0a80b07cfc65acf2d8e9da30e707e61e540730"),
    },
    Pin {
        role: "units",
        file: "Units.bmp",
        bytes: 750134,
        sha256: Cow::Borrowed("7bf8a38fddd4c2023e2bd57f96d5d8c9a4af6da7ba9da859bdde473ac8248a75"),
    },
    Pin {
        role: "cities",
        file: "Cities.bmp",
        bytes: 614454,
        sha256: Cow::Borrowed("85956af43a4db2b8621b703f4e92f2998eb0e017e7a91bb4fb13d53993ef2ac4"),
    },
    Pin {
        role: "city",
        file: "City.bmp",
        bytes: 960054,
        sha256: Cow::Borrowed("c35d356f9db7a622debd5675c993e7e57ab1f14ba69eb3144a1f12c6b44a922f"),
    },
    Pin {
        role: "icons",
        file: "Icons.bmp",
        bytes: 614454,
        sha256: Cow::Borrowed("6ef962ea8e66a74fda9165bbced8a6badf89849fc377ef32f622cb5ed4c8f2d5"),
    },
    Pin {
        role: "people",
        file: "People.bmp",
        bytes: 614454,
        sha256: Cow::Borrowed("c956f81a4f10433764eaf4edf6647a9e71e872ff4d67df22e871fb91106bce77"),
    },
    Pin {
        role: "dialog",
        file: "Dialog.bmp",
        bytes: 666382,
        sha256: Cow::Borrowed("46697809ce8cc308161eef463a02ec906994be7d8fe7aed6ac0117afa6c15b81"),
    },
];

/// Immutable owned buffers and their parsed/decoded resources. The known
/// dependency set is one admission predicate, never a playable capability.
pub struct OriginalCapture {
    selected: String,
    resources: Resources,
    files: ScenarioFiles,
    bytes: BTreeMap<String, Vec<u8>>,
    identity: String,
}

impl OriginalCapture {
    pub const SET: &'static str = "original-reference-dependencies-v1";
    pub const PRESENTATION: &'static str = "terminal-text-no-avi-or-ending-screens-v1";

    pub fn capture(home: &Path, relative: &str) -> Result<Self, String> {
        Self::capture_with_pins(home, relative, &PINS)
    }

    /// Direct Original SAV candidates, sorted by name. Each candidate still
    /// needs its own dependency and save checks before it can be inspected.
    pub(crate) fn list_saves(home: &Path) -> Result<Vec<String>, String> {
        let (_, folder) = original_folder(home)?;
        let mut candidates = BTreeMap::new();
        for entry in fs::read_dir(&folder).map_err(|e| format!("Original directory: {e}"))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let Some(name) = name.to_str().filter(|name| !name.starts_with('.')) else {
                continue;
            };
            if is_save_name(name) {
                candidates
                    .entry(name.to_ascii_lowercase())
                    .and_modify(|candidate| *candidate = None)
                    .or_insert_with(|| {
                        contained(&folder, entry.path())
                            .is_ok_and(|path| path.is_file())
                            .then(|| format!("Original/{name}"))
                    });
            }
        }
        Ok(candidates.into_values().flatten().take(400).collect())
    }

    pub fn resources(&self) -> &Resources {
        &self.resources
    }

    pub fn files(&self) -> &ScenarioFiles {
        &self.files
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Includes "scenario" plus the twelve required roles. Original bytes
    /// stay in memory/private adapter storage, never in public evidence.
    pub fn captured_bytes(&self, role: &str) -> Option<&[u8]> {
        self.bytes.get(role).map(Vec::as_slice)
    }

    /// Detect changed inputs/selection before a future atomic confirmation.
    /// The current capture stays usable and unchanged on every failure.
    pub fn revalidate(&self, home: &Path) -> Result<(), String> {
        self.revalidate_with_pins(home, &PINS)
    }

    fn revalidate_with_pins(&self, home: &Path, pins: &[Pin]) -> Result<(), String> {
        let current = Self::capture_with_pins(home, &self.selected, pins)?;
        if current.identity != self.identity || current.bytes != self.bytes {
            return Err("Original inputs changed since capture; select the save again".into());
        }
        Ok(())
    }

    fn capture_with_pins(home: &Path, relative: &str, pins: &[Pin]) -> Result<Self, String> {
        let selected = relative.replace('\\', "/");
        let (_, filename) = selected
            .split_once('/')
            .filter(|(folder, filename)| {
                folder.eq_ignore_ascii_case("Original") && is_save_name(filename)
            })
            .ok_or("this dependency set requires a SAV directly in Original")?;
        let (root, folder) = original_folder(home)?;
        reject_lua(&folder)?;
        let mut buffers = BTreeMap::new();
        let mut sources = BTreeMap::new();
        let mut text_sources = BTreeMap::new();
        let mut load = |role: &str, file: &str| -> Result<Vec<u8>, String> {
            let path = contained(
                &root,
                child(&folder, file)?
                    .ok_or_else(|| format!("Original/{file}: required dependency missing"))?,
            )?;
            if !path.starts_with(&folder) {
                return Err(format!(
                    "Original/{file}: dependency leaves the selected profile"
                ));
            }
            let data = read(&path).map_err(|e| format!("Original/{file}: {e}"))?;
            let relative = path
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if !relative.is_ascii() || relative.chars().any(|c| c.is_control()) {
                return Err(format!("Original/{file}: unsupported resolved path"));
            }
            let source = Source {
                relative,
                bytes: data.len(),
                fingerprint: fingerprint(&data),
            };
            if ["labels", "describe"].contains(&role) {
                text_sources.insert(role.to_owned(), source);
            } else {
                sources.insert(role.to_owned(), source);
            }
            buffers.insert(role.to_owned(), data.clone());
            Ok(data)
        };
        let snapshot = load("scenario", filename)?;
        for pin in pins {
            let data = load(pin.role, pin.file)?;
            if data.len() != pin.bytes || sha256(&data) != pin.sha256 {
                return Err(format!(
                    "Original/{}: differs from the supported reference dependency set",
                    pin.file
                ));
            }
        }
        let rules = tot_kit::parse_rules(&tot_kit::latin1(&buffers["rules"]));
        let events = tot_kit::parse_events(&tot_kit::latin1(&buffers["events"]))
            .map_err(|e| format!("Original/Events.txt: {e:?}"))?;
        let labels = tot_kit::Labels::parse(&tot_kit::latin1(&buffers["labels"]))
            .ok_or("Original/Labels.txt: labels could not be decoded")?;
        let pedia = tot_kit::Pedia::parse(&tot_kit::latin1(&buffers["describe"]))
            .map_err(|e| format!("Original/Describe.txt: {e}"))?;
        let mut sheets = BTreeMap::new();
        for role in ART_NAMES {
            sheets.insert(
                role.to_owned(),
                decode(&buffers[role]).map_err(|e| format!("Original {role}: {e}"))?,
            );
        }
        let path = sources["scenario"].relative.clone();
        let resources = Resources {
            rules,
            sources,
            text_sources,
            sheets,
            missing: vec![],
            profile: "Original".into(),
            isotrident: None,
            pedia: Some(pedia),
            labels: Some(labels),
        };
        let mut identity = format!(
            "{}dependency-set {}\npresentation {}\n",
            resources.scenario_identity(),
            Self::SET,
            Self::PRESENTATION
        );
        for (role, data) in &buffers {
            identity.push_str(&format!("sha256 {role} {}\n", sha256(data)));
        }
        Ok(Self {
            selected,
            resources,
            files: ScenarioFiles {
                path,
                bytes: snapshot,
                events: Some(events),
                events_unreadable: false,
                lua: false,
            },
            bytes: buffers,
            identity,
        })
    }
}

fn is_save_name(filename: &str) -> bool {
    filename.is_ascii()
        && !filename
            .chars()
            .any(|c| c.is_control() || matches!(c, ':' | '/' | '\\'))
        && filename.to_ascii_lowercase().ends_with(".sav")
}

fn original_folder(home: &Path) -> Result<(PathBuf, PathBuf), String> {
    let root = home
        .canonicalize()
        .map_err(|e| format!("installation directory: {e}"))?;
    let folder = contained(
        &root,
        child(&root, "Original")?.ok_or("Original directory missing")?,
    )?;
    Ok((root, folder))
}

fn reject_lua(folder: &Path) -> Result<(), String> {
    for entry in fs::read_dir(folder).map_err(|e| format!("Original directory: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name == "lua" || name.ends_with(".lua") || name.ends_with(".luac") {
            return Err("Original contains Lua inputs outside this dependency set".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
