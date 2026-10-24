//! Anthropic access tokens for `xtask review`, by workload identity federation.
//!
//! The job's GitHub OIDC token, requested for the Anthropic audience, is traded at Anthropic's
//! token endpoint with a JWT bearer grant for a short-lived access token. Both calls are made in
//! process (`github_api::PlainClient`), so neither token reaches a child's arguments, output or
//! error text; the access token reaches the Claude CLI only as `ANTHROPIC_AUTH_TOKEN`.

use std::fmt;
use std::sync::{Mutex, PoisonError};

use base64::Engine as _;

use crate::github_api::{Credential, Method, PlainClient, Reply};

pub const FEDERATION_RULE_VARIABLE: &str = "ANTHROPIC_FEDERATION_RULE_ID";
pub const ORGANIZATION_VARIABLE: &str = "ANTHROPIC_ORGANIZATION_ID";
pub const SERVICE_ACCOUNT_VARIABLE: &str = "ANTHROPIC_SERVICE_ACCOUNT_ID";
pub const WORKSPACE_VARIABLE: &str = "ANTHROPIC_WORKSPACE_ID";
pub const REQUEST_URL_VARIABLE: &str = "ACTIONS_ID_TOKEN_REQUEST_URL";
pub const REQUEST_TOKEN_VARIABLE: &str = "ACTIONS_ID_TOKEN_REQUEST_TOKEN";
/// The `aud` claim the federation rule matches; requested from GitHub URL-encoded.
const AUDIENCE_ENCODED: &str = "https%3A%2F%2Fapi.anthropic.com";
const TOKEN_ENDPOINT: &str = "https://api.anthropic.com/v1/oauth/token";
const JWT_BEARER_GRANT: &str = "urn:ietf:params:oauth:grant-type:jwt-bearer";
const IDENTIFIER_MAX_CHARS: usize = 128;
const REQUEST_URL_MAX_CHARS: usize = 2048;
const TOKEN_MAX_CHARS: usize = 8192;
const ERROR_CODE_MAX_CHARS: usize = 64;
/// The claims a federation rule matches on, shown before each exchange.
const RULE_CLAIMS: [&str; 4] = ["sub", "environment", "job_workflow_ref", "ref"];
const CLAIM_SHOWN_MAX_CHARS: usize = 200;
/// A day; a longer lifetime is not one this exchange issues.
const EXPIRES_IN_MAX_SECONDS: u64 = 86_400;

/// A token value; it leaves this module only through [`Secret::expose`].
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret([redacted])")
    }
}

/// The federation rule and the identities it issues tokens for. The service account and the
/// workspace are sent only when set.
#[derive(Debug, PartialEq, Eq)]
pub struct Federation {
    rule: String,
    organization: String,
    service_account: Option<String>,
    workspace: Option<String>,
}

/// What a run mints Anthropic tokens from. Each mint asks GitHub for a fresh OIDC token, so
/// tokens minted side by side never present the same single-use assertion. The minter keeps
/// every token it issued, so the run can refuse to post one whatever its format.
#[derive(Debug)]
pub struct Minter {
    federation: Federation,
    oidc: OidcRequest,
    minted: Mutex<Vec<Secret>>,
}

impl Minter {
    pub const fn new(federation: Federation, oidc: OidcRequest) -> Self {
        Self {
            federation,
            oidc,
            minted: Mutex::new(Vec::new()),
        }
    }

    pub fn mint(&self) -> Result<AccessToken, String> {
        let token = mint(&self.federation, &self.oidc)?;
        self.minted
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(token.value.clone());
        Ok(token)
    }

    /// Every token this minter issued, for the check that a posted text names none.
    pub fn minted(&self) -> Vec<Secret> {
        self.minted
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    #[cfg(test)]
    pub fn with_minted(tokens: &[&str]) -> Self {
        let minter = Self::new(
            read_federation(Some("r".into()), Some("o".into()), None, None)
                .unwrap_or_else(|reason| panic!("{reason}")),
            read_oidc_request(Some("https://x/".into()), Some("t".into()))
                .unwrap_or_else(|reason| panic!("{reason}")),
        );
        minter
            .minted
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend(tokens.iter().map(|token| Secret((*token).to_string())));
        minter
    }
}

/// Where the job asks GitHub for its OIDC token, and the bearer token that request carries.
#[derive(Debug)]
pub struct OidcRequest {
    url: String,
    token: Secret,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AccessToken {
    pub value: Secret,
    pub expires_in_seconds: u64,
}

#[derive(Debug, PartialEq, Eq)]
enum OidcOutcome {
    Issued(Secret),
    Failed(u16),
    Unrecognised(&'static str),
}

/// `Unrecognised` carries a fixed reason, never response text: a malformed success body may
/// hold the token.
#[derive(Debug, PartialEq, Eq)]
enum ExchangeOutcome {
    Issued(AccessToken),
    Failed(u16, Option<String>),
    Unrecognised(&'static str),
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn identifier(name: &str, value: Option<String>) -> Result<Option<String>, String> {
    let Some(value) = non_empty(value) else {
        return Ok(None);
    };
    let valid = value.len() <= IDENTIFIER_MAX_CHARS
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if valid {
        Ok(Some(value))
    } else {
        Err(format!(
            "{name} is not an identifier: letters, digits, `_` and `-`, at most \
             {IDENTIFIER_MAX_CHARS}"
        ))
    }
}

/// Reads the federation identifiers; the rule and the organization are required.
pub fn read_federation(
    rule: Option<String>,
    organization: Option<String>,
    service_account: Option<String>,
    workspace: Option<String>,
) -> Result<Federation, String> {
    let rule = identifier(FEDERATION_RULE_VARIABLE, rule)?;
    let organization = identifier(ORGANIZATION_VARIABLE, organization)?;
    let (Some(rule), Some(organization)) = (rule, organization) else {
        return Err(format!(
            "{FEDERATION_RULE_VARIABLE} and {ORGANIZATION_VARIABLE} must be set, with \
             {SERVICE_ACCOUNT_VARIABLE} and {WORKSPACE_VARIABLE} when the rule needs them: the \
             review authenticates by workload identity federation (docs/review.md)"
        ));
    };
    Ok(Federation {
        rule,
        organization,
        service_account: identifier(SERVICE_ACCOUNT_VARIABLE, service_account)?,
        workspace: identifier(WORKSPACE_VARIABLE, workspace)?,
    })
}

/// Reads the Actions OIDC request; both values are set only in a job with `id-token: write`.
pub fn read_oidc_request(
    url: Option<String>,
    token: Option<String>,
) -> Result<OidcRequest, String> {
    let (Some(url), Some(token)) = (non_empty(url), non_empty(token)) else {
        return Err(format!(
            "{REQUEST_URL_VARIABLE} and {REQUEST_TOKEN_VARIABLE} are unset: the job needs \
             `id-token: write`"
        ));
    };
    let url_valid = url.starts_with("https://")
        && url.len() <= REQUEST_URL_MAX_CHARS
        && url.chars().all(|c| c.is_ascii_graphic())
        && !url.contains('#');
    if !url_valid {
        return Err(format!("{REQUEST_URL_VARIABLE} is not an HTTPS URL"));
    }
    if token.len() > TOKEN_MAX_CHARS || !token.chars().all(|c| c.is_ascii_graphic()) {
        return Err(format!("{REQUEST_TOKEN_VARIABLE} is not a token"));
    }
    Ok(OidcRequest {
        url,
        token: Secret(token),
    })
}

fn audience_url(request_url: &str) -> String {
    let separator = if request_url.contains('?') { '&' } else { '?' };
    format!("{request_url}{separator}audience={AUDIENCE_ENCODED}")
}

fn usable_token(value: Option<&str>) -> Option<Secret> {
    value
        .filter(|token| {
            !token.is_empty()
                && token.len() <= TOKEN_MAX_CHARS
                && token.chars().all(|c| c.is_ascii_graphic())
        })
        .map(|token| Secret(token.to_string()))
}

/// Three non-empty base64url segments, the compact form of a signed JWT.
fn jwt_shaped(token: &str) -> bool {
    let segments: Vec<&str> = token.split('.').collect();
    segments.len() == 3
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
}

const fn succeeded(status: u16) -> bool {
    status >= 200 && status < 300
}

fn decode_oidc(status: u16, body: &str) -> OidcOutcome {
    if !succeeded(status) {
        return OidcOutcome::Failed(status);
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body.trim()) else {
        return OidcOutcome::Unrecognised("the OIDC response is not JSON");
    };
    match usable_token(value.get("value").and_then(serde_json::Value::as_str)) {
        Some(token) if jwt_shaped(token.expose()) => OidcOutcome::Issued(token),
        _ => OidcOutcome::Unrecognised("the OIDC response has no JWT in `value`"),
    }
}

/// The OAuth `error` code of a refusal when it is a short `snake_case` word, so a body that
/// echoes a token contributes nothing.
fn error_code(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let code = value.get("error")?.as_str()?;
    let plain = !code.is_empty()
        && code.len() <= ERROR_CODE_MAX_CHARS
        && code.chars().all(|c| c.is_ascii_lowercase() || c == '_');
    plain.then(|| code.to_string())
}

fn decode_exchange(status: u16, body: &str) -> ExchangeOutcome {
    if !succeeded(status) {
        return ExchangeOutcome::Failed(status, error_code(body));
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body.trim()) else {
        return ExchangeOutcome::Unrecognised("the token response is not JSON");
    };
    let Some(token) = usable_token(
        value
            .get("access_token")
            .and_then(serde_json::Value::as_str),
    ) else {
        return ExchangeOutcome::Unrecognised("the token response has no usable `access_token`");
    };
    let bearer = value
        .get("token_type")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|kind| kind.eq_ignore_ascii_case("bearer"));
    if !bearer {
        return ExchangeOutcome::Unrecognised("the token response's `token_type` is not Bearer");
    }
    match value.get("expires_in").and_then(serde_json::Value::as_u64) {
        Some(seconds) if seconds > 0 && seconds <= EXPIRES_IN_MAX_SECONDS => {
            ExchangeOutcome::Issued(AccessToken {
                value: token,
                expires_in_seconds: seconds,
            })
        }
        _ => ExchangeOutcome::Unrecognised(
            "the token response's `expires_in` is not a lifetime of at most a day",
        ),
    }
}

fn exchange_body(federation: &Federation, assertion: &Secret) -> String {
    let mut body = serde_json::json!({
        "grant_type": JWT_BEARER_GRANT,
        "assertion": assertion.expose(),
        "federation_rule_id": federation.rule,
        "organization_id": federation.organization,
    });
    if let Some(fields) = body.as_object_mut() {
        if let Some(service_account) = &federation.service_account {
            fields.insert("service_account_id".into(), service_account.as_str().into());
        }
        if let Some(workspace) = &federation.workspace {
            fields.insert("workspace_id".into(), workspace.as_str().into());
        }
    }
    body.to_string()
}

fn response(reply: Reply) -> Result<(u16, String), &'static str> {
    match reply {
        Reply::Response { status, body } => Ok((status, body)),
        Reply::BodyTooLarge => Err("the response is over its size limit"),
        Reply::Failed(reason) => Err(reason),
    }
}

/// The claims a federation rule matches, read from the JWT's payload, for the job log, with
/// the window from `nbf` to `exp`, which caps the access token: the token itself and its
/// signature are never shown. A rule is written against these as the identity provider issues
/// them; GitHub's `sub`, for one, can carry owner and repository ids.
pub fn rule_claims(jwt: &str) -> Option<String> {
    let payload = jwt.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let mut shown: Vec<String> = RULE_CLAIMS
        .iter()
        .filter_map(|name| {
            let value: String = claims
                .get(*name)?
                .as_str()?
                .chars()
                .filter(|c| !c.is_control())
                .take(CLAIM_SHOWN_MAX_CHARS)
                .collect();
            Some(format!("{name}={value}"))
        })
        .collect();
    let time = |name: &str| claims.get(name).and_then(serde_json::Value::as_u64);
    if let (Some(from), Some(expires)) = (time("nbf").or_else(|| time("iat")), time("exp")) {
        shown.push(format!("valid={} s", expires.saturating_sub(from)));
    }
    Some(shown.join(", "))
}

/// Trades the job's OIDC token for an Anthropic access token. Anthropic documents no
/// revocation for it; it lapses after its `expires_in`.
pub fn mint(federation: &Federation, request: &OidcRequest) -> Result<AccessToken, String> {
    mint_at(&PlainClient::https(), federation, request, TOKEN_ENDPOINT)
}

fn mint_at(
    client: &PlainClient,
    federation: &Federation,
    request: &OidcRequest,
    endpoint: &str,
) -> Result<AccessToken, String> {
    let oidc = client.request(
        Method::Get,
        &audience_url(&request.url),
        Credential::Bearer(request.token.expose()),
        None,
    );
    let (status, body) =
        response(oidc).map_err(|reason| format!("the OIDC token request failed: {reason}"))?;
    let assertion = match decode_oidc(status, &body) {
        OidcOutcome::Issued(token) => token,
        OidcOutcome::Failed(status) => {
            return Err(format!("the OIDC token request ended HTTP {status}"));
        }
        OidcOutcome::Unrecognised(reason) => return Err(reason.to_string()),
    };
    if let Some(claims) = rule_claims(assertion.expose()) {
        println!("xtask: exchanging an OIDC token with {claims}");
    }
    let exchanged = client.request(
        Method::Post,
        endpoint,
        Credential::None,
        Some(&exchange_body(federation, &assertion)),
    );
    let (status, body) =
        response(exchanged).map_err(|reason| format!("the token exchange failed: {reason}"))?;
    match decode_exchange(status, &body) {
        ExchangeOutcome::Issued(token) => Ok(token),
        ExchangeOutcome::Failed(status, code) => Err(format!(
            "the token exchange ended HTTP {status}{}; the federation rule must match the \
             claims printed above and the audience https://api.anthropic.com, and the \
             Console's authentication history names the check that failed",
            code.map(|code| format!(", {code}")).unwrap_or_default()
        )),
        ExchangeOutcome::Unrecognised(reason) => Err(reason.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github_api::tests::listener;

    const JWT: &str = "eyJhbGciOiJSUzI1NiJ9.eyJhdWQiOiJ4In0.c2lnbmF0dXJl";
    const ACCESS: &str = "sk-ant-oat01-exampleexampleexample";

    fn federation() -> Federation {
        read_federation(
            Some("fdrl_01Example".into()),
            Some("d7253d24-0000-4000-8000-000000000000".into()),
            Some("svac_01Example".into()),
            Some("wrkspc_01Example".into()),
        )
        .unwrap()
    }

    #[test]
    fn the_rule_and_organization_are_required_and_every_identifier_is_checked() {
        let missing = read_federation(None, Some("org".into()), None, None).unwrap_err();
        assert!(missing.contains(FEDERATION_RULE_VARIABLE), "{missing}");
        assert!(missing.contains("docs/review.md"), "{missing}");
        let blank = read_federation(Some("r".into()), Some("  ".into()), None, None).unwrap_err();
        assert!(blank.contains(ORGANIZATION_VARIABLE), "{blank}");
        let bad = read_federation(
            Some("r".into()),
            Some("o".into()),
            Some("svac 1".into()),
            None,
        )
        .unwrap_err();
        assert!(bad.starts_with(SERVICE_ACCOUNT_VARIABLE), "{bad}");
        let minimal = read_federation(Some("r".into()), Some("o".into()), None, None).unwrap();
        assert_eq!(minimal.service_account, None);
        assert_eq!(minimal.workspace, None);
    }

    #[test]
    fn the_oidc_request_needs_https_and_both_values() {
        assert!(read_oidc_request(None, Some("t".into())).is_err());
        assert!(read_oidc_request(Some("https://x/".into()), None).is_err());
        assert!(read_oidc_request(Some("http://x/".into()), Some("t".into())).is_err());
        assert!(read_oidc_request(Some("https://x/#f".into()), Some("t".into())).is_err());
        assert!(read_oidc_request(Some("https://x/".into()), Some("a b".into())).is_err());
        assert!(read_oidc_request(Some("https://x/?a=1".into()), Some("t".into())).is_ok());
    }

    #[test]
    fn the_audience_is_appended_to_the_request_url() {
        assert_eq!(
            audience_url("https://x/token?api-version=2.0"),
            "https://x/token?api-version=2.0&audience=https%3A%2F%2Fapi.anthropic.com"
        );
        assert_eq!(
            audience_url("https://x/token"),
            "https://x/token?audience=https%3A%2F%2Fapi.anthropic.com"
        );
    }

    #[test]
    fn oidc_decoder_covers_each_state() {
        assert_eq!(
            decode_oidc(200, &format!(r#"{{"count":1,"value":"{JWT}"}}"#)),
            OidcOutcome::Issued(Secret(JWT.to_string()))
        );
        assert_eq!(decode_oidc(403, "{}"), OidcOutcome::Failed(403));
        for body in ["not json", r#"{"value":""}"#, r#"{"value":"opaque"}"#, "{}"] {
            assert!(
                matches!(decode_oidc(200, body), OidcOutcome::Unrecognised(_)),
                "{body}"
            );
        }
    }

    #[test]
    fn exchange_decoder_covers_each_state() {
        let issued = format!(
            r#"{{"access_token":"{ACCESS}","token_type":"Bearer","expires_in":3600,"scope":"x"}}"#
        );
        assert_eq!(
            decode_exchange(200, &issued),
            ExchangeOutcome::Issued(AccessToken {
                value: Secret(ACCESS.to_string()),
                expires_in_seconds: 3600
            })
        );
        assert_eq!(
            decode_exchange(400, r#"{"error":"invalid_grant","error_description":"x"}"#),
            ExchangeOutcome::Failed(400, Some("invalid_grant".to_string()))
        );
        assert_eq!(
            decode_exchange(401, &format!(r#"{{"error":"{ACCESS}"}}"#)),
            ExchangeOutcome::Failed(401, None)
        );
        for body in [
            "not json".to_string(),
            r#"{"token_type":"Bearer","expires_in":60}"#.to_string(),
            format!(r#"{{"access_token":"{ACCESS}","token_type":"mac","expires_in":60}}"#),
            format!(r#"{{"access_token":"{ACCESS}","token_type":"Bearer"}}"#),
            format!(r#"{{"access_token":"{ACCESS}","token_type":"Bearer","expires_in":0}}"#),
            format!(r#"{{"access_token":"{ACCESS}","token_type":"Bearer","expires_in":90000}}"#),
        ] {
            let outcome = decode_exchange(200, &body);
            assert!(
                matches!(outcome, ExchangeOutcome::Unrecognised(_)),
                "{body}"
            );
            assert!(!format!("{outcome:?}").contains(ACCESS));
        }
    }

    #[test]
    fn the_rule_claims_are_shown_without_the_token_or_control_characters() {
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            serde_json::json!({
                "sub": "repo:frits-v@4488681/stencil@1411277668:environment:review",
                "environment": "review\u{1b}[31m",
                "job_workflow_ref": "frits-v/stencil/.github/workflows/review.yml@refs/heads/main",
                "nbf": 1_791_916_602_u64,
                "iat": 1_791_916_902_u64,
                "exp": 1_791_917_202_u64,
                "aud": "https://api.anthropic.com",
            })
            .to_string(),
        );
        let jwt = format!("eyJhbGciOiJSUzI1NiJ9.{payload}.c2lnbmF0dXJlLXZhbHVl");
        let shown = rule_claims(&jwt).unwrap();
        assert_eq!(
            shown,
            "sub=repo:frits-v@4488681/stencil@1411277668:environment:review, \
             environment=review[31m, job_workflow_ref=frits-v/stencil/.github/workflows/\
             review.yml@refs/heads/main, valid=600 s"
        );
        assert!(!shown.contains("c2lnbmF0dXJl") && !shown.contains(&payload));
        assert_eq!(rule_claims("not-a-jwt"), None);
        assert_eq!(rule_claims("a.!!!.c"), None);
    }

    #[test]
    fn mint_requests_the_audience_then_sends_the_jwt_only_in_the_grant_body() {
        let (base, seen) = listener(vec![
            (200, format!(r#"{{"value":"{JWT}"}}"#)),
            (
                200,
                format!(r#"{{"access_token":"{ACCESS}","token_type":"bearer","expires_in":3600}}"#),
            ),
        ]);
        let request = OidcRequest {
            url: format!("{base}/oidc?api-version=2.0"),
            token: Secret("request-token".to_string()),
        };
        let token = mint_at(
            &PlainClient::loopback(),
            &federation(),
            &request,
            &format!("{base}/v1/oauth/token"),
        )
        .unwrap();
        assert_eq!(token.value.expose(), ACCESS);
        assert_eq!(token.expires_in_seconds, 3600);
        let requests = seen.recv().unwrap();
        let oidc = requests.first().unwrap();
        assert!(
            oidc.starts_with(
                "GET /oidc?api-version=2.0&audience=https%3A%2F%2Fapi.anthropic.com HTTP/1.1\r\n"
            ),
            "{oidc}"
        );
        assert!(
            oidc.to_ascii_lowercase()
                .contains("\r\nauthorization: bearer request-token\r\n")
        );
        let exchange = requests.get(1).unwrap();
        assert!(
            exchange.starts_with("POST /v1/oauth/token HTTP/1.1\r\n"),
            "{exchange}"
        );
        assert!(!exchange.to_ascii_lowercase().contains("authorization:"));
        let body: serde_json::Value =
            serde_json::from_str(exchange.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "grant_type": JWT_BEARER_GRANT,
                "assertion": JWT,
                "federation_rule_id": "fdrl_01Example",
                "organization_id": "d7253d24-0000-4000-8000-000000000000",
                "service_account_id": "svac_01Example",
                "workspace_id": "wrkspc_01Example",
            })
        );
    }

    #[test]
    fn a_refused_exchange_names_its_status_and_code_and_never_the_jwt() {
        let (base, _seen) = listener(vec![
            (200, format!(r#"{{"value":"{JWT}"}}"#)),
            (400, r#"{"error":"invalid_grant"}"#.to_string()),
        ]);
        let request = OidcRequest {
            url: format!("{base}/oidc"),
            token: Secret("request-token".to_string()),
        };
        let error = mint_at(
            &PlainClient::loopback(),
            &federation(),
            &request,
            &format!("{base}/v1/oauth/token"),
        )
        .unwrap_err();
        assert!(error.contains("HTTP 400, invalid_grant"), "{error}");
        assert!(!error.contains(JWT) && !error.contains("request-token"));
    }
}
