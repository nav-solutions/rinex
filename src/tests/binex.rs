use crate::navigation::Ephemeris;
use crate::prelude::Rinex;

use binex::prelude::Meta;

#[test]
#[ignore]
fn nav_v3_to_binex() {
    let rinex = Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    let meta = Meta::default();

    let mut streamer = rinex.rnx2bin(meta);

    for message in streamer.iter() {}
}

#[test]
#[ignore]
fn nav_v3_ephemeris() {
    let rinex = Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

    for (k, ephemeris) in rinex.nav_ephemeris_frames_iter() {
        let serialized = ephemeris.to_binex(k.epoch, k.sv).unwrap_or_else(|| {
            panic!("Failed to serialize {}({})", k.epoch, k.sv);
        });

        // mirror
        let (decoded_sv, decoded) =
            Ephemeris::from_binex(k.epoch, serialized).unwrap_or_else(|| {
                panic!("Failed to decoded {}({}) BINEX frame", k.epoch, k.sv);
            });

        // testbench
        assert_eq!(k.sv, decoded_sv, "{}({}) invalid SV", k.epoch, k.sv);
        assert_eq!(
            *ephemeris, decoded,
            "{}({}) invalid content decoded",
            k.epoch, k.sv
        );
    }
}
