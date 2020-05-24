use std::fs;

extern crate bendy;
use bendy::decoding::FromBencode;

extern crate anyhow;
use anyhow::Result;
use anyhow::anyhow;

#[macro_use]
extern crate log;

mod bendy_ext;

mod metainfo;
use crate::metainfo::Metainfo;

const GIMP_TORRENT : &str = "gimp-2.10.18-setup-2.exe.torrent";

fn main() -> Result<()> {
    env_logger::init();

    let contents = fs::read(GIMP_TORRENT).unwrap();
    let metainfo = Metainfo::from_bencode(contents.as_slice())
        .map_err(|e| anyhow!("{:?}", e))?;
    info!("success! read torrent file: piece length is {} bytes", metainfo.info().piece_length());

    Ok(())
}
