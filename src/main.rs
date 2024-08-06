use core::str;
use std::{collections::HashMap, fs::{create_dir_all, File, Metadata}, io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}};

use argon2::Argon2;
use chacha20poly1305::{aead::OsRng, AeadCore, ChaCha20Poly1305, KeyInit};
use thunderdome::{Arena, Index};
use walkdir::WalkDir;
use anyhow::{anyhow, Result};

use sonors::ioutils::*;

use chacha20poly1305::aead::Aead;

// 128 MiB
pub const CHUNK_SIZE: usize = 131_072;


#[derive(Debug)]
pub struct ArchivalNode {
    pub path: PathBuf,
    pub is_leaf: bool,
}

/*
impl ArchivalNode {
    pub fn write_header_bytes<W: Write + Seek>(&self, writer: &mut W) -> Result<Vec<u8>> {

        //let path_bytes = self.path.to_str()
         //   .ok_or_else(|| anyhow!("Failed to represent path {:?} as UTF-8 bytes.", self.path))?;
        
        // Encode the length of the header
        let mut bytes = Vec::new();
        //bytes.extend_from_slice(&((5 + path_bytes.len()) as u32).to_le_bytes());
        //bytes.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
        bytes.push(if self.is_leaf { 0x01 } else { 0x00 });
        //write_pathbuf(writer, &self.path)?;
        //bytes.extend_from_slice(path_bytes.as_bytes());
    
        Ok(bytes)
    }
}
*/

pub fn transfer_archival_node<R: Read + Seek, W: Write + Seek>(reader: &mut R, writer: &mut W) -> Result<()>{


    loop {
        let status = read_byte(reader)?;
        if status == 0x01 {
            break
        }
       
        let mut buf = vec![0u8; read_u32(reader)? as usize];
        reader.read_exact(&mut buf)?;        
        writer.write_all(&buf)?;
    }   
    Ok(())
}


pub fn write_archival_node<T: Write + Seek>(writer: &mut T, node: &ArchivalNode) -> Result<u64> {
    let pos = writer.stream_position()?;

   
    //println!("Node: {:#?}", node);
    // Write headers
    
//    node.write_header_bytes(writer)?;
    //writer.write_all(&node.get_header_bytes()?)?;
    
    if node.is_leaf {
        println!("putting leaf in file...");
        let mut reader = BufReader::new(File::open(&node.path)?);

        loop {
            let buf = &mut [0u8; CHUNK_SIZE];
            let bytes_read = reader.read(buf)?;
            if bytes_read == 0 {
                break;
            }
           // println!("Writing chunks!");
            writer.write_all(&[0x00])?;
            writer.write_all(&(bytes_read as u32).to_le_bytes())?;
            writer.write_all(&buf[..bytes_read])?;
            println!("Wrote some stuff");
        }
        writer.write_all(&[0x01])?;
    }

    
    

    Ok(pos)
}



pub fn write_file_table<T: Write + Seek>(writer: &mut T, table: &SonorousFileTable) -> Result<()> {
    let current_position = writer.stream_position()?;

    for (key, value, node) in table.0.iter() {
        writer.write_all(&key.to_le_bytes())?;
        writer.write_all(&value.to_le_bytes())?;

    



        //let node = nodes.get(*key as usize).unwrap();

        write_bool(writer, node.is_leaf)?;
        write_pathbuf(writer, &node.path)?;
    }

    writer.write_all(&current_position.to_le_bytes())?;
   
    Ok(())
}

pub fn create_sonorous_file(path: impl AsRef<Path>, output: impl AsRef<Path>) -> Result<()> {

    let mut node_list = Vec::new();

    for entry in WalkDir::new(path.as_ref()) {
        let entry = entry?;
        node_list.push(ArchivalNode {
            path: entry.path().to_path_buf(),
            is_leaf: !entry.path().is_dir(),
            //metadata: entry.metadata()?
        });
    }

    let mut file_table = SonorousFileTable::default();

//    let mut file_table: HashMap<u32, u64> = HashMap::new();

    let mut file_writer = BufWriter::new(File::create(output.as_ref())?);
    for (index, node) in node_list.into_iter().enumerate() {
        file_table.0.push((index.try_into()?, write_archival_node(&mut file_writer, &node)?, node));
    }

    write_file_table(&mut file_writer, &file_table)?;


    Ok(())
}

#[derive(Default)]
pub struct SonorousFileTable(Vec<(u32, u64, ArchivalNode)>);


impl SonorousFileTable {
    pub fn from_reader<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        read_sonorous_file_table(reader)
    }
    pub fn files(&self) -> Vec<&PathBuf> {
        self.0.iter().map(|(_, _, node)| &node.path).collect::<Vec<&PathBuf>>()
    }
    pub fn expand_into_files<T: Read + Seek>(&self, reader: &mut T, dest: impl AsRef<Path>) -> Result<()> {
        for (key, position, node) in &self.0 {
            reader.seek(SeekFrom::Start(*position))?;


            // Create the directory tree if it does not exist.
            let path = dest.as_ref().join(&node.path);
            println!("Creating directory.");
            create_directory_tree(&path, node.is_leaf)?;
        
            println!("done");
            if node.is_leaf {
                println!("Writing a leaf node...");
                let writer = &mut BufWriter::new(File::create(&path)?);
                println!("Created writer...");
                transfer_archival_node(reader, writer)?;
            }
        }

        Ok(())
    }
}



pub fn read_sonorous_file_table<T: Read + Seek>(reader: &mut T) -> Result<SonorousFileTable> {

    reader.seek(SeekFrom::End(-8))?;
    let stop_position = reader.stream_position()?;

    let table_position = read_u64(reader)?;
    //println!("Table position: {table_position}");
    reader.seek(SeekFrom::Start(table_position))?;


    let mut file_table = SonorousFileTable::default();


    loop {

        let key = read_u32(reader)?;
        let value = read_u64(reader)?;
        let is_leaf = read_bool(reader)?;
        let path = read_pathbuf(reader)?;
        

        //println!("({key}, {value}) -> {path:?}");
    
        file_table.0.push((key, value, ArchivalNode {
            is_leaf,
            path
        }));

        if reader.stream_position()? == stop_position {
            break
        }
    }

    Ok(file_table)
}

pub enum SonorousHeader {
    PasswordSalt = 0x00,
    UtilityVersion = 0x01
}




fn main() -> Result<()> {

   
    
    let password = b"hunter42"; // Bad password; don't actually use!
    let salt = b"example salt"; // Salt should be unique per password

    let mut key = [0u8; 32]; // Can be any desired size
    Argon2::default().hash_password_into(password, salt, &mut key).unwrap();
  


    println!("My borhter {:?}", key);


 //    println!("Plan: {:?}", std::str::from_utf8(&plaintext)); 
   /* 
    println!("Creating file.");
    create_sonorous_file("test", "archive.srs")?;
    println!("File created.");


    let mut reader = BufReader::new(File::open("archive.srs")?);
    let file_table = SonorousFileTable::from_reader(&mut reader)?;
    file_table.expand_into_files(&mut reader, "wowz")?;
    */

    //create_sonorous_file(path, "archive.srs")?;


    Ok(())
}
