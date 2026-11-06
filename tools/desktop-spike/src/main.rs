//! Isolated SDL2 feasibility window, not a production client or game fork.
use std::path::{Path, PathBuf};
use std::time::Duration;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use totui::resources::{Mask, Rect as Crop, Resources};

const TITLE: &str = "ToTEngine — desktop feasibility (#126)";
struct Sprite {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}
struct Model {
    name: String,
    edit: Option<String>,
    focus: usize,
    focused: bool,
    accepted: usize,
    cancelled: usize,
    ignored: usize,
    resized: usize,
    lost: usize,
    gained: usize,
    quit: bool,
}
impl Model {
    fn event(&mut self, event: &Event) {
        match event {
            Event::Quit { .. }
            | Event::Window {
                win_event: WindowEvent::Close,
                ..
            } => self.quit = true,
            Event::Window {
                win_event: WindowEvent::FocusLost,
                ..
            } => {
                self.focused = false;
                self.lost += 1;
            }
            Event::Window {
                win_event: WindowEvent::FocusGained,
                ..
            } => {
                self.focused = true;
                self.gained += 1;
            }
            Event::Window {
                win_event: WindowEvent::SizeChanged(..) | WindowEvent::Resized(..),
                ..
            } => self.resized += 1,
            Event::TextInput { text, .. } => {
                if !self.focused {
                    self.ignored += 1;
                    return;
                }
                if let Some(edit) = &mut self.edit {
                    if self.focus == 0
                        && edit.len() + text.len() <= 63
                        && !text.chars().any(char::is_control)
                    {
                        edit.push_str(text);
                    }
                }
            }
            Event::KeyDown {
                keycode: Some(key),
                keymod,
                repeat: false,
                ..
            } if self.focused => match *key {
                Keycode::N if self.edit.is_none() => {
                    self.edit = Some(String::new());
                    self.focus = 0;
                }
                Keycode::Escape if self.edit.is_some() => {
                    self.edit = None;
                    self.cancelled += 1;
                }
                Keycode::Escape => self.quit = true,
                Keycode::Tab if self.edit.is_some() => {
                    self.focus = (self.focus
                        + if keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) {
                            2
                        } else {
                            1
                        })
                        % 3
                }
                Keycode::Backspace if self.focus == 0 => {
                    if let Some(edit) = &mut self.edit {
                        edit.pop();
                    }
                }
                Keycode::Return if self.edit.is_some() => self.accept(),
                _ => (),
            },
            Event::MouseButtonDown { x, y, .. } if self.focused && self.edit.is_some() => {
                if (210..750).contains(x) && (260..310).contains(y) {
                    self.focus = 0;
                }
                if (210..420).contains(x) && (350..400).contains(y) {
                    self.focus = 1;
                    self.accept();
                }
                if (540..750).contains(x) && (350..400).contains(y) {
                    self.focus = 2;
                    self.accept();
                }
            }
            _ => (),
        }
    }
    fn accept(&mut self) {
        if self.focus == 2 {
            self.edit = None;
            self.cancelled += 1;
        } else if let Some(name) = self
            .edit
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            self.name = name.to_owned();
            self.edit = None;
            self.accepted += 1;
        }
    }
}
fn sprite(sheets: Option<&Resources>, key: &str, r: Crop, mask: Mask) -> Result<Sprite, String> {
    if let Some(sheets) = sheets {
        let img = sheets.crop(key, r, mask)?;
        Ok(Sprite {
            w: img.width(),
            h: img.height(),
            rgba: img.into_raw(),
        })
    } else {
        let mut rgba = vec![0; (r.w * r.h * 4) as usize];
        for y in 0..r.h {
            for x in 0..r.w {
                let i = ((y * r.w + x) * 4) as usize;
                let inside = if key == "terrain1" {
                    (x as f32 / r.w as f32 - 0.5).abs() + (y as f32 / r.h as f32 - 0.5).abs() < 0.5
                } else {
                    x > r.w / 4 && x < r.w * 3 / 4 && y > r.h / 4
                };
                rgba[i..i + 4].copy_from_slice(&[
                    40,
                    if key == "terrain1" { 180 } else { 90 },
                    80,
                    if inside { 255 } else { 0 },
                ]);
            }
        }
        Ok(Sprite {
            w: r.w,
            h: r.h,
            rgba,
        })
    }
}
fn blit(
    canvas: &mut Canvas<Window>,
    tc: &TextureCreator<WindowContext>,
    sprite: &Sprite,
    rect: Rect,
) -> Result<(), String> {
    let mut texture = tc
        .create_texture_static(PixelFormatEnum::RGBA32, sprite.w, sprite.h)
        .map_err(|e| e.to_string())?;
    texture.set_blend_mode(BlendMode::Blend);
    texture
        .update(None, &sprite.rgba, sprite.w as usize * 4)
        .map_err(|e| e.to_string())?;
    canvas.copy(&texture, None, rect)
}
fn text(
    canvas: &mut Canvas<Window>,
    tc: &TextureCreator<WindowContext>,
    font: &fontdue::Font,
    label: &str,
    mut x: i32,
    y: i32,
) -> Result<(), String> {
    for ch in label.chars() {
        let (metrics, bitmap) = font.rasterize(ch, 22.0);
        if metrics.width > 0 && metrics.height > 0 {
            let mut rgba = Vec::with_capacity(bitmap.len() * 4);
            for alpha in bitmap {
                rgba.extend_from_slice(&[240, 237, 215, alpha]);
            }
            let sprite = Sprite {
                w: metrics.width as u32,
                h: metrics.height as u32,
                rgba,
            };
            blit(
                canvas,
                tc,
                &sprite,
                Rect::new(
                    x + metrics.xmin,
                    y + 26 - metrics.ymin - metrics.height as i32,
                    sprite.w,
                    sprite.h,
                ),
            )?;
        }
        x += metrics.advance_width.ceil() as i32;
    }
    Ok(())
}
fn font_path() -> Option<PathBuf> {
    std::env::var_os("TOTENGINE_FONT")
        .map(PathBuf::from)
        .or_else(|| {
            #[cfg(windows)]
            {
                std::env::var_os("WINDIR").map(|p| PathBuf::from(p).join("Fonts/arial.ttf"))
            }
            #[cfg(not(windows))]
            {
                Some(PathBuf::from(
                    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                ))
            }
        })
}
fn key(window_id: u32, code: Keycode) -> Event {
    Event::KeyDown {
        timestamp: 0,
        window_id,
        keycode: Some(code),
        scancode: None,
        keymod: Mod::NOMOD,
        repeat: false,
    }
}
fn scripted(frame: u32, window_id: u32) -> Option<Event> {
    let window = |win_event| Event::Window {
        timestamp: 0,
        window_id,
        win_event,
    };
    let input = |text: &str| Event::TextInput {
        timestamp: 0,
        window_id,
        text: text.into(),
    };
    match frame {
        5 | 40 => Some(key(window_id, Keycode::N)),
        10 => Some(input("Athénx")),
        15 => Some(key(window_id, Keycode::Backspace)),
        20 => Some(input("a")),
        25 => Some(key(window_id, Keycode::Tab)),
        30 => Some(key(window_id, Keycode::Return)),
        45 => Some(input("cancel this")),
        50 => Some(key(window_id, Keycode::Escape)),
        55 => Some(window(WindowEvent::FocusLost)),
        60 => Some(input("ignored")),
        65 => Some(window(WindowEvent::FocusGained)),
        85 => Some(Event::Quit { timestamp: 0 }),
        _ => None,
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let report = if matches!(
        args.first().map(String::as_str),
        Some("--self-test" | "--external-test")
    ) {
        Some(PathBuf::from(
            args.get(1).ok_or("--self-test needs a report path")?,
        ))
    } else {
        None
    };
    if !args.is_empty() && report.is_none() {
        return Err("usage: tot-desktop-spike [--self-test REPORT.json | --external-test REPORT.json]; CIVTOT_HOME and TOTENGINE_FONT are optional environment variables".into());
    }
    let scripted_input = args.first().map(String::as_str) == Some("--self-test");
    let resources = std::env::var_os("CIVTOT_HOME")
        .map(|p| Resources::load(Path::new(&p), "Original"))
        .transpose()?;
    let terrain = sprite(
        resources.as_ref(),
        "terrain1",
        Crop {
            x: 1,
            y: 67,
            w: 64,
            h: 32,
        },
        Mask {
            diamond: true,
            ..Default::default()
        },
    )?;
    let unit = sprite(
        resources.as_ref(),
        "units",
        Crop {
            x: 131,
            y: 1,
            w: 64,
            h: 64,
        },
        Mask {
            matte: true,
            unit: true,
            ..Default::default()
        },
    )?;
    let city = sprite(
        resources.as_ref(),
        "cities",
        Crop {
            x: 1,
            y: 88,
            w: 64,
            h: 48,
        },
        Mask {
            matte: true,
            ..Default::default()
        },
    )?;
    let font = font_path().ok_or("set TOTENGINE_FONT to an owned system TTF font")?;
    let font = fontdue::Font::from_bytes(
        std::fs::read(font)
            .map_err(|e| format!("system font unavailable: {e}; set TOTENGINE_FONT"))?,
        fontdue::FontSettings::default(),
    )
    .map_err(str::to_owned)?;
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window(TITLE, 960, 640)
        .position_centered()
        .resizable()
        .allow_highdpi()
        .build()
        .map_err(|e| e.to_string())?;
    let id = window.id();
    let mut canvas = window
        .into_canvas()
        .software()
        .build()
        .map_err(|e| e.to_string())?;
    canvas
        .set_logical_size(960, 640)
        .map_err(|e| e.to_string())?;
    let tc = canvas.texture_creator();
    let mut pump = sdl.event_pump()?;
    let events = sdl.event()?;
    let mut model = Model {
        name: "Alder".into(),
        edit: None,
        focus: 0,
        focused: true,
        accepted: 0,
        cancelled: 0,
        ignored: 0,
        resized: 0,
        lost: 0,
        gained: 0,
        quit: false,
    };
    let mut frame = 0;
    while !model.quit {
        if report.is_some() && scripted_input {
            if frame == 2 {
                canvas
                    .window_mut()
                    .set_size(1200, 800)
                    .map_err(|e| e.to_string())?;
            }
            if frame == 70 {
                canvas
                    .window_mut()
                    .set_size(960, 640)
                    .map_err(|e| e.to_string())?;
            }
            if let Some(event) = scripted(frame, id) {
                // rust-sdl2 cannot convert TextInput back to an SDL queue
                // event. Keep this fixture explicit; normal typing still
                // arrives through the real SDL event pump below.
                if matches!(event, Event::TextInput { .. }) {
                    model.event(&event);
                } else {
                    events.push_event(event)?;
                }
            }
        }
        for event in pump.poll_iter() {
            model.event(&event);
        }
        if model.edit.is_some() && model.focused {
            video.text_input().start();
        } else {
            video.text_input().stop();
        }
        canvas.set_draw_color(Color::RGB(25, 30, 28));
        canvas.clear();
        text(
            &mut canvas,
            &tc,
            &font,
            "ToTEngine — native desktop feasibility",
            30,
            20,
        )?;
        text(
            &mut canvas,
            &tc,
            &font,
            "N: enter city name    Tab/Shift-Tab: focus    Enter/Esc: accept/cancel",
            30,
            60,
        )?;
        for y in 0..5 {
            for x in 0..5 {
                blit(
                    &mut canvas,
                    &tc,
                    &terrain,
                    Rect::new(220 + x * 64 + (y % 2) * 32, 180 + y * 16, 64, 32),
                )?;
            }
        }
        blit(&mut canvas, &tc, &unit, Rect::new(300, 180, 64, 64))?;
        blit(&mut canvas, &tc, &city, Rect::new(440, 194, 64, 48))?;
        text(
            &mut canvas,
            &tc,
            &font,
            &format!("City: {}", model.name),
            30,
            450,
        )?;
        text(
            &mut canvas,
            &tc,
            &font,
            if resources.is_some() {
                "Artwork: owned Original profile; nothing bundled"
            } else {
                "Artwork: authored synthetic fixture"
            },
            30,
            500,
        )?;
        if let Some(edit) = &model.edit {
            canvas.set_draw_color(Color::RGB(60, 63, 57));
            canvas.fill_rect(Rect::new(170, 180, 620, 250))?;
            text(&mut canvas, &tc, &font, "Name the city", 210, 205)?;
            canvas.set_draw_color(Color::RGB(20, 22, 20));
            canvas.fill_rect(Rect::new(210, 260, 540, 50))?;
            text(&mut canvas, &tc, &font, edit, 220, 270)?;
            text(&mut canvas, &tc, &font, "Accept", 260, 360)?;
            text(&mut canvas, &tc, &font, "Cancel", 590, 360)?;
            canvas.set_draw_color(Color::RGB(255, 225, 125));
            canvas.draw_rect(match model.focus {
                0 => Rect::new(210, 260, 540, 50),
                1 => Rect::new(210, 350, 210, 50),
                _ => Rect::new(540, 350, 210, 50),
            })?;
        }
        // Optional private visual inspection; never copy these captures to Git
        // when CIVTOT_HOME selects owned game artwork.
        if report.is_some() && matches!(frame, 20 | 35) {
            if let Some(prefix) = std::env::var_os("TOTENGINE_SPIKE_CAPTURE") {
                let (w, h) = canvas.output_size()?;
                let mut pixels = canvas.read_pixels(None, PixelFormatEnum::RGB24)?;
                let surface = sdl2::surface::Surface::from_data(
                    &mut pixels,
                    w,
                    h,
                    w * 3,
                    PixelFormatEnum::RGB24,
                )?;
                surface.save_bmp(format!("{}-{frame}.bmp", PathBuf::from(prefix).display()))?;
            }
        }
        canvas.present();
        frame += 1;
        if report.is_some() && frame > 1500 {
            return Err("desktop input test timed out".into());
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    if let Some(path) = report {
        if model.name != if scripted_input { "Athéna" } else { "athena" }
            || model.accepted != 1
            || model.cancelled != 1
            || (scripted_input && model.ignored != 1)
            || model.resized == 0
            || model.lost == 0
            || model.gained == 0
        {
            return Err(format!("desktop event checks failed: name={} accepted={} cancelled={} ignored={} resized={} lost={} gained={}", model.name, model.accepted, model.cancelled, model.ignored, model.resized, model.lost, model.gained));
        }
        let method = if scripted_input {
            "queued SDL key/window events; direct TextInput fixture"
        } else {
            "external OS keyboard and focus events"
        };
        std::fs::write(path, format!("{{\"schema\":1,\"platform\":\"{}\",\"sdl\":\"{}\",\"video\":\"{}\",\"frames\":{},\"installed_art\":{},\"accepted\":{},\"cancelled\":{},\"ignored_unfocused\":{},\"resize_events\":{},\"focus_lost\":{},\"focus_gained\":{},\"input_method\":\"{}\",\"name\":\"{}\"}}\n", std::env::consts::OS, sdl2::version::version(), video.current_video_driver(), frame, resources.is_some(), model.accepted, model.cancelled, model.ignored, model.resized, model.lost, model.gained, method, model.name)).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("desktop spike: {error}");
        std::process::exit(1);
    }
}
