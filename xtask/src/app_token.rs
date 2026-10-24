//! GitHub App installation tokens: the review App's, for the code review.
//!
//! The App's private key signs a short JWT, sent as a bearer token to trade for an installation
//! token scoped to one repository and two permissions. Every call is made in process
//! (`github_api`), so the JWT and the token never reach a child process, its arguments, output
//! or error text.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{Algorithm, EncodingKey, Header};

use crate::github_api::{Client, Method, Reply, github_message};

/// One GitHub App: where its credentials come from, and the grant its tokens are limited to.
#[derive(Debug)]
pub struct AppSpec {
    pub client_id_variable: &'static str,
    pub app_id_variable: &'static str,
    pub key_variable: &'static str,
    /// Where the credentials live and why, for the error that names them missing.
    pub setup: &'static str,
    /// The grant requested; GitHub adds `metadata: read` to every installation token.
    pub permissions: &'static [(&'static str, &'static str)],
}

pub const REVIEW: AppSpec = AppSpec {
    client_id_variable: "FRITS_CODE_REVIEW_CLIENT_ID",
    app_id_variable: "FRITS_CODE_REVIEW_APP_ID",
    key_variable: "FRITS_CODE_REVIEW_APP_PRIVATE_KEY",
    setup: "must be set: the review is posted with a GitHub App installation token (Code \
            review in docs/review.md)",
    permissions: &[
        ("checks", "read"),
        ("pull_requests", "write"),
        ("statuses", "read"),
    ],
};
const REPOSITORY_VARIABLE: &str = "GITHUB_REPOSITORY";
const APP_ID_MAX_DIGITS: usize = 20;
/// GitHub App client ids are `Iv` and a short alphanumeric suffix, about twenty characters.
const CLIENT_ID_MAX_CHARS: usize = 64;
const PRIVATE_KEY_MAX_BYTES: usize = 16 * 1024;
const REPOSITORY_MAX_CHARS: usize = 140;
const TOKEN_MAX_CHARS: usize = 1024;
const SLUG_MAX_CHARS: usize = 64;
/// GitHub rejects an `iat` ahead of its clock; backdating absorbs runner clock drift.
const JWT_BACKDATE_SECONDS: u64 = 60;
/// GitHub refuses an `exp` more than ten minutes ahead; with the backdate, `exp` lands nine
/// minutes ahead of now.
pub const JWT_MAX_LIFETIME_SECONDS: u64 = 600;
const IMPLICIT_PERMISSION: (&str, &str) = ("metadata", "read");

/// The App's identity and signing key, read from the environment. `issuer` is the JWT `iss`:
/// the App's client id, or its numeric App id when no client id is set.
pub struct AppCredentials {
    spec: &'static AppSpec,
    issuer: String,
    key: EncodingKey,
}

impl fmt::Debug for AppCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppCredentials")
            .field("app", &self.spec.client_id_variable)
            .field("issuer", &self.issuer)
            .field("key", &"[redacted]")
            .finish()
    }
}

/// An installation token; its value leaves this module only through [`InstallationToken::expose`].
#[derive(PartialEq, Eq)]
pub struct InstallationToken(String);

impl InstallationToken {
    pub fn expose(&self) -> &str {
        &self.0
    }

    #[cfg(test)]
    pub fn for_tests(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for InstallationToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("InstallationToken([redacted])")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Repository {
    owner: String,
    name: String,
}

impl Repository {
    /// `owner/name`, as API paths name it.
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum InstallationOutcome {
    Found(u64),
    /// A non-2xx status and GitHub's message when it reads as plain text.
    Failed(u16, Option<String>),
    Unrecognised(&'static str),
}

/// `Unrecognised` carries a fixed reason, never response text: a malformed success body may
/// hold the token. A token GitHub issued but the xtask refuses travels in its variant so it can
/// be revoked.
#[derive(Debug, PartialEq, Eq)]
enum TokenOutcome {
    Issued(InstallationToken),
    Failed(u16, Option<String>),
    Overgranted(InstallationToken),
    UnreadableGrant(InstallationToken),
    Unrecognised(&'static str),
}

fn missing_credentials(spec: &AppSpec) -> String {
    format!(
        "{} (variable, preferred) or {} (variable, accepted), and {} (secret), {}",
        spec.client_id_variable, spec.app_id_variable, spec.key_variable, spec.setup
    )
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// The JWT issuer: the client id when set, else the numeric App id. A client id that is set
/// but malformed is an error, not a reason to fall back.
fn read_issuer(
    spec: &AppSpec,
    client_id: Option<String>,
    app_id: Option<String>,
) -> Result<String, String> {
    if let Some(client_id) = non_empty(client_id) {
        let valid = client_id.len() <= CLIENT_ID_MAX_CHARS
            && client_id.chars().all(|c| c.is_ascii_alphanumeric());
        return if valid {
            Ok(client_id)
        } else {
            Err(format!(
                "{} is not a GitHub App client id: letters and digits, at most \
                 {CLIENT_ID_MAX_CHARS}",
                spec.client_id_variable
            ))
        };
    }
    let Some(app_id) = non_empty(app_id) else {
        return Err(missing_credentials(spec));
    };
    let digits = app_id.len() <= APP_ID_MAX_DIGITS && app_id.chars().all(|c| c.is_ascii_digit());
    match app_id.parse::<u64>() {
        Ok(number) if digits && number > 0 => Ok(number.to_string()),
        _ => Err(format!("{} is not a GitHub App id", spec.app_id_variable)),
    }
}

/// Reads the App's client id (or its App id) and private key; the key never appears in an
/// error.
pub fn read_credentials(
    spec: &'static AppSpec,
    client_id: Option<String>,
    app_id: Option<String>,
    key: Option<String>,
) -> Result<AppCredentials, String> {
    let Some(key) = non_empty(key) else {
        return Err(missing_credentials(spec));
    };
    let issuer = read_issuer(spec, client_id, app_id)?;
    let key = key.as_str();
    if key.len() > PRIVATE_KEY_MAX_BYTES {
        return Err(format!(
            "{} is larger than {PRIVATE_KEY_MAX_BYTES} bytes",
            spec.key_variable
        ));
    }
    let Ok(key) = EncodingKey::from_rsa_pem(key.as_bytes()) else {
        return Err(format!(
            "{} is not an RSA private key in PEM form",
            spec.key_variable
        ));
    };
    Ok(AppCredentials { spec, issuer, key })
}

/// `owner/name` from the Actions environment, bounded and limited to GitHub's name characters:
/// an owner is letters, digits and `-`; a name adds `_` and `.` but is never `.` or `..`.
pub fn read_repository(value: Option<String>) -> Result<Repository, String> {
    let value = value.unwrap_or_default();
    let valid_owner = |owner: &str| {
        !owner.is_empty() && owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    };
    let valid_name = |name: &str| {
        !name.is_empty()
            && name != "."
            && name != ".."
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    match value.split_once('/') {
        Some((owner, name))
            if value.len() <= REPOSITORY_MAX_CHARS && valid_owner(owner) && valid_name(name) =>
        {
            Ok(Repository {
                owner: owner.to_string(),
                name: name.to_string(),
            })
        }
        _ => Err(format!(
            "{REPOSITORY_VARIABLE} must be `owner/name`; Actions sets it for every job"
        )),
    }
}

fn unix_now() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(|_| "the system clock is before 1970".to_string())
}

/// The App JWT: `iat` backdated, `exp` exactly [`JWT_MAX_LIFETIME_SECONDS`] after `iat`, and
/// `iss` the client id, or the App id as a string, the JSON web token `StringOrURI` form.
fn app_jwt(credentials: &AppCredentials, now: u64) -> Result<String, String> {
    let issued = now
        .checked_sub(JWT_BACKDATE_SECONDS)
        .ok_or("the system clock is before 1970")?;
    let expires = issued
        .checked_add(JWT_MAX_LIFETIME_SECONDS)
        .ok_or("the JWT expiry overflows")?;
    let claims = serde_json::json!({
        "iat": issued,
        "exp": expires,
        "iss": credentials.issuer,
    });
    jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &credentials.key).map_err(|_| {
        format!(
            "{} could not sign the App JWT; it is not a usable RSA key",
            credentials.spec.key_variable
        )
    })
}

/// Reduces a reply to a status and body, or to a fixed reason it could not be read.
fn response(reply: Reply) -> Result<(u16, String), &'static str> {
    match reply {
        Reply::Response { status, body } => Ok((status, body)),
        Reply::BodyTooLarge => Err("the response is over its size limit"),
        Reply::Failed(reason) => Err(reason),
    }
}

const fn succeeded(status: u16) -> bool {
    status >= 200 && status < 300
}

fn decode_installation(status: u16, body: &str) -> InstallationOutcome {
    if !succeeded(status) {
        return InstallationOutcome::Failed(status, github_message(body));
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body.trim()) else {
        return InstallationOutcome::Unrecognised("installation response is not JSON");
    };
    match value.get("id").and_then(serde_json::Value::as_u64) {
        Some(id) if id > 0 => InstallationOutcome::Found(id),
        _ => InstallationOutcome::Unrecognised("installation response has no numeric `id`"),
    }
}

/// The granted permissions must be the requested ones plus GitHub's implicit metadata read.
fn grant_is_exact(
    spec: &AppSpec,
    permissions: &serde_json::Map<String, serde_json::Value>,
) -> bool {
    let requested_present = spec.permissions.iter().all(|(name, level)| {
        permissions.get(*name).and_then(serde_json::Value::as_str) == Some(level)
    });
    let nothing_else = permissions.iter().all(|(name, level)| {
        let level = level.as_str();
        spec.permissions
            .iter()
            .chain(std::iter::once(&IMPLICIT_PERMISSION))
            .any(|(allowed, allowed_level)| name == allowed && level == Some(allowed_level))
    });
    requested_present && nothing_else
}

fn single_repository(value: &serde_json::Value, repository: &Repository) -> bool {
    let names: Option<Vec<&str>> = value
        .get("repositories")
        .and_then(serde_json::Value::as_array)
        .and_then(|list| {
            list.iter()
                .map(|entry| entry.get("name").and_then(serde_json::Value::as_str))
                .collect()
        });
    let selected = value
        .get("repository_selection")
        .and_then(serde_json::Value::as_str)
        == Some("selected");
    selected && names.is_some_and(|names| names == [repository.name.as_str()])
}

fn decode_access_token(
    spec: &AppSpec,
    status: u16,
    body: &str,
    repository: &Repository,
) -> TokenOutcome {
    if !succeeded(status) {
        return TokenOutcome::Failed(status, github_message(body));
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body.trim()) else {
        return TokenOutcome::Unrecognised("token response is not JSON");
    };
    let token = match value.get("token").and_then(serde_json::Value::as_str) {
        Some(token)
            if !token.is_empty()
                && token.len() <= TOKEN_MAX_CHARS
                && token.chars().all(|c| c.is_ascii_graphic()) =>
        {
            token
        }
        _ => return TokenOutcome::Unrecognised("token response has no usable `token`"),
    };
    let token = InstallationToken(token.to_string());
    let Some(permissions) = value
        .get("permissions")
        .and_then(serde_json::Value::as_object)
    else {
        return TokenOutcome::UnreadableGrant(token);
    };
    if !grant_is_exact(spec, permissions) || !single_repository(&value, repository) {
        return TokenOutcome::Overgranted(token);
    }
    TokenOutcome::Issued(token)
}

/// The request body that down-scopes the token to this repository and the App's grant.
fn access_token_body(spec: &AppSpec, repository: &Repository) -> String {
    let permissions: serde_json::Map<String, serde_json::Value> = spec
        .permissions
        .iter()
        .map(|(name, level)| ((*name).to_string(), serde_json::Value::from(*level)))
        .collect();
    serde_json::json!({
        "repositories": [repository.name],
        "permissions": permissions,
    })
    .to_string()
}

fn with_message(status: u16, message: Option<&String>) -> String {
    message.map_or_else(
        || format!("HTTP {status}"),
        |message| format!("HTTP {status}, {message}"),
    )
}

fn describe_token_failure(outcome: &TokenOutcome) -> String {
    match outcome {
        TokenOutcome::Issued(_) => "token issued".to_string(),
        TokenOutcome::Failed(status, message) => format!(
            "the installation token request ended {}",
            with_message(*status, message.as_ref())
        ),
        TokenOutcome::Overgranted(_) => "the issued token's grant is not exactly the requested \
                                         one on this repository; refusing it"
            .to_string(),
        TokenOutcome::UnreadableGrant(_) => {
            "the token response has no `permissions` object; refusing the token".to_string()
        }
        TokenOutcome::Unrecognised(reason) => reason.to_string(),
    }
}

/// Mints an installation token for `repository`, down-scoped to the requested permissions.
pub fn mint(
    credentials: &AppCredentials,
    repository: &Repository,
) -> Result<InstallationToken, String> {
    mint_at(&Client::github(), credentials, repository, unix_now()?)
}

fn mint_at(
    client: &Client,
    credentials: &AppCredentials,
    repository: &Repository,
    now: u64,
) -> Result<InstallationToken, String> {
    let jwt = app_jwt(credentials, now)?;
    let lookup_path = format!(
        "/repos/{}/{}/installation",
        repository.owner, repository.name
    );
    let (status, body) = response(client.request(Method::Get, &lookup_path, &jwt, None))
        .map_err(|reason| format!("the installation lookup failed: {reason}"))?;
    let installation = match decode_installation(status, &body) {
        InstallationOutcome::Found(id) => id,
        InstallationOutcome::Failed(status, message) => {
            return Err(format!(
                "the installation lookup for {}/{} ended {}; the App must be installed on this \
                 repository and the key must belong to the App the id names",
                repository.owner,
                repository.name,
                with_message(status, message.as_ref())
            ));
        }
        InstallationOutcome::Unrecognised(reason) => return Err(reason.to_string()),
    };
    let token_path = format!("/app/installations/{installation}/access_tokens");
    let request_body = access_token_body(credentials.spec, repository);
    let (status, body) =
        response(client.request(Method::Post, &token_path, &jwt, Some(&request_body)))
            .map_err(|reason| format!("the installation token request failed: {reason}"))?;
    let outcome = decode_access_token(credentials.spec, status, &body, repository);
    let reason = describe_token_failure(&outcome);
    match outcome {
        TokenOutcome::Issued(token) => Ok(token),
        TokenOutcome::Overgranted(token) | TokenOutcome::UnreadableGrant(token) => {
            Err(match revoke_at(client, &token) {
                Ok(()) => format!("{reason}; the token is revoked"),
                Err(revocation) => format!("{reason}; {revocation}"),
            })
        }
        TokenOutcome::Failed(..) | TokenOutcome::Unrecognised(_) => Err(reason),
    }
}

/// The login the App posts as, `<slug>[bot]`, from `GET /app` with the App JWT.
pub fn bot_login(credentials: &AppCredentials) -> Result<String, String> {
    bot_login_at(&Client::github(), credentials, unix_now()?)
}

fn bot_login_at(client: &Client, credentials: &AppCredentials, now: u64) -> Result<String, String> {
    let jwt = app_jwt(credentials, now)?;
    let (status, body) = response(client.request(Method::Get, "/app", &jwt, None))
        .map_err(|reason| format!("the App lookup failed: {reason}"))?;
    if !succeeded(status) {
        return Err(format!(
            "the App lookup ended {}",
            with_message(status, github_message(&body).as_ref())
        ));
    }
    let slug = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| value.get("slug")?.as_str().map(str::to_string))
        .filter(|slug| {
            !slug.is_empty()
                && slug.len() <= SLUG_MAX_CHARS
                && slug
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
        .ok_or("the App response has no usable `slug`")?;
    Ok(format!("{slug}[bot]"))
}

/// Revokes the token so it does not outlive the job that minted it.
pub fn revoke(token: &InstallationToken) -> Result<(), String> {
    revoke_at(&Client::github(), token)
}

fn revoke_at(client: &Client, token: &InstallationToken) -> Result<(), String> {
    let revoked = client.request(Method::Delete, "/installation/token", token.expose(), None);
    match response(revoked) {
        Ok((status, _)) if succeeded(status) => Ok(()),
        Ok((status, body)) => Err(format!(
            "revoking the installation token ended {}",
            with_message(status, github_message(&body).as_ref())
        )),
        Err(reason) => Err(format!("revoking the installation token failed: {reason}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_lc_rs::encoding::AsDer;
    use aws_lc_rs::rsa::{KeyPair, KeySize};
    use aws_lc_rs::signature::KeyPair as _;
    use jsonwebtoken::{DecodingKey, Validation};

    const NOW: u64 = 1_800_000_000;
    const TOKEN: &str = "ghs_16C7e42F292c6912E7710c838347Ae178B4a";

    /// A fresh key per run, in the `RSA PRIVATE KEY` form GitHub downloads, and its public half.
    fn test_key() -> (String, String) {
        let (pkcs1, _, public) = test_key_forms();
        (pkcs1, public)
    }

    /// One fresh key as `RSA PRIVATE KEY` (the form GitHub downloads), as `PRIVATE KEY`, both
    /// with LF line ends as downloaded, and its public half.
    fn test_key_forms() -> (String, String, String) {
        let pair = KeyPair::generate(KeySize::Rsa2048).unwrap();
        let pkcs8 = pem::encode(&pem::Pem::new(
            "PRIVATE KEY",
            pair.as_der().unwrap().as_ref(),
        ));
        let pkcs1_der = EncodingKey::from_rsa_pem(pkcs8.as_bytes()).unwrap();
        let pkcs1 = pem::encode(&pem::Pem::new("RSA PRIVATE KEY", pkcs1_der.as_bytes()));
        let public_der = pair.public_key().as_der().unwrap();
        let public = pem::encode(&pem::Pem::new("PUBLIC KEY", public_der.as_ref()));
        let lf = |text: String| text.replace("\r\n", "\n");
        (lf(pkcs1), lf(pkcs8), public)
    }

    fn repository() -> Repository {
        read_repository(Some("frits-v/stencil".into())).unwrap()
    }

    fn token_response(permissions: &str, repositories: &str) -> String {
        format!(
            r#"{{"token":"{TOKEN}","expires_at":"2026-10-09T12:00:00Z","permissions":{permissions},"repository_selection":"selected","repositories":{repositories}}}"#
        )
    }

    #[test]
    fn app_jwt_header_and_claims_decode_and_verify_against_the_public_key() {
        let (private, public) = test_key();
        assert!(private.starts_with("-----BEGIN RSA PRIVATE KEY-----"));
        let credentials =
            read_credentials(&REVIEW, None, Some("123456".into()), Some(private)).unwrap();
        let jwt = app_jwt(&credentials, NOW).unwrap();
        let header = jsonwebtoken::decode_header(&jwt).unwrap();
        assert_eq!(header.alg, Algorithm::RS256);
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = false;
        validation.set_required_spec_claims(&["iat", "exp", "iss"]);
        let decoded = jsonwebtoken::decode::<serde_json::Value>(
            &jwt,
            &DecodingKey::from_rsa_pem(public.as_bytes()).unwrap(),
            &validation,
        )
        .unwrap();
        let claim = |name: &str| decoded.claims.get(name).and_then(serde_json::Value::as_u64);
        assert_eq!(
            decoded
                .claims
                .get("iss")
                .and_then(serde_json::Value::as_str),
            Some("123456")
        );
        assert_eq!(claim("iat"), Some(NOW.saturating_sub(JWT_BACKDATE_SECONDS)));
        let expires = claim("exp").unwrap();
        let lifetime = expires.saturating_sub(claim("iat").unwrap());
        assert!(lifetime <= JWT_MAX_LIFETIME_SECONDS, "{lifetime}");
        assert!(expires > NOW && expires.saturating_sub(NOW) < JWT_MAX_LIFETIME_SECONDS);
    }

    #[test]
    fn app_jwt_does_not_verify_against_another_key() {
        let (private, _) = test_key();
        let (_, other_public) = test_key();
        let credentials = read_credentials(&REVIEW, None, Some("7".into()), Some(private)).unwrap();
        let jwt = app_jwt(&credentials, NOW).unwrap();
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = false;
        let verified = jsonwebtoken::decode::<serde_json::Value>(
            &jwt,
            &DecodingKey::from_rsa_pem(other_public.as_bytes()).unwrap(),
            &validation,
        );
        assert!(verified.is_err());
    }

    #[test]
    fn missing_credentials_name_both_and_the_setup_document() {
        let (private, _) = test_key();
        for (app_id, key) in [
            (None, None),
            (Some("1".to_string()), None),
            (None, Some(private.clone())),
            (Some("  ".to_string()), Some(private.clone())),
            (Some("1".to_string()), Some(" ".to_string())),
        ] {
            let error = read_credentials(&REVIEW, None, app_id, key).unwrap_err();
            assert!(error.contains(REVIEW.client_id_variable), "{error}");
            assert!(error.contains("preferred"), "{error}");
            assert!(error.contains(REVIEW.app_id_variable), "{error}");
            assert!(error.contains(REVIEW.key_variable), "{error}");
            assert!(error.contains("docs/review.md"), "{error}");
        }
        assert!(read_credentials(&REVIEW, None, Some("1".into()), Some(private)).is_ok());
    }

    fn signed_issuer(credentials: &AppCredentials, public: &str) -> Option<String> {
        let jwt = app_jwt(credentials, NOW).unwrap();
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = false;
        let decoded = jsonwebtoken::decode::<serde_json::Value>(
            &jwt,
            &DecodingKey::from_rsa_pem(public.as_bytes()).unwrap(),
            &validation,
        )
        .unwrap();
        decoded
            .claims
            .get("iss")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    }

    #[test]
    fn the_client_id_is_the_issuer_when_set_and_the_app_id_is_the_fallback() {
        let (private, public) = test_key();
        let both = read_credentials(
            &REVIEW,
            Some(" Iv23liAbC123 ".into()),
            Some("123456".into()),
            Some(private.clone()),
        )
        .unwrap();
        assert_eq!(
            signed_issuer(&both, &public).as_deref(),
            Some("Iv23liAbC123")
        );
        let client_only = read_credentials(
            &REVIEW,
            Some("Iv23liAbC123".into()),
            None,
            Some(private.clone()),
        )
        .unwrap();
        assert_eq!(
            signed_issuer(&client_only, &public).as_deref(),
            Some("Iv23liAbC123")
        );
        for empty_client_id in [None, Some(String::new()), Some("  ".to_string())] {
            let fallback = read_credentials(
                &REVIEW,
                empty_client_id,
                Some("123456".into()),
                Some(private.clone()),
            )
            .unwrap();
            assert_eq!(signed_issuer(&fallback, &public).as_deref(), Some("123456"));
        }
    }

    #[test]
    fn a_malformed_client_id_is_refused_without_falling_back() {
        let (private, _) = test_key();
        let too_long = "a".repeat(CLIENT_ID_MAX_CHARS.saturating_add(1));
        for client_id in [
            "Iv23-li",
            "Iv23 li",
            "Iv23li\u{e9}",
            "Iv23li\"x",
            "Iv.23",
            too_long.as_str(),
        ] {
            let error = read_credentials(
                &REVIEW,
                Some(client_id.into()),
                Some("123456".into()),
                Some(private.clone()),
            )
            .unwrap_err();
            assert!(
                error.contains(REVIEW.client_id_variable),
                "{client_id}: {error}"
            );
            assert!(!error.contains(client_id), "{error}");
        }
        let longest = "a".repeat(CLIENT_ID_MAX_CHARS);
        assert!(read_credentials(&REVIEW, Some(longest), None, Some(private)).is_ok());
    }

    #[test]
    fn refuses_a_key_that_is_not_pem_and_an_app_id_that_is_not_a_number() {
        let not_pem = "MIIEowIBAAKCAQEA-not-a-pem-key";
        let error =
            read_credentials(&REVIEW, None, Some("1".into()), Some(not_pem.into())).unwrap_err();
        assert!(error.contains("PEM"), "{error}");
        assert!(!error.contains(not_pem));
        let oversized = "x".repeat(PRIVATE_KEY_MAX_BYTES.saturating_add(1));
        assert!(read_credentials(&REVIEW, None, Some("1".into()), Some(oversized)).is_err());
        let (private, _) = test_key();
        for app_id in ["abc", "0", "-1", "123456789012345678901"] {
            let refused =
                read_credentials(&REVIEW, None, Some(app_id.into()), Some(private.clone()));
            assert!(refused.is_err(), "{app_id}");
        }
    }

    #[test]
    fn repository_must_be_owner_and_name() {
        assert_eq!(
            repository(),
            Repository {
                owner: "frits-v".into(),
                name: "stencil".into()
            }
        );
        for bad in [
            "", "stencil", "/x", "x/", "a/b c", "a/b?x=1", "a/..", "a/.", "../x", "./x", "a.b/x",
            "a_b/x", "a/b/c",
        ] {
            assert!(read_repository(Some(bad.into())).is_err(), "{bad}");
        }
        assert!(read_repository(None).is_err());
    }

    #[test]
    fn installation_decoder_covers_each_state() {
        assert_eq!(
            decode_installation(200, r#"{"id":42,"app_id":7}"#),
            InstallationOutcome::Found(42)
        );
        assert_eq!(
            decode_installation(404, r#"{"message":"Not Found"}"#),
            InstallationOutcome::Failed(404, Some("Not Found".into()))
        );
        for unrecognised in ["", "not json", "{}", r#"{"id":"42"}"#, r#"{"id":0}"#] {
            assert!(
                matches!(
                    decode_installation(200, unrecognised),
                    InstallationOutcome::Unrecognised(_)
                ),
                "{unrecognised}"
            );
        }
    }

    #[test]
    fn token_decoder_accepts_only_the_requested_grant() {
        let exact =
            r#"{"checks":"read","pull_requests":"write","statuses":"read","metadata":"read"}"#;
        let one_repository = r#"[{"name":"stencil"}]"#;
        assert_eq!(
            decode_access_token(
                &REVIEW,
                201,
                &token_response(exact, one_repository),
                &repository()
            ),
            TokenOutcome::Issued(InstallationToken(TOKEN.into()))
        );
        let wider = r#"{"checks":"read","pull_requests":"write","statuses":"read","metadata":"read","workflows":"write"}"#;
        let narrower = r#"{"checks":"read","metadata":"read"}"#;
        let admin =
            r#"{"checks":"read","pull_requests":"write","statuses":"read","metadata":"write"}"#;
        let two_repositories = r#"[{"name":"stencil"},{"name":"other"}]"#;
        let unnamed_entry = r#"[{"name":"stencil"},{}]"#;
        for (permissions, repositories) in [
            (wider, one_repository),
            (narrower, one_repository),
            (admin, one_repository),
            (exact, two_repositories),
            (exact, unnamed_entry),
            (exact, "[]"),
        ] {
            assert_eq!(
                decode_access_token(
                    &REVIEW,
                    201,
                    &token_response(permissions, repositories),
                    &repository()
                ),
                TokenOutcome::Overgranted(InstallationToken(TOKEN.into())),
                "{permissions} {repositories}"
            );
        }
        let all = token_response(exact, one_repository).replace("selected", "all");
        assert_eq!(
            decode_access_token(&REVIEW, 201, &all, &repository()),
            TokenOutcome::Overgranted(InstallationToken(TOKEN.into()))
        );
    }

    #[test]
    fn a_refused_token_travels_in_its_variant_for_revocation() {
        let without_permissions = format!(
            r#"{{"token":"{TOKEN}","repository_selection":"selected","repositories":[{{"name":"stencil"}}]}}"#
        );
        assert_eq!(
            decode_access_token(&REVIEW, 201, &without_permissions, &repository()),
            TokenOutcome::UnreadableGrant(InstallationToken(TOKEN.into()))
        );
        let permissions_not_an_object = format!(r#"{{"token":"{TOKEN}","permissions":"all"}}"#);
        let refused = decode_access_token(&REVIEW, 201, &permissions_not_an_object, &repository());
        assert!(
            matches!(&refused, TokenOutcome::UnreadableGrant(token) if token.expose() == TOKEN),
            "{refused:?}"
        );
        let wider = token_response(
            r#"{"checks":"read","pull_requests":"write","statuses":"read","metadata":"read","administration":"write"}"#,
            r#"[{"name":"stencil"}]"#,
        );
        let refused = decode_access_token(&REVIEW, 201, &wider, &repository());
        assert!(
            matches!(&refused, TokenOutcome::Overgranted(token) if token.expose() == TOKEN),
            "{refused:?}"
        );
        assert!(!describe_token_failure(&refused).contains(TOKEN));
    }

    #[test]
    fn token_decoder_reports_failures_without_the_token_or_key_material() {
        let (private, _) = test_key();
        let key_line = private.lines().nth(1).unwrap().to_string();
        let leaky_failure = format!(
            r#"{{"message":"Bad credentials {TOKEN} {key_line}","documentation_url":"x"}}"#
        );
        let malformed_success = format!(r#"{{"token":"{TOKEN}","permissions":"all"}}"#);
        let bad_token = r#"{"token":"has space","permissions":{}}"#;
        let cases = [
            (401, leaky_failure.as_str()),
            (500, leaky_failure.as_str()),
            (201, malformed_success.as_str()),
            (201, bad_token),
            (201, "not json"),
        ];
        for (status, stdout) in cases {
            let outcome = decode_access_token(&REVIEW, status, stdout, &repository());
            assert!(!matches!(outcome, TokenOutcome::Issued(_)), "{stdout}");
            let shown = format!("{} {outcome:?}", describe_token_failure(&outcome));
            assert!(!shown.contains(TOKEN), "{shown}");
            assert!(!shown.contains(&key_line), "{shown}");
            assert!(!shown.contains("BEGIN"), "{shown}");
        }
        let issued = InstallationToken(TOKEN.into());
        assert!(!format!("{issued:?}").contains(TOKEN));
        let credentials = read_credentials(&REVIEW, None, Some("1".into()), Some(private)).unwrap();
        assert!(!format!("{credentials:?}").contains(&key_line));
    }

    #[test]
    fn token_request_scopes_to_one_repository_and_two_permissions() {
        let body: serde_json::Value =
            serde_json::from_str(&access_token_body(&REVIEW, &repository())).unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "repositories": ["stencil"],
                "permissions": {"checks": "read", "pull_requests": "write", "statuses": "read"},
            })
        );
    }

    #[test]
    fn both_private_key_forms_sign_with_or_without_surrounding_newlines() {
        let (pkcs1, pkcs8, public) = test_key_forms();
        assert!(pkcs1.starts_with("-----BEGIN RSA PRIVATE KEY-----\n"));
        assert!(pkcs8.starts_with("-----BEGIN PRIVATE KEY-----\n"));
        for key in [
            pkcs1.clone(),
            pkcs8.clone(),
            pkcs1.replace('\n', "\r\n"),
            pkcs8.replace('\n', "\r\n"),
            format!("\n{pkcs1}\n\n"),
            format!("\r\n{pkcs8}"),
        ] {
            let credentials =
                read_credentials(&REVIEW, Some("Iv23liAbC123".into()), None, Some(key)).unwrap();
            assert_eq!(
                signed_issuer(&credentials, &public).as_deref(),
                Some("Iv23liAbC123")
            );
        }
    }

    fn bearer_of(raw: &str) -> String {
        raw.lines()
            .find_map(|line| {
                let (name, value) = line.split_once(": ")?;
                name.eq_ignore_ascii_case("authorization")
                    .then(|| value.strip_prefix("Bearer ").unwrap().to_string())
            })
            .unwrap()
    }

    fn client_credentials() -> (AppCredentials, String) {
        let (private, public) = test_key();
        let credentials =
            read_credentials(&REVIEW, Some("Iv23liAbC123".into()), None, Some(private)).unwrap();
        (credentials, public)
    }

    #[test]
    fn mint_sends_the_jwt_as_a_bearer_token_and_scopes_the_request() {
        let (credentials, public) = client_credentials();
        let exact =
            r#"{"checks":"read","pull_requests":"write","statuses":"read","metadata":"read"}"#;
        let (base, seen) = crate::github_api::tests::listener(vec![
            (200, r#"{"id":42}"#.to_string()),
            (201, token_response(exact, r#"[{"name":"stencil"}]"#)),
        ]);
        let token = mint_at(&Client::loopback(&base), &credentials, &repository(), NOW).unwrap();
        assert_eq!(token.expose(), TOKEN);
        let requests = seen.recv().unwrap();
        let [lookup, issue] = requests.as_slice() else {
            panic!("{requests:?}");
        };
        assert!(lookup.starts_with("GET /repos/frits-v/stencil/installation HTTP/1.1\r\n"));
        assert!(issue.starts_with("POST /app/installations/42/access_tokens HTTP/1.1\r\n"));
        let jwt = bearer_of(lookup);
        assert_eq!(bearer_of(issue), jwt);
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = false;
        let claims = jsonwebtoken::decode::<serde_json::Value>(
            &jwt,
            &DecodingKey::from_rsa_pem(public.as_bytes()).unwrap(),
            &validation,
        )
        .unwrap()
        .claims;
        assert_eq!(
            claims.get("iss").and_then(serde_json::Value::as_str),
            Some("Iv23liAbC123")
        );
        for raw in [lookup, issue] {
            assert_eq!(raw.matches(jwt.as_str()).count(), 1, "{raw}");
        }
        let (_, body) = issue.split_once("\r\n\r\n").unwrap();
        let body: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "repositories": ["stencil"],
                "permissions": {"checks": "read", "pull_requests": "write", "statuses": "read"},
            })
        );
    }

    #[test]
    fn the_review_app_asks_for_its_own_grant_and_refuses_a_wider_one() {
        let (private, _) = test_key();
        let credentials =
            read_credentials(&REVIEW, Some("Iv23liReview".into()), None, Some(private)).unwrap();
        let exact =
            r#"{"checks":"read","pull_requests":"write","statuses":"read","metadata":"read"}"#;
        let wider_grant = r#"{"contents":"write","pull_requests":"write","metadata":"read"}"#;
        let one = r#"[{"name":"stencil"}]"#;
        let (base, seen) = crate::github_api::tests::listener(vec![
            (200, r#"{"id":42}"#.to_string()),
            (201, token_response(exact, one)),
            (200, r#"{"id":42}"#.to_string()),
            (201, token_response(wider_grant, one)),
            (204, String::new()),
        ]);
        let client = Client::loopback(&base);
        assert!(mint_at(&client, &credentials, &repository(), NOW).is_ok());
        let refused = mint_at(&client, &credentials, &repository(), NOW).unwrap_err();
        assert!(refused.contains("the token is revoked"), "{refused}");
        let requests = seen.recv().unwrap();
        let (_, body) = requests.get(1).unwrap().split_once("\r\n\r\n").unwrap();
        let body: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(
            body.get("permissions"),
            Some(
                &serde_json::json!({"checks": "read", "pull_requests": "write", "statuses": "read"})
            )
        );
        let missing = read_credentials(&REVIEW, None, None, None).unwrap_err();
        assert!(
            missing.contains("FRITS_CODE_REVIEW_APP_PRIVATE_KEY"),
            "{missing}"
        );
        assert!(missing.contains("docs/review.md"), "{missing}");
    }

    #[test]
    fn the_bot_login_is_the_app_slug_and_a_bad_slug_is_refused() {
        let (credentials, _) = client_credentials();
        let (base, seen) = crate::github_api::tests::listener(vec![
            (200, r#"{"id":1,"slug":"frits-code-review"}"#.to_string()),
            (200, r#"{"id":1,"slug":"Bad Slug"}"#.to_string()),
            (
                401,
                r#"{"message":"A JSON web token could not be decoded"}"#.to_string(),
            ),
        ]);
        let client = Client::loopback(&base);
        assert_eq!(
            bot_login_at(&client, &credentials, NOW),
            Ok("frits-code-review[bot]".to_string())
        );
        assert!(bot_login_at(&client, &credentials, NOW).is_err());
        let refused = bot_login_at(&client, &credentials, NOW).unwrap_err();
        assert!(refused.contains("HTTP 401"), "{refused}");
        let requests = seen.recv().unwrap();
        assert!(
            requests
                .first()
                .unwrap()
                .starts_with("GET /app HTTP/1.1\r\n")
        );
    }

    #[test]
    fn mint_revokes_a_wider_token_with_the_token_as_bearer() {
        let (credentials, _) = client_credentials();
        let wider = r#"{"checks":"read","pull_requests":"write","statuses":"read","metadata":"read","workflows":"write"}"#;
        let (base, seen) = crate::github_api::tests::listener(vec![
            (200, r#"{"id":42}"#.to_string()),
            (201, token_response(wider, r#"[{"name":"stencil"}]"#)),
            (204, String::new()),
        ]);
        let error =
            mint_at(&Client::loopback(&base), &credentials, &repository(), NOW).unwrap_err();
        assert!(
            error.contains("refusing it; the token is revoked"),
            "{error}"
        );
        assert!(!error.contains(TOKEN), "{error}");
        let requests = seen.recv().unwrap();
        let revocation = requests.get(2).unwrap();
        assert!(
            revocation.starts_with("DELETE /installation/token HTTP/1.1\r\n"),
            "{revocation}"
        );
        assert_eq!(bearer_of(revocation), TOKEN);
    }

    #[test]
    fn a_rejected_lookup_reports_githubs_message_and_no_credential() {
        let (credentials, _) = client_credentials();
        let (base, seen) = crate::github_api::tests::listener(vec![(
            401,
            r#"{"message":"A JSON web token could not be decoded","status":"401"}"#.to_string(),
        )]);
        let error =
            mint_at(&Client::loopback(&base), &credentials, &repository(), NOW).unwrap_err();
        let jwt = bearer_of(seen.recv().unwrap().first().unwrap());
        assert!(
            error.contains("ended HTTP 401, A JSON web token could not be decoded"),
            "{error}"
        );
        assert!(!error.contains(&jwt), "{error}");
        assert!(!error.contains("eyJ"), "{error}");
    }
}
