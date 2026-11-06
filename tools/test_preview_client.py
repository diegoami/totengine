"""Install-free checks for artwork selection and bounded HTTP exposure."""
import http.client
import json
from pathlib import Path
import tempfile
import threading
import unittest
from http.server import ThreadingHTTPServer

from preview_client import artwork, handler


class ArtworkSelection(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.profile = self.root / "Original"
        self.profile.mkdir()

    def test_profile_precedes_root_and_bmp_precedes_gif_with_windows_case(self):
        (self.root / "Units.BMP").write_bytes(b"root bmp")
        (self.profile / "UNITS.gif").write_bytes(b"profile gif")
        selected, manifest = artwork(self.root, "original")
        self.assertEqual(selected["units"].read_bytes(), b"profile gif")
        (self.profile / "units.BmP").write_bytes(b"profile bmp")
        selected, manifest = artwork(self.root, "ORIGINAL")
        self.assertEqual(selected["units"].read_bytes(), b"profile bmp")
        self.assertEqual(manifest["assets"]["units"]["source"], "Original/units.BmP")
        self.assertNotIn(str(self.root), json.dumps(manifest))
        self.assertIn("terrain1", manifest["missing"])

    def test_missing_profile_asset_falls_back_to_root_gif(self):
        (self.root / "Terrain1.GIF").write_bytes(b"root gif")
        selected, _ = artwork(self.root, "Original")
        self.assertEqual(selected["terrain1"].read_bytes(), b"root gif")

    def test_ambiguous_case_and_missing_profile_fail(self):
        with self.assertRaises(ValueError):
            artwork(self.root, "absent")
        (self.profile / "units.bmp").write_bytes(b"a")
        (self.profile / "UNITS.BMP").write_bytes(b"b")
        with self.assertRaisesRegex(ValueError, "ambiguous"):
            artwork(self.root, "Original")

    def test_traversal_and_symlinks_outside_install_fail(self):
        for profile in ("../outside", "/Original", "Original/..", "Original/.", "Original/../Original"):
            with self.subTest(profile=profile), self.assertRaises(ValueError):
                artwork(self.root, profile)
        with tempfile.TemporaryDirectory() as other:
            outside = Path(other)
            (self.root / "linked").symlink_to(outside, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "inside"):
                artwork(self.root, "linked")
            (outside / "secret.bmp").write_bytes(b"outside")
            (self.profile / "units.bmp").symlink_to(outside / "secret.bmp")
            with self.assertRaisesRegex(ValueError, "inside"):
                artwork(self.root, "Original")


class BoundedHttp(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        root = Path(cls.temp.name)
        (root / "Original").mkdir()
        (root / "Terrain1.gif").write_bytes(b"GIF fixture bytes")
        selected, manifest = artwork(root, "Original")
        base = handler(selected, manifest)

        class QuietHandler(base):
            def log_message(self, *args):
                pass

        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), QuietHandler)
        cls.thread = threading.Thread(target=cls.server.serve_forever)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join()
        cls.temp.cleanup()

    def get(self, path, method="GET"):
        connection = http.client.HTTPConnection(*self.server.server_address, timeout=3)
        self.addCleanup(connection.close)
        connection.request(method, path)
        response = connection.getresponse()
        return response.status, dict(response.getheaders()), response.read()

    def test_exact_asset_and_manifest(self):
        status, headers, data = self.get("/assets/terrain1?refresh=1")
        self.assertEqual(status, 200)
        self.assertEqual(data, b"GIF fixture bytes")
        self.assertEqual(headers["Content-Type"], "image/gif")
        self.assertEqual(headers["Cross-Origin-Resource-Policy"], "same-origin")
        self.assertNotIn("Access-Control-Allow-Origin", headers)
        status, _, data = self.get("/assets/manifest")
        self.assertEqual(status, 200)
        self.assertEqual(json.loads(data)["assets"]["terrain1"]["source"], "Terrain1.gif")

    def test_only_allowlisted_routes_and_no_write_method(self):
        for path in ("/assets/../Rules.txt", "/assets/%2e%2e/Rules.txt", "/assets/units", "/README.md", "/assets/terrain1/extra"):
            with self.subTest(path=path):
                self.assertEqual(self.get(path)[0], 404)
        self.assertEqual(self.get("/assets/terrain1", method="POST")[0], 501)
        self.assertEqual(self.get("/")[0], 200)
        self.assertEqual(self.get("/asset-preview.js")[0], 200)
        self.assertEqual(self.get("/client-design.md")[0], 200)
        self.assertEqual(self.get("/client-architecture.md")[0], 200)


if __name__ == "__main__":
    unittest.main()
