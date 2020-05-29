use std::io::Cursor;
use std::net::UdpSocket;

use crate::metainfo::Metainfo;
use crate::messages;
use crate::messages::{FromUdpMsg, ToUdpMsg};
use byteorder::{BigEndian, ReadBytesExt};

extern crate url;
use url::Url;

extern crate anyhow;
use anyhow::{Context, Result};

const UDP_ADDRESS: &str = "0.0.0.0:8000";

pub fn get_peers(metainfo: &Metainfo) -> Result<()> {
    let socket = UdpSocket::bind(UDP_ADDRESS).expect("couldn't bind to address");
    info!("UDP successfully binded to: {}", UDP_ADDRESS);

    let announce_url = metainfo.announce_url();
    let host = announce_url.host_str().context("announce_url host")?;
    let port = announce_url.port().context("announce_url port")?;
    let announce_url = format!("{}:{}", host, port);

    info!("attempting to connect to announce_url: {}", announce_url);
    socket.connect(announce_url).context("couldn't connect to announce_url")?;
    info!("successfully connected to url");

    let connect_request = messages::ConnectRequest::new();

    info!("sending connect request: {:?}", connect_request);
    socket.send(connect_request.to_udp_msg()?.as_slice()).context("failed to  send connect request")?;
    info!("Successfully sent connect request");

    let mut bytes_read: usize = 0;
    let mut buf = [0u8; 20];

    while let Ok(received) = socket.recv(&mut buf) {
        let mut c = Cursor::new(buf);

        let msg = match c.read_u32::<BigEndian>()? {
            0 => {
                let connect_response = messages::ConnectResponse::from_udp_msg(&buf)?;
                send_announce(&socket, &metainfo, &connect_response);
                Ok(connect_response)
            },
            // 1 => messages::AnnounceResponse::from_udp_msg(&buf),
            unhandled => Err(anyhow!("Unhandled udp message with action code: {}", unhandled)),
        };

        info!("got message: {:?}", msg);
    }

    // 1. send connection request
    // 2. receive / parse connect response
    // 3. send announce request
    // 4. pass peers back to client

    Ok(())
}

fn send_announce(socket: &UdpSocket, metainfo: &metainfo, connect_response: &messages::ConnectResponse) -> Result(()) {
    connect_response.transaction_id();

    let request = messages::AnnounceRequest::new(
        connect_response.connection_id(),
        connect_response.transaction_id(),
        ,
    )

    socket
        .send(connect_response.to_udp_msg()?.as_slice())
        .context("failed to send connect request")?;

    connect_response
}

fn info_hash() {
    panic!()
}
