use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};
use syn::{BinOp, Expr, ExprAssign, ExprBinary, ExprField, File, Member};
use walkdir::WalkDir;

#[derive(Debug)]
struct Violation {
    path: PathBuf,
    field: String,
}

struct AssignmentVisitor<'a> {
    path: &'a Path,
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
}

impl AssignmentVisitor<'_> {
    fn check_assignment_target(&mut self, expr: &Expr) {
        let Some(field) = assigned_field_name(expr) else {
            return;
        };
        if !matches!(field.as_str(), "claim_version" | "composition_version") {
            return;
        }
        if assignment_allowed(self.path, &field) {
            return;
        }
        self.violations.push(Violation {
            path: self.path.to_path_buf(),
            field,
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

fn assignment_allowed(path: &Path, field: &str) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    match field {
        "claim_version" => normalized.ends_with("src-tauri/src/services/claims.rs"),
        "composition_version" => normalized.ends_with("src-tauri/src/services/compositions.rs"),
        _ => false,
    }
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
            .map(|violation| format!("{} -> {}", violation.path.display(), violation.field))
            .collect::<Vec<_>>()
            .join(", ")
    );
}
