use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use crate::{constants::SALT_LENGTH_IN_BYTES, ioutils::{read_bool, read_pathbuf, read_u32, read_u64, write_bool, write_pathbuf}, security::secure::{create_key, read_encrypted, write_encrypted}};
use anyhow::Result;
use super::node::ArchivalNode;


/// Allows the indexing of the contents of the files and serves as the access
/// mechanism for all archived volumes.
#[derive(Default, Debug)]
pub struct FileTable {
    /// The actual table. 
    ///
    /// (Index, File Positon, ArchivalNode)
    pub map: Vec<(u32, u64, ArchivalNode)>,
    /// The encryption key being used for the table.
    key: Vec<u8>,
    /// The salt used for the table.
    salt: Vec<u8>
}

impl FileTable {
    /// Creates a blank new file table.
    pub fn new(key: Vec<u8>, salt: &[u8]) -> Self {
        Self {
            map: Vec::default(),
            key,
            salt: salt.to_vec()
        }
    }
    /// Adds a node to the file table structure.
    ///
    /// Takes:
    /// - Index (position within the file table)
    /// - File Index (position within the file)
    /// - Node, an [ArchivalNode] representing the object to represent
    /// within the table.
    pub fn add(&mut self, index: u32, file_index: u64, node: ArchivalNode) {
        self.map.push((index, file_index, node))
    }
    /// Returns the key.
    ///
    /// # Safety
    /// This function is potentially unsafe as cloning the key in 
    /// any form could prevent it from being zeroed and therefore
    /// leading to it being seen in memory.
    pub unsafe fn key(&self) -> &[u8] {
        &self.key
    }
    /// Creates a `FileTable` from a mutable reader object.
    pub fn from_reader<T: Read + Seek>(reader: &mut T, password: &str) -> Result<Self> {
        read_file_table(reader, password)
    }
    /// Writes the file table to a [Writer](std::io) object.
    pub fn write<T: Write + Seek>(&self, writer: &mut T) -> Result<()> {
        write_file_table(writer, self)
    }
//    pub fn expand_into_files<T: Read + Seek>(&self, reader: &mut T, dest: impl AsRef<Path>) -> Result<()> {
 //       
   // }
}

fn write_file_table<T: Write + Seek>(writer: &mut T, table: &FileTable) -> Result<()> {
    let current_position = writer.stream_position()?;



    writer.write_all(&table.salt)?;


    // Create a write to to write pre-encryption.
    let mut table_writer = Cursor::new(Vec::new());

    for (key, value, node) in table.map.iter() {
        table_writer.write_all(&key.to_le_bytes())?;
        table_writer.write_all(&value.to_le_bytes())?;

        write_bool(&mut table_writer, node.is_leaf)?;
        write_pathbuf(&mut table_writer, &node.path)?;
    }

    write_encrypted(writer, &table.key, table_writer.into_inner().as_ref())?;

    writer.write_all(&current_position.to_le_bytes())?;
   

//    println!("Done writing {:?}", writer.stream_position()?);
    Ok(())
}



fn read_file_table<T: Read + Seek>(reader: &mut T, password: &str) -> Result<FileTable> {

   // reader.seek(SeekFrom::Start(0))?;
    
//    let mut salt = vec![0u8; SALT_LENGTH_IN_BYTES];
   // let salt = &mut [0u8; SALT_LENGTH_IN_BYTES];
   // reader.read_exact(salt)?;


    reader.seek(SeekFrom::End(-8))?;

    //    let stop_position = reader.stream_position()?;



    let table_position = read_u64(reader)?;
    //println!("Table position: {table_position}");
    reader.seek(SeekFrom::Start(table_position))?;

    let mut salt = vec![0u8; SALT_LENGTH_IN_BYTES];
    reader.read_exact(&mut salt)?;

    let key = create_key(&salt, password.as_bytes())?;

    // Decrypt the file table.
    let decrypted = read_encrypted(reader, &key)?;


    let decrypted_len = decrypted.len();
    let mut reader = Cursor::new(Vec::from(decrypted));
    let mut file_table = FileTable::new(key, &salt);

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
    use std::{io::Cursor, path::Path};

    use anyhow::Result;

    use crate::security::secure::{create_key, generate_salt};

    use super::FileTable;



    #[test]
    pub fn test_file_table() -> Result<()> {
        let mut export = Cursor::new(Vec::new());

        let password = "default_password";


        let salt = generate_salt();
        let key = create_key(&salt, password.as_bytes())?;

        let mut file_table = FileTable::new(key, &salt);
        file_table.add(0, 32, crate::structure::node::ArchivalNode { path: Path::new("hello").to_path_buf(), is_leaf: true });

        file_table.write(&mut export)?;



        let mut export = Cursor::new(export.into_inner());
        let file_table = FileTable::from_reader(&mut export, password)?;

        let first_entry = file_table.map.first().unwrap();
        assert_eq!(first_entry.0, 0);
        assert_eq!(first_entry.1, 32);
        assert_eq!(first_entry.2.path.to_str().unwrap(), "hello");
        assert_eq!(first_entry.2.is_leaf, true);


        Ok(())
    }
}

