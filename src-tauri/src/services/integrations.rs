// Integrations service — extracted from commands.rs
// Business logic for Claude Desktop MCP configuration.

/// Result of Claude Desktop MCP configuration.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopConfigResult {
    pub success: bool,
    pub message: String,
    pub config_path: Option<String>,
    pub binary_path: Option<String>,
}

/// Check whether DailyOS is already registered in Claude Desktop's MCP config.
pub fn get_claude_desktop_status() -> ClaudeDesktopConfigResult {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            return ClaudeDesktopConfigResult {
                success: false,
                message: "Could not find home directory".to_string(),
                config_path: None,
                binary_path: None,
            }
        }
    };

    let config_path = home
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");

    if !config_path.exists() {
        return ClaudeDesktopConfigResult {
            success: false,
            message: "Not configured".to_string(),
            config_path: None,
            binary_path: None,
        };
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => {
            return ClaudeDesktopConfigResult {
                success: false,
                message: "Could not read config".to_string(),
                config_path: Some(config_path.to_string_lossy().to_string()),
                binary_path: None,
            }
        }
    };

    let config: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => {
            return ClaudeDesktopConfigResult {
                success: false,
                message: "Config file is not valid JSON".to_string(),
                config_path: Some(config_path.to_string_lossy().to_string()),
                binary_path: None,
            }
        }
    };

    let entry = config
        .get("mcpServers")
        .and_then(|s| s.get("dailyos"));

    match entry {
        Some(server) => {
            let binary = server
                .get("command")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let binary_exists = binary
                .as_ref()
                .map(|p| std::path::Path::new(p).exists())
                .unwrap_or(false);
            ClaudeDesktopConfigResult {
                success: binary_exists,
                message: if binary_exists {
                    "Connected".to_string()
                } else {
                    "Configured but binary not found — reconfigure or reinstall".to_string()
                },
                config_path: Some(config_path.to_string_lossy().to_string()),
                binary_path: binary,
            }
        }
        None => ClaudeDesktopConfigResult {
            success: false,
            message: "Not configured".to_string(),
            config_path: Some(config_path.to_string_lossy().to_string()),
            binary_path: None,
        },
    }
}

/// Configure Claude Desktop to use the DailyOS MCP server.
///
/// Reads (or creates) `~/Library/Application Support/Claude/claude_desktop_config.json`
/// and adds/updates the `mcpServers.dailyos` entry pointing to the bundled binary.
pub fn configure_claude_desktop() -> ClaudeDesktopConfigResult {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            return ClaudeDesktopConfigResult {
                success: false,
                message: "Could not find home directory".to_string(),
                config_path: None,
                binary_path: None,
            }
        }
    };

    // Resolve MCP binary path: check common locations
    let binary_name = "dailyos-mcp";
    let binary_path = resolve_mcp_binary_path(&home, binary_name);

    let binary_path_str = match &binary_path {
        Some(p) => {
            // Ensure binary is executable (build may not set +x)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(p) {
                    let mut perms = meta.permissions();
                    let mode = perms.mode();
                    if mode & 0o111 == 0 {
                        perms.set_mode(mode | 0o755);
                        let _ = std::fs::set_permissions(p, perms);
                    }
                }
            }
            p.to_string_lossy().to_string()
        }
        None => {
            return ClaudeDesktopConfigResult {
                success: false,
                message: format!(
                    "The {binary_name} component is missing from this installation. \
                     Please reinstall DailyOS from the latest release at https://daily-os.com"
                ),
                config_path: None,
                binary_path: None,
            }
        }
    };

    // Claude Desktop config path
    let config_path = home
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");

    // Read existing config or start fresh
    let mut config: serde_json::Value = if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| {
                serde_json::json!({})
            }),
            Err(_) => serde_json::json!({}),
        }
    } else {
        serde_json::json!({})
    };

    // Ensure mcpServers object exists
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = serde_json::json!({});
    }

    // Set the dailyos entry
    config["mcpServers"]["dailyos"] = serde_json::json!({
        "command": binary_path_str,
        "args": [],
        "env": {}
    });

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ClaudeDesktopConfigResult {
                    success: false,
                    message: format!("Failed to create config directory: {e}"),
                    config_path: None,
                    binary_path: Some(binary_path_str),
                };
            }
        }
    }

    // Write config
    let formatted = match serde_json::to_string_pretty(&config) {
        Ok(s) => s,
        Err(e) => {
            return ClaudeDesktopConfigResult {
                success: false,
                message: format!("Failed to serialize config: {e}"),
                config_path: Some(config_path.to_string_lossy().to_string()),
                binary_path: Some(binary_path_str),
            }
        }
    };

    match std::fs::write(&config_path, formatted) {
        Ok(()) => ClaudeDesktopConfigResult {
            success: true,
            message: "Claude Desktop configured. Restart Claude Desktop to connect.".to_string(),
            config_path: Some(config_path.to_string_lossy().to_string()),
            binary_path: Some(binary_path_str),
        },
        Err(e) => ClaudeDesktopConfigResult {
            success: false,
            message: format!("Failed to write config: {e}"),
            config_path: Some(config_path.to_string_lossy().to_string()),
            binary_path: Some(binary_path_str),
        },
    }
}

/// Resolve the MCP binary path by checking common locations.
fn resolve_mcp_binary_path(
    home: &std::path::Path,
    binary_name: &str,
) -> Option<std::path::PathBuf> {
    // 1. Check if in PATH (cargo install location)
    let cargo_bin = home.join(".cargo").join("bin").join(binary_name);
    if cargo_bin.exists() {
        return Some(cargo_bin);
    }

    // 2. Check alongside the running executable (app bundle)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let sibling = exe_dir.join(binary_name);
            if sibling.exists() {
                return Some(sibling);
            }
            // macOS .app bundle: Contents/MacOS/
            let macos_sibling = exe_dir.join(binary_name);
            if macos_sibling.exists() {
                return Some(macos_sibling);
            }
        }
    }

    // 3. Check dev build location (target/debug)
    let cwd = std::env::current_dir().ok()?;
    let dev_paths = [
        cwd.join("target/debug").join(binary_name),
        cwd.join("src-tauri/target/debug").join(binary_name),
        cwd.join("target/release").join(binary_name),
        cwd.join("src-tauri/target/release").join(binary_name),
    ];
    for path in &dev_paths {
        if path.exists() {
            return Some(path.clone());
        }
    }

    None
}
