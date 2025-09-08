use crate::navigation::Ephemeris;
use crate::prelude::{Constellation, SV};

use ublox::MgaGpsEphRef;

impl Ephemeris {
    /// Converts this [Ephemeris] to UBX [MgaGpsEph] frame.
    ///
    /// ## Input
    /// - sv: attached [SV]
    pub fn to_ublox_mga_gps<'a>(&self, sv: SV) -> Option<MgaGpsEphRef<'a>> {
        if !matches!(sv.constellation, Constellation::GPS | Constellation::QZSS) {
            // invalid use of the API
            return None;
        }

        Some(MgaGpsEphRef {
            mgs_type: 0,
            version: 0,
            sv_id: {
                if sv.constellation.is_sbas() {
                    sv.prn - 100
                } else {
                    sv.prn
                }
            },
            reserved1: 0,
            reserved2: 0,
            reserved2: 0,
            fit_interval: 0,
            ura_index: 0,
            tgd_s: 0,
            iodc: 0,
            toc: 0,
            af2: 0,
            af1: 0,
            af0: 0,
            crs_rad: 0,
            dn_semicircles: 0,
            m0_semicircles: 0,
            cuc: 0,
            cus: 0,
            e: 0,
            sqrt_a: 0,
            toe: 0,
            cic: 0,
            omega0_semicircles: 0,
            cis: 0,
            crc: 0,
            i0_semicircles: 0,
            omega_semicircles: 0,
            omega_dot: 0,
            idot_semicircles: 0,
        })
    }
}
