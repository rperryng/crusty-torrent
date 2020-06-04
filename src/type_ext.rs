use anyhow::Result;

use crate::messages::ToProtocolMsg;

pub type PeerId = [u8; 20];
pub type Sha1 = [u8; 20];

impl ToProtocolMsg for Sha1 {
    fn to_protocol_msg(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }
}
