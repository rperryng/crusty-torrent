extern crate anyhow;
use anyhow::Result;

extern crate byteorder;
use byteorder::WriteBytesExt;

use crate::messages::ToUdpMsg;

pub type Sha1 = [u8; 20];

impl ToUdpMsg for Sha1 {
    fn to_udp_msg(&self) -> Result<Vec<u8>> {
        let mut msg = Vec::new();
        for &n in self {
            msg.write_u8(n).unwrap()
        }
        Ok(msg)
    }
}
