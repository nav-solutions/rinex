use crate::navigation::{BdModel, KbModel};

use ublox::{MgaBdsIonoBuilder, MgaGpsIonoBuilder};

impl BdModel {
    /// Encodes this [BdModel] to UBX MGA-IONO-BDS frame
    pub fn to_ubx_mga_iono_bds(&self) -> [u8; 24] {
        let builder = MgaBdsIonoBuilder {
            msg_type: 0,
            version: 0,
            reserved1: [0, 0],
            reserved2: [0, 0, 0, 0],
            alpha0: self.alpha.0,
            alpha1: self.alpha.1,
            alpha2: self.alpha.2,
            alpha3: self.alpha.3,
            beta0: self.alpha.4,
            beta1: self.alpha.5,
            beta2: self.alpha.6,
            beta3: self.alpha.7,
        };

        builder.into_packet_bytes()
    }
}

impl KbModel {
    /// Encodes this [KbModel] to UBX MGA-IONO-GPS/QZSS frame
    pub fn to_ubx_mga_iono_gps_qzss(&self) -> [u8; 24] {
        let builder = MgaGpsIonoBuilder {
            msg_type: 0,
            version: 0,
            reserved1: [0, 0],
            reserved2: [0, 0, 0, 0],
            alpha0: self.alpha.0,
            alpha1: self.alpha.1,
            alpha2: self.alpha.2,
            alpha3: self.alpha.3,
            beta0: self.beta.0,
            beta1: self.beta.1,
            beta2: self.beta.2,
            beta3: self.beta.3,
        };

        builder.into_packet_bytes()
    }
}
