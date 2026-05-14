//! `#[ability]` proc macro and AST scoring for DailyOS abilities.

#[allow(dead_code)]
mod scoring;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    braced, bracketed, parse_macro_input, Attribute, FnArg, GenericArgument, Ident, Item, ItemFn,
    LitBool, LitInt, LitStr, Pat, PathArguments, ReturnType, Token, Type,
};

#[proc_macro_attribute]
pub fn ability(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AbilityArgs);
    let item_fn = parse_macro_input!(item as ItemFn);

    match expand_ability(args, item_fn) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_ability(args: AbilityArgs, item_fn: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    if item_fn.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            item_fn.sig.fn_token,
            "#[ability] functions must be async",
        ));
    }

    let (ctx_ident, input_ident, input_ty) = ability_signature_parts(&item_fn)?;
    let output_ty = ability_result_output_ty(&item_fn)?;

    let mut visitor = scoring::MutationVisitor::new();
    visitor.scan_fn_body(&item_fn.block);
    let detected = visitor.detected;

    let mut boundary_visitor = scoring::BoundaryVisitor::new();
    if let Some(module_source) = same_source_module_items_for_ability(&item_fn) {
        boundary_visitor.scan_fn_body_with_crate_items(
            &item_fn,
            &module_source.items,
            &module_source.containing_module_path,
        );
    } else {
        boundary_visitor.scan_fn_body(&item_fn.block);
    }
    let detected_boundary_bypasses = boundary_visitor.detected;
    if !detected_boundary_bypasses.is_empty() {
        return Err(syn::Error::new_spanned(
            &item_fn.sig.ident,
            format!(
                "ability bodies cannot bypass the ServiceContext boundary; detected: {}",
                detected_boundary_bypasses.join(", ")
            ),
        ));
    }

    if matches!(
        args.category,
        AbilityCategoryArg::Read | AbilityCategoryArg::Transform
    ) && !detected.is_empty()
        && !args.experimental
    {
        return Err(syn::Error::new_spanned(
            &item_fn.sig.ident,
            format!(
                "{} abilities cannot call mutating services; detected: {}",
                args.category.as_str(),
                detected.join(", ")
            ),
        ));
    }

    if args.experimental && args.registered_at.is_none() {
        return Err(syn::Error::new_spanned(
            &item_fn.sig.ident,
            "experimental abilities must declare registered_at",
        ));
    }

    if args.experimental && args.may_publish {
        return Err(syn::Error::new_spanned(
            &item_fn.sig.ident,
            "experimental abilities must set may_publish = false",
        ));
    }

    // W1-B compile-error gate (ADR-0102 §7.6):
    // an ability whose `allowed_actors` includes `SurfaceClient` must
    // either declare a non-empty `required_scopes` set, or explicitly
    // opt out with `no_scope_required`. The gate fires regardless of
    // `client_side_executable` value — actor membership in
    // `allowed_actors` is the trigger because that is what makes the
    // ability reachable from a SurfaceClient at all.
    let exposes_to_surface_client = args
        .allowed_actors
        .iter()
        .any(|actor| matches!(actor, ActorArg::SurfaceClient));
    if exposes_to_surface_client && args.required_scopes.is_empty() && !args.no_scope_required {
        return Err(syn::Error::new_spanned(
            &item_fn.sig.ident,
            "abilities whose allowed_actors includes SurfaceClient must declare \
             a non-empty required_scopes = [...] or explicitly opt out with \
             no_scope_required (ADR-0102 §7.6, W1-B)",
        ));
    }

    let fn_ident = item_fn.sig.ident.clone();
    let inner_ident = format_ident!("__{}_ability_impl", fn_ident);
    let erased_ident = format_ident!("__{}_erased", fn_ident);
    let input_schema_ident = format_ident!("__{}_input_schema", fn_ident);
    let output_schema_ident = format_ident!("__{}_output_schema", fn_ident);
    let error_kind_ident = format_ident!("__{}_ability_error_kind", fn_ident);

    let static_suffix = sanitize_ident_suffix(&args.name);
    let descriptor_ident = format_ident!("__ABILITY_DESCRIPTOR_{}", static_suffix);
    let full_descriptor_ident = format_ident!("__ABILITY_DESCRIPTOR_FULL_{}", static_suffix);
    let subscriber_ident = format_ident!("__ABILITY_EVALUATE_SUBSCRIBER_{}", static_suffix);
    let actors_ident = format_ident!("__ABILITY_DESCRIPTOR_ACTORS_{}", static_suffix);
    let modes_ident = format_ident!("__ABILITY_DESCRIPTOR_MODES_{}", static_suffix);
    let composes_ident = format_ident!("__ABILITY_DESCRIPTOR_COMPOSES_{}", static_suffix);
    let mutates_ident = format_ident!("__ABILITY_DESCRIPTOR_MUTATES_{}", static_suffix);
    let signals_ident = format_ident!("__ABILITY_DESCRIPTOR_SIGNALS_{}", static_suffix);
    let scopes_ident = format_ident!("__ABILITY_DESCRIPTOR_SCOPES_{}", static_suffix);

    let mut inner_fn = item_fn.clone();
    inner_fn.sig.ident = inner_ident.clone();
    inner_fn.vis = syn::Visibility::Inherited;

    let vis = &item_fn.vis;
    let attrs = &item_fn.attrs;
    let wrapper_sig = &item_fn.sig;

    let name = args.name.as_str();
    let version = args.version.as_str();
    let schema_version = args.schema_version;
    let requires_confirmation = args.requires_confirmation;
    let may_publish = args.may_publish;
    let experimental = args.experimental;
    let registered_at_static = match args.registered_at.as_deref() {
        Some(value) => quote! { Some(#value) },
        None => quote! { None },
    };
    let category_expr = args.category.registry_expr();
    let category_str = args.category.as_str();

    let actor_exprs: Vec<_> = args
        .allowed_actors
        .iter()
        .map(ActorArg::registry_expr)
        .collect();
    let mode_exprs: Vec<_> = args
        .allowed_modes
        .iter()
        .map(ExecutionModeArg::registry_expr)
        .collect();
    let compose_exprs: Vec<_> = args
        .composes
        .iter()
        .map(ComposeArg::registry_expr)
        .collect();
    let mutates_exprs: Vec<_> = detected.iter().map(|path| quote! { #path }).collect();
    let signal_exprs = args
        .signal_policy
        .emits_on_output_change
        .iter()
        .map(|signal| quote! { #signal })
        .collect::<Vec<_>>();
    let actor_count = actor_exprs.len();
    let mode_count = mode_exprs.len();
    let compose_count = compose_exprs.len();
    let mutates_count = mutates_exprs.len();
    let signal_count = signal_exprs.len();
    let signal_coalesce = args.signal_policy.coalesce;
    let scope_exprs: Vec<_> = args
        .required_scopes
        .iter()
        .map(|scope| quote! { #scope })
        .collect();
    let scope_count = scope_exprs.len();
    let mcp_exposure_expr = args.mcp_exposure.registry_expr();
    let client_side_executable = args.client_side_executable;
    let rate_limit_expr = args
        .rate_limit
        .map(|limit| limit.registry_expr())
        .unwrap_or_else(|| quote! { None });
    let experimental_cfg = if experimental {
        quote! { #[cfg(feature = "experimental")] }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #experimental_cfg
        #inner_fn

        #experimental_cfg
        pub static #subscriber_ident: ::std::sync::OnceLock<::std::sync::Arc<crate::observability::EvaluateModeSubscriber>> =
            ::std::sync::OnceLock::new();

        #experimental_cfg
        #(#attrs)*
        #[::tracing::instrument(
            level = "info",
            skip_all,
            fields(
                invocation_id = ::tracing::field::Empty,
                ability_name = #name,
                ability_category = #category_str,
                actor = ::tracing::field::Empty,
                mode = ::tracing::field::Empty,
                started_at = ::tracing::field::Empty,
                ended_at = ::tracing::field::Empty,
                outcome = ::tracing::field::Empty,
                duration_ms = ::tracing::field::Empty
            )
        )]
        #vis #wrapper_sig {
            let __invocation_id = ::uuid::Uuid::new_v4();
            let __started_at = ::chrono::Utc::now(); // dos-210-grandfathered: macro-emitted instrumentation clock.
            let __span = ::tracing::Span::current();
            let __actor = format!("{:?}", #ctx_ident.actor);
            let __mode = #ctx_ident.mode().as_str().to_string();
            __span.record("invocation_id", &::tracing::field::display(&__invocation_id));
            __span.record("actor", &::tracing::field::display(&__actor));
            __span.record("mode", &::tracing::field::display(&__mode));
            __span.record("started_at", &::tracing::field::display(&__started_at));

            let __ability_result = #inner_ident(#ctx_ident, #input_ident).await;

            let __ended_at = ::chrono::Utc::now(); // dos-210-grandfathered: macro-emitted instrumentation clock.
            let __duration_ms = __ended_at
                .signed_duration_since(__started_at)
                .num_milliseconds()
                .max(0) as u64;
            let __outcome_kind = match &__ability_result {
                Ok(_) => None,
                Err(__err) => Some(#error_kind_ident(&__err.kind)),
            };
            let __outcome_label = if __outcome_kind.is_some() { "err" } else { "ok" };
            __span.record("ended_at", &::tracing::field::display(&__ended_at));
            __span.record("outcome", &::tracing::field::display(&__outcome_label));
            __span.record("duration_ms", &__duration_ms);

            if #ctx_ident.mode() == crate::services::context::ExecutionMode::Evaluate {
                if let Some(__subscriber) = #subscriber_ident.get() {
                    let __outcome = match __outcome_kind {
                        Some(__kind) => crate::observability::Outcome::Err { kind: __kind },
                        None => crate::observability::Outcome::Ok,
                    };
                    __subscriber.record(crate::observability::InvocationRecord {
                        invocation_id: __invocation_id,
                        ability_name: #name.to_string(),
                        ability_category: #category_str.to_string(),
                        actor: __actor,
                        mode: __mode,
                        started_at: __started_at,
                        ended_at: __ended_at,
                        outcome: __outcome,
                        duration_ms: __duration_ms,
                    });
                }
            }

            __ability_result
        }

        #experimental_cfg
        fn #error_kind_ident(kind: &crate::abilities::registry::AbilityErrorKind) -> String {
            match kind {
                crate::abilities::registry::AbilityErrorKind::Validation => "Validation".to_string(),
                crate::abilities::registry::AbilityErrorKind::Capability => "Capability".to_string(),
                crate::abilities::registry::AbilityErrorKind::OptionalComposedReadFailed { .. } => {
                    "OptionalComposedReadFailed".to_string()
                }
                crate::abilities::registry::AbilityErrorKind::SubjectNotOwned => {
                    "SubjectNotOwned".to_string()
                }
                crate::abilities::registry::AbilityErrorKind::HardError(_) => "HardError".to_string(),
            }
        }

        #experimental_cfg
        fn #erased_ident<'a>(
            ctx: &'a crate::abilities::registry::AbilityContext<'a>,
            input_json: ::serde_json::Value,
        ) -> ::std::pin::Pin<
            Box<
                dyn ::std::future::Future<
                    Output = Result<::serde_json::Value, crate::abilities::registry::AbilityError>
                > + Send + 'a
            >
        > {
            Box::pin(async move {
                let input: #input_ty = ::serde_json::from_value(input_json).map_err(|error| {
                    crate::abilities::registry::AbilityError {
                        kind: crate::abilities::registry::AbilityErrorKind::Validation,
                        message: format!("invalid input for ability `{}`: {}", #name, error),
                    }
                })?;
                let output = #fn_ident(ctx, input).await?;
                ::serde_json::to_value(&output).map_err(|error| {
                    crate::abilities::registry::AbilityError {
                        kind: crate::abilities::registry::AbilityErrorKind::Validation,
                        message: format!("invalid output for ability `{}`: {}", #name, error),
                    }
                })
            })
        }

        #experimental_cfg
        fn #input_schema_ident() -> ::serde_json::Value {
            let mut __schema = ::serde_json::to_value(::schemars::schema_for!(#input_ty))
                .expect("schemars input schema should serialize");
            crate::abilities::registry::close_schema_objects(&mut __schema);
            __schema
        }

        #experimental_cfg
        fn #output_schema_ident() -> ::serde_json::Value {
            ::serde_json::to_value(::schemars::schema_for!(
                crate::abilities::provenance::AbilityOutput::<#output_ty>
            ))
                .expect("schemars output schema should serialize")
        }

        #experimental_cfg
        pub static #actors_ident: [crate::abilities::registry::ActorKind; #actor_count] =
            [#(#actor_exprs),*];
        #experimental_cfg
        pub static #modes_ident: [crate::services::context::ExecutionMode; #mode_count] =
            [#(#mode_exprs),*];
        #experimental_cfg
        pub static #composes_ident: [crate::abilities::registry::ComposesEntry; #compose_count] =
            [#(#compose_exprs),*];
        #experimental_cfg
        pub static #mutates_ident: [&'static str; #mutates_count] =
            [#(#mutates_exprs),*];
        #experimental_cfg
        pub static #signals_ident: [&'static str; #signal_count] =
            [#(#signal_exprs),*];
        #experimental_cfg
        pub static #scopes_ident: [&'static str; #scope_count] =
            [#(#scope_exprs),*];

        #experimental_cfg
        #[allow(dead_code)]
        pub fn #full_descriptor_ident() -> crate::abilities::registry::AbilityDescriptor {
            crate::abilities::registry::AbilityDescriptor {
                name: #name,
                version: #version,
                schema_version: #schema_version,
                category: #category_expr,
                policy: crate::abilities::registry::AbilityPolicy {
                    allowed_actors: &#actors_ident,
                    allowed_modes: &#modes_ident,
                    requires_confirmation: #requires_confirmation,
                    may_publish: #may_publish,
                    required_scopes: &#scopes_ident,
                    mcp_exposure: #mcp_exposure_expr,
                    client_side_executable: #client_side_executable,
                    rate_limit: #rate_limit_expr,
                },
                composes: &#composes_ident,
                mutates: &#mutates_ident,
                experimental: #experimental,
                registered_at: #registered_at_static,
                signal_policy: crate::abilities::registry::SignalPolicy {
                    emits_on_output_change: &#signals_ident,
                    coalesce: #signal_coalesce,
                },
                invoke_erased: #erased_ident,
                input_schema: #input_schema_ident,
                output_schema: #output_schema_ident,
            }
        }

        #experimental_cfg
        pub static #descriptor_ident: crate::abilities::registry::AbilityDescriptor =
            crate::abilities::registry::AbilityDescriptor {
                name: #name,
                version: #version,
                schema_version: #schema_version,
                category: #category_expr,
                policy: crate::abilities::registry::AbilityPolicy {
                    allowed_actors: &#actors_ident,
                    allowed_modes: &#modes_ident,
                    requires_confirmation: #requires_confirmation,
                    may_publish: #may_publish,
                    required_scopes: &#scopes_ident,
                    mcp_exposure: #mcp_exposure_expr,
                    client_side_executable: #client_side_executable,
                    rate_limit: #rate_limit_expr,
                },
                composes: &#composes_ident,
                mutates: &#mutates_ident,
                experimental: #experimental,
                registered_at: #registered_at_static,
                signal_policy: crate::abilities::registry::SignalPolicy {
                    emits_on_output_change: &#signals_ident,
                    coalesce: #signal_coalesce,
                },
                invoke_erased: #erased_ident,
                input_schema: #input_schema_ident,
                output_schema: #output_schema_ident,
            };

        #experimental_cfg
        ::inventory::submit! {
            crate::abilities::registry::AbilityDescriptor {
                name: #name,
                version: #version,
                schema_version: #schema_version,
                category: #category_expr,
                policy: crate::abilities::registry::AbilityPolicy {
                    allowed_actors: &#actors_ident,
                    allowed_modes: &#modes_ident,
                    requires_confirmation: #requires_confirmation,
                    may_publish: #may_publish,
                    required_scopes: &#scopes_ident,
                    mcp_exposure: #mcp_exposure_expr,
                    client_side_executable: #client_side_executable,
                    rate_limit: #rate_limit_expr,
                },
                composes: &#composes_ident,
                mutates: &#mutates_ident,
                experimental: #experimental,
                registered_at: #registered_at_static,
                signal_policy: crate::abilities::registry::SignalPolicy {
                    emits_on_output_change: &#signals_ident,
                    coalesce: #signal_coalesce,
                },
                invoke_erased: #erased_ident,
                input_schema: #input_schema_ident,
                output_schema: #output_schema_ident,
            }
        }
    };

    Ok(expanded)
}

struct BoundaryModuleSource {
    items: Vec<Item>,
    containing_module_path: Vec<String>,
}

fn same_source_module_items_for_ability(item_fn: &ItemFn) -> Option<BoundaryModuleSource> {
    let source_path = item_fn.sig.ident.span().local_file()?;
    if let Some(crate_root) = crate_root_source_path(&source_path) {
        if let Some(module_source) = parse_module_source(&crate_root, &item_fn.sig.ident) {
            return Some(module_source);
        }
    }

    parse_module_source(&source_path, &item_fn.sig.ident)
}

fn parse_module_source(source_path: &Path, fn_ident: &Ident) -> Option<BoundaryModuleSource> {
    let source = fs::read_to_string(source_path).ok()?;
    let mut file = syn::parse_file(&source).ok()?;
    let mut visited = HashSet::new();
    hydrate_external_modules(&mut file.items, source_path, &mut visited);
    module_path_containing_ability_fn(&file.items, fn_ident, &mut Vec::new()).map(
        |containing_module_path| BoundaryModuleSource {
            items: file.items,
            containing_module_path,
        },
    )
}

fn crate_root_source_path(source_path: &Path) -> Option<PathBuf> {
    for ancestor in source_path.parent()?.ancestors() {
        if !ancestor.join("Cargo.toml").is_file() {
            continue;
        }
        if !source_path.starts_with(ancestor.join("src")) {
            continue;
        }

        let lib = ancestor.join("src/lib.rs");
        if lib.is_file() {
            return Some(lib);
        }

        let main = ancestor.join("src/main.rs");
        if main.is_file() {
            return Some(main);
        }
    }

    None
}

fn hydrate_external_modules(
    items: &mut [Item],
    source_path: &Path,
    visited: &mut HashSet<PathBuf>,
) {
    for item in items {
        let Item::Mod(module) = item else {
            continue;
        };

        if let Some((_, nested_items)) = &mut module.content {
            hydrate_external_modules(nested_items, source_path, visited);
            continue;
        }

        let Some(module_path) = module_source_path(source_path, &module.ident) else {
            continue;
        };
        let visited_path = module_path
            .canonicalize()
            .unwrap_or_else(|_| module_path.clone());
        if !visited.insert(visited_path) {
            continue;
        }

        let Ok(source) = fs::read_to_string(&module_path) else {
            continue;
        };
        let Ok(mut file) = syn::parse_file(&source) else {
            continue;
        };
        hydrate_external_modules(&mut file.items, &module_path, visited);
        module.content = Some((syn::token::Brace::default(), file.items));
        module.semi = None;
    }
}

fn module_source_path(source_path: &Path, ident: &Ident) -> Option<PathBuf> {
    module_source_candidates(source_path, ident)
        .into_iter()
        .find(|candidate| candidate.is_file())
}

fn module_source_candidates(source_path: &Path, ident: &Ident) -> Vec<PathBuf> {
    let module_name = ident.to_string();
    let Some(parent) = source_path.parent() else {
        return Vec::new();
    };

    let file_name = source_path.file_name().and_then(|name| name.to_str());
    let base_dir = if matches!(file_name, Some("lib.rs" | "main.rs" | "mod.rs")) {
        parent.to_path_buf()
    } else if let Some(stem) = source_path.file_stem() {
        parent.join(stem)
    } else {
        parent.to_path_buf()
    };

    vec![
        base_dir.join(format!("{module_name}.rs")),
        base_dir.join(module_name).join("mod.rs"),
    ]
}

fn module_path_containing_ability_fn(
    items: &[Item],
    fn_ident: &Ident,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    if items.iter().any(|item| {
        matches!(
            item,
            Item::Fn(candidate)
                if candidate.sig.ident == *fn_ident && has_ability_attribute(&candidate.attrs)
        )
    }) {
        return Some(path.clone());
    }

    for item in items {
        if let Item::Mod(module) = item {
            let Some((_, nested_items)) = &module.content else {
                continue;
            };
            path.push(module.ident.to_string());
            if let Some(found) = module_path_containing_ability_fn(nested_items, fn_ident, path) {
                path.pop();
                return Some(found);
            }
            path.pop();
        }
    }

    None
}

fn has_ability_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "ability")
    })
}

fn ability_signature_parts(item_fn: &ItemFn) -> syn::Result<(Ident, Ident, Type)> {
    let mut inputs = item_fn.sig.inputs.iter();
    let Some(first) = inputs.next() else {
        return Err(syn::Error::new_spanned(
            &item_fn.sig.ident,
            "#[ability] functions must take &AbilityContext as the first parameter",
        ));
    };
    let Some(second) = inputs.next() else {
        return Err(syn::Error::new_spanned(
            &item_fn.sig.ident,
            "#[ability] functions must take exactly one input payload after &AbilityContext",
        ));
    };
    if inputs.next().is_some() {
        return Err(syn::Error::new_spanned(
            &item_fn.sig.inputs,
            "#[ability] functions may not accept handles after the input payload",
        ));
    }

    let FnArg::Typed(ctx_arg) = first else {
        return Err(syn::Error::new_spanned(
            first,
            "#[ability] functions must not use a self receiver",
        ));
    };
    if !is_ability_context_ref(&ctx_arg.ty) {
        return Err(syn::Error::new_spanned(
            &ctx_arg.ty,
            "first #[ability] parameter must be &AbilityContext",
        ));
    }
    let ctx_ident = pat_ident(&ctx_arg.pat)?;

    let FnArg::Typed(input_arg) = second else {
        return Err(syn::Error::new_spanned(
            second,
            "#[ability] input payload must be a typed parameter",
        ));
    };
    let input_ident = pat_ident(&input_arg.pat)?;
    Ok((ctx_ident, input_ident, (*input_arg.ty).clone()))
}

fn ability_result_output_ty(item_fn: &ItemFn) -> syn::Result<Type> {
    let ReturnType::Type(_, ty) = &item_fn.sig.output else {
        return Err(syn::Error::new_spanned(
            &item_fn.sig.ident,
            "#[ability] functions must return AbilityResult<Output>",
        ));
    };

    let Type::Path(type_path) = ty.as_ref() else {
        return Err(syn::Error::new_spanned(
            ty,
            "#[ability] functions must return AbilityResult<Output>",
        ));
    };

    let Some(segment) = type_path.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            ty,
            "#[ability] functions must return AbilityResult<Output>",
        ));
    };
    if segment.ident != "AbilityResult" {
        return Err(syn::Error::new_spanned(
            ty,
            "#[ability] functions must return AbilityResult<Output>",
        ));
    }

    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            ty,
            "#[ability] functions must return AbilityResult<Output>",
        ));
    };
    let Some(GenericArgument::Type(output_ty)) = arguments.args.first() else {
        return Err(syn::Error::new_spanned(
            ty,
            "#[ability] functions must return AbilityResult<Output>",
        ));
    };

    Ok(output_ty.clone())
}

fn is_ability_context_ref(ty: &Type) -> bool {
    let Type::Reference(reference) = ty else {
        return false;
    };
    let Type::Path(type_path) = reference.elem.as_ref() else {
        return false;
    };
    type_path
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "AbilityContext")
}

fn pat_ident(pat: &Pat) -> syn::Result<Ident> {
    match pat {
        Pat::Ident(pat_ident) => Ok(pat_ident.ident.clone()),
        _ => Err(syn::Error::new_spanned(
            pat,
            "#[ability] parameters must use simple identifiers",
        )),
    }
}

fn sanitize_ident_suffix(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() || out.as_bytes()[0].is_ascii_digit() {
        out.insert_str(0, "ABILITY_");
    }
    out
}

struct AbilityArgs {
    name: String,
    category: AbilityCategoryArg,
    version: String,
    schema_version: u32,
    allowed_actors: Vec<ActorArg>,
    allowed_modes: Vec<ExecutionModeArg>,
    requires_confirmation: bool,
    may_publish: bool,
    composes: Vec<ComposeArg>,
    experimental: bool,
    registered_at: Option<String>,
    signal_policy: SignalPolicyArg,
    /// W1-B: scope vocabulary a SurfaceClient instance must carry.
    /// Defaults to empty (preserves v1.4.0/v1.4.1 semantics for
    /// non-SurfaceClient abilities; the compile-error gate ensures any
    /// SurfaceClient-reachable ability declares scopes or opts out).
    required_scopes: Vec<String>,
    /// W1-B: MCP enumeration tier per ADR-0102 §7.1 (W0-D amendment).
    /// Default `McpExposureArg::None`.
    mcp_exposure: McpExposureArg,
    /// W1-B: whether a SurfaceClient may invoke after policy/scope/
    /// actor checks pass. Default `false`.
    client_side_executable: bool,
    /// W1-B: explicit opt-out from the SurfaceClient scope-declaration
    /// gate. Allows `allowed_actors: [SurfaceClient]` with empty
    /// `required_scopes`. Boolean flag form: `no_scope_required = true`.
    no_scope_required: bool,
    /// W2-D: optional lower-only rate-limit override.
    rate_limit: Option<AbilityRateLimitArg>,
}

impl Parse for AbilityArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut name = None;
        let mut category = None;
        let mut version = None;
        let mut schema_version = None;
        let mut allowed_actors = None;
        let mut allowed_modes = None;
        let mut requires_confirmation = None;
        let mut may_publish = None;
        let mut composes = Vec::new();
        let mut experimental = false;
        let mut registered_at = None;
        let mut signal_policy = SignalPolicyArg::default();
        let mut required_scopes: Vec<String> = Vec::new();
        let mut mcp_exposure = McpExposureArg::None;
        let mut client_side_executable = false;
        let mut no_scope_required = false;
        let mut rate_limit = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "name" => name = Some(input.parse::<LitStr>()?.value()),
                "category" => category = Some(parse_category(input)?),
                "version" => version = Some(input.parse::<LitStr>()?.value()),
                "schema_version" => {
                    schema_version = Some(input.parse::<LitInt>()?.base10_parse::<u32>()?);
                }
                "allowed_actors" => allowed_actors = Some(parse_actor_array(input)?),
                "allowed_modes" => allowed_modes = Some(parse_mode_array(input)?),
                "requires_confirmation" => {
                    requires_confirmation = Some(input.parse::<LitBool>()?.value);
                }
                "may_publish" => may_publish = Some(input.parse::<LitBool>()?.value),
                "composes" => composes = parse_composes_array(input)?,
                "experimental" => experimental = input.parse::<LitBool>()?.value,
                "registered_at" => registered_at = Some(input.parse::<LitStr>()?.value()),
                "signal_policy" => signal_policy = parse_signal_policy(input)?,
                "required_scopes" => required_scopes = parse_string_array(input)?,
                "mcp_exposure" => mcp_exposure = parse_mcp_exposure(input)?,
                "client_side_executable" => {
                    client_side_executable = input.parse::<LitBool>()?.value;
                }
                "no_scope_required" => no_scope_required = input.parse::<LitBool>()?.value,
                "rate_limit" => rate_limit = Some(parse_rate_limit(input)?),
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown #[ability] attribute `{other}`"),
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            name: name.ok_or_else(|| input.error("missing required #[ability] name"))?,
            category: category
                .ok_or_else(|| input.error("missing required #[ability] category"))?,
            version: version.ok_or_else(|| input.error("missing required #[ability] version"))?,
            schema_version: schema_version
                .ok_or_else(|| input.error("missing required #[ability] schema_version"))?,
            allowed_actors: allowed_actors
                .ok_or_else(|| input.error("missing required #[ability] allowed_actors"))?,
            allowed_modes: allowed_modes
                .ok_or_else(|| input.error("missing required #[ability] allowed_modes"))?,
            requires_confirmation: requires_confirmation
                .ok_or_else(|| input.error("missing required #[ability] requires_confirmation"))?,
            may_publish: may_publish
                .ok_or_else(|| input.error("missing required #[ability] may_publish"))?,
            composes,
            experimental,
            registered_at,
            signal_policy,
            required_scopes,
            mcp_exposure,
            client_side_executable,
            no_scope_required,
            rate_limit,
        })
    }
}

#[derive(Clone, Copy)]
enum AbilityCategoryArg {
    Read,
    Transform,
    Publish,
    Maintenance,
}

impl AbilityCategoryArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Read => "Read",
            Self::Transform => "Transform",
            Self::Publish => "Publish",
            Self::Maintenance => "Maintenance",
        }
    }

    fn registry_expr(self) -> proc_macro2::TokenStream {
        match self {
            Self::Read => quote! { crate::abilities::registry::AbilityCategory::Read },
            Self::Transform => quote! { crate::abilities::registry::AbilityCategory::Transform },
            Self::Publish => quote! { crate::abilities::registry::AbilityCategory::Publish },
            Self::Maintenance => {
                quote! { crate::abilities::registry::AbilityCategory::Maintenance }
            }
        }
    }
}

fn parse_category(input: ParseStream<'_>) -> syn::Result<AbilityCategoryArg> {
    let ident: Ident = input.parse()?;
    match ident.to_string().as_str() {
        "Read" => Ok(AbilityCategoryArg::Read),
        "Transform" => Ok(AbilityCategoryArg::Transform),
        "Publish" => Ok(AbilityCategoryArg::Publish),
        "Maintenance" => Ok(AbilityCategoryArg::Maintenance),
        other => Err(syn::Error::new(
            ident.span(),
            format!("unknown ability category `{other}`"),
        )),
    }
}

enum ActorArg {
    Agent,
    User,
    Admin,
    System,
    /// ADR-0111 §8: third-party local surface invoking on
    /// behalf of a paired user. Carries instance identity + scope grant
    /// at runtime; the macro records membership in `allowed_actors` here
    /// to drive the W1-B compile-error gate.
    SurfaceClient,
}

impl ActorArg {
    /// Emit the registry token for this actor's `ActorKind` discriminator.
    ///
    /// Per ADR-0102 §7.6 (W0-D amended 2026-05-10) W1-B,
    /// `AbilityPolicy::allowed_actors` is a `&'static [ActorKind]` slice
    /// rather than `&'static [Actor]`. This separation lets `SurfaceClient`
    /// — a struct variant carrying per-invocation owned state — be listed
    /// in `allowed_actors` declarations without breaking `inventory::submit!`'s
    /// `static`-only descriptor invariant. The W1-B compile-error gate
    /// (parse-time, see `ability` attribute handler) still enforces the
    /// `required_scopes` / `no_scope_required` requirement for any ability
    /// that admits `SurfaceClient`.
    fn registry_expr(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Agent => quote! { crate::abilities::registry::ActorKind::Agent },
            Self::User => quote! { crate::abilities::registry::ActorKind::User },
            Self::Admin => quote! { crate::abilities::registry::ActorKind::Admin },
            Self::System => quote! { crate::abilities::registry::ActorKind::System },
            Self::SurfaceClient => {
                quote! { crate::abilities::registry::ActorKind::SurfaceClient }
            }
        }
    }
}

fn parse_actor_array(input: ParseStream<'_>) -> syn::Result<Vec<ActorArg>> {
    let content;
    bracketed!(content in input);
    let values = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
    values
        .into_iter()
        .map(|ident| match ident.to_string().as_str() {
            "Agent" => Ok(ActorArg::Agent),
            "User" => Ok(ActorArg::User),
            "Admin" => Ok(ActorArg::Admin),
            "System" => Ok(ActorArg::System),
            "SurfaceClient" => Ok(ActorArg::SurfaceClient),
            other => Err(syn::Error::new(
                ident.span(),
                format!("unknown actor `{other}`"),
            )),
        })
        .collect()
}

#[derive(Clone, Copy)]
enum McpExposureArg {
    None,
    MetadataOnly,
    Invocable,
}

impl McpExposureArg {
    fn registry_expr(self) -> proc_macro2::TokenStream {
        match self {
            Self::None => quote! { crate::abilities::registry::McpExposure::None },
            Self::MetadataOnly => {
                quote! { crate::abilities::registry::McpExposure::MetadataOnly }
            }
            Self::Invocable => quote! { crate::abilities::registry::McpExposure::Invocable },
        }
    }
}

fn parse_mcp_exposure(input: ParseStream<'_>) -> syn::Result<McpExposureArg> {
    let ident: Ident = input.parse()?;
    match ident.to_string().as_str() {
        "None" => Ok(McpExposureArg::None),
        "MetadataOnly" => Ok(McpExposureArg::MetadataOnly),
        "Invocable" => Ok(McpExposureArg::Invocable),
        other => Err(syn::Error::new(
            ident.span(),
            format!("unknown mcp_exposure `{other}` (expected None | MetadataOnly | Invocable)"),
        )),
    }
}

#[derive(Clone, Copy)]
struct AbilityRateLimitArg {
    requests_per_minute: u32,
    burst_per_second: u32,
}

impl AbilityRateLimitArg {
    fn registry_expr(self) -> proc_macro2::TokenStream {
        let requests_per_minute = self.requests_per_minute;
        let burst_per_second = self.burst_per_second;
        quote! {
            Some(crate::abilities::registry::AbilityRateLimit {
                requests_per_minute: #requests_per_minute,
                burst_per_second: #burst_per_second,
            })
        }
    }
}

fn parse_rate_limit(input: ParseStream<'_>) -> syn::Result<AbilityRateLimitArg> {
    let content;
    braced!(content in input);
    let mut requests_per_minute = None;
    let mut burst_per_second = None;

    while !content.is_empty() {
        let key: Ident = content.parse()?;
        content.parse::<Token![=]>()?;
        match key.to_string().as_str() {
            "requests_per_minute" => {
                requests_per_minute = Some(content.parse::<LitInt>()?.base10_parse::<u32>()?);
            }
            "burst_per_second" => {
                burst_per_second = Some(content.parse::<LitInt>()?.base10_parse::<u32>()?);
            }
            other => {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown rate_limit field `{other}`"),
                ));
            }
        }
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }

    Ok(AbilityRateLimitArg {
        requests_per_minute: requests_per_minute
            .ok_or_else(|| content.error("missing rate_limit.requests_per_minute"))?,
        burst_per_second: burst_per_second
            .ok_or_else(|| content.error("missing rate_limit.burst_per_second"))?,
    })
}

fn parse_string_array(input: ParseStream<'_>) -> syn::Result<Vec<String>> {
    let content;
    bracketed!(content in input);
    let mut values = Vec::new();
    while !content.is_empty() {
        values.push(content.parse::<LitStr>()?.value());
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(values)
}

enum ExecutionModeArg {
    Live,
    Simulate,
    Evaluate,
}

impl ExecutionModeArg {
    fn registry_expr(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Live => quote! { crate::services::context::ExecutionMode::Live },
            Self::Simulate => quote! { crate::services::context::ExecutionMode::Simulate },
            Self::Evaluate => quote! { crate::services::context::ExecutionMode::Evaluate },
        }
    }
}

fn parse_mode_array(input: ParseStream<'_>) -> syn::Result<Vec<ExecutionModeArg>> {
    let content;
    bracketed!(content in input);
    let values = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
    values
        .into_iter()
        .map(|ident| match ident.to_string().as_str() {
            "Live" => Ok(ExecutionModeArg::Live),
            "Simulate" => Ok(ExecutionModeArg::Simulate),
            "Evaluate" => Ok(ExecutionModeArg::Evaluate),
            other => Err(syn::Error::new(
                ident.span(),
                format!("unknown execution mode `{other}`"),
            )),
        })
        .collect()
}

struct ComposeArg {
    id: String,
    ability: String,
    optional: bool,
}

impl ComposeArg {
    fn registry_expr(&self) -> proc_macro2::TokenStream {
        let id = self.id.as_str();
        let ability = self.ability.as_str();
        let optional = self.optional;
        quote! {
            crate::abilities::registry::ComposesEntry {
                id: crate::abilities::provenance::CompositionId::from_static(#id),
                ability: #ability,
                optional: #optional,
            }
        }
    }
}

fn parse_composes_array(input: ParseStream<'_>) -> syn::Result<Vec<ComposeArg>> {
    let content;
    bracketed!(content in input);
    let mut entries = Vec::new();
    while !content.is_empty() {
        let entry;
        braced!(entry in content);
        entries.push(parse_compose_entry(&entry)?);
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(entries)
}

fn parse_compose_entry(input: ParseStream<'_>) -> syn::Result<ComposeArg> {
    let mut id = None;
    let mut ability = None;
    let mut optional = None;
    while !input.is_empty() {
        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        match key.to_string().as_str() {
            "id" => id = Some(input.parse::<LitStr>()?.value()),
            "ability" => ability = Some(input.parse::<LitStr>()?.value()),
            "optional" => optional = Some(input.parse::<LitBool>()?.value),
            other => {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown composes field `{other}`"),
                ));
            }
        }
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
    }
    Ok(ComposeArg {
        id: id.ok_or_else(|| input.error("missing composes.id"))?,
        ability: ability.ok_or_else(|| input.error("missing composes.ability"))?,
        optional: optional.ok_or_else(|| input.error("missing composes.optional"))?,
    })
}

#[derive(Default)]
struct SignalPolicyArg {
    emits_on_output_change: Vec<String>,
    coalesce: bool,
}

fn parse_signal_policy(input: ParseStream<'_>) -> syn::Result<SignalPolicyArg> {
    let content;
    braced!(content in input);
    let mut policy = SignalPolicyArg::default();
    while !content.is_empty() {
        let key: Ident = content.parse()?;
        content.parse::<Token![=]>()?;
        match key.to_string().as_str() {
            "emits_on_output_change" => {
                let array;
                bracketed!(array in content);
                let mut values = Vec::new();
                while !array.is_empty() {
                    values.push(parse_string_or_ident(&array)?);
                    if array.peek(Token![,]) {
                        array.parse::<Token![,]>()?;
                    }
                }
                policy.emits_on_output_change = values;
            }
            "coalesce" => policy.coalesce = content.parse::<LitBool>()?.value,
            other => {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown signal_policy field `{other}`"),
                ));
            }
        }
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }
    Ok(policy)
}

fn parse_string_or_ident(input: ParseStream<'_>) -> syn::Result<String> {
    if input.peek(LitStr) {
        Ok(input.parse::<LitStr>()?.value())
    } else {
        Ok(input.parse::<Ident>()?.to_string())
    }
}
