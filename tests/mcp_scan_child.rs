use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

#[test]
fn served_sniff_reports_findings_from_the_scan_child() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("alpha.py"), "import beta\n").unwrap();
    fs::write(tmp.path().join("beta.py"), "import alpha\n").unwrap();
    let mut server = Command::new(env!("CARGO_BIN_EXE_sensez"))
        .args(["mcp", "serve"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = server.stdin.take().unwrap();
    let mut stdout = BufReader::new(server.stdout.take().unwrap());
    let call = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "noze_sniff", "arguments": {"path": tmp.path(), "diff": false}},
    });
    writeln!(stdin, "{call}").unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    drop(stdin);
    let mut stderr = String::new();
    server
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    server.wait().unwrap();

    let response: Value = serde_json::from_str(&line).unwrap();
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let report: Value = serde_json::from_str(text).unwrap();
    assert_eq!(response["result"]["isError"], false);
    assert_eq!(report["cycles"].as_array().unwrap().len(), 1, "{text}");
    assert!(!stderr.contains("scanning in-process"), "{stderr}");
}
