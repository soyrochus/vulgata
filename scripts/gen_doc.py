#!/usr/bin/env python3
"""Concatenate all markdown files in docs/ into a single versioned spec file."""

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).parent.parent
CARGO_TOML = REPO_ROOT / "Cargo.toml"
DOCS_DIR = REPO_ROOT / "docs"


def get_version() -> str:
    text = CARGO_TOML.read_text()
    match = re.search(r'^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
    if not match:
        print("Error: could not find version in Cargo.toml", file=sys.stderr)
        sys.exit(1)
    return match.group(1)


def main() -> None:
    version = get_version()
    output_path = DOCS_DIR / f"vulgata_spec_v{version}.md"

    md_files = sorted(
        f for f in DOCS_DIR.glob("*.md")
        if f.name != output_path.name
    )

    if not md_files:
        print("No markdown files found in docs/", file=sys.stderr)
        sys.exit(1)

    parts: list[str] = []
    for md_file in md_files:
        parts.append(md_file.read_text())

    output_path.write_text("\n\n".join(parts))
    print(f"Written: {output_path.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    main()
