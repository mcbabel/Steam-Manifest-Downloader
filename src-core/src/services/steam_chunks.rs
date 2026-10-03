use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::services::steam_manifest::symmetric_decrypt;

pub fn decode_chunk(
    encrypted: &[u8],
    depot_key: &[u8; 32],
    expected_size: u32,
) -> Result<Vec<u8>, String> {
    let decrypted = symmetric_decrypt(encrypted, depot_key)?;
    let decompressed = decompress_vz(&decrypted)?;
    if decompressed.len() as u32 != expected_size {
        return Err(format!(
            "chunk size mismatch after decompress: got {}, expected {}",
            decompressed.len(),
            expected_size
        ));
    }
    Ok(decompressed)
}

fn decompress_vz(input: &[u8]) -> Result<Vec<u8>, String> {
    if input.len() < 16 {
        return Err("VZ blob too short".to_string());
    }
    if &input[0..2] == b"VZ" {
        decompress_vz1(input)
    } else if &input[0..2] == b"VS" && input.get(2) == Some(&b'Z') {
        decompress_vzstd(input)
    } else {
        Err(format!(
            "unrecognised chunk container magic: {:02X} {:02X}",
            input[0], input[1]
        ))
    }
}

fn decompress_vz1(input: &[u8]) -> Result<Vec<u8>, String> {
    if input.len() < 22 {
        return Err("VZ1 blob too short".to_string());
    }
    if input[2] != b'a' {
        return Err(format!(
            "Unsupported VZ version '{}' (only 'a' known)",
            input[2] as char
        ));
    }
    let header = &input[7..12];

    let body_end = input.len() - 10;
    let lzma_body = &input[12..body_end];

    let mut footer = Cursor::new(&input[body_end..body_end + 8]);
    let expected_crc = footer
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("VZ footer crc read failed: {}", e))?;
    let expected_size = footer
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("VZ footer size read failed: {}", e))?;

    let mut lzma_stream = Vec::with_capacity(13 + lzma_body.len());
    lzma_stream.extend_from_slice(header);
    lzma_stream.extend_from_slice(&(expected_size as u64).to_le_bytes());
    lzma_stream.extend_from_slice(lzma_body);

    let mut out = Vec::with_capacity(expected_size as usize);
    let mut reader = Cursor::new(lzma_stream);
    lzma_rs::lzma_decompress(&mut reader, &mut out)
        .map_err(|e| format!("LZMA decompress failed: {}", e))?;

    if out.len() as u32 != expected_size {
        return Err(format!(
            "VZ1 size mismatch: decompressed {} vs expected {}",
            out.len(),
            expected_size
        ));
    }

    let actual_crc = crc32fast::hash(&out);
    if actual_crc != expected_crc {
        return Err(format!(
            "VZ1 CRC mismatch: got 0x{:08X}, expected 0x{:08X}",
            actual_crc, expected_crc
        ));
    }

    Ok(out)
}

fn decompress_vzstd(input: &[u8]) -> Result<Vec<u8>, String> {
    const HEADER_LEN: usize = 8;
    const TRAILER_LEN: usize = 15;
    if input.len() < HEADER_LEN + TRAILER_LEN {
        return Err("VSZTD blob too short".to_string());
    }
    if &input[0..3] != b"VSZ" {
        return Err("VSZTD magic mismatch".to_string());
    }
    if input[3] != b'a' {
        return Err(format!(
            "Unsupported VSZTD version 0x{:02X} (only 'a' known)",
            input[3]
        ));
    }
    let trailer_start = input.len() - TRAILER_LEN;
    let mut footer = Cursor::new(&input[trailer_start..]);
    let footer_crc = footer
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("VSZTD footer crc read failed: {}", e))?;
    let expected_size = footer
        .read_u64::<LittleEndian>()
        .map_err(|e| format!("VSZTD footer size read failed: {}", e))?;
    if &input[trailer_start + 12..] != b"zsv" {
        return Err("VSZTD terminator 'zsv' not found".to_string());
    }

    let header_crc = u32::from_le_bytes(input[4..8].try_into().unwrap());
    if header_crc != footer_crc {
        return Err(format!(
            "VSZTD header CRC 0x{:08X} != footer CRC 0x{:08X}",
            header_crc, footer_crc
        ));
    }

    let compressed = &input[HEADER_LEN..trailer_start];
    let out = zstd::bulk::decompress(compressed, expected_size as usize)
        .map_err(|e| format!("zstd decode failed: {}", e))?;

    if out.len() as u64 != expected_size {
        return Err(format!(
            "VSZTD size mismatch: decompressed {} vs expected {}",
            out.len(),
            expected_size
        ));
    }

    let actual_crc = crc32fast::hash(&out);
    if actual_crc != header_crc {
        return Err(format!(
            "VSZTD CRC mismatch: got 0x{:08X}, expected 0x{:08X}",
            actual_crc, header_crc
        ));
    }

    Ok(out)
}
