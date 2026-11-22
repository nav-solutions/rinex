use crate::{
    navigation::{BdModel, Ephemeris, IonosphereModel, KbModel, NavKey, NavMessageType, NgModel},
    prelude::{
        nav::{Almanac, AzElRange, Orbit},
        Epoch, Rinex, SV,
    },
};

pub struct SelectedEphemerisRef<'a> {
    /// Satellite
    pub satellite: SV,

    /// Time of clock
    pub toc: Epoch,

    /// Time of Issue of Ephemeris
    pub toe: Epoch,

    /// Navigation message (= signal modulation)
    /// which broadcasted the selected ephemeris frame
    pub message: NavMessageType,

    /// Ephemeris
    pub eph: &'a Ephemeris,
}

impl Rinex {
    /// [SV] orbital state vector determination attempt, that only applies
    /// to Navigation [Rinex].
    /// ## Inputs
    /// - sv: desired [SV]
    /// - t: desired [Epoch] to express the [Orbit]al state
    /// ## Returns
    /// - orbital state: expressed as ECEF [Orbit]
    pub fn sv_orbit(&self, sv: SV, t: Epoch) -> Option<Orbit> {
        let sel = self.nav_ephemeris_selection(sv, t)?;
        sel.eph.kepler2position(sv, t)
    }

    /// [SV] (azimuth, elevation, slant range) triplet determination,
    /// that only applies to Navigation [Rinex].
    /// ## Inputs
    /// - sv: target [SV]
    /// - t: target [Epoch]
    /// - rx_orbit: RX position expressed as an [Orbit]
    /// - almanac: [Almanac] context
    /// ## Returns
    /// - [AzElRange] on calculations success
    pub fn nav_azimuth_elevation_range(
        &self,
        sv: SV,
        t: Epoch,
        rx_orbit: Orbit,
        almanac: &Almanac,
    ) -> Option<AzElRange> {
        let sv_orbit = self.sv_orbit(sv, t)?;
        let azelrange = almanac
            .azimuth_elevation_range_sez(sv_orbit, rx_orbit, None, None)
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
    pub fn nav_ephemeris_selection<'a>(
        &'a self,
        sv: SV,
        t: Epoch,
    ) -> Option<SelectedEphemerisRef<'a>> {
        if sv.constellation.is_sbas() {
            self.nav_ephemeris_frames_iter()
                .filter_map(|(k, eph)| {
                    if k.sv == sv {
                        Some(SelectedEphemerisRef {
                            eph,
                            toc: k.epoch,
                            toe: k.epoch,
                            satellite: k.sv,
                            message: k.msgtype,
                        })
                    } else {
                        None
                    }
                })
                .min_by_key(|sel| t - sel.toc)
        } else {
            self.nav_ephemeris_frames_iter()
                .filter_map(|(k, eph)| {
                    if k.sv == sv {
                        if eph.is_valid(sv, t) {
                            if let Some(toe) = eph.toe(k.sv) {
                                Some(SelectedEphemerisRef {
                                    eph,
                                    toc: k.epoch,
                                    toe: toe,
                                    satellite: k.sv,
                                    message: k.msgtype,
                                })
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
                .min_by_key(|sel| (t - sel.toe).abs())
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
