//! Template headless CLI & MCP server entry point.

use serde_json::Value;
use std::env;
use template_engine::Session;
use template_mcp::McpServer;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("help");

    match subcommand {
        "mcp" => {
            let mut server = McpServer::new();
            if let Err(e) = server.run_stdio() {
                eprintln!("MCP server error: {e}");
                std::process::exit(1);
            }
        }
        "run" => {
            let mut session = Session::new();
            let cmd_arg = args.iter().find(|a| a.starts_with("--cmd=")).or_else(|| {
                args.windows(2)
                    .find(|w| w[0] == "--cmd")
                    .and_then(|w| w.get(1))
            });

            if let Some(arg) = cmd_arg {
                let stripped = arg.strip_prefix("--cmd=").unwrap_or(arg);
                let (cmd, json_str) = stripped.split_once('=').unwrap_or((stripped, "{}"));
                let params: Value =
                    serde_json::from_str(json_str).unwrap_or(Value::Object(Default::default()));

                match session.execute(cmd, &params) {
                    Ok(result) => println!("{result}"),
                    Err(e) => {
                        eprintln!("Error executing `{cmd}`: {e}");
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("Usage: template-cli run --cmd 'command.id={{...}}'");
                std::process::exit(1);
            }
        }
        "info" => {
            let session = Session::new();
            println!("TemplateApp Document Info:");
            println!("  Name: {}", session.doc.name);
            println!("  Items: {}", session.doc.len());
            println!("  Revision: {}", session.revision);
        }
        _ => {
            println!("TemplateApp CLI");
            println!("Usage:");
            println!("  template-cli run --cmd 'command.id={{\"...\"}}'  Execute engine command");
            println!(
                "  template-cli mcp                               Run Model Context Protocol server over stdio"
            );
            println!(
                "  template-cli info                              Display active document info"
            );
        }
    }
}
