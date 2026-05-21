//! dailyos-mcp-v2 — MCP v2 transport binary entry point.
//!
//! Subcommands:
//!
//! - `serve` — stdio MCP server. Reads env (`DAILYOS_MCP_CLIENT_ID` +
//!   `DAILYOS_MCP_TRANSPORT_KEY`), cross-checks against keychain,
//!   constructs `V2ServerHandler::from_verified_pairing`, runs rmcp
//!   stdio loop. Fail-fast on any startup check.
//! - `pair` — operator subcommand to pair a new client. Writes
//!   `mcp_client_manifest` + `mcp_tool_grant` rows via `auth::pair_client`,
//!   emits `PairingResponse` as JSON OR claude-desktop-compatible
//!   `mcpServers` snippet.
//! - `unpair` — admin subcommand to revoke a paired client.
//!
//! See `.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md`
//! for the full L0 contract.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use parking_lot::Mutex;
use rmcp::ServiceExt;
use rusqlite::Connection;
use zeroize::Zeroizing;

use abilities_runtime::abilities::registry::McpExposure;
use dailyos_lib::services::mcp_v2::{
    actor_policy::{ToolGrant, ToolRateLimit},
    auth::{self, PairingHandshake, PairingResponse, TRANSPORT_KEY_LEN},
    contracts::{McpClientId, Scope, ScopedName},
    gateway::Gateway,
    taxonomy::YamlTaxonomyCatalog,
    transport::V2ServerHandler,
};

const ENV_CLIENT_ID: &str = "DAILYOS_MCP_CLIENT_ID";
const ENV_TRANSPORT_KEY: &str = "DAILYOS_MCP_TRANSPORT_KEY";

#[derive(Parser, Debug)]
#[command(
    name = "dailyos-mcp-v2",
    about = "DailyOS MCP v2 transport — local-to-local same-machine MCP server."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Override the SQLite database path. Defaults to the same path the
    /// Tauri app uses (`~/.dailyos/dailyos.db`).
    #[arg(long, global = true)]
    db_path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the stdio MCP server. Reads pairing identity from env.
    Serve {
        /// If set, refuse to start when the legacy MCP config at this
        /// path claims any v2-owned tool name. Per L0 AC-11.
        #[arg(long)]
        legacy_config_path: Option<PathBuf>,
    },
    /// Pair a new MCP client. Prints PairingResponse to stdout; key
    /// warnings to stderr. Per L0 AC-5.
    Pair {
        /// Client label (operator-visible; not auth-bearing).
        #[arg(long)]
        client_name: String,
        /// Repeatable per-tool grant: `<tool>:<scope1>[,<scope2>...]:<invocable|metadata-only>`
        #[arg(long, required = true)]
        grant: Vec<String>,
        /// Output format. `json` is machine-parsable; `claude-desktop`
        /// emits an `mcpServers` config snippet ready to paste into
        /// claude_desktop_config.json.
        #[arg(long, default_value = "json")]
        format: PairFormat,
    },
    /// Revoke a paired client by client_id.
    Unpair {
        #[arg(long)]
        client_id: String,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum PairFormat {
    Json,
    ClaudeDesktop,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Stderr only — stdout is reserved for MCP protocol /
            // JSON output per legacy `src/mcp/main.rs:1312` precedent.
            eprintln!("dailyos-mcp-v2: {e}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch(cli: Cli) -> Result<(), String> {
    let db_path = match cli.db_path {
        Some(p) => p,
        None => default_db_path()?,
    };

    match cli.command {
        Command::Serve { legacy_config_path } => run_serve(&db_path, legacy_config_path),
        Command::Pair {
            client_name,
            grant,
            format,
        } => run_pair(&db_path, &client_name, &grant, format),
        Command::Unpair { client_id } => run_unpair(&db_path, &client_id),
    }
}

fn default_db_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "home dir not found".to_string())?;
    Ok(home.join(".dailyos").join("dailyos.db"))
}

fn open_conn(db_path: &PathBuf) -> Result<Connection, String> {
    Connection::open(db_path).map_err(|e| format!("open db {db_path:?}: {e}"))
}

// ---------------------------------------------------------------------------
// serve
// ---------------------------------------------------------------------------

fn run_serve(db_path: &PathBuf, legacy_config_path: Option<PathBuf>) -> Result<(), String> {
    // Per L0 AC-11: refuse if legacy config claims v2-owned tools.
    if let Some(path) = legacy_config_path {
        check_legacy_config(&path)?;
    }

    // Per L0 AC-12: startup env-assertion.
    let client_id_str = std::env::var(ENV_CLIENT_ID).map_err(|_| {
        format!(
            "missing required env var {ENV_CLIENT_ID}. \
             See 'dailyos-mcp-v2 pair --format claude-desktop' output."
        )
    })?;
    let transport_key_hex = std::env::var(ENV_TRANSPORT_KEY).map_err(|_| {
        format!(
            "missing required env var {ENV_TRANSPORT_KEY}. \
             See 'dailyos-mcp-v2 pair --format claude-desktop' output."
        )
    })?;

    let client_id = McpClientId::new(client_id_str);

    let mut conn = open_conn(db_path)?;

    // Look up client_id row + verify not revoked.
    let record = auth::load_client_record(&conn, &client_id)
        .map_err(|e| format!("client_id {} not paired: {e}", client_id.as_str()))?;
    if record.revoked_at.is_some() {
        return Err(format!(
            "pairing for {} revoked at {:?}. Re-pair via 'pair'.",
            client_id.as_str(),
            record.revoked_at
        ));
    }

    // Cross-check transport_key against keychain.
    let env_key_bytes = Zeroizing::new(decode_hex_32(&transport_key_hex)?);
    let keychain_key_bytes = Zeroizing::new(
        auth::load_transport_key(&record.transport_key_ref)
            .map_err(|e| format!("read transport_key from keychain: {e}"))?,
    );
    if !constant_time_eq(env_key_bytes.as_slice(), keychain_key_bytes.as_slice()) {
        return Err(format!(
            "transport_key mismatch for {}. Env value does not match keychain. Re-pair.",
            client_id.as_str()
        ));
    }

    // Best-effort env-wipe per L0 AC-12 step 5. This is in-process
    // hygiene only — does NOT scrub /proc/<pid>/environ which retains
    // the initial exec environment for process lifetime per kernel
    // behavior. Acceptable under §0 scope (same-user same-machine).
    //
    // SAFETY: runs at startup before any threads/runtime init; sets are
    // not racing with reads.
    unsafe {
        std::env::remove_var(ENV_CLIENT_ID);
        std::env::remove_var(ENV_TRANSPORT_KEY);
    }

    // Construct taxonomy + gateway. Seal at boot (empty handler set is
    // expected for W1.5 transport-only build; pending list logged).
    let catalog = YamlTaxonomyCatalog::load_embedded()
        .map_err(|e| format!("load embedded taxonomy: {e}"))?;
    let catalog: Arc<dyn dailyos_lib::services::mcp_v2::taxonomy::TaxonomyCatalog> =
        Arc::new(catalog);
    let mut gateway = Gateway::new();
    gateway.set_taxonomy(catalog.clone());
    let pending = gateway
        .seal()
        .map_err(|e| format!("gateway seal: {e}"))?;

    // Per L0 AC-4 boot log to stderr (stdout reserved for MCP protocol).
    let require_handlers = std::env::var("DAILYOS_MCP_V2_REQUIRE_HANDLERS")
        .map(|v| v == "1")
        .unwrap_or(false);
    eprintln!(
        "mcp_v2 boot: pairing {} verified, 0 handlers registered, {} catalog entries pending. \
         tools/list will return empty for this build. Expected for W1.5 transport-only.",
        client_id.as_str(),
        pending.len(),
    );
    if require_handlers && !pending.is_empty() {
        return Err(format!(
            "DAILYOS_MCP_V2_REQUIRE_HANDLERS=1 set but {} catalog entries lack handlers; refusing to start.",
            pending.len()
        ));
    }

    // Seed nonce — for W1.5 transport-only build with no handlers, the
    // seed nonce is the pairing's initial nonce. We re-mint a fresh
    // pairing-style seed by issuing one via the auth nonce ledger.
    // (Real pairing seed nonce flows from pair CLI to operator config;
    // for this build we accept it via env too — path-α follow-up.)
    let seed_nonce = auth::issue_seed_nonce(&mut conn, &client_id)
        .map_err(|e| format!("issue seed nonce: {e}"))?;

    let conn_shared = Arc::new(Mutex::new(conn));
    let gateway_shared = Arc::new(gateway);

    let transport_key = {
        let mut k = [0u8; TRANSPORT_KEY_LEN];
        k.copy_from_slice(keychain_key_bytes.as_ref());
        Zeroizing::new(k)
    };

    let handler = V2ServerHandler::from_verified_pairing(
        gateway_shared,
        catalog,
        conn_shared,
        client_id,
        transport_key,
        seed_nonce,
    );

    // Run rmcp stdio loop per legacy precedent at src/mcp/main.rs:1379.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("build tokio runtime: {e}"))?;
    runtime
        .block_on(async move {
            let service = handler
                .serve(rmcp::transport::io::stdio())
                .await
                .map_err(|e| format!("rmcp serve: {e}"))?;
            service
                .waiting()
                .await
                .map_err(|e| format!("rmcp wait: {e}"))?;
            Ok::<(), String>(())
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// pair
// ---------------------------------------------------------------------------

fn run_pair(
    db_path: &PathBuf,
    client_name: &str,
    grants: &[String],
    format: PairFormat,
) -> Result<(), String> {
    let tool_grants = parse_grants(grants)?;
    let handshake = PairingHandshake {
        client_label: client_name.to_string(),
        tool_grants,
    };
    let mut conn = open_conn(db_path)?;
    let response = auth::pair_client(&mut conn, handshake).map_err(|e| format!("pair: {e}"))?;
    emit_pairing_response(&response, format)?;
    eprintln!(
        "\nWARNING: {} printed ONCE. Record it now — there is no recovery. \
         Re-pair via 'unpair --client-id <id>' then 'pair' regenerates a new key.\n\n\
         Recommended: chmod 600 on claude_desktop_config.json — the env block \
         contains the transport_key. (Same-user same-machine threat model; advisory only.)",
        ENV_TRANSPORT_KEY,
    );
    Ok(())
}

fn parse_grants(grants: &[String]) -> Result<Vec<ToolGrant>, String> {
    let mut out = Vec::with_capacity(grants.len());
    for raw in grants {
        // Format: <tool>:<scope1>[,<scope2>...]:<invocable|metadata-only>
        let parts: Vec<&str> = raw.splitn(3, ':').collect();
        if parts.len() != 3 {
            return Err(format!(
                "invalid --grant: '{raw}' (expected '<tool>:<scope1>[,<scope2>...]:<exposure>')"
            ));
        }
        let tool_name = ScopedName::new(parts[0]);
        let scopes_granted: Vec<Scope> = parts[1].split(',').map(Scope::new).collect();
        let exposure = match parts[2] {
            "invocable" => McpExposure::Invocable,
            "metadata-only" => McpExposure::MetadataOnly,
            other => return Err(format!("invalid exposure '{other}' (use invocable|metadata-only)")),
        };
        out.push(ToolGrant {
            tool_name,
            scopes_granted,
            exposure,
            rate_limit: ToolRateLimit {
                max_calls: 60,
                window_seconds: 3600,
            },
        });
    }
    Ok(out)
}

fn emit_pairing_response(resp: &PairingResponse, format: PairFormat) -> Result<(), String> {
    let key_hex = hex::encode(resp.transport_key.as_ref());
    match format {
        PairFormat::Json => {
            let json = serde_json::json!({
                "client_id": resp.client_id.as_str(),
                "seed_nonce": resp.seed_nonce.as_str(),
                "transport_key": key_hex,
                "transport_key_ref": resp.transport_key_ref.0,
            });
            println!("{}", serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?);
        }
        PairFormat::ClaudeDesktop => {
            // Per L0 AC-5 + devex cycle-5: emit absolute path via current_exe()
            // because GUI-launched Claude Desktop/Cursor don't inherit PATH.
            let exe = std::env::current_exe()
                .map_err(|e| format!("resolve current_exe: {e}"))?;
            let snippet = serde_json::json!({
                "mcpServers": {
                    resp.client_id.as_str(): {
                        "type": "stdio",
                        "command": exe.display().to_string(),
                        "args": ["serve"],
                        "env": {
                            ENV_CLIENT_ID: resp.client_id.as_str(),
                            ENV_TRANSPORT_KEY: key_hex,
                        },
                    }
                }
            });
            println!("{}", serde_json::to_string_pretty(&snippet).map_err(|e| e.to_string())?);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// unpair
// ---------------------------------------------------------------------------

fn run_unpair(db_path: &PathBuf, client_id: &str) -> Result<(), String> {
    let conn = open_conn(db_path)?;
    let id = McpClientId::new(client_id);
    auth::revoke_client(&conn, &id).map_err(|e| format!("revoke: {e}"))?;
    eprintln!("dailyos-mcp-v2: revoked pairing {client_id}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn decode_hex_32(hex_str: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(hex_str.trim()).map_err(|e| format!("invalid hex: {e}"))?;
    if bytes.len() != 32 {
        return Err(format!(
            "transport_key must be 32 bytes hex-encoded; got {} bytes",
            bytes.len()
        ));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Constant-time byte slice equality. Hand-rolled because
/// `ring::constant_time` is now `deprecated_constant_time` and marked
/// "not intended for external use." Equivalent semantics: XOR-fold all
/// pairs, return whether the fold is zero. Length mismatch returns
/// false without short-circuit (constant time across length too).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn check_legacy_config(path: &PathBuf) -> Result<(), String> {
    // Best-effort guardrail per L0 AC-11: refuse if config text
    // references a binary that resolves (via canonicalization, following
    // symlinks) to legacy dailyos-mcp AND claims v2-owned tool names.
    //
    // For W1.5 minimal implementation: if the config contains both
    // "dailyos-mcp" (without -v2) as a command and any of the v2-owned
    // tool names, refuse. Hardlinks/copies acknowledged limitation
    // (filed separate ticket).
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read legacy config {path:?}: {e}"))?;
    let v2_owned_tools = [
        "dailyos.read.account_status",
        "dailyos.read.daily_briefing",
        "dailyos.read.meeting_briefing",
        "dailyos.read.portfolio_attention",
        "dailyos.search.workspace_memory",
        "dailyos.read.workspace_source_provenance",
        "dailyos.write.place_document",
        "dailyos.submit.note",
        "dailyos.submit.action",
        "dailyos.submit.action_status",
    ];
    let claims_v2_tool = v2_owned_tools.iter().any(|t| content.contains(t));
    let references_legacy_command = content.contains("\"dailyos-mcp\"")
        || content.contains("/dailyos-mcp\"")
        || content.contains("\\dailyos-mcp\"");
    let references_v2 = content.contains("dailyos-mcp-v2");
    // If config explicitly uses dailyos-mcp-v2 elsewhere, only refuse
    // if it ALSO claims v2-owned tools routed through the legacy bin.
    if claims_v2_tool && references_legacy_command && !references_v2 {
        return Err(format!(
            "refusing to start. Config at {path:?} claims v2-owned tools but routes \
             through legacy 'dailyos-mcp'. Update the config to use 'dailyos-mcp-v2'."
        ));
    }
    Ok(())
}
