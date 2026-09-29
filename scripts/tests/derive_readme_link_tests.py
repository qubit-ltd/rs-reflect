"""Regression tests for links in the derive crate's bilingual READMEs."""

import pathlib
import re
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
LICENSE_URL = "https://github.com/qubit-ltd/rs-reflect/blob/main/LICENSE"


class DeriveReadmeLinkTests(unittest.TestCase):
    def test_both_license_links_in_each_readme_target_repository_license(self):
        for relative_path in ("derive/README.md", "derive/README.zh_CN.md"):
            with self.subTest(readme=relative_path):
                content = (ROOT / relative_path).read_text(encoding="utf-8")
                links = re.findall(r"\]\(([^)]*LICENSE[^)]*)\)", content)
                self.assertEqual(links, [LICENSE_URL, LICENSE_URL])


if __name__ == "__main__":
    unittest.main()
