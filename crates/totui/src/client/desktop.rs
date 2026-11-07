//! Native SDL presentation: direct calls to the shared controller; no HTTP.
use super::{Client, Draw, Input, HEIGHT, WIDTH};
use sdl2::{
    event::{Event, WindowEvent},
    keyboard::{Keycode, Mod},
    pixels::{Color, PixelFormatEnum},
    rect::Rect,
    render::BlendMode,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
fn color(s: &str) -> Color {
    let n = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
    Color::RGB((n >> 16) as u8, (n >> 8) as u8, n as u8)
}
fn key(k: Keycode, shift: bool, ctrl: bool) -> Option<String> {
    if ctrl {
        if shift {
            return None;
        }
        return match k {
            Keycode::S => Some("Ctrl+s".into()),
            Keycode::L => Some("Ctrl+l".into()),
            Keycode::N => Some("Ctrl+n".into()),
            _ => None,
        };
    }
    if shift && k == Keycode::Z {
        return Some("Shift+z".into());
    }
    let name = match k {
        Keycode::Return => "Enter",
        Keycode::Escape => "Escape",
        Keycode::Tab => {
            if shift {
                "ShiftTab"
            } else {
                "Tab"
            }
        }
        Keycode::Backspace => "Backspace",
        Keycode::Home => "Home",
        Keycode::PageDown => "PageDown",
        Keycode::PageUp => "PageUp",
        Keycode::Left => "ArrowLeft",
        Keycode::Right => "ArrowRight",
        Keycode::Up => "ArrowUp",
        Keycode::Down => "ArrowDown",
        Keycode::Space => " ",
        Keycode::Plus | Keycode::Equals => "+",
        Keycode::Minus => "-",
        Keycode::A => "a",
        Keycode::B => "b",
        Keycode::V => "v",
        Keycode::Z => "z",
        Keycode::X => "x",
        Keycode::F => "f",
        Keycode::R => "r",
        Keycode::C => "c",
        Keycode::O => "o",
        Keycode::M => "m",
        Keycode::E => "e",
        Keycode::W => "w",
        Keycode::S => "s",
        Keycode::L => "l",
        Keycode::Num1 | Keycode::Kp1 => "1",
        Keycode::Num2 | Keycode::Kp2 => "2",
        Keycode::Num3 | Keycode::Kp3 => "3",
        Keycode::Num4 | Keycode::Kp4 => "4",
        Keycode::Num6 | Keycode::Kp6 => "6",
        Keycode::Num7 | Keycode::Kp7 => "7",
        Keycode::Num8 | Keycode::Kp8 => "8",
        Keycode::Num9 | Keycode::Kp9 => "9",
        _ => return None,
    };
    if shift && name.len() == 1 && name.as_bytes()[0].is_ascii_alphabetic() {
        return None;
    }
    Some(name.into())
}
fn font_path() -> PathBuf {
    if let Some(p) = std::env::var_os("TOTENGINE_FONT") {
        return p.into();
    }
    #[cfg(windows)]
    {
        PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:/Windows".into()))
            .join("Fonts/arial.ttf")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf")
    }
}
pub fn run(mut client: Client) -> Result<(), String> {
    let font = fontdue::Font::from_bytes(
        std::fs::read(font_path()).map_err(|e| {
            format!("System font missing: {e}. Set TOTENGINE_FONT to an owned TTF file.")
        })?,
        fontdue::FontSettings::default(),
    )
    .map_err(str::to_owned)?;
    let animation_start = Instant::now();
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window("ToTEngine — playable desktop", WIDTH, HEIGHT)
        .resizable()
        .allow_highdpi()
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window
        .into_canvas()
        .software()
        .build()
        .map_err(|e| e.to_string())?;
    canvas
        .set_logical_size(WIDTH, HEIGHT)
        .map_err(|e| e.to_string())?;
    let tc = canvas.texture_creator();
    let mut sprites = BTreeMap::new();
    let mut text_cache = BTreeMap::new();
    let mut assets = String::new();
    let mut pump = sdl.event_pump()?;
    let mut id = 0u64;
    let mut painted_revision = None;
    let mut editor = super::editor::Editor::default();
    'running: loop {
        let before = client.frame();
        if let Some(entry) = &before.entry {
            editor.sync(entry);
        }
        let batch_revision = before.revision;
        let modal_before = client.modal_id;
        let in_modal = client.modal.is_some();
        let original_field = before.entry.as_ref().map(|e| e.id.clone());
        if before.entry.is_some() {
            video.text_input().start()
        } else {
            video.text_input().stop()
        }
        for event in pump.poll_iter() {
            if matches!(event, Event::KeyDown { .. }) {
                if let Some(path) = std::env::var_os("TOTENGINE_CLIENT_TRACE") {
                    use std::io::Write;
                    if let Ok(mut log) = std::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(PathBuf::from(path).with_extension("keys.log"))
                    {
                        let _ = writeln!(
                            log,
                            "{event:?}; field={original_field:?}; batch={batch_revision}"
                        );
                    }
                }
            }
            let mut revision = batch_revision;
            let input = match event {
                Event::Quit { .. }
                | Event::Window {
                    win_event: WindowEvent::Close,
                    ..
                } => break 'running,
                Event::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } => Some(Input::Focus { focused: false }),
                Event::Window {
                    win_event: WindowEvent::FocusGained,
                    ..
                } => Some(Input::Focus { focused: true }),
                Event::MouseButtonDown {
                    x,
                    y,
                    mouse_btn: sdl2::mouse::MouseButton::Left,
                    ..
                } => Some(Input::Click {
                    x: f64::from(x),
                    y: f64::from(y),
                }),
                Event::TextInput { text, .. } => {
                    let entry = client
                        .frame()
                        .entry
                        .filter(|e| Some(&e.id) == original_field.as_ref());
                    entry.map(|mut e| {
                        editor.insert(&text);
                        e.value = editor.value.clone();
                        revision = client.frame().revision;
                        Input::Edit {
                            field: e.id,
                            value: e.value,
                        }
                    })
                }
                Event::KeyDown {
                    keycode: Some(k),
                    keymod,
                    repeat,
                    ..
                } => {
                    let entry = client
                        .frame()
                        .entry
                        .filter(|e| Some(&e.id) == original_field.as_ref());
                    if let Some(mut entry) = entry {
                        let ctrl = keymod.intersects(
                            Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LGUIMOD | Mod::RGUIMOD,
                        );
                        let shift = keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
                        let changed = if ctrl && k == Keycode::A {
                            editor.key(super::editor::Key::All, false);
                            false
                        } else if ctrl && k == Keycode::V && client.focused {
                            if let Ok(text) = video.clipboard().clipboard_text() {
                                editor.insert(&text);
                                true
                            } else {
                                false
                            }
                        } else if ctrl && (k == Keycode::C || k == Keycode::X) && client.focused {
                            let _ = video.clipboard().set_clipboard_text(&editor.selected());
                            if k == Keycode::X {
                                editor.insert("");
                                true
                            } else {
                                false
                            }
                        } else {
                            let edit_key = match k {
                                Keycode::Left => Some(super::editor::Key::Left),
                                Keycode::Right => Some(super::editor::Key::Right),
                                Keycode::Home => Some(super::editor::Key::Home),
                                Keycode::End => Some(super::editor::Key::End),
                                Keycode::Backspace => Some(super::editor::Key::Backspace),
                                Keycode::Delete => Some(super::editor::Key::Delete),
                                _ => None,
                            };
                            edit_key.is_some_and(|k| editor.key(k, shift))
                        };
                        if changed {
                            entry.value = editor.value.clone();
                            revision = client.frame().revision;
                            Some(Input::Edit {
                                field: entry.id,
                                value: entry.value,
                            })
                        } else if !repeat {
                            key(k, shift, ctrl)
                                .filter(|s| {
                                    matches!(s.as_str(), "Enter" | "Escape" | "Tab" | "ShiftTab")
                                })
                                .map(|key| Input::Key { key })
                        } else {
                            None
                        }
                    } else if !repeat && !keymod.intersects(Mod::LALTMOD | Mod::RALTMOD) {
                        key(
                            k,
                            keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
                            keymod.intersects(
                                Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LGUIMOD | Mod::RGUIMOD,
                            ),
                        )
                        .map(|key| Input::Key { key })
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(input) = input {
                // Focus/tab edits and input inside the same dialog do not
                // retarget a unit. Map orders retain their received revision.
                if matches!(&input, Input::Focus { .. })
                    || matches!(&input,Input::Key{key} if key=="Tab"||key=="ShiftTab")
                {
                    revision = client.frame().revision;
                }
                if in_modal
                    && client.modal.is_some()
                    && client.modal_id == modal_before
                    && matches!(&input, Input::Key { .. } | Input::Edit { .. })
                {
                    revision = client.frame().revision;
                }
                id += 1;
                let _ = client.input(revision, id, input);
            }
        }
        if client.frame().phase == "Opponents" && client.frame().advancing {
            id += 1;
            let _ = client.input(client.frame().revision, id, Input::Tick);
        }
        if client.closed {
            break;
        }
        let frame = client.frame();
        if frame.assets != assets {
            assets = frame.assets.clone();
            sprites.clear();
        }
        canvas.set_draw_color(Color::RGB(22, 28, 21));
        canvas.clear();
        let paint_trace = std::env::var_os("TOTENGINE_CLIENT_TRACE")
            .filter(|_| painted_revision != Some(frame.revision))
            .and_then(|p| {
                serde_json::to_vec(&frame)
                    .ok()
                    .map(|data| (PathBuf::from(p).with_extension("painted.json"), data))
            });
        let revision = frame.revision;
        for draw in frame.draws {
            if matches!(&draw, Draw::UnitSprite { blink: true, .. })
                && !(animation_start.elapsed().as_millis() / 500).is_multiple_of(2)
            {
                continue;
            }
            match draw {
                Draw::Rect {
                    x,
                    y,
                    w,
                    h,
                    color: s,
                } => {
                    canvas.set_draw_color(color(&s));
                    canvas.fill_rect(Rect::new(x as i32, y as i32, w as u32, h as u32))?;
                }
                Draw::Cursor {
                    x,
                    y,
                    w,
                    h,
                    color: s,
                    blink,
                } => {
                    if !blink || (animation_start.elapsed().as_millis() / 500).is_multiple_of(2) {
                        canvas.set_draw_color(color(&s));
                        let points = [
                            (x - w / 2., y),
                            (x, y - h / 2.),
                            (x + w / 2., y),
                            (x, y + h / 2.),
                            (x - w / 2., y),
                        ];
                        for pair in points.windows(2) {
                            canvas.draw_line(
                                (pair[0].0 as i32, pair[0].1 as i32),
                                (pair[1].0 as i32, pair[1].1 as i32),
                            )?;
                        }
                    }
                }
                Draw::Line {
                    x,
                    y,
                    x2,
                    y2,
                    color: s,
                    width,
                } => {
                    canvas.set_draw_color(color(&s));
                    for n in 0..(width.ceil() as i32).max(1) {
                        canvas.draw_line((x as i32, y as i32 + n), (x2 as i32, y2 as i32 + n))?;
                    }
                }
                Draw::Tile { x, y, w, h, sprite } => {
                    if !sprites.contains_key(&sprite) {
                        if let Some(s) = client.atlas().and_then(|a| a.sprites.get(&sprite)) {
                            let mut t = tc
                                .create_texture_static(PixelFormatEnum::RGBA32, s.w, s.h)
                                .map_err(|e| e.to_string())?;
                            t.set_blend_mode(BlendMode::Blend);
                            t.update(None, &s.rgba, s.w as usize * 4)
                                .map_err(|e| e.to_string())?;
                            sprites.insert(sprite.clone(), t);
                        }
                    }
                    if let Some(t) = sprites.get(&sprite) {
                        let q = t.query();
                        let clip = canvas.clip_rect();
                        canvas.set_clip_rect(Rect::new(x as i32, y as i32, w as u32, h as u32));
                        let mut yy = 0;
                        while yy < h as u32 {
                            let mut xx = 0;
                            while xx < w as u32 {
                                canvas.copy(
                                    t,
                                    None,
                                    Rect::new(
                                        x as i32 + xx as i32,
                                        y as i32 + yy as i32,
                                        q.width,
                                        q.height,
                                    ),
                                )?;
                                xx += q.width;
                            }
                            yy += q.height;
                        }
                        canvas.set_clip_rect(clip);
                    }
                }
                Draw::Sprite { x, y, w, h, sprite }
                | Draw::UnitSprite {
                    x, y, w, h, sprite, ..
                } => {
                    if !sprites.contains_key(&sprite) {
                        if let Some(s) = client.atlas().and_then(|a| a.sprites.get(&sprite)) {
                            let mut t = tc
                                .create_texture_static(PixelFormatEnum::RGBA32, s.w, s.h)
                                .map_err(|e| e.to_string())?;
                            t.set_blend_mode(BlendMode::Blend);
                            t.update(None, &s.rgba, s.w as usize * 4)
                                .map_err(|e| e.to_string())?;
                            sprites.insert(sprite.clone(), t);
                        }
                    }
                    if let Some(t) = sprites.get(&sprite) {
                        canvas.copy(t, None, Rect::new(x as i32, y as i32, w as u32, h as u32))?;
                    }
                }
                Draw::Text {
                    x,
                    y,
                    text,
                    color: s,
                    size,
                } => {
                    let cache_key = (text.clone(), size, s.clone());
                    if !text_cache.contains_key(&cache_key) {
                        let layout = fontdue::layout::TextStyle::new(&text, size as f32, 0);
                        let mut glyphs = fontdue::layout::Layout::new(
                            fontdue::layout::CoordinateSystem::PositiveYDown,
                        );
                        glyphs.reset(&fontdue::layout::LayoutSettings::default());
                        glyphs.append(&[&font], &layout);
                        let w = glyphs
                            .glyphs()
                            .iter()
                            .map(|g| g.x.ceil() as usize + g.width)
                            .max()
                            .unwrap_or(1)
                            .max(1);
                        let h = glyphs.height().ceil().max(size as f32) as usize;
                        let mut rgba = vec![0; w * h * 4];
                        let c = color(&s);
                        for g in glyphs.glyphs() {
                            let (m, bitmap) = font.rasterize_config(g.key);
                            for yy in 0..m.height {
                                for xx in 0..m.width {
                                    let px = g.x as usize + xx;
                                    let py = g.y as usize + yy;
                                    if px < w && py < h {
                                        let at = (py * w + px) * 4;
                                        rgba[at..at + 4].copy_from_slice(&[
                                            c.r,
                                            c.g,
                                            c.b,
                                            bitmap[yy * m.width + xx],
                                        ]);
                                    }
                                }
                            }
                        }
                        let mut t = tc
                            .create_texture_static(PixelFormatEnum::RGBA32, w as u32, h as u32)
                            .map_err(|e| e.to_string())?;
                        t.set_blend_mode(BlendMode::Blend);
                        t.update(None, &rgba, w * 4).map_err(|e| e.to_string())?;
                        if text_cache.len() > 512 {
                            text_cache.clear();
                        }
                        text_cache.insert(cache_key.clone(), t);
                    }
                    if let Some(t) = text_cache.get(&cache_key) {
                        let q = t.query();
                        canvas.copy(t, None, Rect::new(x as i32, y as i32, q.width, q.height))?;
                    }
                }
            }
        }
        if let Some(entry) = client.frame().entry {
            if entry.id == editor.field && client.focused {
                let offset = editor.value[..editor.cursor]
                    .chars()
                    .map(|c| font.metrics(c, 18.).advance_width)
                    .sum::<f32>()
                    .min((entry.w - 16.) as f32);
                canvas.set_draw_color(Color::RGB(250, 235, 170));
                let x = entry.x as i32 + 7 + offset as i32;
                canvas.draw_line(
                    (x, entry.y as i32 + 6),
                    (x, entry.y as i32 + entry.h as i32 - 6),
                )?;
            }
        }
        canvas.present();
        if let Some((path, data)) = paint_trace {
            let _ = std::fs::write(path, data);
        }
        painted_revision = Some(revision);
        std::thread::sleep(Duration::from_millis(16));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_keys_and_modifiers_are_preserved() {
        for (k, want) in [
            (Keycode::A, "a"),
            (Keycode::B, "b"),
            (Keycode::V, "v"),
            (Keycode::Kp8, "8"),
        ] {
            assert_eq!(key(k, false, false).as_deref(), Some(want));
        }
        assert_eq!(key(Keycode::N, false, true).as_deref(), Some("Ctrl+n"));
        assert_eq!(key(Keycode::S, false, true).as_deref(), Some("Ctrl+s"));
        assert_eq!(key(Keycode::L, false, true).as_deref(), Some("Ctrl+l"));
        assert_eq!(key(Keycode::Z, true, false).as_deref(), Some("Shift+z"));
        assert_eq!(key(Keycode::B, false, true), None);
        assert_eq!(key(Keycode::B, true, false), None);
        assert_eq!(key(Keycode::S, true, true), None);
    }
}
