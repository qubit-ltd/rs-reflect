#!/usr/bin/env python3
"""Check bilingual reflection facts against documentation and real project inputs."""

from pathlib import Path
import re
import sys
import tomllib
from urllib.parse import unquote
from urllib.parse import urlsplit

FACTS = {
    "runtime.direct": "qubit_reflect",
    "provider.qualified": "custom-provider",
    "panic.async_poll": "not-caught",
    "panic.abort": "catching-unavailable",
    "dispatch.input_recovery": "original-input",
    "examples.native": "cargo-example",
    "typeref.constructor": "owned-resolved",
    "lazy.retry": "panic-retry",
    "field.lookup": "linear",
}
GROUPS = (
    ("README.md", "README.zh_CN.md", ("runtime.direct", "examples.native")),
    ("derive/README.md", "derive/README.zh_CN.md", ("runtime.direct", "provider.qualified")),
    ("doc/user_guide.md", "doc/user_guide.zh_CN.md", (
        "runtime.direct", "provider.qualified", "panic.async_poll", "panic.abort",
        "dispatch.input_recovery", "examples.native", "typeref.constructor",
        "lazy.retry", "field.lookup")),
)
EXAMPLES = ("field_patch", "customer_patch", "support_action")
GUIDE_CHECKLISTS = (
    ("doc/user_guide.md", "Boundaries and a practice checklist", "Tests"),
    ("doc/user_guide.zh_CN.md", "边界与实践清单", "测试"),
)
CHECKLIST_APIS = ("FieldSetFailure", "InvocationRecovery", "RegistryError")


def check_guide_test_checklists(root):
    """Require the bilingual boundary checklist to preserve test guidance."""
    root = Path(root)
    for name, heading, test_marker in GUIDE_CHECKLISTS:
        document = root / name
        lines = document.read_text(encoding="utf-8").splitlines()
        headings = [index for index, line in enumerate(lines) if line == f"## {heading}"]
        if len(headings) != 1:
            raise ValueError(f"{document}: expected exactly one '## {heading}' section")
        start = headings[0] + 1
        end = next(
            (index for index in range(start, len(lines)) if lines[index].startswith("## ")),
            len(lines),
        )
        items = [line[2:] for line in lines[start:end] if line.startswith("- ")]
        test_items = [item for item in items if test_marker in item]
        if not test_items:
            raise ValueError(f"{document}: checklist is missing a test item containing {test_marker!r}")
        missing = [api for api in CHECKLIST_APIS if not any(api in item for item in test_items)]
        if missing:
            raise ValueError(f"{document}: test checklist item is missing API names {missing}")


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


def check_local_links(root):
    """Reject broken and machine-local links in maintained Markdown documents."""
    names = (
        "README.md",
        "README.zh_CN.md",
        "derive/README.md",
        "derive/README.zh_CN.md",
        "doc/user_guide.md",
        "doc/user_guide.zh_CN.md",
    )
    root = Path(root).resolve()
    documents = [root / name for name in names]
    for document in documents:
        if not document.is_file():
            raise ValueError(f"missing maintained document: {document}")
        fence = None
        for line_number, line in enumerate(document.read_text(encoding="utf-8").splitlines(), 1):
            fence_match = re.match(r"^\s{0,3}(`{3,}|~{3,})", line)
            if fence_match:
                kind = fence_match.group(1)[0]
                if fence is None:
                    fence = kind
                elif fence == kind:
                    fence = None
                continue
            if fence is not None:
                continue
            for raw_target in re.findall(r"\]\(([^)\s]+)(?:\s+[^)]*)?\)", line):
                if raw_target.startswith("#"):
                    continue
                target_url = urlsplit(raw_target)
                if target_url.scheme in {"http", "https", "mailto"}:
                    continue
                relative_path = Path(unquote(target_url.path))
                target = (document.parent / relative_path).resolve()
                if (
                    relative_path.is_absolute()
                    or not target.is_relative_to(root)
                    or not target.exists()
                ):
                    raise ValueError(f"{document}:{line_number}: invalid local link: {raw_target}")


def check_contracts(root):
    """Validate the required keys, bilingual values, and actual source contracts."""
    root = Path(root)
    expected_values = dict(FACTS)
    check_examples(root)
    check_source_checkout_docs(root)
    check_guide_test_checklists(root)
    check_local_links(root)
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
