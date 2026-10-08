//! N09d: real BeiDou-2 C10 D1 IGSO, dated BDCS2019v01 marked approximation.
#![cfg(feature = "nav")]

#[path = "support/nav_bds_frame.rs"]
mod support;

const FIXTURE: &str = "tests/fixtures/nav_legacy_kms_2022159.rnx";
const EXPECTED: &str = include_str!("reference/nav_bds_igso_frame_expected.json");
const TOE: &str = "2022-06-08T07:00:00 BDT";

#[test]
fn real_c10_d1_igso_reaches_marked_itrf2014_at_its_own_epoch() {
    support::check_real(
        FIXTURE,
        EXPECTED,
        "C10",
        rinex::navigation::NavMessageType::D1,
        TOE,
    );
}

#[test]
fn asserted_bdcs_point_uses_same_path_and_rejects_strict_requests() {
    support::check_asserted(EXPECTED, TOE);
}
