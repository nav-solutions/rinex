use std::collections::HashMap;

use hifitime::prelude::Unit;

use crate::{
    navigation::{Ephemeris, OrbitItem},
    prelude::{Constellation, Epoch, SV},
};

use rtcm_rs::msg::{Msg1019T, Msg1020T};

#[cfg(doc)]
use crate::prelude::Rinex;

impl Ephemeris {
    /// Converts this [Ephemeris] to [Constellation::GPS] [Msg1019T]
    /// ## Input
    /// - epoch: [Epoch] of message reception.
    /// - sv: attached satellite as [SV] which must a [Constellation::GPS] vehicle.
    ///
    /// ## Output
    /// - [Msg1019T]
    pub fn to_rtcm_msg1019(&self, epoch: Epoch, sv: SV) -> Option<Msg1019T> {
        if sv.constellation != Constellation::GPS {
            return None; // invalid API usage
        }

        let (toc_week, toc_week_nanos) = epoch.to_time_of_week();
        let toc_s = (toc_week_nanos as f32) * 1.0E-9;

        let toe = self.toe(sv)?;
        let toe_week_nanos = toe.to_time_of_week().1;
        let toe_s = (toe_week_nanos as f32) * 1.0E-9;

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
        let tgd_s = self.tgd()?.to_unit(Unit::Second) as f32;
        let sv_health_ind = self.get_orbit_f64("health")? as u8;
        let l2_p_data_flag = self.get_orbit_f64("l2p")? as u8;
        let fit_interval_ind = self.get_orbit_f64("fitInt")? as u8;

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

    /// Converts this [Ephemeris] to Glonass [Msg1020T]
    /// ## Input
    /// - epoch: [Epoch] of message reception
    /// - sv: attached satellite as [SV] which must a [Constellation::Glonass] vehicle.
    ///
    /// ## Output
    /// - [Msg1020T]
    pub fn to_rtcm_msg1001(&self, epoch: Epoch, sv: SV) -> Option<Msg1020T> {
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
}
