#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Check portable local links and concise development documents."""
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET
from urllib.parse import unquote, urlsplit

root = Path(__file__).resolve().parents[1]
paths = list(root.glob("*.md"))
for folder in ("docs", ".agents", ".agent-harness", ".github"):
    paths.extend((root / folder).rglob("*.md"))
errors = []
for path in paths:
    text = path.read_text()
    if len(text.splitlines()) > 100:
        errors.append(f"{path.relative_to(root)}: split development documentation into files of at most 100 lines")
    for target in re.findall(r"\[[^\]]*\]\(([^)]+)\)", text):
        target = target.strip("<>")
        if urlsplit(target).scheme or target.startswith("#"):
            continue
        destination = (path.parent / unquote(target.split("#")[0])).resolve()
        if not destination.is_relative_to(root) or not destination.exists():
            errors.append(f"{path.relative_to(root)}: broken/nonportable link {target}")
for path in (root / "docs").rglob("*.svg"):
    try:
        ET.parse(path)
    except ET.ParseError as error:
        errors.append(f"{path.relative_to(root)}: {error}")
if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
print(f"Documentation: {len(paths)} files checked; local links and SVGs are valid.")
