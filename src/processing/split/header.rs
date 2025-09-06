use crate::{header::Header, prelude::Epoch};

use qc_traits::Split;

impl Split for Header {
    fn split(&self, epoch: Epoch) -> (Self, Self)
    where
        Self: Sized,
    {
        let (mut a, mut b) = (self.clone(), self.clone());

        if let Some(obs) = &mut a.obs {
            if let Some(timeof) = &mut obs.timeof_first_obs {
                *timeof = std::cmp::min(*timeof, epoch);
            }
            if let Some(timeof) = &mut obs.timeof_last_obs {
                *timeof = std::cmp::max(*timeof, epoch);
            }
        }

        (a, b)
    }

    fn split_even_dt(&self, _dt: hifitime::Duration) -> Vec<Self>
    where
        Self: Sized,
    {
        let ret = Vec::<Self>::new();
        ret
    }

    fn split_mut(&mut self, epoch: Epoch) -> Self {
        let copy = self.clone();
        copy
    }
}
