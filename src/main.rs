use std::fs;

use anyhow::{Result, bail};
use bendy::decoding::FromBencode;

// Extensions to external crates
mod bendy_ext;

// Internal mods
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

const GIMP_TORRENT: &str = "gimp-2.10.18-setup-2.exe.torrent";

fn main() -> Result<()> {
    env_logger::init();

    // Parse .torrent file
    let contents = fs::read(GIMP_TORRENT).unwrap();
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
