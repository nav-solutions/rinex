"""Independent E03 raw-RINEX orbit reference for the mixed first epoch.

Reuses the existing Python implementation of the broadcast Kepler equations;
no Rust result is read. Both declared frame edges to G2296 have zero numeric
parameters, so the approximate target XYZ equals this native XYZ.
"""
from pathlib import Path
from hashlib import sha256
import json
from nav_legacy import position

SOURCE = Path(__file__).resolve().parents[1] / "fixtures/nav_mixed_2024131_first_epoch.rnx"


def main():
    raw = SOURCE.read_bytes()
    manifest = json.loads((SOURCE.parent / "nav_mixed_2024131_source.json").read_text())
    assert sha256(raw).hexdigest() == manifest["fixture_sha256"]
    lines = raw.decode("ascii").splitlines()
    matches = [i for i, line in enumerate(lines)
               if line == "> EPH E03 INAV" and "02 50 00" in lines[i + 1]]
    assert len(matches) == 1
    i = matches[0]
    fields = []
    for row in lines[i + 2:i + 9]:
        for slot in range(4):
            raw = row[4 + 19 * slot:23 + 19 * slot]
            fields.append(float(raw) if raw.strip() else None)
    assert fields[8] == 442200.0  # Friday 02:50 GST
    # Query 03:00 GPST: Galileo and GPS nominal seconds agree here.
    source_record = next(row for row in manifest["records"] if row["descriptor"] == "> EPH E03 INAV")
    assert source_record["source_lines"][0] == 870
    print(f"DLR source line={source_record['source_lines'][0]} toe_sow={fields[8]:.0f} offset_s=600")
    print("native_and_approx_g2296_km=", position(fields, "E", 600))


if __name__ == "__main__":
    main()
