//! Model Context Protocol server for DrawCraft.
//!
//! Line-delimited JSON-RPC 2.0 over stdio. The server is intentionally thin:
//! every tool ultimately calls [`drawcraft_session::Session::execute`], the same
//! path used by the UI and CLI. Notifications receive no response; protocol and
//! tool errors are reported in the standard JSON-RPC/MCP shapes.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::field_reassign_with_default
    )
)]

use std::io::{BufRead, Write};

use base64::Engine as _;
use drawcraft_command::{ParamKind, Registry, Scope, ids};
use drawcraft_session::Session;
use serde_json::{Value, json};
use thiserror::Error;

/// Maximum accepted JSON-RPC line, to keep a misbehaving client bounded.
pub const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

/// MCP transport errors.
#[derive(Debug, Error)]
pub enum McpError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, McpError>;

/// Handle one JSON-RPC line. Returns `None` for notifications.
#[must_use]
pub fn handle_line(session: &mut Session, line: &str) -> Option<Value> {
    match serde_json::from_str::<Value>(line) {
        Ok(request) => handle(session, &request),
        Err(error) => Some(json!({
            "jsonrpc": "2.0",
            "id": Value::Null,
            "error": {"code": -32700, "message": format!("parse error: {error}")}
        })),
    }
}

/// Handle one parsed JSON-RPC request or notification.
#[must_use]
pub fn handle(session: &mut Session, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    let outcome: std::result::Result<Value, RpcError> = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}, "resources": {}},
            "serverInfo": {"name": "drawcraft", "version": env!("CARGO_PKG_VERSION")}
        })),
        "notifications/initialized" | "notifications/cancelled" => return None,
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools()})),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            Ok(call_tool(session, &name, &arguments))
        }
        "resources/list" => Ok(json!({"resources": resources()})),
        "resources/read" => {
            let uri = params
                .get("uri")
                .and_then(Value::as_str)
                .unwrap_or_default();
            read_resource(session, uri)
                .map(|text| {
                    json!({"contents": [{"uri": uri, "mimeType": "application/json", "text": text}]})
                })
                .map_err(RpcError::invalid_params)
        }
        "prompts/list" => Ok(json!({"prompts": []})),
        "shutdown" => Ok(Value::Null),
        other => Err(RpcError::method_not_found(other)),
    };

    let id = id?;
    Some(match outcome {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": error.code, "message": error.message}
        }),
    })
}

/// Run the stdio server until EOF.
pub fn serve_stdio(session: &mut Session) -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if line.len() > MAX_LINE_BYTES {
            let response = json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": {"code": -32600, "message": "request exceeds the 8 MiB line limit"}
            });
            serde_json::to_writer(&mut stdout, &response)?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
            continue;
        }
        if let Some(response) = handle_line(session, &line) {
            serde_json::to_writer(&mut stdout, &response)?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

#[derive(Debug)]
struct RpcError {
    code: i64,
    message: String,
}

impl RpcError {
    fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("method not found: {method}"),
        }
    }

    fn invalid_params(message: String) -> Self {
        Self {
            code: -32602,
            message,
        }
    }
}

fn tools() -> Vec<Value> {
    let command_ids: Vec<&str> = Registry::new()
        .all()
        .iter()
        .map(|command| command.id)
        .collect();
    vec![
        json!({
            "name": "drawcraft_command",
            "description": "Run any DrawCraft command. Use drawcraft_list_commands to discover ids.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {"type": "string", "enum": command_ids},
                    "params": {"type": "object", "default": {}}
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "drawcraft_list_commands",
            "description": "List every command with its parameters, category, and scope.",
            "inputSchema": {"type": "object", "properties": {}}
        }),
        json!({
            "name": "drawcraft_document",
            "description": "Return the current document as JSON plus session info.",
            "inputSchema": {"type": "object", "properties": {}}
        }),
        json!({
            "name": "drawcraft_open",
            "description": "Open a .drawcraft document from disk.",
            "inputSchema": {
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            }
        }),
        json!({
            "name": "drawcraft_save",
            "description": "Save the current document, optionally to a new path.",
            "inputSchema": {
                "type": "object",
                "properties": {"path": {"type": "string"}}
            }
        }),
        json!({
            "name": "drawcraft_export",
            "description": "Export the page or selection as SVG or PNG.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "format": {"type": "string", "enum": ["svg", "png"]},
                    "scope": {"type": "string", "enum": ["page", "selection"]},
                    "scale": {"type": "number", "default": 1.0},
                    "transparent": {"type": "boolean", "default": false}
                },
                "required": ["path", "format"]
            }
        }),
        json!({
            "name": "drawcraft_render_preview",
            "description": "Render the active page to a base64 PNG image (or write it to path).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "scale": {"type": "number", "default": 1.0},
                    "transparent": {"type": "boolean", "default": false}
                }
            }
        }),
    ]
}

fn resources() -> Vec<Value> {
    vec![
        json!({
            "uri": "drawcraft://document",
            "name": "Current document",
            "description": "The active .drawcraft document as JSON.",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "drawcraft://commands",
            "name": "Command catalog",
            "description": "All command ids and parameter schemas.",
            "mimeType": "application/json"
        }),
    ]
}

fn read_resource(session: &Session, uri: &str) -> std::result::Result<String, String> {
    match uri {
        "drawcraft://document" => {
            serde_json::to_string_pretty(&session.editor.doc).map_err(|error| error.to_string())
        }
        "drawcraft://commands" => {
            serde_json::to_string_pretty(&command_catalog()).map_err(|error| error.to_string())
        }
        other => Err(format!("unknown resource: {other}")),
    }
}

fn command_catalog() -> Vec<Value> {
    Registry::new()
        .all()
        .iter()
        .map(|command| {
            json!({
                "id": command.id,
                "label": command.label,
                "description": command.description,
                "category": command.category.as_str(),
                "scope": match command.scope {
                    Scope::Editor => "editor",
                    Scope::Session => "session",
                },
                "params": command.params.iter().map(|param| json!({
                    "name": param.name,
                    "label": param.label,
                    "type": param.kind.json_type(),
                    "required": param.required,
                    "description": param.description,
                    "values": match param.kind {
                        ParamKind::Enum(values) => Some(values),
                        _ => None,
                    }
                })).collect::<Vec<_>>()
            })
        })
        .collect()
}

fn call_tool(session: &mut Session, name: &str, arguments: &Value) -> Value {
    match name {
        "drawcraft_command" => {
            let id = arguments
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let params = arguments
                .get("params")
                .cloned()
                .unwrap_or_else(|| json!({}));
            tool_result(
                session
                    .execute(id, &params)
                    .map_err(|error| error.to_string()),
            )
        }
        "drawcraft_list_commands" => tool_result(Ok(json!(command_catalog()))),
        "drawcraft_document" => tool_result(
            serde_json::to_value(&session.editor.doc)
                .map(|document| json!({"document": document, "info": session.info()}))
                .map_err(|error| error.to_string()),
        ),
        "drawcraft_open" => {
            let path = arguments
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            tool_result(
                session
                    .execute(ids::FILE_OPEN, &json!({"path": path}))
                    .map_err(|error| error.to_string()),
            )
        }
        "drawcraft_save" => {
            let mut params = json!({});
            if let Some(path) = arguments.get("path").and_then(Value::as_str) {
                params = json!({"path": path});
            }
            tool_result(
                session
                    .execute(ids::FILE_SAVE, &params)
                    .map_err(|error| error.to_string()),
            )
        }
        "drawcraft_export" => {
            let path = arguments
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let format = arguments
                .get("format")
                .and_then(Value::as_str)
                .unwrap_or("svg");
            let scope = arguments
                .get("scope")
                .and_then(Value::as_str)
                .unwrap_or("page");
            let params = json!({
                "path": path,
                "scope": scope,
                "scale": arguments.get("scale").and_then(Value::as_f64).unwrap_or(1.0),
                "transparent": arguments.get("transparent").and_then(Value::as_bool).unwrap_or(false),
            });
            let command = if format == "png" {
                ids::FILE_EXPORT_PNG
            } else {
                ids::FILE_EXPORT_SVG
            };
            tool_result(
                session
                    .execute(command, &params)
                    .map_err(|error| error.to_string()),
            )
        }
        "drawcraft_render_preview" => {
            let scale = arguments
                .get("scale")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let transparent = arguments
                .get("transparent")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            render_preview(session, scale, transparent, arguments)
        }
        other => tool_error(format!("unknown tool: {other}")),
    }
}

fn render_preview(session: &Session, scale: f64, transparent: bool, arguments: &Value) -> Value {
    let selection = session.selection_ids();
    let scope = if selection.is_empty() {
        drawcraft_format::ExportScope::Page
    } else {
        drawcraft_format::ExportScope::Selection
    };
    let bytes = match drawcraft_format::export_png_bytes(
        &session.editor.doc,
        scope,
        &selection,
        scale,
        transparent,
    ) {
        Ok(bytes) => bytes,
        Err(error) => return tool_error(error.to_string()),
    };
    if let Some(path) = arguments.get("path").and_then(Value::as_str)
        && let Err(error) = std::fs::write(path, &bytes)
    {
        return tool_error(error.to_string());
    }
    let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
    json!({
        "content": [
            {"type": "image", "data": data, "mimeType": "image/png"}
        ]
    })
}

fn tool_result(result: std::result::Result<Value, String>) -> Value {
    match result {
        Ok(value) => json!({
            "content": [{"type": "text", "text": value.to_string()}]
        }),
        Err(error) => tool_error(error),
    }
}

fn tool_error(message: impl Into<String>) -> Value {
    json!({
        "content": [{"type": "text", "text": message.into()}],
        "isError": true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session::new()
    }

    #[test]
    fn initialize_and_list_tools() {
        let mut session = session();
        let response = handle_line(
            &mut session,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        )
        .unwrap_or(Value::Null);
        assert_eq!(response.get("id").and_then(Value::as_u64), Some(1));
        assert!(response.get("result").is_some());

        let response = handle_line(
            &mut session,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        )
        .unwrap_or(Value::Null);
        let tools = response
            .get("result")
            .and_then(|result| result.get("tools"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            tools
                .iter()
                .any(|tool| tool.get("name") == Some(&json!("drawcraft_command")))
        );
    }

    #[test]
    fn notifications_get_no_response() {
        let mut session = session();
        assert!(
            handle_line(
                &mut session,
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
            )
            .is_none()
        );
    }

    #[test]
    fn parse_errors_are_reported() {
        let mut session = session();
        let response = handle_line(&mut session, "{not json").unwrap_or(Value::Null);
        assert_eq!(
            response
                .get("error")
                .and_then(|error| error.get("code"))
                .and_then(Value::as_i64),
            Some(-32700)
        );
    }

    #[test]
    fn command_tool_adds_shape() {
        let mut session = session();
        let response = handle_line(
            &mut session,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"shape.add","params":{"kind":"rectangle","x":0,"y":0,"width":10,"height":10}}}}"#,
        )
        .unwrap_or(Value::Null);
        assert!(response.get("result").is_some());
        assert_eq!(session.editor.doc.shapes().len(), 1);
    }

    #[test]
    fn resource_read_returns_document() {
        let mut session = session();
        let response = handle_line(
            &mut session,
            r#"{"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"drawcraft://document"}}"#,
        )
        .unwrap_or(Value::Null);
        assert!(response.get("result").is_some());
    }
}
