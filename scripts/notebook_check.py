#!/usr/bin/env python3
"""Check that SECOND-ORDER.md can be read the way the procedure reads it.

The notebook is read by the person, not by a test, so this checks its form
and nothing about its content:

- every row in the notebook's table has a number, a kind (divergence,
  finding or worklist) and a status (done, open or person);
- no number is used twice, so a commit or a triage answer that cites one
  means one row;
- every row whose status is *person* is named in HANDOFF.md, so an item the
  person must do reaches the file they are told to read.

A row whose cells are all empty after the number is the template's blank
row and is skipped.

    python3 scripts/notebook_check.py
"""

import re
import sys
from pathlib import Path

KINDS = {"divergence", "finding", "worklist"}
STATUSES = {"done", "open", "person"}


def cells(line):
    return [c.strip() for c in line.strip().strip("|").split("|")]


def rows(text):
    """Yield (number, kind, status) for each row of the notebook's table."""
    header = None
    for line in text.splitlines():
        if not line.lstrip().startswith("|"):
            header = None
            continue
        c = cells(line)
        if header is None:
            if "Kind" in c and "Status" in c:
                header = c
            continue
        if set("".join(c)) <= set("-: "):
            continue
        c += [""] * (len(header) - len(c))
        if not any(c[1:]):
            continue
        yield (
            c[0],
            c[header.index("Kind")].strip("*_ ").lower(),
            c[header.index("Status")].strip("*_ ").lower(),
        )


def main(root):
    root = Path(root)
    notebook = (root / "SECOND-ORDER.md").read_text()
    handoff = (root / "HANDOFF.md").read_text()
    problems, seen = [], set()
    for number, kind, status in rows(notebook):
        name = number or "a row with no number"
        if not number:
            problems.append("SECOND-ORDER.md: a row has no number")
        elif number in seen:
            problems.append(f"SECOND-ORDER.md: {number} is used for two rows")
        seen.add(number)
        if kind not in KINDS:
            problems.append(f"SECOND-ORDER.md: {name} has kind {kind!r}; use divergence, finding or worklist")
        if status not in STATUSES:
            problems.append(f"SECOND-ORDER.md: {name} has status {status!r}; use done, open or person")
        if status == "person" and number and not re.search(rf"(?<![\w.]){re.escape(number)}(?![\w]|\.\d)", handoff):
            problems.append(f"SECOND-ORDER.md: {number} is the person's to do and HANDOFF.md does not name it")
    for p in problems:
        print(f"FAIL {p}")
    if problems:
        return 1
    print(f"notebook: {len(seen)} rows, each with a kind and a status; person rows named in HANDOFF.md")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent))
