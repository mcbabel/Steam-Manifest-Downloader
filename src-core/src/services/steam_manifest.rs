use std::io::{Cursor, Read};

use aes::Aes256;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use byteorder::{LittleEndian, ReadBytesExt};
use cbc::cipher::block_padding::{NoPadding, Pkcs7};
use cbc::cipher::{BlockModeDecrypt, KeyInit, KeyIvInit};
use protobuf::Message;
use steam_vent_proto::content_manifest::{
    ContentManifestMetadata, ContentManifestPayload, ContentManifestSignature,
};

type Aes256EcbDec = ecb::Decryptor<Aes256>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;

const PAYLOAD_MAGIC: u32 = 0x71F617D0;
const METADATA_MAGIC: u32 = 0x1F4812BE;
const SIGNATURE_MAGIC: u32 = 0x1B81B817;
const ENDOFMANIFEST_MAGIC: u32 = 0x32C415AB;

#[derive(Debug)]
pub struct DecodedManifest {
    pub payload: ContentManifestPayload,
}

pub fn decode_manifest(bytes: &[u8], depot_key: &[u8; 32]) -> Result<DecodedManifest, String> {
    let raw_owned;
    let raw: &[u8] = if bytes.len() >= 4 && &bytes[0..4] == b"PK\x03\x04" {
        raw_owned = unzip_single_entry(bytes)?;
        raw_owned.as_slice()
    } else {
        bytes
    };
    let mut cursor = Cursor::new(raw);

    let mut payload = read_section(&mut cursor, &raw, PAYLOAD_MAGIC, "payload")
        .and_then(parse::<ContentManifestPayload>)?;

    let metadata = read_section(&mut cursor, &raw, METADATA_MAGIC, "metadata")
        .and_then(parse::<ContentManifestMetadata>)?;

    read_section(&mut cursor, &raw, SIGNATURE_MAGIC, "signature")
        .and_then(parse::<ContentManifestSignature>)?;

    let end = cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("end-of-manifest read failed: {}", e))?;
    if end != ENDOFMANIFEST_MAGIC {
        return Err(format!(
            "End-of-manifest magic mismatch: got 0x{:08X}",
            end
        ));
    }

    if metadata.filenames_encrypted.unwrap_or(false) {
        for mapping in payload.mappings.iter_mut() {
            if let Some(filename_b64) = mapping.filename.take() {
                mapping.filename = Some(decrypt_filename(&filename_b64, depot_key)?);
            }
        }
    }

    Ok(DecodedManifest { payload })
}

fn read_section<'a>(
    cursor: &mut Cursor<&[u8]>,
    raw: &'a [u8],
    expected_magic: u32,
    label: &'static str,
) -> Result<&'a [u8], String> {
    let magic = cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("{} header read failed: {}", label, e))?;
    if magic != expected_magic {
        return Err(format!(
            "{} magic mismatch: got 0x{:08X}, expected 0x{:08X}",
            label, magic, expected_magic
        ));
    }
    let len = cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("{} length read failed: {}", label, e))? as usize;
    let start = cursor.position() as usize;
    let end = start
        .checked_add(len)
        .ok_or_else(|| format!("{} length overflow", label))?;
    if end > raw.len() {
        return Err(format!("{} extends past manifest body", label));
    }
    cursor.set_position(end as u64);
    Ok(&raw[start..end])
}

fn parse<T: Message>(bytes: &[u8]) -> Result<T, String> {
    T::parse_from_bytes(bytes).map_err(|e| format!("protobuf parse failed: {}", e))
}

fn decrypt_filename(b64: &str, depot_key: &[u8; 32]) -> Result<String, String> {
    let stripped: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = B64
        .decode(&stripped)
        .map_err(|e| format!("filename base64 decode failed: {}", e))?;
    let plain = symmetric_decrypt(&bytes, depot_key)?;
    let trimmed = plain
        .iter()
        .position(|&b| b == 0)
        .map(|idx| &plain[..idx])
        .unwrap_or(&plain);
    String::from_utf8(trimmed.to_vec()).map_err(|e| format!("filename not valid UTF-8: {}", e))
}

pub fn symmetric_decrypt(ciphertext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if ciphertext.len() < 16 {
        return Err("symmetric ciphertext too short".to_string());
    }
    let (iv_block, body) = ciphertext.split_at(16);
    let mut iv = [0u8; 16];
    iv.copy_from_slice(iv_block);

    let ecb = <Aes256EcbDec as KeyInit>::new_from_slice(key)
        .map_err(|e| format!("AES-256-ECB init failed: {}", e))?;
    let mut iv_buf = iv;
    ecb.decrypt_padded::<NoPadding>(&mut iv_buf)
        .map_err(|e| format!("AES-256-ECB IV decrypt failed: {}", e))?;

    let mut buf = body.to_vec();
    let cbc = <Aes256CbcDec as KeyIvInit>::new_from_slices(key, &iv_buf)
        .map_err(|e| format!("AES-256-CBC init failed: {}", e))?;
    let plain = cbc
        .decrypt_padded::<Pkcs7>(&mut buf)
        .map_err(|e| format!("AES-256-CBC decrypt failed: {}", e))?;
    Ok(plain.to_vec())
}

fn unzip_single_entry(zip_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let reader = Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| format!("manifest zip open failed: {}", e))?;
    if archive.is_empty() {
        return Err("manifest zip is empty".to_string());
    }
    let mut entry = archive
        .by_index(0)
        .map_err(|e| format!("manifest zip entry read failed: {}", e))?;
    let mut out = Vec::with_capacity(entry.size() as usize);
    entry
        .read_to_end(&mut out)
        .map_err(|e| format!("manifest zip entry decompress failed: {}", e))?;
    Ok(out)
}

#[cfg(test)]
mod symmetric_decrypt_tests {
    use super::*;

    const GOLDEN_KEY: [u8; 32] = [7u8; 32];
    const GOLDEN_CIPHERTEXT: &str = "7c8c709c1cb19dd3ad88f73e01685647137dc98e95ef3bbbc48263ed59aa86a82527c6c8fcc1ccf2a7c099790b6d99a2";
    const GOLDEN_PLAINTEXT: &[u8] = b"steam manifest golden vector 42";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn a_known_steam_ciphertext_still_decrypts_to_the_same_bytes() {
        let out = symmetric_decrypt(&unhex(GOLDEN_CIPHERTEXT), &GOLDEN_KEY).unwrap();
        assert_eq!(out, GOLDEN_PLAINTEXT);
    }

    #[test]
    fn a_wrong_key_does_not_yield_the_plaintext() {
        let mut key = GOLDEN_KEY;
        key[0] ^= 0xff;
        match symmetric_decrypt(&unhex(GOLDEN_CIPHERTEXT), &key) {
            Ok(out) => assert_ne!(out, GOLDEN_PLAINTEXT),
            Err(_) => {}
        }
    }

    #[test]
    fn a_truncated_ciphertext_is_rejected_instead_of_panicking() {
        assert!(symmetric_decrypt(&[0u8; 8], &GOLDEN_KEY).is_err());
    }

    #[test]
    fn a_tampered_body_never_returns_the_original_plaintext() {
        let mut ct = unhex(GOLDEN_CIPHERTEXT);
        let last = ct.len() - 1;
        ct[last] ^= 0x01;
        match symmetric_decrypt(&ct, &GOLDEN_KEY) {
            Ok(out) => assert_ne!(out, GOLDEN_PLAINTEXT),
            Err(_) => {}
        }
    }
}
