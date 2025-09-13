use std::collections::HashMap;

use hifitime::prelude::{Duration, Unit};

use crate::{
    navigation::{Ephemeris, OrbitItem},
    prelude::{Constellation, Epoch, SV},
};

use rtcm_rs::msg::{
    Msg1019T,
    Msg1020T,
    Msg1042T, //Msg1043T,
    Msg1044T,
    Msg1045T,
    Msg1046T,
};

#[cfg(doc)]
use crate::prelude::Rinex;

impl Ephemeris {
    /// Converts this [Ephemeris] to [Msg1019T] [Constellation::GPS] ephemeris message.
    /// ## Input
    /// - toc: Time of Clock as [Epoch]
    /// - sv: attached satellite as [SV] which must a [Constellation::GPS] vehicle.
    ///
    /// ## Output
    /// - [Msg1019T] GPS ephemeris message.
    pub fn to_rtcm_gps1019(&self, toc: Epoch, sv: SV) -> Option<Msg1019T> {
        if sv.constellation != Constellation::GPS {
            return None; // invalid API usage
        }

        let (toc_week, toc_week_nanos) = toc.to_time_of_week();

        let toc_s = (toc_week_nanos as f32) * 1.0E-9;
        let toe_s = self.toe(sv)?.duration.to_unit(Unit::Second) as f32;

        let gps_satellite_id = sv.prn;

        let ura_index = self.get_orbit_f64("accuracy").unwrap_or_default() as u8;
        let idot_sc_s = self.get_orbit_f64("idot")?;
        let iodc = self.get_orbit_f64("iodc")? as u16;
        let crs_m = self.get_orbit_f64("crs")? as f32;
        let delta_n_sc_s = self.get_orbit_f64("deltaN")? as f32;
        let m0_sc = self.get_orbit_f64("m0")?;
        let cic_rad = self.get_orbit_f64("cic")? as f32;
        let cis_rad = self.get_orbit_f64("cis")? as f32;
        let cuc_rad = self.get_orbit_f64("cuc")? as f32;
        let cus_rad = self.get_orbit_f64("cus")? as f32;
        let eccentricity = self.get_orbit_f64("e")?;
        let sqrt_a_sqrt_m = self.get_orbit_f64("sqrta")?;
        let i0_sc = self.get_orbit_f64("i0")?;
        let iode = self.get_orbit_f64("iode")? as u8;
        let crc_m = self.get_orbit_f64("crc")? as f32;
        let omega_sc = self.get_orbit_f64("omega")?;
        let omegadot_sc_s = self.get_orbit_f64("omegaDot")?;
        let omega0_sc = self.get_orbit_f64("omega0")?;
        let sv_health_ind = self.get_orbit_f64("health")? as u8;
        let l2_p_data_flag = self.get_orbit_f64("l2p")? as u8;
        let fit_interval_ind = self.get_orbit_f64("fitInt").unwrap_or_default() as u8; // TODO fit int issue
        let tgd_s = self.tgd().unwrap_or(Duration::ZERO).to_unit(Unit::Second) as f32;

        let code_on_l2_ind = 0; // TODO

        Some(Msg1019T {
            gps_satellite_id,
            gps_week_number: toc_week as u16,
            ura_index,
            code_on_l2_ind,
            idot_sc_s,
            toc_s,
            iode,
            omega0_sc,
            omegadot_sc_s,
            af2_s_s2: self.clock_drift_rate as f32,
            af1_s_s: self.clock_drift as f32,
            af0_s: self.clock_bias,
            iodc,
            crs_m,
            delta_n_sc_s,
            m0_sc,
            cuc_rad,
            eccentricity,
            cus_rad,
            sqrt_a_sqrt_m,
            toe_s,
            cic_rad,
            i0_sc,
            crc_m,
            omega_sc,
            cis_rad,
            tgd_s,
            sv_health_ind,
            l2_p_data_flag,
            fit_interval_ind,
        })
    }

    /// Converts this [Ephemeris] to [Msg1020T] [Constellation::Glonass] ephemeris message.
    /// ## Input
    /// - toc: Time of Clock as [Epoch]
    /// - sv: attached satellite as [SV] which must a [Constellation::Glonass] vehicle.
    ///
    /// ## Output
    /// - [Msg1020T] Glonass ephemeris message.
    pub fn to_rtcm_glo1020(&self, toc: Epoch, sv: SV) -> Option<Msg1020T> {
        if sv.constellation != Constellation::Glonass {
            return None; // invalid API usage
        }

        let toe = self.toe(sv)?;

        let tweek_seconds = toe.to_time_of_week().1 * 1_000_000_000;

        let tk_h = 0; // TODO
        let tk_min = 0; // TODO
        let tk_s = 0; // TODO

        let glo_satellite_freq_chan_number = 0; // TODO
        let glo_alm_health_flag = 0; // TODO
        let glo_alm_health_avail_flag = 0; // TODO

        let glo_eph_health_flag = 0; // TODO
        let p1_ind = 0; // TODO
        let p2_flag = 0; // TODO
        let p3_flag = 0; // TODO
        let additional_data_flag = 0; // TODO

        let gamma_n = 0.0; // TODO
        let tb_min = 0; // TODO
        let tau_c_s = 0.0; // TODO
        let tau_n_s = 0.0; // TODO

        let xn_km = 0.0; // TODO
        let yn_km = 0.0; // TODO
        let zn_km = 0.0; // TODO

        let xn_first_deriv_km_s = 0.0; // TODO
        let yn_first_deriv_km_s = 0.0; // TODO
        let zn_first_deriv_km_s = 0.0; // TODO

        let xn_second_deriv_km_s2 = 0.0; // TODO
        let yn_second_deriv_km_s2 = 0.0; // TODO
        let zn_second_deriv_km_s2 = 0.0; // TODO

        let en_d = 0; // TODO
        let na_d = 0; // TODO

        let glo_m_m_ind = 0; // TODO
        let glo_m_p_ind = 0; // TODO
        let glo_m_ft_ind = 0; // TODO
        let glo_m_nt_d = 0; // TODO
        let glo_m_m_d = 0; // TODO
        let glo_m_delta_tau_n_s = 0.0; // TODO
        let glo_m_p4_flag = 0; // TODO
        let glo_m_n4_year = 0; // TODO
        let glo_m_tau_gps_s = 0.0; // TODO
        let glo_m_3str_ln_flag = 0; // TODO
        let glo_m_5str_ln_flag = 0; // TODO
        let reserved_353_7 = 0; // TODO

        Some(Msg1020T {
            glo_satellite_id: sv.prn,
            glo_satellite_freq_chan_number,
            glo_alm_health_flag,
            glo_alm_health_avail_flag,
            p1_ind,
            tk_h,
            tk_min,
            tk_s,
            glo_eph_health_flag,
            p2_flag,
            tb_min,
            xn_first_deriv_km_s,
            xn_km,
            xn_second_deriv_km_s2,
            yn_first_deriv_km_s,
            yn_km,
            yn_second_deriv_km_s2,
            zn_first_deriv_km_s,
            zn_km,
            zn_second_deriv_km_s2,
            p3_flag,
            gamma_n,
            glo_m_p_ind,
            glo_m_3str_ln_flag,
            tau_n_s,
            glo_m_delta_tau_n_s,
            en_d,
            glo_m_p4_flag,
            glo_m_ft_ind,
            glo_m_nt_d,
            glo_m_m_ind,
            additional_data_flag,
            na_d,
            tau_c_s,
            glo_m_n4_year,
            glo_m_tau_gps_s,
            glo_m_5str_ln_flag,
            reserved_353_7,
        })
    }

    /// Converts this [Ephemeris] to [Msg1045T] [Constellation::Galileo] ephemeris message.
    /// ## Input
    /// - toc: Time of Clock as [Epoch]
    /// - sv: attached satellite as [SV] which must a [Constellation::Galileo] vehicle.
    ///
    /// ## Output
    /// - [Msg1045T] Galileo ephemeris message.
    pub fn to_rtcm_gal1045(&self, toc: Epoch, sv: SV) -> Option<Msg1045T> {
        if sv.constellation != Constellation::Galileo {
            return None; // invalid API usage
        }

        let (toc_week, toc_nanos) = toc.to_time_of_week();

        let toc_s = (toc_nanos as f32) * 1.0E-9;
        let toe_s = self.toe(sv)?.duration.to_unit(Unit::Second) as f32;

        let crc_m = self.get_orbit_f64("crc")? as f32;
        let crs_m = self.get_orbit_f64("crs")? as f32;
        let cic_rad = self.get_orbit_f64("cic")? as f32;
        let cis_rad = self.get_orbit_f64("cis")? as f32;
        let cuc_rad = self.get_orbit_f64("cuc")? as f32;
        let cus_rad = self.get_orbit_f64("cus")? as f32;
        let delta_n_sc_s = self.get_orbit_f64("deltaN")? as f32;
        let eccentricity = self.get_orbit_f64("e")?;
        let i0_sc = self.get_orbit_f64("i0")?;
        let m0_sc = self.get_orbit_f64("m0")?;
        let idot_sc_s = self.get_orbit_f64("idot")? as f32;
        let omega0_sc = self.get_orbit_f64("omega0")?;
        let omega_sc = self.get_orbit_f64("omega")?;
        let omegadot_sc_s = self.get_orbit_f64("omegaDot")?;
        let sqrt_a_sqrt_m = self.get_orbit_f64("sqrta")?;
        let iodnav = self.get_orbit_f64("iodnav").unwrap_or_default() as u16; // TODO IODNAV issue?
        let bgd_e1_e5a_s = self.get_orbit_f64("bgdE5aE1").unwrap_or_default() as f32; // TODO BGD_E1/E5A
        let sisa_e1_e5a_index = self.get_orbit_f64("sisa").unwrap_or_default() as u8; // TODO SISA index

        let e5a_data_validity_flag = 0; // TODO
        let e5a_sig_health_ind = 0; // TODO
        let reserved_489_7 = 0; // TODO

        Some(Msg1045T {
            af0_s: self.clock_bias,
            af1_s_s: self.clock_drift,
            af2_s_s2: self.clock_drift_rate as f32,
            bgd_e1_e5a_s,
            cic_rad,
            cis_rad,
            cuc_rad,
            cus_rad,
            crs_m,
            crc_m,
            eccentricity,
            delta_n_sc_s,
            e5a_data_validity_flag,
            e5a_sig_health_ind,
            gal_satellite_id: sv.prn,
            gal_week_number: toc_week as u16,
            i0_sc,
            m0_sc,
            idot_sc_s,
            iodnav,
            omega0_sc,
            omega_sc,
            omegadot_sc_s,
            reserved_489_7,
            sisa_e1_e5a_index,
            sqrt_a_sqrt_m,
            toc_s,
            toe_s,
        })
    }

    /// Converts this [Ephemeris] to [Msg1046T] [Constellation::Galileo] ephemeris message.
    /// ## Input
    /// - toc: Time of Clock as [Epoch]
    /// - sv: attached satellite as [SV] which must a [Constellation::Galileo] vehicle.
    ///
    /// ## Output
    /// - [Msg1045T] Galileo ephemeris message.
    pub fn to_rtcm_gal1046(&self, toc: Epoch, sv: SV) -> Option<Msg1046T> {
        if sv.constellation != Constellation::Galileo {
            return None; // invalid API usage
        }

        let (toc_week, toc_nanos) = toc.to_time_of_week();

        let toc_s = (toc_nanos as f32) * 1.0e-9;
        let toe_s = self.toe(sv)?.duration.to_unit(Unit::Second) as f32;

        let crc_m = self.get_orbit_f64("crc")? as f32;
        let crs_m = self.get_orbit_f64("crs")? as f32;
        let cic_rad = self.get_orbit_f64("cic")? as f32;
        let cis_rad = self.get_orbit_f64("cis")? as f32;
        let cuc_rad = self.get_orbit_f64("cuc")? as f32;
        let cus_rad = self.get_orbit_f64("cus")? as f32;
        let i0_sc = self.get_orbit_f64("i0")?;
        let m0_sc = self.get_orbit_f64("m0")?;
        let idot_sc_s = self.get_orbit_f64("idot")? as f32;
        let eccentricity = self.get_orbit_f64("e")?;
        let delta_n_sc_s = self.get_orbit_f64("deltaN")? as f32;
        let omega_sc = self.get_orbit_f64("omega")?;
        let omegadot_sc_s = self.get_orbit_f64("omegaDot")?;
        let omega0_sc = self.get_orbit_f64("omega0")?;
        let sqrt_a_sqrt_m = self.get_orbit_f64("sqrta")?;

        let bgd_e1_e5a_s = 0.0; // TODO
        let bgd_e1_e5b_s = 0.0; // TODO

        let iodnav = 0; // TODO
        let sisa_e1_e5b_index = 0; // TODO
        let reserved_502_2 = 0; // TODO
        let e1_b_data_validity_flag = 0; // TODO
        let e1_b_sig_health_ind = 0; // TODO
        let e5b_data_validity_flag = 0; // TODO
        let e5b_sig_health_ind = 0; // TODO

        Some(Msg1046T {
            af0_s: self.clock_bias,
            af1_s_s: self.clock_drift,
            af2_s_s2: self.clock_drift_rate as f32,
            bgd_e1_e5a_s,
            bgd_e1_e5b_s,
            cic_rad,
            cis_rad,
            cuc_rad,
            cus_rad,
            crc_m,
            crs_m,
            gal_satellite_id: sv.prn,
            gal_week_number: toc_week as u16,
            i0_sc,
            m0_sc,
            delta_n_sc_s,
            e1_b_data_validity_flag,
            e1_b_sig_health_ind,
            e5b_data_validity_flag,
            e5b_sig_health_ind,
            eccentricity,
            idot_sc_s,
            iodnav,
            omega0_sc,
            omega_sc,
            omegadot_sc_s,
            reserved_502_2,
            sisa_e1_e5b_index,
            sqrt_a_sqrt_m,
            toc_s,
            toe_s,
        })
    }

    /// Converts this [Ephemeris] to [Msg1042T] [Constellation::BeiDou] ephemeris message.
    /// ## Input
    /// - toc: Time of Clock as [Epoch]
    /// - sv: attached satellite as [SV] which must a [Constellation::BeiDou] vehicle.
    ///
    /// ## Output
    /// - [Msg1042T] BDS ephemeris message.
    pub fn to_rtcm_bds1042(&self, toc: Epoch, sv: SV) -> Option<Msg1042T> {
        if sv.constellation != Constellation::BeiDou {
            return None; // invalid API usage
        }

        let (toc_week, toc_nanos) = toc.to_time_of_week();

        let toc_s = (toc_nanos as f32) * 1.0e-9;
        let toe_s = self.toe(sv)?.duration.to_unit(Unit::Second) as f32;

        let aodc = 0; // TODO
        let aode = 0; // TODO

        let crc_m = self.get_orbit_f64("crc")? as f32;
        let crs_m = self.get_orbit_f64("crs")? as f32;
        let cic_rad = self.get_orbit_f64("cic")? as f32;
        let cis_rad = self.get_orbit_f64("cis")? as f32;
        let cuc_rad = self.get_orbit_f64("cuc")? as f32;
        let cus_rad = self.get_orbit_f64("cus")? as f32;
        let delta_n_sc_s = self.get_orbit_f64("deltaN")? as f32;
        let i0_sc = self.get_orbit_f64("i0")?;
        let m0_sc = self.get_orbit_f64("m0")?;
        let idot_sc_s = self.get_orbit_f64("idot")?;
        let eccentricity = self.get_orbit_f64("e")?;
        let omega_sc = self.get_orbit_f64("omega")?;
        let omegadot_sc_s = self.get_orbit_f64("omegaDot")?;
        let omega0_sc = self.get_orbit_f64("omega0")?;
        let sqrt_a_sqrt_m = self.get_orbit_f64("sqrta")?;

        let sv_health_flag = 0; // TODO
        let ura_index = 0; // TODO

        let tgd1_s = self.tgd().unwrap_or(Duration::ZERO).to_unit(Unit::Second) as f32;
        let tgd2_s = tgd1_s; // TODO

        Some(Msg1042T {
            a0_s: self.clock_bias,
            a1_s_s: self.clock_drift,
            a2_s_s2: self.clock_drift_rate as f32,
            aodc,
            aode,
            bds_satellite_id: sv.prn,
            bds_week_number: toc_week as u16,
            cic_rad,
            cis_rad,
            crc_m,
            crs_m,
            cuc_rad,
            cus_rad,
            delta_n_sc_s,
            eccentricity,
            i0_sc,
            m0_sc,
            idot_sc_s,
            omega0_sc,
            omega_sc,
            omegadot_sc_s,
            sqrt_a_sqrt_m,
            sv_health_flag,
            tgd1_s,
            tgd2_s,
            toc_s,
            toe_s,
            ura_index,
        })
    }

    /// Converts this [Ephemeris] to [Msg1044T] [Constellation::QZSS] ephemeris message.
    /// ## Input
    /// - epoch: [Epoch] of message reception.
    /// - sv: attached satellite as [SV] which must a [Constellation::QZSS] vehicle.
    ///
    /// ## Output
    /// - [Msg1044T] QZSS ephemeris message.
    pub fn to_rtcm_qzss1044(&self, epoch: Epoch, sv: SV) -> Option<Msg1044T> {
        if sv.constellation != Constellation::QZSS {
            return None; // invalid API usage
        }

        let (toc_week, toc_week_nanos) = epoch.to_time_of_week();

        let toc_s = (toc_week_nanos as f32) * 1.0E-9;
        let toe_s = self.toe(sv)?.duration.to_unit(Unit::Second) as f32;

        let idot_sc_s = self.get_orbit_f64("idot")?;
        let iodc = self.get_orbit_f64("iodc")? as u16;
        let crs_m = self.get_orbit_f64("crs")? as f32;
        let delta_n_sc_s = self.get_orbit_f64("deltaN")? as f32;
        let m0_sc = self.get_orbit_f64("m0")?;
        let cic_rad = self.get_orbit_f64("cic")? as f32;
        let cis_rad = self.get_orbit_f64("cis")? as f32;
        let cuc_rad = self.get_orbit_f64("cuc")? as f32;
        let cus_rad = self.get_orbit_f64("cus")? as f32;
        let eccentricity = self.get_orbit_f64("e")?;
        let sqrt_a_sqrt_m = self.get_orbit_f64("sqrta")?;
        let i0_sc = self.get_orbit_f64("i0")?;
        let iode = self.get_orbit_f64("iode")? as u8;
        let crc_m = self.get_orbit_f64("crc")? as f32;
        let omega_sc = self.get_orbit_f64("omega")?;
        let omegadot_sc_s = self.get_orbit_f64("omegaDot")?;
        let omega0_sc = self.get_orbit_f64("omega0")?;
        let tgd_s = self.tgd()?.to_unit(Unit::Second) as f32;
        let sv_health_ind = self.get_orbit_f64("health")? as u8;
        let l2_p_data_flag = self.get_orbit_f64("l2p")? as u8;
        let fit_interval_ind = self.get_orbit_f64("fitInt").unwrap_or_default() as u8; // TODO fitInt issue

        let code_on_l2_ind = 0; // TODO
        let ura_index = 0; // TODO

        Some(Msg1044T {
            qzss_satellite_id: sv.prn,
            qzss_week_number: toc_week as u16,
            ura_index,
            code_on_l2_ind,
            idot_sc_s,
            toc_s,
            iode,
            omega0_sc,
            omegadot_sc_s,
            af2_s_s2: self.clock_drift_rate as f32,
            af1_s_s: self.clock_drift as f32,
            af0_s: self.clock_bias,
            iodc,
            crs_m,
            delta_n_sc_s,
            m0_sc,
            cuc_rad,
            eccentricity,
            cus_rad,
            sqrt_a_sqrt_m,
            toe_s,
            cic_rad,
            i0_sc,
            crc_m,
            omega_sc,
            cis_rad,
            tgd_s,
            sv_health_ind,
            fit_interval_ind,
        })
    }

    // /// Converts this [Ephemeris] to [Msg1043T] [Constellation::SBAS] ephemeris message.
    // /// ## Input
    // /// - epoch: [Epoch] of message reception.
    // /// - sv: attached satellite as [SV] which must a [Constellation::SBAS] vehicle.
    // ///
    // /// ## Output
    // /// - [Msg1043T] SBAS ephemeris message.
    // pub fn to_rtcm_sbas_msg1043(&self, epoch: Epoch, sv: SV) -> Option<Msg1043T> {
    //     if !sv.constellation.is_sbas() {
    //         return None; // invalid API usage
    //     }

    //     Some(Msg1043T {

    //     })
    // }
}
