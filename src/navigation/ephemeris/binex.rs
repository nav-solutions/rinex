use crate::{
    navigation::{Ephemeris, OrbitItem},
    prelude::{Constellation, Epoch, SV},
};

use std::collections::HashMap;

use binex::prelude::{EphemerisFrame, GALEphemeris, GLOEphemeris, GPSEphemeris, SBASEphemeris};

impl Ephemeris {
    /// Converts this BINEX [EphemerisFrame] to [Ephemeris], ready to format.
    /// We support GPS, QZSS, Galileo, Glonass and SBAS frames.
    ///
    /// ## Inputs
    /// - now: usually the [Epoch] of message reception
    pub fn from_binex(now: Epoch, message: EphemerisFrame) -> Option<(SV, Self)> {
        match message {
            EphemerisFrame::GPS(serialized) => Some((
                SV::new(Constellation::GPS, serialized.sv_prn),
                Self {
                    clock_bias: 0.0,
                    clock_drift: 0.0,
                    clock_drift_rate: 0.0,
                    orbits: HashMap::from_iter([("week".to_string(), OrbitItem::from(0.0f64))]),
                },
            )),
            EphemerisFrame::SBAS(serialized) => Some((
                SV::new(Constellation::SBAS, serialized.sbas_prn),
                Self {
                    clock_bias: 0.0,
                    clock_drift: 0.0,
                    clock_drift_rate: 0.0,
                    orbits: HashMap::from_iter([("week".to_string(), OrbitItem::from(0.0f64))]),
                },
            )),
            EphemerisFrame::GLO(serialized) => Some((
                SV::new(Constellation::Glonass, serialized.slot),
                Self {
                    clock_bias: 0.0,
                    clock_drift: 0.0,
                    clock_drift_rate: 0.0,
                    orbits: HashMap::from_iter([("week".to_string(), OrbitItem::from(0.0f64))]),
                },
            )),
            EphemerisFrame::GAL(serialized) => Some((
                SV::new(Constellation::Galileo, serialized.sv_prn),
                Self {
                    clock_bias: 0.0,
                    clock_drift: 0.0,
                    clock_drift_rate: 0.0,
                    orbits: HashMap::from_iter([("week".to_string(), OrbitItem::from(0.0f64))]),
                },
            )),
            _ => None,
        }
    }

    /// Encodes this [Ephemeris] to BINEX [EphemerisFrame], ready to encode.
    /// We currently support GPS, QZSS, SBAS, Galileo and Glonass.
    ///
    /// ## Inputs
    /// - toc: time of clock as [Epoch]
    /// - sv: [SV] attached to this [Ephemeris]
    ///
    /// ## Output
    /// - [EphemerisFrame]: all required fields must exist
    /// so we can forge a frame.
    pub fn to_binex(&self, toc: Epoch, sv: SV) -> Option<EphemerisFrame> {
        let tow = toc.to_time_of_week().1 / 1_000_000_000;
        let toc = toc.to_time_of_week().1 / 1_000_000_000;

        match sv.constellation {
            Constellation::GPS | Constellation::QZSS => {
                let clock_offset = self.clock_bias as f32;
                let clock_drift = self.clock_drift as f32;
                let clock_drift_rate = self.clock_drift_rate as f32;

                let toe = self.orbits.get("toe")?.as_f64() as u16;

                let cic = self.orbits.get("cic")?.as_f64() as f32;
                let crc = self.orbits.get("crc")?.as_f64() as f32;
                let cis = self.orbits.get("cis")?.as_f64() as f32;
                let crs = self.orbits.get("crs")?.as_f64() as f32;
                let cuc = self.orbits.get("cuc")?.as_f64() as f32;
                let cus = self.orbits.get("cus")?.as_f64() as f32;

                let sv_health = self.orbits.get("health")?.as_f64() as u16;

                let e = self.orbits.get("e")?.as_f64();
                let m0_rad = self.orbits.get("m0")?.as_f64();
                let i0_rad = self.orbits.get("i0")?.as_f64();
                let sqrt_a = self.orbits.get("sqrta")?.as_f64();
                let omega_rad = self.orbits.get("omega")?.as_f64();
                let omega_0_rad = self.orbits.get("omega0")?.as_f64();
                let omega_dot_rad_s = self.orbits.get("oemgaDot")?.as_f64() as f32;

                let i_dot_rad_s = self.orbits.get("idot")?.as_f64() as f32;
                let delta_n_rad_s = self.orbits.get("delta_n")?.as_f64() as f32;

                let tgd = self.orbits.get("tgd")?.as_f64() as f32;
                let iode = self.orbits.get("iode")?.as_u32() as i32;
                let iodc = self.orbits.get("iodc")?.as_u32() as i32;

                Some(EphemerisFrame::GPS(GPSEphemeris {
                    sv_prn: sv.prn,
                    iode,
                    iodc,
                    toe,
                    tow: tow as i32,
                    toc: toc as i32,
                    tgd,
                    clock_offset,
                    clock_drift,
                    clock_drift_rate,
                    delta_n_rad_s,
                    m0_rad,
                    e,
                    sqrt_a,
                    cic,
                    crc,
                    cis,
                    crs,
                    cuc,
                    cus,
                    omega_0_rad,
                    omega_rad,
                    i_dot_rad_s,
                    omega_dot_rad_s,
                    i0_rad,
                    ura_m: 0.0, // TODO
                    sv_health,
                    uint2: 0, // TODO
                }))
            },
            Constellation::Glonass => {
                let clock_offset_s = self.clock_bias;
                let clock_rel_freq_bias = self.clock_drift;

                // let slot = self.orbits.get("channel")?.as_u8();
                let sv_health = self.orbits.get("health")?.as_u8();

                let x_km = self.orbits.get("satPosX")?.as_f64();
                let vel_x_km = self.orbits.get("velX")?.as_f64();
                let acc_x_km = self.orbits.get("accelX")?.as_f64();

                let y_km = self.orbits.get("satPosX")?.as_f64();
                let vel_y_km = self.orbits.get("velY")?.as_f64();
                let acc_y_km = self.orbits.get("accelY")?.as_f64();

                let z_km = self.orbits.get("satPosX")?.as_f64();
                let vel_z_km = self.orbits.get("velZ")?.as_f64();
                let acc_z_km = self.orbits.get("accelZ")?.as_f64();

                Some(EphemerisFrame::GLO(GLOEphemeris {
                    slot: 0,  // TODO
                    day: 0,   // TODO
                    tod_s: 0, // TODO
                    clock_offset_s,
                    clock_rel_freq_bias,
                    t_k_sec: 0,
                    x_km,
                    vel_x_km,
                    acc_x_km,
                    y_km,
                    vel_y_km,
                    acc_y_km,
                    z_km,
                    vel_z_km,
                    acc_z_km,
                    sv_health,
                    freq_channel: 0,
                    age_op_days: 0,
                    leap_s: 0,
                    tau_gps_s: 0.0,
                    l1_l2_gd: 0.0,
                }))
            },
            Constellation::Galileo => {
                let clock_offset = self.clock_bias as f32;
                let clock_drift = self.clock_drift as f32;
                let clock_drift_rate = self.clock_drift_rate as f32;

                let cic = self.orbits.get("cic")?.as_f64() as f32;
                let crc = self.orbits.get("crc")?.as_f64() as f32;
                let cis = self.orbits.get("cis")?.as_f64() as f32;
                let crs = self.orbits.get("crs")?.as_f64() as f32;
                let cuc = self.orbits.get("cuc")?.as_f64() as f32;
                let cus = self.orbits.get("cus")?.as_f64() as f32;

                let e = self.orbits.get("e")?.as_f64();
                let m0_rad = self.orbits.get("m0")?.as_f64();
                let i0_rad = self.orbits.get("i0")?.as_f64();
                let sqrt_a = self.orbits.get("sqrta")?.as_f64();
                let omega_rad = self.orbits.get("omega")?.as_f64();
                let omega_0_rad = self.orbits.get("omega0")?.as_f64();

                let omega_dot_rad_s = self.orbits.get("oemgaDot")?.as_f64() as f32;
                let omega_dot_semi_circles = omega_dot_rad_s;

                let i_dot_rad_s = self.orbits.get("idot")?.as_f64() as f32;
                let idot_semi_circles_s = i_dot_rad_s;

                let delta_n_rad_s = self.orbits.get("delta_n")?.as_f64() as f32;
                let delta_n_semi_circles_s = delta_n_rad_s;

                let sv_health = self.get_orbit_f64("health")? as u16;
                let sisa = self.get_orbit_f64("sisa").unwrap_or_default() as f32; // TODO SISA issue?
                let iodnav = self.get_orbit_f64("iodnav").unwrap_or_default() as i32; // TODO IODNAV issue?

                let (toe_week, toe_nanos) = self.toe(sv)?.to_time_of_week();
                let toe_s = (toe_nanos / 1_000_000_000) as i32;

                Some(EphemerisFrame::GAL(GALEphemeris {
                    sv_prn: sv.prn,
                    tow: tow as i32,
                    toe_week: toe_week as u16,
                    toe_s,
                    bgd_e5a_e1_s: 0.0, // TODO
                    bgd_e5b_e1_s: 0.0, // TODO
                    iodnav,
                    clock_drift_rate,
                    clock_drift,
                    clock_offset,
                    delta_n_semi_circles_s,
                    m0_rad,
                    e,
                    sqrt_a,
                    cic,
                    crc,
                    cis,
                    cuc,
                    cus,
                    crs,
                    omega_0_rad,
                    omega_rad,
                    i0_rad,
                    omega_dot_semi_circles,
                    idot_semi_circles_s,
                    sisa,
                    sv_health,
                    source: 0, // TODO
                }))
            },
            constellation => {
                if constellation.is_sbas() {
                    let clock_offset = self.clock_bias;
                    let clock_drift = self.clock_drift;

                    let x_km = self.orbits.get("satPosX")?.as_f64();
                    let vel_x_km = self.orbits.get("velX")?.as_f64();
                    let acc_x_km = self.orbits.get("accelX")?.as_f64();

                    let y_km = self.orbits.get("satPosX")?.as_f64();
                    let vel_y_km = self.orbits.get("velY")?.as_f64();
                    let acc_y_km = self.orbits.get("accelY")?.as_f64();

                    let z_km = self.orbits.get("satPosX")?.as_f64();
                    let vel_z_km = self.orbits.get("velZ")?.as_f64();
                    let acc_z_km = self.orbits.get("accelZ")?.as_f64();

                    let iodn = self.orbits.get("iodn")?.as_u8();

                    Some(EphemerisFrame::SBAS(SBASEphemeris {
                        sbas_prn: sv.prn,
                        toe: 0,
                        tow: 0,
                        clock_offset,
                        clock_drift,
                        x_km,
                        vel_x_km,
                        acc_x_km,
                        y_km,
                        vel_y_km,
                        acc_y_km,
                        z_km,
                        vel_z_km,
                        acc_z_km,
                        uint1: 0, // TODO
                        ura: 0,   // TODO
                        iodn,
                    }))
                } else {
                    None
                }
            },
        }
    }
}
