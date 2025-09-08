use crate::navigation::Ephemeris;
use crate::prelude::{Constellation, Epoch, SV};

use ublox::{MgaBdsEphBuilder, MgaGloEphBuilder, MgaGpsEphBuilder};

#[cfg(doc)]
use crate::prelude::Rinex;

#[cfg(doc)]
use ublox::{MgaBdsEphRef, MgaGloEphRef, MgaGpsEphRef};

impl Ephemeris {
    /// Encodes this [Ephemeris] as UBX [MgaGpsEphRef] frame.
    ///
    /// ## Input
    /// - toc: time of clock as [Epoch]
    /// - sv: attached [SV] which must be [Constellation::GPS] or [Constellation::QZSS].
    ///
    /// ## Output
    /// - None
    ///   - if [SV] is not a [Constellation::GPS] or [Constellation::QZSS] satellite
    ///   - if any of the required field is missing.
    /// - [MgaGpsEphRef] encoded frame with all required fields.
    pub fn to_ubx_mga_gps_qzss(&self, toc: Epoch, sv: SV) -> Option<[u8; 76]> {
        if !matches!(sv.constellation, Constellation::GPS | Constellation::QZSS) {
            // invalid use of the API
            return None;
        }

        let toc = (toc.to_time_of_week().1 / 1_000_000_000) as f64;

        let toe = self.get_orbit_f64("toe")?;
        let tgd_s = self.get_orbit_f64("tgd")?;
        let iodc = self.get_orbit_f64("iodc")? as u16;
        let sv_health = self.get_orbit_f64("health")? as u8;

        let (cuc, cus) = (self.get_orbit_f64("cuc")?, self.get_orbit_f64("cus")?);
        let (cic, cis) = (self.get_orbit_f64("cic")?, self.get_orbit_f64("cis")?);
        let (crc, crs_rad) = (self.get_orbit_f64("crc")?, self.get_orbit_f64("crs")?);

        let e = self.get_orbit_f64("e")?;
        let sqrt_a = self.get_orbit_f64("sqrta")?;
        let omega0_semicircles = self.get_orbit_f64("omega0")?;
        let omega_semicircles = self.get_orbit_f64("omega")?;
        let omega_dot = self.get_orbit_f64("omegaDot")?;
        let dn_semicircles = self.get_orbit_f64("deltaN")?;
        let m0_semicircles = self.get_orbit_f64("m0")?;
        let i0_semicircles = self.get_orbit_f64("i0")?;
        let idot_semicircles = self.get_orbit_f64("idot")?;

        // TODO check whether these exist in V2
        let ura_index = self.get_orbit_f64("accuracy").unwrap_or_default() as u8;
        let fit_interval = self.get_orbit_f64("fitInt").unwrap_or_default() as u8;

        let builder = MgaGpsEphBuilder {
            msg_type: 0,
            version: 0,
            sv_id: sv.prn,
            reserved1: 0,
            reserved2: 0,
            reserved3: [0, 0],
            sv_health,
            fit_interval: 0,
            ura_index,
            tgd_s,
            iodc,
            toc,
            af2: self.clock_drift_rate,
            af1: self.clock_drift,
            af0: self.clock_bias,
            dn_semicircles,
            m0_semicircles,
            cic,
            cis,
            cuc,
            cus,
            crc,
            crs_rad,
            e,
            toe,
            sqrt_a,
            omega0_semicircles,
            i0_semicircles,
            omega_semicircles,
            omega_dot,
            idot_semicircles,
        };

        Some(builder.into_packet_bytes())
    }

    /// Encodes this [Ephemeris] as UBX [MgaBdsEphRef] frame.
    ///
    /// ## Input
    /// - toc: time of clock as [Epoch]
    /// - sv: attached [SV] which must be [Constellation::BeiDou]
    ///
    /// ## Output
    /// - None
    ///   - if [SV] is not a [Constellation::BeiDou] satellite.
    ///   - if any of the required field is missing.
    /// - [MgaBdsEphRef] encoded frame with all required fields.
    pub fn to_ubx_mga_bds(&self, toc: Epoch, sv: SV) -> Option<[u8; 96]> {
        if sv.constellation != Constellation::BeiDou {
            // invalid use of the API
            return None;
        }

        let toc = (toc.to_time_of_week().1 / 1_000_000_000) as f64;

        // TODO: is that AODE?
        let iode = 0;

        // TODO (V2/V3)
        let iodc = self.get_orbit_f64("iodc").unwrap_or_default() as u8;

        let ura = self.get_orbit_f64("accuracy")? as u8;

        // TODO TGD versus signals
        let tgd_ns = match self.get_orbit_f64("tgd1b1b3") {
            Some(tgd) => tgd * 1.0E9,
            None => self.get_orbit_f64("tgd2b2b3").unwrap_or_default() * 1.0E9,
        };

        let (cuc_rad, cus_rad) = (self.get_orbit_f64("cuc")?, self.get_orbit_f64("cus")?);
        let (cic_rad, cis_rad) = (self.get_orbit_f64("cic")?, self.get_orbit_f64("cis")?);
        let (crc_rad, crs_rad) = (self.get_orbit_f64("crc")?, self.get_orbit_f64("crs")?);

        let e = self.get_orbit_f64("e")?;
        let sqrt_a = self.get_orbit_f64("sqrta")?;
        let omega0_semicircles = self.get_orbit_f64("omega0")?;
        let omega_semicircles = self.get_orbit_f64("omega")?;
        let omega_dot_semicircles = self.get_orbit_f64("omegaDot")?;
        let dn_semicircles = self.get_orbit_f64("deltaN")?;
        let m0_semicircles = self.get_orbit_f64("m0")?;
        let i0_semicircles = self.get_orbit_f64("i0")?;
        let i_dot_semicircles = self.get_orbit_f64("idot")?;

        let toe = self.get_orbit_f64("toe")?;

        // TODO (V2, V3, exists in V4).
        let ura = self.get_orbit_f64("accuracy").unwrap_or_default() as u8;

        let builder = MgaBdsEphBuilder {
            msg_type: 0,
            version: 0,
            sv_id: sv.prn,
            reserved1: 0,
            sat_h1: 0,
            iodc,
            toc,
            tgd_ns,
            iode,
            toe,
            sqrt_a,
            e,
            ura,
            a0: self.clock_bias,
            a1: self.clock_drift,
            a2: self.clock_drift_rate,
            omega_semicircles,
            dn_semicircles,
            m0_semicircles,
            i_dot_semicircles,
            omega0_semicircles,
            omega_dot_semicircles,
            i0_semicircles,
            cuc_rad,
            cus_rad,
            crc_rad,
            crs_rad,
            cic_rad,
            cis_rad,
            reserved2: [0, 0, 0, 0],
        };

        Some(builder.into_packet_bytes())
    }

    /// Encodes this [Ephemeris] as UBX [MgaGloEphRef] frame.
    ///
    /// ## Input
    /// - sv: attached [SV] which must be [Constellation::Glonass]
    ///
    /// ## Output
    /// - None
    ///   - if [SV] is not a [Constellation::Glonass] satellite.
    ///   - if any of the required field is missing.
    /// - [MgaGloEphRef] encoded frame with all required fields.
    pub fn to_ubx_mga_glo(&self, sv: SV) -> Option<[u8; 56]> {
        if sv.constellation != Constellation::Glonass {
            // invalid use of the API
            return None;
        }

        // TODO: user range accuracy
        let ft = 0;

        // TODO: Type of glonass satellite - '1' means Glonass-M
        let m = 0;

        // TODO: relative frequency deviation
        let gamma = 0.0f64;

        // TODO: delta tau (s)
        let delta_tau_s = 0.0f64;

        // TODO tb_mins
        let tb_mins = 0;

        let b = self.get_orbit_f64("health")? as u8;
        let h = self.get_orbit_f64("channel")? as i8;
        let eph_age_days = self.get_orbit_f64("ageOp")? as u8;

        let (x_km, y_km, z_km) = (
            self.get_orbit_f64("satPosX")? / 1000.0,
            self.get_orbit_f64("satPosY")? / 1000.0,
            self.get_orbit_f64("satPosZ")? / 1000.0,
        );

        let (dx_km_s, dy_km_s, dz_km_s) = (
            self.get_orbit_f64("velX")? / 1000.0,
            self.get_orbit_f64("velY")? / 1000.0,
            self.get_orbit_f64("velZ")? / 1000.0,
        );

        let (ddx_km_s2, ddy_km_s2, ddz_km_s2) = (
            self.get_orbit_f64("accelX")? / 1000.0,
            self.get_orbit_f64("accelY")? / 1000.0,
            self.get_orbit_f64("accelZ")? / 1000.0,
        );

        let builder = MgaGloEphBuilder {
            msg_type: 0,
            version: 0,
            sv_id: sv.prn,
            reserved1: 0,
            ft,
            b,
            m,
            h,
            x_km,
            y_km,
            z_km,
            gamma,
            dx_km_s,
            dy_km_s,
            dz_km_s,
            ddx_km_s2,
            ddy_km_s2,
            ddz_km_s2,
            eph_age_days,
            delta_tau_s,
            tb_mins,
            tau_s: self.clock_bias,
            reserved2: [0, 0, 0, 0],
        };

        Some(builder.into_packet_bytes())
    }
}
