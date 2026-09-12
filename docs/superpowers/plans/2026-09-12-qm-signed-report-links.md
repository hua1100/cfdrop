# QM Signed Report Links Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a native cfdrop mode that publishes a temporary static report behind a bearer link, reports the real expiry as strict JSON, and ships pinned Linux artifacts for QM.

**Architecture:** Extend generic `cfdrop deploy`; do not use the comment-enabled `cfdrop report deploy`. A small `signed` module generates a 256-bit path token, renders a Worker-first access guard, bounds report expiry by the temporary account, and emits an exact machine result. Release CI builds immutable musl binaries and checksum sidecars.

**Tech Stack:** Rust 2021, Clap, Cloudflare temporary accounts/Workers Static Assets, GitHub Actions, musl cross builds.

---

## File map

- Create `src/signed.rs`: token generation, lifetime calculation, Worker guard, access URL, and JSON DTO.
- Modify `src/main.rs`: CLI flags, validation, signed deployment branch, and human/machine output.
- Modify `Cargo.toml` and `Cargo.lock`: direct CSPRNG dependency and version `0.8.0`.
- Modify `.github/workflows/release.yml`: Linux checksum assets and complete-release verification.
- Modify `README.md`: signed mode, expiry, security, and Linux install contract.
- Do not modify `src/report.rs`, `src/report_worker.rs`, or `src/report_ui.rs`.

### Task 1: Tracer bullet from CLI to guarded Worker

**Files:**
- Create: `src/signed.rs`
- Modify: `src/main.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

- [ ] **Step 1: Write the failing CLI and signed-core tests**

Add a `cli_tests::parses_signed_machine_deploy` test in `src/main.rs` that parses:

```rust
let cli = Cli::try_parse_from([
    "cfdrop", "deploy", "-d", "site", "-y", "--fresh",
    "--signed-link", "--expires-in-seconds", "300",
    "--min-valid-for-seconds", "240", "--json",
]).unwrap();
match cli.command {
    Command::Deploy { signed_link, expires_in_seconds, min_valid_for_seconds, json, .. } => {
        assert!(signed_link);
        assert_eq!(expires_in_seconds, 300);
        assert_eq!(min_valid_for_seconds, 240);
        assert!(json);
    }
    _ => panic!("expected deploy"),
}
```

Create `src/signed.rs` with only a `#[cfg(test)]` module first. Tests must require:

```rust
#[test]
fn token_is_256_bit_base64url() {
    let token = generate_bearer_token().unwrap();
    assert_eq!(token.len(), 43);
    assert!(token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
}

#[test]
fn expiry_is_bounded_by_account_lifetime() {
    let now = Utc.with_ymd_and_hms(2026, 9, 12, 12, 0, 0).unwrap();
    let account = now + Duration::minutes(58);
    assert_eq!(effective_expiry(now, 3600, account, 60, 300).unwrap(), account - Duration::seconds(60));
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
```

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test cli_tests::parses_signed_machine_deploy -- --exact
cargo test signed::tests::token_is_256_bit_base64url -- --exact
```

Expected: the CLI rejects `--signed-link`, and the signed test does not compile because its functions do not exist.

- [ ] **Step 3: Implement the minimal signed module and CLI plumbing**

Add `rand = "0.8.5"` and `mod signed;`. Add these deploy fields with explicit bounds:

```rust
#[arg(long, conflicts_with = "auth")]
signed_link: bool,
#[arg(long, default_value_t = 3600, value_parser = clap::value_parser!(u64).range(60..=3600))]
expires_in_seconds: u64,
#[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u64).range(60..=3300))]
min_valid_for_seconds: u64,
#[arg(long, conflicts_with = "notify")]
json: bool,
```

Implement `src/signed.rs` with these public boundaries:

```rust
pub struct SignedAccess {
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize)]
pub struct MachineDeployOutput<'a> {
    pub deployment_url: &'a str,
    pub access_url: &'a str,
    pub expires_at: String,
}

pub fn generate_bearer_token() -> anyhow::Result<String>;
pub fn effective_expiry(
    now: DateTime<Utc>, requested_ttl_seconds: u64,
    account_expires_at: DateTime<Utc>, safety_margin_seconds: u64,
    min_valid_for_seconds: u64,
) -> anyhow::Result<DateTime<Utc>>;
pub fn access_url(deployment_url: &str, token: &str) -> anyhow::Result<String>;
pub fn signed_worker_script(token: &str, expires_at_epoch_seconds: i64) -> anyhow::Result<String>;
```

Use `OsRng.fill_bytes(&mut [0u8; 32])` and `URL_SAFE_NO_PAD`. The Worker must run first for every request, accept only `/_cfdrop/<token>/...`, strip that prefix before `env.ASSETS.fetch`, return 403 for a wrong/missing token and 410 after expiry, and set these headers on every response:

```text
Cache-Control: private, no-store
Referrer-Policy: no-referrer
X-Content-Type-Options: nosniff
Content-Security-Policy: default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'none'; frame-ancestors 'none'; base-uri 'none'
```

In `deploy()`, require `--fresh` for signed mode, compute the deadline from `account.account_expires_at` with a 60-second safety margin, and call existing `deploy_worker_with_script(..., json!(true), vec![])`.

- [ ] **Step 4: Run focused and full tests and verify GREEN**

Run:

```bash
cargo test cli_tests::parses_signed_machine_deploy -- --exact
cargo test signed::tests -- --nocapture
cargo fmt --check
cargo clippy --quiet -- -D warnings
cargo test --quiet
```

Expected: all commands exit 0; the suite contains at least the 48 baseline tests plus the new signed tests.

- [ ] **Step 5: Commit the tracer bullet**

```bash
git add Cargo.toml Cargo.lock src/main.rs src/signed.rs
git commit -m "feat: deploy reports behind signed links"
```

### Task 2: Strict machine output and fail-closed lifetime behavior

**Files:**
- Modify: `src/signed.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add failing output and validation tests**

Add tests proving:

```rust
#[test]
fn machine_output_has_exact_public_fields() {
    let json = serde_json::to_value(MachineDeployOutput {
        deployment_url: "https://site.example.workers.dev",
        access_url: "https://site.example.workers.dev/_cfdrop/token/",
        expires_at: "2026-09-12T12:55:00Z".into(),
    }).unwrap();
    assert_eq!(json.as_object().unwrap().keys().cloned().collect::<BTreeSet<_>>(),
        BTreeSet::from(["access_url".into(), "deployment_url".into(), "expires_at".into()]));
    let rendered = json.to_string();
    assert!(!rendered.contains("claim"));
    assert!(!rendered.contains("api_token"));
}

#[test]
fn rejects_insufficient_remaining_lifetime() {
    let now = Utc::now();
    assert!(effective_expiry(now, 3600, now + Duration::seconds(200), 60, 300).is_err());
}
```

Add CLI tests that reject `--signed-link` without `--fresh`, reject `--json --notify`, and reject `min_valid_for_seconds >= expires_in_seconds` before any network call.

- [ ] **Step 2: Run tests and verify RED**

Run `cargo test signed::tests cli_tests -- --nocapture`.

Expected: at least one assertion fails because strict output/validation is not complete.

- [ ] **Step 3: Implement exact output and second lifetime check**

Before upload and again immediately before printing the result, verify the computed expiry remains at least `min_valid_for_seconds` in the future. For `--json`, stdout must be exactly one `serde_json::to_string(&MachineDeployOutput)` line. All progress stays on stderr. Never print `account.claim_url`, account token, bearer token by itself, or Worker source in machine mode.

Human output may keep the existing claim URL only for non-JSON, non-signed deployments. Signed human output prints the access URL and RFC 3339 expiry, not a rounded minute estimate.

- [ ] **Step 4: Verify all behavior**

Run:

```bash
cargo fmt --check
cargo clippy --quiet -- -D warnings
cargo test --quiet
git diff --check
```

Expected: all exit 0, and tests confirm rejected paths never call the asset fallback.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/signed.rs
git commit -m "feat: add strict cfdrop machine output"
```

### Task 3: Immutable Linux release contract

**Files:**
- Modify: `Cargo.toml`
- Modify: `.github/workflows/release.yml`
- Modify: `README.md`

- [ ] **Step 1: Add a failing workflow contract check**

Run this check before editing:

```bash
rg -n 'sha256sum|\.sha256|cfdrop-linux-amd64|cfdrop-linux-arm64' .github/workflows/release.yml
```

Expected: Linux asset names exist but checksum generation/upload is absent.

- [ ] **Step 2: Update version and release workflow**

Set `version = "0.8.0"`. For each Linux matrix build, create and upload both files:

```bash
tar -czf "${{ matrix.asset }}.tar.gz" -C "target/${{ matrix.target }}/release" cfdrop
sha256sum "${{ matrix.asset }}.tar.gz" > "${{ matrix.asset }}.tar.gz.sha256"
gh release upload "$GITHUB_REF_NAME" \
  "${{ matrix.asset }}.tar.gz" "${{ matrix.asset }}.tar.gz.sha256" \
  --repo "$GITHUB_REPOSITORY" --clobber
```

Remove `--clobber`: an existing asset name must make the job fail instead of replacing a released binary. Add a final Ubuntu job that waits for Linux and macOS jobs, lists release assets, and fails unless the amd64/arm64 tarballs and both sidecars are present. Preserve immutable tag URLs; do not make the QM consumer trust a downloaded sidecar at runtime.

- [ ] **Step 3: Document the supported contract**

Document the command:

```bash
cfdrop deploy -d ./report -n qm-report -y --fresh \
  --signed-link --expires-in-seconds 3600 \
  --min-valid-for-seconds 300 --json
```

Document the exact three-field JSON response, bearer-link forwarding risk, real account-bounded expiry, 403/410 behavior, no third-party assets, and versioned Linux asset names.

- [ ] **Step 4: Verify release-ready source**

Run:

```bash
cargo fmt --check
cargo clippy --quiet -- -D warnings
cargo test --quiet
cargo build --release
git diff --check
```

Expected: all exit 0; `target/release/cfdrop --version` prints `cfdrop 0.8.0`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock .github/workflows/release.yml README.md
git commit -m "chore: prepare cfdrop v0.8.0"
```

Do not tag or publish from this task. Release publication happens in the integration rollout only after code and security review.
