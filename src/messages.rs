use std::convert::{TryFrom, TryInto};
use std::io::{Cursor, Read};
use std::net::Ipv4Addr;

use anyhow::{anyhow, ensure, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use log::info;
use rand::prelude::*;

use crate::metainfo::Metainfo;
use crate::type_ext::{PeerId, Sha1};

pub trait ToProtocolMsg {
    fn to_protocol_msg(&self) -> Result<Vec<u8>>;
}

pub trait FromProtocolMsg {
    fn from_protocol_msg(msg: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct PeerUrl {
    ip: Ipv4Addr,
    port: u16,
}

impl PeerUrl {
    pub fn ip(&self) -> Ipv4Addr {
        self.ip
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl FromProtocolMsg for PeerUrl {
    fn from_protocol_msg(msg: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let ip: [u8; 4] = msg[..4].try_into()?;
        let ip: Ipv4Addr = Ipv4Addr::from(ip);
        let port: u16 = Cursor::new(&msg[4..6]).read_u16::<BigEndian>()?;
        Ok(PeerUrl { ip, port })
    }
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
    pub const ACTION: u32 = 0;
    const PROTOCOL_ID: u64 = 0x41727101980;

    pub fn new() -> ConnectRequest {
        ConnectRequest {
            transaction_id: random(),
        }
    }
}

impl ToProtocolMsg for ConnectRequest {
    fn to_protocol_msg(&self) -> Result<Vec<u8>> {
        let mut msg = Vec::new();
        msg.write_u64::<BigEndian>(ConnectRequest::PROTOCOL_ID)?;
        msg.write_u32::<BigEndian>(ConnectRequest::ACTION)?;
        msg.write_u32::<BigEndian>(self.transaction_id)?;
        Ok(msg)
    }
}

#[derive(Debug)]
pub struct ConnectResponse {
    transaction_id: u32,
    connection_id: u64,
}

impl ConnectResponse {
    pub const ACTION: u32 = 0;

    pub fn transaction_id(&self) -> u32 {
        self.transaction_id
    }

    pub fn connection_id(&self) -> u64 {
        self.connection_id
    }
}

impl FromProtocolMsg for ConnectResponse {
    fn from_protocol_msg(msg: &[u8]) -> Result<ConnectResponse> {
        let mut c = Cursor::new(msg);
        let action = c.read_u32::<BigEndian>()?;
        ensure!(
            action == ConnectResponse::ACTION,
            "message had wrong action: {}, expected {}",
            action,
            ConnectResponse::ACTION
        );

        let transaction_id = c.read_u32::<BigEndian>()?;
        let connection_id = c.read_u64::<BigEndian>()?;

        assert_eq!(action, ConnectResponse::ACTION);
        Ok(ConnectResponse {
            transaction_id,
            connection_id,
        })
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

impl TryFrom<u32> for AnnounceRequestEvent {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AnnounceRequestEvent::None),
            1 => Ok(AnnounceRequestEvent::Competed),
            2 => Ok(AnnounceRequestEvent::Started),
            3 => Ok(AnnounceRequestEvent::Stopped),
            _ => Err(anyhow!("No matching enum for u32: {}", value)),
        }
    }
}

#[derive(Debug)]
pub struct AnnounceRequest {
    connection_id: u64,
    transaction_id: u32,
    info_hash: Sha1,
    peer_id: PeerId,
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
    pub const ACTION: u32 = 1;
    pub const DEFAULT_PORT: u16 = 6881;
    const DEFAULT_IP_ADDRESS: u32 = 0;
    const DEFAULT_KEY: u32 = 0; // ??
    const DEFAULT_NUM_WANT: i32 = -1;

    pub fn new(
        metainfo: &Metainfo,
        connect_response: &ConnectResponse,
        peer_id: Sha1,
    ) -> Result<AnnounceRequest> {
        Ok(AnnounceRequest {
            connection_id: connect_response.connection_id,
            transaction_id: connect_response.transaction_id,
            info_hash: metainfo.info_hash().to_owned(),
            peer_id,
            downloaded: 0,
            left: metainfo.info().size(),
            uploaded: 0,
            event: AnnounceRequestEvent::Started,
            port: AnnounceRequest::DEFAULT_PORT,
            ip_address: None,
            key: Some(random()),
            num_want: None,
        })
    }
}

impl ToProtocolMsg for AnnounceRequest {
    fn to_protocol_msg(&self) -> Result<Vec<u8>> {
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
        msg.write_u32::<BigEndian>(
            self.ip_address
                .unwrap_or(AnnounceRequest::DEFAULT_IP_ADDRESS),
        )?;
        msg.write_u32::<BigEndian>(self.key.unwrap_or(AnnounceRequest::DEFAULT_KEY))?;
        msg.write_i32::<BigEndian>(self.num_want.unwrap_or(AnnounceRequest::DEFAULT_NUM_WANT))?;
        msg.write_u16::<BigEndian>(self.port)?;

        Ok(msg)
    }
}

impl FromProtocolMsg for AnnounceRequest {
    fn from_protocol_msg(msg: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let mut c = Cursor::new(msg);
        let connection_id = c.read_u64::<BigEndian>()?;
        let action = c.read_u32::<BigEndian>()?;
        info!("AnnounceRequest::FromProtocolMsg, action: {}", action);

        let transaction_id = c.read_u32::<BigEndian>()?;

        let mut info_hash = [0u8; 20];
        c.read_exact(&mut info_hash)?;

        let mut peer_id = [0u8; 20];
        c.read_exact(&mut peer_id)?;

        let downloaded = c.read_u64::<BigEndian>()?;
        let left = c.read_u64::<BigEndian>()?;
        let uploaded = c.read_u64::<BigEndian>()?;

        let event = AnnounceRequestEvent::try_from(c.read_u32::<BigEndian>()?)?;
        let ip_address = c.read_u32::<BigEndian>()?;
        let key = c.read_u32::<BigEndian>()?;
        let num_want = c.read_i32::<BigEndian>()?;
        let port = c.read_u16::<BigEndian>()?;

        Ok(AnnounceRequest {
            connection_id,
            transaction_id,
            info_hash,
            peer_id,
            downloaded,
            left,
            uploaded,
            event,
            port,
            ip_address: Some(ip_address),
            key: Some(key),
            num_want: Some(num_want),
        })
    }
}

#[derive(Debug)]
pub struct AnnounceResponse {
    transaction_id: u32,
    interval: u32,
    leechers: u32,
    seeders: u32,
    peers: Vec<PeerUrl>,
}

impl AnnounceResponse {
    pub const ACTION: u32 = 1;

    pub fn peers(&self) -> &Vec<PeerUrl> {
        &self.peers
    }
}

impl FromProtocolMsg for AnnounceResponse {
    fn from_protocol_msg(msg: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let mut c = Cursor::new(msg);
        let action = c.read_u32::<BigEndian>()?;
        ensure!(
            action == AnnounceResponse::ACTION,
            "message had wrong action: {}, expected {}",
            action,
            AnnounceResponse::ACTION,
        );

        let transaction_id = c.read_u32::<BigEndian>()?;
        let interval = c.read_u32::<BigEndian>()?;
        let leechers = c.read_u32::<BigEndian>()?;
        let seeders = c.read_u32::<BigEndian>()?;

        let mut peers: Vec<PeerUrl> = Vec::new();
        let mut buf: [u8; 6] = [0u8; 6];
        while let Ok(_) = c.read_exact(&mut buf) {
            peers.push(PeerUrl::from_protocol_msg(&buf)?);
        }

        Ok(AnnounceResponse {
            transaction_id,
            interval,
            leechers,
            seeders,
            peers,
        })
    }
}
