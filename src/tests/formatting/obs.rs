use std::collections::HashMap;
use std::str::FromStr;

use crate::{
    observation::HeaderFields,
    prelude::{Constellation, Epoch, Observable},
    tests::formatting::{Utf8Buffer, generic_formatted_lines_test},
};

use std::io::BufWriter;

#[test]
fn obs_v1_single_line_formatting() {
    let mut buf = BufWriter::new(Utf8Buffer::new(1024));

    let gps = Constellation::GPS;

    let l1 = Observable::PhaseRange("L1".to_string());

    let gps_codes = vec![l1.clone(), l1.clone(), l1.clone(), l1.clone()];
    let mut hd = HeaderFields::default()
        .with_time_of_first_obs(Epoch::from_str("2020-01-01T00:00:00 GPST").unwrap())
        .with_time_of_last_obs(Epoch::from_str("2020-01-01T23:30:00 GPST").unwrap());

    hd.codes.insert(gps, gps_codes);

    hd.format(&mut buf, 2).unwrap();

    let content = buf.into_inner().unwrap().to_ascii_utf8();

    generic_formatted_lines_test(
        &content,
        HashMap::from_iter([
            (
                0,
                "  2020     1     1     0     0    0.0000000     GPS         TIME OF FIRST OBS",
            ),
            (
                1,
                "  2020     1     1    23    30    0.0000000     GPS         TIME OF LAST OBS",
            ),
            (
                2,
                "     4    L1    L1    L1    L1                              # / TYPES OF OBSERV",
            ),
        ]),
    );
}

#[test]
fn obs_v1_lengthy_formatting() {
    let mut buf = BufWriter::new(Utf8Buffer::new(1024));

    let gps = Constellation::GPS;

    let l1 = Observable::PhaseRange("L1".to_string());

    let gps_codes = vec![
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
        l1.clone(),
    ];
    let mut hd = HeaderFields::default()
        .with_time_of_first_obs(Epoch::from_str("2020-01-01T00:00:00 GPST").unwrap())
        .with_time_of_last_obs(Epoch::from_str("2020-01-01T23:30:00 GPST").unwrap());

    hd.codes.insert(gps, gps_codes);

    hd.format(&mut buf, 2).unwrap();

    let content = buf.into_inner().unwrap().to_ascii_utf8();

    generic_formatted_lines_test(
        &content,
        HashMap::from_iter([
            (
                0,
                "  2020     1     1     0     0    0.0000000     GPS         TIME OF FIRST OBS",
            ),
            (
                1,
                "  2020     1     1    23    30    0.0000000     GPS         TIME OF LAST OBS",
            ),
            (
                2,
                "    22    L1    L1    L1    L1    L1    L1    L1    L1    L1# / TYPES OF OBSERV",
            ),
            (
                3,
                "          L1    L1    L1    L1    L1    L1    L1    L1    L1# / TYPES OF OBSERV",
            ),
            (
                4,
                "          L1    L1    L1    L1                              # / TYPES OF OBSERV",
            ),
        ]),
    );
}

#[test]
fn obs_v1_header_formatting() {
    let mut buf = BufWriter::new(Utf8Buffer::new(1024));

    let gps = Constellation::GPS;

    let l1 = Observable::PhaseRange("L1".to_string());
    let c1 = Observable::PseudoRange("C1".to_string());
    let s1 = Observable::SSI("S1".to_string());
    let p1 = Observable::PseudoRange("P1".to_string());

    let l2 = Observable::PhaseRange("L2".to_string());
    let c2 = Observable::PseudoRange("C2".to_string());
    let s2 = Observable::SSI("S2".to_string());
    let p2 = Observable::PseudoRange("P2".to_string());

    let l5 = Observable::PhaseRange("L5".to_string());
    let c5 = Observable::PseudoRange("C5".to_string());
    let s5 = Observable::SSI("S5".to_string());

    let gps_codes = vec![
        c1.clone(),
        c2.clone(),
        c5.clone(),
        l1.clone(),
        l2.clone(),
        l5.clone(),
        p1.clone(),
        p2.clone(),
        s1.clone(),
        s2.clone(),
        s5.clone(),
    ];

    let mut hd = HeaderFields::default()
        .with_time_of_first_obs(Epoch::from_str("2020-01-01T00:00:00 GPST").unwrap())
        .with_time_of_last_obs(Epoch::from_str("2020-01-01T23:30:00 GPST").unwrap());

    hd.codes.insert(gps, gps_codes);

    hd.format(&mut buf, 2).unwrap();

    let content = buf.into_inner().unwrap().to_ascii_utf8();

    generic_formatted_lines_test(
        &content,
        HashMap::from_iter([
            (
                0,
                "  2020     1     1     0     0    0.0000000     GPS         TIME OF FIRST OBS",
            ),
            (
                1,
                "  2020     1     1    23    30    0.0000000     GPS         TIME OF LAST OBS",
            ),
            (
                2,
                "    11    C1    C2    C5    L1    L2    L5    P1    P2    S1# / TYPES OF OBSERV",
            ),
            (
                3,
                "          S2    S5                                          # / TYPES OF OBSERV",
            ),
        ]),
    );
}

#[test]
fn obs_v3_header_formatting() {
    let mut buf = BufWriter::new(Utf8Buffer::new(1024));

    let gps = Constellation::GPS;
    let gal = Constellation::Galileo;
    let glo = Constellation::Glonass;
    let bds = Constellation::BeiDou;

    let l1c = Observable::PhaseRange("L1C".to_string());
    let c1c = Observable::PseudoRange("C1C".to_string());
    let d1c = Observable::Doppler("D1C".to_string());
    let s1c = Observable::SSI("S1C".to_string());

    let l2c = Observable::PhaseRange("L2C".to_string());
    let c2c = Observable::PseudoRange("C2C".to_string());
    let d2c = Observable::Doppler("D2C".to_string());
    let s2c = Observable::SSI("S2C".to_string());

    let l5c = Observable::PhaseRange("L5C".to_string());
    let c5c = Observable::PseudoRange("C5C".to_string());
    let d5c = Observable::Doppler("D5C".to_string());
    let s5c = Observable::SSI("S5C".to_string());

    let l5q = Observable::PhaseRange("L5Q".to_string());
    let c5q = Observable::PseudoRange("C5Q".to_string());
    let d5q = Observable::Doppler("D5Q".to_string());
    let s5q = Observable::SSI("S5Q".to_string());

    let l1p = Observable::PhaseRange("L1P".to_string());
    let l2p = Observable::PhaseRange("L2P".to_string());
    let l1x = Observable::PhaseRange("L1X".to_string());
    let l2x = Observable::PhaseRange("L2X".to_string());

    let gps_codes = vec![
        l1c.clone(),
        c1c.clone(),
        d1c.clone(),
        s1c.clone(),
        l2c.clone(),
        c2c.clone(),
        d2c.clone(),
        s2c.clone(),
        l5c.clone(),
        c5c.clone(),
        d5c.clone(),
        s5c.clone(),
        l1p.clone(),
        l2p.clone(),
    ];

    let gal_codes = vec![
        l1c.clone(),
        c1c.clone(),
        d1c.clone(),
        s1c.clone(),
        l2c.clone(),
        c2c.clone(),
        d2c.clone(),
        s2c.clone(),
        l5q.clone(),
        c5q.clone(),
        d5q.clone(),
        s5q.clone(),
        l1x.clone(),
        l2x.clone(),
        l1p.clone(),
        l2p.clone(),
    ];

    let glo_codes = vec![l1c.clone(), c1c.clone()];

    let bds_codes = vec![
        l1c.clone(),
        c1c.clone(),
        d1c.clone(),
        s1c.clone(),
        l2c.clone(),
        c2c.clone(),
        d2c.clone(),
        s2c.clone(),
        l5c.clone(),
        c5c.clone(),
        d5c.clone(),
        s5c.clone(),
        l1p.clone(),
    ];

    let mut hd = HeaderFields::default()
        .with_time_of_first_obs(Epoch::from_str("2020-01-01T00:00:00 GPST").unwrap())
        .with_time_of_last_obs(Epoch::from_str("2020-01-01T23:00:00 GPST").unwrap());

    hd.codes.insert(gps, gps_codes);
    hd.codes.insert(gal, gal_codes);
    hd.codes.insert(glo, glo_codes);
    hd.codes.insert(bds, bds_codes);

    hd.format(&mut buf, 3).unwrap();

    let content = buf.into_inner().unwrap().to_ascii_utf8();

    generic_formatted_lines_test(
        &content,
        HashMap::from_iter([
            (
                0,
                "  2020     1     1     0     0    0.0000000     GPS         TIME OF FIRST OBS",
            ),
            (
                1,
                "  2020     1     1    23     0    0.0000000     GPS         TIME OF LAST OBS",
            ),
            (
                2,
                "G   14 L1C C1C D1C S1C L2C C2C D2C S2C L5C C5C D5C S5C L1P  SYS / # / OBS TYPES",
            ),
            (
                3,
                "       L2P                                                  SYS / # / OBS TYPES",
            ),
            (
                4,
                "R    2 L1C C1C                                              SYS / # / OBS TYPES",
            ),
            (
                5,
                "C   13 L1C C1C D1C S1C L2C C2C D2C S2C L5C C5C D5C S5C L1P  SYS / # / OBS TYPES",
            ),
            (
                6,
                "E   16 L1C C1C D1C S1C L2C C2C D2C S2C L5Q C5Q D5Q S5Q L1X  SYS / # / OBS TYPES",
            ),
            (
                7,
                "       L2X L1P L2P                                          SYS / # / OBS TYPES",
            ),
        ]),
    );
}

#[test]
fn high_precision_obs() {
    let mut buf = BufWriter::new(Utf8Buffer::new(1024));

    let gps = Constellation::GPS;
    let gal = Constellation::Galileo;
    let l1c = Observable::PhaseRange("L1C".to_string());
    let l5c = Observable::PhaseRange("L5C".to_string());
    let l1x = Observable::PhaseRange("L1X".to_string());
    let l5q = Observable::PhaseRange("L5Q".to_string());

    let mut hd = HeaderFields::default();

    hd.with_scaling(gps, l1c, 10);
    hd.with_scaling(gps, l5c, 20);
    hd.with_scaling(gal, l1x, 30);
    hd.with_scaling(gal, l5q, 40);

    hd.format(&mut buf, 3).unwrap();
}
