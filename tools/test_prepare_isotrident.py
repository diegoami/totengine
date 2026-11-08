import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from prepare_isotrident import SPECS, copy_pack, sprites


class PackTests(unittest.TestCase):
    def test_grids_borders_aliases_and_comments(self):
        result = sprites('''[grid_main]
x_top_left = 1
y_top_left = 2
dx = 64
dy = 32
pixel_border = 1
tiles = { "row", "column", "tag"
; 0, 0, "disabled"
2, 3, "first", "alias" ; comment
}
[grid_other]
dx = 30
dy = 30
tiles = { "row", "column", "tag"
1, 2, "unit"
}
''', 'test')
        self.assertEqual(result['first']['rect'], [196, 68, 64, 32])
        self.assertEqual(result['alias'], result['first'])
        self.assertEqual(result['unit']['rect'], [60, 30, 30, 30])
        self.assertNotIn('disabled', result)

    def test_copy_requires_complete_unchanged_notices_and_omits_extras(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            pack = root / 'pack'
            files = {'COPYING', 'FREECIV-README', 'isotrident.tilespec', 'atlas.json'}
            files.update(s + ext for s in SPECS for ext in ('.png', '.spec'))
            hashes = {}
            for name in files:
                p = pack / name
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_bytes(name.encode())
                hashes[name] = hashlib.sha256(p.read_bytes()).hexdigest()
            (pack / 'SHA256.json').write_text(json.dumps(hashes))
            (pack / 'unrelated.save').write_text('must not enter package')
            copy_pack(pack, root / 'copied')
            self.assertEqual({str(p.relative_to(root / 'copied')) for p in (root / 'copied').rglob('*') if p.is_file()}, files | {'SHA256.json'})
            (pack / 'COPYING').write_text('changed license')
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                copy_pack(pack, root / 'bad')
            self.assertFalse((root / 'bad').exists())


if __name__ == '__main__':
    unittest.main()
