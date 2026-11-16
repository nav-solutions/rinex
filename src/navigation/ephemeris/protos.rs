use hifitime::prelude::{Duration, Unit};

use crate::{
    navigation::Ephemeris,
    prelude::{Constellation, Epoch, SV},
};

use gnss_protos::{
    GpsQzssFrame, GpsQzssFrame1, GpsQzssFrame2, GpsQzssFrame3, GpsQzssFrameId, GpsQzssHow,
    GpsQzssSubframe, GpsQzssTelemetry,
};

impl Ephemeris {
    /// Converts this [Ephemeris] frame to a burst of [GpsQzssFrame]s, which
    /// you can then stream in binary.
    /// The burst will contain:
    /// - TODO
    ///
    /// ## Input
    /// - toc: Time of Clock as [Epoch]
    /// - sv: attached satellite as [SV] which must either be GPS or QZSS satellite.
    /// - iodc: IODC message identifier
    /// - telemetry: attached [GpsQzssTelemetry]
    ///
    /// ## Output
    /// - burst of [GpsQzssFrame]s
    pub fn to_gps_qzss_frames(
        &self,
        toc: Epoch,
        sv: SV,
        iodc: u16,
        telemetry: GpsQzssTelemetry,
    ) -> Option<[GpsQzssFrame; 3]> {
        match sv.constellation {
            Constellation::GPS | Constellation::QZSS => {},
            _ => return None,
        }

        let (tow, week) = toc.to_time_of_week();

        let subframe1 = GpsQzssSubframe::Ephemeris1(
            GpsQzssFrame1::default()
                .with_week(week)
                .with_iodc(iodc)
                .with_ca_or_p_l2_mask(self.get_orbit_f64("l2p").unwrap_or_default())
                .with_l2p_flag(self.get_orbit_f64("l2p").unwrap_or_default())
                .with_health(self.get_orbit_f64("health").unwrap_or_default())
                .with_clock_offset_nanoseconds()
                .with_clock_drift_seconds_s()
                .with_ura(self.get_orbit_f64("accuracy").unwrap_or_default())
                .with_clock_drift_rate_seconds_s2()
                .with_total_group_delay_nanos(self.get_orbit_f64("tgd").unwrap_or_default()),
        );

        let frame1 = GpsQzssFrame {
            telemetry,
            how: GpsQzssHow {
                tow,
                // The alert bit serves two purposes.
                // For block 000 satellites, '1' here indicates a maneuver.
                // For other satellites, '1' here means the URA may be worse than indicated in subframe 1
                // and user shall use this SV at their own risk.
                alert: false,
                // is this lost in the RINEX fmt ?
                anti_spoofing: false,
                frame_id: GpsQzssFrameId::Ephemeris1,
            },
            frame: GpsQzssFrame::Ephemeris1(subframe1),
        };

        let subframe2 = GpsQzssFrame::Ephemeris2(
            GpsQzssFrame2::default()
                .with_toe(self.toe(sv)?.duration.to_unit(Unit::Second) as u32)
                .with_m0
                .with_crs
                .with_e
                .with_fit_int_flag
                .with_sqrt_a
                .with_dn
                .with_cus
                .with_aodo
                .with_iode(self.get_orbit_f64("iode").unwrap_or_default()),
        );

        let frame2 = GpsQzssFrame {
            telemetry,
            how: GpsQzssHow {
                tow,
                // The alert bit serves two purposes.
                // For block 000 satellites, '1' here indicates a maneuver.
                // For other satellites, '1' here means the URA may be worse than indicated in subframe 1
                // and user shall use this SV at their own risk.
                alert: false,
                // is this lost in the RINEX fmt ?
                anti_spoofing: false,
                frame_id: GpsQzssFrameId::Ephemeris2,
            },
            frame: GpsQzssFrame::Ephemeris2(subframe2),
        };

        let subframe3 = GpsQzssFrame::Ephemeris3(
            GpsQzssFrame3::default()
                .with_cic
                .with_cis
                .with_crc
                .with_i0
                .with_iode
                .with_idot
                .with_omega0
                .with_omega
                .with_omega_dot,
        );

        let frame3 = GpsQzssFrame {
            telemetry,
            how: GpsQzssHow {
                tow,
                // The alert bit serves two purposes.
                // For block 000 satellites, '1' here indicates a maneuver.
                // For other satellites, '1' here means the URA may be worse than indicated in subframe 1
                // and user shall use this SV at their own risk.
                alert: false,
                // is this lost in the RINEX fmt ?
                anti_spoofing: false,
            },
            frame: GpsQzssFrame::Ephemeris3(subframe3),
            frame_id: GpsQzssFrameId::Ephemeris2,
        };

        Some([frame1, frame2, frame3])
    }
}
