use std::fs;

use anyhow::{Result, bail};
use bendy::decoding::FromBencode;

mod bendy_ext;
mod download;
mod messages;
mod metainfo;
mod tracker;
mod type_ext;
mod util;

use crate::download::download;
use crate::metainfo::Metainfo;
use crate::tracker::get_peers;
use crate::util::gen_peer_id;

const TORRENT_PATH: &str = "libreoffice-6.4.3.macos64bit.torrent";

fn main() -> Result<()> {
    env_logger::init();

    // Parse .torrent file
    let contents = fs::read(TORRENT_PATH).unwrap();
    let metainfo = Metainfo::from_bencode(contents.as_slice())
        .or_else(|e| bail!("failed de-serializing metainfo bencode.  Error: {}", e))?;

    // Get peers from announce url
    let peer_id = gen_peer_id();
    let announce_response = get_peers(&metainfo, &peer_id)?;
    let peer = announce_response
        .peers()
        .first()
        .expect(&format!("No peers found: {:?}", announce_response));

    // Initiate download from peer
    download(&metainfo, peer, &peer_id)?;

    Ok(())
}
