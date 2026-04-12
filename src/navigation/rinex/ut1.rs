use crate::prelude::{ut1::DeltaTaiUt1, Duration, Rinex};

impl Rinex {
    /// Forms a [DeltaTaiUt1] [Iterator] from all Earth Orientation parameters contained in this NAV V4 RINEX.
    /// Does not apply to any other formats.
    pub fn nav_delta_tai_ut1_iter(&self) -> Box<dyn Iterator<Item = DeltaTaiUt1> + '_> {
        Box::new(
            self.nav_earth_orientation_frames_iter()
                .map(|(k, eop)| DeltaTaiUt1 {
                    epoch: k.epoch,
                    delta_tai_minus_ut1: Duration::from_seconds(eop.delta_ut1.0),
                }),
        )
    }
}
