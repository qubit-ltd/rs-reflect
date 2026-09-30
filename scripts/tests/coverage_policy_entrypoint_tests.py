#!/usr/bin/env python3
"""Regression checks for the shared coverage policy entry points."""

import unittest
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[2]


class CoveragePolicyEntrypointTests(unittest.TestCase):
    def test_standalone_coverage_uses_global_check_after_report(self):
        coverage = PROJECT_ROOT / ".infra/bin/coverage.sh"
        self.assertTrue(coverage.is_file(), "missing .infra/bin/coverage.sh")
        if not coverage.is_file():
            return
        source = coverage.read_text(encoding="utf-8")
        collect = source.index("rs-infra-coverage")
        report = source.index("coverage-report.sh")
        check = source.index("check --input")
        self.assertLess(collect, report)
        self.assertLess(report, check)
        self.assertNotIn("check-critical-coverage-report.sh", source)

    def test_ci_and_alignment_do_not_apply_project_only_thresholds(self):
        for relative_path in (".infra/bin/ci-check.sh", ".infra/bin/align-ci.sh"):
            with self.subTest(script=relative_path):
                path = PROJECT_ROOT / relative_path
                self.assertTrue(path.is_file(), f"missing {relative_path}")
                if path.is_file():
                    self.assertNotIn("check-critical-coverage-report.sh", path.read_text(encoding="utf-8"))

    def test_project_critical_threshold_config_is_absent(self):
        self.assertFalse((PROJECT_ROOT / ".infra/ci/critical-coverage.json").exists())


if __name__ == "__main__":
    unittest.main()
