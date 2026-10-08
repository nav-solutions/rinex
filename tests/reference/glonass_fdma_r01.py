#!/usr/bin/env python3
"""Independent, stdlib-only numeric reference for the public RINEX 4.02 R01 record.

The equations and constants follow RTKLIB ephemeris.c deq/glorbit (Takasu),
checked against IGS RINEX 4.02 Table A15 and ESA Navipedia's GLONASS model.
This is a separately executable reference, not a call into the Rust library.
Run: python3 tests/reference/glonass_fdma_r01.py
"""
import hashlib
import json
from pathlib import Path

SOURCE = Path('data/NAV/V4/rinex402_examples_MN.rnx')
raw = SOURCE.read_bytes()
lines = raw.decode().splitlines()
i = lines.index('> EPH R01 FDMA')
clock = [float(lines[i + 1][23 + 19*j:42 + 19*j]) for j in range(3)]
rows = [[float(lines[i + 2 + axis][4 + 19*j:23 + 19*j]) for j in range(4)] for axis in range(3)]
state0 = [row[0] for row in rows] + [row[1] for row in rows]
acc = [row[2] for row in rows]
mu, ae, j2, omega = 398600.44, 6378.136, 1.0826257e-3, 7.292115e-5

def rate(q):
    x, y, z, vx, vy, vz = q
    rr = x*x + y*y + z*z
    r3 = rr * rr**0.5
    j = 1.5*j2*mu*ae*ae/(rr*r3)
    k = -mu/r3 - j*(1 - 5*z*z/rr)
    return [vx, vy, vz,
            (k+omega*omega)*x + 2*omega*vy + acc[0],
            (k+omega*omega)*y - 2*omega*vx + acc[1],
            (k-2*j)*z + acc[2]]

def plus(q, v, h):
    return [a + h*b for a, b in zip(q, v)]

def propagate(seconds):
    q = state0[:]
    remaining = seconds
    while abs(remaining) > 1e-10:
        h = min(60., abs(remaining)) * (1 if remaining > 0 else -1)
        k1 = rate(q)
        k2 = rate(plus(q, k1, h/2))
        k3 = rate(plus(q, k2, h/2))
        k4 = rate(plus(q, k3, h))
        q = [v + h*(a + 2*b + 2*c + d)/6 for v, a, b, c, d in zip(q, k1, k2, k3, k4)]
        remaining -= h
    return q

out = {
    'input_sha256': hashlib.sha256(raw).hexdigest(),
    'sv': 'R01', 'epoch_utc': '2020-09-15T23:45:00 UTC',
    'state_km_km_s': {str(dt): [float(f'{v:.12f}') for v in propagate(dt)]
                      for dt in (0, 60, -60, 300, -300, 899.5, -899.5)},
    'clock_raw': clock, 'acceleration_km_s2': acc,
}
print(json.dumps(out, indent=2, sort_keys=True))
