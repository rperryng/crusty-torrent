use rand::Rng;

use crate::type_ext::PeerId;

pub fn gen_peer_id() -> PeerId {
    let mut bytes: [u8; 20] = rand::thread_rng().gen();
    bytes[..7].copy_from_slice(b"-CR001-");
    bytes
}
