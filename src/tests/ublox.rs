use crate::prelude::Rinex;

#[test]
#[cfg(feature = "nav")]
fn rinex_nav_to_ublox() {
    let rinex = Rinex::from_file("data/NAV/V3/AMEL00NLD_R_20210010000_01D_MN.rnx").unwrap();

    // MGA-EPH-XXX
}
