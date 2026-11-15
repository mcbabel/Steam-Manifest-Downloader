use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    Unknown,
    Loading,
    Ready([f32; 2]),
    Failed,
}

#[derive(Default)]
struct Inner {
    entries: HashMap<String, Status>,
    wanted: Vec<String>,
}

#[derive(Clone, Default)]
pub struct Images {
    inner: Arc<Mutex<Inner>>,
}

impl Images {
    pub fn status(&self, uri: &str) -> Status {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match inner.entries.get(uri) {
            Some(status) => *status,
            None => {
                inner.entries.insert(uri.to_string(), Status::Loading);
                inner.wanted.push(uri.to_string());
                Status::Unknown
            }
        }
    }

    pub fn take_wanted(&self) -> Vec<String> {
        std::mem::take(&mut self.inner.lock().unwrap_or_else(|e| e.into_inner()).wanted)
    }

    pub fn set(&self, uri: &str, status: Status) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).entries.insert(uri.to_string(), status);
    }

    pub fn forget(&self, uri: &str) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).entries.remove(uri);
    }
}

pub fn dimensions(bytes: &[u8]) -> Option<[f32; 2]> {
    if bytes.starts_with(b"<") || bytes.windows(4).take(512).any(|w| w == b"<svg") {
        return Some([24.0, 24.0]);
    }
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?;
    let (width, height) = reader.into_dimensions().ok()?;
    Some([width as f32, height as f32])
}

fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return None,
        };
        buffer = (buffer << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

pub async fn fetch(uri: &str) -> Option<Vec<u8>> {
    if let Some(rest) = uri.strip_prefix("data:") {
        let (meta, data) = rest.split_once(',')?;
        return if meta.ends_with(";base64") { decode_base64(data) } else { Some(data.as_bytes().to_vec()) };
    }
    if uri.starts_with("http://") || uri.starts_with("https://") {
        let reply = reqwest::Client::new().get(uri).timeout(std::time::Duration::from_secs(20)).send().await.ok()?;
        if !reply.status().is_success() {
            return None;
        }
        return reply.bytes().await.ok().map(|b| b.to_vec());
    }
    let path = uri.strip_prefix("file://").unwrap_or(uri);
    tokio::fs::read(path).await.ok()
}
