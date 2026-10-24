//! Calls to the GitHub REST API, and to the few HTTPS endpoints outside it that a credential
//! exchange needs, made in process, so a credential travels only as an `Authorization: Bearer`
//! header or a request body inside the TLS stream: never in a child process's arguments,
//! environment or output. TLS is rustls on aws-lc-rs with the webpki root store compiled in.

use std::sync::Arc;
use std::time::Duration;

use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

const GITHUB_API: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";
const USER_AGENT: &str = "stencil-xtask";
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const RESPONSE_MAX_BYTES: u64 = 256 * 1024;
const GITHUB_HEADERS: [(&str, &str); 2] = [
    ("Accept", "application/vnd.github+json"),
    ("X-GitHub-Api-Version", API_VERSION),
];
const PLAIN_HEADERS: [(&str, &str); 1] = [("Accept", "application/json")];

/// How a plain request authenticates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Credential<'a> {
    None,
    Bearer(&'a str),
    /// The base64 of `user:password`.
    Basic(&'a str),
}

impl Credential<'_> {
    fn header(self) -> Option<String> {
        match self {
            Self::None => None,
            Self::Bearer(token) => Some(format!("Bearer {token}")),
            Self::Basic(encoded) => Some(format!("Basic {encoded}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Delete,
}

/// What a request came back with. Transport failures carry a fixed description, never the
/// error's text, so no header or URL detail can ride along.
#[derive(Debug, PartialEq, Eq)]
pub enum Reply {
    Response { status: u16, body: String },
    BodyTooLarge,
    Failed(&'static str),
}

/// An agent that never follows a redirect, ignores proxy variables, and, when `https_only`,
/// refuses plain HTTP; only tests build one without it, to talk to a loopback listener.
fn agent(https_only: bool) -> Agent {
    let tls = TlsConfig::builder()
        .provider(TlsProvider::Rustls)
        .root_certs(RootCerts::WebPki)
        .unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::aws_lc_rs::default_provider()))
        .build();
    Agent::config_builder()
        .tls_config(tls)
        .https_only(https_only)
        .proxy(None)
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_global(Some(REQUEST_TIMEOUT))
        .user_agent(USER_AGENT)
        .build()
        .into()
}

const fn failure(error: &ureq::Error) -> &'static str {
    match error {
        ureq::Error::Timeout(_) => "the request timed out",
        ureq::Error::Io(_) => "the connection failed",
        ureq::Error::Tls(_) | ureq::Error::Rustls(_) => "the TLS handshake failed",
        ureq::Error::RequireHttpsOnly(_) => "the URL is not HTTPS",
        _ => "the request failed",
    }
}

/// A client for one API base. Outside tests the only one is GitHub's, HTTPS only.
#[derive(Debug)]
pub struct Client {
    base: String,
    agent: Agent,
}

impl Client {
    pub fn github() -> Self {
        Self {
            base: GITHUB_API.to_string(),
            agent: agent(true),
        }
    }

    #[cfg(test)]
    pub fn loopback(base: &str) -> Self {
        Self {
            base: base.to_string(),
            agent: agent(false),
        }
    }

    /// Sends one request to the base + `path` with `credential` as the bearer token and reads
    /// the body up to [`RESPONSE_MAX_BYTES`].
    pub fn request(
        &self,
        method: Method,
        path: &str,
        credential: &str,
        json_body: Option<&str>,
    ) -> Reply {
        send(
            &self.agent,
            &format!("{}{path}", self.base),
            method,
            &GITHUB_HEADERS,
            Credential::Bearer(credential),
            json_body,
        )
    }
}

/// A client for absolute HTTPS URLs outside GitHub's REST API, such as the Actions OIDC
/// endpoint and a token exchange; it sends JSON headers only, and a credential only when given.
#[derive(Debug)]
pub struct PlainClient {
    agent: Agent,
}

impl PlainClient {
    pub fn https() -> Self {
        Self { agent: agent(true) }
    }

    #[cfg(test)]
    pub fn loopback() -> Self {
        Self {
            agent: agent(false),
        }
    }

    pub fn request(
        &self,
        method: Method,
        url: &str,
        credential: Credential<'_>,
        json_body: Option<&str>,
    ) -> Reply {
        self.request_with(method, url, credential, &[], json_body)
    }

    /// A request with headers beyond the JSON ones, such as an API version.
    pub fn request_with(
        &self,
        method: Method,
        url: &str,
        credential: Credential<'_>,
        headers: &[(&str, &str)],
        json_body: Option<&str>,
    ) -> Reply {
        let all: Vec<(&str, &str)> = PLAIN_HEADERS.iter().chain(headers).copied().collect();
        send(&self.agent, url, method, &all, credential, json_body)
    }
}

fn with_headers<Body>(
    mut builder: ureq::RequestBuilder<Body>,
    headers: &[(&str, &str)],
    authorization: Option<&str>,
) -> ureq::RequestBuilder<Body> {
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    if let Some(authorization) = authorization {
        builder = builder.header("Authorization", authorization);
    }
    builder
}

fn send(
    agent: &Agent,
    url: &str,
    method: Method,
    headers: &[(&str, &str)],
    credential: Credential<'_>,
    json_body: Option<&str>,
) -> Reply {
    let authorization = credential.header();
    let authorization = authorization.as_deref();
    let sent = match method {
        Method::Get => with_headers(agent.get(url), headers, authorization).call(),
        Method::Delete => with_headers(agent.delete(url), headers, authorization).call(),
        Method::Post => with_headers(agent.post(url), headers, authorization)
            .content_type("application/json")
            .send(json_body.unwrap_or("{}")),
    };
    let mut response = match sent {
        Ok(response) => response,
        Err(error) => return Reply::Failed(failure(&error)),
    };
    let status = response.status().as_u16();
    match response
        .body_mut()
        .with_config()
        .limit(RESPONSE_MAX_BYTES)
        .read_to_string()
    {
        Ok(body) => Reply::Response { status, body },
        Err(ureq::Error::BodyExceedsLimit(_)) => Reply::BodyTooLarge,
        Err(error) => Reply::Failed(failure(&error)),
    }
}

/// GitHub's `message` from an error body, kept only when it reads as a short sentence: no word
/// longer than 32 characters and only letters, digits, spaces and plain punctuation, so a body
/// that echoes a token, a JWT or key material contributes nothing.
pub fn github_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let message = value.get("message")?.as_str()?;
    let plain = message.len() <= 160
        && message
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || " .,'()-".contains(c))
        && message.split(' ').all(|word| word.len() <= 32);
    plain.then(|| message.to_string())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    const REQUEST_MAX_BYTES: usize = 64 * 1024;

    /// A one-connection-per-reply HTTP listener on loopback: answers each request with the next
    /// canned `(status, body)` and hands back every raw request it read.
    pub fn listener(replies: Vec<(u16, String)>) -> (String, mpsc::Receiver<Vec<String>>) {
        let bound = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", bound.local_addr().unwrap());
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut seen = Vec::new();
            for (status, body) in replies {
                let (mut stream, _) = bound.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                seen.push(read_request(&mut stream));
                let reply = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(reply.as_bytes()).unwrap();
            }
            sender.send(seen).unwrap();
        });
        (base, receiver)
    }

    fn read_request(stream: &mut std::net::TcpStream) -> String {
        let mut raw = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let read = stream.read(&mut chunk).unwrap();
            raw.extend_from_slice(&chunk[..read]);
            let text = String::from_utf8_lossy(&raw).into_owned();
            if let Some(end) = text.find("\r\n\r\n") {
                let length = text
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(str::to_string)
                    })
                    .map_or(0, |value| value.trim().parse::<usize>().unwrap());
                if raw.len() >= end.saturating_add(4).saturating_add(length) {
                    return text;
                }
            }
            assert!(read > 0 && raw.len() < REQUEST_MAX_BYTES, "short request");
        }
    }

    #[test]
    fn the_credential_travels_once_as_a_bearer_header() {
        let (base, seen) = listener(vec![(200, r#"{"id":42}"#.to_string())]);
        let reply = Client::loopback(&base).request(
            Method::Get,
            "/repos/o/r/installation",
            "the.jwt.value",
            None,
        );
        assert_eq!(
            reply,
            Reply::Response {
                status: 200,
                body: r#"{"id":42}"#.to_string()
            }
        );
        let requests = seen.recv().unwrap();
        let raw = requests.first().unwrap();
        assert!(
            raw.starts_with("GET /repos/o/r/installation HTTP/1.1\r\n"),
            "{raw}"
        );
        assert!(
            raw.contains("\r\nauthorization: Bearer the.jwt.value\r\n")
                || raw.contains("\r\nAuthorization: Bearer the.jwt.value\r\n"),
            "{raw}"
        );
        assert_eq!(raw.matches("the.jwt.value").count(), 1, "{raw}");
        assert!(
            raw.to_ascii_lowercase()
                .contains("\r\nx-github-api-version: 2022-11-28\r\n")
        );
    }

    #[test]
    fn a_post_carries_its_json_body_and_a_body_over_the_cap_is_refused() {
        let oversized = "x".repeat(
            usize::try_from(RESPONSE_MAX_BYTES)
                .unwrap()
                .saturating_add(1),
        );
        let (base, seen) = listener(vec![(201, oversized)]);
        let reply =
            Client::loopback(&base).request(Method::Post, "/p", "credential", Some(r#"{"a":1}"#));
        assert_eq!(reply, Reply::BodyTooLarge);
        let requests = seen.recv().unwrap();
        let raw = requests.first().unwrap();
        assert!(raw.starts_with("POST /p HTTP/1.1\r\n"), "{raw}");
        assert!(raw.ends_with("\r\n\r\n{\"a\":1}"), "{raw}");
    }

    #[test]
    fn a_plain_request_carries_json_headers_and_a_credential_only_when_given() {
        let (base, seen) = listener(vec![
            (200, r#"{"value":"v"}"#.to_string()),
            (200, "{}".to_string()),
        ]);
        let client = PlainClient::loopback();
        let with_credential = client.request(
            Method::Get,
            &format!("{base}/token?api-version=2.0&audience=a"),
            Credential::Bearer("request.token"),
            None,
        );
        assert!(matches!(
            with_credential,
            Reply::Response { status: 200, .. }
        ));
        let without = client.request_with(
            Method::Post,
            &format!("{base}/x"),
            Credential::Basic("cGs6c2s="),
            &[("x-langfuse-ingestion-version", "4")],
            Some("{}"),
        );
        assert!(matches!(without, Reply::Response { status: 200, .. }));
        let requests = seen.recv().unwrap();
        let first = requests.first().unwrap().to_ascii_lowercase();
        assert!(
            first.starts_with("get /token?api-version=2.0&audience=a http/1.1\r\n"),
            "{first}"
        );
        assert!(first.contains("\r\nauthorization: bearer request.token\r\n"));
        assert!(first.contains("\r\naccept: application/json\r\n"));
        assert!(!first.contains("x-github-api-version"));
        let second = requests.get(1).unwrap().to_ascii_lowercase();
        assert!(
            second.contains("\r\nauthorization: basic cgs6c2s=\r\n"),
            "{second}"
        );
        assert!(second.contains("\r\nx-langfuse-ingestion-version: 4\r\n"));
        assert!(second.contains("\r\naccept: application/json\r\n"));
    }

    #[test]
    fn the_plain_client_is_https_only_without_proxy_or_redirects() {
        let config = PlainClient::https().agent.config().clone();
        assert!(config.https_only());
        assert!(config.proxy().is_none());
        assert_eq!(config.max_redirects(), 0);
        assert_eq!(
            PlainClient::https().request(
                Method::Get,
                "http://127.0.0.1:9/",
                Credential::None,
                None
            ),
            Reply::Failed("the URL is not HTTPS")
        );
    }

    #[test]
    fn a_refused_connection_is_a_fixed_failure() {
        let unused = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", unused.local_addr().unwrap());
        drop(unused);
        assert_eq!(
            Client::loopback(&base).request(Method::Get, "/", "secret", None),
            Reply::Failed("the connection failed")
        );
    }

    #[test]
    fn the_github_client_is_https_only_without_proxy_or_redirects() {
        let client = Client::github();
        let config = client.agent.config();
        assert!(config.https_only());
        assert!(config.proxy().is_none());
        assert_eq!(config.max_redirects(), 0);
        let plain = Client {
            base: "http://127.0.0.1:9".to_string(),
            agent: agent(true),
        };
        assert_eq!(
            plain.request(Method::Get, "/", "secret", None),
            Reply::Failed("the URL is not HTTPS")
        );
    }

    #[test]
    fn a_redirect_is_returned_not_followed() {
        let second = TcpListener::bind("127.0.0.1:0").unwrap();
        second.set_nonblocking(true).unwrap();
        let target = format!("http://{}/elsewhere", second.local_addr().unwrap());
        let bound = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", bound.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut stream, _) = bound.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let raw = read_request(&mut stream);
            let reply = format!(
                "HTTP/1.1 302 Found\r\nLocation: {target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(reply.as_bytes()).unwrap();
            raw
        });
        let reply = Client::loopback(&base).request(Method::Get, "/start", "secret", None);
        assert_eq!(
            reply,
            Reply::Response {
                status: 302,
                body: String::new()
            }
        );
        assert!(
            server
                .join()
                .unwrap()
                .starts_with("GET /start HTTP/1.1\r\n")
        );
        let followed = second.accept();
        assert!(
            matches!(&followed, Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
            "{followed:?}"
        );
    }

    #[test]
    fn only_a_plain_short_message_is_kept() {
        for (body, kept) in [
            (
                r#"{"message":"A JSON web token could not be decoded"}"#,
                Some("A JSON web token could not be decoded"),
            ),
            (
                r#"{"message":"Integration not found"}"#,
                Some("Integration not found"),
            ),
            (
                r#"{"message":"Bad credentials ghs_16C7e42F292c6912E7710c838347Ae178B4a"}"#,
                None,
            ),
            (
                r#"{"message":"key MIIEowIBAAKCAQEAxyzxyzxyzxyzxyzxyzxyzxyzxyzxyz"}"#,
                None,
            ),
            (
                r#"{"message":"eyJhbGciOiJSUzI1NiJ9.eyJpc3MiOiIxIn0.c2ln"}"#,
                None,
            ),
            ("not json", None),
        ] {
            assert_eq!(github_message(body).as_deref(), kept, "{body}");
        }
    }
}
