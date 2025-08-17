use crate::header::Header;

use qc_traits::Split;

impl Split for Header {
    fn split(&self, t: hifitime::Epoch) -> (Self, Self)
    where
        Self: Sized,
    {
        let (mut a, mut b) = (self.clone(), self.clone());

        if let Some(obs) = &mut a.obs {
            if let Some(timeof) = &mut obs.timeof_first_obs {
                *timeof = std::cmp::min(*timeof, t);
            }
            if let Some(timeof) = &mut obs.timeof_last_obs {
                *timeof = std::cmp::max(*timeof, t);
            }
        }

        if let Some(obs) = &mut b.obs {
            if let Some(timeof) = &mut obs.timeof_first_obs {
                *timeof = std::cmp::max(*timeof, t);
            }
        }

        if let Some(doris) = &mut a.doris {
            if let Some(timeof) = &mut doris.timeof_first_obs {
                *timeof = std::cmp::min(*timeof, t);
            }
            if let Some(timeof) = &mut doris.timeof_last_obs {
                *timeof = std::cmp::max(*timeof, t);
            }
        }

        if let Some(obs) = &mut b.doris {
            if let Some(timeof) = &mut obs.timeof_first_obs {
                *timeof = std::cmp::max(*timeof, t);
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

    fn split_mut(&mut self, _t: hifitime::Epoch) -> Self {
        let copy = self.clone();
        copy
    }
}
