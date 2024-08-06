use std::io::{Read, Write};

use anyhow::{anyhow, Result};
use chacha20poly1305::{aead::{Aead, AeadMut, OsRng}, AeadCore, ChaCha20Poly1305, KeyInit};

use crate::ioutils::{read_u32, write_u32};





pub fn write_encrypted<W: Write>(writer: &mut W, key: &[u8], data: &[u8]) -> Result<()> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| anyhow!("Failed to create a ChaCha20Poly1305 instance from a block. Error: {e}"))?;
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let encrypted = cipher.encrypt(&nonce, data)
        .map_err(|e| anyhow!("Failed to encrypt with error: {e}"))?;

    //panic!("Gen nonce of {}", nonce.len());
    

    writer.write_all(&nonce)?;
    println!("Encrypted Length: {}", encrypted.len());
    write_u32(writer, encrypted.len() as u32)?;
    writer.write_all(&encrypted)?;
    Ok(())
}

pub fn read_encrypted<R: Read>(reader: &mut R, key: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| anyhow!("Failed to create a ChaCha20Poly1305 instance from a block. Error: {e}"))?;
    
    let nonce = &mut [0u8; 12];
    reader.read_exact(nonce)?;
    

    let encrypted_len = read_u32(reader)?;
    println!("pulled out encrypted length: {}", encrypted_len);
    let mut data = vec![0u8; encrypted_len as usize];
    reader.read_exact(&mut data)?;


    let decrypted = cipher.decrypt(nonce.as_ref().into(), data.as_ref()).map_err(|e| anyhow!("Decryption failed: {e}"))?;

    //let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    Ok(decrypted)
}


#[cfg(test)]
mod tests {

    use std::io::{Cursor, Seek, SeekFrom, Write};

    use anyhow::Result;
    use chacha20poly1305::{aead::OsRng, ChaCha20Poly1305, KeyInit};

    use crate::secure::read_encrypted;

    use super::write_encrypted;

    #[test]
    fn encrypt_decrypt() -> Result<()> {

        let contents = &[0x00, 0x01, 0x02, 0x03, 0x04];

        let key = ChaCha20Poly1305::generate_key(&mut OsRng);

        let mut simulated_file = Cursor::new(Vec::new());
        write_encrypted(&mut simulated_file, &key, contents)?;


        simulated_file.seek(SeekFrom::Start(0))?;


        assert_eq!(&read_encrypted(&mut simulated_file, &key)?, &contents);

        //simulated_file.write_all(contents)?;


        Ok(())
    }

}
