use crate::{cf, manifest, report_worker, state, storage};
use anyhow::{bail, Context, Result};
use base64::Engine;
use chrono::{Duration, Utc};
use std::io::Write;
use std::path::PathBuf;

fn sanitize_name(raw: &str) -> String {
    let mut s: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "cfdrop-report".to_string()
    } else {
        s.chars().take(54).collect()
    }
}

fn confirm_terms() -> Result<bool> {
    eprintln!(
        "Continuing creates a temporary Cloudflare account and means you accept:\n  Terms of Service: {}\n  Privacy Policy:   {}",
        cf::TERMS_URL,
        cf::PRIVACY_URL
    );
    eprint!("Proceed? [y/N] ");
    std::io::stderr().flush().ok();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(matches!(line.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn report_run_worker_first(auth_token: Option<&str>) -> serde_json::Value {
    if auth_token.is_some() {
        serde_json::json!(true)
    } else {
        serde_json::json!(["/api/*"])
    }
}

pub fn deploy_report(
    directory: PathBuf,
    name: Option<String>,
    yes: bool,
    fresh: bool,
    auth: Option<String>,
) -> Result<()> {
    let auth_token = match &auth {
        Some(cred) => {
            let (user, pass) = cred
                .split_once(':')
                .context("--auth must be in the form user:pass")?;
            if user.is_empty() || pass.is_empty() {
                bail!("--auth must be in the form user:pass (both non-empty)");
            }
            Some(base64::engine::general_purpose::STANDARD.encode(cred))
        }
        None => None,
    };
    let directory = directory
        .canonicalize()
        .with_context(|| format!("directory not found: {}", directory.display()))?;
    let script_name = sanitize_name(&name.unwrap_or_else(|| {
        directory
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "cfdrop-report".into())
    }));
    let staged = crate::report_ui::inject_report_ui(&directory, &script_name)?;
    let entries = manifest::build_manifest(staged.path())?;
    let total_bytes: u64 = entries.iter().map(|e| e.size).sum();
    eprintln!(
        "Found {} file(s), {:.1} KiB total.",
        entries.len(),
        total_bytes as f64 / 1024.0
    );

    let client = cf::CfClient::new()?;
    let state_path = state::state_path()?;
    let cached = if fresh {
        None
    } else {
        state::load(&state_path)
    };
    let margin = Duration::minutes(5);
    let (account, reused) = match cached {
        Some(acc) if acc.is_usable(Utc::now(), margin) => (acc, true),
        _ => {
            if !yes && !confirm_terms()? {
                bail!("aborted: terms not accepted");
            }
            eprintln!("Provisioning temporary Cloudflare account...");
            let acc = client.provision_temp_account()?;
            state::save(&state_path, &acc)?;
            (acc, false)
        }
    };
    eprintln!(
        "Temporary account {} ({}), expires {}",
        account.account_name,
        if reused { "reused" } else { "created" },
        account.account_expires_at.format("%H:%M UTC")
    );
    let session = client.start_upload_session(&account, &script_name, &entries)?;
    let completion_jwt = client.upload_assets(&account, &session, &entries)?;
    let storage = storage::provision_comments_storage(&client, &account, &script_name)?;
    let script = report_worker::comments_worker_script_with_storage(
        &script_name,
        storage.label(),
        auth_token.as_deref(),
    );
    let run_worker_first = report_run_worker_first(auth_token.as_deref());
    client.deploy_worker_with_script(
        &account,
        &script_name,
        &completion_jwt,
        &script,
        run_worker_first,
        vec![storage.binding()],
    )?;
    client.enable_workers_dev(&account, &script_name)?;
    let subdomain = client.get_subdomain(&account)?;
    let url = format!("https://{script_name}.{subdomain}.workers.dev");
    let minutes_left = (account.claim_expires_at - Utc::now()).num_minutes().max(0);
    println!("\n✅ Deployed: {url}");
    println!("Comments API: {url}/api/comments");
    println!("Read comments: cfdrop report comments --url {url} --format md");
    println!("\nThis temporary account expires in ~{minutes_left} minutes.");
    println!("Keep it by claiming: {}", account.claim_url);
    Ok(())
}

pub fn fetch_comments(url: &str, format: &str) -> Result<()> {
    let base = url.trim_end_matches('/');
    let endpoint = match format {
        "json" => format!("{base}/api/comments"),
        "md" => format!("{base}/api/comments.md"),
        other => bail!("unsupported format: {other}"),
    };
    let resp = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?
        .get(&endpoint)
        .send()
        .with_context(|| format!("fetching comments from {endpoint}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        let snippet: String = body.chars().take(240).collect();
        bail!("comments endpoint {endpoint} returned {status}: {snippet}");
    }
    println!("{}", resp.text()?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::report_run_worker_first;

    #[test]
    fn report_worker_runs_only_for_api_without_auth() {
        assert_eq!(report_run_worker_first(None), serde_json::json!(["/api/*"]));
    }

    #[test]
    fn report_worker_runs_for_all_requests_with_auth() {
        assert_eq!(
            report_run_worker_first(Some("dXNlcjpwYXNz")),
            serde_json::json!(true)
        );
    }
}
