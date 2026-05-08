//! Mutation allowlist + AST visitor for compile-time category enforcement.

include!(concat!(env!("OUT_DIR"), "/mutation_allowlist.rs"));

use std::collections::{HashMap, HashSet};

use syn::visit::Visit;
use syn::{
    Block, Expr, ExprCall, ExprMethodCall, ExprPath, Item, ItemFn, ItemUse, Path, UseName, UsePath,
    UseRename, UseTree,
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
///   - Service module alias tracking from `use crate::services::accounts;`
///     followed by `accounts::update_account_field(...)`
///
/// Does NOT do cross-function transitive analysis.
pub struct MutationVisitor {
    pub aliases: HashMap<String, String>,
    pub module_aliases: HashMap<String, String>,
    pub detected: Vec<String>,
}

impl MutationVisitor {
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
            module_aliases: HashMap::new(),
            detected: Vec::new(),
        }
    }

    pub fn scan_fn_body(&mut self, body: &syn::Block) {
        let mut alias_scan = AliasScan {
            aliases: &mut self.aliases,
            module_aliases: &mut self.module_aliases,
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
            return;
        }

        if let Some(canonical) = self.resolve_module_alias_path(path) {
            self.record_if_allowlisted(canonical);
        }
    }

    fn resolve_module_alias_path(&self, path: &Path) -> Option<String> {
        if path.leading_colon.is_some() {
            return None;
        }

        let mut segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string());
        let first = segments.next()?;
        let canonical_prefix = self.module_aliases.get(&first)?;
        let rest = segments.collect::<Vec<_>>();
        Some(join_segments(canonical_prefix, &rest))
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

/// AST visitor that records direct ability-body bypasses of the
/// ServiceContext/runtime-crate boundary. Unlike mutation scoring, these are
/// forbidden for every ability category because they reach raw app state,
/// SQLite, or filesystem handles directly.
pub struct BoundaryVisitor {
    pub aliases: HashMap<String, String>,
    pub module_aliases: HashMap<String, String>,
    pub detected: Vec<String>,
}

impl BoundaryVisitor {
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
            module_aliases: HashMap::new(),
            detected: Vec::new(),
        }
    }

    pub fn scan_fn_body(&mut self, body: &syn::Block) {
        let mut alias_scan = BoundaryAliasScan {
            aliases: &mut self.aliases,
            module_aliases: &mut self.module_aliases,
            detected: &mut self.detected,
        };
        alias_scan.visit_block(body);
        self.visit_block(body);
    }

    pub fn scan_fn_body_with_module_items(&mut self, item_fn: &ItemFn, module_items: &[Item]) {
        let module_context = BoundaryModuleContext::new(module_items);
        self.aliases.extend(module_context.aliases.clone());
        self.module_aliases
            .extend(module_context.module_aliases.clone());
        self.scan_fn_body(&item_fn.block);

        let mut visited = HashSet::new();
        self.scan_same_module_helpers(&item_fn.block, &module_context, &mut visited);
    }

    fn scan_same_module_helpers(
        &mut self,
        body: &Block,
        module_context: &BoundaryModuleContext<'_>,
        visited: &mut HashSet<String>,
    ) {
        for helper_call in same_module_helper_calls(body) {
            let Some((helper_key, helper_context, helper)) =
                module_context.resolve_helper_call(&helper_call)
            else {
                continue;
            };
            if !visited.insert(helper_key) {
                continue;
            }

            let mut helper_visitor = BoundaryVisitor::new();
            helper_visitor
                .aliases
                .extend(helper_context.aliases.clone());
            helper_visitor
                .module_aliases
                .extend(helper_context.module_aliases.clone());
            helper_visitor.scan_fn_body(&helper.block);
            self.extend_detected(helper_visitor.detected);
            self.scan_same_module_helpers(&helper.block, helper_context, visited);
        }
    }

    fn record_call_path(&mut self, path: &Path) {
        if let Some(canonical) = path_segments(path).map(|segments| join_all_segments(&segments)) {
            self.record_if_forbidden(canonical);
        }

        if let Some(alias) = single_segment_path(path) {
            if let Some(canonical) = self.aliases.get(&alias).cloned() {
                self.record_if_forbidden(canonical);
            }
            return;
        }

        if let Some(canonical) = self.resolve_module_alias_path(path) {
            self.record_if_forbidden(canonical);
        }
    }

    fn resolve_boundary_path(&self, path: &Path) -> Option<String> {
        if let Some(alias) = single_segment_path(path) {
            if let Some(canonical) = self.aliases.get(&alias).cloned() {
                return Some(canonical);
            }
            return path_segments(path).map(|segments| join_all_segments(&segments));
        }

        if let Some(canonical) = self.resolve_module_alias_path(path) {
            return Some(canonical);
        }

        path_segments(path).map(|segments| join_all_segments(&segments))
    }

    fn resolve_module_alias_path(&self, path: &Path) -> Option<String> {
        if path.leading_colon.is_some() {
            return None;
        }

        let mut segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string());
        let first = segments.next()?;
        let canonical_prefix = self.module_aliases.get(&first)?;
        let rest = segments.collect::<Vec<_>>();
        Some(join_segments(canonical_prefix, &rest))
    }

    fn record_if_forbidden(&mut self, canonical: String) {
        if let Some(forbidden) = forbidden_boundary_path(&canonical) {
            self.record_detected(forbidden);
        }
    }

    fn record_detected(&mut self, forbidden: &str) {
        if !self.detected.iter().any(|detected| detected == forbidden) {
            self.detected.push(forbidden.to_string());
        }
    }

    fn extend_detected(&mut self, detected: Vec<String>) {
        for forbidden in detected {
            self.record_detected(&forbidden);
        }
    }
}

impl Default for BoundaryVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ast> Visit<'ast> for BoundaryVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(ExprPath {
            qself: None, path, ..
        }) = node.func.as_ref()
        {
            self.record_call_path(path);
        }

        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if let Some(forbidden) = self.open_options_write_open_boundary(node) {
            self.record_detected(forbidden);
        }

        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        record_boundary_from_use_tree(
            &node.tree,
            &mut Vec::new(),
            &mut self.aliases,
            &mut self.module_aliases,
            &mut self.detected,
        );
        syn::visit::visit_item_use(self, node);
    }
}

impl BoundaryVisitor {
    fn open_options_write_open_boundary(&self, node: &ExprMethodCall) -> Option<&'static str> {
        if node.method != "open" {
            return None;
        }

        let mut methods = vec![node.method.to_string()];
        let root = collect_method_chain_root(node.receiver.as_ref(), &mut methods);
        let Expr::Call(ExprCall { func, .. }) = root else {
            return None;
        };
        let Expr::Path(ExprPath {
            qself: None, path, ..
        }) = func.as_ref()
        else {
            return None;
        };

        if path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            != Some("new".to_string())
        {
            return None;
        }

        let canonical = self.resolve_boundary_path(path)?;
        if !matches!(
            canonical.as_str(),
            "std::fs::OpenOptions::new" | "tokio::fs::OpenOptions::new"
        ) {
            return None;
        }

        let writes = methods.iter().any(|method| {
            matches!(
                method.as_str(),
                "append" | "create" | "create_new" | "truncate" | "write"
            )
        });
        if !writes {
            return None;
        }

        match canonical.as_str() {
            "std::fs::OpenOptions::new" => Some("std::fs::OpenOptions::open(write)"),
            "tokio::fs::OpenOptions::new" => Some("tokio::fs"),
            _ => None,
        }
    }
}

fn collect_method_chain_root<'a>(expr: &'a Expr, methods: &mut Vec<String>) -> &'a Expr {
    match expr {
        Expr::MethodCall(method_call) => {
            methods.push(method_call.method.to_string());
            collect_method_chain_root(method_call.receiver.as_ref(), methods)
        }
        other => other,
    }
}

struct BoundaryModuleContext<'a> {
    aliases: HashMap<String, String>,
    module_aliases: HashMap<String, String>,
    functions: HashMap<String, &'a ItemFn>,
    modules: HashMap<String, BoundaryModuleContext<'a>>,
}

impl<'a> BoundaryModuleContext<'a> {
    fn new(module_items: &'a [Item]) -> Self {
        let mut aliases = HashMap::new();
        let mut module_aliases = HashMap::new();
        let mut ignored_detected = Vec::new();
        let mut functions = HashMap::new();
        let mut modules = HashMap::new();

        for item in module_items {
            match item {
                Item::Use(item_use) => record_boundary_from_use_tree(
                    &item_use.tree,
                    &mut Vec::new(),
                    &mut aliases,
                    &mut module_aliases,
                    &mut ignored_detected,
                ),
                Item::Fn(item_fn) => {
                    functions.insert(item_fn.sig.ident.to_string(), item_fn);
                }
                Item::Mod(module) => {
                    if let Some((_, nested_items)) = &module.content {
                        modules.insert(
                            module.ident.to_string(),
                            BoundaryModuleContext::new(nested_items),
                        );
                    }
                }
                _ => {}
            }
        }

        Self {
            aliases,
            module_aliases,
            functions,
            modules,
        }
    }

    fn resolve_helper_call(
        &self,
        call: &SameModuleHelperCall,
    ) -> Option<(String, &BoundaryModuleContext<'a>, &'a ItemFn)> {
        let mut context = self;
        let mut key = Vec::new();

        for module in &call.module_path {
            context = context.modules.get(module)?;
            key.push(module.clone());
        }

        let helper = context.functions.get(call.function.as_str()).copied()?;
        key.push(call.function.clone());
        Some((key.join("::"), context, helper))
    }
}

fn same_module_helper_calls(body: &Block) -> Vec<SameModuleHelperCall> {
    let mut collector = SameModuleHelperCallCollector::default();
    collector.visit_block(body);
    collector.calls
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SameModuleHelperCall {
    module_path: Vec<String>,
    function: String,
}

#[derive(Default)]
struct SameModuleHelperCallCollector {
    calls: Vec<SameModuleHelperCall>,
}

impl<'ast> Visit<'ast> for SameModuleHelperCallCollector {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(ExprPath {
            qself: None, path, ..
        }) = node.func.as_ref()
        {
            if let Some(call) = same_module_helper_call(path) {
                if !self.calls.contains(&call) {
                    self.calls.push(call);
                }
            }
        }

        syn::visit::visit_expr_call(self, node);
    }
}

fn same_module_helper_call(path: &Path) -> Option<SameModuleHelperCall> {
    if path.leading_colon.is_some() {
        return None;
    }

    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();

    match segments.as_slice() {
        [function] => Some(SameModuleHelperCall {
            module_path: Vec::new(),
            function: function.clone(),
        }),
        [prefix, function] if prefix == "self" => Some(SameModuleHelperCall {
            module_path: Vec::new(),
            function: function.clone(),
        }),
        [prefix, rest @ ..] if prefix == "self" && rest.len() >= 2 => {
            let function = rest.last()?.clone();
            let module_path = rest[..rest.len() - 1].to_vec();
            Some(SameModuleHelperCall {
                module_path,
                function,
            })
        }
        [first, ..] if matches!(first.as_str(), "crate" | "super") => None,
        [..] if segments.len() >= 2 => {
            let function = segments.last()?.clone();
            let module_path = segments[..segments.len() - 1].to_vec();
            Some(SameModuleHelperCall {
                module_path,
                function,
            })
        }
        _ => None,
    }
}

struct BoundaryAliasScan<'a> {
    aliases: &'a mut HashMap<String, String>,
    module_aliases: &'a mut HashMap<String, String>,
    detected: &'a mut Vec<String>,
}

impl<'ast> Visit<'ast> for BoundaryAliasScan<'_> {
    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        record_boundary_from_use_tree(
            &node.tree,
            &mut Vec::new(),
            self.aliases,
            self.module_aliases,
            self.detected,
        );
        syn::visit::visit_item_use(self, node);
    }
}

impl<'ast> Visit<'ast> for MutationVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(ExprPath {
            qself: None, path, ..
        }) = node.func.as_ref()
        {
            self.record_call_path(path);
        }

        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        record_aliases_from_use_tree(
            &node.tree,
            &mut Vec::new(),
            &mut self.aliases,
            &mut self.module_aliases,
        );
        syn::visit::visit_item_use(self, node);
    }
}

struct AliasScan<'a> {
    aliases: &'a mut HashMap<String, String>,
    module_aliases: &'a mut HashMap<String, String>,
}

impl<'ast> Visit<'ast> for AliasScan<'_> {
    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        record_aliases_from_use_tree(
            &node.tree,
            &mut Vec::new(),
            self.aliases,
            self.module_aliases,
        );
        syn::visit::visit_item_use(self, node);
    }
}

fn record_aliases_from_use_tree(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    aliases: &mut HashMap<String, String>,
    module_aliases: &mut HashMap<String, String>,
) {
    match tree {
        UseTree::Path(UsePath { ident, tree, .. }) => {
            prefix.push(ident.to_string());
            record_aliases_from_use_tree(tree, prefix, aliases, module_aliases);
            prefix.pop();
        }
        UseTree::Name(UseName { ident }) => {
            let alias = ident.to_string();
            prefix.push(alias.clone());
            record_alias(alias, prefix, aliases, module_aliases);
            prefix.pop();
        }
        UseTree::Rename(UseRename { ident, rename, .. }) => {
            prefix.push(ident.to_string());
            record_alias(rename.to_string(), prefix, aliases, module_aliases);
            prefix.pop();
        }
        UseTree::Group(group) => {
            for tree in &group.items {
                record_aliases_from_use_tree(tree, prefix, aliases, module_aliases);
            }
        }
        UseTree::Glob(_) => {}
    }
}

fn record_alias(
    alias: String,
    path_segments: &[String],
    aliases: &mut HashMap<String, String>,
    module_aliases: &mut HashMap<String, String>,
) {
    if let Some(canonical) = canonical_segments(path_segments) {
        if MUTATION_ALLOWLIST.contains(&canonical.as_str()) {
            aliases.insert(alias, canonical);
        } else if allowlist_has_module_prefix(&canonical) {
            module_aliases.insert(alias, canonical);
        }
    }
}

fn allowlist_has_module_prefix(canonical: &str) -> bool {
    let prefix = format!("{canonical}::");
    MUTATION_ALLOWLIST
        .iter()
        .any(|path| path.starts_with(&prefix))
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

    path.segments
        .first()
        .map(|segment| segment.ident.to_string())
}

fn path_segments(path: &Path) -> Option<Vec<String>> {
    if path.leading_colon.is_some() {
        return None;
    }
    Some(
        path.segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect(),
    )
}

fn join_all_segments(segments: &[String]) -> String {
    segments.join("::")
}

fn record_boundary_from_use_tree(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    aliases: &mut HashMap<String, String>,
    module_aliases: &mut HashMap<String, String>,
    detected: &mut Vec<String>,
) {
    match tree {
        UseTree::Path(UsePath { ident, tree, .. }) => {
            prefix.push(ident.to_string());
            record_boundary_from_use_tree(tree, prefix, aliases, module_aliases, detected);
            prefix.pop();
        }
        UseTree::Name(UseName { ident }) => {
            prefix.push(ident.to_string());
            record_boundary_alias(ident.to_string(), prefix, aliases, module_aliases, detected);
            prefix.pop();
        }
        UseTree::Rename(UseRename { ident, rename, .. }) => {
            prefix.push(ident.to_string());
            record_boundary_alias(
                rename.to_string(),
                prefix,
                aliases,
                module_aliases,
                detected,
            );
            prefix.pop();
        }
        UseTree::Group(group) => {
            for tree in &group.items {
                record_boundary_from_use_tree(tree, prefix, aliases, module_aliases, detected);
            }
        }
        UseTree::Glob(_) => {
            let canonical = join_all_segments(prefix);
            record_forbidden_boundary_import(canonical, detected);
        }
    }
}

fn record_boundary_alias(
    alias: String,
    path_segments: &[String],
    aliases: &mut HashMap<String, String>,
    module_aliases: &mut HashMap<String, String>,
    detected: &mut Vec<String>,
) {
    let canonical = join_all_segments(path_segments);
    record_forbidden_boundary_import(canonical.clone(), detected);
    if forbidden_boundary_path(&canonical).is_some() {
        aliases.insert(alias.clone(), canonical.clone());
        module_aliases.insert(alias, canonical);
    } else if boundary_has_forbidden_prefix(&canonical) || is_open_options_type_path(&canonical) {
        module_aliases.insert(alias, canonical);
    }
}

fn record_forbidden_boundary_import(canonical: String, detected: &mut Vec<String>) {
    if let Some(forbidden) = forbidden_boundary_path(&canonical) {
        let forbidden = forbidden.to_string();
        if !detected.contains(&forbidden) {
            detected.push(forbidden);
        }
    }
}

fn boundary_has_forbidden_prefix(canonical: &str) -> bool {
    FORBIDDEN_BOUNDARY_PATHS
        .iter()
        .any(|path| path.starts_with(&format!("{canonical}::")))
}

fn is_open_options_type_path(canonical: &str) -> bool {
    matches!(canonical, "std::fs::OpenOptions" | "tokio::fs::OpenOptions")
}

const FORBIDDEN_BOUNDARY_PATHS: &[&str] = &[
    "crate::db",
    "crate::state",
    "crate::db_service",
    "crate::queries",
    "rusqlite",
    "tokio::fs",
    "std::fs::write",
    "std::fs::File::create",
    "std::fs::File::create_new",
    "std::fs::File::open",
    "File::create",
    "File::create_new",
    "File::open",
    "std::fs::OpenOptions::open(write)",
];

fn forbidden_boundary_path(canonical: &str) -> Option<&'static str> {
    match canonical {
        "OpenOptions::new"
        | "std::fs::OpenOptions::new"
        | "File::options"
        | "std::fs::File::options" => {
            return Some("std::fs::OpenOptions::open(write)");
        }
        "tokio::fs::OpenOptions::new" => {
            return Some("tokio::fs");
        }
        _ => {}
    }

    FORBIDDEN_BOUNDARY_PATHS.iter().copied().find(|path| {
        canonical == *path
            || matches!(
                *path,
                "crate::db"
                    | "crate::state"
                    | "crate::db_service"
                    | "crate::queries"
                    | "rusqlite"
                    | "tokio::fs"
            ) && canonical.starts_with(&format!("{path}::"))
    })
}
