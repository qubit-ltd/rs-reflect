import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
HELPER = ROOT / "scripts/check-critical-coverage-report.sh"
CONFIG = json.loads((ROOT / ".infra/ci/critical-coverage.json").read_text())


def report_for(project_root: Path, *, failing: bool = False, missing_group: bool = False):
    files = []
    for relative_path in CONFIG["files"]:
        files.append((project_root / relative_path, False))
    for group in CONFIG["groups"].values():
        if missing_group and group["path_prefix"] == "src/invoke/pinned/":
            continue
        files.append((project_root / group["path_prefix"] / "fixture.rs", True))

    entries = []
    for path, is_group in files:
        metrics = {}
        for name in ("functions", "lines", "regions"):
            if is_group:
                threshold = CONFIG["groups"]["invocation"][name]
            else:
                threshold = CONFIG["files"][str(path.relative_to(project_root))][name]
            actual = 100
            if failing and path.name == "reflect_registry.rs" and name == "lines":
                actual = threshold - 1
            metrics[name] = {"covered": actual, "count": 100, "percent": actual}
        entries.append({"filename": str(path), "summary": metrics})
    return {"data": [{"files": entries}]}


class CriticalCoverageEntrypointTests(unittest.TestCase):
    def test_helper_propagates_pass_threshold_failure_and_missing_report(self):
        with tempfile.TemporaryDirectory(prefix="critical-coverage-entrypoint-") as temp:
            temp_path = Path(temp)
            passing = temp_path / "passing.json"
            passing.write_text(json.dumps(report_for(ROOT)))
            result = subprocess.run([str(HELPER), str(passing)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("actual weighted: functions=100/100 (100%)", result.stdout)

            failing = temp_path / "failing.json"
            failing.write_text(json.dumps(report_for(ROOT, failing=True)))
            result = subprocess.run([str(HELPER), str(failing)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("threshold failed", result.stderr)

            missing_group = temp_path / "missing-group.json"
            missing_group.write_text(json.dumps(report_for(ROOT, missing_group=True)))
            result = subprocess.run([str(HELPER), str(missing_group)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("group is absent", result.stderr)

            absent = temp_path / "absent.json"
            result = subprocess.run([str(HELPER), str(absent)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("does not exist", result.stderr)

    def test_wrappers_check_only_after_report_generation_with_one_helper(self):
        coverage = (ROOT / "coverage.sh").read_text()
        ci_check = (ROOT / "ci-check.sh").read_text()
        align = (ROOT / "align-ci.sh").read_text()
        report = (ROOT / ".infra/tools/coverage-report.sh").read_text()
        self.assertLess(coverage.index('"$project_root/.infra/tools/coverage-report.sh"'),
                        coverage.index("check-critical-coverage-report.sh"))
        self.assertLess(ci_check.index('"$project_root/.infra/tools/infra-tool.sh"'),
                        ci_check.index('"$project_root/.infra/tools/coverage-report.sh"'))
        self.assertLess(ci_check.index('"$project_root/.infra/tools/coverage-report.sh"'),
                        ci_check.index("check-critical-coverage-report.sh"))
        self.assertIn('"$project_root/scripts/check-critical-coverage-report.sh" "$project_root/coverage.json"', coverage)
        self.assertIn('"$project_root/scripts/check-critical-coverage-report.sh" "$project_root/coverage.json"', ci_check)
        self.assertIn('if [ "${RUN_COVERAGE_IN_ALIGN:-0}" = 1 ]', align)
        self.assertIn('"$project_root/scripts/check-critical-coverage-report.sh" "$project_root/coverage.json"', align)
        self.assertLess(align.index('rs-infra-coverage --project'), align.index('"$project_root/.infra/tools/coverage-report.sh"'))
        self.assertLess(align.index('"$project_root/.infra/tools/coverage-report.sh"'), align.index("check-critical-coverage-report.sh"))
        self.assertIn("coverage.json", report)

    def test_ci_check_selects_default_or_explicit_coverage_tasks(self):
        ci_check = (ROOT / "ci-check.sh").read_text()
        self.assertIn("coverage_selected=true", ci_check)
        self.assertIn('[ "$argument" = "--only" ]', ci_check)
        self.assertIn('[[ "$argument" == --only=* ]]', ci_check)
        self.assertIn('if [ "$task" = coverage ]', ci_check)


if __name__ == "__main__":
    unittest.main()
