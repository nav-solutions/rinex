use crate::{
    navigation::{
        ephemeris::EphemerisError, BdModel, Ephemeris, IonosphereModel, KbModel, NavKey, NgModel,
    },
    prelude::{
        nav::{Almanac, AzElRange, Orbit},
        Constellation, Epoch, Rinex, SV,
    },
};

impl Rinex {
    /// Resolve the [Orbit]al state of this [SV] at desired [Epoch].
    /// This only applies to NAV RINEX.
    ///
    /// ## Inputs
    /// - sv: desired [SV]
    /// - epoch: desired [Epoch]
    /// - max_iter: maximal number of iterations in the solving process.
    ///
    /// ## Returns
    /// - state as [Orbit] (ECEF)
    pub fn nav_satellite_orbit(
        &self,
        sv: SV,
        epoch: Epoch,
        max_iter: usize,
    ) -> Result<Orbit, EphemerisError> {
        let (toc, _, eph) = self
            .nav_ephemeris_selection(sv, epoch)
            .ok_or(EphemerisError::FrameSelectionError(epoch, sv))?;

        eph.sv_orbit(sv, toc, epoch, max_iter)
    }

    /// [SV] (azimuth, elevation, slant range) triplet determination,
    /// that only applies to Navigation [Rinex].
    ///
    /// ## Inputs
    /// - sv: target [SV]
    /// - t: target [Epoch]
    /// - rx_orbit: RX position expressed as an [Orbit]
    /// - almanac: [Almanac] context
    /// - max_iter: maximal number of iterations in the solving process
    ///
    /// ## Returns
    /// - [AzElRange] on calculations success
    pub fn nav_azimuth_elevation_range(
        &self,
        sv: SV,
        t: Epoch,
        rx_orbit: Orbit,
        almanac: &Almanac,
        max_iter: usize,
    ) -> Result<AzElRange, EphemerisError> {
        let sv_orbit = self.nav_satellite_orbit(sv, t, max_iter)?;

        let azelrange = almanac.azimuth_elevation_range_sez(sv_orbit, rx_orbit, None, None)?;

        Ok(azelrange)
    }

    /// Ephemeris selection, that only applies to Navigation [Rinex].
    /// ## Inputs
    /// - sv: desired [SV]
    /// - epoch: desired [Epoch]
    /// ## Returns
    /// - (toc, toe, [Ephemeris]) triplet if an [Ephemeris] message
    /// was decoded in the correct time frame.
    /// Note that `ToE` does not exist for GEO/SBAS [SV], so `ToC` is simply
    /// copied in this case, to maintain the API.
    pub fn nav_ephemeris_selection(&self, sv: SV, t: Epoch) -> Option<(Epoch, Epoch, &Ephemeris)> {
        if sv.constellation.is_sbas() || sv.constellation == Constellation::Glonass {
            self.nav_ephemeris_frames_iter()
                .filter_map(|(k, eph)| {
                    if k.sv == sv {
                        Some((k.epoch, k.epoch, eph))
                    } else {
                        None
                    }
                })
                .min_by_key(|(toc, _, _)| (t - *toc).abs())
        } else {
            self.nav_ephemeris_frames_iter()
                .filter_map(|(k, eph)| {
                    if k.sv == sv {
                        if eph.is_valid(k.sv, k.epoch, t) {
                            if let Ok(toe) = eph.toe(k.sv) {
                                Some((k.epoch, toe, eph))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .min_by_key(|(_, toe, _)| (t - *toe).abs())
        }
    }

    /// Klobuchar [KbModel] Ionosphere model [Iterator].
    /// RINEX V4 is the true application of this, as it provides
    /// regular model updates (reflecting radio message stream).
    /// Klobuchar Ionosphere models exist in RINEX2 and this
    /// method applies similarly.
    pub fn nav_klobuchar_models_iter(&self) -> Box<dyn Iterator<Item = (&NavKey, &KbModel)> + '_> {
        Box::new(
            self.nav_ionosphere_models_iter()
                .filter_map(|(k, v)| match v {
                    IonosphereModel::Klobuchar(model) => Some((k, model)),
                    _ => None,
                }),
        )
    }

    /// BDGIM [BdModel] Ionosphere model [Iterator].
    /// Refer to [Self::nav_klobuchar_models_iter] for similar examples.
    pub fn nav_bdgim_models_iter(&self) -> Box<dyn Iterator<Item = (&NavKey, &BdModel)> + '_> {
        Box::new(
            self.nav_ionosphere_models_iter()
                .filter_map(|(k, v)| match v {
                    IonosphereModel::Bdgim(model) => Some((k, model)),
                    _ => None,
                }),
        )
    }

    /// Nequick-G [NgModel] Ionosphere model [Iterator].
    /// Refer to [Self::nav_klobuchar_models_iter] for similar examples.
    pub fn nav_nequickg_models_iter(&self) -> Box<dyn Iterator<Item = (&NavKey, &NgModel)> + '_> {
        Box::new(
            self.nav_ionosphere_models_iter()
                .filter_map(|(k, v)| match v {
                    IonosphereModel::NequickG(model) => Some((k, model)),
                    _ => None,
                }),
        )
    }
}
