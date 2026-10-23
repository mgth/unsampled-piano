//! The tools on a stand-in controller, and one exchange over HTTP.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use piano_core::params::PianoModel;
use piano_params::{PianoControl, Status, Value};
use serde_json::{json, Value as Json};

#[derive(Default)]
struct Stub {
    model: PianoModel,
    loads: u32,
}

impl PianoControl for Stub {
    fn model(&self) -> PianoModel {
        self.model
    }
    fn begin_edit(&mut self, _: &str) {}
    fn set(&mut self, path: &str, value: Value) {
        self.model = piano_params::set(&self.model, path, value).unwrap();
    }
    fn end_edit(&mut self, _: &str) {}
    fn load(&mut self, model: PianoModel) {
        self.model = model;
        self.loads += 1;
    }
    fn status(&self) -> Status {
        Status { voices: 3, ..Default::default() }
    }
}

fn presets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../presets")
}

fn server() -> (piano_mcp::Server<Stub>, Arc<Mutex<Stub>>) {
    let control = Arc::new(Mutex::new(Stub::default()));
    (piano_mcp::Server::new(control.clone(), Some(presets())), control)
}

/// A tool's structured result, or its error text.
fn call(server: &piano_mcp::Server<Stub>, tool: &str, args: Json) -> Result<Json, String> {
    let reply = server.handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": tool, "arguments": args } })).unwrap();
    let r = &reply["result"];
    if r["isError"] == true { Err(r["content"][0]["text"].as_str().unwrap().to_string()) } else { Ok(r["structuredContent"].clone()) }
}

#[test]
fn handshake_and_tool_list() {
    let (s, _) = server();
    let init = s.handle(&json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "t", "version": "0" } } })).unwrap();
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert!(init["result"]["capabilities"]["tools"].is_object());
    assert_eq!(s.handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })), None);
    let list = s.handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" })).unwrap();
    let names: Vec<&str> = list["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["list_params", "get", "set", "load_preset", "status"]);
    let presets = &list["result"]["tools"][3]["inputSchema"]["properties"]["name"]["enum"];
    assert!(presets.as_array().unwrap().contains(&json!("listening/Room/dry")), "{presets}");
    let unknown = s.handle(&json!({ "jsonrpc": "2.0", "id": 2, "method": "resources/list" })).unwrap();
    assert_eq!(unknown["error"]["code"], -32601);
}

#[test]
fn list_and_get() {
    let (s, _) = server();
    let damper = call(&s, "list_params", json!({ "group": "damper" })).unwrap();
    assert!(damper["count"].as_u64().unwrap() >= 5);
    assert!(damper["params"].as_array().unwrap().iter().all(|p| p["path"].as_str().unwrap().starts_with("damper.")));
    let gain = call(&s, "get", json!({ "path": "output_gain_db" })).unwrap();
    assert_eq!(gain["kind"], "live");
    assert_eq!(gain["unit"], "dB");
    assert_eq!(gain["range_kind"], "curated");
    assert!(gain["what"].as_str().unwrap().contains("level"));
    let e = call(&s, "get", json!({ "path": "fall_time" })).unwrap_err();
    assert!(e.contains("damper.fall_time"), "{e}");
    assert!(call(&s, "list_params", json!({ "filter": "zzz" })).unwrap_err().contains("groups"));
}

#[test]
fn set_live_and_rebuild() {
    let (s, control) = server();
    let r = call(&s, "set", json!({ "path": "output_gain_db", "value": -6 })).unwrap();
    assert_eq!(r["applied"], "immediately");
    assert_eq!(r["value"], -6.0);
    let r = call(&s, "set", json!({ "path": "a4", "value": 442 })).unwrap();
    assert_eq!(r["applied"], "instrument rebuilt");
    assert_eq!(r["previous"], 440.0);
    assert_eq!(piano_params::get(&control.lock().unwrap().model, "a4"), Some(Value::Float(442.0)));
    // Curated range: refused, model untouched.
    let e = call(&s, "set", json!({ "path": "output_gain_db", "value": 60 })).unwrap_err();
    assert!(e.contains("outside"), "{e}");
    assert_eq!(control.lock().unwrap().model.output_gain_db, -6.0);
    assert!(call(&s, "set", json!({ "path": "output_gain_db", "value": "loud" })).is_err());
}

#[test]
fn load_presets() {
    let (s, control) = server();
    call(&s, "set", json!({ "path": "room.size[0]", "value": 7.5 })).unwrap();
    let r = call(&s, "load_preset", json!({ "name": "honky_tonk" })).unwrap();
    assert_eq!(r["as"], "instrument");
    // An instrument keeps the listening set-up.
    assert_eq!(piano_params::get(&control.lock().unwrap().model, "room.size[0]"), Some(Value::Float(7.5)));
    let r = call(&s, "load_preset", json!({ "name": "listening/Room/dry" })).unwrap();
    assert_eq!(r["as"], "listening");
    assert_eq!(control.lock().unwrap().loads, 2);
    assert!(call(&s, "load_preset", json!({ "name": "nope" })).unwrap_err().contains("available"));
    let st = call(&s, "status", json!({})).unwrap();
    assert_eq!(st["voices"], 3);
}

#[test]
fn over_http() {
    let control = Arc::new(Mutex::new(Stub::default()));
    let url = piano_mcp::serve("127.0.0.1:0", control, None).unwrap();
    let addr = url.trim_start_matches("http://").trim_end_matches("/mcp").to_string();
    let post = |body: &str, origin: Option<&str>| {
        let mut tcp = std::net::TcpStream::connect(&addr).unwrap();
        let origin = origin.map_or(String::new(), |o| format!("Origin: {o}\r\n"));
        write!(
            tcp,
            "POST /mcp HTTP/1.1\r\nHost: {addr}\r\n{origin}Content-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        let mut reply = String::new();
        tcp.read_to_string(&mut reply).unwrap();
        reply
    };
    let r = post(r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"status","arguments":{}}}"#, None);
    assert!(r.starts_with("HTTP/1.1 200"), "{r}");
    assert!(r.contains("application/json") && r.contains(r#""id":7"#), "{r}");
    assert!(post(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#, None).starts_with("HTTP/1.1 202"));
    assert!(post("{}", Some("https://evil.example")).starts_with("HTTP/1.1 403"));
    assert!(post("not json", None).starts_with("HTTP/1.1 400"));
}
