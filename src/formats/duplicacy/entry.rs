use std::{collections::HashMap, io::Read, path::Path};

use rmp::decode;
use serde::Deserialize;

use super::{config::Config, decoder::Decoder};
use crate::error::Result;

#[derive(Deserialize, Debug)]
pub struct Entry {
    pub path: String,
    pub size: i64,
    pub time: i64,
    pub mode: i64,
    pub link: String,
    pub hash: Vec<u8>,

    pub uid: i32,
    pub gid: i32,

    pub start_chunk: i32,
    pub start_offset: i32,
    pub end_chunk: i32,
    pub end_offset: i32,

    //number_of_attributes: i32,
    pub attributes: HashMap<String, Vec<u8>>,
}

impl Entry {
    pub fn from_file(config: &Config, path: impl AsRef<Path>, hash: &[u8]) -> Result<Vec<Self>> {
        // load file
        let file = std::fs::read(path.as_ref())?;

        let decoded = if config.encrypted {
            let key = config.derive_key(&config.chunk_key, hash)?;
            let decoder = Decoder::new(key);
            decoder.decode(&file)?
        } else {
            let decoder = Decoder::new(None);
            decoder.decode(&file)?
        };

        // parse the file manually
        // this is encoded with msgpack, but it doesn't seem to be done in a standard way
        // there are no array or object markers, it's just done sequentially
        let mut cursor = std::io::Cursor::new(decoded);
        let mut entries = Vec::new();

        while !cursor.is_empty() {
            let entry = Entry {
                path: decode::read_str(&mut cursor, &mut vec![0; 5000])
                    .expect("invalid utf-8")
                    .to_string(),
                size: decode::read_int(&mut cursor).unwrap(),
                time: decode::read_int(&mut cursor).unwrap(),
                mode: decode::read_int(&mut cursor).unwrap(),
                link: decode::read_str(&mut cursor, &mut vec![0; 5000])
                    .expect("invalid utf-8")
                    .to_string(),
                hash: hex::decode(
                    decode::read_str(&mut cursor, &mut vec![0; 5000]).expect("invalid utf-8"),
                )?,
                start_chunk: decode::read_int(&mut cursor).unwrap(),
                start_offset: decode::read_int(&mut cursor).unwrap(),
                end_chunk: decode::read_int(&mut cursor).unwrap(),
                end_offset: decode::read_int(&mut cursor).unwrap(),
                uid: decode::read_int(&mut cursor).unwrap(),
                gid: decode::read_int(&mut cursor).unwrap(),
                attributes: {
                    let number_of_attributes: i32 = decode::read_int(&mut cursor).unwrap();
                    let mut map = HashMap::new();
                    for _ in 0..number_of_attributes {
                        let key = decode::read_str(&mut cursor, &mut vec![0; 5000])
                            .expect("invalid utf-8")
                            .to_string();
                        // read the string length, then read that many bytes directly
                        // according to rmp, the spec dictates that this should be valid utf-8
                        // but duplicacy uses a function to read a string yet this still breaks
                        // so it's likely duplicacy's lib isn't properly validating or something
                        // anyhow - it just converts the whole thing to bytes anyway
                        let value_len = decode::read_str_len(&mut cursor).unwrap();
                        let mut buf = vec![0; value_len as usize];
                        cursor.read_exact(&mut buf)?;
                        // let value = decode::read_str(&mut cursor, &mut vec![0; 5000])
                        //     .unwrap()
                        //     .as_bytes()
                        //     .to_vec();
                        map.insert(key, buf);
                    }
                    map
                },
            };

            entries.push(entry);
        }

        Ok(entries)
    }
}
