//! Loopback-only human host. GET only reads; all changes require token+origin.
use super::{storage, Client, Input};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::time::Duration;

const HTML: &str = include_str!("play.html");
const SCRIPT: &str = include_str!("play.js");
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    revision: u64,
    id: u64,
    input: Input,
}
fn response(stream: &mut TcpStream, status: &str, kind: &str, bytes: &[u8]) -> std::io::Result<()> {
    write!(stream,"HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nCross-Origin-Resource-Policy: same-origin\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'\r\nConnection: close\r\n\r\n",bytes.len())?;
    stream.write_all(bytes)
}
fn failure(stream: &mut TcpStream, status: &str, message: &str) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(&serde_json::json!({"error":message})).unwrap();
    response(stream, status, "application/json", &bytes)
}
pub fn serve(client: Client, open: bool, port: u16) -> Result<(), String> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).map_err(|e| e.to_string())?;
    let origin = format!(
        "http://127.0.0.1:{}",
        listener.local_addr().map_err(|e| e.to_string())?.port()
    );
    let token = storage::nonce()?;
    eprintln!("Playable browser client: {origin}");
    if open {
        let _ = open_browser(&origin);
    }
    let mut client = client;
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let _ = connection(&mut stream, &mut client, &origin, &token);
                if client.closed {
                    break;
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
fn open_browser(url: &str) -> std::io::Result<std::process::Child> {
    #[cfg(windows)]
    {
        std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()
    }
}
fn connection(
    stream: &mut TcpStream,
    client: &mut Client,
    origin: &str,
    token: &str,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut first = String::new();
    reader.by_ref().take(2049).read_line(&mut first)?;
    let parts: Vec<_> = first.split_whitespace().collect();
    if parts.len() != 3 || first.len() > 2048 || parts[2] != "HTTP/1.1" {
        return failure(stream, "400 Bad Request", "Invalid request");
    }
    let (method, path) = (parts[0], parts[1]);
    let mut headers = BTreeMap::new();
    let mut total = first.len();
    loop {
        let mut line = String::new();
        reader.by_ref().take(8193).read_line(&mut line)?;
        total += line.len();
        if total > 16384 {
            return failure(
                stream,
                "431 Request Header Fields Too Large",
                "Headers too large",
            );
        }
        if line == "\r\n" {
            break;
        }
        if line.is_empty() {
            return failure(stream, "400 Bad Request", "Incomplete headers");
        }
        let Some((name, value)) = line.trim_end().split_once(':') else {
            return failure(stream, "400 Bad Request", "Invalid header");
        };
        if headers
            .insert(name.to_ascii_lowercase(), value.trim().to_string())
            .is_some()
        {
            return failure(stream, "400 Bad Request", "Duplicate header");
        }
    }
    if headers.get("host").map(String::as_str) != origin.strip_prefix("http://") {
        return failure(stream, "403 Forbidden", "Wrong host");
    }
    if headers.contains_key("transfer-encoding") {
        return failure(stream, "400 Bad Request", "Transfer encoding unsupported");
    }
    if headers
        .get("sec-fetch-site")
        .is_some_and(|v| v != "same-origin" && v != "none")
    {
        return failure(stream, "403 Forbidden", "Cross-site access rejected");
    }
    match (method, path) {
        ("GET", "/") => response(
            stream,
            "200 OK",
            "text/html; charset=utf-8",
            HTML.replace("__TOKEN__", token).as_bytes(),
        ),
        ("GET", "/play.js") => response(
            stream,
            "200 OK",
            "text/javascript; charset=utf-8",
            SCRIPT.as_bytes(),
        ),
        ("GET", "/artwork-license") => response(
            stream,
            "200 OK",
            "text/plain; charset=utf-8",
            concat!(
                "ToTEngine code: MIT.\n\n",
                "Optional Isotrident artwork: Freeciv 3.2.7, GPL-2.0-or-later.\n",
                "Artists include Tim F. Smith, Tatu Rissanen, Jerzy Klek, Marco Saupe,\n",
                "Daniel Speyer, Alexandre Beraud, Jeff Mallatt, Davide Pagnin,\n",
                "GriffonSpade, Eleazar, Vincent Croisier and Jacob Nevins.\n",
                "Full credits and editable PNG/spec source accompany the isotrident pack.\n",
                "Unmodified upstream source: https://files.freeciv.org/stable/freeciv-3.2.7.tar.xz\n",
                "Open demo gameplay rules are independently authored, MIT; they are not ToT rules.\n\n",
                include_str!("../../../../docs/third-party/isotrident-GPL-2.0.txt")
            ).as_bytes(),
        ),
        ("GET", "/state") => response(
            stream,
            "200 OK",
            "application/json",
            &serde_json::to_vec(&client.frame()).unwrap(),
        ),
        ("GET", p) if p.starts_with("/sprite/") => {
            let Some((asset, key)) = p[8..].split_once('/') else {
                return failure(stream, "404 Not Found", "Unknown sprite");
            };
            if client.frame().assets != asset {
                return failure(stream, "409 Conflict", "Resources changed; refresh");
            }
            let Some(sprite) = client.atlas().and_then(|a| a.sprites.get(key)) else {
                return failure(stream, "404 Not Found", "Unknown sprite");
            };
            let mut data = Vec::with_capacity(sprite.rgba.len() + 8);
            data.extend_from_slice(&sprite.w.to_le_bytes());
            data.extend_from_slice(&sprite.h.to_le_bytes());
            data.extend_from_slice(&sprite.rgba);
            response(stream, "200 OK", "application/octet-stream", &data)
        }
        ("POST", "/input") => {
            if headers.get("origin").map(String::as_str) != Some(origin)
                || headers.get("x-totengine-token").map(String::as_str) != Some(token)
            {
                return failure(stream, "403 Forbidden", "Origin or launch token differs");
            }
            if headers.get("content-type").map(String::as_str) != Some("application/json") {
                return failure(stream, "415 Unsupported Media Type", "Expected JSON");
            }
            let Some(length) = headers
                .get("content-length")
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|&n| n <= 16384)
            else {
                return failure(
                    stream,
                    "413 Payload Too Large",
                    "Expected a bounded content length",
                );
            };
            let mut bytes = vec![0; length];
            reader.read_exact(&mut bytes)?;
            let envelope = match serde_json::from_slice::<Envelope>(&bytes) {
                Ok(e) => e,
                Err(_) => return failure(stream, "400 Bad Request", "Invalid input envelope"),
            };
            match client.input(envelope.revision, envelope.id, envelope.input) {
                Ok(()) => response(
                    stream,
                    "200 OK",
                    "application/json",
                    &serde_json::to_vec(&client.frame()).unwrap(),
                ),
                Err(e) => failure(stream, "409 Conflict", &e),
            }
        }
        ("GET", _) => failure(stream, "404 Not Found", "Not found"),
        _ => failure(
            stream,
            "405 Method Not Allowed",
            "Only read GET and authenticated input POST are supported",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn exchange(client: &mut Client, method: &str, path: &str, extra: &str, body: &str) -> String {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let origin = format!("http://{address}");
        let request=format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
        let sender = std::thread::spawn(move || {
            let mut s = TcpStream::connect(address).unwrap();
            s.write_all(request.as_bytes()).unwrap();
            let mut response = String::new();
            s.read_to_string(&mut response).unwrap();
            response
        });
        let (mut socket, _) = listener.accept().unwrap();
        connection(&mut socket, client, &origin, "test-token").unwrap();
        drop(socket);
        sender.join().unwrap()
    }
    #[test]
    fn reads_do_not_mutate_and_cross_origin_or_unauthed_posts_are_rejected() {
        let mut client = Client::new(String::new(), std::env::temp_dir());
        let before = serde_json::to_string(&client.frame()).unwrap();
        for path in [
            "/state",
            "/",
            "/play.js",
            "/input",
            "/checkpoint",
            "/../../../etc/passwd",
        ] {
            let _ = exchange(&mut client, "GET", path, "", "");
            assert_eq!(serde_json::to_string(&client.frame()).unwrap(), before);
        }
        let body = r#"{"revision":0,"id":1,"input":{"type":"click","x":102,"y":392}}"#;
        assert!(exchange(
            &mut client,
            "POST",
            "/input",
            "Content-Type: application/json\r\n",
            body
        )
        .starts_with("HTTP/1.1 403"));
        assert!(exchange(&mut client,"POST","/input","Origin: https://evil.example\r\nX-Totengine-Token: test-token\r\nContent-Type: application/json\r\n",body).starts_with("HTTP/1.1 403"));
        assert!(exchange(
            &mut client,
            "GET",
            "/state",
            "Sec-Fetch-Site: cross-site\r\n",
            ""
        )
        .starts_with("HTTP/1.1 403"));
        assert_eq!(serde_json::to_string(&client.frame()).unwrap(), before);
    }
    #[test]
    fn input_retry_is_idempotent_and_changed_or_stale_input_is_rejected() {
        let mut client = Client::new(String::new(), std::env::temp_dir());
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let origin = format!("http://{address}");
        for (revision, id, x, expected) in [
            (0, 1, 102, "200"),
            (0, 1, 102, "200"),
            (0, 1, 362, "409"),
            (0, 2, 362, "409"),
        ] {
            let body=format!("{{\"revision\":{revision},\"id\":{id},\"input\":{{\"type\":\"click\",\"x\":{x},\"y\":392}}}}");
            let request=format!("POST /input HTTP/1.1\r\nHost: {address}\r\nOrigin: {origin}\r\nX-Totengine-Token: test-token\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",body.len());
            let sender = std::thread::spawn(move || {
                let mut s = TcpStream::connect(address).unwrap();
                s.write_all(request.as_bytes()).unwrap();
                let mut response = String::new();
                s.read_to_string(&mut response).unwrap();
                response
            });
            let (mut socket, _) = listener.accept().unwrap();
            connection(&mut socket, &mut client, &origin, "test-token").unwrap();
            drop(socket);
            assert!(sender
                .join()
                .unwrap()
                .starts_with(&format!("HTTP/1.1 {expected}")));
        }
        assert_eq!(client.civs, 6);
        assert_eq!(client.human, 1);
        assert_eq!(client.revision, 1);
    }
}
