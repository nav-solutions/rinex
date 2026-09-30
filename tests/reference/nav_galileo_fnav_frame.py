"""Independent broadcast-position reference for the real E08 F/NAV record.

Read RINEX slots directly, use the existing Python Galileo Kepler calculation,
then apply the same declared frame operations as the I/NAV reference.
"""
import datetime as dt
import json
from pathlib import Path

from nav_galileo_frame import to_itrf2014
from nav_legacy import position


SOURCE = Path(__file__).resolve().parents[1] / "fixtures/nav_galileo_e08_fnav_2024128.rnx"
UTC_TO_GST_S = 18  # RINEX header LEAP SECONDS for this 2024 record


def record():
    lines = SOURCE.read_text().splitlines()
    start = lines.index("> EPH E08 FNAV")
    first = lines[start + 1]
    toc = dt.datetime.strptime(first[4:23].strip(), "%Y %m %d %H %M %S")
    fields = []
    for line in lines[start + 2:start + 9]:
        for slot in range(4):
            raw = line[4 + 19 * slot:23 + 19 * slot]
            fields.append(float(raw) if raw.strip() else None)
    assert toc == dt.datetime(2024, 5, 7, 0, 40)
    assert fields[8] == 175200.0
    return toc, fields


def main():
    toc, fields = record()
    rows = []
    for offset in (-300, 0, 300):
        native = position(fields, "E", offset)
        epoch_utc = toc + dt.timedelta(seconds=offset - UTC_TO_GST_S)
        rows.append({
            "offset_s": offset,
            "epoch_utc": epoch_utc.isoformat() + "Z",
            "native_position_km": native,
            "itrf2014_position_km": to_itrf2014(native, epoch_utc),
        })
    print(json.dumps(rows, indent=2))


if __name__ == "__main__":
    main()
