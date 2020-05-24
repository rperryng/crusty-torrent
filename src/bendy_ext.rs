use core::fmt::Display;

use bendy::decoding::{FromBencode, Object, Error, ResultExt};

pub fn decode_option<T : FromBencode>(
    context: impl Display,
    object: Object
) -> Result<Option<T>, Error> {
    T::decode_bencode_object(object)
        .context(context)
        .map(Some)
}
