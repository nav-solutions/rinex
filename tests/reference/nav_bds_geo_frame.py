"""Independent C05 D2 GEO orbit and declared BDCS zero-offset reference."""
import datetime as dt
import json
from pathlib import Path

from nav_bds_geo import fields, position


SOURCE = Path(__file__).resolve().parents[1] / "fixtures/nav_bds_geo_c05_2022159.rnx"


def main():
    lines = SOURCE.read_text().splitlines()
    start = lines.index("> EPH C05 D2")
    toc = dt.datetime.strptime(lines[start + 1][4:23].strip(), "%Y %m %d %H %M %S")
    orbit = fields()
    assert toc == dt.datetime(2022, 6, 8, 9, 0)
    assert orbit[8] == 291600.0
    rows = []
    for offset in (-300, 0, 300):
        native = position(orbit, offset)
        rows.append({
            "offset_s": offset,
            "epoch_bdt": (toc + dt.timedelta(seconds=offset)).isoformat() + " BDT",
            "native_position_km": native,
            # Explicitly unbounded zero-offset approximation.
            "itrf2014_position_km": native,
        })
    print(json.dumps(rows, indent=2))


if __name__ == "__main__":
    main()
