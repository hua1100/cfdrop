use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, Utc};
use rand::{rngs::OsRng, RngCore};

const TOKEN_BYTES: usize = 32;
const TOKEN_LENGTH: usize = 43;

pub struct SignedAccess {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(serde::Serialize)]
pub struct MachineDeployOutput<'a> {
    pub deployment_url: &'a str,
    pub access_url: &'a str,
    pub expires_at: String,
}

pub fn generate_bearer_token() -> Result<String> {
    let mut bytes = [0_u8; TOKEN_BYTES];
    OsRng.fill_bytes(&mut bytes);
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn effective_expiry(
    now: DateTime<Utc>,
    requested_ttl_seconds: u64,
    account_expires_at: DateTime<Utc>,
    safety_margin_seconds: u64,
    min_valid_for_seconds: u64,
) -> Result<DateTime<Utc>> {
    let requested_ttl = seconds(requested_ttl_seconds, "requested lifetime")?;
    let safety_margin = seconds(safety_margin_seconds, "safety margin")?;
    let minimum_lifetime = seconds(min_valid_for_seconds, "minimum lifetime")?;
    let requested_expiry = now
        .checked_add_signed(requested_ttl)
        .context("requested signed-link expiry is out of range")?;
    let account_deadline = account_expires_at
        .checked_sub_signed(safety_margin)
        .context("temporary-account expiry is out of range")?;
    let expires_at = requested_expiry.min(account_deadline);
    let minimum_expiry = now
        .checked_add_signed(minimum_lifetime)
        .context("minimum signed-link expiry is out of range")?;
    if expires_at < minimum_expiry {
        bail!("temporary account cannot provide the minimum signed-link lifetime");
    }
    Ok(expires_at)
}

pub fn ensure_minimum_remaining_lifetime(
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    min_valid_for_seconds: u64,
) -> Result<()> {
    let minimum_lifetime = seconds(min_valid_for_seconds, "minimum lifetime")?;
    let minimum_expiry = now
        .checked_add_signed(minimum_lifetime)
        .context("minimum signed-link expiry is out of range")?;
    if expires_at < minimum_expiry {
        bail!("signed link no longer has the minimum required lifetime");
    }
    Ok(())
}

pub fn access_url(deployment_url: &str, token: &str) -> Result<String> {
    validate_token(token)?;
    let mut url = reqwest::Url::parse(deployment_url).context("invalid deployment URL")?;
    if url.scheme() != "https" {
        bail!("deployment URL must use HTTPS");
    }
    url.set_path(&format!("/_cfdrop/{token}/"));
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

pub fn signed_worker_script(token: &str, expires_at_epoch_seconds: i64) -> Result<String> {
    validate_token(token)?;
    if expires_at_epoch_seconds <= 0 {
        bail!("signed-link expiry must be after the Unix epoch");
    }
    let token = serde_json::to_string(token)?;
    Ok(format!(
        r#"const TOKEN = {token};
const PREFIX = `/_cfdrop/${{TOKEN}}`;
const EXPIRES_AT = {expires_at_epoch_seconds} * 1000;
const SECURITY_HEADERS = {{
  "Cache-Control": "private, no-store",
  "Referrer-Policy": "no-referrer",
  "X-Content-Type-Options": "nosniff",
  "Content-Security-Policy": "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'none'; frame-ancestors 'none'; base-uri 'none'",
}};

function page(message, status) {{
  return new Response(`<!doctype html><meta name="viewport" content="width=device-width"><title>cfdrop report</title><p>${{message}}</p>`, {{
    status,
    headers: {{ ...SECURITY_HEADERS, "Content-Type": "text/html; charset=utf-8" }},
  }});
}}

function forbidden() {{
  return page("This report link is invalid.", 403);
}}

function expired() {{
  return page("This report link has expired.", 410);
}}

function prefixedLocation(location, requestUrl) {{
  let target;
  try {{
    target = new URL(location, requestUrl.origin);
  }} catch {{
    return location;
  }}
  if (target.origin !== requestUrl.origin) return location;
  if (target.pathname !== PREFIX && !target.pathname.startsWith(`${{PREFIX}}/`)) {{
    target.pathname = `${{PREFIX}}${{target.pathname}}`;
  }}
  return `${{target.pathname}}${{target.search}}${{target.hash}}`;
}}

function secured(response, requestUrl) {{
  const headers = new Headers(response.headers);
  for (const [name, value] of Object.entries(SECURITY_HEADERS)) headers.set(name, value);
  const location = headers.get("Location");
  if (location) headers.set("Location", prefixedLocation(location, requestUrl));
  return new Response(response.body, {{
    status: response.status,
    statusText: response.statusText,
    headers,
  }});
}}

export default {{
  async fetch(request, env) {{
    const url = new URL(request.url);
    if (url.pathname !== PREFIX && !url.pathname.startsWith(`${{PREFIX}}/`)) {{
      return forbidden();
    }}
    if (Date.now() >= EXPIRES_AT) return expired(); // status: 410
    url.pathname = url.pathname.slice(PREFIX.length) || "/";
    const response = await env.ASSETS.fetch(new Request(url, request));
    return secured(response, url);
  }},
}};
"#
    ))
}

fn seconds(value: u64, label: &str) -> Result<Duration> {
    let value = i64::try_from(value).with_context(|| format!("{label} is too large"))?;
    Ok(Duration::seconds(value))
}

fn validate_token(token: &str) -> Result<()> {
    if token.len() != TOKEN_LENGTH
        || !token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!("signed-link token must be a 256-bit base64url value");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        effective_expiry, ensure_minimum_remaining_lifetime, generate_bearer_token,
        signed_worker_script, MachineDeployOutput,
    };
    use chrono::{Duration, TimeZone, Utc};
    use std::collections::BTreeSet;

    #[test]
    fn machine_output_has_exact_public_fields() {
        let json = serde_json::to_value(MachineDeployOutput {
            deployment_url: "https://site.example.workers.dev",
            access_url: "https://site.example.workers.dev/_cfdrop/token/",
            expires_at: "2026-09-12T12:55:00Z".into(),
        })
        .unwrap();
        assert_eq!(
            json.as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "access_url".into(),
                "deployment_url".into(),
                "expires_at".into()
            ])
        );
        let rendered = json.to_string();
        assert!(!rendered.contains("claim"));
        assert!(!rendered.contains("api_token"));
    }

    #[test]
    fn token_is_256_bit_base64url() {
        let token = generate_bearer_token().unwrap();
        assert_eq!(token.len(), 43);
        assert!(token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
    }

    #[test]
    fn expiry_is_bounded_by_account_lifetime() {
        let now = Utc.with_ymd_and_hms(2026, 9, 12, 12, 0, 0).unwrap();
        let account = now + Duration::minutes(58);
        assert_eq!(
            effective_expiry(now, 3600, account, 60, 300).unwrap(),
            account - Duration::seconds(60)
        );
    }

    #[test]
    fn rejects_insufficient_remaining_lifetime() {
        let now = Utc::now();
        assert!(effective_expiry(now, 3600, now + Duration::seconds(200), 60, 300).is_err());
    }

    #[test]
    fn recheck_rejects_lifetime_consumed_during_deploy() {
        let now = Utc.with_ymd_and_hms(2026, 9, 12, 12, 0, 0).unwrap();
        let expires_at = now + Duration::seconds(299);

        assert!(ensure_minimum_remaining_lifetime(now, expires_at, 300).is_err());
    }

    #[test]
    fn guard_denies_before_assets_fallback() {
        let script = signed_worker_script("A".repeat(43).as_str(), 1_789_200_000).unwrap();
        let deny = script.find("return forbidden()").unwrap();
        let assets = script.find("env.ASSETS.fetch").unwrap();
        assert!(deny < assets);
        assert!(script.contains("status: 410"));
        assert!(script.contains("Referrer-Policy"));
        assert!(script.contains("Cache-Control"));
    }

    #[test]
    fn guard_validates_bearer_path_before_expiry() {
        let script = signed_worker_script("A".repeat(43).as_str(), 1_789_200_000).unwrap();
        let fetch = script.find("async fetch(request, env)").unwrap();
        let handler = &script[fetch..];
        let path = handler.find("const url = new URL(request.url)").unwrap();
        let forbidden = handler.find("return forbidden()").unwrap();
        let expiry = handler.find("Date.now() >= EXPIRES_AT").unwrap();

        assert!(path < forbidden);
        assert!(forbidden < expiry);
    }

    #[test]
    fn guard_prefixes_root_relative_nested_index_redirects() {
        let script = signed_worker_script("A".repeat(43).as_str(), 1_789_200_000).unwrap();

        assert!(script.contains("function prefixedLocation(location, requestUrl)"));
        assert!(script.contains("new URL(location, requestUrl.origin)"));
        assert!(script.contains("target.pathname = `${PREFIX}${target.pathname}`"));
        assert!(
            script.contains("headers.set(\"Location\", prefixedLocation(location, requestUrl))")
        );
    }

    #[test]
    fn guard_prefixes_same_origin_html_canonical_redirects() {
        let script = signed_worker_script("A".repeat(43).as_str(), 1_789_200_000).unwrap();

        assert!(script.contains("target.origin !== requestUrl.origin"));
        assert!(script.contains("!target.pathname.startsWith(`${PREFIX}/`)"));
        assert!(script.contains("`${target.pathname}${target.search}${target.hash}`"));
    }
}
