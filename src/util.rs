extern crate rand;
use rand::Rng;

pub fn gen_peer_id() -> [u8; 20] {
    let mut bytes: [u8; 20] = rand::thread_rng().gen();
    bytes[..7].copy_from_slice(b"-CR001-");
    bytes
}
