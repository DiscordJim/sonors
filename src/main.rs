use core::{str};
use std::{collections::HashMap, fs::{create_dir_all, File, Metadata}, io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}};

use argon2::Argon2;
use chacha20poly1305::{aead::{Buffer, OsRng}, AeadCore, ChaCha20Poly1305, KeyInit};
use thunderdome::{Arena, Index};
use walkdir::WalkDir;
use anyhow::{anyhow, Result};

use sonors::{constants::CHUNK_SIZE, ioutils::*, security::secure::{create_key, generate_salt, read_encrypted, write_encrypted}, structure::node::ArchivalNode};

use chacha20poly1305::aead::Aead;




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


pub fn write_archival_node<T: Write + Seek>(writer: &mut T, node: &ArchivalNode, key: &[u8]) -> Result<u64> {
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

            writer.write_all(&[0x00])?;
            write_encrypted(writer, key, &buf[..bytes_read])?;
        }
        writer.write_all(&[0x01])?;
    }

    
    

    Ok(pos)
}



pub fn write_file_table<T: Write + Seek>(writer: &mut T, table: &SonorousFileTable, key: &[u8]) -> Result<()> {
    let current_position = writer.stream_position()?;

    let mut table_writer = Cursor::new(Vec::new());

    for (key, value, node) in table.map.iter() {
        table_writer.write_all(&key.to_le_bytes())?;
        table_writer.write_all(&value.to_le_bytes())?;

    



        //let node = nodes.get(*key as usize).unwrap();

        write_bool(&mut table_writer, node.is_leaf)?;
        write_pathbuf(&mut table_writer, &node.path)?;
    }

    write_encrypted(writer, key, table_writer.into_inner().as_ref())?;


    writer.write_all(&current_position.to_le_bytes())?;
   
    Ok(())
}

pub fn create_sonorous_file(path: impl AsRef<Path>, output: impl AsRef<Path>, password: &str) -> Result<()> {

    let salt = generate_salt();
    let key = create_key(&salt, password.as_bytes())?;
    println!("Crearef key -> {key:?}");

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
    file_writer.write_all(&salt)?;

    for (index, node) in node_list.into_iter().enumerate() {
        file_table.map.push((index.try_into()?, write_archival_node(&mut file_writer, &node, &key)?, node));
    }

    write_file_table(&mut file_writer, &file_table, &key)?;


    Ok(())
}

#[derive(Default, Debug)]
pub struct SonorousFileTable {
    map: Vec<(u32, u64, ArchivalNode)>,
    key: Vec<u8>
}


impl SonorousFileTable {
    pub fn new(key: &[u8]) -> Self {
        Self {
            map: Vec::default(),
            key: key.to_vec()
        }
    }
    pub fn from_reader<T: Read + Seek>(reader: &mut T, password: &str) -> Result<Self> {
        read_sonorous_file_table(reader, password)
    }
    pub fn files(&self) -> Vec<&PathBuf> {
        self.map.iter().map(|(_, _, node)| &node.path).collect::<Vec<&PathBuf>>()
    }
    pub fn expand_into_files<T: Read + Seek>(&self, reader: &mut T, dest: impl AsRef<Path>) -> Result<()> {
        for (key, position, node) in &self.map {
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
                transfer_archival_node(reader, writer, &self.key)?;
            }
        }

        Ok(())
    }
}



pub fn read_sonorous_file_table<T: Read + Seek>(reader: &mut T, password: &str) -> Result<SonorousFileTable> {

    reader.seek(SeekFrom::Start(0))?;
    
    let salt = &mut [0u8; 32];
    reader.read_exact(salt)?;

    let key = &create_key(salt, password.as_bytes())?;

    println!("Created key -> {key:?}");


    reader.seek(SeekFrom::End(-8))?;
    let stop_position = reader.stream_position()?;

    let table_position = read_u64(reader)?;
    //println!("Table position: {table_position}");
    reader.seek(SeekFrom::Start(table_position))?;


    let decrypted = read_encrypted(reader, key)?;
    
    let decrypted_len = decrypted.len();
    let mut reader = Cursor::new(Vec::from(decrypted));



    let mut file_table = SonorousFileTable::new(key);


    loop {

        let key = read_u32(&mut reader)?;
        let value = read_u64(&mut reader)?;
        let is_leaf = read_bool(&mut reader)?;
        let path = read_pathbuf(&mut reader)?;
       

        println!("Found some data...");

        //println!("({key}, {value}) -> {path:?}");
    
        file_table.map.push((key, value, ArchivalNode {
            is_leaf,
            path
        }));


        if reader.stream_position()? == decrypted_len as u64 {
            break
        }
        //if reader.stream_position()? == stop_position {
        //    break
        //}
    }
    println!("Done reading... {file_table:?}");

    Ok(file_table)
}

//pub enum SonorousHeader {
//    PasswordSalt = 0x00,
//    UtilityVersion = 0x01
//}




fn main() -> Result<()> {
   
    
  
    //let (salt, key) = create_key("hello".as_bytes().as_ref())?;
    //println!("key: {:?}", key);




 //   println!("My borhter {:?}", key);


 //    println!("Plan: {:?}", std::str::from_utf8(&plaintext)); 
   
    println!("Creating file.");
    create_sonorous_file("test", "archive.srs", "hello")?;
    println!("File created.");


    let mut reader = BufReader::new(File::open("archive.srs")?);
    let file_table = SonorousFileTable::from_reader(&mut reader, "hello")?;
    file_table.expand_into_files(&mut reader, "wowz")?;
    

    //create_sonorous_file(path, "archive.srs")?;


    Ok(())
}
