//! A minimal in-process mock Soroban RPC server for integration tests.
//!
//! Serves `getLatestLedger`, `getNetwork`, and `getLedgerEntries` from fixture
//! files plus a programmatically-built entry map. Real HTTP, no extra
//! dependencies — a bare `tokio::net::TcpListener` speaking HTTP/1.1.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A mock Soroban RPC server bound to an ephemeral local port.
#[derive(Debug, Clone)]
pub struct MockRpc {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    latest_ledger: String,
    network: String,
    /// base64 LedgerKey -> getLedgerEntries entry object (JSON string).
    entries: HashMap<String, String>,
    /// Latest ledger sequence reported by getLedgerEntries.
    latest_ledger_seq: u32,
}

impl MockRpc {
    /// Create a server preloaded with the fixture responses.
    pub fn from_fixtures(fixtures: &str) -> Self {
        let latest_ledger = std::fs::read_to_string(format!("{fixtures}/getLatestLedger.json"))
            .expect("getLatestLedger.json fixture");
        let network = std::fs::read_to_string(format!("{fixtures}/getNetwork.json"))
            .expect("getNetwork.json fixture");
        Self {
            inner: Arc::new(Inner {
                latest_ledger,
                network,
                entries: HashMap::new(),
                latest_ledger_seq: 4_566_959,
            }),
        }
    }

    /// Register a synthetic entry (base64 `LedgerKey` -> entry JSON object).
    pub fn with_entry(mut self, key_b64: String, entry_json: String) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("no clones yet")
            .entries
            .insert(key_b64, entry_json);
        self
    }

    /// Register the real config-setting fixture response.
    pub fn with_config_fixture(mut self, fixtures: &str) -> Self {
        let raw = std::fs::read_to_string(format!("{fixtures}/getLedgerEntries_config.json"))
            .expect("getLedgerEntries_config.json fixture");
        let parsed: serde_json::Value =
            serde_json::from_str(&raw).expect("valid config fixture JSON");
        for entry in parsed["result"]["entries"].as_array().expect("entries") {
            let key = entry["key"].as_str().expect("key").to_string();
            let entry = entry.to_string();
            Arc::get_mut(&mut self.inner)
                .expect("no clones yet")
                .entries
                .insert(key, entry);
        }
        self
    }

    /// Start the server and return the base URL.
    pub async fn start(self) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let inner = self.inner;
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    continue;
                };
                let inner = inner.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 64 * 1024];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    let request = String::from_utf8_lossy(&buf[..n]).to_string();
                    let body = request_body(&request);
                    let response = handle(inner, body);
                    let _ = socket.write_all(response.as_bytes()).await;
                });
            }
        });
        format!("http://{addr}")
    }
}

/// Extract the JSON body from a raw HTTP/1.1 request.
fn request_body(request: &str) -> String {
    if let Some(idx) = request.find("\r\n\r\n") {
        request[idx + 4..].trim_end().to_string()
    } else {
        String::new()
    }
}

/// Route one JSON-RPC request and produce an HTTP response.
fn handle(inner: Arc<Inner>, body: String) -> String {
    let response = match route(&inner, &body) {
        Ok(json) => json,
        Err(err) => {
            let code = serde_json::json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32603, "message": err }
            });
            code.to_string()
        }
    };
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response.len(),
        response
    )
}

fn route(inner: &Arc<Inner>, body: &str) -> Result<String, String> {
    let req: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("bad JSON-RPC request: {e}"))?;
    let method = req["method"]
        .as_str()
        .ok_or_else(|| "missing method".to_string())?;
    let id = req["id"].clone();

    let result = match method {
        "getLatestLedger" => serde_json::from_str::<serde_json::Value>(&inner.latest_ledger)
            .map_err(|e| e.to_string())?
            .get("result")
            .cloned()
            .ok_or_else(|| "latest ledger fixture missing result".to_string())?,
        "getNetwork" => serde_json::from_str::<serde_json::Value>(&inner.network)
            .map_err(|e| e.to_string())?
            .get("result")
            .cloned()
            .ok_or_else(|| "network fixture missing result".to_string())?,
        "getLedgerEntries" => {
            let keys = req["params"]["keys"]
                .as_array()
                .ok_or_else(|| "getLedgerEntries missing keys".to_string())?;
            let mut entries = Vec::new();
            for key in keys {
                let key = key.as_str().ok_or_else(|| "key not a string".to_string())?;
                if let Some(entry) = inner.entries.get(key) {
                    let entry: serde_json::Value =
                        serde_json::from_str(entry).map_err(|e| e.to_string())?;
                    entries.push(entry);
                }
            }
            serde_json::json!({
                "latestLedger": inner.latest_ledger_seq,
                "entries": entries,
            })
        }
        other => return Err(format!("unexpected method: {other}")),
    };

    Ok(serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
    .to_string())
}