//! Shared owned-install resolution and BMP/GIF decoding for both clients.
//!
//! Selection is our explicit policy, not a claim about native scenario lookup:
//! profile BMP, profile GIF, root BMP, root GIF; Rules from profile then root.
use std::collections::{BTreeMap, VecDeque};
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use image::{ImageFormat, ImageReader, Limits, RgbaImage};

pub const ART_NAMES: [&str; 8] = [
    "terrain1", "terrain2", "units", "cities", "city", "icons", "people", "dialog",
];
const MAX_FILE: u64 = 16 * 1024 * 1024;

fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

fn child(directory: &Path, name: &str) -> Result<Option<PathBuf>, String> {
    let mut found = None;
    for entry in
        fs::read_dir(directory).map_err(|e| format!("cannot read resource directory: {e}"))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(name)
        {
            if found.is_some() {
                return Err(format!("ambiguous case-insensitive resource: {name}"));
            }
            found = Some(entry.path());
        }
    }
    Ok(found)
}

fn contained(root: &Path, path: PathBuf) -> Result<PathBuf, String> {
    let resolved = path
        .canonicalize()
        .map_err(|e| format!("cannot resolve resource: {e}"))?;
    if !resolved.starts_with(root) {
        return Err("resource symlink escapes the installation".into());
    }
    Ok(resolved)
}

pub(crate) fn client_rules_path(root: &Path) -> Result<Option<PathBuf>, String> {
    if let Some(profile) = child(root, "Original")? {
        let profile = contained(root, profile)?;
        if let Some(rules) = child(&profile, "Rules.txt")? {
            return contained(root, rules).map(Some);
        }
    }
    child(root, "Rules.txt")?
        .map(|p| contained(root, p))
        .transpose()
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    if !path.is_file() {
        return Err("resource is not a regular file".into());
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE {
        return Err("resource exceeds 16 MiB".into());
    }
    Ok(bytes)
}

/// Source identities are relative to the install, with content fingerprints.
/// No absolute install path is included in the identity given to checkpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub relative: String,
    pub bytes: usize,
    pub fingerprint: u64,
}

pub struct Resources {
    pub rules: tot_kit::Rules,
    pub sources: BTreeMap<String, Source>,
    pub sheets: BTreeMap<String, RgbaImage>,
    pub missing: Vec<String>,
    pub profile: String,
    pub isotrident: Option<BTreeMap<String, FreecivSprite>>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreecivSprite {
    pub sheet: String,
    pub rect: [u32; 4],
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FreecivManifest {
    version: u32,
    upstream: String,
    license: String,
    source: String,
    sprites: BTreeMap<String, FreecivSprite>,
}

impl Resources {
    pub fn load(home: &Path, profile: &str) -> Result<Self, String> {
        let root = home
            .canonicalize()
            .map_err(|e| format!("installation directory: {e}"))?;
        let profile = profile.replace('\\', "/");
        if profile
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.contains(':') || !p.is_ascii())
        {
            return Err(
                "profile must be a relative ASCII directory without . or .. components".into(),
            );
        }
        let mut folder = root.clone();
        for part in profile.split('/') {
            folder = contained(
                &root,
                child(&folder, part)?
                    .ok_or_else(|| format!("profile directory missing: {profile}"))?,
            )?;
            if !folder.is_dir() {
                return Err("profile is not a directory".into());
            }
        }
        let select = |stem: &str, extensions: &[&str]| -> Result<Option<PathBuf>, String> {
            for directory in [&folder, &root] {
                for ext in extensions {
                    if let Some(path) = child(directory, &format!("{stem}{ext}"))? {
                        return contained(&root, path).map(Some);
                    }
                }
            }
            Ok(None)
        };
        let mut sources = BTreeMap::new();
        let mut load = |key: &str, path: &Path| -> Result<Vec<u8>, String> {
            let bytes = read(path).map_err(|e| format!("{key}: {e}"))?;
            sources.insert(
                key.to_owned(),
                Source {
                    relative: path
                        .strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    bytes: bytes.len(),
                    fingerprint: fingerprint(&bytes),
                },
            );
            Ok(bytes)
        };
        let rules_path = select("rules", &[".txt"])?
            .ok_or("Rules.txt missing in profile and installation root")?;
        let rules_bytes = load("rules", &rules_path)?;
        // Original Rules text is byte-oriented; keep the existing decoder's
        // Latin-1-compatible policy rather than silently replacing characters.
        let rules = tot_kit::parse_rules(&tot_kit::latin1(&rules_bytes));
        let mut sheets = BTreeMap::new();
        let mut missing = Vec::new();
        for key in ART_NAMES {
            if let Some(path) = select(key, &[".bmp", ".gif"])? {
                let data = load(key, &path)?;
                let sheet = decode(&data).map_err(|e| format!("{key}: {e}"))?;
                sheets.insert(key.to_owned(), sheet);
            } else {
                missing.push(key.to_owned());
            }
        }
        for key in ["terrain1", "units", "cities"] {
            if !sheets.contains_key(key) {
                return Err(format!("required artwork missing: {key}"));
            }
        }
        Ok(Self {
            rules,
            sources,
            sheets,
            missing,
            profile,
            isotrident: None,
        })
    }

    /// A separately installed redistributable art pack, with explicitly supplied
    /// gameplay rules. Freeciv rules and game code are never loaded here.
    pub fn load_isotrident(pack: &Path, rules_path: Option<&Path>) -> Result<Self, String> {
        let root = pack
            .canonicalize()
            .map_err(|e| format!("Isotrident pack: {e}"))?;
        let bytes = match rules_path {
            Some(path) => read(path)?,
            None => include_bytes!("../data/open-demo-rules.txt").to_vec(),
        };
        let rules = tot_kit::parse_rules(&tot_kit::latin1(&bytes));
        let mut sources = BTreeMap::from([(
            "rules".into(),
            Source {
                relative: "rules.txt".into(),
                bytes: bytes.len(),
                fingerprint: fingerprint(&bytes),
            },
        )]);
        let data = read(&contained(&root, root.join("atlas.json"))?)?;
        let manifest: FreecivManifest =
            serde_json::from_slice(&data).map_err(|e| format!("Isotrident manifest: {e}"))?;
        if manifest.version != 1
            || manifest.upstream != "Freeciv 3.2.7"
            || manifest.license != "GPL-2.0-or-later"
            || manifest.source.is_empty()
            || manifest.sprites.len() > 4096
        {
            return Err("unsupported Isotrident pack".into());
        }
        sources.insert(
            "atlas".into(),
            Source {
                relative: "atlas.json".into(),
                bytes: data.len(),
                fingerprint: fingerprint(&data),
            },
        );
        let mut sheets = BTreeMap::new();
        for sprite in manifest.sprites.values() {
            let path = Path::new(&sprite.sheet);
            if path.extension().and_then(|s| s.to_str()) != Some("png")
                || !path
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_)))
            {
                return Err("invalid Isotrident sheet path".into());
            }
            if !sheets.contains_key(&sprite.sheet) {
                if sheets.len() >= 16 {
                    return Err("too many Isotrident sheets".into());
                }
                let data = read(&contained(&root, root.join(path))?)?;
                sources.insert(
                    sprite.sheet.clone(),
                    Source {
                        relative: sprite.sheet.clone(),
                        bytes: data.len(),
                        fingerprint: fingerprint(&data),
                    },
                );
                sheets.insert(
                    sprite.sheet.clone(),
                    decode_format(&data, ImageFormat::Png)?,
                );
            }
            let [x, y, w, h] = sprite.rect;
            let sheet = &sheets[&sprite.sheet];
            if w == 0
                || h == 0
                || w > 256
                || h > 256
                || x.checked_add(w).is_none_or(|n| n > sheet.width())
                || y.checked_add(h).is_none_or(|n| n > sheet.height())
            {
                return Err("Isotrident crop outside sheet".into());
            }
        }
        Ok(Self {
            rules,
            sources,
            sheets,
            missing: vec![],
            profile: if rules_path.is_some() {
                "ToT rules + Isotrident"
            } else {
                "Open demo + Isotrident"
            }
            .into(),
            isotrident: Some(manifest.sprites),
        })
    }

    /// Stable versioned Rules/art identity for human checkpoints. This is an
    /// accidental-change detector, not a cryptographic authenticity signature.
    pub fn identity(&self) -> String {
        let mut text = format!(
            "totengine-resources 1\nprofile {}\n",
            self.profile.to_ascii_lowercase()
        );
        for (key, source) in &self.sources {
            text.push_str(&format!(
                "{key} {} {} {:016x}\n",
                source.relative.to_ascii_lowercase(),
                source.bytes,
                source.fingerprint
            ));
        }
        text
    }

    pub fn crop(&self, sheet: &str, rect: Rect, mask: Mask) -> Result<RgbaImage, String> {
        crop(
            self.sheets
                .get(sheet)
                .ok_or_else(|| format!("artwork unavailable: {sheet}"))?,
            rect,
            mask,
        )
    }
}

/// BMP or first GIF frame, bounded before pixel allocation. Other formats are
/// rejected even if renamed to match the resource allowlist.
pub fn decode(bytes: &[u8]) -> Result<RgbaImage, String> {
    if bytes.len() as u64 > MAX_FILE {
        return Err("image exceeds 16 MiB".into());
    }
    let format = image::guess_format(bytes).map_err(|e| e.to_string())?;
    if !matches!(format, ImageFormat::Bmp | ImageFormat::Gif) {
        return Err("only BMP/GIF sheets are supported".into());
    }
    decode_format(bytes, format)
}

fn decode_format(bytes: &[u8], format: ImageFormat) -> Result<RgbaImage, String> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map(|img| img.into_rgba8())
        .map_err(|e| format!("unsupported or damaged artwork: {e}"))
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Mask {
    pub diamond: bool,
    pub unit: bool,
    pub matte: bool,
    pub pink: bool,
}

/// Shared measured prototype mask, retaining isolated interior gray details.
/// Owner recoloring and a complete native atlas are separate implementation.
pub fn crop(sheet: &RgbaImage, r: Rect, mask: Mask) -> Result<RgbaImage, String> {
    if r.w == 0
        || r.h == 0
        || r.x.checked_add(r.w).is_none_or(|v| v > sheet.width())
        || r.y.checked_add(r.h).is_none_or(|v| v > sheet.height())
    {
        return Err("sprite rectangle lies outside its sheet".into());
    }
    let mut image = image::imageops::crop_imm(sheet, r.x, r.y, r.w, r.h).to_image();
    let pink = image.get_pixel(0, 0).0;
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let rgb = [pixel[0], pixel[1], pixel[2]];
        let outside = mask.diamond
            && ((2.0 * f64::from(x) + 1.0 - f64::from(r.w)) / f64::from(r.w)).abs()
                + ((2.0 * f64::from(y) + 1.0 - f64::from(r.h)) / f64::from(r.h)).abs()
                > 1.0;
        if outside || rgb == [255, 0, 255] || (mask.unit && rgb == [132, 82, 132]) {
            pixel[3] = 0;
        }
    }
    if mask.matte {
        let mut seen = vec![false; (r.w * r.h) as usize];
        let mut queue = VecDeque::new();
        for x in 0..r.w {
            queue.push_back((x, 0));
            queue.push_back((x, r.h - 1));
        }
        for y in 0..r.h {
            queue.push_back((0, y));
            queue.push_back((r.w - 1, y));
        }
        while let Some((x, y)) = queue.pop_front() {
            let idx = (y * r.w + x) as usize;
            if seen[idx] {
                continue;
            }
            seen[idx] = true;
            let p = image.get_pixel_mut(x, y);
            if p[3] != 0 && p.0[..3] != [132, 132, 132] && !(mask.pink && p.0[..3] == pink[..3]) {
                continue;
            }
            p[3] = 0;
            if x > 0 {
                queue.push_back((x - 1, y));
            }
            if y > 0 {
                queue.push_back((x, y - 1));
            }
            if x + 1 < r.w {
                queue.push_back((x + 1, y));
            }
            if y + 1 < r.h {
                queue.push_back((x, y + 1));
            }
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests;
