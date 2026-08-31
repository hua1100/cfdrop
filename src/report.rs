use anyhow::{bail, Result};
use std::path::PathBuf;

pub fn deploy_report(
    _directory: PathBuf,
    _name: Option<String>,
    _yes: bool,
    _fresh: bool,
    _auth: Option<String>,
) -> Result<()> {
    bail!("report deploy is not implemented yet")
}

pub fn fetch_comments(_url: &str, _format: &str) -> Result<()> {
    bail!("report comments is not implemented yet")
}
