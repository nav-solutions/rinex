use thiserror::Error;

use gnss_rs::{
    constellation::ParsingError as ConstellationParsingError, cospar::Error as CosparParsingError,
    domes::Error as DOMESParsingError, sv::ParsingError as SVParsingError,
};

use hifitime::{HifitimeError, ParsingError as HifitimeParsingError};

use std::io::Error as IoError;

use crate::hatanaka::Error as HatanakaError;

/// Errors that may rise in Parsing process
#[derive(Debug, Error)]
pub enum ParsingError {
    #[error("header line too short (invalid)")]
    HeaderLineTooShort,
    #[error("empty epoch")]
    EmptyEpoch,
    #[error("invalid epoch flag")]
    EpochFlag,
    #[error("number of sat")]
    NumSat,
    #[error("marker type")]
    MarkerType,
    #[error("nav: clock parsing")]
    ClockParsing,
    #[error("invalid epoch format")]
    EpochFormat,
    #[error("epoch parsing")]
    EpochParsing,
    #[error("datime parsing")]
    DatetimeParsing,
    #[error("datime invalid format")]
    DatetimeFormat,
    #[error("bad rinex revision format")]
    VersionFormat,
    #[error("rinex revision parsing")]
    VersionParsing,
    #[error("rinex format identification")]
    TypeParsing,
    #[error("observable parsing")]
    ObservableParsing,
    #[error("constellation parsing")]
    ConstellationParsing(#[from] ConstellationParsingError),
    #[error("undefined constellation: bad header?")]
    UndefinedConstellation,
    #[error("sv parsing")]
    SVParsing(#[from] SVParsingError),
    #[error("cospar parsing")]
    COSPAR(#[from] CosparParsingError),
    #[error("nav: eop missing line")]
    EopMissingData,
    #[error("clock TYPE OF DATA parsing")]
    ClockTypeofData,
    #[error("OBS RINEX invalid timescale")]
    BadObsBadTimescaleDefinition,
    #[error("bad RINEX: missing timescale specs")]
    NoTimescaleDefinition,
    #[error("SYS / SCALE FACTOR parsing")]
    SystemScalingFactor,
    #[error("REF CLOCK OFFS parsing")]
    RcvClockOffsApplied,
    #[error("header coordinates parsing")]
    Coordinates,
    #[error("antenna coordinates parsing")]
    AntennaCoordinates,
    #[error("sensor coordinates parsing")]
    SensorCoordinates,
    #[error("ANTEX version parsing")]
    AntexVersion,
    #[error("IONEX version parsing")]
    IonexVersion,
    #[error("invalid ionex referencing")]
    IonexReferenceSystem,
    #[error("non supported rinex revision")]
    NonSupportedVersion,
    #[error("invalid mapping function")]
    IonexMappingFunction,
    #[error("unknown / non supported observable")]
    UnknownObservable,
    #[error("invalid observable")]
    BadObservable,
    #[error("invalid leap second specs")]
    LeapFormat,
    #[error("leap second parsing")]
    LeapParsing,
    #[error("hifitime parsing")]
    HifitimeParsing(#[from] HifitimeParsingError),
    #[error("hifitime error")]
    Hifitime(#[from] HifitimeError),
    #[error("DORIS L1/L2 date offset")]
    DorisL1L2DateOffset,
    #[error("DORIS L1/L2 date offset")]
    NumberOfCalibratedAntennasParsing,
    #[error("antex: antenna calibration #")]
    AntexAntennaCalibrationNumber,
    #[error("antex: apc coordinates")]
    AntexAPCCoordinates,
    #[error("antex: zenith grid")]
    AntexZenithGrid,
    #[error("antex: frequency")]
    AntexFrequency,
    #[error("doris: invalid station format")]
    DorisStationFormat,
    #[error("doris: station parsing")]
    DorisStation,
    #[error("obs/doris: missing observable specs")]
    MissingObservableDefinition,
    #[error("clock profile type parsing")]
    ClockProfileType,
    #[error("clock profile parsing")]
    ClockProfile,
    #[error("DOMES parsing")]
    DOMES(#[from] DOMESParsingError),
    #[error("ionex: map index parsing")]
    IonexMapIndex,
    #[error("ionex: grid specs parsing")]
    IonexGridSpecs,
    #[error("ionex: invalid grid specs")]
    BadIonexGridSpecs,
    #[error("ionex: map coordinates parsing")]
    IonexGridCoordinates,
    #[error("nav: invalid frame class")]
    NavFrameClass,
    #[error("nav: invalid timescale")]
    NavInvalidTimescale,
    #[error("nav: invalid message type")]
    NavMsgType,
    #[error("nav: (ref) epoch week counter parsing")]
    NavEpochWeekCounter,
    #[error("nav: time offset parsing")]
    NavTimeOffsetParinsg,
    #[error("nav: unknown radio message")]
    NoNavigationDefinition,
    #[error("nav: invalid health flag definition")]
    NavHealthFlagDefinition,
    #[error("nav: invalid bitfield")]
    NavFlagsMapping,
    #[error("nav: invalid data source flag definition")]
    NavDataSourceDefinition,
    #[error("nav: unknown complex type")]
    NavUnknownComplexType,
    #[error("nav: invalid / missing flag definition")]
    NavFlagsDefinition,
    #[error("nav: illegal null orbit field")]
    NavNullOrbit,
    #[error("nav:ion klobuchar data")]
    KlobucharData,
    #[error("nav:ion nequick-g data")]
    NequickGData,
    #[error("nav:ion bdgim data")]
    BdgimData,
    #[error("nav:sto data")]
    SystemTimeData,
    #[error("ionex: earth obs sat")]
    IonexEarthObservationSat,
    #[error("ionex: model")]
    IonexModel,
    #[error("antex: calibration method")]
    AntexCalibrationMethod,
    #[error("obs: hardware events not supported yet")]
    ObsHardwareEvent,
    #[error("obs: bad v2 satellites description")]
    BadV2SatellitesDescription,
    #[error("obs: numsat parsing")]
    NumSatParsing,
    #[error("CRINEX error: {0}")]
    CRINEX(HatanakaError),
    #[error("bad utf-8 generated by CRINEX recovering process")]
    BadUtf8Crinex,
    #[error("doris station identification")]
    DorisStationIdentification,
    #[error("doris sat clock parsing")]
    DorisClockParsing,
    #[error("ionex scaling exponent")]
    IonexScalingExponent,
}

/// Errors that may rise in Formatting process
#[derive(Error, Debug)]
pub enum FormattingError {
    #[error("i/o: output error")]
    OutputError(#[from] IoError),

    #[error("missing constellation information")]
    UndefinedConstellation,

    #[error("missing navigation standard specs")]
    MissingNavigationStandards,

    #[error("undefined observables")]
    UndefinedObservables,

    #[error("missing observable definition")]
    MissingObservableDefinition,

    #[error("nav: unknown radio message")]
    NoNavigationDefinition,

    #[error("nav: missing grid defs")]
    NoGridDefinition,
}

/// General error (processing, analysis..)
#[derive(Debug)]
pub enum Error {
    /// Invalid frequency
    InvalidFrequency,

    /// Sampling Period is not determined
    UndeterminedSamplingPeriod,

    /// Unknown frequency
    UnknownFrequency,

    /// Non supported GPS [Observable]
    UnknownGPSObservable,

    /// Non supported Galileo [Observable]
    UnknownGalieoObservable,

    /// Non supported Glonass [Observable]
    UnknownGlonassObservable,

    /// Non supported QZSS [Observable]
    UnknownQZSSObservable,

    /// Non supported BDS [Observable]
    UnknownBeiDouObservable,

    /// Non supported IRNSS [Observable]
    UnknownIRNSSObservable,

    /// Non supported SBAS [Observable]
    UnknownSBASObservable,

    /// Non supported DORIS [Observable]
    UnknownDORISObservable,

    /// Unknown GPS Frequency
    UnknownGPSFrequency,

    /// Unknown Galileo Frequency
    UnknownGalileoFrequency,

    /// Unknown QZSS Frequency
    UnknownQzssFrequency,

    /// Unknown Glonass Frequency
    UnknownGlonassFrequency,

    /// Unknown BDS Frequency
    UnknownBDSFrequency,

    /// Unknown IRNSS Frequency
    UnknownIRNSSFrequency,

    /// Unknown SBAS Frequency
    UnknownSBASFrequency,

    /// Unknown DORIS Frequency
    UnknownDORISFrequency,
}
