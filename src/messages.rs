use std::io::Cursor;

extern crate anyhow;
use anyhow::Result;

extern crate rand;
use rand::prelude::*;

extern crate byteorder;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub trait ToUdpMsg {
    fn to_udp_msg(&self) -> Result<Vec<u8>>;
}

pub trait FromUdpMsg {
    fn from_udp_msg( msg: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

pub enum Message {
    ConnectRequest(ConnectRequest),
    ConnectResponse(ConnectResponse),
}

#[derive(Debug)]
pub struct ConnectRequest {
    transaction_id: u32,
}

impl ConnectRequest {
    const PROTOCOL_ID: u64 = 0x41727101980;
    const ACTION: u32 = 0;

    pub fn new() -> ConnectRequest {
        ConnectRequest { transaction_id: random() }
    }
}

impl ToUdpMsg for ConnectRequest {
    fn to_udp_msg(&self) -> Result<Vec<u8>> {
        let mut msg = Vec::new();
        msg.write_u64::<BigEndian>(ConnectRequest::PROTOCOL_ID).unwrap();
        msg.write_u32::<BigEndian>(ConnectRequest::ACTION).unwrap();
        msg.write_u32::<BigEndian>(self.transaction_id).unwrap();
        Ok(msg)
    }
}

#[derive(Debug)]
pub struct ConnectResponse {
    transaction_id: u32,
    connection_id: u64,
}

impl ConnectResponse {
    const ACTION: u32 = 0;
}

impl FromUdpMsg for ConnectResponse {
    fn from_udp_msg(msg: &[u8]) -> Result<ConnectResponse> {
        let mut c = Cursor::new(msg);
        let action = c.read_u32::<BigEndian>()?;
        let transaction_id = c.read_u32::<BigEndian>()?;
        let connection_id = c.read_u64::<BigEndian>()?;

        assert_eq!(action, ConnectResponse::ACTION);
        Ok(ConnectResponse { transaction_id, connection_id })
    }
}
