use std::io::Cursor;

extern crate anyhow;
use anyhow::Result;

extern crate rand;
use rand::prelude::*;

extern crate byteorder;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::type_ext::Sha1;

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

    pub fn transaction_id(&self) -> u32 {
        self.transaction_id
    }

    pub fn connection_id(&self) -> u64 {
        self.connection_id
    }
}

impl FromUdpMsg for ConnectResponse {
    fn from_udp_msg(msg: &[u8]) -> Result<ConnectResponse> {
        let mut c = Cursor::new(msg);
        let action = c.read_u32::<BigEndian>()?;
        ensure!(action == ConnectResponse::ACTION, "message had wrong action: {}, expected {}", action, ConnectResponse::ACTION);

        let transaction_id = c.read_u32::<BigEndian>()?;
        let connection_id = c.read_u64::<BigEndian>()?;

        assert_eq!(action, ConnectResponse::ACTION);
        Ok(ConnectResponse { transaction_id, connection_id })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AnnounceRequestEvent {
    None = 0,
    Competed = 1,
    Started = 2,
    Stopped = 3,
}

impl AnnounceRequestEvent {
    fn value(&self) -> u32 {
        *self as u32
    }
}

#[derive(Debug)]
pub struct AnnounceRequest {
    connection_id: u64,
    transaction_id: u32,
    info_hash: Sha1,
    peer_id: Sha1,
    downloaded: u64,
    left: u64,
    uploaded: u64,
    event: AnnounceRequestEvent,
    ip_address: Option<u32>, // default 0??
    key: Option<u32>,
    num_want: Option<i32>,
    port: u16,
}

impl AnnounceRequest {
    const ACTION: u32 = 1;
    const DEFAULT_IP_ADDRESS: u32 = 0;
    const DEFAULT_KEY: u32 = 0; // ??
    const NUM_WANT_ALL: i32 = 0;

    pub fn new(
        connection_id: u64,
        transaction_id: u32,
        info_hash: Sha1,
        peer_id: Sha1,
        downloaded: u64,
        left: u64,
        uploaded: u64,
        event: AnnounceRequestEvent,
        port: u16,
    ) -> AnnounceRequest {
        AnnounceRequest {
            connection_id,
            transaction_id,
            info_hash,
            peer_id,
            downloaded,
            left,
            uploaded,
            event,
            port,
            ip_address: None,
            key: None,
            num_want: None,
        }
    }
}

impl ToUdpMsg for AnnounceRequest {
    fn to_udp_msg(&self) -> Result<Vec<u8>> {
        let mut msg = Vec::new();
        msg.write_u64::<BigEndian>(self.connection_id).unwrap();
        msg.write_u32::<BigEndian>(AnnounceRequest::ACTION).unwrap();
        msg.write_u32::<BigEndian>(self.transaction_id).unwrap();

        msg.extend(self.info_hash.iter());
        msg.extend(self.peer_id.iter());

        msg.write_u64::<BigEndian>(self.downloaded)?;
        msg.write_u64::<BigEndian>(self.left)?;
        msg.write_u64::<BigEndian>(self.uploaded)?;

        msg.write_u32::<BigEndian>(self.event.value())?;
        msg.write_u32::<BigEndian>(self.ip_address.unwrap_or(AnnounceRequest::DEFAULT_IP_ADDRESS))?;
        msg.write_u32::<BigEndian>(self.key.unwrap_or(AnnounceRequest::DEFAULT_KEY))?;
        msg.write_i32::<BigEndian>(self.num_want.unwrap_or(AnnounceRequest::NUM_WANT_ALL))?;
        msg.write_u16::<BigEndian>(self.port)?;

        Ok(msg)
    }
}
