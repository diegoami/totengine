#!/usr/bin/env python3
"""Serve the design prototype with artwork from a user's own ToT install.

CIVTOT_HOME=/path/to/CIVTOT python3 tools/preview_client.py --profile Original
No assets are copied, converted or written. Only the fixed artwork allowlist
and fixed reviewed design files are exposed, on IPv4 loopback only.
"""
import argparse
import hashlib
import json
import mimetypes
import os
from pathlib import Path
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit

REPO = Path(__file__).resolve().parents[1]
ART_NAMES = ("terrain1", "terrain2", "units", "cities", "city", "icons", "people", "dialog")
DESIGN_FILES = {
    "/": "client-prototype.html",
    "/asset-preview.js": "asset-preview.js",
    **{f"/{name}.md": f"{name}.md" for name in ("client-design", "discovery", "screen-contracts", "client-milestones")},
}


def child(directory, name):
    """Resolve a single Windows-style case-insensitive filename, unambiguously."""
    found = [p for p in directory.iterdir() if p.name.casefold() == name.casefold()]
    if len(found) > 1:
        raise ValueError(f"ambiguous case-insensitive filename: {directory / name}")
    return found[0] if found else None


def artwork(home, profile):
    root = Path(home).resolve(strict=True)
    folder = root
    parts = profile.replace("\\", "/").split("/")
    if any(p in ("", ".", "..") for p in parts):
        raise ValueError("profile must be a relative directory without . or .. components")
    for part in parts:
        folder = child(folder, part)
        if folder is None or not folder.is_dir():
            raise ValueError(f"profile directory not found: {profile}")
        folder = folder.resolve(strict=True)
        if not folder.is_relative_to(root):
            raise ValueError("profile must remain inside the install")
    selected = {}
    for name in ART_NAMES:
        for directory in (folder, root):
            for ext in (".bmp", ".gif"):
                path = child(directory, name + ext)
                if path is None:
                    continue
                path = path.resolve(strict=True)
                if not path.is_file() or not path.is_relative_to(root):
                    raise ValueError(f"artwork must be a file inside the install: {name}")
                selected[name] = path
                break
            if name in selected:
                break
    manifest = {
        "kind": "totengine-design-artwork",
        "profile": str(folder.relative_to(root)),
        "simulation": True,
        "assets": {
            key: {
                "url": "/assets/" + key,
                "source": str(path.relative_to(root)),
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            }
            for key, path in selected.items()
        },
        "missing": [n for n in ART_NAMES if n not in selected],
    }
    return selected, manifest


def handler(selected, manifest):
    class PreviewHandler(BaseHTTPRequestHandler):
        def do_GET(self):
            path = urlsplit(self.path).path
            if path == "/assets/manifest":
                payload = json.dumps(manifest).encode()
                content_type = "application/json"
            elif path in DESIGN_FILES:
                source = REPO / "docs/ui" / DESIGN_FILES[path]
                payload = source.read_bytes()
                content_type = mimetypes.guess_type(source.name)[0] or "application/octet-stream"
            elif path.startswith("/assets/") and path[8:] in selected:
                source = selected[path[8:]]
                payload = source.read_bytes()
                content_type = mimetypes.guess_type(source.name)[0] or "application/octet-stream"
            else:
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(payload)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.send_header("Cross-Origin-Resource-Policy", "same-origin")
            self.end_headers()
            self.wfile.write(payload)

    return PreviewHandler


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--home", default=os.environ.get("CIVTOT_HOME"))
    parser.add_argument("--profile", default="Original")
    parser.add_argument("--port", type=int, default=8765)
    args = parser.parse_args()
    if not 0 <= args.port <= 65535:
        parser.error("--port must be between 0 and 65535")
    if not args.home:
        parser.error("set CIVTOT_HOME or pass --home")
    try:
        selected, manifest = artwork(args.home, args.profile)
    except (OSError, ValueError) as error:
        parser.error(str(error))
    required = {"terrain1", "units", "cities"} - selected.keys()
    if required:
        parser.error("missing required artwork: " + ", ".join(sorted(required)))
    try:
        server = ThreadingHTTPServer(("127.0.0.1", args.port), handler(selected, manifest))
    except OSError as error:
        parser.error(f"cannot listen on 127.0.0.1:{args.port}: {error}; choose another --port")
    print(f"Design preview: http://127.0.0.1:{server.server_port} (profile {manifest['profile']})", flush=True)
    print("Simulation only. Artwork stays in the install; do not commit screenshots containing it.", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
