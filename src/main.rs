use std::fs;

extern crate bendy;
use bendy::decoding::FromBencode;

extern crate anyhow;
use anyhow::Result;
use anyhow::anyhow;

mod models;
use crate::models::TorrentMetainfo;

const GIMP_TORRENT : &str = "gimp-2.10.18-setup-2.exe.torrent";


fn main() -> Result<()> {
    println!("Hello, world!");

    let contents = fs::read(GIMP_TORRENT).unwrap();
    let readable = String::from_utf8_lossy(&contents);
    println!("raw: {}", readable);

    let metainfo = TorrentMetainfo::from_bencode(contents.as_slice())
        .map_err(|e| anyhow!("{:?}", e))?;
    println!("success! read torrent file: {}", metainfo.info().piece_length());

    Ok(())
}
