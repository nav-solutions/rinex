//! OBS RINEX formatting
use crate::{
    epoch::format as epoch_format,
    error::FormattingError,
    observation::{HeaderFields, ObsKey, Observations},
    prelude::{Constellation, RinexType, SV},
};

use itertools::Itertools;

use std::io::{BufWriter, Write};

impl Observations {
    /// Format [Observations] according to standard RINEX specifications.
    pub fn format<W: Write>(
        &self,
        v2: bool,
        key: &ObsKey,
        header: &HeaderFields,
        w: &mut BufWriter<W>,
    ) -> Result<(), FormattingError> {
        let sv_list = self
            .signals
            .iter()
            .map(|sig| sig.satellite)
            .unique()
            .sorted()
            .collect::<Vec<_>>();

        let numsat = sv_list.len();

        if v2 {
            self.format_v2(w, key, &header, &sv_list, numsat)
        } else {
            self.format_v3(w, key, &header, &sv_list, numsat)
        }
    }

    /// Formats [Observations] according to RINEXv2 standards.
    fn format_v2<W: Write>(
        &self,
        w: &mut BufWriter<W>,
        key: &ObsKey,
        header: &HeaderFields,
        sv_list: &[SV],
        numsat: usize,
    ) -> Result<(), FormattingError> {
        let observables = &header.codes;

        const BLANKING: &str = "                ";
        const OBSERVATIONS_PER_LINE: usize = 5;

        self.format_epoch_v2(w, key, sv_list, numsat)?;

        for sv in sv_list.iter() {
            let sv_observables = observables.get(&sv.constellation);

            let sv_observables = match sv_observables {
                Some(sv_observables) => sv_observables, // correctly identified
                None => {
                    // handles SBAS case
                    if sv.constellation.is_sbas() {
                        match observables.get(&Constellation::SBAS) {
                            Some(sv_observables) => sv_observables,
                            _ => continue, // abort
                        }
                    } else {
                        // abort: normally formatted RINEX
                        // will never end up here
                        continue;
                    }
                },
            };

            let mut modulo = 0;

            // following header specs (strictly)
            for (nth, observable) in sv_observables.iter().enumerate() {
                // retrieve observed signal (if any)
                if let Some(observation) = self
                    .signals
                    .iter()
                    .filter(|sig| &sig.satellite == sv && &sig.observable == observable)
                    .reduce(|k, _| k)
                {
                    write!(w, "{:14.3}", observation.value)?;

                    if let Some(lli) = observation.lli_flags {
                        write!(w, "{:x}", lli)?;
                    } else {
                        write!(w, " ")?;
                    }

                    if let Some(snr) = observation.signal_noise_ratio {
                        write!(w, "{:x}", snr)?;
                    } else {
                        write!(w, " ")?;
                    }
                } else {
                    // Blanking
                    write!(w, "{}", BLANKING)?;
                }

                if (nth % OBSERVATIONS_PER_LINE) == OBSERVATIONS_PER_LINE - 1 {
                    write!(w, "{}", '\n')?;
                }

                modulo = nth % OBSERVATIONS_PER_LINE;
            }

            if modulo != OBSERVATIONS_PER_LINE - 1 {
                write!(w, "{}", '\n')?;
            }
        }
        Ok(())
    }

    /// Format new Epoch according to V2 RINEX format
    pub(crate) fn format_epoch_v2<W: Write>(
        &self,
        w: &mut BufWriter<W>,
        key: &ObsKey,
        sv_list: &[SV],
        numsat: usize,
    ) -> Result<(), FormattingError> {
        const NUM_SV_PER_LINE: usize = 12;
        const NEW_LINE_PADDING: &str = "                                ";

        if let Some(clock) = self.clock {
            write!(
                w,
                " {}  {} {:2}",
                epoch_format(key.epoch, RinexType::ObservationData, 2),
                key.flag,
                numsat,
            )?;
        } else {
            write!(
                w,
                " {}  {} {:2}",
                epoch_format(key.epoch, RinexType::ObservationData, 2),
                key.flag,
                numsat,
            )?;
        }

        for (nth, sv) in sv_list.iter().enumerate() {
            if nth > 0 && (nth % NUM_SV_PER_LINE) == 0 {
                write!(w, "{}", NEW_LINE_PADDING)?;
            }
            write!(w, "{:x}", sv)?;
            if nth < numsat - 1 && nth % NUM_SV_PER_LINE == NUM_SV_PER_LINE - 1 {
                write!(w, "{}", '\n')?;
            }
            if nth == numsat - 1 {
                write!(w, "{}", '\n')?;
            }
        }

        Ok(())
    }

    /// Formats [Observations] according to RINEXv3 standards.
    fn format_v3<W: Write>(
        &self,
        w: &mut BufWriter<W>,
        key: &ObsKey,
        header: &HeaderFields,
        sv_list: &[SV],
        numsat: usize,
    ) -> Result<(), FormattingError> {
        const BLANKING: &str = "                ";

        let observables = &header.codes;

        // retrieve unique constellation list
        let constell_list = sv_list
            .iter()
            .map(|sv| sv.constellation)
            .unique()
            .sorted()
            .collect::<Vec<_>>();

        // encode new epoch
        self.format_epoch_v3(w, key, numsat)?;

        // by sorted constellation then SV
        for constell in constell_list.iter() {
            // by sorted SV
            for sv in sv_list.iter().filter(|sv| sv.constellation == *constell) {
                write!(w, "{:x}", sv)?;

                // following header definitions
                let sv_observables = observables.get(&sv.constellation);

                let sv_observables = match sv_observables {
                    Some(observables) => observables, // correctly identified
                    None => {
                        // handles SBAS case
                        if sv.constellation.is_sbas() {
                            match observables.get(&Constellation::SBAS) {
                                Some(sv_observables) => sv_observables,
                                _ => continue, // abort
                            }
                        } else {
                            // abort: normally formatted RINEX
                            // will never end up here
                            continue;
                        }
                    },
                };

                for observable in sv_observables.iter() {
                    if let Some(observation) = self
                        .signals
                        .iter()
                        .filter(|sig| sig.satellite == *sv && sig.observable == *observable)
                        .reduce(|k, _| k)
                    {
                        write!(w, "{:14.3}", observation.value)?;

                        if let Some(lli) = &observation.lli_flags {
                            write!(w, "{}", lli.bits())?;
                        } else {
                            write!(w, " ")?;
                        }

                        if let Some(snr) = &observation.signal_noise_ratio {
                            write!(w, "{:x}", snr)?;
                        } else {
                            write!(w, " ")?;
                        }
                    } else {
                        write!(w, "{}", BLANKING)?;
                    }
                }
                write!(w, "{}", '\n')?;
            }
        }
        Ok(())
    }

    /// Format new Epoch according to V3 RINEX format
    pub(crate) fn format_epoch_v3<W: Write>(
        &self,
        w: &mut BufWriter<W>,
        key: &ObsKey,
        numsat: usize,
    ) -> Result<(), FormattingError> {
        if let Some(clock) = self.clock {
            writeln!(
                w,
                "> {}  {} {:2}  {:13.4}",
                epoch_format(key.epoch, RinexType::ObservationData, 3),
                key.flag,
                numsat,
                clock.offset_s,
            )?;
        } else {
            writeln!(
                w,
                "> {}  {} {:2}",
                epoch_format(key.epoch, RinexType::ObservationData, 3),
                key.flag,
                numsat,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use crate::{
        observation::{EpochFlag, ObsKey, Observations, SignalObservation},
        prelude::{Epoch, Observable, SV},
    };

    use std::io::BufWriter;
    use std::str::FromStr;

    use itertools::Itertools;

    use crate::tests::formatting::Utf8Buffer;

    #[test]
    fn test_format_epoch_v2() {
        let key = ObsKey {
            flag: EpochFlag::Ok,
            epoch: Epoch::from_str("2017-01-01T00:00:00 GPST").unwrap(),
        };

        let c1c = Observable::from_str("C1C").unwrap();

        let mut buf = BufWriter::new(Utf8Buffer::new(1024));

        let obs = Observations {
            clock: None,
            signals: vec![
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G03").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G08").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G14").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G16").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G22").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G23").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G26").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G27").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G31").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G32").unwrap(),
                },
            ],
        };

        let sv_list = obs
            .signals
            .iter()
            .map(|sig| sig.sv)
            .unique()
            .sorted()
            .collect::<Vec<_>>();

        obs.format_epoch_v2(&mut buf, &key, &sv_list, obs.signals.len())
            .unwrap();

        let content = buf.into_inner().unwrap().to_ascii_utf8();
        assert_eq!(
            content,
            " 17  1  1  0  0  0.0000000  0 10G03G08G14G16G22G23G26G27G31G32\n",
        );

        let obs = Observations {
            clock: None,
            signals: vec![
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("R01").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("R16").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("R17").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("R15").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("R02").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G07").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G15").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G23").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G26").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G13").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G20").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G21").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G18").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("R24").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("R09").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G08").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G27").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G10").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G16").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("R18").unwrap(),
                },
            ],
        };

        let mut buf = BufWriter::new(Utf8Buffer::new(1024));

        let sv_list = obs
            .signals
            .iter()
            .map(|sig| sig.sv)
            .unique()
            .sorted()
            .collect::<Vec<_>>();

        obs.format_epoch_v2(&mut buf, &key, &sv_list, obs.signals.len())
            .unwrap();

        let content = buf.into_inner().unwrap().to_ascii_utf8();
        assert_eq!(
            content,
            " 17  1  1  0  0  0.0000000  0 20R01R02G07G08R09G10G13G15R15G16R16R17\n                                G18R18G20G21G23R24G26G27\n"
        );

        let obs = Observations {
            clock: None,
            signals: vec![
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G07").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G08").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G10").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G16").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G18").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G20").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G21").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G23").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G26").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("G27").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("R09").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("R18").unwrap(),
                },
                SignalObservation {
                    value: 1.0,
                    observable: c1c.clone(),
                    lli: None,
                    snr: None,
                    sv: SV::from_str("R24").unwrap(),
                },
            ],
        };

        let mut buf = BufWriter::new(Utf8Buffer::new(1024));

        let sv_list = obs
            .signals
            .iter()
            .map(|sig| sig.sv)
            .unique()
            .sorted()
            .collect::<Vec<_>>();

        obs.format_epoch_v2(&mut buf, &key, &sv_list, obs.signals.len())
            .unwrap();

        let content = buf.into_inner().unwrap().to_ascii_utf8();
        assert_eq!(
            content,
            " 17  1  1  0  0  0.0000000  0 13G07G08R09G10G16G18R18G20G21G23R24G26\n                                G27\n",
        );

        let mut obs = Observations {
            clock: None,
            signals: Vec::new(),
        };

        for sv in [
            "G07", "G08", "G10", "G13", "G15", "G16", "G18", "G20", "G21", "G23", "G26", "G27",
            "R01", "R02", "R09", "R15", "R16", "R17", "R18", "R24", "C01", "C02", "C03", "C04",
        ] {
            obs.signals.push(SignalObservation {
                value: 1.0,
                observable: c1c.clone(),
                lli: None,
                snr: None,
                sv: SV::from_str(sv).unwrap(),
            });
        }

        let mut buf = BufWriter::new(Utf8Buffer::new(1024));

        let sv_list = obs
            .signals
            .iter()
            .map(|sig| sig.sv)
            .unique()
            .sorted()
            .collect::<Vec<_>>();

        obs.format_epoch_v2(&mut buf, &key, &sv_list, obs.signals.len())
            .unwrap();

        let content = buf.into_inner().unwrap().to_ascii_utf8();
        assert_eq!(
            content,
            " 17  1  1  0  0  0.0000000  0 24R01C01R02C02C03C04G07G08R09G10G13G15\n                                R15G16R16R17G18R18G20G21G23R24G26G27\n"

        );

        //let sv_list = [
        //    SV::from_str("G07").unwrap(),
        //    SV::from_str("G23").unwrap(),
        //    SV::from_str("G26").unwrap(),
        //    SV::from_str("G20").unwrap(),
        //    SV::from_str("G21").unwrap(),
        //    SV::from_str("G18").unwrap(),
        //    SV::from_str("R24").unwrap(),
        //    SV::from_str("R09").unwrap(),
        //    SV::from_str("G08").unwrap(),
        //    SV::from_str("G27").unwrap(),
        //    SV::from_str("G10").unwrap(),
        //    SV::from_str("G16").unwrap(),
        //    SV::from_str("R18").unwrap(),
        //    SV::from_str("G13").unwrap(),
        //    SV::from_str("R01").unwrap(),
        //    SV::from_str("R16").unwrap(),
        //    SV::from_str("R17").unwrap(),
        //    SV::from_str("G15").unwrap(),
        //    SV::from_str("R02").unwrap(),
        //    SV::from_str("R15").unwrap(),
        //    SV::from_str("C01").unwrap(),
        //    SV::from_str("C02").unwrap(),
        //    SV::from_str("C03").unwrap(),
        //    SV::from_str("C04").unwrap(),
        //    SV::from_str("C05").unwrap(),
        //];

        //let mut buf = BufWriter::new(Utf8Buffer::new(1024));
        //format_epoch_v2(&mut buf, &key, &sv_list, None).unwrap();

        //let content = buf.into_inner().unwrap().to_ascii_utf8();
        //assert_eq!(
        //    content,
        //    " 17  1  1  0  0  0.0000000  0 25G07G23G26G20G21G18R24R09G08G27G10G16\n                                R18G13R01R16R17G15R02R15C01C02C03C04\n                                C05\n",
        //);
    }

    #[test]
    fn test_format_epoch_v3() {
        let key = ObsKey {
            flag: EpochFlag::Ok,
            epoch: Epoch::from_str("2021-01-01T00:00:00 GPST").unwrap(),
        };

        let c1c = Observable::from_str("C1C").unwrap();

        let mut obs = Observations {
            clock: None,
            signals: Vec::new(),
        };

        for sv in ["G01", "G02", "G03", "G04"] {
            obs.signals.push(SignalObservation {
                lli: None,
                snr: None,
                sv: SV::from_str(sv).unwrap(),
                value: 1.0,
                observable: c1c.clone(),
            });
        }

        let mut buf = BufWriter::new(Utf8Buffer::new(1024));

        obs.format_epoch_v3(&mut buf, &key, obs.signals.len())
            .unwrap();

        let content = buf.into_inner().unwrap().to_ascii_utf8();
        assert_eq!(content, "> 2021 01 01 00 00  0.0000000  0  4\n",);

        for sv in ["C01", "C02", "C03", "C04", "C05", "C06"] {
            obs.signals.push(SignalObservation {
                lli: None,
                snr: None,
                sv: SV::from_str(sv).unwrap(),
                value: 1.0,
                observable: c1c.clone(),
            });
        }

        let mut buf = BufWriter::new(Utf8Buffer::new(1024));

        obs.format_epoch_v3(&mut buf, &key, obs.signals.len())
            .unwrap();

        let content = buf.into_inner().unwrap().to_ascii_utf8();

        assert_eq!(content, "> 2021 01 01 00 00  0.0000000  0 10\n",);
    }
}
