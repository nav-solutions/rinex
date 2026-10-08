//! Message-aware selection across supported broadcast orbit families.
use super::selection::{
    NativeFrame, NavCandidate, NavRejection, NavSelection, UnknownHealthPolicy,
};
use crate::{
    navigation::{Ephemeris, NavKey, NavMessageType},
    prelude::{Constellation, Duration, Epoch, Rinex, SV},
};
fn supported(key: &NavKey, major: u8) -> Option<(f64, NativeFrame, u8)> {
    use Constellation::*;
    use NavMessageType::*;
    if key.subtype.is_some() {
        return None;
    }
    if key.sv.constellation.is_sbas() {
        // RINEX 4 uses SBAS EPH, while a legacy LNAV key can carry the
        // same GEO ephemeris fields after ingestion or a version transition.
        return matches!(key.msgtype, NavMessageType::SBAS | LNAV).then_some((
            360.0,
            NativeFrame::SbasBroadcast,
            0,
        ));
    }
    let version_ok = match (key.sv.constellation, key.msgtype) {
        (Glonass, FDMA) | (Galileo, INAV | FNAV) | (BeiDou, D1 | D2) => major >= 4,
        (Glonass, LNAV) => major < 4,
        (QZSS | IRNSS, LNAV) => major >= 3,
        _ => true,
    };
    if !version_ok {
        return None;
    }
    Some(match (key.sv.constellation, key.msgtype) {
        (GPS, LNAV) => (7200.0, NativeFrame::GpsBroadcastWgs84, 0),
        // IS-QZSS-PNT-006 Table 4.1.1-2: 2 h total validity, |t-toe| < 1 h.
        (QZSS, LNAV) => (3600.0, NativeFrame::QzssBroadcastJgs, 0),
        // NavIC ICD 1.1 has nominal 2 h IODEC updates but no fit flag in
        // RINEX A30. Use the existing NavIC 2 h bound from validity_duration
        // as a selection policy, not an ICD-certified fit interval.
        (IRNSS, LNAV) => (7200.0, NativeFrame::NavicBroadcastWgs84, 0),
        (Galileo, INAV) => (10800.0, NativeFrame::GalileoBroadcastGtrf, 0),
        (Galileo, FNAV) => (10800.0, NativeFrame::GalileoBroadcastGtrf, 1),
        (BeiDou, D1) if !key.sv.is_beidou_geo() => {
            (21600.0, NativeFrame::BeiDouBroadcastCgcs2000, 0)
        },
        (BeiDou, D2) if key.sv.is_beidou_geo() => {
            (21600.0, NativeFrame::BeiDouBroadcastCgcs2000, 0)
        },
        // Conservative |t - t_b| < 15 min integration interval (ESA TM-23 I).
        (Glonass, FDMA) => (900.0, NativeFrame::GlonassBroadcastPz90, 0),
        (Glonass, LNAV) => (900.0, NativeFrame::GlonassBroadcastPz90, 1),
        _ => return None,
    })
}

fn orbit_fields_present(key: &NavKey, eph: &Ephemeris) -> bool {
    let fields: &[&str] = if key.sv.constellation == Constellation::Glonass
        || key.sv.constellation.is_sbas()
    {
        &[
            "satPosX", "satPosY", "satPosZ", "velX", "velY", "velZ", "accelX", "accelY", "accelZ",
        ]
    } else {
        &["sqrta", "e", "i0", "omega", "omega0", "m0", "toe", "week"]
    };
    fields
        .iter()
        .all(|f| eph.get_orbit_f64(f).is_some_and(f64::is_finite))
}

fn health(key: &NavKey, eph: &Ephemeris) -> Option<bool> {
    use crate::navigation::{bds::BdsSatH1, glonass::GlonassHealth};
    let item = eph.orbits.get("health")?;
    if key.sv.constellation.is_sbas() {
        let bits = item.as_geo_health_flag()?.bits();
        // MT17 status unavailable is unknown; URA=15 and any nonzero MT17
        // health bits exclude this record from a usable state.
        return if bits & 0x20 != 0 {
            Some(false)
        } else if bits & 0x10 != 0 {
            None
        } else {
            Some(bits == 0)
        };
    }
    match key.sv.constellation {
        // Without a requested signal, only the all-healthy broadcast code is usable.
        Constellation::GPS | Constellation::QZSS => {
            item.as_gps_qzss_l1l2l5_health_flag().map(|f| f.healthy())
        },
        Constellation::Glonass => item
            .as_glonass_health_flag()
            .map(|f| !f.intersects(GlonassHealth::UNHEALTHY)),
        Constellation::BeiDou => item
            .as_bds_sat_h1_flag()
            .map(|f| !f.intersects(BdsSatH1::UNHEALTHY)),
        Constellation::Galileo => item.as_galileo_health_flag().map(|f| f.is_empty()),
        // RINEX A30: 0 means both L5 and S healthy. No signal was requested.
        Constellation::IRNSS if key.msgtype == NavMessageType::LNAV => {
            item.as_irnss_health_flag().map(|f| f.is_empty())
        },
        _ => None,
    }
}

impl Rinex {
    /// Offline selection: closest orbit reference, then message priority, then full NavKey.
    /// Future references are allowed within the validity window. Unknown health is
    /// rejected unless explicitly allowed; unhealthy and invalid data always reject.
    pub fn nav_select_ephemeris(
        &self,
        sv: SV,
        t: Epoch,
        unknown: UnknownHealthPolicy,
    ) -> NavSelection<'_> {
        let mut candidates = Vec::new();
        let mut best: Option<(usize, f64, u8, NavKey)> = None;
        for (key, eph) in self.nav_ephemeris_frames_iter().filter(|(k, _)| k.sv == sv) {
            let support = supported(key, self.header.version.major);
            let reference = if support.is_some() {
                if key.sv.constellation == Constellation::Glonass || key.sv.constellation.is_sbas()
                {
                    Some(key.epoch)
                } else {
                    eph.toe(key.sv)
                }
            } else {
                None
            };
            let half = support.map(|(seconds, _, _)| Duration::from_seconds(seconds));
            let status = health(key, eph);
            let mut rejection = if support.is_none() {
                Some(NavRejection::UnsupportedMessage)
            } else if reference.is_none() {
                Some(NavRejection::MissingOrbitReference)
            } else if !orbit_fields_present(key, eph) {
                Some(NavRejection::MissingOrbitField)
            } else if status == Some(false) {
                Some(NavRejection::Unhealthy)
            } else if status.is_none() && unknown == UnknownHealthPolicy::Reject {
                Some(NavRejection::UnknownHealth)
            } else if eph.get_orbit_f64("dataValidity").is_some_and(|v| v != 0.0) {
                Some(NavRejection::InvalidData)
            } else if key.sv.constellation == Constellation::QZSS
                && eph.get_orbit_f64("fitInt").is_some_and(|v| v != 0.0)
            {
                // QZSS LNAV declares a fixed 2 h fit interval. A conflicting
                // nonzero flag cannot silently widen the selection window.
                Some(NavRejection::InvalidData)
            } else if (t - reference.unwrap()).abs() >= half.unwrap() {
                Some(NavRejection::OutOfValidity)
            } else if !matches!(key.sv.constellation, Constellation::Glonass)
                && !key.sv.constellation.is_sbas()
                && (t - key.epoch).abs() >= half.unwrap()
            {
                // Each supported Kepler message uses its own orbit policy
                // window for both ToE and ToC; these are separate references.
                Some(NavRejection::OutOfValidity)
            } else {
                None
            };
            let index = candidates.len();
            let mut candidate = NavCandidate {
                key,
                ephemeris: eph,
                orbit_reference: reference,
                clock_reference: key.epoch,
                validity_half_window: half,
                native_frame: support.map(|(_, frame, _)| frame),
                health: status,
                rejection,
            };
            if rejection.is_none() {
                let propagatable = if key.sv.constellation == Constellation::Glonass {
                    candidate.fdma_state_at(t).is_ok()
                } else if key.sv.constellation.is_sbas() {
                    candidate.sbas_state_at(t).is_ok()
                } else {
                    candidate.kepler_state_at(t).is_ok()
                };
                if !propagatable {
                    rejection = Some(NavRejection::Unpropagatable);
                    candidate.rejection = rejection;
                }
            }
            candidates.push(candidate);
            if rejection.is_none() {
                let rank = (
                    (t - reference.unwrap()).abs().to_seconds(),
                    support.unwrap().2,
                    *key,
                );
                if best.as_ref().is_none_or(|(_, d, p, k)| rank < (*d, *p, *k)) {
                    best = Some((index, rank.0, rank.1, rank.2));
                }
            }
        }
        let selected = best.map(|v| v.0);
        for (index, candidate) in candidates.iter_mut().enumerate() {
            if candidate.rejection.is_none() && selected != Some(index) {
                candidate.rejection = Some(NavRejection::LowerRank);
            }
        }
        NavSelection {
            candidates,
            selected,
        }
    }
}
