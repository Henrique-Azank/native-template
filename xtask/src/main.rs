//! Template automation tasks (xtask).
//! Usage:
//! - `cargo xtask layers`: verify architectural layer boundaries
//! - `cargo xtask ci`: run fmt, clippy, tests, and layer checks
//! - `cargo xtask wasm`: verify WebAssembly compilation
//! - `cargo xtask version [set X.Y.Z]`: read or set the workspace version
//! - `cargo xtask ico <out.ico> <in.png>...`: pack PNGs into a Windows .ico file

mod ico;
mod layers;
mod version;

use std::path::PathBuf;
use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = args.first().map(String::as_str).unwrap_or("ci");
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();

    let root = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => PathBuf::from(dir)
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .to_path_buf(),
        Err(_) => PathBuf::from("."),
    };

    match task {
        "layers" => {
            if !layers::check(&root) {
                exit(1);
            }
        }
        "version" => {
            if let Err(e) = version::run(&root, &rest) {
                eprintln!("xtask version: {e}");
                exit(1);
            }
        }
        "ico" => {
            if let Err(e) = ico::run(&rest) {
                eprintln!("xtask ico: {e}");
                exit(1);
            }
        }
        "wasm" => {
            println!("==> Checking wasm32-unknown-unknown compilation...");
            let ok = run_cmd(
                "cargo",
                &[
                    "check",
                    "--target",
                    "wasm32-unknown-unknown",
                    "-p",
                    "template-core",
                    "-p",
                    "template-model",
                    "-p",
                    "template-engine",
                    "--offline",
                    "--ignore-rust-version",
                ],
                &root,
            );
            if !ok {
                eprintln!(
                    "\nNote: If `wasm32-unknown-unknown` target is not yet installed in your rustup toolchain, install it with:\n  rustup target add wasm32-unknown-unknown\n"
                );
                exit(1);
            }
        }
        "ci" => {
            println!("==> 1. Checking code formatting...");
            if !run_cmd("cargo", &["fmt", "--all", "--", "--check"], &root) {
                exit(1);
            }

            println!("==> 2. Verifying layer boundaries...");
            if !layers::check(&root) {
                exit(1);
            }

            println!("==> 3. Running clippy...");
            if !run_cmd(
                "cargo",
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--offline",
                    "--ignore-rust-version",
                    "--",
                    "-D",
                    "warnings",
                ],
                &root,
            ) {
                exit(1);
            }

            println!("==> 4. Running test suite...");
            if !run_cmd(
                "cargo",
                &["test", "--workspace", "--offline", "--ignore-rust-version"],
                &root,
            ) {
                exit(1);
            }

            println!("==> CI passed successfully!");
        }
        other => {
            eprintln!("Unknown task `{other}`. Available: layers, ci, wasm, version, ico");
            exit(1);
        }
    }
}

fn run_cmd(cmd: &str, args: &[&str], root: &std::path::Path) -> bool {
    let mut command = Command::new(cmd);
    command.args(args).current_dir(root);
    let status = command.status();
    match status {
        Ok(s) => s.success(),
        Err(e) => {
            eprintln!("Command `{cmd} {}` failed: {e}", args.join(" "));
            false
        }
    }
}
