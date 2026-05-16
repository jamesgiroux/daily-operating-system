use std::fs;
use std::path::Path;

const FORBIDDEN_AGGREGATE_FIELDS: &[&str] = &[
    "entity_id",
    "claim_text",
    "content_hash",
    "actor",
    "field_path",
    "prompt_template_id",
    "invocation_id",
    "file_path",
];

pub fn validate_aggregate_metric_contract(manifest_dir: &Path) {
    let module_path = manifest_dir.join("src/observability/aggregate_metric/mod.rs");
    println!("cargo:rerun-if-changed={}", module_path.display());

    let module_source = read_to_string(&module_path);
    let catalog = parse_catalog(&module_source);
    if catalog.is_empty() {
        panic!("AGGREGATE_METRIC_CATALOG must declare at least one metric");
    }
    let struct_body = extract_struct_body(&module_source, "AggregateMetric")
        .unwrap_or_else(|| panic!("AggregateMetric struct must be declared"));
    for forbidden in FORBIDDEN_AGGREGATE_FIELDS {
        let field_marker = format!("pub {forbidden}:");
        if struct_body.contains(&field_marker) {
            panic!("AggregateMetric must not declare forbidden field `{forbidden}`");
        }
    }

    let src_dir = manifest_dir.join("src");
    scan_rust_sources(&src_dir, &mut |path, source| {
        println!("cargo:rerun-if-changed={}", path.display());
        if !path
            .to_string_lossy()
            .ends_with("src/observability/aggregate_metric/lint.rs")
        {
            validate_catalog_macro_calls(path, source, &catalog);
        }
        validate_catalog_name_bypass(path, source);
    });
}

fn validate_catalog_macro_calls(path: &Path, source: &str, catalog: &[String]) {
    let mut offset = 0;
    let marker = "aggregate_metric_name!";
    while let Some(relative) = source[offset..].find(marker) {
        let start = offset + relative + marker.len();
        let Some(open_relative) = source[start..].find('(') else {
            break;
        };
        let open = start + open_relative + 1;
        let rest = &source[open..];
        let literal = parse_first_string_literal(rest).unwrap_or_else(|| {
            panic!(
                "{} uses aggregate_metric_name! without a string literal",
                path.display()
            )
        });
        if !catalog.iter().any(|name| name == &literal) {
            panic!(
                "{} references aggregate metric `{}` outside AGGREGATE_METRIC_CATALOG",
                path.display(),
                literal
            );
        }
        offset = open + literal.len();
    }
}

fn validate_catalog_name_bypass(path: &Path, source: &str) {
    let normalized = path.to_string_lossy();
    if normalized.ends_with("src/observability/aggregate_metric/mod.rs")
        || normalized.ends_with("src/observability/aggregate_metric/lint.rs")
    {
        return;
    }
    for bypass in [
        "CatalogName {",
        "CatalogName(",
        "__catalog_macro_support::catalog_name",
    ] {
        if source.contains(bypass) {
            panic!(
                "{} must use aggregate_metric_name!(...) instead of `{}`",
                path.display(),
                bypass
            );
        }
    }
}

fn parse_catalog(source: &str) -> Vec<String> {
    let start = source
        .find("pub const AGGREGATE_METRIC_CATALOG")
        .expect("AGGREGATE_METRIC_CATALOG const must exist");
    let tail = &source[start..];
    let array_start = tail
        .find("&[")
        .expect("AGGREGATE_METRIC_CATALOG must be a slice literal")
        + start
        + 2;
    let array_tail = &source[array_start..];
    let array_end = array_tail
        .find("];")
        .expect("AGGREGATE_METRIC_CATALOG slice literal must close");
    let array_body = &array_tail[..array_end];

    let mut entries = Vec::new();
    let mut rest = array_body;
    while let Some(start_quote) = rest.find('"') {
        let after_start = &rest[start_quote + 1..];
        let Some(end_quote) = after_start.find('"') else {
            break;
        };
        entries.push(after_start[..end_quote].to_string());
        rest = &after_start[end_quote + 1..];
    }
    entries
}

fn extract_struct_body<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("pub struct {name}");
    let start = source.find(&marker)?;
    let open = source[start..].find('{')? + start;
    let mut depth = 0usize;
    for (idx, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(&source[open + 1..open + idx]);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_first_string_literal(source: &str) -> Option<String> {
    let source = source.trim_start();
    let rest = source.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn scan_rust_sources(dir: &Path, visit: &mut impl FnMut(&Path, &str)) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|err| panic!("failed to read {}: {err}", dir.display()));
    for entry in entries {
        let entry = entry
            .unwrap_or_else(|err| panic!("failed to read entry under {}: {err}", dir.display()));
        let path = entry.path();
        if path.is_dir() {
            scan_rust_sources(&path, visit);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            let source = read_to_string(&path);
            visit(&path, &source);
        }
    }
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}
