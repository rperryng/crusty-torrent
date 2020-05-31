use std::io::Cursor;
use std::net::UdpSocket;
use std::convert::TryInto;

use crate::messages;
use crate::messages::{FromUdpMsg, ToUdpMsg};
use crate::metainfo::Metainfo;
use byteorder::{BigEndian, ReadBytesExt};

extern crate url;

extern crate anyhow;
use anyhow::{Context, Result};

extern crate rand;

use crate::util::gen_peer_id;

const UDP_ADDRESS: &str = "0.0.0.0:8000";

pub fn get_peers(metainfo: &Metainfo) -> Result<messages::AnnounceResponse> {
    let socket = UdpSocket::bind(UDP_ADDRESS).expect("couldn't bind to address");
    info!("UDP successfully binded to: {}", UDP_ADDRESS);

    let announce_url = metainfo.announce_url();
    let host = announce_url.host_str().context("announce_url host")?;
    let port = announce_url.port().context("announce_url port")?;
    let announce_url = format!("{}:{}", host, port);

    info!("attempting to connect to announce_url: {}", announce_url);
    socket
        .connect(announce_url)
        .context("couldn't connect to announce_url")?;
    info!("successfully connected to url");

    let connect_request = messages::ConnectRequest::new();

    info!("sending connect request: {:?}", connect_request);
    socket
        .send(connect_request.to_udp_msg()?.as_slice())
        .context("failed to  send connect request")?;
    info!("Successfully sent connect request");

    let mut _bytes_read: usize = 0;
    let mut buf = [0u8; 4];

    while let Ok(_received) = socket.peek(&mut buf) {
        match Cursor::new(buf).read_u32::<BigEndian>()? {
            messages::ConnectResponse::ACTION => {
                let mut buf = [0u8; 16];
                socket.recv(&mut buf)?;
                let response = messages::ConnectResponse::from_udp_msg(&buf)?;
                info!("got connect_response response: {:?}", response);
                send_announce(&socket, &metainfo, &response)?;
            }
            messages::AnnounceResponse::ACTION => {
                let mut buf = [0u8; 512];
                let received = socket.recv(&mut buf)?;
                let response = messages::AnnounceResponse::from_udp_msg(&buf[..received])?;
                info!("got announce response: {:?}", response);
                return Ok(response);
            }
            unhandled => bail!("Unhandled udp message with action code: {}", unhandled),
        }
    }

    Err(anyhow!("Error reading from socket"))
}

fn send_announce(
    socket: &UdpSocket,
    metainfo: &Metainfo,
    connect_response: &messages::ConnectResponse,
) -> Result<()> {
    info!("sending announce request");
    let request = messages::AnnounceRequest::new(
        metainfo,
        connect_response,
        gen_peer_id(),
    )?;

    socket
        .send(request.to_udp_msg()?.as_slice())
        .context("failed to send connect request")?;

    Ok(())
}
