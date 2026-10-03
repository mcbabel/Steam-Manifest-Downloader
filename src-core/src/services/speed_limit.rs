use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

const MEBIBYTE: f64 = 1024.0 * 1024.0;

pub fn parse_speed_limit(input: &str) -> Result<Option<u64>, String> {
    let text = input.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let split = text
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ','))
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let value: f64 = number
        .replace(',', ".")
        .parse()
        .map_err(|_| format!("Invalid speed limit: {}", input))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("Invalid speed limit: {}", input));
    }
    let unit = unit.trim().to_ascii_lowercase().replace(' ', "");
    let unit = unit.strip_suffix("/s").unwrap_or(&unit);
    let bytes_per_unit = match unit {
        "" | "m" | "mb" | "mib" => MEBIBYTE,
        "k" | "kb" | "kib" => 1024.0,
        "g" | "gb" | "gib" => 1024.0 * MEBIBYTE,
        "mbit" | "mbits" | "mbps" => 1_000_000.0 / 8.0,
        "kbit" | "kbits" | "kbps" => 1_000.0 / 8.0,
        "gbit" | "gbits" | "gbps" => 1_000_000_000.0 / 8.0,
        _ => return Err(format!("Unknown speed unit in: {}", input)),
    };
    let bytes = (value * bytes_per_unit).round();
    if bytes < 1.0 {
        return Ok(None);
    }
    Ok(Some(bytes as u64))
}

#[derive(Default)]
struct Pacer {
    next_slot: Option<Instant>,
}

impl Pacer {
    fn reserve(&mut self, now: Instant, bytes: u64, bytes_per_sec: u64) -> Instant {
        let start = match self.next_slot {
            Some(slot) if slot > now => slot,
            _ => now,
        };
        let cost = Duration::from_secs_f64(bytes as f64 / bytes_per_sec as f64);
        self.next_slot = Some(start + cost);
        start
    }
}

struct Limiter {
    bytes_per_sec: AtomicU64,
    pacer: Mutex<Pacer>,
}

fn limiter() -> &'static Limiter {
    static LIMITER: OnceLock<Limiter> = OnceLock::new();
    LIMITER.get_or_init(|| Limiter {
        bytes_per_sec: AtomicU64::new(0),
        pacer: Mutex::new(Pacer::default()),
    })
}

pub fn set_limit(bytes_per_sec: Option<u64>) {
    let l = limiter();
    l.bytes_per_sec.store(bytes_per_sec.unwrap_or(0), Ordering::SeqCst);
    if let Ok(mut pacer) = l.pacer.try_lock() {
        pacer.next_slot = None;
    }
}

pub fn current_limit() -> Option<u64> {
    match limiter().bytes_per_sec.load(Ordering::SeqCst) {
        0 => None,
        v => Some(v),
    }
}

pub async fn throttle(bytes: u64, cancel: &AtomicBool) -> Result<(), String> {
    if bytes == 0 {
        return Ok(());
    }
    'reserve: loop {
        let Some(rate) = current_limit() else {
            return Ok(());
        };
        let start = limiter().pacer.lock().await.reserve(Instant::now(), bytes, rate);
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err("cancelled".to_string());
            }
            if current_limit() != Some(rate) {
                continue 'reserve;
            }
            let now = Instant::now();
            if now >= start {
                return Ok(());
            }
            tokio::time::sleep((start - now).min(Duration::from_millis(200))).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: u64 = 1024 * 1024;

    #[test]
    fn empty_and_zero_mean_unlimited() {
        assert_eq!(parse_speed_limit(""), Ok(None));
        assert_eq!(parse_speed_limit("   "), Ok(None));
        assert_eq!(parse_speed_limit("0"), Ok(None));
        assert_eq!(parse_speed_limit("0 MB/s"), Ok(None));
    }

    #[test]
    fn a_plain_number_is_megabytes_per_second() {
        assert_eq!(parse_speed_limit("10"), Ok(Some(10 * MIB)));
        assert_eq!(parse_speed_limit("2.5"), Ok(Some(5 * MIB / 2)));
        assert_eq!(parse_speed_limit("2,5"), Ok(Some(5 * MIB / 2)));
    }

    #[test]
    fn byte_units_are_understood() {
        assert_eq!(parse_speed_limit("10 MB/s"), Ok(Some(10 * MIB)));
        assert_eq!(parse_speed_limit("10mb"), Ok(Some(10 * MIB)));
        assert_eq!(parse_speed_limit("512 KB/s"), Ok(Some(512 * 1024)));
        assert_eq!(parse_speed_limit("1 GB/s"), Ok(Some(1024 * MIB)));
    }

    #[test]
    fn bit_units_use_the_provider_convention() {
        assert_eq!(parse_speed_limit("75 Mbit/s"), Ok(Some(9_375_000)));
        assert_eq!(parse_speed_limit("75mbps"), Ok(Some(9_375_000)));
        assert_eq!(parse_speed_limit("100 Mbits"), Ok(Some(12_500_000)));
        assert_eq!(parse_speed_limit("1 Gbit/s"), Ok(Some(125_000_000)));
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(parse_speed_limit("fast").is_err());
        assert!(parse_speed_limit("10 parsecs").is_err());
        assert!(parse_speed_limit("1.2.3").is_err());
    }

    #[test]
    fn reservations_are_spaced_by_their_size() {
        let mut pacer = Pacer::default();
        let t0 = Instant::now();
        let rate = 1_000_000;
        assert_eq!(pacer.reserve(t0, 500_000, rate), t0);
        assert_eq!(pacer.reserve(t0, 500_000, rate), t0 + Duration::from_millis(500));
        assert_eq!(pacer.reserve(t0, 1_000_000, rate), t0 + Duration::from_secs(1));
    }

    #[tokio::test]
    async fn throttling_holds_the_configured_rate_and_can_be_lifted() {
        let cancel = AtomicBool::new(false);
        set_limit(Some(1_000_000));
        let started = Instant::now();
        for _ in 0..4 {
            throttle(250_000, &cancel).await.unwrap();
        }
        let elapsed = started.elapsed();
        assert!(elapsed >= Duration::from_millis(700), "{:?}", elapsed);
        assert!(elapsed < Duration::from_millis(1500), "{:?}", elapsed);

        set_limit(None);
        let started = Instant::now();
        for _ in 0..4 {
            throttle(250_000, &cancel).await.unwrap();
        }
        assert!(started.elapsed() < Duration::from_millis(50));

        set_limit(Some(1));
        cancel.store(true, Ordering::SeqCst);
        let started = Instant::now();
        assert!(throttle(1_000_000, &cancel).await.is_err());
        assert!(started.elapsed() < Duration::from_millis(50));
        set_limit(None);
    }

    #[test]
    fn idle_time_is_not_banked_as_a_burst() {
        let mut pacer = Pacer::default();
        let t0 = Instant::now();
        pacer.reserve(t0, 1_000_000, 1_000_000);
        let later = t0 + Duration::from_secs(10);
        assert_eq!(pacer.reserve(later, 1_000_000, 1_000_000), later);
        assert_eq!(
            pacer.reserve(later, 1_000_000, 1_000_000),
            later + Duration::from_secs(1)
        );
    }
}
