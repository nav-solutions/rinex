use crate::{
    navigation::{
        timescale, BdModel, EarthOrientation, Ephemeris, GloCdmaModel, IonosphereModel, KbModel,
        KbRegionCode, NavFrame, NavFrameType, NavKey, NavMessageSubtype, NavMessageType,
        NavicKbModel, NavicNeqnModel, NgModel, TimeOffset,
    },
    prelude::{Constellation, Epoch, ParsingError, SV},
    utils::validate_ascii,
};

/// ([NavKey], [NavFrame]) parsing attempt for a V4 frame.
/// In modern Navigation, all forms may exist.
pub fn parse(content: &str) -> Result<(NavKey, NavFrame), ParsingError> {
    validate_ascii(content, 1)?;
    let mut lines = content.lines();

    let line = match lines.next() {
        Some(l) => l,
        _ => return Err(ParsingError::EmptyEpoch),
    };

    if line.len() < 10 {
        return Err(ParsingError::EpochFormat);
    }

    let (_, rem) = line.split_at(2);
    let (class, rem) = rem.split_at(4);
    let (svnn, rem) = rem.split_at(4);

    // frmtype defines message to follow
    let frmtype = class.trim().parse::<NavFrameType>()?;

    // STO, EOP and ION records may name the constellation only
    // (blank PRN): represented by PRN 0.
    let svnn = svnn.trim();
    let sv = if svnn.len() == 1 {
        SV::new(svnn.parse::<Constellation>()?, 0)
    } else {
        svnn.parse::<SV>()?
    };

    // message type, followed by an optional subtype (RINEX 4.02)
    let mut items = rem.split_ascii_whitespace();

    let msgtype = items
        .next()
        .ok_or(ParsingError::NavMsgType)?
        .parse::<NavMessageType>()?;

    let subtype = match items.next() {
        Some(item) => Some(item.parse::<NavMessageSubtype>()?),
        None => None,
    };

    let ts = timescale(sv.constellation)?;

    // Parses navframe type dependent and epoch of publication
    let (epoch, fr) = match frmtype {
        NavFrameType::Ephemeris => {
            let (epoch, payload_sv, ephemeris) = Ephemeris::parse_v4(msgtype, lines, ts)?;
            if payload_sv != sv {
                return Err(ParsingError::NavSvMismatch);
            }
            (epoch, NavFrame::EPH(ephemeris))
        },
        NavFrameType::IonosphereModel => {
            let (epoch, model) = match msgtype {
                NavMessageType::IFNV => {
                    let (epoch, model) = NgModel::parse(lines, ts)?;
                    (epoch, IonosphereModel::NequickG(model))
                },
                // RINEX 4.02: NavIC L1NV models, told apart by the subtype
                NavMessageType::L1NV => match subtype {
                    Some(NavMessageSubtype::KLOB) => {
                        let (epoch, model) = NavicKbModel::parse(lines, ts)?;
                        (epoch, IonosphereModel::NavicKlobuchar(model))
                    },
                    Some(NavMessageSubtype::NEQN) => {
                        let (epoch, model) = NavicNeqnModel::parse(lines, ts)?;
                        (epoch, IonosphereModel::NavicNequick(model))
                    },
                    _ => return Err(ParsingError::NavMsgSubtype),
                },
                // RINEX 4.02: GLONASS CDMA model
                NavMessageType::LXOC => {
                    let (epoch, model) = GloCdmaModel::parse(lines, ts)?;
                    (epoch, IonosphereModel::GlonassCdma(model))
                },
                NavMessageType::CNVX => match sv.constellation {
                    Constellation::BeiDou => {
                        let (epoch, model) = BdModel::parse(lines, ts)?;
                        (epoch, IonosphereModel::Bdgim(model))
                    },
                    _ => {
                        let (epoch, mut model) = KbModel::parse(lines, ts)?;
                        // RINEX 4.02: QZSS region code moved to the subtype field
                        match subtype {
                            Some(NavMessageSubtype::WIDE) => model.region = KbRegionCode::Worldwide,
                            Some(NavMessageSubtype::JAPN) => model.region = KbRegionCode::Japan,
                            _ => {},
                        }
                        (epoch, IonosphereModel::Klobuchar(model))
                    },
                },
                _ => {
                    let (epoch, model) = KbModel::parse(lines, ts)?;
                    (epoch, IonosphereModel::Klobuchar(model))
                },
            };
            (epoch, NavFrame::ION(model))
        },
        NavFrameType::SystemTimeOffset => {
            // grab next lines
            let line_1 = lines.next().ok_or(ParsingError::EmptyEpoch)?;
            let line_2 = lines.next().ok_or(ParsingError::EmptyEpoch)?;
            let time_offset = TimeOffset::parse_v4(line_1, line_2)?;

            let epoch =
                Epoch::from_time_of_week(time_offset.t_ref.0, time_offset.t_ref.1, time_offset.lhs);

            (epoch, NavFrame::STO(time_offset))
        },
        NavFrameType::EarthOrientation => {
            // grab next lines
            let line_1 = lines.next().ok_or(ParsingError::EmptyEpoch)?;
            let line_2 = lines.next().ok_or(ParsingError::EmptyEpoch)?;
            let line_3 = lines.next().ok_or(ParsingError::EmptyEpoch)?;
            let (epoch, eop) = EarthOrientation::parse(line_1, line_2, line_3, ts)?;
            (epoch, NavFrame::EOP(eop))
        },
    };

    let key = NavKey {
        epoch,
        sv,
        msgtype,
        frmtype,
        subtype,
    };

    Ok((key, fr))
}
