use flate2::read::ZlibDecoder;
use std::io::Read;

use crate::services::lua_parser::{parse_lua_file, LuaParseResult};

// .st wire format:
//   [0..4]   xorkey_raw (u32 LE) → xor_key = (raw ^ 0xFFFEA4C8) & 0xFF
//   [4..8]   size       (u32 LE)
//   [8..12]  xor_key_verify (ignored)
//   [12..12+size]  payload, each byte xor'd with xor_key
//   zlib-inflate payload, skip first 512 bytes, remainder is lua-like text.
pub fn parse_st_file(buffer: &[u8]) -> Result<LuaParseResult, String> {
    if buffer.len() < 12 {
        return Err(format!(
            ".st file too small: {} bytes (need at least 12 for header)",
            buffer.len()
        ));
    }

    let xor_key_raw = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
    let size = u32::from_le_bytes(buffer[4..8].try_into().unwrap()) as usize;
    let xor_key = ((xor_key_raw ^ 0xFFFEA4C8) & 0xFF) as u8;

    if 12 + size > buffer.len() {
        return Err(format!(
            ".st file data size ({}) exceeds buffer length ({})",
            size,
            buffer.len() - 12
        ));
    }

    let encrypted_data = &buffer[12..12 + size];
    let decrypted_data: Vec<u8> = encrypted_data.iter().map(|b| b ^ xor_key).collect();

    let mut decoder = ZlibDecoder::new(&decrypted_data[..]);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Failed to decompress .st data: {}", e))?;

    if decompressed.len() <= 512 {
        return Err(format!(
            ".st decompressed data too small: {} bytes (need >512)",
            decompressed.len()
        ));
    }

    let lua_content = String::from_utf8_lossy(&decompressed[512..]).to_string();
    parse_lua_file(&lua_content).map_err(|e| format!(".st file parsed but its payload is invalid: {}", e))
}
