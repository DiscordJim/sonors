use std::io::{Read, Write};
use anyhow::{anyhow, Result};
use argon2::{password_hash::rand_core::RngCore, Argon2};
use chacha20poly1305::{aead::{Aead, OsRng}, AeadCore, ChaCha20Poly1305, KeyInit};
use crate::{constants::{KEY_LENGTH_IN_BYTES, SALT_LENGTH_IN_BYTES}, ioutils::{read_u32, write_u32}};




/// Writes some bytes to a [Writer](std::io) using a key and some data.
///
/// Encrypts it using [ChaCha20Poly1305](chacha20poly1305::aead) and writes it to the writer
/// in the format:
///
/// [ 12 bytes of nonce ] [ (4 bytes) u32 representing encrypted length ] [ encrypted bytes ]
pub fn write_encrypted<W: Write>(writer: &mut W, key: &[u8], data: &[u8]) -> Result<()> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| anyhow!("Failed to create a ChaCha20Poly1305 instance from a block. Error: {e}"))?;
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let encrypted = cipher.encrypt(&nonce, data)
        .map_err(|e| anyhow!("Failed to encrypt with error: {e}"))?;
 

    writer.write_all(&nonce)?;

    write_u32(writer, encrypted.len() as u32)?;
    writer.write_all(&encrypted)?;
    Ok(())
}


/// Reads encrypted data out to a decrypted vector as per the format specified
/// in [write_encrypted].
pub fn read_encrypted<R: Read>(reader: &mut R, key: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| anyhow!("Failed to create a ChaCha20Poly1305 instance from a block. Error: {e}"))?;
    
    let nonce = &mut [0u8; 12];
    reader.read_exact(nonce)?;
    

    let encrypted_len = read_u32(reader)?;
    let mut data = vec![0u8; encrypted_len as usize];
    reader.read_exact(&mut data)?;


    let decrypted = cipher.decrypt(nonce.as_ref().into(), data.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {e}"))?;

    Ok(decrypted)
}


/// Generates a salt of `SALT_LENGTH_IN_BYTES`
pub fn generate_salt() -> [u8; SALT_LENGTH_IN_BYTES] {
    let mut rng = OsRng;
    let mut salt = [0u8; SALT_LENGTH_IN_BYTES];
    let mut salt_pos = 0;
    for _ in 0..4 {
        for b in rng.next_u64().to_ne_bytes() {
            salt[salt_pos] = b;
            salt_pos += 1;
        }
    }
    salt
}

/// Creates a key from a salt and the UTF-8 bytes of a passowrd.
pub fn create_key(salt: &[u8], password: &[u8]) -> Result<Vec<u8>> {
    let mut key = [0u8; KEY_LENGTH_IN_BYTES];
    Argon2::default().hash_password_into(password, &salt, &mut key)
        .map_err(|e| anyhow!("Failed to produce password with error: {e}"))?;
    Ok(key.to_vec())
}


#[cfg(test)]
mod tests {

    use std::{collections::HashSet, io::{Cursor, Seek, SeekFrom}};

    use anyhow::Result;
    use chacha20poly1305::{aead::OsRng, ChaCha20Poly1305, KeyInit};


    use crate::security::secure::read_encrypted;

    use super::{create_key, generate_salt, write_encrypted};

    #[test]
    fn test_password_gen() -> Result<()> {
        let tests = 3;


        let mut set = HashSet::new();

        for _ in 0..tests {
            let salt = generate_salt();
            let password = generate_salt();

            let key = create_key(&salt, &password)?;
            if set.contains(&key) {
                panic!("Encountered a duplicate entry. Is the password generator truly random?");
            }
            set.insert(key);
        }

        Ok(())
    }

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
