use std::io::{Cursor, Read, Seek, SeekFrom};
use crate::{ioutils::{read_bool, read_pathbuf, read_u32, read_u64}, security::secure::{create_key, read_encrypted}};
use anyhow::Result;
use super::node::ArchivalNode;


/// Allows the indexing of the contents of the files and serves as the access
/// mechanism for all archived volumes.
#[derive(Default, Debug)]
pub struct FileTable {
    /// The actual table. 
    ///
    /// (POS, POS, NODE)
    pub map: Vec<(u32, u64, ArchivalNode)>,
    /// The encryption key being used for the table.
    pub key: Vec<u8>
}

impl FileTable {
    /// Creates a blank new file table.
    pub fn new(key: &[u8]) -> Self {
        Self {
            map: Vec::default(),
            key: key.to_vec()
        }
    }
    /// Creates a `FileTable` from a mutable reader object.
    pub fn from_reader<T: Read + Seek>(reader: &mut T, password: &str) -> Result<Self> {
        read_file_table(reader, password)
    }
//    pub fn expand_into_files<T: Read + Seek>(&self, reader: &mut T, dest: impl AsRef<Path>) -> Result<()> {
 //       
   // }
}

fn read_file_table<T: Read + Seek>(reader: &mut T, password: &str) -> Result<FileTable> {

    reader.seek(SeekFrom::Start(0))?;
    
    let salt = &mut [0u8; 32];
    reader.read_exact(salt)?;

    let key = &create_key(salt, password.as_bytes())?;

    reader.seek(SeekFrom::End(-8))?;
//    let stop_position = reader.stream_position()?;

    let table_position = read_u64(reader)?;
    //println!("Table position: {table_position}");
    reader.seek(SeekFrom::Start(table_position))?;

    
    // Decrypt the file table.
    let decrypted = read_encrypted(reader, key)?;


    let decrypted_len = decrypted.len();
    let mut reader = Cursor::new(Vec::from(decrypted));
    let mut file_table = FileTable::new(key);

    loop {
        let key = read_u32(&mut reader)?;
        let value = read_u64(&mut reader)?;
        let is_leaf = read_bool(&mut reader)?;
        let path = read_pathbuf(&mut reader)?;
       
 
        file_table.map.push((key, value, ArchivalNode {
            path,
            is_leaf
        }));


        if reader.stream_position()? == decrypted_len as u64 {
            break
        }
    }
    Ok(file_table)
}


#[cfg(test)]
mod tests {
    use anyhow::Result;



    #[test]
    pub fn test_file_table() -> Result<()> {
        

        Ok(())
    }
}

