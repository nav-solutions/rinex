#[cfg(test)]
mod test {
    use crate::prelude::*;
    use crate::tests::toolkit::generic_observation_comparison;
    use std::{fs::remove_file as fs_remove_file, path::PathBuf};

    #[test]
    fn crinex1() {
        let pool = vec![
            ("AJAC3550.21D", "AJAC3550.21O"),
            // ("aopr0010.17d", "aopr0010.17o"),
            // ("npaz3550.21d", "npaz3550.21o"),
            // ("wsra0010.21d", "wsra0010.21o"),
            // ("zegv0010.21d", "zegv0010.21o"),
        ];
        for duplet in pool {
            let (crnx_name, rnx_name) = duplet;

            let crnx_path = PathBuf::new()
                .join(env!("CARGO_MANIFEST_DIR"))
                .join("data")
                .join("CRNX")
                .join("V1")
                .join(crnx_name);

            let rnx_path = PathBuf::new()
                .join(env!("CARGO_MANIFEST_DIR"))
                .join("data")
                .join("OBS")
                .join("V2")
                .join(rnx_name);

            let fullpath = rnx_path.to_string_lossy().to_string();

            println!("compressing \"{}\"..", rnx_path.to_string_lossy());
            let rnx = Rinex::from_file(&fullpath).unwrap();
            let dut = rnx.rnx2crnx();

            // dump
            dut.to_file("v1_compressed.txt").unwrap();

            // parse back
            let parsed_back = Rinex::from_file("v1_compressed.txt").unwrap();

            // parse model
            let model_path = crnx_path.to_string_lossy().to_string();
            let model = Rinex::from_file(&model_path).unwrap();

            // run testbench
            generic_observation_comparison(&parsed_back, &model);

            // destroy
            let _ = fs_remove_file("v1_compressed.txt");
        }
    }

    /// The epoch description of a CRINEX 1 file is the RINEX 2 one
    /// (2 digit year, blank padded fields, satellites in file order),
    /// with no padding before the satellite list.
    #[test]
    fn crinex1_epoch_descriptor() {
        let data = PathBuf::new().join(env!("CARGO_MANIFEST_DIR")).join("data");
        let rnx = Rinex::from_file(data.join("OBS").join("V2").join("AJAC3550.21O")).unwrap();

        let tmp = format!("test-crinex1-descriptor-{}.txt", std::process::id());
        rnx.rnx2crnx().to_file(&tmp).unwrap();
        let compressed = std::fs::read_to_string(&tmp).unwrap();
        let _ = fs_remove_file(&tmp);

        let reference =
            std::fs::read_to_string(data.join("CRNX").join("V1").join("AJAC3550.21D")).unwrap();

        let first_descriptor = |content: &str| {
            content
                .lines()
                .find(|line| line.starts_with('&'))
                .unwrap()
                .trim_end()
                .to_string()
        };

        assert_eq!(first_descriptor(&compressed), first_descriptor(&reference));
    }

    #[test]
    fn crinex3() {
        let pool = vec![
            (
                "ACOR00ESP_R_20213550000_01D_30S_MO.crx",
                "ACOR00ESP_R_20213550000_01D_30S_MO.rnx",
            ),
            ("DUTH0630.22D", "DUTH0630.22O"),
            ("VLNS0010.22D", "VLNS0010.22O"),
            ("VLNS0630.22D", "VLNS0630.22O"),
            ("flrs0010.12d", "flrs0010.12o"),
            ("pdel0010.21d", "pdel0010.21o"),
            // receiver clock offsets, with resets and gaps
            (
                "ACRG00GHA_R_20240010000_01H_30S_MO.crx",
                "ACRG00GHA_R_20240010000_01H_30S_MO.rnx",
            ),
        ];
        for duplet in pool {
            let (crnx_name, rnx_name) = duplet;

            let crnx_path = PathBuf::new()
                .join(env!("CARGO_MANIFEST_DIR"))
                .join("data")
                .join("CRNX")
                .join("V3")
                .join(crnx_name);

            let rnx_path = PathBuf::new()
                .join(env!("CARGO_MANIFEST_DIR"))
                .join("data")
                .join("OBS")
                .join("V3")
                .join(rnx_name);

            let fullpath = rnx_path.to_string_lossy().to_string();

            let rnx = Rinex::from_file(&fullpath).unwrap();

            println!("compressing \"{}\"..", rnx_path.to_string_lossy());
            let dut = rnx.rnx2crnx();

            // dump
            dut.to_file("v3_compressed.txt").unwrap();

            // parse back
            let parsed_back = Rinex::from_file("v3_compressed.txt").unwrap();

            // parse model
            let model_path = crnx_path.to_string_lossy().to_string();
            let model = Rinex::from_file(&model_path).unwrap();

            // run testbench
            generic_observation_comparison(&parsed_back, &model);

            // destroy
            let _ = fs_remove_file("v3_compressed.txt");
        }
    }
}
