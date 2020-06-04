use std::io::Cursor;
use std::net::UdpSocket;

use crate::messages;
use crate::metainfo::Metainfo;
use crate::type_ext::PeerId;

use crate::messages::{FromProtocolMsg, ToProtocolMsg};
use anyhow::{anyhow, bail, Context, Result};

use byteorder::{BigEndian, ReadBytesExt};
use log::info;

const LOCAL_UDP_ADDRESS: &str = "0.0.0.0:8000";

pub fn get_peers(metainfo: &Metainfo, peer_id: &PeerId) -> Result<messages::AnnounceResponse> {
    let socket = UdpSocket::bind(LOCAL_UDP_ADDRESS).expect("couldn't bind to address");
    info!("UDP successfully binded to: {}", LOCAL_UDP_ADDRESS);

    let announce_url = metainfo.announce_url();
    let host = announce_url
        .host_str()
        .context("Failed to read host from metainfo announce_url")?;
    let port = announce_url
        .port()
        .context("Failed to read port from metainfo port")?;
    let announce_url = format!("{}:{}", &host, &port);

    // info!("attempting to connect to announce_url: {}", announce_url);
    socket.connect(&announce_url).context(format!(
        "couldn't connect to announce_url: {}",
        announce_url
    ))?;
    info!("successfully connected to url");

    let connect_request = messages::ConnectRequest::new();

    info!("sending connect request: {:?}", connect_request);
    socket
        .send(connect_request.to_protocol_msg()?.as_slice())
        .context("failed to  send connect request")?;
    info!("Successfully sent connect request");

    let mut _bytes_read: usize = 0;
    let mut buf = [0u8; 4];

    while let Ok(_received) = socket.peek(&mut buf) {
        match Cursor::new(buf).read_u32::<BigEndian>()? {
            messages::ConnectResponse::ACTION => {
                let mut buf = [0u8; 16];
                socket.recv(&mut buf)?;
                let response = messages::ConnectResponse::from_protocol_msg(&buf)?;
                info!("got connect_response response: {:?}", response);
                send_announce(&socket, &metainfo, peer_id, &response)?;
            }
            messages::AnnounceResponse::ACTION => {
                let mut buf = [0u8; 512];
                let received = socket.recv(&mut buf)?;
                let response = messages::AnnounceResponse::from_protocol_msg(&buf[..received])?;
                info!("got announce response: {:?}", response);
                return Ok(response);
            }
            unhandled => bail!("Unhandled udp message with action code: {}", unhandled),
        }
    }

    Err(anyhow!("Couldn't read from announce_url"))
}

fn send_announce(
    socket: &UdpSocket,
    metainfo: &Metainfo,
    peer_id: &PeerId,
    connect_response: &messages::ConnectResponse,
) -> Result<()> {
    let announce_request =
        messages::AnnounceRequest::new(metainfo, connect_response, peer_id.to_owned())?;
    info!("sending announce request: {:?}", &announce_request);

    socket
        .send(announce_request.to_protocol_msg()?.as_slice())
        .context("failed to send announce request")?;
    Ok(())
}
