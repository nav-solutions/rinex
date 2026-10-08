//! N09d: real BeiDou-2 C05 D2 GEO, dated BDCS2019v01 marked approximation.
#![cfg(feature = "nav")]

#[path = "support/nav_bds_frame.rs"]
mod support;

const FIXTURE: &str = "tests/fixtures/nav_bds_geo_c05_2022159.rnx";
const EXPECTED: &str = include_str!("reference/nav_bds_geo_frame_expected.json");
const TOE: &str = "2022-06-08T09:00:00 BDT";

#[test]
fn real_c05_d2_geo_reaches_marked_itrf2014_at_its_own_epoch() {
    support::check_real(
        FIXTURE,
        EXPECTED,
        "C05",
        rinex::navigation::NavMessageType::D2,
        TOE,
    );
}

#[test]
fn asserted_bdcs_point_reuses_path_and_rejects_strict_requests() {
    support::check_asserted(EXPECTED, TOE);
}
