use std::io::{self, Read};

// BYTE events (0-63): 1 byte value
pub const FLP_CHAN_TYPE: u8 = 21;

// WORD events (64-127): 2 byte LE value
pub const FLP_NEW_CHAN: u8 = 64;
pub const FLP_NEW_PAT: u8 = 65;
pub const FLP_TEMPO_LEGACY: u8 = 66;

// DWORD events (128-191): 4 byte LE value
pub const FLP_TEMPO: u8 = 156;

// TEXT/VARIABLE events (192-255): varint length + bytes
pub const FLP_TEXT_CHAN_NAME: u8 = 192;
pub const FLP_VERSION: u8 = 199;
pub const FLP_TEXT_PLUGIN_NAME: u8 = 201;

/// Read a variable-length integer (7 bits per byte, MSB = "more bytes follow").
/// Used for the length prefix of TEXT/VARIABLE events (event IDs 192-255).
pub fn read_varint<R: Read>(reader: &mut R) -> io::Result<u64> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        let byte = buf[0];
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "varint overflow"));
        }
    }
    Ok(result)
}
