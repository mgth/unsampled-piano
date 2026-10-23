//! MCP server (Model Context Protocol) of the standalone player: an AI
//! assistant lists, reads and sets the parameters of the instrument that is
//! playing and loads presets, through the same [`PianoControl`] as the
//! interface. Live parameters change the sound at once; the others rebuild
//! the instrument in the background without cutting the held notes.
//!
//! Transport: MCP "streamable HTTP", reduced to what tools need. Each POST
//! to `/mcp` carries one JSON-RPC message and gets a JSON reply (202 with no
//! body for a notification); no event stream, no session. The server only
//! listens on the address it is given (loopback by default) and refuses
//! browser requests from other origins (DNS rebinding). Requests are handled
//! one at a time on their own thread; the controller's lock is released
//! while waiting for a rebuild.

mod tools;

use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use piano_params::{PianoControl, Registry};
use serde_json::{json, Value};

/// Address used by `--mcp` without a value.
pub const DEFAULT_ADDR: &str = "127.0.0.1:7426";

/// Protocol revisions spoken, newest first: tools and text results are the
/// same in all of them (older clients ignore the structured result).
const PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// Largest request body read (a tool call is a few hundred bytes).
const MAX_BODY: u64 = 1 << 20;

const INSTRUCTIONS: &str = "Unsampled Piano: a physically modelled piano (felt hammers, stiff strings, \
soundboard, microphones, room) playing live from the user's MIDI keyboard. The user hears every change \
at once. Parameters are named by their path in the preset (\"damper.fall_time\", \"room.size[0]\"). \
Live parameters apply immediately; the others rebuild the instrument in about 0.1 s while held notes keep \
sounding. Start with list_params (filter by word or group), read get for what a parameter does physically \
before changing it, change one thing at a time and tell the user what to listen for. Instrument presets \
keep the listening set-up (microphones, lid, room); listening presets keep the instrument.";

/// Starts the server on `addr` ("127.0.0.1:7426") on its own thread;
/// returns the endpoint's URL. `presets`: the preset folder offered by
/// `load_preset` (same layout as the interface's).
pub fn serve<C: PianoControl + Send + 'static>(addr: &str, control: Arc<Mutex<C>>, presets: Option<PathBuf>) -> Result<String, String> {
    let http = tiny_http::Server::http(addr).map_err(|e| format!("MCP server on {addr}: {e}"))?;
    let url = match http.server_addr().to_ip() {
        Some(a) => format!("http://{a}/mcp"),
        None => format!("http://{addr}/mcp"),
    };
    let server = Server::new(control, presets);
    std::thread::Builder::new()
        .name("piano-mcp".into())
        .spawn(move || {
            for request in http.incoming_requests() {
                server.respond(request);
            }
        })
        .map_err(|e| format!("MCP server thread: {e}"))?;
    Ok(url)
}

pub struct Server<C> {
    control: Arc<Mutex<C>>,
    presets: Option<PathBuf>,
    registry: Registry,
}

impl<C: PianoControl> Server<C> {
    pub fn new(control: Arc<Mutex<C>>, presets: Option<PathBuf>) -> Self {
        Self { control, presets, registry: Registry::new() }
    }

    /// One JSON-RPC message from the client; the reply, None for a
    /// notification or a response.
    pub fn handle(&self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let Some(method) = msg.get("method").and_then(Value::as_str) else {
            // A response to a request of ours (we send none), or garbage.
            return match (id, msg.get("result").or(msg.get("error"))) {
                (_, Some(_)) => None,
                (id, None) => Some(error(id.unwrap_or(Value::Null), -32600, "invalid request")),
            };
        };
        let id = id?;
        let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
        let result = match method {
            "initialize" => Ok(initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools::list(self) })),
            "tools/call" => tools::call(self, &params),
            _ => Err((-32601, format!("method not found: {method}"))),
        };
        Some(match result {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => error(id, code, &message),
        })
    }

    fn respond(&self, mut request: tiny_http::Request) {
        let reply = |code: u16, body: Option<Value>| {
            let text = body.map_or(String::new(), |b| b.to_string());
            let mut r = tiny_http::Response::from_string(text).with_status_code(code);
            if code == 200 {
                r.add_header(header("Content-Type", "application/json"));
            }
            if code == 405 {
                r.add_header(header("Allow", "POST"));
            }
            r
        };
        let path = request.url().split('?').next().unwrap_or("");
        let response = if path != "/mcp" {
            reply(404, None)
        } else if !origin_allowed(request.headers()) {
            reply(403, None)
        } else if *request.method() != tiny_http::Method::Post {
            // No server-initiated stream (GET), no session to end (DELETE).
            reply(405, None)
        } else {
            let mut body = String::new();
            match request.as_reader().take(MAX_BODY).read_to_string(&mut body) {
                Err(_) => reply(400, Some(error(Value::Null, -32700, "unreadable body"))),
                Ok(_) => match serde_json::from_str::<Value>(&body) {
                    Err(e) => reply(400, Some(error(Value::Null, -32700, &format!("parse error: {e}")))),
                    Ok(Value::Array(_)) => reply(400, Some(error(Value::Null, -32600, "batches are not supported"))),
                    Ok(msg) => match self.handle(&msg) {
                        Some(r) => reply(200, Some(r)),
                        None => reply(202, None),
                    },
                },
            }
        };
        let _ = request.respond(response);
    }
}

fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = PROTOCOL_VERSIONS.iter().find(|v| Some(**v) == asked).unwrap_or(&PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "unsampled-piano", "title": "Unsampled Piano", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn header(name: &str, value: &str) -> tiny_http::Header {
    tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("valid header")
}

/// No Origin (not a browser), or a page served from this machine.
fn origin_allowed(headers: &[tiny_http::Header]) -> bool {
    let Some(origin) = headers.iter().find(|h| h.field.equiv("Origin")) else { return true };
    let origin = origin.value.as_str();
    let host = origin.split_once("://").map_or("", |(_, rest)| rest);
    let host = match host.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => host.split(':').next().unwrap_or(""),
    };
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(name: &str, value: &str) -> Vec<tiny_http::Header> {
        vec![header(name, value)]
    }

    #[test]
    fn origins() {
        assert!(origin_allowed(&[]));
        assert!(origin_allowed(&h("Origin", "http://localhost:5173")));
        assert!(origin_allowed(&h("Origin", "http://127.0.0.1")));
        assert!(origin_allowed(&h("Origin", "http://[::1]:8080")));
        assert!(!origin_allowed(&h("Origin", "https://evil.example")));
        assert!(!origin_allowed(&h("Origin", "http://localhost.evil.example")));
        assert!(!origin_allowed(&h("Origin", "null")));
    }

    #[test]
    fn protocol_version() {
        assert_eq!(initialize(&json!({ "protocolVersion": "2025-03-26" }))["protocolVersion"], "2025-03-26");
        assert_eq!(initialize(&json!({ "protocolVersion": "2023-01-01" }))["protocolVersion"], PROTOCOL_VERSIONS[0]);
    }
}
