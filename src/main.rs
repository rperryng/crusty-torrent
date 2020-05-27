use std::fs;
use std::net::UdpSocket;

extern crate anyhow;
use anyhow::Result;
use anyhow::anyhow;

extern crate bendy;
use bendy::decoding::FromBencode;

extern crate url;
use url::{Url, ParseError};

#[macro_use]
extern crate log;

mod bendy_ext;

mod metainfo;
use crate::metainfo::Metainfo;

mod tracker;

const GIMP_TORRENT : &str = "gimp-2.10.18-setup-2.exe.torrent";

fn main() -> Result<()> {
    env_logger::init();

    let contents = fs::read(GIMP_TORRENT).unwrap();

    // 1. get metainfo
    let metainfo = Metainfo::from_bencode(contents.as_slice())
        .map_err(|e| anyhow!("{:?}", e))?;
    info!("success! read torrent file: piece length is {} bytes", metainfo.info().piece_length());
    info!("Announce URL: {}", metainfo.announce());
    let url = Url::parse(metainfo.announce())?;

    // Socket
    let socket = UdpSocket::bind("127.0.0.1:34254").expect("couldn't bind to address");
    socket.connect(metainfo.announce()).expect("connect function failed");
    socket.send(&[0, 1, 2]).expect("couldn't send message");

    Ok(())
}
