use qc_traits::{TimeCorrection, TimeCorrectionsDB};

use crate::prelude::{Duration, Epoch, Rinex};

impl Rinex {
    /// Collect a [TimeCorrectionsDB] from this Navigation [Rinex],
    /// which you can then use for internal or external precise correction.
    /// Does not apply to any other format.
    /// ```
    /// use std::str::FromStr;
    /// use rinex::prelude::{Rinex, Epoch, TimeScale};
    ///
    /// // This file describes GAGP, GAUT, GPUT (only)
    /// // For example: BDT is not available.
    /// let rinex = Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    ///     .unwrap();
    ///
    /// let db = rinex.time_corrections_database()
    ///     .unwrap_or_else(|| {
    ///         panic!("Time corrections should exist for V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz");
    ///     });
    ///
    /// // In its default form, the database applies without any restriction.
    /// for (t_before, t_in, t_after, ts) in [
    ///     (
    ///         "2020-06-24T12:00:00 GPST",
    ///         "2020-06-25T12:00:00 GPST",
    ///         "2020-06-26T01:00:00 GPST",
    ///         TimeScale::UTC,
    ///     ),
    ///     (
    ///         "2020-06-24T12:00:00 GPST",
    ///         "2020-06-25T12:00:00 GPST",
    ///         "2020-06-26T01:00:00 GPST",
    ///         TimeScale::GST,
    ///     ),
    ///     (
    ///         "2020-06-24T12:00:00 GST",
    ///         "2020-06-25T12:00:00 GST",
    ///         "2020-06-26T01:00:00 GST",
    ///         TimeScale::GPST,
    ///     ),
    ///     (
    ///         "2020-06-24T12:00:00 UTC",
    ///         "2020-06-25T12:00:00 UTC",
    ///         "2020-06-26T01:00:00 UTC",
    ///         TimeScale::GST,
    ///     ),
    /// ] {
    ///     let t = Epoch::from_str(t_before).unwrap();
    ///     
    ///     assert!(db.precise_epoch_correction(t, ts)
    ///         .is_some(),
    ///         "precise correction to {} should be feasible @ {}",
    ///         ts,
    ///         t_before);
    ///     
    ///     let t = Epoch::from_str(t_in).unwrap();
    ///     
    ///     assert!(db.precise_epoch_correction(t, ts)
    ///         .is_some(),
    ///         "precise correction to {} should be feasible @ {}",
    ///         ts,
    ///         t_in);
    ///     
    ///     let t = Epoch::from_str(t_after).unwrap();
    ///     
    ///     assert!(db.precise_epoch_correction(t, ts)
    ///         .is_some(),
    ///         "precise correction to {} should be feasible @ {}",
    ///         ts,
    ///         t_after);
    /// }
    ///
    /// // verify that BDT is indeed not available
    /// let t_before = "2020-06-24T12:00:00 BDT";
    /// let t = Epoch::from_str(t_before).unwrap();
    ///
    /// assert!(db.precise_epoch_correction(t, TimeScale::GPST)
    ///     .is_none(),
    ///     "GPST/BDT is not available!",
    /// );
    ///
    /// let t_in = "2020-06-25T12:00:00 BDT";
    /// let t = Epoch::from_str(t_in).unwrap();
    ///
    /// assert!(db.precise_epoch_correction(t, TimeScale::GPST)
    ///     .is_none(),
    ///     "GPST/BDT is not available!",
    /// );
    ///
    /// let t_after = "2020-06-26T01:00:00 BDT";
    /// let t = Epoch::from_str(t_after).unwrap();
    ///
    /// assert!(db.precise_epoch_correction(t, TimeScale::GPST)
    ///     .is_none(),
    ///     "GPST/BDT is not available!",
    /// );
    /// ```
    #[cfg(all(feature = "nav", feature = "processing"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "nav", feature = "processing"))))]
    pub fn time_corrections_database(&self) -> Option<TimeCorrectionsDB> {
        if !self.is_navigation_rinex() {
            return None;
        }

        let mut db = TimeCorrectionsDB::default();

        // collect from possible V3 header
        let header = self.header.nav.as_ref()?;

        let one_day = Duration::from_days(1.0);

        for value in header.time_offsets.iter() {
            let ref_epoch = Epoch::from_time_of_week(value.t_ref.0, value.t_ref.1, value.lhs);

            let correction = TimeCorrection {
                lhs_timescale: value.lhs,
                rhs_timescale: value.rhs,
                ref_epoch,
                validity_period: one_day,
                polynomial: value.to_hifitime_polynomial(),
            };

            db.add(correction);
        }

        // collect from possible V4 frames
        // TODO: don't know how to interprate the ref time
        // let record = self.record.as_nav()?;

        Some(db)
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::{Epoch, Rinex, TimeScale};
    use std::str::FromStr;

    #[test]
    fn test_v3_time_corrections_database() {
        // This file describes GAGP, GAUT, GPUT (only)
        // For example: BDT is not available.
        let rinex =
            Rinex::from_gzip_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz").unwrap();

        let db = rinex.time_corrections_database().unwrap_or_else(|| {
            panic!("Time corrections should exist for V3/BRDC00GOP_R_20210010000_01D_MN.rnx.gz");
        });

        // In its default form, the database applies without any restriction.

        for (t_before, t_in, t_after, ts) in [
            (
                "2020-06-24T12:00:00 GPST",
                "2020-06-25T12:00:00 GPST",
                "2020-06-26T01:00:00 GPST",
                TimeScale::UTC,
            ),
            (
                "2020-06-24T12:00:00 GPST",
                "2020-06-25T12:00:00 GPST",
                "2020-06-26T01:00:00 GPST",
                TimeScale::GST,
            ),
            (
                "2020-06-24T12:00:00 GST",
                "2020-06-25T12:00:00 GST",
                "2020-06-26T01:00:00 GST",
                TimeScale::GPST,
            ),
            (
                "2020-06-24T12:00:00 UTC",
                "2020-06-25T12:00:00 UTC",
                "2020-06-26T01:00:00 UTC",
                TimeScale::GST,
            ),
        ] {
            let t = Epoch::from_str(t_before).unwrap();

            assert!(
                db.precise_epoch_correction(t, ts).is_some(),
                "precise correction to {} should be feasible @ {}",
                ts,
                t_before
            );

            let t = Epoch::from_str(t_in).unwrap();

            assert!(
                db.precise_epoch_correction(t, ts).is_some(),
                "precise correction to {} should be feasible @ {}",
                ts,
                t_in
            );

            let t = Epoch::from_str(t_after).unwrap();

            assert!(
                db.precise_epoch_correction(t, ts).is_some(),
                "precise correction to {} should be feasible @ {}",
                ts,
                t_after
            );
        }

        // verify that BDT is indeed not available
        let t_before = "2020-06-24T12:00:00 BDT";
        let t = Epoch::from_str(t_before).unwrap();

        assert!(
            db.precise_epoch_correction(t, TimeScale::GPST).is_none(),
            "GPST/BDT is not available!",
        );

        let t_in = "2020-06-25T12:00:00 BDT";
        let t = Epoch::from_str(t_in).unwrap();

        assert!(
            db.precise_epoch_correction(t, TimeScale::GPST).is_none(),
            "GPST/BDT is not available!",
        );

        let t_after = "2020-06-26T01:00:00 BDT";
        let t = Epoch::from_str(t_after).unwrap();

        assert!(
            db.precise_epoch_correction(t, TimeScale::GPST).is_none(),
            "GPST/BDT is not available!",
        );

        // Convert to strict database that only propose corrections
        // within their respective validity period.
        // In NAV V3, V2 (prior V4) the corrections were given in the file header,
        // and apply for a 24h time frame.
        let _db = db.strict_validity();

        // TODO
        // // verify that some corrections still apply, but only during
        // // that time frame
        // for (t_before, t_in, t_after, ts) in [
        //     (
        //         "2020-06-24T12:00:00 GPST",
        //         "2020-06-25T12:00:00 GPST",
        //         "2020-06-26T01:00:00 GPST",
        //         TimeScale::UTC,
        //     ),
        //     (
        //         "2020-06-24T12:00:00 GPST",
        //         "2020-06-25T12:00:00 GPST",
        //         "2020-06-26T01:00:00 GPST",
        //         TimeScale::GST,
        //     ),
        //     (
        //         "2020-06-24T12:00:00 GST",
        //         "2020-06-25T12:00:00 GST",
        //         "2020-06-26T01:00:00 GST",
        //         TimeScale::GPST,
        //     ),
        //     (
        //         "2020-06-24T12:00:00 UTC",
        //         "2020-06-25T12:00:00 UTC",
        //         "2020-06-26T01:00:00 UTC",
        //         TimeScale::GST,
        //     ),
        // ] {
        //     let t = Epoch::from_str(t_before).unwrap();
        //
        //     assert!(db.precise_epoch_correction(t, ts)
        //         .is_none(),
        //         "{} correction is not available yet @ {}",
        //         ts,
        //         t_before);
        //
        //     let t = Epoch::from_str(t_in).unwrap();
        //
        //     assert!(db.precise_epoch_correction(t, ts)
        //         .is_some(),
        //         "precise correction to {} should be feasible @ {}",
        //         ts,
        //         t_in);
        //
        //     let t = Epoch::from_str(t_after).unwrap();
        //
        //     assert!(db.precise_epoch_correction(t, ts)
        //         .is_none(),
        //         "{} correction is not available past @ {}",
        //         ts,
        //         t_after);
        // }
    }
}
