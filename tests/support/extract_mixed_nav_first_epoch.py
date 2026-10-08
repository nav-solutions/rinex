"""Rebuild the tracked NAV microfixture from the public DLR RINEX 4 product.

Usage: python3 tests/support/extract_mixed_nav_first_epoch.py ORIGINAL_NAV
Run from the repository root. The input is read-only; the tracked source
manifest fixes its hash, original header, record line spans, and output hash.
"""

import hashlib
import json
from pathlib import Path
import sys

MANIFEST = Path("tests/fixtures/nav_mixed_2024131_source.json")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main(source):
    meta = json.loads(MANIFEST.read_text())
    original = source.read_bytes()
    assert digest(original) == meta["source_sha256"], "unexpected DLR source"
    lines = original.splitlines(keepends=True)
    first, last = meta["source_header_lines"]
    assert b"END OF HEADER" in lines[last - 1]
    chunks = [b"".join(lines[first - 1:last])]
    for record in meta["records"]:
        first, last = record["source_lines"]
        block = b"".join(lines[first - 1:last])
        assert block.decode("ascii").splitlines()[0].strip() == record["descriptor"]
        assert digest(block) == record["sha256"]
        chunks.append(block)
    fixture = b"".join(chunks)
    assert digest(fixture) == meta["fixture_sha256"]
    target = MANIFEST.parent / meta["fixture"]
    target.write_bytes(fixture)
    print(f"{target}: {len(meta['records'])} NAV records, {len(fixture)} bytes")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: python3 tests/support/extract_mixed_nav_first_epoch.py ORIGINAL_NAV")
    main(Path(sys.argv[1]))
