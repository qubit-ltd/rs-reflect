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
    def test_standalone_job_prepares_paths_and_checks_cargo_metadata_before_coverage(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        job = job_block(workflow, "derive-coverage", "pages")

        checkout = job.index("uses: actions/checkout@")
        prepare = job.index("./.infra/bin/prepare-local-path-dependencies.sh")
        toolchain = job.index("toolchain: 1.94.0")
        metadata = job.index("cargo +1.94.0 metadata --locked --format-version 1")
        llvm_cov = job.index("cargo install cargo-llvm-cov")
        derive_coverage = job.index("./scripts/check-derive-coverage.sh")

        self.assertLess(checkout, prepare)
        self.assertLess(prepare, toolchain)
        self.assertLess(toolchain, metadata)
        self.assertLess(metadata, llvm_cov)
        self.assertLess(llvm_cov, derive_coverage)

    def test_pages_job_waits_for_derive_coverage(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        pages = job_block(workflow, "pages", "pages-deploy")

        self.assertIn("derive-coverage", pages)
        self.assertIn(
            "needs: [verify, feature-matrix, coverage, derive-coverage]", pages
        )


if __name__ == "__main__":
    unittest.main()
