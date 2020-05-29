use std::fs;

#[macro_use]
extern crate anyhow;
use anyhow::Result;
use anyhow::anyhow;

extern crate bendy;
use bendy::decoding::FromBencode;
use bendy::encoding::ToBencode;

#[macro_use]
extern crate log;

// Extensions to external crates
mod bendy_ext;

// Internal mods
mod messages;
mod metainfo;
mod tracker;
mod type_ext;

use crate::metainfo::Metainfo;
use crate::tracker::get_peers;

const GIMP_TORRENT : &str = "gimp-2.10.18-setup-2.exe.torrent";

fn main() -> Result<()> {
    env_logger::init();

    let contents = fs::read(GIMP_TORRENT).unwrap();

    // 1. get metainfo
    let metainfo = Metainfo::from_bencode(contents.as_slice())
        .or_else(|e| bail!("failed de-serializing metainfo bencode.  Error: {}", e))?;

    // Socket
    get_peers(&metainfo)?;

    Ok(())
}
