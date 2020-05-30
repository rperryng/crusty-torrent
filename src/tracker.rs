use std::io::Cursor;
use std::net::UdpSocket;

use crate::messages;
use crate::messages::{FromUdpMsg, ToUdpMsg};
use crate::metainfo::Metainfo;
use byteorder::{BigEndian, ReadBytesExt};

extern crate url;

extern crate anyhow;
use anyhow::{Context, Result};

extern crate rand;

extern crate sha1;
use sha1::{Digest, Sha1};

use crate::util::gen_peer_id;

const UDP_ADDRESS: &str = "0.0.0.0:8000";
const DEFAULT_ANNOUNCE_PORT: u16 = 6881;

pub fn get_peers(metainfo: &Metainfo) -> Result<()> {
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

    let mut bytes_read: usize = 0;
    let mut buf = [0u8; 20];

    while let Ok(received) = socket.recv(&mut buf) {
        let mut c = Cursor::new(buf);

        let msg = match c.read_u32::<BigEndian>()? {
            0 => {
                let connect_response = messages::ConnectResponse::from_udp_msg(&buf)?;
                send_announce(&socket, &metainfo, &connect_response)?;
                Ok(connect_response)
            }
            1 => {
                info!("got announce response: {:?}", buf);
                bail!("Cannot parse announce response yet");
            }
            // 1 => messages::AnnounceResponse::from_udp_msg(&buf),
            unhandled => Err(anyhow!(
                "Unhandled udp message with action code: {}",
                unhandled
            )),
        };

        info!("got message: {:?}", msg);
        bytes_read += received;
    }

    // 1. send connection request
    // 2. receive / parse connect response
    // 3. send announce request
    // 4. pass peers back to client

    Ok(())
}

fn send_announce(
    socket: &UdpSocket,
    metainfo: &Metainfo,
    connect_response: &messages::ConnectResponse,
) -> Result<()> {
    info!("sending announce request");
    connect_response.transaction_id();

    // let mut hasher = Sha1::new();
    // hasher.input(metainfo.info().bytes());
    // let info_hash: [u8; 20] = hasher.result().into();
    // TODO: replace with SHA-1 hash of metainfo.info dictionary bencode
    let info_hash = [0u8; 20];

    let request = messages::AnnounceRequest::new(
        connect_response.connection_id(),
        connect_response.transaction_id(),
        info_hash,
        gen_peer_id(),
        0,                      // downloaded
        metainfo.info().size(), // left
        0,                      // uploaded
        messages::AnnounceRequestEvent::None,
        DEFAULT_ANNOUNCE_PORT,
    );

    socket
        .send(request.to_udp_msg()?.as_slice())
        .context("failed to send connect request")?;

    connect_response;
    Ok(())
}

fn info_hash() {
    panic!()
}
