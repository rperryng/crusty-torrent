use std::net::UdpSocket;

use crate::messages;
use crate::messages::{FromUdpMsg, ToUdpMsg};

extern crate url;
use url::Url;

extern crate anyhow;
use anyhow::{Context, Result};

const UDP_ADDRESS: &str = "0.0.0.0:8000";

pub fn get_peers(announce_url: &Url) -> Result<()> {
    let mut socket = UdpSocket::bind(UDP_ADDRESS).expect("couldn't bind to address");
    info!("UDP successfully binded to: {}", UDP_ADDRESS);

    let announce_url = format!(
        "{}:{}",
        announce_url.host_str().context(format!("announce_url is missing host, url was: {}", announce_url))?,
        announce_url.port().context(format!("announce_url is missing port, url was: {}", announce_url))?
    );
    info!("attempting to connect to announce_url: {}", announce_url);
    socket.connect(announce_url).context("Couldn't connect to announce_url")?;
    info!("successfully connected to url");

    info!("as udp message: {:?}", messages::ConnectRequest::new());

    socket
        .send(messages::ConnectRequest::new().to_udp_msg()?.as_slice())
        .context("failed to  send connect request")?;
    info!("Successfully sent connect request");

    let mut bytes_read: usize = 0;
    let mut buf = [0u8; 20];

    match socket.recv(&mut buf) {
        Ok(received) => {
            if received < 4 {
                return Err(anyhow!("connect response had less than 4 bytes.  received: {:?}", String::from_utf8_lossy(&buf[..received])))
            } else {
                let response = messages::ConnectResponse::from_udp_msg(&buf)?;
                info!("Successfully received connection response: {:?}", response)
            }
            bytes_read += received;
        },
        Err(e) => return Err(anyhow::Error::new(e))
    }

    // 1. send connection request
    // 2. receive / parse connect response
    // 3. send announce request
    // 4. pass peers back to client

    // example
    // let mut buf = [0; 10];
    // let (amt, src) = socket.recv_from(&mut buf);

    // let buf = &mut buf[..amt];
    // buf.reverse();
    // socket.send_to(buf, &src);

    Ok(())
}
