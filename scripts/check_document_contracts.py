#!/usr/bin/env python3
"""Check bilingual reflection facts against documentation and real project inputs."""

import ast
from pathlib import Path
import re
import sys
import tomllib

FACTS = {
    "facade.explicit": "qubit_reflect",
    "provider.qualified": "custom-provider",
    "panic.async_poll": "not-caught",
    "panic.abort": "catching-unavailable",
    "dispatch.input_recovery": "original-input",
    "examples.native": "cargo-example",
    "coverage.configure": "70",
    "coverage.parse": "85",
    "coverage.validate": "80",
    "coverage.expand": "85",
    "registry.source": "src/registry/reflect_registry.rs",
}
GROUPS = (
    ("README.md", "README.zh_CN.md", ("facade.explicit", "examples.native")),
    ("derive/README.md", "derive/README.zh_CN.md", ("facade.explicit", "provider.qualified")),
    ("doc/user_guide.md", "doc/user_guide.zh_CN.md", (
        "facade.explicit", "provider.qualified", "panic.async_poll", "panic.abort",
        "dispatch.input_recovery", "examples.native")),
    ("doc/2026-09-03-qubit-reflect-design.md", "doc/2026-09-03-qubit-reflect-design.zh_CN.md", (
        "panic.async_poll", "panic.abort", "dispatch.input_recovery", "coverage.configure",
        "coverage.parse", "coverage.validate", "coverage.expand", "registry.source")),
    ("doc/2026-09-07-qubit-reflect-api-stability.md", "doc/2026-09-07-qubit-reflect-api-stability.zh_CN.md",
     ("dispatch.input_recovery",)),
    ("doc/derive-contract-matrix.md", "doc/derive-contract-matrix.zh_CN.md", (
        "coverage.configure", "coverage.parse", "coverage.validate", "coverage.expand")),
)
EXAMPLES = ("field_patch", "customer_patch", "support_action")
def document_facts(document):
    """Read unique structured facts with line-located diagnostics."""
    facts = {}
    for line_number, line in enumerate(document.read_text(encoding="utf-8").splitlines(), 1):
        if "<!-- reflect-contract:" not in line:
            continue
        matches = list(re.finditer(r"<!-- reflect-contract:\s*([a-z_.]+)=([^<>]*?)\s*-->", line))
        if len(matches) != line.count("<!-- reflect-contract:"):
            raise ValueError(f"{document}:{line_number}: malformed contract marker")
        for match in matches:
            key, value = match.groups()
            if key not in FACTS:
                raise ValueError(f"{document}:{line_number}: unknown contract key {key}")
            if key in facts:
                raise ValueError(f"{document}:{line_number}: duplicate contract key {key}")
            facts[key] = value
    return facts


def coverage_thresholds(root):
    """Read the gate's literal constant without executing a supplied root's script."""
    source = root / "scripts/derive_coverage_report.py"
    module = ast.parse(source.read_text(encoding="utf-8"), filename=str(source))
    values = []
    for node in module.body:
        if isinstance(node, ast.Assign) and any(isinstance(target, ast.Name) and target.id == "MINIMUM_LINE_COVERAGE" for target in node.targets):
            values.append(ast.literal_eval(node.value))
    if len(values) != 1 or not isinstance(values[0], dict):
        raise ValueError(f"{source}: expected one MINIMUM_LINE_COVERAGE constant")
    expected = {
        key.removeprefix("coverage."): float(value)
        for key, value in FACTS.items()
        if key.startswith("coverage.")
    }
    if values[0] != expected:
        differing = next((stage for stage in expected if values[0].get(stage) != expected[stage]), "stages")
        raise ValueError(f"{source}: coverage.{differing}: threshold drift: {values[0]}")
    return {f"coverage.{stage}": f"{value:g}" for stage, value in values[0].items()}


def check_examples(root):
    """Ensure the native Cargo examples really exist and ship with derive enabled."""
    path = root / "Cargo.toml"
    manifest = tomllib.loads(path.read_text(encoding="utf-8"))
    package = manifest["package"]
    if package.get("autoexamples") is False:
        raise ValueError(f"{path}: examples.native: automatic example discovery is disabled")
    if "/examples/**" not in package.get("include", []):
        raise ValueError(f"{path}: examples.native: package include must contain /examples/**")
    targets = manifest.get("example", [])
    for name in EXAMPLES:
        matches = [example for example in targets if example.get("name") == name]
        expected_path = f"examples/{name}.rs"
        if len(matches) != 1 or matches[0].get("path") != expected_path or matches[0].get("required-features") != ["derive"]:
            raise ValueError(f"{path}: examples.native: invalid {name} target or required-features")
        source = root / expected_path
        if not source.is_file() or not source.resolve().is_relative_to(root.resolve()):
            raise ValueError(f"{path}: examples.native: missing or escaping {expected_path}")


def check_source_checkout_docs(root):
    """Keep source-checkout instructions independent from sibling crate clones."""
    documents = (
        "README.md", "README.zh_CN.md", "doc/user_guide.md", "doc/user_guide.zh_CN.md",
        "derive/README.md", "derive/README.zh_CN.md",
    )
    for name in documents:
        path = root / name
        text = path.read_text(encoding="utf-8")
        if "prepare-local-path-dependencies.sh" in text:
            raise ValueError(f"{path}: source checkout must not require prepare-local-path-dependencies.sh")
        if "cargo metadata --locked --format-version 1" not in text:
            raise ValueError(f"{path}: source checkout must document cargo metadata --locked --format-version 1")
        if "rust-common/rs-id" in text or "rust-common/rs-datatype" in text:
            raise ValueError(f"{path}: use registry dependencies for external crates; sibling checkouts are not required")


def check_contracts(root):
    """Validate the required keys, bilingual values, and actual source contracts."""
    root = Path(root)
    expected_values = dict(FACTS)
    expected_values.update(coverage_thresholds(root))
    check_examples(root)
    check_source_checkout_docs(root)
    registry = root / FACTS["registry.source"]
    if not registry.is_file() or not registry.resolve().is_relative_to(root.resolve()):
        raise ValueError(f"registry.source: missing or escaping {registry}")
    for english, chinese, required in GROUPS:
        pair = []
        for name in (english, chinese):
            document = root / name
            facts = document_facts(document)
            if set(facts) != set(required):
                raise ValueError(f"{document}: missing keys {sorted(set(required) - set(facts))}; unexpected keys {sorted(set(facts) - set(required))}")
            for key, value in facts.items():
                if value != expected_values[key]:
                    raise ValueError(f"{document}: {key}: expected {expected_values[key]!r}, found {value!r}")
            pair.append(facts)
        if pair[0] != pair[1]:
            raise ValueError(f"{english} / {chinese}: bilingual contract facts differ")


def main():
    try:
        check_contracts(Path(__file__).resolve().parents[1])
    except (OSError, ValueError, KeyError, SyntaxError) as error:
        print(f"Document contracts failed: {error}", file=sys.stderr)
        return 1
    print("Validated bilingual document contracts and project inputs.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
