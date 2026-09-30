"""Independent QZSS LNAV numerical reference from one unmodified RINEX record.

Equations and constants: IS-QZSS-PNT-006 sections 5.3, 5.5 and 5.6.1.
RINEX slots: IGS RINEX 4.02 table A19. No rinex Rust code is imported.
"""
import datetime as dt
import json
import math
from pathlib import Path

SOURCE = Path(__file__).resolve().parents[1] / "fixtures/nav_qzss_j02_2023071.rnx"
GPS_EPOCH = dt.datetime(1980, 1, 6)
MU = 3.986005e14  # m^3/s^2; QZSS ICD section 5.3.4
EARTH_RATE = 7.2921151467e-5  # rad/s; section 5.3.3
C = 299792458.0  # m/s; section 5.3.1


def record():
    lines = SOURCE.read_text(encoding="ascii").splitlines()
    start = lines.index("> EPH J02 LNAV")
    first = lines[start + 1]
    toc = dt.datetime.strptime(first[4:23].strip(), "%Y %m %d %H %M %S")
    clock = [float(first[23 + 19 * k:42 + 19 * k]) for k in range(3)]
    fields = []
    for line in lines[start + 2:start + 9]:
        fields.extend(float(line[4 + 19 * k:23 + 19 * k])
                      if line[4 + 19 * k:23 + 19 * k].strip() else None
                      for k in range(4))
    assert len(fields) == 28
    return toc, clock, fields


def eccentric_anomaly(fields, offset):
    e, sqrt_a = fields[5], fields[7]
    a = sqrt_a * sqrt_a
    mean = fields[3] + (math.sqrt(MU / a**3) + fields[2]) * offset
    anomaly = mean
    for _ in range(30):
        delta = (anomaly - e * math.sin(anomaly) - mean) / (1 - e * math.cos(anomaly))
        anomaly -= delta
        if abs(delta) < 1e-14:
            return anomaly
    raise ValueError("Kepler equation did not converge")


def position_km(fields, offset):
    e, sqrt_a, toe = fields[5], fields[7], fields[8]
    a = sqrt_a * sqrt_a
    anomaly = eccentric_anomaly(fields, offset)
    true_anomaly = math.atan2(math.sqrt(1 - e * e) * math.sin(anomaly),
                              math.cos(anomaly) - e)
    phi = true_anomaly + fields[14]
    s2, c2 = math.sin(2 * phi), math.cos(2 * phi)
    latitude = phi + fields[6] * s2 + fields[4] * c2
    radius = a * (1 - e * math.cos(anomaly)) + fields[1] * s2 + fields[13] * c2
    inclination = fields[12] + fields[16] * offset + fields[11] * s2 + fields[9] * c2
    node = fields[10] + (fields[15] - EARTH_RATE) * offset - EARTH_RATE * toe
    x, y = radius * math.cos(latitude), radius * math.sin(latitude)
    cn, sn = math.cos(node), math.sin(node)
    ci, si = math.cos(inclination), math.sin(inclination)
    return [(x * cn - y * ci * sn) / 1000,
            (x * sn + y * ci * cn) / 1000,
            y * si / 1000]


def state(toc, clock, fields, offset):
    toe = GPS_EPOCH + dt.timedelta(weeks=int(fields[18]), seconds=fields[8])
    t = toe + dt.timedelta(seconds=offset)
    p = position_km(fields, offset)
    half_step = 0.05
    before = position_km(fields, offset - half_step)
    after = position_km(fields, offset + half_step)
    v = [(a - b) / (2 * half_step) for a, b in zip(after, before)]
    dt_clock = (t - toc).total_seconds()
    relativistic = -2 * math.sqrt(MU) / C**2 * fields[5] * fields[7] * math.sin(
        eccentric_anomaly(fields, offset))
    return {"offset_s": offset, "position_km": p, "velocity_km_s": v,
            "clock_s_without_group_delay": clock[0] + clock[1] * dt_clock
            + clock[2] * dt_clock**2 + relativistic}


def main():
    toc, clock, fields = record()
    assert int(fields[18]) == 2253 and fields[8] == 0.0
    print(json.dumps([state(toc, clock, fields, offset) for offset in (-1800, 0, 1800)],
                     indent=2) + "\n", end="")


if __name__ == "__main__":
    main()
