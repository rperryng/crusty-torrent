use std::fs;

mod bendy_ext;
mod download;
mod messages;
mod metainfo;
mod tracker;
mod type_ext;
mod util;

use crate::download::download;
use crate::messages::FromProtocolMsg;
use crate::metainfo::Metainfo;
use crate::tracker::get_peers;
use crate::util::gen_peer_id;

use anyhow::{bail, Result};
use bendy::decoding::FromBencode;
use log::info;

const TORRENT_PATH: &str = "gimp-2.10.18-setup-2.exe.torrent";

const QBITTORRENT_HEX_STRING: &'static str = "16236bd9f08a2f7100000001690170412aa1fff0d7ca65b149194ec42957d49cb27836db2d7142343235302d6132675566655a676a5868760000000000000000000000000ab51888000000000000000000000002000000000fee24d7000000c868e8";

fn main() -> Result<()> {
    env_logger::init();

    qbittorrent_test()?;

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

fn qbittorrent_test() -> Result<()> {
    let bytes: Vec<u8> = hex::decode(QBITTORRENT_HEX_STRING)?;
    info!("Decoded hex-byte string into {} len byte vec", bytes.len());

    let qbittorrent_announce_event = messages::AnnounceRequest::from_protocol_msg(&bytes)?;
    info!(
        "QBitTorrent announce request:\n{:?}",
        qbittorrent_announce_event
    );
    Ok(())
}
