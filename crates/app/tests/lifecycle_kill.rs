//! Kill -9 mid-edit then relaunch restores via MCP (persistent session process).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_drawcraft")
}

fn mcp_call(
    child_stdin: &mut impl Write,
    child_stdout: &mut impl BufRead,
    id: u64,
    tool: &str,
    args: Value,
) -> Value {
    let request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {"name": tool, "arguments": args}
    });
    writeln!(child_stdin, "{request}").unwrap();
    child_stdin.flush().unwrap();
    let mut line = String::new();
    child_stdout.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap_or_else(|error| panic!("bad mcp json: {error} / {line}"))
}

fn mcp_command(
    child_stdin: &mut impl Write,
    child_stdout: &mut impl BufRead,
    id: u64,
    command: &str,
    params: Value,
) -> Value {
    mcp_call(
        child_stdin,
        child_stdout,
        id,
        "drawcraft_command",
        json!({"id": command, "params": params}),
    )
}

fn spawn_mcp(file: &std::path::Path, xdg: &std::path::Path) -> std::process::Child {
    Command::new(bin())
        .args(["mcp", "--file", &file.display().to_string()])
        .env("XDG_CONFIG_HOME", xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn mcp: {error}"))
}

fn init_mcp(stdin: &mut impl Write, stdout: &mut impl BufRead) {
    let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    writeln!(stdin, "{init}").unwrap();
    stdin.flush().unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
}

fn seed_file(path: &std::path::Path) {
    let doc = drawcraft_document::Document::new();
    drawcraft_format::save(&doc, path).unwrap();
}

#[test]
fn kill_nine_then_restore_matches_precrash_document() {
    let directory = tempfile::tempdir().unwrap();
    let xdg = directory.path().join("xdg");
    std::fs::create_dir_all(&xdg).unwrap();
    let file = directory.path().join("board.drawcraft");
    seed_file(&file);

    // Edit + autosave, then SIGKILL.
    let expected_shapes;
    let expected_doc;
    {
        let mut child = spawn_mcp(&file, &xdg);
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        init_mcp(&mut stdin, &mut stdout);
        mcp_command(
            &mut stdin,
            &mut stdout,
            2,
            "shape.add",
            json!({"kind":"rectangle","x":0,"y":0,"width":20,"height":20}),
        );
        for i in 0..3 {
            mcp_command(
                &mut stdin,
                &mut stdout,
                10 + i,
                "shape.add",
                json!({"kind":"ellipse","x":i,"y":i,"width":8,"height":8}),
            );
        }
        mcp_command(
            &mut stdin,
            &mut stdout,
            20,
            "session.autosave_now",
            json!({}),
        );
        let info = mcp_command(&mut stdin, &mut stdout, 21, "doc.info", json!({}));
        let info_text = extract_tool_json(&info);
        expected_shapes = info_text.get("shapes").and_then(Value::as_u64).unwrap_or(4);
        let doc = mcp_call(&mut stdin, &mut stdout, 22, "drawcraft_document", json!({}));
        expected_doc = extract_tool_json(&doc);

        // SIGKILL on Unix.
        let _ = child.kill();
        let _ = child.wait();
        std::thread::sleep(Duration::from_millis(50));
    }

    // Relaunch and restore.
    let mut child = spawn_mcp(&file, &xdg);
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    init_mcp(&mut stdin, &mut stdout);
    let list = mcp_command(
        &mut stdin,
        &mut stdout,
        30,
        "session.recover_list",
        json!({}),
    );
    let list_text = extract_tool_json(&list);
    let candidates = list_text
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        candidates.len(),
        1,
        "expected one recovery candidate: {list_text}"
    );

    let restored = mcp_command(
        &mut stdin,
        &mut stdout,
        31,
        "session.recover",
        json!({"action": "restore"}),
    );
    let restored_text = extract_tool_json(&restored);
    assert_eq!(
        restored_text.get("shapes").and_then(Value::as_u64),
        Some(expected_shapes),
        "restore shapes mismatch: {restored_text}"
    );

    let doc = mcp_call(&mut stdin, &mut stdout, 32, "drawcraft_document", json!({}));
    let doc_text = extract_tool_json(&doc);
    let restored_document = doc_text.get("document").cloned().unwrap_or(doc_text);
    let expected_document = expected_doc
        .get("document")
        .cloned()
        .unwrap_or(expected_doc);
    assert_eq!(restored_document, expected_document);

    let info = mcp_command(&mut stdin, &mut stdout, 33, "doc.info", json!({}));
    let info_text = extract_tool_json(&info);
    assert_eq!(
        info_text.get("shapes").and_then(Value::as_u64),
        Some(expected_shapes)
    );
    assert_eq!(info_text.get("dirty").and_then(Value::as_bool), Some(true));

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn kill_nine_discard_removes_candidate() {
    let directory = tempfile::tempdir().unwrap();
    let xdg = directory.path().join("xdg");
    std::fs::create_dir_all(&xdg).unwrap();
    let file = directory.path().join("board.drawcraft");
    seed_file(&file);

    {
        let mut child = spawn_mcp(&file, &xdg);
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        init_mcp(&mut stdin, &mut stdout);
        mcp_command(
            &mut stdin,
            &mut stdout,
            2,
            "shape.add",
            json!({"kind":"rectangle","x":0,"y":0,"width":20,"height":20}),
        );
        mcp_command(&mut stdin, &mut stdout, 3, "file.save", json!({}));
        mcp_command(
            &mut stdin,
            &mut stdout,
            4,
            "shape.add",
            json!({"kind":"ellipse","x":1,"y":1,"width":5,"height":5}),
        );
        mcp_command(
            &mut stdin,
            &mut stdout,
            5,
            "session.autosave_now",
            json!({}),
        );
        let _ = child.kill();
        let _ = child.wait();
    }

    let mut child = spawn_mcp(&file, &xdg);
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    init_mcp(&mut stdin, &mut stdout);
    mcp_command(
        &mut stdin,
        &mut stdout,
        10,
        "session.recover",
        json!({"action": "discard"}),
    );
    let list = mcp_command(
        &mut stdin,
        &mut stdout,
        11,
        "session.recover_list",
        json!({}),
    );
    let list_text = extract_tool_json(&list);
    assert!(
        list_text
            .get("candidates")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty),
        "{list_text}"
    );
    let _ = child.kill();
    let _ = child.wait();
}

fn extract_tool_json(response: &Value) -> Value {
    if let Some(text) = response
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
    {
        if let Ok(value) = serde_json::from_str::<Value>(text) {
            return value;
        }
        // Sometimes the text is already a nested JSON string from the command result.
        return json!({"raw": text});
    }
    response.get("result").cloned().unwrap_or(Value::Null)
}
