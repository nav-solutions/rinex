use crate::{
    navigation::{Ephemeris, OrbitItem},
    prelude::{Constellation, Rinex, SV},
    tests::toolkit::nav::panic_eq_ephemeris,
};

use std::collections::HashMap;

#[test]
fn test_inav_v3() {
    let mut count = 0;

    for gz_file in [
        "BRDC00GOP_R_20210010000_01D_MN.rnx.gz",
        "BRDC00IGS_R_20241310000_01D_MN.rnx.gz",
        "BRDM00DLR_S_20241310000_01D_MN.rnx.gz",
        "BRUX00BEL_R_20240920000_01D_EN.rnx.gz",
        "CORD00ARG_R_20240920000_01D_MN.rnx.gz",
        "ESBC00DNK_R_20201770000_01D_MN.rnx.gz",
        "GRAS00FRA_R_20242090000_01D_EN.rnx.gz",
        "GRAS00FRA_R_20242100000_01D_EN.rnx.gz",
        "HERT00GBR_R_20240920000_01D_GN.rnx.gz",
        "MOJN00DNK_R_20201770000_01D_MN.rnx.gz",
        "NYA100NOR_S_20241240000_01D_CN.rnx.gz",
        "NYA100NOR_S_20241240000_01D_EN.rnx.gz",
        "NYA100NOR_S_20241240000_01D_GN.rnx.gz",
        "NYA100NOR_S_20241270000_01D_EN.rnx.gz",
        "NYA100NOR_S_20241270000_01D_GN.rnx.gz",
        "NYA100NOR_S_20241280000_01D_EN.rnx.gz",
        "NYA100NOR_S_20241280000_01D_GN.rnx.gz",
    ] {
        let fullpath = format!("{}/data/NAV/V3/{}", env!("CARGO_MANIFEST_DIR"), gz_file,);

        let rinex = Rinex::from_gzip_file(&fullpath).unwrap();

        for (key, eph) in rinex.nav_ephemeris_frames_iter() {
            let words = eph.to_galileo_inav_words(key.sv).unwrap_or_else(|e| {
                panic!(
                    "Failed to format GPS subframe burst for {} @ {}",
                    key.sv, key.epoch
                );
            });

            let reciprocal = Ephemeris::from_galileo_inav_words(words).unwrap();
            panic_eq_ephemeris(&reciprocal, eph);
            count += 1;
        }
    }
    assert_eq!(count, 6);
}
