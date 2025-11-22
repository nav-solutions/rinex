use crate::{
    navigation::{BdModel, Ephemeris, IonosphereModel, KbModel, NavKey, NgModel},
    prelude::{
        nav::{Almanac, AzElRange, Orbit},
        Epoch, Rinex, SV,
    },
};

use anise::math::Vector6;

impl Rinex {
    /// Macro to resolve the [Orbit]al state of given satellite [SV] at specificied [Epoch] easily.
    /// This applies to Navigation RINEX files only, the specified [Epoch] and satellite must exist in the record,
    /// and we perform the initial requirements. While this is okay for very few satellites and Epochs, this is quite inefficient
    /// and should not be used to process a complete temporal frame and group of satellites. For a real processing pipeline,
    /// you should:
    ///
    /// - browse the RINEX record and manage the [Ephemeris] pool yourself, a pipelined manner, and use the solver provided
    /// by the [Ephemeris] object.
    /// - operate at a high lever, through our [GNSS-Qc](https://github.com/nav-solutions/gnss-qc) which will wrap the
    /// RINEX library and provide processing pipelines.
    ///
    /// ## Inputs
    /// - satellite: selected [SV] (which must exist)
    /// - epoch: [Epoch] of navigation, which must be within the timeframe of this record.
    /// - max_iteration: maximal number of iteration allowed to reasonnably converge.
    ///
    /// ## Returns
    /// - orbital state: expressed as ECEF [Orbit]
    pub fn nav_satellite_orbital_state(
        &self,
        satellite: SV,
        epoch: Epoch,
        max_iteration: usize,
    ) -> Option<Orbit> {
        let (_, _, eph) = self.nav_ephemeris_selection(satellite, epoch)?;
        eph.resolve_orbital_state(satellite, epoch, max_iteration)
    }

    /// Macro to resolve the [Orbit]al state of given satellite [SV] at specificied [Epoch] easily.
    /// Refer to [Self::nav_satellite_orbital_state].
    ///
    /// ## Inputs
    /// - satellite: selected [SV] (which must exist)
    /// - epoch: [Epoch] of navigation, which must be within the timeframe of this record.
    /// - max_iteration: maximal number of iteration allowed to reasonnably converge.
    ///
    /// ## Returns
    /// - ECEF position and velocity in kilometer, as [Vector6].
    pub fn nav_satellite_position_velocity_km(
        &self,
        satellite: SV,
        epoch: Epoch,
        max_iteration: usize,
    ) -> Option<Vector6> {
        let (_, _, eph) = self.nav_ephemeris_selection(satellite, epoch)?;
        eph.resolve_position_velocity_km(satellite, epoch, max_iteration)
    }

    /// Macro to resolve azimuth, elevation and slant range of desired satellite at desired [Epoch].
    /// This applies to Navigation RINEX files only, the specified [Epoch] and satellite must exist in the record,
    /// and we perform the initial requirements. While this is okay for very few satellites and Epochs, this is quite inefficient
    /// and should not be used to process a complete temporal frame and group of satellites. For a real processing pipeline,
    /// you should:
    ///
    /// - browse the RINEX record and manage the [Ephemeris] pool yourself, a pipelined manner, and use the solver provided
    /// by the [Ephemeris] object.
    /// - operate at a high lever, through our [GNSS-Qc](https://github.com/nav-solutions/gnss-qc) which will wrap the
    /// RINEX library and provide processing pipelines.
    ///
    /// ## Inputs
    /// - satellite: selected [SV] (must exist)
    /// - epoch: [Epoch] of navigation, which must be within the timeframe of this record.
    /// - observer: state of the observer, expressed as an [Orbit]
    /// - almanac: [Almanac] context
    /// - max_iteration: maximal number of iteration allowed to reasonnably converge.
    ///
    /// ## Returns
    /// - [AzElRange] on calculations success
    pub fn nav_satellite_azimuth_elevation_range(
        &self,
        satellite: SV,
        epoch: Epoch,
        observer: Orbit,
        almanac: &Almanac,
        max_iteration: usize,
    ) -> Option<AzElRange> {
        let state = self.nav_satellite_orbital_state(satellite, epoch, max_iteration)?;
        let azelrange = almanac
            .azimuth_elevation_range_sez(state, observer, None, None)
            .ok()?;
        Some(azelrange)
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
        if sv.constellation.is_sbas() {
            self.nav_ephemeris_frames_iter()
                .filter_map(|(k, eph)| {
                    if k.sv == sv {
                        Some((k.epoch, k.epoch, eph))
                    } else {
                        None
                    }
                })
                .min_by_key(|(toc, _, _)| t - *toc)
        } else {
            self.nav_ephemeris_frames_iter()
                .filter_map(|(k, eph)| {
                    if k.sv == sv {
                        if eph.is_valid(sv, t) {
                            if let Some(toe) = eph.toe(k.sv) {
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
