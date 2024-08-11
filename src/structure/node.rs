use std::{fs::File, io::{BufReader, Read, Seek, Write}, path::PathBuf};

use anyhow::Result;

use crate::{constants::CHUNK_SIZE, security::secure::write_encrypted};

#[derive(Clone, Debug)]
pub struct ArchivalNode {
    pub path: PathBuf,
    pub is_leaf: bool,
}

impl ArchivalNode {
    pub fn write<W: Write + Seek>(&self, writer: &mut W, key: &[u8]) -> Result<u64> {
        let starting_position = writer.stream_position()?;

  
        if self.is_leaf {
            let mut reader = BufReader::new(File::open(&self.path)?);
    
            loop {
                let buf = &mut [0u8; CHUNK_SIZE];
                let bytes_read = reader.read(buf)?;
                if bytes_read == 0 {
                    break;
                }

                writer.write_all(&[0x00])?;
                write_encrypted(writer, key, &buf[..bytes_read])?;
            }
            writer.write_all(&[0x01])?;
        }
        Ok(starting_position)
    }
}




