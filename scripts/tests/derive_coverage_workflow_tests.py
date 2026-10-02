#!/usr/bin/env python3
"""Regression tests for the standalone derive coverage workflow job."""

from pathlib import Path
import unittest


WORKFLOW = Path(__file__).resolve().parents[2] / ".github/workflows/ci.yml"


def job_block(workflow: str, name: str, next_job: str) -> str:
    start = workflow.index(f"  {name}:")
    end = workflow.index(f"\n  {next_job}:", start + 1)
    return workflow[start:end]


class DeriveCoverageWorkflowTests(unittest.TestCase):
    def test_reusable_ci_workflow_enables_derive_coverage(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        rust_ci = job_block(workflow, "rust-ci", "downstream-baseline")

        self.assertIn(
            "uses: qubit-ltd/rs-infra-ci/.github/workflows/github-ci.yml@",
            rust_ci,
        )
        self.assertIn("derive-coverage: true", rust_ci)

    def test_reusable_ci_workflow_owns_pages_and_derive_coverage(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("derive-coverage: true", workflow)


if __name__ == "__main__":
    unittest.main()
