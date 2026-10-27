//! Layer 6: Model Context Protocol (MCP) server over stdio for autonomous AI agents.
//! Maps incoming agent requests directly to engine commands.

use serde_json::{Value, json};
use std::io::{BufRead, Write};
use template_engine::Session;

pub struct McpServer {
    pub session: Session,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            session: Session::new(),
        }
    }

    /// Runs the stdio JSON-RPC loop until EOF.
    pub fn run_stdio(&mut self) -> std::io::Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let request: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(e) => {
                    let err = json!({
                        "jsonrpc": "2.0",
                        "error": { "code": -32700, "message": format!("Parse error: {e}") },
                        "id": Value::Null
                    });
                    writeln!(stdout, "{}", serde_json::to_string(&err)?)?;
                    stdout.flush()?;
                    continue;
                }
            };

            let id = request.get("id").cloned().unwrap_or(Value::Null);
            let method = request.get("method").and_then(Value::as_str).unwrap_or("");

            let response = match method {
                "tools/list" => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [
                            {
                                "name": "execute_command",
                                "description": "Execute an engine command by ID with JSON params",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "command": { "type": "string" },
                                        "params": { "type": "object" }
                                    },
                                    "required": ["command"]
                                }
                            },
                            {
                                "name": "get_document",
                                "description": "Get current document state summary",
                                "inputSchema": { "type": "object" }
                            }
                        ]
                    }
                }),
                "tools/call" => {
                    let tool_name = request["params"]["name"].as_str().unwrap_or("");
                    let args = &request["params"]["arguments"];

                    match tool_name {
                        "execute_command" => {
                            let cmd = args["command"].as_str().unwrap_or("");
                            let params = &args["params"];
                            match self.session.execute(cmd, params) {
                                Ok(res) => json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "content": [{ "type": "text", "text": res.to_string() }] }
                                }),
                                Err(err) => json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "isError": true, "content": [{ "type": "text", "text": err }] }
                                }),
                            }
                        }
                        "get_document" => {
                            let summary = json!({
                                "name": self.session.doc.name,
                                "items_count": self.session.doc.len(),
                                "is_dirty": self.session.is_dirty(),
                            });
                            json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": { "content": [{ "type": "text", "text": summary.to_string() }] }
                            })
                        }
                        _ => json!({
                            "jsonrpc": "2.0",
                            "error": { "code": -32601, "message": "Method not found" },
                            "id": id
                        }),
                    }
                }
                _ => json!({
                    "jsonrpc": "2.0",
                    "error": { "code": -32601, "message": "Method not found" },
                    "id": id
                }),
            };

            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }

        Ok(())
    }
}
