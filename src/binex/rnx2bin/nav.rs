use crate::{
    navigation::{Ephemeris, NavKey},
    prelude::{Constellation, Epoch, Rinex, SV},
};

use binex::prelude::{
    EphemerisFrame, GALEphemeris, GLOEphemeris, GPSEphemeris, Message, Meta, Record, SBASEphemeris,
};

/// NAV Record Streamer
pub struct Streamer<'a> {
    meta: Meta,
    ephemeris_iter: Box<dyn Iterator<Item = (&'a NavKey, &'a Ephemeris)> + 'a>,
}

fn forge_gps_ephemeris_frame(_toc: &Epoch, sv: SV, eph: &Ephemeris) -> Option<EphemerisFrame> {
    let clock_offset = eph.clock_bias as f32;
    let clock_drift = eph.clock_drift as f32;
    let clock_drift_rate = eph.clock_drift_rate as f32;

    let toe = eph.orbits.get("toe")?.as_f64() as u16;

    let cic = eph.orbits.get("cic")?.as_f64() as f32;
    let crc = eph.orbits.get("crc")?.as_f64() as f32;
    let cis = eph.orbits.get("cis")?.as_f64() as f32;
    let crs = eph.orbits.get("crs")?.as_f64() as f32;
    let cuc = eph.orbits.get("cuc")?.as_f64() as f32;
    let cus = eph.orbits.get("cus")?.as_f64() as f32;

    let e = eph.orbits.get("e")?.as_f64();
    let m0_rad = eph.orbits.get("m0")?.as_f64();
    let i0_rad = eph.orbits.get("i0")?.as_f64();
    let sqrt_a = eph.orbits.get("sqrta")?.as_f64();
    let omega_rad = eph.orbits.get("omega")?.as_f64();
    let omega_0_rad = eph.orbits.get("omega0")?.as_f64();
    let omega_dot_rad_s = eph.orbits.get("oemgaDot")?.as_f64() as f32;
    let i_dot_rad_s = eph.orbits.get("idot")?.as_f64() as f32;
    let delta_n_rad_s = eph.orbits.get("delta_n")?.as_f64() as f32;

    let tgd = eph.orbits.get("tgd")?.as_f64() as f32;
    let iode = eph.orbits.get("iode")?.as_u32() as i32;
    let iodc = eph.orbits.get("iodc")?.as_u32() as i32;

    Some(EphemerisFrame::GPS(GPSEphemeris {
        sv_prn: sv.prn,
        iode,
        iodc,
        toe,
        tow: 0,
        toc: 0,
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
        ura_m: 0.0,
        sv_health: 0,
        uint2: 0,
    }))
}

fn forge_sbas_ephemeris_frame(_toc: &Epoch, sv: SV, eph: &Ephemeris) -> Option<EphemerisFrame> {
    let sbas_prn = sv.prn;

    let clock_offset = eph.clock_bias;
    let clock_drift = eph.clock_drift;

    let x_km = eph.orbits.get("posX")?.as_f64();
    let vel_x_km = eph.orbits.get("velX")?.as_f64();
    let acc_x_km = eph.orbits.get("accelX")?.as_f64();

    let y_km = eph.orbits.get("posX")?.as_f64();
    let vel_y_km = eph.orbits.get("velY")?.as_f64();
    let acc_y_km = eph.orbits.get("accelY")?.as_f64();

    let z_km = eph.orbits.get("posX")?.as_f64();
    let vel_z_km = eph.orbits.get("velZ")?.as_f64();
    let acc_z_km = eph.orbits.get("accelZ")?.as_f64();

    let iodn = eph.orbits.get("iodn")?.as_u8();

    Some(EphemerisFrame::SBAS(SBASEphemeris {
        sbas_prn,
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
        uint1: 0,
        ura: 0,
        iodn,
    }))
}

fn forge_gal_ephemeris_frame(_toc: &Epoch, sv: SV, eph: &Ephemeris) -> Option<EphemerisFrame> {
    let _sv_prn = sv.prn;

    let clock_offset = eph.clock_bias as f32;
    let clock_drift = eph.clock_drift as f32;
    let clock_drift_rate = eph.clock_drift_rate as f32;

    let cic = eph.orbits.get("cic")?.as_f64() as f32;
    let crc = eph.orbits.get("crc")?.as_f64() as f32;
    let cis = eph.orbits.get("cis")?.as_f64() as f32;
    let crs = eph.orbits.get("crs")?.as_f64() as f32;
    let cuc = eph.orbits.get("cuc")?.as_f64() as f32;
    let cus = eph.orbits.get("cus")?.as_f64() as f32;

    let e = eph.orbits.get("e")?.as_f64();
    let m0_rad = eph.orbits.get("m0")?.as_f64();
    let i0_rad = eph.orbits.get("i0")?.as_f64();
    let sqrt_a = eph.orbits.get("sqrta")?.as_f64();
    let omega_rad = eph.orbits.get("omega")?.as_f64();
    let omega_0_rad = eph.orbits.get("omega0")?.as_f64();

    let omega_dot_rad_s = eph.orbits.get("oemgaDot")?.as_f64() as f32;
    let omega_dot_semi_circles = omega_dot_rad_s;

    let i_dot_rad_s = eph.orbits.get("idot")?.as_f64() as f32;
    let idot_semi_circles_s = i_dot_rad_s;

    let delta_n_rad_s = eph.orbits.get("delta_n")?.as_f64() as f32;
    let delta_n_semi_circles_s = delta_n_rad_s;

    Some(EphemerisFrame::GAL(GALEphemeris {
        sv_prn: 0,
        toe_week: 0,
        tow: 0,
        toe_s: 0,
        bgd_e5a_e1_s: 0.0,
        bgd_e5b_e1_s: 0.0,
        iodnav: 0,
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
        sisa: 0.0,
        sv_health: 0,
        source: 0,
    }))
}

fn forge_glo_ephemeris_frame(eph: &Ephemeris) -> Option<EphemerisFrame> {
    let clock_offset_s = eph.clock_bias;
    let clock_rel_freq_bias = eph.clock_drift;

    let x_km = eph.orbits.get("posX")?.as_f64();
    let vel_x_km = eph.orbits.get("velX")?.as_f64();
    let acc_x_km = eph.orbits.get("accelX")?.as_f64();

    let y_km = eph.orbits.get("posX")?.as_f64();
    let vel_y_km = eph.orbits.get("velY")?.as_f64();
    let acc_y_km = eph.orbits.get("accelY")?.as_f64();

    let z_km = eph.orbits.get("posX")?.as_f64();
    let vel_z_km = eph.orbits.get("velZ")?.as_f64();
    let acc_z_km = eph.orbits.get("accelZ")?.as_f64();

    Some(EphemerisFrame::GLO(GLOEphemeris {
        slot: 0,
        day: 0,
        tod_s: 0,
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
        sv_health: 0,
        freq_channel: 0,
        age_op_days: 0,
        leap_s: 0,
        tau_gps_s: 0.0,
        l1_l2_gd: 0.0,
    }))
}

impl<'a> Streamer<'a> {
    pub fn new(meta: Meta, rinex: &'a Rinex) -> Self {
        Self {
            meta: meta,
            ephemeris_iter: rinex.nav_ephemeris_frames_iter(),
        }
    }
}

impl<'a> Iterator for Streamer<'a> {
    type Item = Message;
    fn next(&mut self) -> Option<Self::Item> {
        let (key, eph) = self.ephemeris_iter.next()?;

        let frame = if key.sv.constellation.is_sbas() {
            forge_sbas_ephemeris_frame(&key.epoch, key.sv, eph)
        } else {
            match key.sv.constellation {
                Constellation::GPS => forge_gps_ephemeris_frame(&key.epoch, key.sv, eph),
                Constellation::Galileo => forge_gal_ephemeris_frame(&key.epoch, key.sv, eph),
                Constellation::Glonass => forge_glo_ephemeris_frame(eph),
                _ => None,
            }
        };

        let frame = frame?;
        Some(Message {
            meta: self.meta,
            record: Record::new_ephemeris_frame(frame),
        })
    }
}
