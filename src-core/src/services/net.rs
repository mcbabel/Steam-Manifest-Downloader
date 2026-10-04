use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use reqwest::{Client, ClientBuilder, Proxy};

struct Current {
    proxy: Option<String>,
    client: &'static Client,
}

static CURRENT: OnceLock<RwLock<Current>> = OnceLock::new();

fn slot() -> &'static RwLock<Current> {
    CURRENT.get_or_init(|| {
        RwLock::new(Current {
            proxy: None,
            client: Box::leak(Box::new(build(None).unwrap_or_default())),
        })
    })
}

pub struct HttpClient;

impl std::ops::Deref for HttpClient {
    type Target = Client;

    fn deref(&self) -> &Client {
        client()
    }
}

pub fn client() -> &'static Client {
    slot().read().map(|c| c.client).unwrap_or_else(|e| e.into_inner().client)
}

pub fn proxy() -> Option<String> {
    slot().read().ok().and_then(|c| c.proxy.clone())
}

pub fn parse_proxy(raw: &str) -> Result<Option<String>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("http://{}", raw)
    };
    let url = reqwest::Url::parse(&with_scheme).map_err(|_| format!("Invalid proxy address: {}", raw))?;
    match url.scheme() {
        "http" | "https" | "socks5" | "socks5h" => {}
        other => return Err(format!("Unsupported proxy type: {}", other)),
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(format!("Invalid proxy address: {}", raw));
    }
    Proxy::all(url.as_str()).map_err(|_| format!("Invalid proxy address: {}", raw))?;
    Ok(Some(url.to_string().trim_end_matches('/').to_string()))
}

pub fn builder() -> ClientBuilder {
    with_proxy(Client::builder(), proxy().as_deref())
}

fn with_proxy(builder: ClientBuilder, proxy: Option<&str>) -> ClientBuilder {
    let builder = builder.connect_timeout(Duration::from_secs(20));
    match proxy.and_then(|p| Proxy::all(p).ok()) {
        Some(p) => builder.proxy(p),
        None => builder,
    }
}

fn build(proxy: Option<&str>) -> Result<Client, String> {
    with_proxy(Client::builder(), proxy)
        .build()
        .map_err(|e| describe(&e))
}

pub fn apply_proxy(raw: &str) -> Result<(), String> {
    let wanted = parse_proxy(raw)?;
    let mut current = slot().write().unwrap_or_else(|e| e.into_inner());
    if current.proxy == wanted {
        return Ok(());
    }
    let client = build(wanted.as_deref())?;
    current.client = Box::leak(Box::new(client));
    current.proxy = wanted;
    Ok(())
}

pub const PROXY_TEST_URL: &str = "https://api.steampowered.com/ISteamWebAPIUtil/GetServerInfo/v1/";

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProxyTest {
    pub proxy: Option<String>,
    pub millis: u64,
    pub status: u16,
}

pub async fn test_proxy(raw: &str) -> Result<ProxyTest, String> {
    test_proxy_against(raw, PROXY_TEST_URL).await
}

async fn test_proxy_against(raw: &str, url: &str) -> Result<ProxyTest, String> {
    let proxy = parse_proxy(raw)?;
    let client = with_proxy(Client::builder(), proxy.as_deref())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| describe(&e))?;
    let started = std::time::Instant::now();
    let response = client
        .get(url)
        .header("User-Agent", "SteamManifestDownloader")
        .send()
        .await
        .map_err(|e| explain(&describe(&e), proxy.is_some()))?;
    let status = response.status();
    if status == reqwest::StatusCode::PROXY_AUTHENTICATION_REQUIRED {
        return Err("Proxy requires a user name and password (HTTP 407)".to_string());
    }
    if status.is_server_error() && proxy.is_some() && status.as_u16() == 502 {
        return Err("Proxy could not reach Steam (HTTP 502)".to_string());
    }
    Ok(ProxyTest {
        proxy,
        millis: started.elapsed().as_millis() as u64,
        status: status.as_u16(),
    })
}

fn explain(detail: &str, via_proxy: bool) -> String {
    let lower = detail.to_ascii_lowercase();
    let summary = if lower.contains("connection refused") {
        if via_proxy {
            "Connection refused, is a proxy running at this address and port?"
        } else {
            "Connection refused"
        }
    } else if lower.contains("dns error") || lower.contains("failed to lookup address") {
        if via_proxy {
            "Proxy address not found, check the host name"
        } else {
            "Steam could not be found, check your DNS or internet connection"
        }
    } else if lower.contains("timed out") {
        "No answer in time, the connection is blocked or too slow"
    } else if lower.contains("certificate") || lower.contains("handshake") {
        "Secure connection failed, an antivirus or company proxy may be intercepting HTTPS"
    } else {
        return detail.to_string();
    };
    format!("{} ({})", summary, detail)
}

pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find("://") {
        let start = rest[..pos]
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.'))
            .map(|i| i + 1)
            .unwrap_or(0);
        let end = rest[pos..]
            .find(|c: char| c.is_whitespace() || c == ')' || c == '"' || c == '\'')
            .map(|i| pos + i)
            .unwrap_or(rest.len());
        out.push_str(&rest[..start]);
        let url = &rest[start..end];
        out.push_str(url.split('?').next().unwrap_or(url));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

pub fn describe(err: &reqwest::Error) -> String {
    let mut parts: Vec<String> = vec![redact(&err.to_string())];
    let mut source = std::error::Error::source(err);
    while let Some(cause) = source {
        let text = redact(&cause.to_string());
        if !text.is_empty() && !parts.iter().any(|p| p.contains(&text)) {
            parts.push(text);
        }
        source = cause.source();
    }
    parts.join(": ")
}

pub fn is_connection_problem(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "error sending request",
        "dns error",
        "failed to lookup address",
        "tcp connect error",
        "connection refused",
        "connection reset",
        "connection timed out",
        "operation timed out",
        "timed out",
        "network is unreachable",
        "certificate",
        "handshake",
        "proxy",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_keys_from_urls() {
        assert_eq!(
            redact("error sending request for url (https://api.example.com/manifest?apikey=secret&depotid=1)"),
            "error sending request for url (https://api.example.com/manifest)"
        );
        assert_eq!(redact("no url here"), "no url here");
        assert_eq!(
            redact("a https://x.org/a?b=1 and http://y.org/c?d=2."),
            "a https://x.org/a and http://y.org/c"
        );
    }

    #[test]
    fn checks_proxy_addresses() {
        assert_eq!(parse_proxy("").unwrap(), None);
        assert_eq!(parse_proxy("proxy.local:8080").unwrap().as_deref(), Some("http://proxy.local:8080"));
        assert_eq!(
            parse_proxy("socks5://127.0.0.1:1080").unwrap().as_deref(),
            Some("socks5://127.0.0.1:1080")
        );
        assert!(parse_proxy("ftp://proxy:21").is_err());
        assert!(parse_proxy("http://").is_err());
    }

    #[test]
    fn recognises_connection_problems() {
        assert!(is_connection_problem("Steam direct failed (error sending request for url (https://x))"));
        assert!(is_connection_problem("invalid peer certificate: UnknownIssuer"));
        assert!(!is_connection_problem("Manifest not found: 1_2.manifest (HTTP 404)"));
    }

    #[tokio::test]
    async fn requests_go_through_the_configured_proxy() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 2048];
            let n = sock.read(&mut buf).await.unwrap();
            sock.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok")
                .await
                .unwrap();
            String::from_utf8_lossy(&buf[..n]).to_string()
        });
        apply_proxy(&format!("127.0.0.1:{}", port)).unwrap();
        assert_eq!(proxy().as_deref(), Some(format!("http://127.0.0.1:{}", port).as_str()));
        let body = client()
            .get("http://smd-proxy-test.invalid/hello?apikey=secret")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert_eq!(body, "ok");
        let request = server.await.unwrap();
        assert!(request.starts_with("GET http://smd-proxy-test.invalid/hello"), "{}", request);
        apply_proxy("").unwrap();
        assert_eq!(proxy(), None);
    }

    #[tokio::test]
    async fn errors_name_the_real_cause_without_keys() {
        let client = Client::builder().no_proxy().build().unwrap();
        let err = client
            .get("http://127.0.0.1:1/manifest?apikey=secret")
            .send()
            .await
            .unwrap_err();
        let text = describe(&err);
        assert!(!text.contains("secret"), "{}", text);
        assert!(text.len() > "error sending request for url (http://127.0.0.1:1/manifest)".len(), "{}", text);
        assert!(is_connection_problem(&text), "{}", text);
        println!("{}", text);
    }

    #[tokio::test]
    async fn proxy_test_reports_success_and_failure() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            for reply in [&b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n"[..], &b"HTTP/1.1 407 Proxy Authentication Required\r\ncontent-length: 0\r\n\r\n"[..]] {
                let (mut sock, _) = listener.accept().await.unwrap();
                let mut buf = vec![0u8; 2048];
                let _ = sock.read(&mut buf).await;
                sock.write_all(reply).await.unwrap();
            }
        });
        let proxy = format!("http://127.0.0.1:{}", port);
        let ok = test_proxy_against(&proxy, "http://steam.invalid/").await.unwrap();
        assert_eq!(ok.status, 200);
        assert_eq!(ok.proxy.as_deref(), Some(proxy.as_str()));
        let auth = test_proxy_against(&proxy, "http://steam.invalid/").await.unwrap_err();
        assert!(auth.contains("407"), "{}", auth);
        let dead = test_proxy_against("http://127.0.0.1:1", "http://steam.invalid/").await.unwrap_err();
        assert!(is_connection_problem(&dead), "{}", dead);
        assert!(test_proxy_against("ftp://x:1", "http://steam.invalid/").await.is_err());
    }
}

