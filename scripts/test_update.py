import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().with_name("update.py")


class UpdateVersionTest(unittest.TestCase):
    def test_increments_vibi_dpu_patch_version(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "vibi-dpu" / "Cargo.toml"
            manifest.parent.mkdir()
            manifest.write_text(
                '[package]\nname = "vibi-dpu"\nversion = "1.2.3"\nedition = "2021"\n'
            )

            subprocess.run([sys.executable, str(SCRIPT)], cwd=directory, check=True)

            self.assertIn('version = "1.2.4"', manifest.read_text())


if __name__ == "__main__":
    unittest.main()
