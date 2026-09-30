"""Independent E08 INAV orbit and marked GTRF23v01 -> ITRF2014 reference.

The RINEX orbit fields are read directly. Broadcast propagation reuses the
existing Python implementation of the published Kepler equations, not Rust.
"""
import datetime as dt
import json
from pathlib import Path

from nav_legacy import position


SOURCE = Path(__file__).resolve().parents[1] / "fixtures/nav_galileo_e08_inav_2024128.rnx"
UTC_TO_GST_S = 18  # RINEX header LEAP SECONDS for this 2024 record


def record():
    lines = SOURCE.read_text().splitlines()
    start = lines.index("> EPH E08 INAV")
    first = lines[start + 1]
    toc = dt.datetime.strptime(first[4:23].strip(), "%Y %m %d %H %M %S")
    fields = []
    for line in lines[start + 2:start + 9]:
        for slot in range(4):
            raw = line[4 + 19 * slot:23 + 19 * slot]
            fields.append(float(raw) if raw.strip() else None)
    assert toc == dt.datetime(2024, 5, 7, 0, 30)
    assert fields[8] == 174600.0
    return toc, fields


def to_itrf2014(native_km, epoch_utc):
    # ITRF2020 Table 2: ITRF2014 - ITRF2020 at 2015.0, rates per Julian year.
    years = (epoch_utc - dt.datetime(2015, 1, 1)).total_seconds() / 31_557_600
    translation_mm = [-1.4, -0.9 - 0.1 * years, 1.4 + 0.2 * years]
    scale = 1.0 - 0.42e-9
    # First edge is explicitly a zero-offset, unbounded GTRF approximation.
    return [scale * xyz + shift * 1e-6
            for xyz, shift in zip(native_km, translation_mm)]


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
