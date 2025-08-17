use crate::{
    observation::{ObsKey, Observations},
    prelude::{Duration, Header, Rinex, TimeScale},
};

use qc_traits::{TimeCorrectionError, TimeCorrectionsDB, Timeshift};

use std::collections::BTreeMap;

impl Timeshift for Header {
    fn timeshift(&self, timescale: TimeScale) -> Self
    where
        Self: Sized,
    {
        let mut s = self.clone();
        s.timeshift_mut(timescale);
        s
    }

    fn timeshift_mut(&mut self, timescale: TimeScale) {
        let one_us = Duration::from_microseconds(1.0);

        if let Some(obs) = &mut self.obs {
            if let Some(epoch) = &mut obs.timeof_first_obs {
                *epoch = epoch.to_time_scale(timescale).round(one_us);
            }

            if let Some(epoch) = &mut obs.timeof_last_obs {
                *epoch = epoch.to_time_scale(timescale).round(one_us);
            }
        }
    }

    fn precise_correction(
        &self,
        db: &TimeCorrectionsDB,
        timescale: TimeScale,
    ) -> Result<Self, TimeCorrectionError>
    where
        Self: Sized,
    {
        let mut s = self.clone();
        s.precise_correction_mut(db, timescale)?;
        Ok(s)
    }

    fn precise_correction_mut(
        &mut self,
        db: &TimeCorrectionsDB,
        timescale: TimeScale,
    ) -> Result<(), TimeCorrectionError> {
        let one_us = Duration::from_microseconds(1.0);

        if let Some(obs) = &mut self.obs {
            if let Some(epoch) = &mut obs.timeof_first_obs {
                *epoch = db
                    .precise_epoch_correction(*epoch, timescale)
                    .ok_or(TimeCorrectionError::NoCorrectionAvailable(
                        epoch.time_scale,
                        timescale,
                    ))?
                    .round(one_us);
            }

            if let Some(epoch) = &mut obs.timeof_last_obs {
                *epoch = db
                    .precise_epoch_correction(*epoch, timescale)
                    .ok_or(TimeCorrectionError::NoCorrectionAvailable(
                        epoch.time_scale,
                        timescale,
                    ))?
                    .round(one_us);
            }
        }

        Ok(())
    }
}

impl Timeshift for Rinex {
    fn timeshift(&self, timescale: TimeScale) -> Self
    where
        Self: Sized,
    {
        let mut s = self.clone();
        s.timeshift_mut(timescale);
        s
    }

    fn timeshift_mut(&mut self, timescale: TimeScale) {
        self.header.timeshift_mut(timescale);

        if let Some(obs_rec) = self.record.as_mut_obs() {
            let mut new_rec = BTreeMap::<ObsKey, Observations>::new();

            for (k, values) in obs_rec.iter() {
                let transposed = k.epoch.to_time_scale(timescale);
                let mut key = k.clone();
                key.epoch = transposed;
                new_rec.insert(key, values.clone());
            }

            *obs_rec = new_rec;
        }
    }

    fn precise_correction(
        &self,
        db: &TimeCorrectionsDB,
        timescale: TimeScale,
    ) -> Result<Self, TimeCorrectionError>
    where
        Self: Sized,
    {
        let mut s = self.clone();
        s.precise_correction_mut(db, timescale)?;
        Ok(s)
    }

    fn precise_correction_mut(
        &mut self,
        db: &TimeCorrectionsDB,
        timescale: TimeScale,
    ) -> Result<(), TimeCorrectionError> {
        self.header.precise_correction_mut(db, timescale)?;

        if let Some(obs_rec) = self.record.as_mut_obs() {
            let mut new_rec = BTreeMap::<ObsKey, Observations>::new();

            for (k, values) in obs_rec.iter() {
                let transposed = db.precise_epoch_correction(k.epoch, timescale).ok_or(
                    TimeCorrectionError::NoCorrectionAvailable(k.epoch.time_scale, timescale),
                )?;

                let mut key = k.clone();
                key.epoch = transposed;
                new_rec.insert(key, values.clone());
            }

            *obs_rec = new_rec;
        }

        Ok(())
    }
}
