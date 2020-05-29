use std::convert::TryInto;

extern crate url;
use url::Url;

use bendy::decoding::{FromBencode, Object, Error, ResultExt};
use crate::bendy_ext::decode_option;
use crate::type_ext::Sha1;

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
                (b"piece length", value) => piece_length = decode_option("piece length", value)?,
                (b"pieces", value) => pieces = decode_pieces(value).map(Some)?,
                (b"private", value) => private = decode_private(value).map(Some)?,

                // Single-file mode specific
                (b"name", value) => name = decode_option("name", value)?,
                (b"length", value) => length = decode_option("length", value)?,
                (b"md5sum", value) => md5sum = decode_option("md5sum", value)?,

                // Multi-file mode
                (b"files", value) => files = decode_files(value)?,

                (unknown_field, _) => info!("unhandled field '{}' while decoding 'info' node", String::from_utf8_lossy(unknown_field)),
            }
        };

        let name = name.ok_or_else(|| Error::missing_field("name"))?;
        let piece_length = piece_length.ok_or_else(|| Error::missing_field("piece length"))?;
        let pieces = pieces.ok_or_else(|| Error::missing_field("pieces"))?;

        let file_info = match (length, files) {
            (Some(length), files) if files.is_empty() => Ok(FileInfo::Single(SingleFile { path: name, length, md5sum })),
            (_, files) if files.len() > 0 => Ok(FileInfo::Multi(MultipleFiles { directory_name: name, files })),
            (_, files) if files.len() == 0 => Err(Error::missing_field("files")),
            (_, _) => Err(Error::missing_field("length"))
        }?;

        Ok(Info { piece_length, pieces, private, file_info })
    }
}

fn decode_pieces(value: Object) -> Result<Vec::<Sha1>, Error> {
    value.try_into_bytes()
        .context("pieces")
        .map(|full_hash| {
            full_hash
                .chunks(20)
                .map(|hash| {
                    assert_eq!(hash.len(), 20);
                    let hash: Sha1 = hash.try_into().expect(&format!("'pieces' node contained malformed SHA1 hash: {:?}", hash));
                    hash
                })
                .collect::<Vec<Sha1>>()
        })
}

fn decode_private(value: Object) -> Result<bool, Error> {
    u8::decode_bencode_object(value)
        .context("private")
        .and_then(|p| match p {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::unexpected_token(format!("private = {}", p), format!("private = '0' or '1'")))
        })
}

fn decode_files(value : Object) -> Result<Vec<SingleFile>, Error> {
    let mut list = value.try_into_list()?;
    let mut files  = Vec::new();

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
                    info!("unhandled field '{}' while decoding entry in 'files' node", String::from_utf8_lossy(unknown_field));
                }
            }
        }

        let path = path.ok_or_else(|| Error::missing_field("path"))?;
        let length = length.ok_or_else(|| Error::missing_field("length"))?;

        files.push(SingleFile { path, length, md5sum });
    };

    Ok(files)
}

pub struct Metainfo {
    info: Info,
    announce_url: Url,
    announce_list: Option<Vec<String>>,
    encoding: Option<String>,
}

impl Metainfo {
    pub fn info(&self) -> &Info {
        &self.info
    }

    pub fn announce_url(&self) -> &Url {
        &self.announce_url
    }
}

impl FromBencode for Metainfo {
    fn decode_bencode_object(object: Object) -> Result<Self, Error> {
        let mut info = None;
        let mut announce: Option<String> = None;
        let mut announce_list = None;
        let mut encoding = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"info", value) => info = decode_option("info", value)?,
                (b"announce", value) =>  announce = decode_option("announce", value)?,

                (unknown_field, _) => {
                    info!("unhandled field '{}' while decoding 'metainfo' file", String::from_utf8_lossy(unknown_field));
                }
            }
        }

        let info = info.ok_or_else(|| Error::missing_field("info"))?;
        let announce_url = announce
            .map(|url_raw| Url::parse(&x))
            .map(|url| url.map_err(op) )
            .ok_or_else(|| Error::missing_field("announce"))?;


        Ok(Metainfo { info, announce_url, announce_list, encoding })
    }
}
