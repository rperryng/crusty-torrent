use std::convert::TryInto;

use bendy::decoding::{FromBencode, Object, Error, ResultExt};

type Sha1 = [u8; 20];

#[derive(Debug)]
pub struct SingleFile {
    path: String,
    length: u32,
    md5sum: Option<String>,
}

#[derive(Debug)]
pub struct MultipleFiles {
    directory_name: String,
    files: Vec<SingleFile>
}

#[derive(Debug)]
pub enum FileInfo {
    Single(SingleFile),
    Multi(MultipleFiles),
}

#[derive(Debug)]
pub struct Info {
    piece_length: u32,
    pieces: Vec::<Sha1>, // string consisting of the concatenation of all 20-byte SHA1 hash values, one per piece (byte string, i.e. not urlencoded)
    private: Option<u8>,
    file_info: FileInfo,
}

impl Info {
    pub fn file_info(&self) -> &FileInfo {
        &self.file_info
    }

    pub fn piece_length(&self) -> u32 {
        self.piece_length
    }
}


impl FromBencode for Info {
    const EXPECTED_RECURSION_DEPTH: usize = 0;

    fn decode_bencode_object(object: Object) -> Result<Self, Error> {
        let mut piece_length = None;
        let mut pieces : Option<Vec<Sha1>> = None;
        let mut private = None;

        // File variables
        let mut name = None;
        let mut length = None;
        let mut md5sum = None;

        // Multi file mode
        let mut files : Vec<SingleFile> = Vec::new();

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                // Always present
                (b"piece length", value) => {
                    piece_length = u32::decode_bencode_object(value)
                        .context("piece length")
                        .map(Some)?;
                },
                (b"pieces", value) => {
                    pieces = value.try_into_bytes()
                        .context("pieces")
                        .map(|full_hash| {
                            full_hash
                                .chunks(20)
                                .map(|hash| {
                                    assert_eq!(hash.len(), 20);
                                    let hash : Sha1 = hash.try_into().expect("'pieces' node contained malformed Sha1Hash");
                                    hash
                                })
                                .collect::<Vec<Sha1>>()
                        })
                        .map(Some)?
                },
                (b"private", value) => {
                    private = u8::decode_bencode_object(value)
                        .context("private")
                        .and_then(|p| match p {
                            0 | 1 => Ok(p),
                            _ => Err(Error::unexpected_token(format!("private = {}", p), format!("private = '0' or '1'")))
                        })
                        .map(Some)?
                },

                // Single-file mode specific
                (b"name", value) => {
                    name = String::decode_bencode_object(value)
                        .context("name")
                        .map(Some)?
                },
                (b"length", value) => {
                    length = u32::decode_bencode_object(value)
                        .context("length")
                        .map(Some)?
                },
                (b"md5sum", value) => {
                    md5sum = String::decode_bencode_object(value)
                        .context("md5sum")
                        .map(Some)?
                },

                // Multi-file mode
                (b"files", value) => {
                    let mut list = value.try_into_list()?;

                    while let Some(file_dict) = list.next_object()? {
                        let mut file_dict = file_dict.try_into_dictionary()?;

                        let mut path = None;
                        let mut length = None;
                        let mut md5sum = None;

                        while let Some(file_dict_pair) = file_dict.next_pair()? {
                            match file_dict_pair {
                                (b"path", value) => {
                                    path = String::decode_bencode_object(value)
                                        .context("path")
                                        .map(Some)?
                                },
                                (b"length", value) => {
                                    path = String::decode_bencode_object(value)
                                        .context("length")
                                        .map(Some)?
                                },
                                (b"md5sum", value) => {
                                    path = String::decode_bencode_object(value)
                                        .context("md5sum")
                                        .map(Some)?
                                },
                                (unknown_field, _) => {
                                    println!("ignoring unhandled field '{}' while decoding entry in 'files' node", String::from_utf8_lossy(unknown_field));
                                }
                            }
                        }

                        let path = path.ok_or_else(|| Error::missing_field("path"))?;
                        let length = length.ok_or_else(|| Error::missing_field("length"))?;
                        let md5sum = md5sum.ok_or_else(|| Error::missing_field("md5sum"))?;

                        files.push(SingleFile { path, length, md5sum });
                    };
                },

                (unknown_field, _) => {
                    println!("ignoring unhandled field '{}' while decoding FileInfo", String::from_utf8_lossy(unknown_field));
                },
            }
        };

        let name = name.ok_or_else(|| Error::missing_field("name"));
        let piece_length = piece_length.ok_or_else(|| Error::missing_field("piece length"))?;
        let pieces = pieces.ok_or_else(|| Error::missing_field("pieces"))?;

        let file_info = match (length, files) {
            (Some(length), files) => {
                assert!(files.is_empty());
                FileInfo::Single(SingleFile { path: name.unwrap(), length, md5sum })
            },
            (length, files) => {
                assert!(length.is_none()); // Impossible?
                assert!(files.len() > 0);

                FileInfo::Multi(MultipleFiles { directory_name: name.unwrap(), files })
            }
        };

        Ok(Info { piece_length, pieces, private, file_info })
    }
}

pub struct TorrentMetainfo {
    info: Info,
    announce: Option<String>,
    announce_list: Option<Vec<String>>,
    encoding: Option<String>,
}

impl TorrentMetainfo {
    pub fn info(&self) -> &Info {
        &self.info
    }
}

impl FromBencode for TorrentMetainfo {
    fn decode_bencode_object(object: Object) -> Result<Self, Error> {
        let mut info = None;
        let mut announce = None;
        let mut announce_list = None;
        let mut encoding = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"info", value) => {
                    info = Info::decode_bencode_object(value)
                        .context("info")
                        .map(Some)?;
                },
                (unknown_field, value) => {
                    println!("ignoring unhandled field '{}' while decoding FileInfo", String::from_utf8_lossy(unknown_field));
                }
            }
        }

        let info = info.ok_or_else(|| Error::missing_field("info"))?;
        Ok(TorrentMetainfo { info, announce, announce_list, encoding })
    }
}
