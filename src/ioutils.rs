use std::{fs::create_dir_all, io::{Read, Seek, Write}, path::{Path, PathBuf}};
use anyhow::{Result, anyhow};

use crate::security::secure::read_encrypted;

pub fn transfer_archival_node<R: Read + Seek, W: Write + Seek>(reader: &mut R, writer: &mut W, key: &[u8]) -> Result<()>{


    loop {
        let status = read_byte(reader)?;
        if status == 0x01 {
            break
        }
       
       // let mut buf = vec![0u8; read_u32(reader)? as usize];
       // reader.read_exact(&mut buf)?;        
        writer.write_all(&read_encrypted(reader, key)?)?;
    }   
    Ok(())
}

pub fn create_directory_tree(path: impl AsRef<Path>, is_file: bool) -> Result<()> {

    let path = path.as_ref();
    if path.exists() {
        return Ok(())
    }
    println!("Path: {:?}", path);
    if is_file {
        if let Some(parent_directory) = path.parent() {
            println!("-> {:?}", parent_directory);
            create_dir_all(&parent_directory)?;
        }
    } else {
        create_dir_all(&path)?;
        
    }
    
    Ok(())
}

pub fn write_pathbuf<T: Write + Seek>(writer: &mut T, buf: &PathBuf) -> Result<()> {
    
    let path_bytes = buf.to_str()
        .ok_or_else(|| anyhow!("Failed to represent path {:?} as UTF-8 bytes.", buf))?.as_bytes();
    writer.write_all(&(path_bytes.len() as u32).to_le_bytes())?;
    writer.write_all(&path_bytes)?;

    Ok(())
}

pub fn read_pathbuf<T: Read + Seek>(reader: &mut T) -> Result<PathBuf> {
    let path_length = read_u32(reader)?;

    let mut buf = vec![0u8; path_length as usize];
    reader.read_exact(&mut buf)?;

    Ok(Path::new(std::str::from_utf8(&buf)?).to_path_buf())
}


pub fn read_u64<R: Read>(reader: &mut R) -> Result<u64> {
    let buf = &mut [0u8; 8];
    reader.read_exact(buf)?;
    Ok(u64::from_le_bytes(*buf))
}

pub fn write_u32<W: Write>(writer: &mut W, number: u32) -> Result<()> {
    writer.write_all(&number.to_le_bytes())?;
    Ok(())
}

pub fn read_u32<R: Read>(reader: &mut R) -> Result<u32> {
    let buf = &mut [0u8; 4];
    reader.read_exact(buf)?;
    Ok(u32::from_le_bytes(*buf))
}

pub fn read_byte<R: Read + Seek>(reader: &mut R) -> Result<u8> {
    let buf = &mut [0u8; 1];
    reader.read_exact(buf)?;
    Ok(buf[0])
}

pub fn read_bool<R: Read + Seek>(reader: &mut R) -> Result<bool> {
    //let buf = &mut [0u8; 1];
    //reader.read_exact(buf)?;
    Ok(match read_byte(reader)? {
        0x01 => true,
        0x00 => false,
        _ => Err(anyhow!("Expected a boolean but found a different code that was not 0x01 or 0x00."))?
    })
}

pub fn write_bool<W: Write + Seek>(writer: &mut W, value: bool) -> Result<()> {
    if value {
        writer.write_all(&[0x01])?;
    } else {
        writer.write_all(&[0x00])?;
    }
    Ok(())
}

