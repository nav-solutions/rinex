use crate::{
    errors::NavRINEXParsingError,
    navigation::{
        BdModel, EarthOrientation, Ephemeris, IonosphereModel, KbModel, NavFrame, NavFrameType,
        NavKey, NavMessageType, NgModel, TimeOffset,
    },
    prelude::{Constellation, Epoch, SV},
};

/// ([NavKey], [NavFrame]) parsing attempt for a V4 frame.
/// In modern Navigation, all forms may exist.
pub fn parse(content: &str) -> Result<(NavKey, NavFrame), NavRINEXParsingError> {
    let mut lines = content.lines();

    let line = match lines.next() {
        Some(l) => l,
        _ => return Err(NavRINEXParsingError::MissingLine),
    };

    let (_, rem) = line.split_at(2);
    let (class, rem) = rem.split_at(4);
    let (svnn, rem) = rem.split_at(4);

    // frmtype defines message to follow
    let frmtype = class.trim().parse::<NavFrameType>()?;
    let sv = svnn.trim().parse::<SV>()?;
    let msgtype = rem.trim().parse::<NavMessageType>()?;

    let ts = sv
        .constellation
        .timescale()
        .ok_or(NavRINEXParsingError::NoTimescaleDefinition)?;

    // Parses navframe type dependent and epoch of publication
    let (epoch, fr) = match frmtype {
        NavFrameType::Ephemeris => {
            let (epoch, _, ephemeris) = Ephemeris::parse_v4(msgtype, lines, ts)?;
            (epoch, NavFrame::EPH(ephemeris))
        },
        NavFrameType::IonosphereModel => {
            let (epoch, model) = match msgtype {
                NavMessageType::IFNV => {
                    let (epoch, model) = NgModel::parse(lines, ts)?;
                    (epoch, IonosphereModel::NequickG(model))
                },
                NavMessageType::CNVX => match sv.constellation {
                    Constellation::BeiDou => {
                        let (epoch, model) = BdModel::parse(lines, ts)?;
                        (epoch, IonosphereModel::Bdgim(model))
                    },
                    _ => {
                        let (epoch, model) = KbModel::parse(lines, ts)?;
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
            let line_1 = lines.next().ok_or(NavRINEXParsingError::MissingLine)?;
            let line_2 = lines.next().ok_or(NavRINEXParsingError::MissingLine)?;
            let time_offset = TimeOffset::parse_v4(line_1, line_2)?;

            let epoch =
                Epoch::from_time_of_week(time_offset.t_ref.0, time_offset.t_ref.1, time_offset.lhs);

            (epoch, NavFrame::STO(time_offset))
        },
        NavFrameType::EarthOrientation => {
            // grab next lines
            let line_1 = lines.next().ok_or(NavRINEXParsingError::MissingLine)?;
            let line_2 = lines.next().ok_or(NavRINEXParsingError::MissingLine)?;
            let line_3 = lines.next().ok_or(NavRINEXParsingError::MissingLine)?;
            let (epoch, eop) = EarthOrientation::parse(line_1, line_2, line_3, ts)?;
            (epoch, NavFrame::EOP(eop))
        },
    };

    let key = NavKey {
        epoch,
        sv,
        msgtype,
        frmtype,
    };

    Ok((key, fr))
}
