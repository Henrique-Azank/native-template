//! Programmatic dependency layering verification.
//! Enforces:
//! 1. Downward dependency rule (L_from > L_to, unless whitelisted).
//! 2. Zero UI crates below Layer 6.
//! 3. Zero Networking crates below Layer 6.

use std::collections::BTreeMap;
use std::process::Command;

/// (crate suffix, layer)
const TABLE: &[(&str, u8)] = &[
    ("core", 0),
    ("model", 1),
    ("engine", 5),
    ("net", 6),
    ("ui-egui", 6),
    ("mcp", 6),
];

/// UI toolkit dependencies: only allowed at L6 and higher.
const UI_CRATES: &[&str] = &["egui", "eframe", "rfd", "winit"];

/// Networking dependencies: only allowed at L6 and higher.
const NETWORK_CRATES: &[&str] = &["reqwest", "tungstenite", "tokio", "mio", "socket2"];

pub fn check(root: &std::path::Path) -> bool {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = match Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(root)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("xtask: failed to run cargo metadata: {e}");
            return false;
        }
    };

    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        eprintln!("xtask: invalid json from cargo metadata");
        return false;
    };

    let layer: BTreeMap<String, u8> = TABLE
        .iter()
        .map(|(n, l)| (format!("template-{n}"), *l))
        .collect();

    let mut ok = true;

    for pkg in v["packages"].as_array().cloned().unwrap_or_default() {
        let name = pkg["name"].as_str().unwrap_or("").to_string();
        let Some(&l) = layer.get(&name) else { continue };

        for dep in pkg["dependencies"].as_array().cloned().unwrap_or_default() {
            let dn = dep["name"].as_str().unwrap_or("");
            if dep["kind"].as_str() == Some("dev") {
                continue;
            }

            if l < 6 && UI_CRATES.contains(&dn) {
                eprintln!("layers violation: {name} (L{l}) depends on UI crate {dn}");
                ok = false;
            }

            if l < 6 && NETWORK_CRATES.contains(&dn) {
                eprintln!("layers violation: {name} (L{l}) depends on networking crate {dn}");
                ok = false;
            }

            if let Some(&dl) = layer.get(dn)
                && dl >= l
                && !(dl == l && same_layer_ok(&name, dn))
            {
                eprintln!("layers violation: {name} (L{l}) may not depend on {dn} (L{dl})");
                ok = false;
            }
        }
    }

    if ok {
        eprintln!("layers: ok");
    }
    ok
}

fn same_layer_ok(from: &str, to: &str) -> bool {
    // Within Layer 6, presentation (ui-egui) and agent protocol (mcp) may use the net engine
    matches!(
        (from, to),
        ("template-ui-egui", "template-net") | ("template-mcp", "template-net")
    )
}
