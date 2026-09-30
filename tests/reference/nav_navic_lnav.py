"""Independent NavIC LNAV reference from an unmodified RINEX 4 record.

IGS RINEX 4.02 table A30 maps the fields. ISRO NavIC SPS ICD 1.1
appendices A and B define the clock and Earth-fixed orbit equations.
"""
import datetime as dt
import json
import math
from pathlib import Path

SOURCE = Path(__file__).resolve().parents[1] / "fixtures/nav_navic_i02_2023071.rnx"
GPS_EPOCH = dt.datetime(1980, 1, 6)
MU = 3.986005e14  # m^3/s^2, NavIC SPS ICD 1.1 Appendix B
EARTH_RATE = 7.2921151467e-5  # rad/s, Appendix B
C = 299792458.0  # m/s


def record():
    lines = SOURCE.read_text(encoding="ascii").splitlines()
    start = lines.index("> EPH I02 LNAV")
    first = lines[start + 1]
    toc = dt.datetime.strptime(first[4:23].strip(), "%Y %m %d %H %M %S")
    clock = [float(first[23 + 19 * k:42 + 19 * k]) for k in range(3)]
    fields = []
    for line in lines[start + 2:start + 9]:
        fields.extend(float(slot) if slot.strip() else None
                      for slot in (line[4 + 19 * k:23 + 19 * k] for k in range(4)))
    assert len(fields) == 28
    return toc, clock, fields


def anomaly(fields, elapsed):
    e, sqrt_a = fields[5], fields[7]
    a = sqrt_a * sqrt_a
    mean = fields[3] + (math.sqrt(MU / a**3) + fields[2]) * elapsed
    eccentric = mean
    for _ in range(30):
        step = (eccentric - e * math.sin(eccentric) - mean) / (1 - e * math.cos(eccentric))
        eccentric -= step
        if abs(step) < 1e-14:
            return eccentric
    raise ValueError("eccentric anomaly did not converge")


def position_km(fields, elapsed):
    e, sqrt_a, toe = fields[5], fields[7], fields[8]
    eccentric = anomaly(fields, elapsed)
    true = math.atan2(math.sqrt(1 - e * e) * math.sin(eccentric),
                      math.cos(eccentric) - e)
    phi = true + fields[14]
    sin2, cos2 = math.sin(2 * phi), math.cos(2 * phi)
    u = phi + fields[6] * sin2 + fields[4] * cos2
    radius = sqrt_a**2 * (1 - e * math.cos(eccentric)) + fields[1] * sin2 + fields[13] * cos2
    inc = fields[12] + fields[16] * elapsed + fields[11] * sin2 + fields[9] * cos2
    node = fields[10] + (fields[15] - EARTH_RATE) * elapsed - EARTH_RATE * toe
    x, y = radius * math.cos(u), radius * math.sin(u)
    return [(x * math.cos(node) - y * math.cos(inc) * math.sin(node)) / 1000,
            (x * math.sin(node) + y * math.cos(inc) * math.cos(node)) / 1000,
            y * math.sin(inc) / 1000]


def state(toc, clock, fields, elapsed):
    # RINEX A30 stores the continuous GPS-aligned week. GPST is the same
    # proxy used by the crate; an IRNSST realization offset is not modeled.
    toe = GPS_EPOCH + dt.timedelta(weeks=int(fields[18]), seconds=fields[8])
    target = toe + dt.timedelta(seconds=elapsed)
    half_step = 0.05
    before = position_km(fields, elapsed - half_step)
    after = position_km(fields, elapsed + half_step)
    velocity = [(a - b) / (2 * half_step) for a, b in zip(after, before)]
    eccentric = anomaly(fields, elapsed)
    relativistic = (-2 * math.sqrt(MU) / C**2) * fields[5] * fields[7] * math.sin(eccentric)
    dt_clock = (target - toc).total_seconds()
    return {"offset_s": elapsed, "position_km": position_km(fields, elapsed),
            "velocity_km_s": velocity,
            "clock_s_without_group_delay": clock[0] + clock[1] * dt_clock
            + clock[2] * dt_clock**2 + relativistic}


def main():
    toc, clock, fields = record()
    assert fields[8] == 0.0 and fields[18] == 2253.0 and fields[21] == 0.0
    print(json.dumps([state(toc, clock, fields, elapsed) for elapsed in (-1800, 0, 1800)],
                     indent=2) + "\n", end="")


if __name__ == "__main__":
    main()
