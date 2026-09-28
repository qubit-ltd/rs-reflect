"""Contract tests for the independent derive coverage summary."""

import unittest
from pathlib import Path

from scripts.derive_coverage_report import CoverageReportError, summarize


SOURCE_ROOT = Path('/fixture/derive/src')


def coverage_file(path: str, covered: int, count: int) -> dict:
    return {
        'filename': str(SOURCE_ROOT / path),
        'summary': {'lines': {'covered': covered, 'count': count}},
    }


class DeriveCoverageReportTests(unittest.TestCase):
    def test_summarizes_each_stage_from_instrumented_source(self):
        report = {
            'data': [{
                'files': [
                    coverage_file('configure/mod.rs', 7, 10),
                    coverage_file('parse/declaration.rs', 17, 20),
                    coverage_file('validate/declaration.rs', 4, 5),
                    coverage_file('expand/dispatcher.rs', 17, 20),
                    coverage_file('lib.rs', 0, 100),
                ],
            }],
        }

        result = summarize(report, SOURCE_ROOT)

        self.assertEqual(result['status'], 'complete')
        self.assertEqual(set(result['stages']), {'configure', 'parse', 'validate', 'expand'})
        self.assertEqual(result['stages']['configure']['covered_lines'], 7)
        self.assertEqual(result['stages']['expand']['line_percent'], 85.0)
        self.assertEqual(result['stages']['expand']['minimum_line_percent'], 85.0)

    def test_rejects_stage_below_its_coverage_floor(self):
        report = {
            'data': [{
                'files': [
                    coverage_file('configure/mod.rs', 69, 100),
                    coverage_file('parse/declaration.rs', 85, 100),
                    coverage_file('validate/declaration.rs', 80, 100),
                    coverage_file('expand/dispatcher.rs', 85, 100),
                ],
            }],
        }

        with self.assertRaises(CoverageReportError) as context:
            summarize(report, SOURCE_ROOT)

        self.assertIn('configure: 69.0% is below the 70.0% minimum', str(context.exception))

    def test_accepts_each_stage_at_its_coverage_floor(self):
        report = {
            'data': [{
                'files': [
                    coverage_file('configure/mod.rs', 70, 100),
                    coverage_file('parse/declaration.rs', 85, 100),
                    coverage_file('validate/declaration.rs', 80, 100),
                    coverage_file('expand/dispatcher.rs', 85, 100),
                ],
            }],
        }

        result = summarize(report, SOURCE_ROOT)

        self.assertEqual(result['status'], 'complete')

    def test_rejects_empty_report_instead_of_claiming_full_coverage(self):
        with self.assertRaises(ValueError) as context:
            summarize({'data': []}, SOURCE_ROOT)
        self.assertIn('no data entries', str(context.exception))

    def test_rejects_missing_or_unexecuted_stage(self):
        report = {'data': [{'files': [coverage_file('parse/mod.rs', 0, 2)]}]}
        with self.assertRaises(CoverageReportError) as context:
            summarize(report, SOURCE_ROOT)
        self.assertIn('configure: no instrumented source files', str(context.exception))
        self.assertIn('configure: no instrumented source files', str(context.exception))
        self.assertIn('parse: no executed lines', str(context.exception))


if __name__ == '__main__':
    unittest.main()
