// SBAS specific tests

#[cfg(feature = "qc")]
use crate::prelude::{Constellation, Rinex, SV};

#[cfg(feature = "qc")]
use std::str::FromStr;

#[cfg(feature = "qc")]
use qc_traits::{Filter, FilterItem, MaskOperand, Preprocessing};

// Formatting issue
#[test]
#[cfg(all(feature = "flate2", feature = "qc"))]
fn test_sbas_obs_v3_formatting() {
    let rinex =
        Rinex::from_gzip_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz").unwrap();

    // basic initial verifications..
    let (mut s23_found, mut s25_found, mut s36_found) = (false, false, false);
    let s23 = SV::from_str("S23").unwrap();
    let s25 = SV::from_str("S25").unwrap();
    let s36 = SV::from_str("S36").unwrap();

    for sv in rinex.satellite_iter() {
        s23_found |= sv == s23;
        s25_found |= sv == s25;
        s36_found |= sv == s36;
    }

    assert!(s23_found, "S23 not present in initial setup!");
    assert!(s25_found, "S25 not present in initial setup!");
    assert!(s36_found, "S36 not present in initial setup!");

    let geo_only = Filter::mask(
        MaskOperand::Equals,
        FilterItem::ConstellationItem(vec![Constellation::SBAS]),
    );

    let mut geo_only = rinex.filter(&geo_only);

    // RINEX
    let rinex_geo_only = geo_only.crnx2rnx();

    // dump
    rinex_geo_only
        .to_file("test_geo-only.txt")
        .unwrap_or_else(|e| panic!("SBAS only formatting issue: {}", e));

    // parse back
    let dut = Rinex::from_file("test_geo-only.txt")
        .unwrap_or_else(|e| panic!("SABS only: failed to parse back: {}", e));

    // test
    let (mut s23_found, mut s25_found, mut s36_found) = (false, false, false);
    for sv in dut.satellite_iter() {
        s23_found |= sv == s23;
        s25_found |= sv == s25;
        s36_found |= sv == s36;
    }

    assert!(s23_found, "S23 not present in parsed-back RINEX!");
    assert!(s25_found, "S25 not present in parsed-back RINEX!");
    assert!(s36_found, "S36 not present in parsed-back RINEX!");

    // CRINEX dump
    geo_only
        .to_file("test_geo-only.txt")
        .unwrap_or_else(|e| panic!("SBAS only CRINEX formatting issue: {}", e));

    // parse back
    let dut = Rinex::from_file("test_geo-only.txt")
        .unwrap_or_else(|e| panic!("SABS only: failed to parse back: {}", e));

    // test
    let (mut s23_found, mut s25_found, mut s36_found) = (false, false, false);
    for sv in dut.satellite_iter() {
        s23_found |= sv == s23;
        s25_found |= sv == s25;
        s36_found |= sv == s36;
    }

    assert!(s23_found, "S23 not present in parsed-back RINEX!");
    assert!(s25_found, "S25 not present in parsed-back RINEX!");
    assert!(s36_found, "S36 not present in parsed-back RINEX!");

    // delete
    let _ = std::fs::remove_file("test_geo-only.txt");
}
