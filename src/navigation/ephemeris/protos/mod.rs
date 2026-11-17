// pub mod gal;
pub mod gps_qzss;

pub(crate) fn rad_to_semicircles(rad: f64) -> f64 {
    rad / core::f64::consts::PI
}

pub(crate) fn semicircles_to_rad(semic: f64) -> f64 {
    semic * core::f64::consts::PI
}
