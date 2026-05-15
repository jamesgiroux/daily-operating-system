use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use syn::{BinOp, Expr, ExprAssign, ExprBinary, ExprField, File, ImplItemFn, ItemFn, Member};
use walkdir::WalkDir;

#[derive(Debug)]
struct Violation {
    path: PathBuf,
    field: String,
    function: Option<String>,
}

/// Functions allowed to assign `claim_version`. Anything else outside this
/// list — even within `services/claims.rs` — fails the gate. Tightens the
/// V1 path-only allowlist into a function-scoped contract so drive-by
/// patches cannot reintroduce ad-hoc version assignment.
const CLAIM_VERSION_FN_ALLOWLIST: &[&str] = &[
    // Insertion path: every fresh claim row carries an explicit version.
    "commit_claim",
    // Helpers that own the version bump for existing-row mutation Txs.
    "bump_existing_claim_version_tx",
    "enforce_claim_mutation_target_tx",
    // IntelligenceClaim struct construction in commit_claim closures (the
    // entire function body is allowed; this allows the struct literal
    // `claim_version: inserted_claim_version` to satisfy the visitor when
    // it's nested deeply enough that the outer fn name isn't captured).
    // Listed explicitly: closure literals don't have names, so the visitor
    // walks up to the enclosing fn.
];

const COMPOSITION_VERSION_FN_ALLOWLIST: &[&str] = &[
    "commit_composition",
    "commit_composition_tx",
];

struct AssignmentVisitor<'a> {
    path: &'a Path,
    function_stack: Vec<String>,
    violations: Vec<Violation>,
}

impl<'ast> Visit<'ast> for AssignmentVisitor<'_> {
    fn visit_expr_assign(&mut self, node: &'ast ExprAssign) {
        self.check_assignment_target(&node.left);
        visit::visit_expr_assign(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        if is_assignment_op(&node.op) {
            self.check_assignment_target(&node.left);
        }
        visit::visit_expr_binary(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.function_stack.push(node.sig.ident.to_string());
        visit::visit_item_fn(self, node);
        self.function_stack.pop();
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        self.function_stack.push(node.sig.ident.to_string());
        visit::visit_impl_item_fn(self, node);
        self.function_stack.pop();
    }
}

impl AssignmentVisitor<'_> {
    fn check_assignment_target(&mut self, expr: &Expr) {
        let Some(field) = assigned_field_name(expr) else {
            return;
        };
        if !matches!(field.as_str(), "claim_version" | "composition_version") {
            return;
        }
        if assignment_allowed(self.path, &field, &self.function_stack) {
            return;
        }
        self.violations.push(Violation {
            path: self.path.to_path_buf(),
            field,
            function: self.function_stack.last().cloned(),
        });
    }
}

fn assigned_field_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Field(ExprField {
            member: Member::Named(ident),
            ..
        }) => Some(ident.to_string()),
        Expr::Paren(paren) => assigned_field_name(&paren.expr),
        _ => None,
    }
}

fn is_assignment_op(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::AddAssign(_)
            | BinOp::SubAssign(_)
            | BinOp::MulAssign(_)
            | BinOp::DivAssign(_)
            | BinOp::RemAssign(_)
            | BinOp::BitXorAssign(_)
            | BinOp::BitAndAssign(_)
            | BinOp::BitOrAssign(_)
            | BinOp::ShlAssign(_)
            | BinOp::ShrAssign(_)
    )
}

fn assignment_allowed(path: &Path, field: &str, function_stack: &[String]) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let path_ok = match field {
        "claim_version" => normalized.ends_with("src-tauri/src/services/claims.rs"),
        "composition_version" => {
            normalized.ends_with("src-tauri/src/services/compositions.rs")
        }
        _ => false,
    };
    if !path_ok {
        return false;
    }
    // Path-scoped allowance is necessary but not sufficient. The function
    // stack (innermost-first scan) must contain at least one entry from the
    // chokepoint allowlist. Closures and nested helpers inside the
    // allowlisted function inherit the allowance.
    let allowlist: &[&str] = match field {
        "claim_version" => CLAIM_VERSION_FN_ALLOWLIST,
        "composition_version" => COMPOSITION_VERSION_FN_ALLOWLIST,
        _ => return false,
    };
    function_stack
        .iter()
        .any(|fn_name| allowlist.contains(&fn_name.as_str()))
}

fn parse_file(path: &Path) -> File {
    let source = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    syn::parse_file(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

#[test]
fn version_fields_are_assigned_only_by_commit_chokepoints() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scan_roots = [
        manifest_dir.join("src"),
        manifest_dir.join("abilities-runtime/src"),
    ];
    let mut violations = Vec::new();

    for root in scan_roots {
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file()
                || entry.path().extension().and_then(|ext| ext.to_str()) != Some("rs")
            {
                continue;
            }
            let syntax = parse_file(entry.path());
            let mut visitor = AssignmentVisitor {
                path: entry.path(),
                function_stack: Vec::new(),
                violations: Vec::new(),
            };
            visitor.visit_file(&syntax);
            violations.extend(visitor.violations);
        }
    }

    assert!(
        violations.is_empty(),
        "version assignment must stay inside commit chokepoints; violations: {}",
        violations
            .iter()
            .map(|violation| format!(
                "{} -> {} in {}",
                violation.path.display(),
                violation.field,
                violation.function.as_deref().unwrap_or("<file-scope>")
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
}
