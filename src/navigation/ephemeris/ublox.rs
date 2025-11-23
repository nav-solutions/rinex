use std::collections::HashMap;

use crate::{
    navigation::{Ephemeris, OrbitItem},
    prelude::{Constellation, Epoch, SV},
};

use ublox::{
    MgaBdsEphBuilder, MgaBdsEphRef, MgaGalEphBuilder, MgaGalEphRef, MgaGloEphBuilder, MgaGloEphRef,
    MgaGpsEphBuilder, MgaGpsEphRef,
};

#[cfg(doc)]
use crate::prelude::Rinex;

impl Ephemeris {
    /// Decodes this UBX [MgaGpsEphRef] frame as [Ephemeris] structure, ready to format.
    ///
    /// ## Inputs
    /// - now: UBX message [Epoch] of reception
    ///
    /// ## Returns
    /// - Identified [Constellation::GPS] message emitter
    /// - [Ephemeris] structure ready to format.
    pub fn from_ubx_mga_gps(now: Epoch, ubx: MgaGpsEphRef) -> (SV, Self) {
        (
            SV {
                prn: ubx.sv_id(),
                constellation: Constellation::GPS,
            },
            Self {
                clock_bias: ubx.af0(),
                clock_drift: ubx.af1(),
                clock_drift_rate: ubx.af2(),
                orbits: HashMap::from_iter([
                    ("week".to_string(), OrbitItem::F64(0.0)),
                    ("toe".to_string(), OrbitItem::F64(ubx.toe())),
                    ("e".to_string(), OrbitItem::F64(ubx.e())),
                    ("cic".to_string(), OrbitItem::F64(ubx.cic())),
                    ("cis".to_string(), OrbitItem::F64(ubx.cis())),
                    ("cuc".to_string(), OrbitItem::F64(ubx.cuc())),
                    ("cus".to_string(), OrbitItem::F64(ubx.cus())),
                    ("crc".to_string(), OrbitItem::F64(ubx.crc())),
                    ("crs".to_string(), OrbitItem::F64(ubx.crs_rad())),
                    ("tgd".to_string(), OrbitItem::F64(ubx.tgd_s())),
                    ("sqrta".to_string(), OrbitItem::F64(ubx.sqrt_a())),
                    ("iodc".to_string(), OrbitItem::F64(ubx.iodc() as f64)),
                    ("m0".to_string(), OrbitItem::F64(ubx.m0_semicircles())),
                    ("deltaN".to_string(), OrbitItem::F64(ubx.dn_semicircles())),
                    ("idot".to_string(), OrbitItem::F64(ubx.idot_semicircles())),
                    ("omega".to_string(), OrbitItem::F64(ubx.omega_semicircles())),
                    ("omegaDot".to_string(), OrbitItem::F64(ubx.omega_dot())),
                    (
                        "omega0".to_string(),
                        OrbitItem::F64(ubx.omega0_semicircles()),
                    ),
                ]),
            },
        )
    }

    /// Decodes this UBX [MgaGpsEphRef] frame as [Ephemeris] structure, ready to format.
    ///
    /// ## Inputs
    /// - now: UBX message [Epoch] of reception
    ///
    /// ## Returns
    /// - Identified [Constellation::QZSS] message emitter
    /// - [Ephemeris] structure ready to format.
    pub fn from_ubx_mga_qzss(now: Epoch, ubx: MgaGpsEphRef) -> (SV, Self) {
        (
            SV {
                prn: ubx.sv_id(),
                constellation: Constellation::QZSS,
            },
            Self {
                clock_bias: ubx.af0(),
                clock_drift: ubx.af1(),
                clock_drift_rate: ubx.af2(),
                orbits: HashMap::from_iter([
                    ("week".to_string(), OrbitItem::F64(0.0)),
                    ("toe".to_string(), OrbitItem::F64(ubx.toe())),
                    ("e".to_string(), OrbitItem::F64(ubx.e())),
                    ("cic".to_string(), OrbitItem::F64(ubx.cic())),
                    ("cis".to_string(), OrbitItem::F64(ubx.cis())),
                    ("cuc".to_string(), OrbitItem::F64(ubx.cuc())),
                    ("cus".to_string(), OrbitItem::F64(ubx.cus())),
                    ("crc".to_string(), OrbitItem::F64(ubx.crc())),
                    ("crs".to_string(), OrbitItem::F64(ubx.crs_rad())),
                    ("sqrta".to_string(), OrbitItem::F64(ubx.sqrt_a())),
                    ("iodc".to_string(), OrbitItem::F64(ubx.iodc() as f64)),
                    ("m0".to_string(), OrbitItem::F64(ubx.m0_semicircles())),
                    ("deltaN".to_string(), OrbitItem::F64(ubx.dn_semicircles())),
                    ("idot".to_string(), OrbitItem::F64(ubx.idot_semicircles())),
                    ("omega".to_string(), OrbitItem::F64(ubx.omega_semicircles())),
                    ("omegaDot".to_string(), OrbitItem::F64(ubx.omega_dot())),
                    (
                        "omega0".to_string(),
                        OrbitItem::F64(ubx.omega0_semicircles()),
                    ),
                ]),
            },
        )
    }

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

        let toe = self.get_orbit_field_f64("toe").ok()?;
        let tgd_s = self.get_orbit_field_f64("tgd").ok()?;
        let iodc = self.get_orbit_field_f64("iodc").ok()? as u16;
        let sv_health = self.get_orbit_field_f64("health").ok()? as u8;

        let (cuc, cus) = (
            self.get_orbit_field_f64("cuc").ok()?,
            self.get_orbit_field_f64("cus").ok()?,
        );
        let (cic, cis) = (
            self.get_orbit_field_f64("cic").ok()?,
            self.get_orbit_field_f64("cis").ok()?,
        );
        let (crc, crs) = (
            self.get_orbit_field_f64("crc").ok()?,
            self.get_orbit_field_f64("crs").ok()?,
        );

        let e = self.get_orbit_field_f64("e").ok()?;
        let sqrt_a = self.get_orbit_field_f64("sqrta").ok()?;
        let omega0_semicircles = self.get_orbit_field_f64("omega0").ok()?;
        let omega_semicircles = self.get_orbit_field_f64("omega").ok()?;
        let omega_dot = self.get_orbit_field_f64("omegaDot").ok()?;
        let dn_semicircles = self.get_orbit_field_f64("deltaN").ok()?;
        let m0_semicircles = self.get_orbit_field_f64("m0").ok()?;
        let i0_semicircles = self.get_orbit_field_f64("i0").ok()?;
        let idot_semicircles = self.get_orbit_field_f64("idot").ok()?;

        // TODO check whether these exist in V2
        let ura_index = self.get_orbit_field_f64("accuracy").unwrap_or_default() as u8;
        let fit_interval = self.get_orbit_field_f64("fitInt").unwrap_or_default() as u8;

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
            crs_rad: crs,
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

    /// Decodes this UBX [MgaBdsEphRef] frame as [Ephemeris] structure, ready to format.
    ///
    /// ## Inputs
    /// - now: UBX message [Epoch] of reception
    ///
    /// ## Returns
    /// - Identified [Constellation::BeiDou] message emitter
    /// - [Ephemeris] structure ready to format.
    pub fn from_ubx_mga_bds(now: Epoch, ubx: MgaBdsEphRef) -> (SV, Self) {
        (
            SV {
                prn: ubx.sv_id(),
                constellation: Constellation::BeiDou,
            },
            Self {
                clock_bias: ubx.a0(),
                clock_drift: ubx.a1(),
                clock_drift_rate: ubx.a2(),
                orbits: HashMap::from_iter([
                    ("week".to_string(), OrbitItem::F64(0.0)),
                    ("toe".to_string(), OrbitItem::F64(ubx.toe())),
                    ("e".to_string(), OrbitItem::F64(ubx.e())),
                    ("cic".to_string(), OrbitItem::F64(ubx.cic_rad())),
                    ("cis".to_string(), OrbitItem::F64(ubx.cis_rad())),
                    ("cuc".to_string(), OrbitItem::F64(ubx.cuc_rad())),
                    ("cus".to_string(), OrbitItem::F64(ubx.cus_rad())),
                    ("crc".to_string(), OrbitItem::F64(ubx.crc_rad())),
                    ("crs".to_string(), OrbitItem::F64(ubx.crs_rad())),
                    ("sqrta".to_string(), OrbitItem::F64(ubx.sqrt_a())),
                    ("iodc".to_string(), OrbitItem::F64(ubx.iodc() as f64)),
                    ("m0".to_string(), OrbitItem::F64(ubx.m0_semicircles())),
                    ("deltaN".to_string(), OrbitItem::F64(ubx.dn_semicircles())),
                    ("idot".to_string(), OrbitItem::F64(ubx.i_dot_semicircles())),
                    (
                        "tgd1b1b2".to_string(),
                        OrbitItem::F64(ubx.tgd_ns() * 1.0E-9),
                    ),
                    (
                        "tgd1b2b3".to_string(),
                        OrbitItem::F64(ubx.tgd_ns() * 1.0E-9),
                    ),
                    ("omega".to_string(), OrbitItem::F64(ubx.omega_semicircles())),
                    (
                        "omega0".to_string(),
                        OrbitItem::F64(ubx.omega0_semicircles()),
                    ),
                    (
                        "omegaDot".to_string(),
                        OrbitItem::F64(ubx.omega_dot_semicircles()),
                    ),
                ]),
            },
        )
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
        let iodc = self.get_orbit_field_f64("iodc").unwrap_or_default() as u8;

        let ura = self.get_orbit_field_f64("accuracy").ok()? as u8;

        // TODO TGD versus signals
        let tgd_ns = match self.get_orbit_field_f64("tgd1b1b3") {
            Ok(tgd) => tgd * 1.0E9,
            Err(_) => self.get_orbit_field_f64("tgd2b2b3").unwrap_or_default() * 1.0E9,
        };

        let (cuc_rad, cus_rad) = (
            self.get_orbit_field_f64("cuc").ok()?,
            self.get_orbit_field_f64("cus").ok()?,
        );
        let (cic_rad, cis_rad) = (
            self.get_orbit_field_f64("cic").ok()?,
            self.get_orbit_field_f64("cis").ok()?,
        );
        let (crc_rad, crs_rad) = (
            self.get_orbit_field_f64("crc").ok()?,
            self.get_orbit_field_f64("crs").ok()?,
        );

        let e = self.get_orbit_field_f64("e").ok()?;
        let sqrt_a = self.get_orbit_field_f64("sqrta").ok()?;
        let omega0_semicircles = self.get_orbit_field_f64("omega0").ok()?;
        let omega_semicircles = self.get_orbit_field_f64("omega").ok()?;
        let omega_dot_semicircles = self.get_orbit_field_f64("omegaDot").ok()?;
        let dn_semicircles = self.get_orbit_field_f64("deltaN").ok()?;
        let m0_semicircles = self.get_orbit_field_f64("m0").ok()?;
        let i0_semicircles = self.get_orbit_field_f64("i0").ok()?;
        let i_dot_semicircles = self.get_orbit_field_f64("idot").ok()?;

        let toe = self.get_orbit_field_f64("toe").ok()?;

        // TODO exists in V4, check V2 and V3
        let ura = self.get_orbit_field_f64("accuracy").unwrap_or_default() as u8;

        let builder = MgaBdsEphBuilder {
            msg_type: 0, // TODO
            version: 0,  // TODO
            sv_id: sv.prn,
            reserved1: 0,
            sat_h1: 0, // TODO
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

    /// Decodes this UBX [MgaGloEphRef] frame as [Ephemeris] structure, ready to format.
    ///
    /// ## Inputs
    /// - now: UBX message [Epoch] of reception
    ///
    /// ## Returns
    /// - Identified [Constellation::Glonass] message emitter
    /// - [Ephemeris] structure ready to format.
    pub fn from_ubx_mga_glo(now: Epoch, ubx: MgaGloEphRef) -> (SV, Self) {
        (
            SV {
                prn: ubx.sv_id(),
                constellation: Constellation::Glonass,
            },
            Self {
                clock_bias: ubx.tau_s(),
                clock_drift: 0.0,
                clock_drift_rate: 0.0,
                orbits: HashMap::from_iter([
                    ("health".to_string(), OrbitItem::F64(0.0)),
                    ("channel".to_string(), OrbitItem::F64(ubx.h() as f64)),
                    ("posX".to_string(), OrbitItem::F64(ubx.x_km())),
                    ("posY".to_string(), OrbitItem::F64(ubx.y_km())),
                    ("posZ".to_string(), OrbitItem::F64(ubx.z_km())),
                    ("velX".to_string(), OrbitItem::F64(ubx.dx_km_s())),
                    ("velY".to_string(), OrbitItem::F64(ubx.dy_km_s())),
                    ("velZ".to_string(), OrbitItem::F64(ubx.dz_km_s())),
                    ("accelX".to_string(), OrbitItem::F64(ubx.ddx_km_s2())),
                    ("accelY".to_string(), OrbitItem::F64(ubx.ddy_km_s2())),
                    ("accelZ".to_string(), OrbitItem::F64(ubx.ddz_km_s2())),
                ]),
            },
        )
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

        let b = self.get_orbit_field_f64("health").ok()? as u8;
        let h = self.get_orbit_field_f64("channel").ok()? as i8;
        let eph_age_days = self.get_orbit_field_f64("ageOp").ok()? as u8;

        let (x_km, y_km, z_km) = (
            self.get_orbit_field_f64("posX").ok()? / 1000.0,
            self.get_orbit_field_f64("posY").ok()? / 1000.0,
            self.get_orbit_field_f64("posZ").ok()? / 1000.0,
        );

        let (dx_km_s, dy_km_s, dz_km_s) = (
            self.get_orbit_field_f64("velX").ok()? / 1000.0,
            self.get_orbit_field_f64("velY").ok()? / 1000.0,
            self.get_orbit_field_f64("velZ").ok()? / 1000.0,
        );

        let (ddx_km_s2, ddy_km_s2, ddz_km_s2) = (
            self.get_orbit_field_f64("accelX").ok()? / 1000.0,
            self.get_orbit_field_f64("accelY").ok()? / 1000.0,
            self.get_orbit_field_f64("accelZ").ok()? / 1000.0,
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

    /// Decodes this UBX [MgaGalEphRef] frame as [Ephemeris] structure, ready to format.
    ///
    /// ## Inputs
    /// - now: UBX message [Epoch] of reception
    ///
    /// ## Returns
    /// - Identified [Constellation::Galileo] message emitter
    /// - [Ephemeris] structure ready to format.
    pub fn from_ubx_mga_gal(now: Epoch, ubx: MgaGalEphRef) -> (SV, Self) {
        (
            SV {
                prn: ubx.sv_id(),
                constellation: Constellation::Galileo,
            },
            Self {
                clock_bias: ubx.af0(),
                clock_drift: ubx.af1(),
                clock_drift_rate: ubx.af2(),
                orbits: HashMap::from_iter([
                    ("week".to_string(), OrbitItem::F64(0.0)),
                    ("toe".to_string(), OrbitItem::F64(ubx.toe())),
                    ("e".to_string(), OrbitItem::F64(ubx.e())),
                    ("cic".to_string(), OrbitItem::F64(ubx.cic_rad())),
                    ("cis".to_string(), OrbitItem::F64(ubx.cis_rad())),
                    ("cuc".to_string(), OrbitItem::F64(ubx.cuc_rad())),
                    ("cus".to_string(), OrbitItem::F64(ubx.cus_rad())),
                    ("crc".to_string(), OrbitItem::F64(ubx.crc_rad())),
                    ("crs".to_string(), OrbitItem::F64(ubx.crs_rad())),
                    ("sqrta".to_string(), OrbitItem::F64(ubx.sqrt_a())),
                    ("m0".to_string(), OrbitItem::F64(ubx.m0_semicircles())),
                    ("deltaN".to_string(), OrbitItem::F64(ubx.dn_semicircles())),
                    ("idot".to_string(), OrbitItem::F64(ubx.i_dot_semicircles())),
                    ("omega".to_string(), OrbitItem::F64(ubx.omega_semicircles())),
                    ("iodnav".to_string(), OrbitItem::F64(ubx.iodnav() as f64)),
                    (
                        "health".to_string(),
                        OrbitItem::F64(ubx.e1b_health() as f64),
                    ),
                    ("bdgE5aE1".to_string(), OrbitItem::F64(ubx.bgd_e1_e5b_s())),
                    ("bdgE5bE1".to_string(), OrbitItem::F64(ubx.bgd_e1_e5b_s())),
                    ("sisa".to_string(), OrbitItem::F64(ubx.sisa_e1_e5b() as f64)),
                    (
                        "source".to_string(),
                        OrbitItem::F64(ubx.omega_semicircles()),
                    ),
                    (
                        "omega0".to_string(),
                        OrbitItem::F64(ubx.omega0_semicircles()),
                    ),
                    (
                        "omegaDot".to_string(),
                        OrbitItem::F64(ubx.omega_dot_semicircles()),
                    ),
                ]),
            },
        )
    }

    /// Encodes this [Ephemeris] as UBX [MgaGalEphRef] frame.
    ///
    /// ## Input
    /// - toc: time of clock as [Epoch]
    /// - sv: attached [SV] which must be [Constellation::Galileo]
    ///
    /// ## Output
    /// - None
    ///   - if [SV] is not a [Constellation::Galileo] satellite.
    ///   - if any of the required field is missing.
    /// - [MgaGalEphRef] encoded frame with all required fields.
    pub fn to_ubx_mga_gal(&self, toc: Epoch, sv: SV) -> Option<[u8; 84]> {
        if sv.constellation != Constellation::Galileo {
            // invalid use of the API
            return None;
        }

        let toc = (toc.to_time_of_week().1 / 1_000_000_000) as f64;

        let (cuc_rad, cus_rad) = (
            self.get_orbit_field_f64("cuc").ok()?,
            self.get_orbit_field_f64("cus").ok()?,
        );
        let (cic_rad, cis_rad) = (
            self.get_orbit_field_f64("cic").ok()?,
            self.get_orbit_field_f64("cis").ok()?,
        );
        let (crc_rad, crs_rad) = (
            self.get_orbit_field_f64("crc").ok()?,
            self.get_orbit_field_f64("crs").ok()?,
        );

        let e = self.get_orbit_field_f64("e").ok()?;
        let sqrt_a = self.get_orbit_field_f64("sqrta").ok()?;
        let omega0_semicircles = self.get_orbit_field_f64("omega0").ok()?;
        let omega_semicircles = self.get_orbit_field_f64("omega").ok()?;
        let omega_dot_semicircles = self.get_orbit_field_f64("omegaDot").ok()?;
        let dn_semicircles = self.get_orbit_field_f64("deltaN").ok()?;
        let m0_semicircles = self.get_orbit_field_f64("m0").ok()?;
        let i0_semicircles = self.get_orbit_field_f64("i0").ok()?;
        let i_dot_semicircles = self.get_orbit_field_f64("idot").ok()?;

        let toe = self.get_orbit_field_f64("toe").ok()?;

        // TODO exists in V4, check V2 and V3
        let ura = self.get_orbit_field_f64("accuracy").unwrap_or_default() as u8;

        let builder = MgaGalEphBuilder {
            msg_type: 0, // TODO
            version: 0,  // TODO
            sv_id: sv.prn,
            reserved1: 0,
            toc,
            toe,
            sqrt_a,
            e,
            af0: self.clock_bias,
            af1: self.clock_drift,
            af2: self.clock_drift_rate,
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
            iodnav: 0,         // TODO
            e1b_health: 0,     // TODO
            e5b_health: 0,     // TODO
            e5b_validity: 0,   // TODO
            bgd_e1_e5b_s: 0.0, // TODO
            sisa_e1_e5b: 0,    // TODO
            e1b_validity: 0,   // TODO
            reserved2: [0, 0],
            reserved3: [0, 0, 0, 0],
        };

        Some(builder.into_packet_bytes())
    }
}
