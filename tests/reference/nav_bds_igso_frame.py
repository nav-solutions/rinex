"""Independent C10 D1 IGSO orbit and declared BDCS zero-offset reference."""
import datetime as dt
import json
from pathlib import Path

from nav_legacy import position


SOURCE = Path(__file__).resolve().parents[1] / "fixtures/nav_legacy_kms_2022159.rnx"


def record():
    lines = SOURCE.read_text().splitlines()
    start = lines.index("> EPH C10 D1")
    first = lines[start + 1]
    toc = dt.datetime.strptime(first[4:23].strip(), "%Y %m %d %H %M %S")
    fields = []
    for line in lines[start + 2:start + 9]:
        for slot in range(4):
            raw = line[4 + 19 * slot:23 + 19 * slot]
            fields.append(float(raw) if raw.strip() else None)
    assert toc == dt.datetime(2022, 6, 8, 7, 0)
    assert fields[8] == 284400.0
    return toc, fields


def main():
    toc, fields = record()
    rows = []
    for offset in (-300, 0, 300):
        native = position(fields, "C", offset)
        rows.append({
            "offset_s": offset,
            "epoch_bdt": (toc + dt.timedelta(seconds=offset)).isoformat() + " BDT",
            "native_position_km": native,
            # This is an explicitly unbounded zero-offset approximation.
            "itrf2014_position_km": native,
        })
    print(json.dumps(rows, indent=2))


if __name__ == "__main__":
    main()
