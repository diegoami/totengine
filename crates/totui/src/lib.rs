//! Browser interfaces and shared client resources. No game assets are bundled.
pub mod resources;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::time::Duration;

use engine::observer::Recorder;
use engine::player::Game;

pub const HTML: &str = include_str!("observer.html");
pub const SCRIPT: &str = include_str!("observer.js");

/// A live game advances only when a browser requests the next frame. Cached
/// history can be revisited without applying any orders again.
pub struct Observer {
    game: Game,
    recorder: Recorder,
    frames: Vec<String>,
    remaining: usize,
}

impl Observer {
    pub fn live(game: Game, turns: usize) -> Self {
        let mut recorder = Recorder::default();
        let frames = vec![recorder.capture(&game).to_json()];
        Self {
            game,
            recorder,
            frames,
            remaining: turns,
        }
    }

    /// Build frames only from a successfully verified canonical order log.
    pub fn replay(
        name: &str,
        save: &[u8],
        rules: tot_kit::Rules,
        events: Option<engine::events::EventEngine>,
        log: &str,
    ) -> Result<Self, String> {
        if log
            .lines()
            .skip(3)
            .last()
            .is_some_and(|line| !line.starts_with("R "))
        {
            return Err("observer replay needs a log ending at a completed turn".into());
        }
        let mut recorder = Recorder::default();
        let mut frames = Vec::new();
        let game = Game::replay_observed(name, save, rules, events, log, |g| {
            frames.push(recorder.capture(g).to_json())
        })?;
        Ok(Self {
            game,
            recorder,
            frames,
            remaining: 0,
        })
    }

    pub fn frame(&mut self, index: usize) -> Option<&str> {
        if index == self.frames.len() && self.remaining > 0 {
            self.game.play_turn();
            self.remaining -= 1;
            self.frames
                .push(self.recorder.capture(&self.game).to_json());
        }
        self.frames.get(index).map(String::as_str)
    }

    /// Run to the configured limit and export a portable observation archive.
    pub fn export(&mut self, mut out: impl Write) -> io::Result<()> {
        while self.remaining > 0 {
            self.frame(self.frames.len());
        }
        for frame in &self.frames {
            writeln!(out, "{frame}")?;
        }
        Ok(())
    }

    pub fn log(&self) -> &str {
        self.game.log()
    }

    fn route(&mut self, path: &str) -> (u16, &'static str, String) {
        match path {
            "/" => (200, "text/html; charset=utf-8", HTML.into()),
            "/observer.js" => (200, "text/javascript; charset=utf-8", SCRIPT.into()),
            "/status" => (
                200,
                "application/json",
                format!(
                    "{{\"frames\":{},\"total\":{}}}",
                    self.frames.len(),
                    self.frames.len() + self.remaining
                ),
            ),
            "/replay.ndjson" => (
                200,
                "application/x-ndjson",
                format!("{}\n", self.frames.join("\n")),
            ),
            "/order.log" => (200, "text/plain; charset=utf-8", self.log().into()),
            _ => {
                if let Some(index) = path.strip_prefix("/frame/").and_then(|s| s.parse().ok()) {
                    if let Some(frame) = self.frame(index) {
                        return (200, "application/json", frame.into());
                    }
                }
                (404, "text/plain", "Not found".into())
            }
        }
    }

    /// Loopback only. Exact routes serve embedded assets and in-memory game
    /// data; no URL is ever interpreted as a filesystem path.
    pub fn serve(mut self, port: u16) -> io::Result<()> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
        eprintln!(
            "Observer: http://{} (Ctrl-C to stop)",
            listener.local_addr()?
        );
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle(stream) {
                        eprintln!("observer request: {e}");
                    }
                }
                Err(e) => eprintln!("observer connection: {e}"),
            }
        }
        Ok(())
    }

    fn handle(&mut self, mut stream: TcpStream) -> io::Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        // Bound the request header; close each connection after one response.
        let mut reader = BufReader::new(&stream);
        let mut request = String::new();
        use std::io::Read as _;
        reader.by_ref().take(8192).read_line(&mut request)?;
        let parts: Vec<_> = request.split_whitespace().collect();
        let (status, mime, body) = match parts.as_slice() {
            ["GET", path, "HTTP/1.1" | "HTTP/1.0"] => self.route(path),
            _ => (400, "text/plain", "Bad request".into()),
        };
        write!(stream, "HTTP/1.1 {status} {}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'unsafe-inline'; connect-src 'self'; object-src 'none'; frame-ancestors 'none'\r\nConnection: close\r\n\r\n{body}", if status == 200 { "OK" } else { "Error" }, body.len())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_frames_never_advance_twice_and_bad_routes_do_nothing() {
        let game = Game::new(
            "test.sav",
            &engine::tests_support::synthetic_save_bytes(),
            1,
            tot_kit::parse_rules(""),
            None,
        )
        .unwrap();
        let mut observer = Observer::live(game, 2);
        let boot = observer.frame(0).unwrap().to_owned();
        assert_eq!(observer.route("/frame/99").0, 404);
        assert_eq!(observer.route("/../../etc/passwd").0, 404);
        let first = observer.frame(1).unwrap().to_owned();
        assert_eq!(observer.frame(1).unwrap(), first);
        assert_eq!(observer.frame(0).unwrap(), boot);
        assert_eq!(observer.remaining, 1);
        let mut archive = Vec::new();
        observer.export(&mut archive).unwrap();
        assert_eq!(String::from_utf8(archive).unwrap().lines().count(), 3);
        assert!(observer.frame(3).is_none());
    }

    #[test]
    fn replay_rejects_orders_after_the_last_complete_turn() {
        let save = engine::tests_support::synthetic_save_bytes();
        let rules = || tot_kit::parse_rules("");
        let mut game = Game::new("test.sav", &save, 1, rules(), None).unwrap();
        game.play_turn();
        let complete = game.log().to_string();
        assert!(Observer::replay("test.sav", &save, rules(), None, &complete).is_ok());
        let partial = format!("{complete}T {} C 1 end = ended\n", game.world.turn);
        // A partial log is valid for the general player API, but cannot
        // be represented completely by per-turn observer snapshots.
        assert!(Game::replay("test.sav", &save, rules(), None, &partial).is_ok());
        assert!(Observer::replay("test.sav", &save, rules(), None, &partial).is_err());
    }
}
