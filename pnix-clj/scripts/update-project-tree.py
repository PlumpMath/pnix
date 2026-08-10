#!/usr/bin/env python3
from __future__ import annotations

import datetime as dt
import json
import pathlib
import re
import subprocess
import sys

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    tomllib = None


ROOT = pathlib.Path(__file__).resolve().parent.parent
DOC = ROOT / "project-tree.md"

TOP_LEVEL_ORDER = [
    "Cargo.toml",
    "Cargo.lock",
    "README.md",
    "CHANGELOG.md",
    "LICENSE",
    "SECURITY.md",
    "CONTRIBUTING.md",
    "executor.md",
    "pnix-core.md",
    "prd.md",
    "todo.md",
    "todo-3d.md",
    "plan.md",
    "roadmap.md",
    "docker-compose.yml",
    "flake.nix",
    "flake.lock",
    "rustfmt.toml",
    "backends/",
    "crates/",
    "docs/",
    "examples/",
    "fixtures/",
    "scripts/",
    "schema/",
    "stdlib/",
    "editors/",
    "demo/",
    "fuzz/",
    "dist/",
    "target/",
    "tmp/",
]

TOP_LEVEL_COMMENTS = {
    "dist/": "generated artifacts / test outputs",
    "target/": "cargo build outputs",
    "tmp/": "local scratch",
}

DOC_ENTRYPOINTS = [
    "docs/index.md",
    "docs/file-layout.md",
    "docs/architecture.md",
    "docs/cli.md",
    "docs/runtime-overview.md",
    "docs/legacy-vm-loop.md",
    "docs/ai-collaboration-workflow.md",
    "docs/migration-status.md",
]


def load_toml(path: pathlib.Path) -> dict:
    if tomllib is None:
        raise RuntimeError(
            "tomllib is unavailable; install Python 3.11+ or ensure cargo metadata path is usable"
        )
    with path.open("rb") as handle:
        return tomllib.load(handle)


def load_workspace_metadata() -> tuple[list[str], dict[str, str]] | None:
    try:
        raw = subprocess.check_output(
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            cwd=ROOT,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None

    meta = json.loads(raw)
    package_by_id = {pkg["id"]: pkg for pkg in meta.get("packages", [])}
    members: list[str] = []
    descriptions: dict[str, str] = {}

    for pkg_id in meta.get("workspace_members", []):
        pkg = package_by_id.get(pkg_id)
        if not pkg:
            continue
        manifest = pathlib.Path(pkg.get("manifest_path", ""))
        if not manifest:
            continue
        try:
            member = str(manifest.parent.relative_to(ROOT))
        except ValueError:
            continue
        members.append(member)
        descriptions[member] = pkg.get("description") or "no description set in Cargo.toml"

    return members, descriptions


def replace_block(text: str, name: str, new_block: str) -> str:
    start = f"<!-- AUTO:{name}:BEGIN -->"
    end = f"<!-- AUTO:{name}:END -->"
    pattern = re.compile(re.escape(start) + r"(.*?)" + re.escape(end), re.S)
    if not pattern.search(text):
        raise SystemExit(f"Missing markers for {name}")
    return pattern.sub(start + "\n" + new_block.rstrip() + "\n" + end, text, count=1)


def upsert_optional_section(
    text: str,
    name: str,
    title: str,
    new_block: str,
    after_marker: str,
) -> str:
    start = f"<!-- AUTO:{name}:BEGIN -->"
    end = f"<!-- AUTO:{name}:END -->"
    pattern = re.compile(re.escape(start) + r"(.*?)" + re.escape(end), re.S)
    block_exists = pattern.search(text) is not None

    if not new_block.strip():
        if block_exists:
            section_pattern = re.compile(
                rf"\n## {re.escape(title)}\n\n{re.escape(start)}.*?{re.escape(end)}\n",
                re.S,
            )
            text, count = section_pattern.subn("\n", text, count=1)
            if count == 0:
                text = pattern.sub("", text, count=1)
        return text

    if block_exists:
        return replace_block(text, name, new_block)

    insert_after = f"<!-- AUTO:{after_marker}:END -->"
    section = f"\n\n## {title}\n\n{start}\n{new_block.rstrip()}\n{end}\n"
    if insert_after in text:
        return text.replace(insert_after, insert_after + section, 1)

    return text.rstrip() + section + "\n"


def upsert_optional_section_before_heading(
    text: str,
    name: str,
    title: str,
    new_block: str,
    heading: str,
) -> str:
    start = f"<!-- AUTO:{name}:BEGIN -->"
    end = f"<!-- AUTO:{name}:END -->"
    pattern = re.compile(re.escape(start) + r"(.*?)" + re.escape(end), re.S)
    block_exists = pattern.search(text) is not None

    if not new_block.strip():
        if block_exists:
            section_pattern = re.compile(
                rf"\n## {re.escape(title)}\n\n{re.escape(start)}.*?{re.escape(end)}\n",
                re.S,
            )
            text, count = section_pattern.subn("\n", text, count=1)
            if count == 0:
                text = pattern.sub("", text, count=1)
        return text

    if block_exists:
        return replace_block(text, name, new_block)

    section = f"\n\n## {title}\n\n{start}\n{new_block.rstrip()}\n{end}\n"
    heading_marker = f"\n## {heading}\n"
    if heading_marker in text:
        return text.replace(heading_marker, section + heading_marker, 1)

    return text.rstrip() + section + "\n"


def format_tree(root_label: str, entries: list[str]) -> str:
    lines = [f"{root_label}/"]
    for idx, entry in enumerate(entries):
        connector = "└──" if idx == len(entries) - 1 else "├──"
        lines.append(f"{connector} {entry}")
    return "```\n" + "\n".join(lines) + "\n```"


def list_entries(path: pathlib.Path, kind: str) -> list[str]:
    if not path.exists():
        return []
    entries = []
    for child in sorted(path.iterdir(), key=lambda p: p.name):
        if child.name.startswith("."):
            continue
        if kind == "dirs" and not child.is_dir():
            continue
        if kind == "files" and not child.is_file():
            continue
        name = child.name + ("/" if child.is_dir() else "")
        entries.append(name)
    return entries


def rel_path(path: pathlib.Path) -> str:
    return path.relative_to(ROOT).as_posix()


def bullet_list(paths: list[pathlib.Path]) -> str:
    return "\n".join(f"- `{rel_path(path)}`" for path in paths)


def parse_args(argv: list[str]) -> bool:
    check_only = False
    for arg in argv[1:]:
        if arg == "--check":
            check_only = True
        elif arg == "--write":
            check_only = False
        else:
            print("Usage: scripts/update-project-tree.py [--check|--write]", file=sys.stderr)
            raise SystemExit(2)
    return check_only


def extract_updated_date(text: str) -> str | None:
    match = re.search(r"^> Updated: (.*)$", text, re.M)
    if not match:
        return None
    return match.group(1).strip()


def build_updated_text(text: str, updated_date: str) -> str:
    text = re.sub(r"^> Updated: .*", f"> Updated: {updated_date}", text, flags=re.M)

    top_level_entries = []
    for entry in TOP_LEVEL_ORDER:
        path = ROOT / entry.rstrip("/")
        if entry.endswith("/"):
            if path.is_dir():
                top_level_entries.append(entry)
        else:
            if path.is_file():
                top_level_entries.append(entry)

    formatted_entries = []
    for entry in top_level_entries:
        comment = TOP_LEVEL_COMMENTS.get(entry)
        if comment:
            formatted_entries.append(f"{entry:<26} # {comment}")
        else:
            formatted_entries.append(entry)

    top_level_block = format_tree("pnix", formatted_entries)

    backends_block = format_tree(
        "backends", list_entries(ROOT / "backends", "dirs")
    )
    examples_block = format_tree(
        "examples", list_entries(ROOT / "examples", "all")
    )
    fixtures_block = format_tree(
        "fixtures", list_entries(ROOT / "fixtures", "dirs")
    )
    schemas_block = format_tree(
        "schema", list_entries(ROOT / "schema", "files")
    )
    stdlib_block = format_tree(
        "stdlib", list_entries(ROOT / "stdlib", "all")
    )
    editors_block = format_tree(
        "editors", list_entries(ROOT / "editors", "dirs")
    )

    workspace_meta = load_workspace_metadata()
    if workspace_meta is not None:
        members, descriptions = workspace_meta
    else:
        workspace = load_toml(ROOT / "Cargo.toml").get("workspace", {})
        members = workspace.get("members", [])
        descriptions = {}
    crate_lines = []
    for member in members:
        cargo_path = ROOT / member / "Cargo.toml"
        if not cargo_path.exists():
            desc = "missing Cargo.toml"
        elif member in descriptions:
            desc = descriptions[member]
        else:
            cargo = load_toml(cargo_path)
            desc = cargo.get("package", {}).get("description") or "no description set in Cargo.toml"
        crate_lines.append(f"- `{member}`: {desc}")
    crates_block = "\n".join(crate_lines)

    doc_lines = []
    for path in DOC_ENTRYPOINTS:
        if (ROOT / path).is_file():
            doc_lines.append(f"- `{path}`")
    doc_block = "\n".join(doc_lines)

    docs_all = sorted((ROOT / "docs").glob("*.md"))
    docs_all_block = bullet_list(docs_all)

    docs_adr_dir = ROOT / "docs" / "adr"
    docs_adr = []
    if docs_adr_dir.is_dir():
        docs_adr = sorted(docs_adr_dir.glob("*.md"))
    docs_adr_block = bullet_list(docs_adr)

    docs_tutorials_dir = ROOT / "docs" / "tutorials"
    docs_tutorials = []
    if docs_tutorials_dir.is_dir():
        docs_tutorials = sorted(docs_tutorials_dir.glob("*.md"))
    docs_tutorials_block = bullet_list(docs_tutorials)

    fixtures_second_lines = []
    fixtures_root = ROOT / "fixtures"
    if fixtures_root.is_dir():
        for top_dir in sorted([p for p in fixtures_root.iterdir() if p.is_dir()], key=lambda p: p.name):
            subdirs = sorted([p for p in top_dir.iterdir() if p.is_dir()], key=lambda p: p.name)
            if not subdirs:
                continue
            sub_paths = ", ".join(f"`{rel_path(subdir)}/`" for subdir in subdirs)
            fixtures_second_lines.append(f"- `{rel_path(top_dir)}/`: {sub_paths}")
    fixtures_second_block = "\n".join(fixtures_second_lines)

    fixtures_third_lines = []
    if fixtures_root.is_dir():
        for top_dir in sorted([p for p in fixtures_root.iterdir() if p.is_dir()], key=lambda p: p.name):
            second_dirs = sorted([p for p in top_dir.iterdir() if p.is_dir()], key=lambda p: p.name)
            for second_dir in second_dirs:
                third_dirs = sorted([p for p in second_dir.iterdir() if p.is_dir()], key=lambda p: p.name)
                if not third_dirs:
                    continue
                third_paths = ", ".join(f"`{rel_path(third_dir)}/`" for third_dir in third_dirs)
                fixtures_third_lines.append(f"- `{rel_path(second_dir)}/`: {third_paths}")
    fixtures_third_block = "\n".join(fixtures_third_lines)

    fixtures_fourth_lines = []
    if fixtures_root.is_dir():
        for top_dir in sorted([p for p in fixtures_root.iterdir() if p.is_dir()], key=lambda p: p.name):
            second_dirs = sorted([p for p in top_dir.iterdir() if p.is_dir()], key=lambda p: p.name)
            for second_dir in second_dirs:
                third_dirs = sorted([p for p in second_dir.iterdir() if p.is_dir()], key=lambda p: p.name)
                for third_dir in third_dirs:
                    fourth_dirs = sorted([p for p in third_dir.iterdir() if p.is_dir()], key=lambda p: p.name)
                    if not fourth_dirs:
                        continue
                    fourth_paths = ", ".join(f"`{rel_path(fourth_dir)}/`" for fourth_dir in fourth_dirs)
                    fixtures_fourth_lines.append(f"- `{rel_path(third_dir)}/`: {fourth_paths}")
    fixtures_fourth_block = "\n".join(fixtures_fourth_lines)

    text = replace_block(text, "TOP_LEVEL", top_level_block)
    text = replace_block(text, "BACKENDS", backends_block)
    text = replace_block(text, "WORKSPACE_CRATES", crates_block)
    text = replace_block(text, "EXAMPLES", examples_block)
    text = replace_block(text, "FIXTURES", fixtures_block)
    text = replace_block(text, "SCHEMAS", schemas_block)
    text = replace_block(text, "STDLIB", stdlib_block)
    text = replace_block(text, "EDITORS", editors_block)
    text = replace_block(text, "DOC_ENTRYPOINTS", doc_block)
    text = replace_block(text, "DOCS_ALL", docs_all_block)
    text = replace_block(text, "DOCS_ADR", docs_adr_block)
    text = replace_block(text, "DOCS_TUTORIALS", docs_tutorials_block)
    text = replace_block(text, "FIXTURES_SECOND", fixtures_second_block)
    text = replace_block(text, "FIXTURES_THIRD", fixtures_third_block)
    text = upsert_optional_section_before_heading(
        text,
        "FIXTURES_FOURTH",
        "Fixtures (Fourth Level)",
        fixtures_fourth_block,
        "Known Issues & TODOs",
    )

    return text


def main() -> int:
    if not DOC.exists():
        print("project-tree.md not found", file=sys.stderr)
        return 1

    check_only = parse_args(sys.argv)
    original_text = DOC.read_text(encoding="utf-8")

    if check_only:
        updated_date = extract_updated_date(original_text) or dt.date.today().isoformat()
    else:
        updated_date = dt.date.today().isoformat()

    updated_text = build_updated_text(original_text, updated_date)

    if check_only:
        if updated_text != original_text:
            print("project-tree.md is out of date. Run: python3 scripts/update-project-tree.py", file=sys.stderr)
            return 1
        return 0

    DOC.write_text(updated_text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
