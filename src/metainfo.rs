use std::convert::TryInto;

use url::Url;

use sha1::Digest;
use sha1::Sha1 as Sha1Hasher;

use anyhow::Result;
use log::{info};

use crate::bendy_ext::decode_option;
use crate::type_ext::Sha1;
use bendy::decoding;
use bendy::decoding::{FromBencode, Object, ResultExt};
use bendy::encoding;
use bendy::encoding::{SingleItemEncoder, ToBencode};

#[derive(Debug)]
pub struct SingleFile {
    name: String,
    length: u32,
    md5sum: Option<String>,
}

impl ToBencode for SingleFile {
    const MAX_DEPTH: usize = 1;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), encoding::Error> {
        encoder.emit_dict(|mut dict| {
            dict.emit_pair(b"length", self.length)?;
            if let Some(md5) = &self.md5sum { dict.emit_pair(b"md5sum", md5)? }
            dict.emit_pair(b"name", &self.name)?;

            Ok(())
        })?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct MultiFile {
    length: u64,
    md5sum: Option<String>,
    path: Vec<String>,
}

impl ToBencode for MultiFile {
    const MAX_DEPTH: usize = 3;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), encoding::Error> {
        encoder.emit_dict(|mut dict| {
            dict.emit_pair(b"length", self.length)?;
            if let Some(md5) = &self.md5sum { dict.emit_pair(b"md5sum", md5)? }
            dict.emit_pair(b"path", &self.path)?;

            Ok(())
        })
    }
}

#[derive(Debug)]
pub struct MultipleFiles {
    directory_name: String,
    files: Vec<MultiFile>,
}

impl ToBencode for MultipleFiles {
    const MAX_DEPTH: usize = 3;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), encoding::Error> {
        encoder.emit_dict(|mut dict| {
            dict.emit_pair(b"files", &self.files)?;
            dict.emit_pair(b"name", &self.directory_name)?;
            Ok(())
        })
    }
}

#[derive(Debug)]
pub enum FileInfo {
    Single(SingleFile),
    Multi(MultipleFiles),
}

impl ToBencode for FileInfo {
    const MAX_DEPTH: usize = 3;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), encoding::Error> {
        match self {
            FileInfo::Single(single) => encoder.emit(single),
            FileInfo::Multi(multi) => encoder.emit(multi),
        }
    }
}

#[derive(Debug)]
pub struct Info {
    piece_length: u32,
    pieces: Vec<Sha1>, // string consisting of the concatenation of all 20-byte SHA1 hash values, one per piece (byte string, i.e. not urlencoded)
    private: Option<bool>,
    file_info: FileInfo,
}

impl Info {
    pub fn file_info(&self) -> &FileInfo {
        &self.file_info
    }

    pub fn piece_length(&self) -> u32 {
        self.piece_length
    }

    pub fn size(&self) -> u64 {
        match &(self.file_info) {
            FileInfo::Single(file) => file.length as u64,
            FileInfo::Multi(files) => files.files.iter().fold(0, |acc, f| acc + f.length) as u64,
        }
    }
}

impl FromBencode for Info {
    const EXPECTED_RECURSION_DEPTH: usize = 0;

    fn decode_bencode_object(object: Object) -> Result<Self, decoding::Error> {
        let mut piece_length = None;
        let mut pieces: Option<Vec<Sha1>> = None;
        let mut private = None;

        // File variables
        let mut name = None;
        let mut length = None;
        let mut md5sum = None;

        // Multi file mode
        let mut files: Vec<MultiFile> = Vec::new();

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                // Always present
                (b"piece length", value) => piece_length = decode_option("piece length", value)?,
                (b"pieces", value) => pieces = decode_pieces(value).map(Some)?,
                (b"private", value) => private = decode_private(value).map(Some)?,

                // Single-file mode specific
                (b"name", value) => name = decode_option("name", value)?,
                (b"length", value) => length = decode_option("length", value)?,
                (b"md5sum", value) => md5sum = decode_option("md5sum", value)?,

                // Multi-file mode
                (b"files", value) => files = decode_files(value)?,

                (unknown_field, _) => info!(
                    "unhandled field '{}' while decoding 'info' node",
                    String::from_utf8_lossy(unknown_field)
                ),
            }
        }

        let name = name.ok_or_else(|| decoding::Error::missing_field("name"))?;
        let piece_length =
            piece_length.ok_or_else(|| decoding::Error::missing_field("piece length"))?;
        let pieces = pieces.ok_or_else(|| decoding::Error::missing_field("pieces"))?;

        let file_info = match (length, files) {
            (Some(length), files) if files.is_empty() => Ok(FileInfo::Single(SingleFile {
                name,
                length,
                md5sum,
            })),
            (_, files) if files.len() > 0 => Ok(FileInfo::Multi(MultipleFiles {
                directory_name: name,
                files,
            })),
            (_, files) if files.len() == 0 => Err(decoding::Error::missing_field("files")),
            (_, _) => Err(decoding::Error::missing_field("length")),
        }?;

        Ok(Info {
            piece_length,
            pieces,
            private,
            file_info,
        })
    }
}

fn decode_pieces(value: Object) -> Result<Vec<Sha1>, decoding::Error> {
    value.try_into_bytes().context("pieces").map(|full_hash| {
        full_hash
            .chunks(20)
            .map(|hash| {
                assert_eq!(hash.len(), 20);
                let hash: Sha1 = hash.try_into().expect(&format!(
                    "'pieces' node contained malformed SHA1 hash: {:?}",
                    hash
                ));
                hash
            })
            .collect::<Vec<Sha1>>()
    })
}

fn decode_private(value: Object) -> Result<bool, decoding::Error> {
    u8::decode_bencode_object(value)
        .context("private")
        .and_then(|p| match p {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(decoding::Error::unexpected_token(
                format!("private = {}", p),
                format!("private = '0' or '1'"),
            )),
        })
}

fn decode_files(value: Object) -> Result<Vec<MultiFile>, decoding::Error> {
    let mut list = value.try_into_list()?;
    let mut files = Vec::new();

    while let Some(file_dict) = list.next_object()? {
        let mut file_dict = file_dict.try_into_dictionary()?;

        let mut path = None;
        let mut length = None;
        let mut md5sum = None;

        while let Some(file_dict_pair) = file_dict.next_pair()? {
            match file_dict_pair {
                (b"path", value) => path = decode_option("path", value)?,
                (b"length", value) => length = decode_option("length", value)?,
                (b"md5sum", value) => md5sum = decode_option("md5sum", value)?,
                (unknown_field, _) => {
                    info!(
                        "unhandled field '{}' while decoding entry in 'files' node",
                        String::from_utf8_lossy(unknown_field)
                    );
                }
            }
        }

        let path = path.ok_or_else(|| decoding::Error::missing_field("path"))?;
        let length = length.ok_or_else(|| decoding::Error::missing_field("length"))?;

        files.push(MultiFile {
            path,
            length,
            md5sum,
        });
    }

    Ok(files)
}

impl ToBencode for Info {
    const MAX_DEPTH: usize = 5;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), encoding::Error> {
        encoder.emit_unsorted_dict(|dict| {
            dict.emit_pair(b"piece_length", self.piece_length)?;

            let pieces = self.pieces.iter().fold(Vec::new(), |mut acc: Vec<u8>, sha1| {
                acc.extend_from_slice(sha1);
                acc
            });
            dict.emit_pair(b"pieces", pieces)?;

            if let Some(p) = &self.private {
                dict.emit_pair(b"private", *p as u32)?;
            }

            dict.emit_pair(b"file_info", &self.file_info)
        })
    }
}

pub struct Metainfo {
    info: Info,
    announce_url: Url,
    announce_list: Option<Vec<String>>,
    encoding: Option<String>,
    info_hash: Sha1,
}

impl Metainfo {
    pub fn info(&self) -> &Info {
        &self.info
    }

    pub fn announce_url(&self) -> &Url {
        &self.announce_url
    }

    pub fn info_hash(&self) -> &Sha1 {
        &self.info_hash
    }
}

impl FromBencode for Metainfo {
    fn decode_bencode_object(object: Object) -> Result<Self, decoding::Error> {
        let mut info = None;
        let mut announce: Option<String> = None;
        let mut announce_list = None;
        let mut encoding = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"info", value) => info = decode_option("info", value)?,
                (b"announce", value) => announce = decode_option("announce", value)?,

                (unknown_field, _) => {
                    info!(
                        "unhandled field '{}' while decoding 'metainfo' file",
                        String::from_utf8_lossy(unknown_field)
                    );
                }
            }
        }

        let info: Info = info.ok_or_else(|| decoding::Error::missing_field("info"))?;
        let announce = announce.ok_or_else(|| decoding::Error::missing_field("announce"))?;
        let announce_url = Url::parse(&announce)?;

        info!("producing info_hash from info");
        info!("ok: {:?}", info.to_bencode());
        let info_hash: Sha1 = info
            .to_bencode()
            .map_err(|e| decoding::Error::malformed_content(e))
            .map(|bencoded_info| {
                let mut hasher = Sha1Hasher::new();
                hasher.input(bencoded_info);
                let hash: Sha1 = hasher.result().into();
                hash
            })?;
        info!("producing info_hash");

        Ok(Metainfo {
            info,
            announce_url,
            announce_list,
            encoding,
            info_hash,
        })
    }
}
