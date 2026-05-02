//! Splash Host (Layer 2) — reads templates, enforces widget / local function
//! whitelists, binds state, injects host-owned attribution chrome, emits
//! Splash DSL strings compatible with the existing `content.splash_card.set_text(...)`
//! timeline seam.
//!
//! Contract: see `specs/task-agent-to-app-splash-host-evolution.spec.md`
//! and `docs/design/agent-to-app-design.md` §5 Layer 2 + §6 W5/W7 + §6.1
//! attribution.
//!
//! v1 implements static template load/preflight, state binding, cache,
//! fallback primitives, path-scoped replace/remove updates, and action
//! classification. Matrix action transport is intentionally left to the
//! L2 actions spec.

use std::sync::OnceLock;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value as JsonValue;

use super::{capability_descriptors, local_functions, mission_dashboard, mission_room, news, template_cache, weather, widget_manifest};

/// Host-owned attribution envelope rendered around every template. Values
/// come from the Robrix-side static `CapabilityDescriptor` table (see
/// `capability_descriptors.rs`, plan Step 1.3a); templates **cannot**
/// override these fields — the host validates that at template load time.
#[derive(Debug, Clone)]
pub struct AttributionChrome {
    pub capability_id: String,
    pub display_name: String,
    pub icon_url: Option<String>,
    pub trust_badge: TrustBadge,
}

/// Trust classification of the capability that produced the app reply.
///
/// v1 only emits `Builtin`. `Verified` / `Unverified` are reserved for the
/// future envelope-chrome path (spec `§Attribution Chrome` future path:
/// third-party / orchestrator-gated capabilities).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustBadge {
    Builtin,
    Verified,
    Unverified,
}

/// Opaque pre-validated template form. v1 wraps the source text plus
/// metadata about `$state.path` binding sites and `${fn(...)}` call sites
/// captured during `load_template`. Fields are private because only the
/// host's own `render_to_splash` should traverse them.
#[derive(Debug, Clone)]
pub struct TemplateHandle {
    capability_id: String,
    template_id: String,
    /// Source Splash DSL text from the `.splash` file (post-validation).
    #[allow(dead_code)]
    source: String,
    // Future fields (plan Step 1.7): AST nodes for `$state.path` sites +
    // `${fn(...)}` sites. Kept opaque so the parsing strategy can change
    // without breaking downstream `RenderedApp::render` impls.
}

impl TemplateHandle {
    pub fn capability_id(&self) -> &str {
        &self.capability_id
    }

    pub fn template_id(&self) -> &str {
        &self.template_id
    }

    /// Test-only constructor used by unit tests that need a handle without
    /// exercising `SplashHost::load_template`. Production code must go
    /// through `load_template` so validation is enforced.
    #[cfg(test)]
    pub(crate) fn new_for_test(capability_id: impl Into<String>, template_id: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            capability_id: capability_id.into(),
            template_id: template_id.into(),
            source: source.into(),
        }
    }
}

/// Errors returned by any `SplashHost` method. Every variant carries enough
/// information to produce the spec's structured validation error payload
/// (`{code, path, message}`) when the repair loop lands with the first
/// generative capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    /// The requested `(capability_id, template_id)` pair has no
    /// corresponding `.splash` file under `src/home/app_registry/templates/`.
    TemplateNotFound {
        capability_id: String,
        template_id: String,
    },
    /// Template parse failure (malformed Splash DSL). `line` is 1-based.
    ParseError {
        message: String,
        line: usize,
    },
    /// Template references a widget that is not on the `WidgetManifest`,
    /// or whose `trust_level` is not `Public`. `trust_level` is `Some`
    /// when the widget exists but is gated.
    WidgetNotAllowed {
        name: String,
        trust_level: Option<String>,
    },
    /// Template references a `${fn(...)}` or `functionCall.call` whose
    /// name is not on the `LocalFunctionRegistry` (W7).
    LocalFunctionNotAllowed {
        name: String,
    },
    /// Template attempts to set a host-owned attribution field in its
    /// content region (`capability_id`, `display_name`, `icon`,
    /// `trust_badge`). Templates may render content only; the host owns
    /// chrome.
    AttributionFieldInTemplate {
        field: String,
    },
    /// State binding `$state.path` could not be resolved against the
    /// supplied `state` JSON. Carries the JSON Pointer that failed.
    BindingError {
        path: String,
        message: String,
    },
    /// Template references a `$state.path` that the capability schema
    /// does not declare for this `(app_type, app_version)` pair.
    BindingPathNotInSchema {
        path: String,
        app_type: String,
        app_version: u32,
    },
    /// `apply_state_update` received a path-op semantics the v1 host
    /// does not implement (`append`, `splice`, array-index insert, etc.).
    /// v1 only supports `replace` and `remove`; the rest land in
    /// `task-agent-to-app-l2-actions`.
    UpdateOpNotYetSupported {
        op: String,
    },
    /// `load_template` received a capability-declared `TemplateSlot`
    /// whose `kind` is `Generated` (Layer 5b Template-Author LLM path).
    /// v1 does not implement the repair loop; locked behind this error
    /// to prevent silent bypass.
    GeneratedTemplateNotYetSupported,
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TemplateNotFound { capability_id, template_id } => {
                write!(f, "template not found: capability={capability_id} template={template_id}")
            }
            Self::ParseError { message, line } => {
                write!(f, "template parse error at line {line}: {message}")
            }
            Self::WidgetNotAllowed { name, trust_level } => match trust_level {
                Some(level) => write!(f, "widget not allowed: {name} (trust_level={level})"),
                None => write!(f, "widget not allowed: {name} (not on manifest)"),
            },
            Self::LocalFunctionNotAllowed { name } => {
                write!(f, "local function not allowed: {name}")
            }
            Self::AttributionFieldInTemplate { field } => {
                write!(f, "attribution field {field} may not appear in template content")
            }
            Self::BindingError { path, message } => {
                write!(f, "state binding error at {path}: {message}")
            }
            Self::BindingPathNotInSchema { path, app_type, app_version } => {
                write!(
                    f,
                    "binding path not in schema: {path} (app_type={app_type} version={app_version})"
                )
            }
            Self::UpdateOpNotYetSupported { op } => {
                write!(f, "state update op not yet supported in v1: {op}")
            }
            Self::GeneratedTemplateNotYetSupported => {
                f.write_str("generated template slots not yet supported in v1")
            }
        }
    }
}

impl std::error::Error for HostError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub code: &'static str,
    pub path: String,
    pub message: String,
}

impl HostError {
    pub fn to_validation_error(&self) -> ValidationError {
        match self {
            HostError::ParseError { message, line } => ValidationError {
                code: "PARSE_ERROR",
                path: format!("line:{line}"),
                message: message.clone(),
            },
            HostError::WidgetNotAllowed { name, .. } => ValidationError {
                code: "WIDGET_NOT_ALLOWED",
                path: name.clone(),
                message: self.to_string(),
            },
            HostError::LocalFunctionNotAllowed { name } => ValidationError {
                code: "LOCAL_FUNCTION_NOT_ALLOWED",
                path: name.clone(),
                message: self.to_string(),
            },
            HostError::AttributionFieldInTemplate { field } => ValidationError {
                code: "ATTRIBUTION_OVERRIDE",
                path: field.clone(),
                message: self.to_string(),
            },
            HostError::BindingPathNotInSchema { path, .. } => ValidationError {
                code: "BINDING_PATH_NOT_IN_SCHEMA",
                path: path.clone(),
                message: self.to_string(),
            },
            HostError::BindingError { path, .. } => ValidationError {
                code: "BINDING_ERROR",
                path: path.clone(),
                message: self.to_string(),
            },
            HostError::TemplateNotFound { template_id, .. } => ValidationError {
                code: "TEMPLATE_NOT_FOUND",
                path: template_id.clone(),
                message: self.to_string(),
            },
            HostError::UpdateOpNotYetSupported { op } => ValidationError {
                code: "UPDATE_OP_NOT_SUPPORTED",
                path: op.clone(),
                message: self.to_string(),
            },
            HostError::GeneratedTemplateNotYetSupported => ValidationError {
                code: "GENERATED_TEMPLATE_NOT_SUPPORTED",
                path: String::new(),
                message: self.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackReason {
    TemplateFailed {
        capability_id: String,
        preferred_template_id: String,
        underlying: Box<HostError>,
    },
    AllTemplatesFailed {
        final_error: Box<HostError>,
    },
    HostVersionMismatch {
        expected_host_version: u32,
        got: u32,
    },
}

/// Outcome of dispatching a template-emitted action through the host.
#[derive(Debug, Clone)]
pub enum ActionOutcome {
    /// The action name resolved to a remote capability action; payload is
    /// forwarded to the capability's `on_action` handler. v1 threads this
    /// through without touching Matrix transport wiring
    /// (`task-agent-to-app-l2-actions` defines the transport path).
    RemoteDispatch {
        capability_id: String,
        action_id: String,
        payload: JsonValue,
    },
    /// The action name resolved to a W7 local function. The v1 host
    /// classifies it here without a capability round-trip; platform
    /// side effects such as opening a URL are wired by the later L2
    /// action transport.
    LocalFunctionCalled {
        name: String,
    },
    /// The action name did not appear on the capability's W6 action
    /// whitelist or the W7 function registry. Dropped; caller logs.
    UnknownAction {
        action_id: String,
    },
}

/// Path-scoped state update kind. v1 accepts only `Replace` and `Remove`;
/// other variants return `HostError::UpdateOpNotYetSupported` so a future
/// L2 action path can extend without retrofitting callers.
#[derive(Debug, Clone)]
pub enum StateUpdateOp<'a> {
    Replace { value: &'a JsonValue },
    Remove,
    Append { value: &'a JsonValue },
    // Future (L2): Splice, ArrayIndexInsert, ...
}

/// Layer 2 Splash Host. See module-level docs for the contract.
pub trait SplashHost: Send + Sync {
    /// Load a static `.splash` template for `(capability_id, template_id)`.
    /// Runs W5, W7, and attribution-lock validation in one pass; returns
    /// `Ok(TemplateHandle)` or the first validation error encountered.
    fn load_template(
        &self,
        capability_id: &str,
        template_id: &str,
    ) -> Result<TemplateHandle, HostError>;

    /// Bind `state` to `handle`, wrap with `chrome`, return Splash DSL
    /// string suitable for `content.splash_card.set_text(...)`.
    fn render_to_splash(
        &self,
        handle: &TemplateHandle,
        state: &JsonValue,
        chrome: &AttributionChrome,
    ) -> Result<String, HostError>;

    /// Apply a path-scoped update to `state`. Returns `Ok(true)` if the
    /// state mutated (re-render required), `Ok(false)` if the update was
    /// a no-op (e.g. removing an absent key), or an error for unsupported
    /// ops. v1 accepts only `StateUpdateOp::Replace` and `StateUpdateOp::Remove`.
    fn apply_state_update(
        &self,
        handle: &TemplateHandle,
        state: &mut JsonValue,
        path: &str,
        op: StateUpdateOp<'_>,
    ) -> Result<bool, HostError>;

    /// Dispatch an action emitted by a rendered template. v1 classifies
    /// remote capability actions and W7 local-function actions without
    /// touching Matrix transport or platform side-effect wiring.
    fn route_action(
        &self,
        capability_id: &str,
        action_id: &str,
        payload: &JsonValue,
    ) -> Result<ActionOutcome, HostError>;
}

pub(crate) trait CapabilitySchema: Send + Sync {
    fn app_type(&self) -> &'static str;
    fn app_version(&self) -> u32;
    fn contains_path(&self, path: &str) -> bool;
}

#[derive(Debug, Default)]
struct SplashAst {
    widget_refs: Vec<String>,
    binding_paths: Vec<String>,
    local_function_calls: Vec<String>,
    attribution_fields: Vec<String>,
}

/// Default v1 host implementation. Uses the static `WidgetManifest` and
/// `LocalFunctionRegistry` from sibling modules (plan Steps 1.2 + 1.3).
#[derive(Debug, Default)]
pub struct DefaultSplashHost {
    template_cache: OnceLock<RwLock<template_cache::TemplateCache>>,
    parse_invocations: AtomicUsize,
}

impl DefaultSplashHost {
    pub const fn new() -> Self {
        Self {
            template_cache: OnceLock::new(),
            parse_invocations: AtomicUsize::new(0),
        }
    }

    fn validate_template_source(
        &self,
        capability_id: &str,
        template_id: &str,
        source: &str,
    ) -> Result<TemplateHandle, HostError> {
        self.parse_invocations.fetch_add(1, Ordering::Relaxed);
        let ast = parse_to_ast(source)?;
        validate_widget_refs(&ast)?;
        validate_local_function_calls(&ast)?;
        validate_attribution_fields(&ast)?;
        validate_binding_paths(capability_id, &ast)?;

        Ok(TemplateHandle {
            capability_id: capability_id.to_string(),
            template_id: template_id.to_string(),
            source: source.to_string(),
        })
    }

    fn load_template_source(
        &self,
        capability_id: &str,
        template_id: &str,
    ) -> Result<&'static str, HostError> {
        if template_id.starts_with("generated:") {
            return Err(HostError::GeneratedTemplateNotYetSupported);
        }

        // Single-source-of-truth lookup per P1a: resolve through
        // `templates::source_for` rather than carrying a parallel
        // `include_str!` table here. Editing a template path in one
        // place now stays in sync everywhere.
        super::templates::source_for(capability_id, template_id).ok_or_else(|| {
            HostError::TemplateNotFound {
                capability_id: capability_id.to_string(),
                template_id: template_id.to_string(),
            }
        })
    }

    fn template_cache(&self) -> &RwLock<template_cache::TemplateCache> {
        self.template_cache
            .get_or_init(|| RwLock::new(template_cache::TemplateCache::default()))
    }

    #[cfg(test)]
    pub(crate) fn validate_source_for_test(
        &self,
        capability_id: &str,
        template_id: &str,
        source: &str,
    ) -> Result<TemplateHandle, HostError> {
        self.validate_template_source(capability_id, template_id, source)
    }

    /// **P2 test-only helper** — feeds an arbitrary template source
    /// through the same preflight + cache flow as production
    /// `load_template`, but bypasses the `templates::source_for` lookup
    /// so tests can inject unsafe templates the host MUST reject.
    ///
    /// Production code **cannot** call this (gated behind `#[cfg(test)]`).
    /// Release builds of robrix that accidentally reference this
    /// method fail to compile — the same guard that protects
    /// `bind_guidance_template` / `bind_news_template`.
    #[cfg(test)]
    pub(crate) fn load_template_from_source(
        &self,
        capability_id: &str,
        template_id: &str,
        source: &str,
    ) -> Result<TemplateHandle, HostError> {
        let cache_key = cache_key_for(capability_id, template_id, source);

        if let Ok(cache) = self.template_cache().read() {
            if let Some(handle) = cache.get(&cache_key) {
                return Ok(handle.as_ref().clone());
            }
        }

        let handle = self.validate_template_source(capability_id, template_id, source)?;

        if let Ok(cache) = self.template_cache().write() {
            cache.insert(cache_key, Arc::new(handle.clone()));
        }

        Ok(handle)
    }

    #[cfg(test)]
    fn reset_parse_invocations_for_test(&self) {
        self.parse_invocations.store(0, Ordering::Relaxed);
    }

    #[cfg(test)]
    fn parse_invocations_for_test(&self) -> usize {
        self.parse_invocations.load(Ordering::Relaxed)
    }
}

impl SplashHost for DefaultSplashHost {
    fn load_template(
        &self,
        capability_id: &str,
        template_id: &str,
    ) -> Result<TemplateHandle, HostError> {
        let source = self.load_template_source(capability_id, template_id)?;
        let cache_key = cache_key_for(capability_id, template_id, source);

        if let Ok(cache) = self.template_cache().read() {
            if let Some(handle) = cache.get(&cache_key) {
                return Ok(handle.as_ref().clone());
            }
        }

        let handle = self.validate_template_source(capability_id, template_id, source)?;

        if let Ok(cache) = self.template_cache().write() {
            cache.insert(cache_key, Arc::new(handle.clone()));
        }

        Ok(handle)
    }

    fn render_to_splash(
        &self,
        handle: &TemplateHandle,
        state: &JsonValue,
        chrome: &AttributionChrome,
    ) -> Result<String, HostError> {
        let ast = parse_to_ast(&handle.source)?;
        let mut rendered = render_local_function_interpolations(&handle.source, state)?;
        let mut paths = ast.binding_paths;
        paths.sort_by_key(|path| std::cmp::Reverse(path.len()));
        paths.dedup();

        for path in paths {
            let value = resolve_state_binding(state, &path).ok_or_else(|| HostError::BindingError {
                path: path.clone(),
                message: "binding path missing from render state".into(),
            })?;
            rendered = rendered.replace(&path, &value);
        }

        Ok(wrap_with_chrome(&rendered, chrome))
    }

    fn apply_state_update(
        &self,
        handle: &TemplateHandle,
        state: &mut JsonValue,
        path: &str,
        op: StateUpdateOp<'_>,
    ) -> Result<bool, HostError> {
        let _ = handle;
        match op {
            StateUpdateOp::Replace { value } => replace_json_pointer(state, path, value),
            StateUpdateOp::Remove => remove_json_pointer(state, path),
            StateUpdateOp::Append { value } => {
                let _ = value;
                Err(HostError::UpdateOpNotYetSupported { op: "append".into() })
            }
        }
    }

    fn route_action(
        &self,
        capability_id: &str,
        action_id: &str,
        payload: &JsonValue,
    ) -> Result<ActionOutcome, HostError> {
        if local_functions::is_registered(action_id) {
            return Ok(ActionOutcome::LocalFunctionCalled {
                name: action_id.to_string(),
            });
        }

        let capability_known = capability_descriptors::iter()
            .any(|(_, descriptor)| descriptor.capability_id == capability_id);

        if capability_known {
            Ok(ActionOutcome::RemoteDispatch {
                capability_id: capability_id.to_string(),
                action_id: action_id.to_string(),
                payload: payload.clone(),
            })
        } else {
            Ok(ActionOutcome::UnknownAction {
                action_id: action_id.to_string(),
            })
        }
    }
}

/// Process-wide singleton accessor (plan Step 1.8). `RenderedApp::render`
/// impls in `weather.rs` / `news.rs` use this to reach the host without
/// taking the trait object as a constructor argument.
pub fn splash_host() -> &'static dyn SplashHost {
    static HOST: OnceLock<DefaultSplashHost> = OnceLock::new();
    HOST.get_or_init(DefaultSplashHost::new)
}

fn parse_to_ast(source: &str) -> Result<SplashAst, HostError> {
    validate_balanced_splash(source)?;
    Ok(SplashAst {
        widget_refs: collect_widget_refs(source),
        binding_paths: collect_binding_paths(source),
        local_function_calls: collect_local_function_calls(source),
        attribution_fields: collect_attribution_fields(source),
    })
}

fn validate_balanced_splash(source: &str) -> Result<(), HostError> {
    let mut brace_depth = 0usize;
    let mut line = 1usize;
    let mut in_string = false;
    let mut escaped = false;

    for ch in source.chars() {
        if ch == '\n' {
            line += 1;
        }

        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => brace_depth += 1,
            '}' => {
                if brace_depth == 0 {
                    return Err(HostError::ParseError {
                        message: "unexpected closing brace".into(),
                        line,
                    });
                }
                brace_depth -= 1;
            }
            _ => {}
        }
    }

    if in_string {
        return Err(HostError::ParseError {
            message: "unterminated string literal".into(),
            line,
        });
    }

    if brace_depth != 0 {
        return Err(HostError::ParseError {
            message: "unbalanced braces".into(),
            line,
        });
    }

    Ok(())
}

fn collect_widget_refs(source: &str) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut refs = Vec::new();
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];
        if !ch.is_ascii_uppercase() {
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        while index < chars.len() && (chars[index].is_ascii_alphanumeric() || chars[index] == '_') {
            index += 1;
        }

        let ident: String = chars[start..index].iter().collect();
        let mut next = index;
        while next < chars.len() && chars[next].is_whitespace() {
            next += 1;
        }
        if next >= chars.len() || chars[next] != '{' {
            continue;
        }

        let mut prev = start;
        while prev > 0 && chars[prev - 1].is_whitespace() {
            prev -= 1;
        }
        if prev > 0 && chars[prev - 1] == ':' {
            continue;
        }

        refs.push(ident);
    }

    refs
}

fn collect_binding_paths(source: &str) -> Vec<String> {
    collect_prefixed_tokens(source, "$state.")
}

fn collect_prefixed_tokens(source: &str, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = source;
    while let Some(found) = rest.find(prefix) {
        let candidate = &rest[found..];
        let end = candidate
            .char_indices()
            .take_while(|(idx, ch)| {
                if *idx < prefix.len() {
                    return true;
                }
                ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '.'
            })
            .last()
            .map(|(idx, ch)| idx + ch.len_utf8())
            .unwrap_or(prefix.len());
        out.push(candidate[..end].to_string());
        rest = &candidate[end..];
    }
    out
}

fn collect_local_function_calls(source: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut rest = source;

    while let Some(found) = rest.find("${") {
        let candidate = &rest[found + 2..];
        let name: String = candidate
            .chars()
            .skip_while(|ch| ch.is_whitespace())
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect();
        if !name.is_empty() {
            calls.push(name);
        }
        rest = candidate;
    }

    calls.extend(collect_function_call_names(source));
    calls
}

fn collect_function_call_names(source: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut rest = source;

    while let Some(found) = rest.find("call") {
        let candidate = &rest[found + "call".len()..];
        let candidate = candidate.trim_start();
        let Some(candidate) = candidate.strip_prefix(':') else {
            rest = &rest[found + "call".len()..];
            continue;
        };
        let candidate = candidate.trim_start();
        let Some(candidate) = candidate.strip_prefix('"') else {
            rest = candidate;
            continue;
        };
        let Some(end) = candidate.find('"') else {
            break;
        };
        let name = &candidate[..end];
        if !name.is_empty() {
            calls.push(name.to_string());
        }
        rest = &candidate[end + 1..];
    }

    calls
}

fn collect_attribution_fields(source: &str) -> Vec<String> {
    ["capability_id", "display_name", "icon", "trust_badge"]
        .into_iter()
        .filter(|field| contains_identifier(source, field))
        .map(str::to_string)
        .collect()
}

fn contains_identifier(source: &str, ident: &str) -> bool {
    let bytes = source.as_bytes();
    let ident_bytes = ident.as_bytes();

    bytes.windows(ident_bytes.len()).enumerate().any(|(idx, window)| {
        if window != ident_bytes {
            return false;
        }
        let prev = idx.checked_sub(1).and_then(|i| bytes.get(i)).copied();
        let next = bytes.get(idx + ident_bytes.len()).copied();
        !matches!(prev, Some(b) if b.is_ascii_alphanumeric() || b == b'_')
            && !matches!(next, Some(b) if b.is_ascii_alphanumeric() || b == b'_')
    })
}

fn validate_widget_refs(ast: &SplashAst) -> Result<(), HostError> {
    for name in &ast.widget_refs {
        if widget_manifest::is_template_reachable(name) {
            continue;
        }
        return Err(HostError::WidgetNotAllowed {
            name: name.clone(),
            trust_level: widget_manifest::lookup(name)
                .map(|descriptor| format!("{:?}", descriptor.trust_level)),
        });
    }
    Ok(())
}

fn validate_local_function_calls(ast: &SplashAst) -> Result<(), HostError> {
    for name in &ast.local_function_calls {
        if !local_functions::is_registered(name) {
            return Err(HostError::LocalFunctionNotAllowed { name: name.clone() });
        }
    }
    Ok(())
}

fn validate_attribution_fields(ast: &SplashAst) -> Result<(), HostError> {
    if let Some(field) = ast.attribution_fields.first() {
        return Err(HostError::AttributionFieldInTemplate {
            field: field.clone(),
        });
    }
    Ok(())
}

fn validate_binding_paths(capability_id: &str, ast: &SplashAst) -> Result<(), HostError> {
    let Some(schema) = schema_for_capability(capability_id) else {
        return Ok(());
    };
    for path in &ast.binding_paths {
        if !schema.contains_path(path) {
            return Err(HostError::BindingPathNotInSchema {
                path: path.clone(),
                app_type: schema.app_type().to_string(),
                app_version: schema.app_version(),
            });
        }
    }
    Ok(())
}

fn resolve_state_binding(state: &JsonValue, path: &str) -> Option<String> {
    if let Some(value) = state.get(path) {
        return Some(value_to_splash(value));
    }

    let relative = path.strip_prefix("$state.")?;
    let mut current = state;
    for segment in relative.split('.') {
        current = current.get(segment)?;
    }
    Some(value_to_splash(current))
}

fn value_to_splash(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => splash_escape(s),
        JsonValue::Bool(v) => v.to_string(),
        JsonValue::Number(v) => v.to_string(),
        JsonValue::Null => String::new(),
        other => splash_escape(&other.to_string()),
    }
}

fn wrap_with_chrome(content: &str, chrome: &AttributionChrome) -> String {
    let display_name = splash_escape(&chrome.display_name);
    let capability_id = splash_escape(&chrome.capability_id);
    format!(
        "View {{ width: Fill height: Fit flow: Down \
Label {{ visible: false text: \"{display_name}\" }} \
Label {{ visible: false text: \"{capability_id}\" }} \
{content} }}"
    )
}

fn render_local_function_interpolations(
    source: &str,
    state: &JsonValue,
) -> Result<String, HostError> {
    let mut rendered = String::with_capacity(source.len());
    let mut rest = source;

    while let Some(start) = rest.find("${") {
        rendered.push_str(&rest[..start]);
        let expression_start = start + 2;
        let expression_rest = &rest[expression_start..];
        let Some(end) = expression_rest.find('}') else {
            return Err(HostError::ParseError {
                message: "unterminated local function interpolation".into(),
                line: source[..source.len() - expression_rest.len()].lines().count().max(1),
            });
        };

        let expression = expression_rest[..end].trim();
        let value = evaluate_local_function_expression(expression, state)?;
        rendered.push_str(&value_to_splash(&value));
        rest = &expression_rest[end + 1..];
    }

    rendered.push_str(rest);
    Ok(rendered)
}

fn evaluate_local_function_expression(
    expression: &str,
    state: &JsonValue,
) -> Result<JsonValue, HostError> {
    let Some(open_paren) = expression.find('(') else {
        return Err(HostError::ParseError {
            message: format!("local function expression `{expression}` is missing `(`"),
            line: 1,
        });
    };
    let Some(close_paren) = expression.rfind(')') else {
        return Err(HostError::ParseError {
            message: format!("local function expression `{expression}` is missing `)`"),
            line: 1,
        });
    };

    let name = expression[..open_paren].trim();
    if !local_functions::is_registered(name) {
        return Err(HostError::LocalFunctionNotAllowed {
            name: name.to_string(),
        });
    }

    let args_source = &expression[open_paren + 1..close_paren];
    let args = parse_local_function_args(args_source, state)?;
    match local_functions::invoke(name, &JsonValue::Array(args)) {
        Some(Ok(value)) => Ok(value),
        Some(Err(err)) => Err(HostError::BindingError {
            path: expression.to_string(),
            message: err.to_string(),
        }),
        None => Err(HostError::LocalFunctionNotAllowed {
            name: name.to_string(),
        }),
    }
}

fn parse_local_function_args(
    args_source: &str,
    state: &JsonValue,
) -> Result<Vec<JsonValue>, HostError> {
    let mut args = Vec::new();
    for arg in split_function_args(args_source)? {
        let arg = arg.trim();
        if arg.is_empty() {
            continue;
        }
        args.push(parse_local_function_arg(arg, state)?);
    }
    Ok(args)
}

fn split_function_args(args_source: &str) -> Result<Vec<String>, HostError> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in args_source.chars() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                current.push(ch);
            }
            ',' => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if in_string {
        return Err(HostError::ParseError {
            message: "unterminated string literal in local function args".into(),
            line: 1,
        });
    }

    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }

    Ok(args)
}

fn parse_local_function_arg(arg: &str, state: &JsonValue) -> Result<JsonValue, HostError> {
    if arg.starts_with("$state.") {
        return resolve_state_json_binding(state, arg)
            .cloned()
            .ok_or_else(|| HostError::BindingError {
                path: arg.to_string(),
                message: "binding path missing from render state".into(),
            });
    }

    if arg.starts_with('"') && arg.ends_with('"') {
        return serde_json::from_str::<JsonValue>(arg).map_err(|err| HostError::ParseError {
            message: format!("invalid string literal in local function arg: {err}"),
            line: 1,
        });
    }

    if arg == "true" {
        return Ok(JsonValue::Bool(true));
    }
    if arg == "false" {
        return Ok(JsonValue::Bool(false));
    }
    if arg == "null" {
        return Ok(JsonValue::Null);
    }

    serde_json::from_str::<JsonValue>(arg).map_err(|err| HostError::ParseError {
        message: format!("invalid local function arg `{arg}`: {err}"),
        line: 1,
    })
}

fn resolve_state_json_binding<'a>(state: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    if let Some(value) = state.get(path) {
        return Some(value);
    }

    let relative = path.strip_prefix("$state.")?;
    let mut current = state;
    for segment in relative.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn replace_json_pointer(
    state: &mut JsonValue,
    path: &str,
    value: &JsonValue,
) -> Result<bool, HostError> {
    let mut segments = split_json_pointer(path)?;
    if segments.is_empty() {
        let changed = state != value;
        *state = value.clone();
        return Ok(changed);
    }

    let last = segments.pop().expect("non-empty path has last segment");
    let parent = json_pointer_parent_mut(state, &segments, path)?;

    match parent {
        JsonValue::Object(map) => {
            let changed = map.get(&last) != Some(value);
            map.insert(last, value.clone());
            Ok(changed)
        }
        JsonValue::Array(items) => {
            if last == "-" {
                return Err(HostError::UpdateOpNotYetSupported { op: "append".into() });
            }
            let index = parse_json_pointer_index(&last, path)?;
            let Some(slot) = items.get_mut(index) else {
                return Err(HostError::BindingError {
                    path: path.to_string(),
                    message: "array index out of bounds".into(),
                });
            };
            let changed = slot != value;
            *slot = value.clone();
            Ok(changed)
        }
        _ => Err(HostError::BindingError {
            path: path.to_string(),
            message: "parent value is not an object or array".into(),
        }),
    }
}

fn remove_json_pointer(state: &mut JsonValue, path: &str) -> Result<bool, HostError> {
    let mut segments = split_json_pointer(path)?;
    if segments.is_empty() {
        let changed = !state.is_null();
        *state = JsonValue::Null;
        return Ok(changed);
    }

    let last = segments.pop().expect("non-empty path has last segment");
    let parent = json_pointer_parent_mut(state, &segments, path)?;

    match parent {
        JsonValue::Object(map) => Ok(map.remove(&last).is_some()),
        JsonValue::Array(items) => {
            if last == "-" {
                return Ok(false);
            }
            let index = parse_json_pointer_index(&last, path)?;
            if index >= items.len() {
                return Ok(false);
            }
            items.remove(index);
            Ok(true)
        }
        _ => Err(HostError::BindingError {
            path: path.to_string(),
            message: "parent value is not an object or array".into(),
        }),
    }
}

fn json_pointer_parent_mut<'a>(
    mut current: &'a mut JsonValue,
    segments: &[String],
    full_path: &str,
) -> Result<&'a mut JsonValue, HostError> {
    for segment in segments {
        match current {
            JsonValue::Object(map) => {
                current = map.get_mut(segment).ok_or_else(|| HostError::BindingError {
                    path: full_path.to_string(),
                    message: format!("missing object key `{segment}`"),
                })?;
            }
            JsonValue::Array(items) => {
                let index = parse_json_pointer_index(segment, full_path)?;
                current = items.get_mut(index).ok_or_else(|| HostError::BindingError {
                    path: full_path.to_string(),
                    message: format!("array index {index} out of bounds"),
                })?;
            }
            _ => {
                return Err(HostError::BindingError {
                    path: full_path.to_string(),
                    message: format!("segment `{segment}` cannot be resolved through scalar value"),
                });
            }
        }
    }
    Ok(current)
}

fn split_json_pointer(path: &str) -> Result<Vec<String>, HostError> {
    if path.is_empty() {
        return Ok(Vec::new());
    }
    if !path.starts_with('/') {
        return Err(HostError::BindingError {
            path: path.to_string(),
            message: "state update path must be a JSON Pointer starting with `/`".into(),
        });
    }

    path[1..]
        .split('/')
        .map(|segment| decode_json_pointer_segment(segment, path))
        .collect()
}

fn decode_json_pointer_segment(segment: &str, full_path: &str) -> Result<String, HostError> {
    let mut decoded = String::with_capacity(segment.len());
    let mut chars = segment.chars();

    while let Some(ch) = chars.next() {
        if ch != '~' {
            decoded.push(ch);
            continue;
        }

        match chars.next() {
            Some('0') => decoded.push('~'),
            Some('1') => decoded.push('/'),
            Some(other) => {
                return Err(HostError::BindingError {
                    path: full_path.to_string(),
                    message: format!("invalid JSON Pointer escape `~{other}`"),
                });
            }
            None => {
                return Err(HostError::BindingError {
                    path: full_path.to_string(),
                    message: "unterminated JSON Pointer escape".into(),
                });
            }
        }
    }

    Ok(decoded)
}

fn parse_json_pointer_index(segment: &str, full_path: &str) -> Result<usize, HostError> {
    segment.parse::<usize>().map_err(|_| HostError::BindingError {
        path: full_path.to_string(),
        message: format!("array segment `{segment}` is not a valid index"),
    })
}

fn splash_escape(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn schema_for_capability(capability_id: &str) -> Option<&'static dyn CapabilitySchema> {
    match capability_id {
        "mission_dashboard" => Some(&mission_dashboard::MISSION_DASHBOARD_CAPABILITY_SCHEMA),
        "mission_room" => Some(&mission_room::MISSION_ROOM_CAPABILITY_SCHEMA),
        "news_guidance" => Some(&news::NEWS_CAPABILITY_SCHEMA),
        "weather_guidance" => Some(&weather::WEATHER_CAPABILITY_SCHEMA),
        _ => None,
    }
}

fn cache_key_for(capability_id: &str, template_id: &str, source: &str) -> template_cache::CacheKey {
    let (app_type, app_version) = schema_for_capability(capability_id)
        .map(|schema| (schema.app_type().to_string(), schema.app_version()))
        .unwrap_or_else(|| (capability_id.to_string(), 1));
    let manifest_version = capability_descriptors::lookup(&app_type)
        .map(|descriptor| descriptor.manifest_version)
        .unwrap_or(1);

    template_cache::CacheKey {
        app_type,
        app_version,
        template_id: template_id.to_string(),
        template_hash: template_cache::template_hash(source),
        manifest_version,
        host_version: capability_descriptors::HOST_VERSION,
    }
}

#[allow(dead_code)]
pub(crate) fn render_with_fallback<F>(
    capability_id: &str,
    preferred_template_id: &str,
    preferred_result: Result<String, HostError>,
    fallback_template_id: Option<&str>,
    mut render_fallback: F,
) -> Result<String, FallbackReason>
where
    F: FnMut(&str) -> Result<String, HostError>,
{
    match preferred_result {
        Ok(rendered) => Ok(rendered),
        Err(preferred_error) => {
            makepad_widgets::log!(
                "template fallback level=template capability_id={} preferred_template_id={} reason={}",
                capability_id,
                preferred_template_id,
                preferred_error,
            );

            if let Some(fallback_template_id) = fallback_template_id {
                match render_fallback(fallback_template_id) {
                    Ok(rendered) => return Ok(rendered),
                    Err(final_error) => {
                        makepad_widgets::log!(
                            "template fallback level=plain_text capability_id={} preferred_template_id={} fallback_template_id={} reason={}",
                            capability_id,
                            preferred_template_id,
                            fallback_template_id,
                            final_error,
                        );
                        return Err(FallbackReason::AllTemplatesFailed {
                            final_error: Box::new(final_error),
                        });
                    }
                }
            }

            Err(FallbackReason::AllTemplatesFailed {
                final_error: Box::new(preferred_error),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribution_chrome_holds_all_four_fields() {
        let chrome = AttributionChrome {
            capability_id: "weather_guidance".into(),
            display_name: "Weather".into(),
            icon_url: Some("file:///weather.svg".into()),
            trust_badge: TrustBadge::Builtin,
        };
        assert_eq!(chrome.capability_id, "weather_guidance");
        assert_eq!(chrome.display_name, "Weather");
        assert_eq!(chrome.icon_url.as_deref(), Some("file:///weather.svg"));
        assert_eq!(chrome.trust_badge, TrustBadge::Builtin);
    }

    #[test]
    fn template_handle_exposes_pair_identity() {
        let handle = TemplateHandle::new_for_test("weather_guidance", "card_standard", "");
        assert_eq!(handle.capability_id(), "weather_guidance");
        assert_eq!(handle.template_id(), "card_standard");
    }

    #[test]
    fn host_error_display_is_human_readable() {
        let err = HostError::WidgetNotAllowed {
            name: "EvilWidget".into(),
            trust_level: Some("Sensitive".into()),
        };
        let rendered = format!("{err}");
        assert!(rendered.contains("EvilWidget"));
        assert!(rendered.contains("Sensitive"));
    }

    #[test]
    fn host_error_variants_cover_spec_scenarios() {
        // Presence check — every spec scenario in the Completion Criteria
        // table binds to one of these variants. If a scenario is added
        // that does not map here, this module needs a new variant before
        // the binding step can land.
        let _ = HostError::TemplateNotFound { capability_id: "".into(), template_id: "".into() };
        let _ = HostError::ParseError { message: "".into(), line: 1 };
        let _ = HostError::WidgetNotAllowed { name: "".into(), trust_level: None };
        let _ = HostError::LocalFunctionNotAllowed { name: "".into() };
        let _ = HostError::AttributionFieldInTemplate { field: "".into() };
        let _ = HostError::BindingError { path: "".into(), message: "".into() };
        let _ = HostError::BindingPathNotInSchema {
            path: "".into(),
            app_type: "".into(),
            app_version: 1,
        };
        let _ = HostError::UpdateOpNotYetSupported { op: "".into() };
        let _ = HostError::GeneratedTemplateNotYetSupported;
    }

    #[test]
    fn default_host_new_is_const() {
        // Locks the `const fn new` contract so `static HOST: DefaultSplashHost`
        // patterns remain viable (used by `splash_host()`).
        const _HOST: DefaultSplashHost = DefaultSplashHost::new();
    }

    #[test]
    fn splash_host_accessor_returns_same_instance() {
        let a = splash_host() as *const _;
        let b = splash_host() as *const _;
        assert_eq!(a, b, "splash_host() must be a process-wide singleton");
    }

    #[test]
    fn preflight_rejects_parse_error() {
        let err = DefaultSplashHost::new()
            .validate_source_for_test(
                "weather_guidance",
                "broken",
                "RoundedView { Label { text: \"unterminated\" ",
            )
            .expect_err("malformed splash must fail preflight");

        match err {
            HostError::ParseError { .. } => {}
            other => panic!("expected ParseError, got {other:?}"),
        }
    }

    #[test]
    fn preflight_rejects_unlisted_widget() {
        let err = DefaultSplashHost::new()
            .validate_source_for_test(
                "weather_guidance",
                "bad_widget",
                "RoundedView { EvilWidget { text: \"boom\" } }",
            )
            .expect_err("unknown widget must fail preflight");

        assert_eq!(
            err,
            HostError::WidgetNotAllowed {
                name: "EvilWidget".into(),
                trust_level: None,
            }
        );
    }

    #[test]
    fn preflight_rejects_unlisted_local_function() {
        let err = DefaultSplashHost::new()
            .validate_source_for_test(
                "weather_guidance",
                "bad_fn",
                "RoundedView { Label { text: \"${exec_shell($state.location)}\" } }",
            )
            .expect_err("unknown local function must fail preflight");

        assert_eq!(
            err,
            HostError::LocalFunctionNotAllowed {
                name: "exec_shell".into(),
            }
        );
    }

    #[test]
    fn preflight_rejects_unlisted_function_call_action() {
        let err = DefaultSplashHost::new()
            .validate_source_for_test(
                "weather_guidance",
                "bad_function_call",
                r#"RoundedView { Button { text: "Run" action: { functionCall: { call: "exec_shell" args: {} } } } }"#,
            )
            .expect_err("unknown functionCall.call must fail preflight");

        assert_eq!(
            err,
            HostError::LocalFunctionNotAllowed {
                name: "exec_shell".into(),
            }
        );
    }

    #[test]
    fn preflight_accepts_registered_function_call_action() {
        let handle = DefaultSplashHost::new()
            .validate_source_for_test(
                "weather_guidance",
                "good_function_call",
                r#"RoundedView { Button { text: "Open" action: { functionCall: { call: "open_url" args: {} } } } }"#,
            )
            .expect("registered functionCall.call should pass W7 preflight");

        assert_eq!(handle.template_id(), "good_function_call");
    }

    #[test]
    fn generated_template_slot_is_rejected_before_static_lookup() {
        let err = DefaultSplashHost::new()
            .load_template("weather_guidance", "generated:adaptive")
            .expect_err("generated template slots are disabled in v1");

        assert_eq!(err, HostError::GeneratedTemplateNotYetSupported);
    }

    #[test]
    fn preflight_rejects_attribution_override() {
        let err = DefaultSplashHost::new()
            .validate_source_for_test(
                "weather_guidance",
                "bad_attribution",
                "RoundedView { Label { text: \"$state.display_name\" } }",
            )
            .expect_err("host-owned chrome fields must be blocked");

        assert_eq!(
            err,
            HostError::AttributionFieldInTemplate {
                field: "display_name".into(),
            }
        );
    }

    #[test]
    fn preflight_rejects_binding_path_not_in_schema() {
        let err = DefaultSplashHost::new()
            .validate_source_for_test(
                "weather_guidance",
                "bad_path",
                "RoundedView { Label { text: \"$state.unknown_field\" } }",
            )
            .expect_err("unknown binding path must fail preflight");

        match err {
            HostError::BindingPathNotInSchema { path, app_type, app_version } => {
                assert_eq!(path, "$state.unknown_field");
                assert_eq!(app_type, "weather");
                assert_eq!(app_version, 2);
            }
            other => panic!("expected BindingPathNotInSchema, got {other:?}"),
        }
    }

    #[test]
    fn cache_hit_skips_parse() {
        let host = DefaultSplashHost::new();
        host.reset_parse_invocations_for_test();

        let _first = host
            .load_template("weather_guidance", "card_standard")
            .expect("first load should succeed");
        let _second = host
            .load_template("weather_guidance", "card_standard")
            .expect("second load should reuse cached handle");

        assert_eq!(host.parse_invocations_for_test(), 1);
    }

    #[test]
    fn host_error_to_validation_error_maps_stable_codes() {
        let err = HostError::BindingPathNotInSchema {
            path: "$state.unknown_field".into(),
            app_type: "weather".into(),
            app_version: 2,
        };

        let validation = err.to_validation_error();
        assert_eq!(validation.code, "BINDING_PATH_NOT_IN_SCHEMA");
        assert_eq!(validation.path, "$state.unknown_field");
        assert!(validation.message.contains("binding path not in schema"));
    }

    #[test]
    fn fallback_template_id_succeeds() {
        let rendered = render_with_fallback(
            "weather_guidance",
            "preferred",
            Err(HostError::ParseError {
                message: "broken".into(),
                line: 1,
            }),
            Some("fallback_template"),
            |template_id| {
                assert_eq!(template_id, "fallback_template");
                Ok("RoundedView {}".into())
            },
        )
        .expect("fallback template should recover");

        assert_eq!(rendered, "RoundedView {}");
    }

    #[test]
    fn fallback_plain_text() {
        let err = render_with_fallback(
            "weather_guidance",
            "preferred",
            Err(HostError::ParseError {
                message: "broken".into(),
                line: 1,
            }),
            Some("fallback_template"),
            |_template_id| {
                Err(HostError::WidgetNotAllowed {
                    name: "Broken".into(),
                    trust_level: None,
                })
            },
        )
        .expect_err("all templates failing should bubble fallback reason");

        match err {
            FallbackReason::AllTemplatesFailed { final_error } => {
                assert!(matches!(*final_error, HostError::WidgetNotAllowed { .. }));
            }
            other => panic!("expected AllTemplatesFailed, got {other:?}"),
        }
    }

    #[test]
    fn fallback_does_not_oscillate() {
        let mut attempts = 0usize;
        let rendered = render_with_fallback(
            "weather_guidance",
            "preferred",
            Err(HostError::ParseError {
                message: "broken".into(),
                line: 1,
            }),
            Some("fallback_template"),
            |_template_id| {
                attempts += 1;
                Ok("RoundedView {}".into())
            },
        )
        .expect("fallback should succeed once");

        assert_eq!(rendered, "RoundedView {}");
        assert_eq!(attempts, 1, "fallback path should run at most once");
    }

    #[test]
    fn render_to_splash_binds_state_tokens() {
        let host = DefaultSplashHost::new();
        let handle = host
            .validate_source_for_test(
                "weather_guidance",
                "custom",
                "RoundedView { Label { text: \"$state.location\" visible: $state.range.visible } }",
            )
            .expect("template should pass preflight");
        let chrome = AttributionChrome {
            capability_id: "weather_guidance".into(),
            display_name: "Weather".into(),
            icon_url: None,
            trust_badge: TrustBadge::Builtin,
        };
        let state = serde_json::json!({
            "$state.location": "Beijing",
            "$state.range.visible": true
        });

        let rendered = host
            .render_to_splash(&handle, &state, &chrome)
            .expect("state binding should render");

        assert!(rendered.contains("text: \"Beijing\""));
        assert!(rendered.contains("visible: true"));
        assert!(!rendered.contains("$state."));
    }

    #[test]
    fn render_to_splash_invokes_local_function_interpolation() {
        let host = DefaultSplashHost::new();
        let handle = host
            .validate_source_for_test(
                "test_capability",
                "custom",
                "RoundedView { Label { text: \"${format_number($state.temp, 1)}°\" } }",
            )
            .expect("template should pass preflight");
        let chrome = AttributionChrome {
            capability_id: "test_capability".into(),
            display_name: "Test".into(),
            icon_url: None,
            trust_badge: TrustBadge::Builtin,
        };
        let state = serde_json::json!({
            "$state.temp": 23.456
        });

        let rendered = host
            .render_to_splash(&handle, &state, &chrome)
            .expect("function interpolation should render");

        assert!(rendered.contains("text: \"23.5°\""));
        assert!(!rendered.contains("${"));
        assert!(!rendered.contains("$state.temp"));
    }

    #[test]
    fn render_to_splash_wraps_host_owned_chrome() {
        let host = DefaultSplashHost::new();
        let handle = host
            .validate_source_for_test(
                "test_capability",
                "custom",
                "RoundedView { Label { text: \"$state.location\" } }",
            )
            .expect("template should pass preflight");
        let chrome = AttributionChrome {
            capability_id: "weather_guidance".into(),
            display_name: "Weather".into(),
            icon_url: None,
            trust_badge: TrustBadge::Builtin,
        };
        let state = serde_json::json!({
            "$state.location": "Beijing"
        });

        let rendered = host
            .render_to_splash(&handle, &state, &chrome)
            .expect("state binding should render");

        assert!(rendered.starts_with("View {"), "host should wrap content in chrome container: {rendered}");
        assert!(rendered.contains("text: \"Weather\""), "host-owned display name missing: {rendered}");
        assert!(rendered.contains("text: \"weather_guidance\""), "host-owned capability id missing: {rendered}");
        assert!(rendered.contains("text: \"Beijing\""), "template content should remain present: {rendered}");
    }

    #[test]
    fn apply_state_update_replace_mutates_json_pointer_path() {
        let host = DefaultSplashHost::new();
        let handle = TemplateHandle::new_for_test("weather_guidance", "custom", "");
        let mut state = serde_json::json!({ "user": { "name": "Alice" } });
        let value = serde_json::json!("Bob");

        let changed = host
            .apply_state_update(&handle, &mut state, "/user/name", StateUpdateOp::Replace { value: &value })
            .expect("replace should succeed");

        assert!(changed);
        assert_eq!(state, serde_json::json!({ "user": { "name": "Bob" } }));
    }

    #[test]
    fn apply_state_update_remove_existing_key_mutates() {
        let host = DefaultSplashHost::new();
        let handle = TemplateHandle::new_for_test("weather_guidance", "custom", "");
        let mut state = serde_json::json!({ "user": { "name": "Alice", "city": "Paris" } });

        let changed = host
            .apply_state_update(&handle, &mut state, "/user/city", StateUpdateOp::Remove)
            .expect("remove should succeed");

        assert!(changed);
        assert_eq!(state, serde_json::json!({ "user": { "name": "Alice" } }));
    }

    #[test]
    fn apply_state_update_remove_absent_key_is_noop() {
        let host = DefaultSplashHost::new();
        let handle = TemplateHandle::new_for_test("weather_guidance", "custom", "");
        let mut state = serde_json::json!({ "user": { "name": "Alice" } });
        let before = state.clone();

        let changed = host
            .apply_state_update(&handle, &mut state, "/user/city", StateUpdateOp::Remove)
            .expect("remove of absent key should be a no-op");

        assert!(!changed);
        assert_eq!(state, before);
    }

    #[test]
    fn apply_state_update_append_is_rejected_without_mutating() {
        let host = DefaultSplashHost::new();
        let handle = TemplateHandle::new_for_test("weather_guidance", "custom", "");
        let mut state = serde_json::json!({ "items": [] });
        let before = state.clone();
        let value = serde_json::json!("new");

        let err = host
            .apply_state_update(&handle, &mut state, "/items", StateUpdateOp::Append { value: &value })
            .expect_err("append is reserved for L2 actions");

        assert_eq!(err, HostError::UpdateOpNotYetSupported { op: "append".into() });
        assert_eq!(state, before);
    }

    #[test]
    fn route_action_registered_local_function_returns_local_outcome() {
        let host = DefaultSplashHost::new();
        let outcome = host
            .route_action("weather_guidance", "open_url", &serde_json::json!({ "url": "https://example.com" }))
            .expect("registered W7 function should route locally");

        match outcome {
            ActionOutcome::LocalFunctionCalled { name } => assert_eq!(name, "open_url"),
            other => panic!("expected local function outcome, got {other:?}"),
        }
    }

    #[test]
    fn route_action_known_capability_returns_remote_dispatch() {
        let host = DefaultSplashHost::new();
        let payload = serde_json::json!({ "unit": "celsius" });
        let outcome = host
            .route_action("weather_guidance", "refresh", &payload)
            .expect("known capability action should route remotely");

        match outcome {
            ActionOutcome::RemoteDispatch { capability_id, action_id, payload: routed_payload } => {
                assert_eq!(capability_id, "weather_guidance");
                assert_eq!(action_id, "refresh");
                assert_eq!(routed_payload, payload);
            }
            other => panic!("expected remote dispatch outcome, got {other:?}"),
        }
    }

    #[test]
    fn route_action_unknown_capability_returns_unknown_action() {
        let host = DefaultSplashHost::new();
        let outcome = host
            .route_action("unknown_capability", "refresh", &JsonValue::Null)
            .expect("unknown action should be reported, not panic");

        match outcome {
            ActionOutcome::UnknownAction { action_id } => assert_eq!(action_id, "refresh"),
            other => panic!("expected unknown action outcome, got {other:?}"),
        }
    }
}
