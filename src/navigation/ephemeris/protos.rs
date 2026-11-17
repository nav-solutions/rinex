use hifitime::prelude::{Duration, Unit};

use crate::{
    navigation::{ephemeris::GpsQzssl1cHealth, Ephemeris, OrbitItem},
    prelude::{Constellation, Epoch, SV},
};

use std::collections::HashMap;

use gnss_protos::{
    GpsQzssFrame, GpsQzssFrame1, GpsQzssFrame2, GpsQzssFrame3, GpsQzssFrameId, GpsQzssHow,
    GpsQzssSubframe, GpsQzssTelemetry,
};

impl Ephemeris {
    /// Builds an [Ephemeris] frame from a serie of received [GpsQzssFrame] decoded
    /// from a raw binary stream.
    /// This method does not verify the validity of the message and combination
    /// of the 3 messages, which should be done prior calling this method.
    pub fn from_gps_qzss_messages(
        eph_1: GpsQzssFrame1,
        eph_2: GpsQzssFrame2,
        eph_3: GpsQzssFrame3,
    ) -> Self {
        let mut orbits = HashMap::<String, OrbitItem>::new();

        // orbits.insert("i0".to_string(), eph_1.i0_rad);
        orbits.insert("week".to_string(), OrbitItem::U32(eph_1.week.into()));
        orbits.insert(
            "l2p".to_string(),
            OrbitItem::Gpsl2pFlag(eph_1.l2_p_data_flag),
        );
        orbits.insert("tgd".to_string(), OrbitItem::F64(eph_1.tgd));
        orbits.insert("ura".to_string(), OrbitItem::U8(eph_1.ura));

        let l1c_health = Into::<u8>::into(eph_1.health);
        let l1c_health = GpsQzssl1cHealth::from_bits(l1c_health as u32).unwrap_or_default();

        orbits.insert(
            "health".to_string(),
            OrbitItem::GpsQzssl1cHealth(l1c_health),
        );

        Self {
            clock_bias: eph_1.af0,
            clock_drift: eph_1.af1,
            clock_drift_rate: eph_1.af2,
            orbits,
        }
    }

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
    pub fn to_gps_qzss_messages(
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
        let week = (week & 0x3ff) as u16;

        let l2p_mask = self.get_orbit_f64("l2p").unwrap_or_default();
        let health = self.get_orbit_f64("health").unwrap_or_default();
        let tgd_nanos = self.get_orbit_f64("tgd").unwrap_or_default();

        let ura_m = self.get_orbit_f64("accuracy").unwrap_or_default();

        let mut subframe1 = GpsQzssFrame1::default()
            .with_week(week)
            .with_iodc(iodc)
            .with_ca_or_p_l2_mask(l2p_mask as u8)
            .with_health((health as u8).into())
            .with_clock_offset_seconds(self.clock_bias)
            .with_clock_drift_seconds_s(self.clock_drift)
            .with_user_range_accuracy_m(ura_m)
            .with_clock_drift_rate_seconds_s2(self.clock_drift_rate)
            .with_total_group_delay_nanos(tgd_nanos);

        // TODO .with_l2p_flag

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
            subframe: GpsQzssSubframe::Ephemeris1(subframe1),
        };

        let toe_s = self.toe(sv)?.duration.to_unit(Unit::Second);

        let i0_rad = self.get_orbit_f64("i0").unwrap_or_default();
        let crs_m = self.get_orbit_f64("crs").unwrap_or_default();
        let cus_rad = self.get_orbit_f64("cus").unwrap_or_default();
        let semi_major_axis_m = self.get_orbit_f64("sqrt_a").unwrap_or_default().powi(2);
        let mean_motion_diff_rad = self.get_orbit_f64("delta_n").unwrap_or_default();

        let iode = self.get_orbit_f64("iode").unwrap_or_default();

        let mut subframe2 = GpsQzssFrame2::default()
            .with_toe_seconds(toe_s as u32)
            .with_mean_anomaly_radians(i0_rad)
            .with_crs_meters(crs_m)
            .with_iode(iode as u8)
            .with_eccentricity(self.get_orbit_f64("e").unwrap_or_default())
            .with_semi_major_axis_meters(semi_major_axis_m)
            .with_mean_motion_difference_radians(mean_motion_diff_rad)
            .with_cus_radians(cus_rad);

        // TODO subframe2 .with_fit_interval

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
            subframe: GpsQzssSubframe::Ephemeris2(subframe2),
        };

        let i0_rad = self.get_orbit_f64("i0").unwrap_or_default();
        let idot_rad_s = self.get_orbit_f64("idot").unwrap_or_default();

        let crc_m = self.get_orbit_f64("crc").unwrap_or_default();
        let cic_rad = self.get_orbit_f64("cic").unwrap_or_default();
        let cis_rad = self.get_orbit_f64("cis").unwrap_or_default();

        let omega0_rad = self.get_orbit_f64("omega0").unwrap_or_default();
        let omega_rad = self.get_orbit_f64("omega").unwrap_or_default();
        let omega_dot_rad_s = self.get_orbit_f64("omegaDot").unwrap_or_default();

        let mut subframe3 = GpsQzssFrame3::default()
            .with_cic_radians(cic_rad)
            .with_cis_radians(cis_rad)
            .with_crc_meters(crc_m)
            .with_inclination_radians(i0_rad)
            .with_iode(iode as u8)
            .with_inclination_rate_radians_s(idot_rad_s)
            .with_longitude_ascending_node_radians(omega0_rad)
            .with_omega_radians(omega_rad)
            .with_omega_dot_rad_s(omega_dot_rad_s);

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
                frame_id: GpsQzssFrameId::Ephemeris3,
            },
            subframe: GpsQzssSubframe::Ephemeris3(subframe3),
        };

        Some([frame1, frame2, frame3])
    }
}
