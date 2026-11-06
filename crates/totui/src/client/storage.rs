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
