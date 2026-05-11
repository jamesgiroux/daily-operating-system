//! `emit_ability_inventory` — W1-C.
//!
//! Enumerates every `inventory::submit!`-registered [`AbilityDescriptor`],
//! projects each into an [`AbilitySurfaceInventoryEntry`], and emits the
//! canonical, deterministic JSON artifact consumed by:
//!
//! - the WordPress plugin's `class-dailyos-ability-registry.php` at
//!   install / activation,
//! - the custom MCP server's allowlist,
//! - SurfaceClient introspection (`list_tools` filtered by `mcp_exposure`),
//! - Wave 4 block code for renderable abilities.
//!
//! ## Output path
//!
//! Writes to `--out <path>` when supplied, otherwise prints to stdout. The
//! companion CI gate at `scripts/check_ability_inventory.sh` regenerates
//! the artifact into a temp file and diffs against
//! `tools/dailyos-abilities.json` — drift fails the build.
//!
//! ## Exit codes
//!
//! - `0`: emitted successfully.
//! - `1`: registry build failed (one or more
//!   [`RegistryViolation`](abilities_runtime::abilities::registry::RegistryViolation)).
//! - `2`: I/O or serialization error (CLI arg parsing, file write,
//!   stdout write, or JSON serialization).

use std::io::Write;
use std::process::ExitCode;

use abilities_runtime::abilities::registry::AbilityRegistry;
use abilities_runtime::inventory::AbilitySurfaceInventory;

fn main() -> ExitCode {
    let mut out_path: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => match args.next() {
                Some(value) => out_path = Some(value),
                None => {
                    eprintln!("emit_ability_inventory: --out requires a path argument");
                    return ExitCode::from(2);
                }
            },
            "--help" | "-h" => {
                println!(
                    "emit_ability_inventory — W1-C\n\nUsage: \
                     emit_ability_inventory [--out PATH]\n\nWrites the canonical \
                     ability-surface inventory JSON to PATH (or stdout)."
                );
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("emit_ability_inventory: unknown argument: {other}");
                return ExitCode::from(2);
            }
        }
    }

    // Build the registry from `inventory::submit!`. The checked build
    // surfaces structural violations (duplicate names, unknown composes,
    // category-transitivity breaks, experimental drift). We refuse to
    // emit an inventory off a broken registry — the inventory is a
    // contract, not a snapshot of garbage in.
    let registry = match AbilityRegistry::from_inventory_checked() {
        Ok(registry) => registry,
        Err(violations) => {
            eprintln!(
                "emit_ability_inventory: registry build failed with {} violation(s):",
                violations.len()
            );
            for violation in &violations {
                eprintln!("  - {violation:?}");
            }
            return ExitCode::from(1);
        }
    };

    let descriptors: Vec<_> = registry.iter_all().collect();
    let inventory = AbilitySurfaceInventory::from_descriptors(descriptors);
    let json = match inventory.to_canonical_json() {
        Ok(json) => json,
        Err(err) => {
            eprintln!("emit_ability_inventory: serialization failed: {err}");
            return ExitCode::from(2);
        }
    };

    match out_path {
        Some(path) => {
            if let Err(err) = std::fs::write(&path, json.as_bytes()) {
                eprintln!("emit_ability_inventory: failed to write {path}: {err}");
                return ExitCode::from(2);
            }
        }
        None => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            if let Err(err) = handle.write_all(json.as_bytes()) {
                eprintln!("emit_ability_inventory: stdout write failed: {err}");
                return ExitCode::from(2);
            }
        }
    }

    ExitCode::SUCCESS
}
