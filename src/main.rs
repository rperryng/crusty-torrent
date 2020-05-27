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

// Extensions to external crates
mod bendy_ext;
mod url_ext;

// crates
mod metainfo;
mod tracker;

use crate::metainfo::Metainfo;
use crate::tracker::get_peers;

const GIMP_TORRENT : &str = "gimp-2.10.18-setup-2.exe.torrent";

fn main() -> Result<()> {
    env_logger::init();

    let contents = fs::read(GIMP_TORRENT).unwrap();

    // 1. get metainfo
    let metainfo = Metainfo::from_bencode(contents.as_slice())
        .map_err(|e| anyhow!("{:?}", e))?;
    info!("success! read torrent file: piece length is {} bytes", metainfo.info().piece_length());
    info!("Announce URL: {}", metainfo.announce());

    // Socket
    get_peers(metainfo.announce());

    Ok(())
}
