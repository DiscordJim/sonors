use core::str;
use std::{collections::HashMap, fs::{File, Metadata}, io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write}, path::{Path, PathBuf}};

use thunderdome::{Arena, Index};
use walkdir::WalkDir;
use anyhow::{anyhow, Result};


#[derive(Debug)]
pub struct ArchivalNode {
    pub path: PathBuf,
    pub metadata: Metadata,
    pub is_leaf: bool,
}

impl ArchivalNode {
    pub fn get_header_bytes(&self) -> Result<Vec<u8>> {

        let path_bytes = self.path.to_str()
            .ok_or_else(|| anyhow!("Failed to represent path {:?} as UTF-8 bytes.", self.path))?;
        
        // Encode the length of the header
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&((5 + path_bytes.len()) as u32).to_le_bytes());
        bytes.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
        bytes.push(if self.is_leaf { 0x01 } else { 0x00 });
        bytes.extend_from_slice(path_bytes.as_bytes());
    
        Ok(bytes)
    }
}


pub fn encode_archival_tree<T: Write>(writer: BufWriter<T>, items: Vec<ArchivalNode>) {
    
}


pub struct SrsSection {
    length: usize,
    data: Vec<u8>
}

/*
pub fn create_section(data: &[u8]) -> SrsSection {
    SrsSection {
        length: data.len(),
        data
    }
}

*/

// 128 MiB
pub const CHUNK_SIZE: usize = 131_072;

pub fn write_archival_node<T: Write + Seek>(writer: &mut BufWriter<T>, node: &ArchivalNode) -> Result<u64> {
    let pos = writer.stream_position()?;

   
    //println!("Node: {:#?}", node);
    // Write headers
    writer.write_all(&node.get_header_bytes()?)?;
    
    if node.is_leaf {
        let mut reader = BufReader::new(File::open(&node.path)?);

        loop {
            let buf = &mut [0u8; CHUNK_SIZE];
            let bytes_read = reader.read(buf)?;
            if bytes_read == 0 {
                break;
            }
           // println!("Writing chunks!");
            writer.write_all(&(bytes_read as u32).to_le_bytes())?;
            writer.write_all(&buf[..bytes_read])?;
        }
        

        //let file = File::open(node.path)?;
        //let file_size = file.metadata()?.len();
        //writer.write_all(&file_size.to_le_bytes())?;

        //let reader = BufReader::new(File::open(node.path)?);
        
    }
    
    

    Ok(pos)
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

    Ok(Path::new(str::from_utf8(&buf)?).to_path_buf())
}

pub fn write_file_table<T: Write + Seek>(writer: &mut T, nodes: &Vec<ArchivalNode>, map: &HashMap<u32, u64>) -> Result<()> {
    let current_position = writer.stream_position()?;

    for (key, value) in map.iter() {
        writer.write_all(&key.to_le_bytes())?;
        writer.write_all(&value.to_le_bytes())?;

        write_pathbuf(writer, &nodes.get(*key as usize).unwrap().path)?;
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
            metadata: entry.metadata()?
        });
    }

    let mut file_table: HashMap<u32, u64> = HashMap::new();

    let mut file_writer = BufWriter::new(File::create_new(output.as_ref())?);
    for (index, node) in node_list.iter().enumerate() {
        file_table.insert(index.try_into()?, write_archival_node(&mut file_writer, node)?);
    }

    write_file_table(&mut file_writer, &node_list, &file_table)?;


    Ok(())
}

pub fn read_u64<T: Read + Seek>(reader: &mut T) -> Result<u64> {
    let buf = &mut [0u8; 8];
    reader.read_exact(buf)?;
    Ok(u64::from_le_bytes(*buf))
}

pub fn read_u32<T: Read + Seek>(reader: &mut T) -> Result<u32> {
    let buf = &mut [0u8; 4];
    reader.read_exact(buf)?;
    Ok(u32::from_le_bytes(*buf))
}

#[derive(Default)]
pub struct SonorousFileTable(Vec<(u32, u64, PathBuf)>);


impl SonorousFileTable {
    pub fn from_reader<T: Read + Seek>(reader: &mut T) -> Result<Self> {
        read_sonorous_file_table(reader)
    }
    pub fn files(&self) -> Vec<&PathBuf> {
        self.0.iter().map(|(_, _, path)| path).collect::<Vec<&PathBuf>>()
    }
    pub fn expand_into_file<T: Read + Seek>(reader: &mut T) -> Result<()> {


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
        let path = read_pathbuf(reader)?;

        println!("({key}, {value}) -> {path:?}");
    
        file_table.0.push((key, value, path));

        if reader.stream_position()? == stop_position {
            break
        }
    }

    Ok(file_table)
}




fn main() -> Result<()> {

//    std::fs::remove_file("archive.srs")?;

    let path = "wowz";


    let mut reader = BufReader::new(File::open("archive.srs")?);
    let file_table = SonorousFileTable::from_reader(&mut reader)?;




    //create_sonorous_file(path, "archive.srs")?;


    Ok(())
}
