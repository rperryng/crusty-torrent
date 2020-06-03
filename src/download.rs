use std::io::prelude::*;
use std::io::{Cursor, Write};
use std::net::{TcpStream, UdpSocket};

use crate::messages::{FromProtocolMsg, PeerUrl, ToProtocolMsg};
use crate::metainfo::Metainfo;
use crate::type_ext::{PeerId, Sha1};

use anyhow::{bail, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};
use log::info;

pub fn download(metainfo: &Metainfo, peer: &PeerUrl, peer_id: &PeerId) -> Result<()> {
    info!("download init");

    let url = format!("{}:{}", peer.ip(), peer.port());
    info!("(TCP) Attempting to connect to: {}", url);
    let mut stream = TcpStream::connect((peer.ip(), peer.port()))?;
    info!("(TCP) connected");

    send_handshake(&mut stream, metainfo, peer_id)?;

    let mut is_handshake = true;
    let mut buf = [0u8; 1024];
    while let Ok(bytes_received) = stream.read(&mut buf) {
        let mut c = Cursor::new(&buf[..]);

        if is_handshake {
            let handshake = Handshake::from_protocol_msg(&buf)?;
            info!("Got handshake: {:?}", handshake);
            info!("Sent info_hash: {:?}", metainfo.info_hash());
            info!("Received info_hash: {:?}", handshake.info_hash());
            info!("Sent peer_id: {:?}", peer_id);
            info!("Received peer_id: {:?}", handshake.peer_id());
            is_handshake = false;
        } else {
            bail!("Received message after handshake; exiting");
        }
    }

    Ok(())
}

fn send_handshake(stream: &mut TcpStream, metainfo: &Metainfo, peer_id: &PeerId) -> Result<()> {
    let handshake = Handshake::new(metainfo.info_hash().to_owned(), peer_id.to_owned());
    stream.write_all(&handshake.to_protocol_msg()?)?;
    Ok(())
}

#[derive(Debug)]
struct Handshake {
    info_hash: Sha1,
    peer_id: PeerId,
}

impl Handshake {
    const PSTR: u8 = 19;
    const PROTOCOL: &'static [u8] = "BitTorrent Protocol".as_bytes();

    fn new(info_hash: Sha1, peer_id: [u8; 20]) -> Self {
        Handshake { info_hash, peer_id }
    }

    fn info_hash(&self) -> Sha1 {
        self.info_hash
    }

    fn peer_id(&self) -> PeerId {
        self.peer_id
    }
}

impl ToProtocolMsg for Handshake {
    fn to_protocol_msg(&self) -> Result<Vec<u8>> {
        let mut msg: Vec<u8> = Vec::with_capacity(68);
        msg.write_u8(Handshake::PSTR)?;
        msg.write_all(Handshake::PROTOCOL)?;
        msg.write_all(&[0u8; 8])?; // reserved
        msg.write_all(&self.info_hash)?;
        msg.write_all(&self.peer_id)?;
        Ok(msg)
    }
}

impl FromProtocolMsg for Handshake {
    fn from_protocol_msg(msg: &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let mut c = Cursor::new(msg);

        let protocol_str_len = c.read_u8()?;
        let mut protocol_str = Vec::with_capacity(protocol_str_len as usize);
        c.read_exact(&mut protocol_str)?;
        if protocol_str != Handshake::PROTOCOL {
            bail!(
                "Returned handshake had unexpected protocol: {}",
                String::from_utf8_lossy(&protocol_str)
            );
        }

        // Reserved bits
        c.read_exact(&mut [0u8; 8])?;

        let mut info_hash: Sha1 = [0u8; 20];
        let mut peer_id: PeerId = [0u8; 20];
        c.read_exact(&mut info_hash)?;
        c.read_exact(&mut peer_id)?;

        Ok(Handshake { info_hash, peer_id })
    }
}
