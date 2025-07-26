use crate::prelude::{Duration, Epoch, Header, Record, Rinex};

use gnss_qc_traits::Split;

mod clock;
mod doris;
mod header;
mod meteo;
mod nav;
mod obs;
mod production;

use obs::{split as obs_split, split_even_dt as obs_split_even_dt, split_mut as obs_split_mut};

use nav::{split as nav_split, split_even_dt as nav_split_even_dt, split_mut as nav_split_mut};

use meteo::{
    split as meteo_split, split_even_dt as meteo_split_even_dt, split_mut as meteo_split_mut,
};

use clock::{
    split as clock_split, split_even_dt as clock_split_even_dt, split_mut as clock_split_mut,
};

use doris::{
    split as doris_split, split_even_dt as doris_split_even_dt, split_mut as doris_split_mut,
};

impl Split for Rinex {
    fn split(&self, t: Epoch) -> (Self, Self) {
        let (h0, h1) = self.header.split(t);
        let (p0, p1) = self.production.split(t);

        let (r0, r1) = if let Some(r) = self.record.as_obs() {
            let (r0, r1) = obs_split(r, t);
            (Record::ObsRecord(r0), Record::ObsRecord(r1))
        } else if let Some(r) = self.record.as_nav() {
            let (r0, r1) = nav_split(r, t);
            (Record::NavRecord(r0), Record::NavRecord(r1))
        } else if let Some(r) = self.record.as_clock() {
            let (r0, r1) = clock_split(r, t);
            (Record::ClockRecord(r0), Record::ClockRecord(r1))
        } else if let Some(r) = self.record.as_doris() {
            let (r0, r1) = doris_split(r, t);
            (Record::DorisRecord(r0), Record::DorisRecord(r1))
        } else if let Some(r) = self.record.as_meteo() {
            let (r0, r1) = meteo_split(r, t);
            (Record::MeteoRecord(r0), Record::MeteoRecord(r1))
        } else {
            (
                Record::ObsRecord(Default::default()),
                Record::ObsRecord(Default::default()),
            )
        };

        // TODO: improve this
        //  split comments timewise
        //  implement Split for Header
        //  implement Split for production attributes
        (
            Rinex {
                record: r0,
                header: h0,
                production: p0,
                comments: self.comments.clone(),
            },
            Rinex {
                record: r1,
                header: h1,
                production: p1,
                comments: self.comments.clone(),
            },
        )
    }

    fn split_mut(&mut self, t: Epoch) -> Self {
        self.header.program = Some(format!(
            "rs-rust v{}",
            Header::format_pkg_version(env!("CARGO_PKG_VERSION"),)
        ));

        let record = if let Some(r) = self.record.as_mut_obs() {
            Record::ObsRecord(obs_split_mut(r, t))
        } else if let Some(r) = self.record.as_mut_nav() {
            Record::NavRecord(nav_split_mut(r, t))
        } else if let Some(r) = self.record.as_mut_clock() {
            Record::ClockRecord(clock_split_mut(r, t))
        } else if let Some(r) = self.record.as_mut_doris() {
            Record::DorisRecord(doris_split_mut(r, t))
        } else if let Some(r) = self.record.as_mut_meteo() {
            Record::MeteoRecord(meteo_split_mut(r, t))
        } else {
            self.record.clone()
        };

        // TODO: improve this
        //  split comments timewise
        //  implement Split for Header ?
        //  implement Split for production attributes ?
        Self {
            record,
            header: self.header.clone(),
            comments: self.comments.clone(),
            production: self.production.clone(),
        }
    }

    fn split_even_dt(&self, dt: Duration) -> Vec<Self> {
        let records = if let Some(r) = self.record.as_obs() {
            obs_split_even_dt(r, dt)
                .into_iter()
                .map(|rec| Record::ObsRecord(rec))
                .collect::<Vec<_>>()
        } else if let Some(r) = self.record.as_clock() {
            clock_split_even_dt(r, dt)
                .into_iter()
                .map(|rec| Record::ClockRecord(rec))
                .collect::<Vec<_>>()
        } else if let Some(r) = self.record.as_meteo() {
            meteo_split_even_dt(r, dt)
                .into_iter()
                .map(|rec| Record::MeteoRecord(rec))
                .collect::<Vec<_>>()
        } else if let Some(r) = self.record.as_doris() {
            doris_split_even_dt(r, dt)
                .into_iter()
                .map(|rec| Record::DorisRecord(rec))
                .collect::<Vec<_>>()
        } else if let Some(r) = self.record.as_nav() {
            nav_split_even_dt(r, dt)
                .into_iter()
                .map(|rec| Record::NavRecord(rec))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // TODO:
        // comments (timewise) should be split
        // header section could be split as well:
        //  impl split_event_dt on Header directly

        records
            .iter()
            .map(|rec| Rinex {
                header: self.header.clone(),
                comments: self.comments.clone(),
                production: self.production.clone(),
                record: rec.clone(),
            })
            .collect()
    }
}
