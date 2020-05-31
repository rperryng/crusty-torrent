use std::io::{Cursor, Read};
use std::net::Ipv4Addr;
use std::convert::TryInto;

extern crate anyhow;
use anyhow::Result;

extern crate rand;
use rand::prelude::*;

extern crate byteorder;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

extern crate bendy;
use bendy::encoding::ToBencode;

extern crate sha1;
use sha1::Digest;
use sha1::Sha1 as Sha1Hasher;

extern crate url;
use url::Url;

use crate::type_ext::Sha1;
use crate::metainfo::Metainfo;

pub trait ToUdpMsg {
    fn to_udp_msg(&self) -> Result<Vec<u8>>;
}

pub trait FromUdpMsg {
    fn from_udp_msg(msg: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

#[derive(Debug)]
pub struct PeerUrl {
    ip: Ipv4Addr,
    port: u16,
}

impl FromUdpMsg for PeerUrl {
    fn from_udp_msg(msg: &[u8]) -> Result<Self>
    where Self: Sized {
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
    const PROTOCOL_ID: u64 = 0x41727101980;
    const ACTION: u32 = 0;

    pub fn new() -> ConnectRequest {
        ConnectRequest {
            transaction_id: random(),
        }
    }
}

impl ToUdpMsg for ConnectRequest {
    fn to_udp_msg(&self) -> Result<Vec<u8>> {
        let mut msg = Vec::new();
        msg.write_u64::<BigEndian>(ConnectRequest::PROTOCOL_ID)
            .unwrap();
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
    const DEFAULT_PORT: u16 = 6881;
    const DEFAULT_KEY: u32 = 0; // ??
    const DEFAULT_NUM_WANT: i32 = 50;

    pub fn new(
        metainfo: &Metainfo,
        connect_response: &ConnectResponse,
        peer_id: Sha1,
    ) -> Result<AnnounceRequest> {
        let info_hash: Sha1 = metainfo
            .info()
            .to_bencode()
            .map_err(|e| anyhow!("Failed to re-bencode metainfo: {}", e))
            .map(|bencoded_info| {
                let mut hasher = Sha1Hasher::new();
                hasher.input(bencoded_info);
                hasher.result().into()
            })?;

        Ok(AnnounceRequest {
            connection_id: connect_response.connection_id,
            transaction_id: connect_response.transaction_id,
            info_hash,
            peer_id,
            downloaded: 0,
            left: metainfo.info().size(),
            uploaded: 0,
            event: AnnounceRequestEvent::None,
            port: AnnounceRequest::DEFAULT_PORT,
            ip_address: None,
            key: None,
            num_want: None,
        })
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

#[derive(Debug)]
pub struct AnnounceResponse {
    transaction_id: u32,
    interval: u32,
    leechers: u32,
    seeders: u32,
    peers: Vec::<PeerUrl>,
}

impl AnnounceResponse {
    const ACTION: u32 = 1;
}

impl FromUdpMsg for AnnounceResponse {
    fn from_udp_msg(msg: &[u8]) -> Result<Self>
    where Self: Sized {
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
          peers.push(PeerUrl::from_udp_msg(&buf)?);
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
