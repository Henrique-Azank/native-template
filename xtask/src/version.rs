//! `cargo xtask version [set X.Y.Z[-pre]]`: workspace version management (single source of truth).
//!
//! The version lives in `[workspace.package] version` in the root `Cargo.toml`; every crate
//! inherits it with `version.workspace = true`, and all packaging scripts read it from here.

use std::path::Path;

/// Reads the `version = "..."` value inside `[workspace.package]`.
pub fn read(manifest: &str) -> Result<String, String> {
    let (_, line) = find_line(manifest)?;
    let v = line
        .split_once('=')
        .map(|(_, v)| v.trim().trim_matches('"').to_string())
        .unwrap_or_default();
    if v.is_empty() {
        return Err("empty [workspace.package] version".into());
    }
    Ok(v)
}

/// Returns `manifest` with the workspace version replaced; everything else is byte-for-byte unchanged.
pub fn replace(manifest: &str, new: &str) -> Result<String, String> {
    validate(new)?;
    let (idx, _) = find_line(manifest)?;
    let mut out = String::with_capacity(manifest.len() + 8);
    for (i, line) in manifest.split_inclusive('\n').enumerate() {
        if i == idx {
            let eol = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            out.push_str(&format!("version = \"{new}\"{eol}"));
        } else {
            out.push_str(line);
        }
    }
    Ok(out)
}

/// Validates semver without build metadata: `MAJOR.MINOR.PATCH` plus an optional `-pre.release` tag.
pub fn validate(v: &str) -> Result<(), String> {
    let bad = || {
        Err(format!(
            "`{v}` is not a valid semver like 1.2.3 or 1.2.3-rc.1"
        ))
    };
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (v, None),
    };
    let nums: Vec<&str> = core.split('.').collect();
    if nums.len() != 3
        || nums.iter().any(|n| {
            n.is_empty()
                || !n.bytes().all(|b| b.is_ascii_digit())
                || (n.len() > 1 && n.starts_with('0'))
        })
    {
        return bad();
    }
    if let Some(p) = pre
        && (p.is_empty()
            || p.split('.').any(|id| {
                id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            }))
    {
        return bad();
    }
    Ok(())
}

fn find_line(manifest: &str) -> Result<(usize, &str), String> {
    let mut in_section = false;
    for (i, line) in manifest.lines().enumerate() {
        let t = line.trim();
        if t.starts_with('[') {
            in_section = t == "[workspace.package]";
            continue;
        }
        if in_section
            && let Some(rest) = t.strip_prefix("version")
            && rest.trim_start().starts_with('=')
        {
            return Ok((i, line));
        }
    }
    Err("no `version = \"...\"` in [workspace.package] of the root Cargo.toml".into())
}

pub fn run(root: &Path, args: &[&str]) -> Result<(), String> {
    let path = root.join("Cargo.toml");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    match args {
        [] => {
            println!("{}", read(&text)?);
            Ok(())
        }
        ["set", new] => {
            let old = read(&text)?;
            let updated = replace(&text, new)?;
            let tmp = root.join("target").join("Cargo.toml.xtask-version");
            std::fs::create_dir_all(root.join("target"))
                .map_err(|e| format!("create target/: {e}"))?;
            std::fs::write(&tmp, updated).map_err(|e| format!("write {}: {e}", tmp.display()))?;
            std::fs::rename(&tmp, &path).map_err(|e| format!("replace {}: {e}", path.display()))?;
            println!("version: {old} -> {new}");
            Ok(())
        }
        _ => Err("usage: cargo xtask version [set X.Y.Z[-pre]]".into()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const MANIFEST: &str = "[workspace]\nmembers = [\"a\"]\n\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace.dependencies]\nfoo = { version = \"1\" }\n";

    #[test]
    fn reads_and_replaces_workspace_version() {
        assert_eq!(read(MANIFEST).unwrap(), "0.1.0");
        let out = replace(MANIFEST, "1.2.3-rc.1").unwrap();
        assert_eq!(read(&out).unwrap(), "1.2.3-rc.1");
        assert!(validate("1.0.0").is_ok());
        assert!(validate("1.0.0-rc.1").is_ok());
        assert!(validate("v1.0.0").is_err());
        assert!(validate("01.0.0").is_err());
    }
}
