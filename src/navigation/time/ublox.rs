use crate::{
    navigation::time::TimeOffset,
    prelude::{Epoch, TimeScale},
};

use ublox::{MgaGalTimeBuilder, MgaGpsUtcBuilder};

#[cfg(doc)]
use ublox::{MgaGalTimeRef, MgaGpsUtcRef};

impl TimeOffset {
    /// Encodes this [TimeOffset] as [MgaGpsUtcRef] if it is referenced
    /// to [TimeScale::GPST] and [TimeScale::UTC].
    pub fn to_ubx_mga_gps_utc(&self, ref_epoch: Epoch) -> Option<[u8; 28]> {
        let (utc_wn_t, utc_tot) = ref_epoch.to_time_scale(TimeScale::UTC).to_time_of_week();

        if self.lhs == TimeScale::GPST {
            if self.rhs == TimeScale::UTC {
                let builder = MgaGpsUtcBuilder {
                    msg_type: 1,
                    version: 0,
                    utc_a0: self.polynomial.0,
                    utc_a1: self.polynomial.1,
                    utc_dt_ls: 0, // Delta time due to current leap seconds
                    utc_tot: (utc_tot / 1_000_000_000) as u8, // UTC reference time of week
                    utc_wn_t: utc_wn_t as u8, // UTC reference week number
                    utc_wn_lsf: 0,
                    utc_dn: 0,
                    utc_dt_lsf: 0,
                    reserved1: [0, 0],
                    reserved2: [0, 0],
                };

                return Some(builder.into_packet_bytes());
            }
        }

        None
    }

    /// Encodes this [TimeOffset] as [MgaGalTimeRef] if it is referenced
    /// to [TimeScale::GST] and [TimeScale::GPST].
    pub fn to_ubx_mga_gal_time(&self, ref_epoch: Epoch) -> Option<[u8; 20]> {
        let (wn0g, t0g) = ref_epoch.to_time_of_week();

        if self.lhs == TimeScale::GST {
            if self.rhs == TimeScale::GPST {
                let builder = MgaGalTimeBuilder {
                    msg_type: 1,
                    version: 0,
                    wn0g: wn0g as f64,
                    a0g: self.polynomial.0,
                    a1g: self.polynomial.1,
                    t0g: (t0g / 1_000_000_000) as f64,
                    reserved1: [0, 0],
                    reserved2: [0, 0],
                };

                return Some(builder.into_packet_bytes());
            }
        }

        None
    }
}
