// use hifitime::prelude::Unit;

use thiserror::Error;

use crate::{
    navigation::ephemeris::{rad_to_semicircles, semicircles_to_rad, Ephemeris, OrbitItem},
    prelude::{
        // Constellation,
        Epoch,
        //TimeScale,
        SV,
    },
};

use std::collections::HashMap;

use gnss_protos::galileo::{
    GalileoInavWord1, GalileoInavWord2, GalileoInavWord3, GalileoInavWord4, GalileoInavWord5,
    GalileoSignalHealth, GalileoSisa,
};

#[derive(Error, Debug)]
pub enum Error {}

/// Sequence of Galileo INAV data words that must be decoded & extracted
/// from a bit stream. It is possible to form a RINEX [Ephemeris] from any
/// valid combination of these data words.
#[derive(Debug, Copy, Clone, Default)]
pub struct GalileoInavWords {
    /// Word1: Ephemeris
    pub eph_1: GalileoInavWord1,

    /// Word2: Ephemeris
    pub eph_2: GalileoInavWord2,

    /// Word3: Ephemeris
    pub eph_3: GalileoInavWord3,

    /// Word4: Ephemeris
    pub eph_4: GalileoInavWord4,

    /// Word5: Ephemeris
    pub eph_5: GalileoInavWord5,
}

impl Ephemeris {
    /// Builds an [Ephemeris] from collected [GalileoInavWords].
    pub fn from_galileo_inav_words(words: GalileoInavWords) -> Result<Self, Error> {
        let mut orbits = HashMap::<String, OrbitItem>::new();

        let (word_1, word_2, word_3, word_4, word_5) = (
            words.eph_1,
            words.eph_2,
            words.eph_3,
            words.eph_4,
            words.eph_5,
        );

        let mut orbits = HashMap::<String, OrbitItem>::new();

        let ecc = word_1.e;
        let toe = word_1.toe;
        let sqrt_a = word_1.sqrt_a;
        let m0_rad = semicircles_to_rad(word_1.m0);

        let omega_rad = semicircles_to_rad(word_2.omega);
        let omega0_rad = semicircles_to_rad(word_2.omega0);
        let i0_rad = semicircles_to_rad(word_2.i0);
        let omega_rad = semicircles_to_rad(word_2.omega);
        let idot_rad_s = semicircles_to_rad(word_2.idot);

        let iodnav = word_2.iodnav;
        let omega_dot_rad_s = semicircles_to_rad(word_3.omega_dot);
        let dn = semicircles_to_rad(word_3.dn);
        let sisa = word_3.sisa.0;

        let toc = word_4.toc;

        let health = 0.0; // TODO?
        let source = 0.0; // TODO ?

        let bgd_e1_e5a = word_5.bgd_e1_e5a;
        let bgd_e1_e5b = word_5.bgd_e1_e5b;
        let e1b_health = word_5.e1b_health;
        let e5b_health = word_5.e5b_health;
        let wn = word_5.wn;
        let tow = word_5.tow;

        orbits.insert("iodnav".to_string(), OrbitItem::F64(iodnav as f64));
        orbits.insert("crs".to_string(), OrbitItem::F64(word_3.crs));
        orbits.insert("deltaN".to_string(), OrbitItem::F64(dn));
        orbits.insert("m0".to_string(), OrbitItem::F64(m0_rad));
        orbits.insert("cuc".to_string(), OrbitItem::F64(word_3.cuc));
        orbits.insert("e".to_string(), OrbitItem::F64(ecc));
        orbits.insert("cus".to_string(), OrbitItem::F64(word_3.cus));
        orbits.insert("sqrtA".to_string(), OrbitItem::F64(sqrt_a));
        orbits.insert("toe".to_string(), OrbitItem::F64(toe as f64));
        orbits.insert("cic".to_string(), OrbitItem::F64(word_4.cic));
        orbits.insert("omega0".to_string(), OrbitItem::F64(omega0_rad));
        orbits.insert("cis".to_string(), OrbitItem::F64(word_4.cis));
        orbits.insert("i0".to_string(), OrbitItem::F64(i0_rad));
        orbits.insert("crc".to_string(), OrbitItem::F64(word_3.crc));
        orbits.insert("omega".to_string(), OrbitItem::F64(omega_rad));
        orbits.insert("omegaDot".to_string(), OrbitItem::F64(omega_dot_rad_s));
        orbits.insert("idot".to_string(), OrbitItem::F64(idot_rad_s));
        orbits.insert("source".to_string(), OrbitItem::F64(source));
        orbits.insert("week".to_string(), OrbitItem::F64(wn as f64));

        // orbits.insert("spare1".to_string(), OrbitItem::F64(spare1));

        orbits.insert("sisa".to_string(), OrbitItem::F64(sisa as f64));
        orbits.insert("health".to_string(), OrbitItem::F64(health));
        orbits.insert("bgdE5aE1".to_string(), OrbitItem::F64(bgd_e1_e5a));
        orbits.insert("bgdE5bE1".to_string(), OrbitItem::F64(bgd_e1_e5b));

        Ok(Self {
            clock_bias: word_4.af0,
            clock_drift: word_4.af1,
            clock_drift_rate: word_4.af2,
            orbits,
        })
    }

    pub fn to_galileo_inva_words(&self, toc: Epoch, satellite: SV) -> GalileoInavWords {
        let sisa = self.get_orbit_f64("sisa").unwrap_or_default();
        let week = self.get_orbit_f64("week").unwrap_or_default();
        let health = self.get_orbit_f64("health").unwrap_or_default();
        let toe_s = (self.get_orbit_f64("toe").unwrap_or_default()) as u32;
        let iodnav = self.get_orbit_f64("iodnav").unwrap_or_default();
        let toc_s_60s = (toc.to_time_of_week().1 / 1_000_000_000 / 60) as u32;
        let bgd_e1_e5a = self.get_orbit_f64("bgde5aE1").unwrap_or_default();
        let bgd_e1_e5b = self.get_orbit_f64("bgde5bE1").unwrap_or_default();

        let wn = week as u16;
        let tow = (toc.to_time_of_week().1 / 1_000_000_000 / 60) as u32;
        let sisa = GalileoSisa(sisa as u8);
        let e1b_health = GalileoSignalHealth::decode(health as u8);
        let e5b_health = GalileoSignalHealth::decode(health as u8);

        let ecc = self.get_orbit_f64("e").unwrap_or_default();
        let m0_rad = self.get_orbit_f64("m0").unwrap_or_default();
        let i0_rad = self.get_orbit_f64("i0").unwrap_or_default();
        let idot_rad = self.get_orbit_f64("idot").unwrap_or_default();
        let dn_rad = self.get_orbit_f64("deltaN").unwrap_or_default();
        let sqrt_a = self.get_orbit_f64("sqrta").unwrap_or_default();
        let omega_rad = self.get_orbit_f64("omega").unwrap_or_default();
        let omega0_rad = self.get_orbit_f64("omega0").unwrap_or_default();
        let omega_dot_rad = self.get_orbit_f64("omegaDot").unwrap_or_default();

        let crc_m = self.get_orbit_f64("crc").unwrap_or_default();
        let crs_m = self.get_orbit_f64("crs").unwrap_or_default();
        let cuc_rad = self.get_orbit_f64("cuc").unwrap_or_default();
        let cus_rad = self.get_orbit_f64("cus").unwrap_or_default();
        let cic_rad = self.get_orbit_f64("cic").unwrap_or_default();
        let cis_rad = self.get_orbit_f64("cis").unwrap_or_default();

        GalileoInavWords {
            eph_1: GalileoInavWord1 {
                iodnav: iodnav as u16,
                toe: toe_s,
                e: ecc,
                sqrt_a,
                reserved: 0, // TODO ?
                m0: rad_to_semicircles(m0_rad),
            },
            eph_2: GalileoInavWord2 {
                reserved: 0, // TODO ?
                iodnav: iodnav as u16,
                i0: rad_to_semicircles(i0_rad),
                omega: rad_to_semicircles(omega_rad),
                idot: rad_to_semicircles(idot_rad),
                omega0: rad_to_semicircles(omega0_rad),
            },
            eph_3: GalileoInavWord3 {
                iodnav: iodnav as u16,
                sisa,
                cuc: cuc_rad,
                cus: cus_rad,
                crc: crc_m,
                crs: crs_m,
                dn: rad_to_semicircles(dn_rad),
                omega_dot: rad_to_semicircles(omega_dot_rad),
            },
            eph_4: GalileoInavWord4 {
                iodnav: iodnav as u16,
                cic: cic_rad,
                cis: cis_rad,
                toc: toc_s_60s,
                svid: satellite.prn,
                af0: self.clock_bias,
                af1: self.clock_drift,
                af2: self.clock_drift_rate,
                spare: 0, // TODO ?
            },
            eph_5: GalileoInavWord5 {
                wn,
                tow,
                ai0: 0.0,                                              // TODO ?
                ai1: 0.0,                                              // TODO ?
                ai2: 0.0,                                              // TODO ?
                iono_disturbance: [false, false, false, false, false], // TODO ?
                bgd_e1_e5a,
                bgd_e1_e5b,
                e5b_health,
                e1b_health,
                spare: 0,       // TODO ?
                e1b_dvs: false, // TODO ?
                e5b_dvs: false, // TODO ?
            },
        }
    }
}
