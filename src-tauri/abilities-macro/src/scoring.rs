//! Mutation allowlist + AST visitor for compile-time category enforcement.

include!(concat!(env!("OUT_DIR"), "/mutation_allowlist.rs"));

use std::collections::HashMap;

use syn::visit::Visit;
use syn::{
    Expr, ExprCall, ExprMethodCall, ExprPath, ItemUse, Path, UseName, UsePath, UseRename, UseTree,
};

/// Returns true if the path matches an allowlisted mutator.
pub fn path_is_allowlisted_mutator(path: &Path) -> bool {
    canonical_path(path)
        .as_deref()
        .is_some_and(|path| MUTATION_ALLOWLIST.contains(&path))
}

/// AST visitor that records allowlisted mutator calls inside a function body.
/// Recognizes:
///   - Direct path calls: services::accounts::update_account_field(...)
///   - crate::services::* prefixed calls
///   - Syntactic alias tracking from `use services::accounts::update_account_field as foo;`
///     followed by `foo(...)` in the same function body
///
/// Does NOT do cross-function transitive analysis.
pub struct MutationVisitor {
    pub aliases: HashMap<String, String>,
    pub detected: Vec<String>,
}

impl MutationVisitor {
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
            detected: Vec::new(),
        }
    }

    pub fn scan_fn_body(&mut self, body: &syn::Block) {
        let mut alias_scan = AliasScan {
            aliases: &mut self.aliases,
        };
        alias_scan.visit_block(body);
        self.visit_block(body);
    }

    fn record_call_path(&mut self, path: &Path) {
        if let Some(canonical) = canonical_path(path) {
            self.record_if_allowlisted(canonical);
            return;
        }

        if let Some(alias) = single_segment_path(path) {
            if let Some(canonical) = self.aliases.get(&alias).cloned() {
                self.record_if_allowlisted(canonical);
            }
        }
    }

    fn record_if_allowlisted(&mut self, canonical: String) {
        if MUTATION_ALLOWLIST.contains(&canonical.as_str()) && !self.detected.contains(&canonical) {
            self.detected.push(canonical);
        }
    }
}

impl Default for MutationVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Visit<'ast> for MutationVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(ExprPath { qself: None, path, .. }) = node.func.as_ref() {
            self.record_call_path(path);
        }

        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        record_aliases_from_use_tree(&node.tree, &mut Vec::new(), &mut self.aliases);
        syn::visit::visit_item_use(self, node);
    }
}

struct AliasScan<'a> {
    aliases: &'a mut HashMap<String, String>,
}

impl<'ast> Visit<'ast> for AliasScan<'_> {
    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        record_aliases_from_use_tree(&node.tree, &mut Vec::new(), self.aliases);
        syn::visit::visit_item_use(self, node);
    }
}

fn record_aliases_from_use_tree(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    aliases: &mut HashMap<String, String>,
) {
    match tree {
        UseTree::Path(UsePath { ident, tree, .. }) => {
            prefix.push(ident.to_string());
            record_aliases_from_use_tree(tree, prefix, aliases);
            prefix.pop();
        }
        UseTree::Name(UseName { ident }) => {
            let alias = ident.to_string();
            prefix.push(alias.clone());
            record_alias(alias, prefix, aliases);
            prefix.pop();
        }
        UseTree::Rename(UseRename { ident, rename, .. }) => {
            prefix.push(ident.to_string());
            record_alias(rename.to_string(), prefix, aliases);
            prefix.pop();
        }
        UseTree::Group(group) => {
            for tree in &group.items {
                record_aliases_from_use_tree(tree, prefix, aliases);
            }
        }
        UseTree::Glob(_) => {}
    }
}

fn record_alias(alias: String, path_segments: &[String], aliases: &mut HashMap<String, String>) {
    if let Some(canonical) = canonical_segments(path_segments) {
        if MUTATION_ALLOWLIST.contains(&canonical.as_str()) {
            aliases.insert(alias, canonical);
        }
    }
}

fn canonical_path(path: &Path) -> Option<String> {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();

    canonical_segments(&segments)
}

fn canonical_segments(segments: &[String]) -> Option<String> {
    match segments {
        [first, rest @ ..] if first == "services" => Some(join_segments(first, rest)),
        [first, second, rest @ ..] if first == "crate" && second == "services" => {
            Some(join_segments(second, rest))
        }
        _ => None,
    }
}

fn join_segments(first: &str, rest: &[String]) -> String {
    rest.iter().fold(first.to_string(), |mut path, segment| {
        path.push_str("::");
        path.push_str(segment);
        path
    })
}

fn single_segment_path(path: &Path) -> Option<String> {
    if path.leading_colon.is_some() || path.segments.len() != 1 {
        return None;
    }

    path.segments.first().map(|segment| segment.ident.to_string())
}
