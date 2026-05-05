//! W7 Local Function Registry — closed set of named primitives that
//! templates may reference via `${fn(...)}` string interpolation or
//! `action: { functionCall: { call: "..." } }` action definitions.
//!
//! The v1 registry is a hard-coded closed set. Templates may NOT define
//! new functions (no `let` / `fn` in Splash DSL), and the host provides
//! no general expression evaluator — only `$state.path` binding plus
//! these named calls.
//!
//! Adding a new local function requires a Rust change + design review;
//! templates themselves cannot extend this registry. This is a key
//! guardrail from design doc §6 W7.
//!
//! Contract: see `specs/task-agent-to-app-splash-host-evolution.spec.md`
//! `§W7 Local Function Registry`.

use std::collections::HashMap;
use std::sync::LazyLock;

use chrono::{DateTime, FixedOffset};
use serde_json::Value as JsonValue;

/// Argument type kinds accepted by a registered function. v1 stays
/// narrow; expanded when a concrete function needs a new kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgKind {
    /// UTF-8 string (literal or `$state.path`-bound).
    String,
    /// JSON number (integer or float, caller coerces).
    Number,
    /// Boolean.
    Bool,
    /// Any JSON value (escape hatch; rare, used only when a function
    /// genuinely needs polymorphic input).
    Any,
}

/// Return-type kind of a registered function. Informs the preflight
/// validator which sites a function may appear at (e.g. a
/// `check: required(...)` must return `Bool`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnKind {
    String,
    Number,
    Bool,
    /// Void / side-effect only — e.g. `open_url` has no return value.
    Unit,
}

/// One argument on a registered function's signature.
#[derive(Debug, Clone, Copy)]
pub struct ArgSpec {
    pub name: &'static str,
    pub kind: ArgKind,
    /// `true` for required args. v1 has no variadic / optional args;
    /// this field is plumbed for forward compatibility with preflight.
    pub required: bool,
}

/// Registry entry: function signature metadata. V1 uses this table for
/// template preflight, render-time formatter/check execution, and action
/// classification. Platform side effects are wired by later L2 action work.
#[derive(Debug, Clone)]
pub struct LocalFunctionSpec {
    pub name: &'static str,
    pub args: &'static [ArgSpec],
    pub returns: ReturnKind,
    /// Human-readable purpose line (used in error messages and future
    /// template-author documentation). Templates never see this.
    pub doc: &'static str,
}

/// Static v1 registry. Closed set: the 5 primitives that `weather` and
/// `news` templates actually need. New entries require a code change
/// here plus an audit in the design doc.
static LOCAL_FUNCTIONS: LazyLock<HashMap<&'static str, LocalFunctionSpec>> =
    LazyLock::new(|| {
        let mut m: HashMap<&'static str, LocalFunctionSpec> = HashMap::new();

        m.insert(
            "open_url",
            LocalFunctionSpec {
                name: "open_url",
                args: &[ArgSpec { name: "url", kind: ArgKind::String, required: true }],
                returns: ReturnKind::Unit,
                doc: "Open a URL using the platform's default handler (reuses Robrix OpenLink behavior).",
            },
        );
        m.insert(
            "format_date",
            LocalFunctionSpec {
                name: "format_date",
                args: &[
                    ArgSpec { name: "value", kind: ArgKind::String, required: true },
                    ArgSpec { name: "pattern", kind: ArgKind::String, required: true },
                ],
                returns: ReturnKind::String,
                doc: "Format an ISO-8601 datetime string using the given chrono pattern.",
            },
        );
        m.insert(
            "format_number",
            LocalFunctionSpec {
                name: "format_number",
                args: &[
                    ArgSpec { name: "value", kind: ArgKind::Number, required: true },
                    ArgSpec { name: "precision", kind: ArgKind::Number, required: false },
                ],
                returns: ReturnKind::String,
                doc: "Format a number with optional decimal precision.",
            },
        );
        m.insert(
            "required",
            LocalFunctionSpec {
                name: "required",
                args: &[ArgSpec { name: "value", kind: ArgKind::Any, required: true }],
                returns: ReturnKind::Bool,
                doc: "Returns true when the value is non-null, non-empty-string, non-empty-array.",
            },
        );
        m.insert(
            "regex_match",
            LocalFunctionSpec {
                name: "regex_match",
                args: &[
                    ArgSpec { name: "value", kind: ArgKind::String, required: true },
                    ArgSpec { name: "pattern", kind: ArgKind::String, required: true },
                ],
                returns: ReturnKind::Bool,
                doc: "Returns true when the value matches the given regex pattern.",
            },
        );

        m
    });

/// Look up a function by its template-visible name.
pub fn lookup(name: &str) -> Option<&'static LocalFunctionSpec> {
    LOCAL_FUNCTIONS.get(name)
}

/// Whether the given name is a registered W7 local function. Called
/// by template preflight (Step 1.6) for every `${fn(...)}` and
/// `functionCall.call` site.
pub fn is_registered(name: &str) -> bool {
    LOCAL_FUNCTIONS.contains_key(name)
}

/// Iterate all registered function specs. Used by the build-time
/// linter and future template-author documentation generator.
pub fn iter() -> impl Iterator<Item = &'static LocalFunctionSpec> {
    LOCAL_FUNCTIONS.values()
}

/// Execute a registered local function with a JSON-shaped argument map.
/// Returns `None` when the function name is not registered (caller
/// should return `HostError::LocalFunctionNotAllowed`). Errors during
/// execution (malformed args, failed URL, bad regex) return `Some(Err)`.
/// Formatter/check functions execute in v1 so templates can render
/// `${format_number(...)}`-style interpolations. Side-effecting functions
/// validate args but do not perform platform actions until L2 action
/// transport lands.
pub fn invoke(name: &str, args: &JsonValue) -> Option<Result<JsonValue, LocalFunctionError>> {
    let spec = lookup(name)?;

    Some(match name {
        "open_url" => validate_open_url(spec, args),
        "format_date" => invoke_format_date(spec, args),
        "format_number" => invoke_format_number(spec, args),
        "required" => invoke_required(spec, args),
        "regex_match" => invoke_regex_match(spec, args),
        _ => Err(LocalFunctionError::Domain {
            function: spec.name,
            message: "registered function has no executor".into(),
        }),
    })
}

fn validate_open_url(spec: &'static LocalFunctionSpec, args: &JsonValue) -> Result<JsonValue, LocalFunctionError> {
    let url = string_arg(spec, args, 0)?;
    let parsed = url::Url::parse(&url).map_err(|err| LocalFunctionError::Domain {
        function: spec.name,
        message: format!("invalid url: {err}"),
    })?;
    match parsed.scheme() {
        "http" | "https" | "mailto" => Ok(JsonValue::Null),
        other => Err(LocalFunctionError::Domain {
            function: spec.name,
            message: format!("unsupported url scheme `{other}`"),
        }),
    }
}

fn invoke_format_date(spec: &'static LocalFunctionSpec, args: &JsonValue) -> Result<JsonValue, LocalFunctionError> {
    let value = string_arg(spec, args, 0)?;
    let pattern = string_arg(spec, args, 1)?;
    let parsed = DateTime::parse_from_rfc3339(&value).map_err(|err| LocalFunctionError::Domain {
        function: spec.name,
        message: format!("invalid RFC3339 datetime: {err}"),
    })?;
    let formatted: DateTime<FixedOffset> = parsed;
    Ok(JsonValue::String(formatted.format(&pattern).to_string()))
}

fn invoke_format_number(spec: &'static LocalFunctionSpec, args: &JsonValue) -> Result<JsonValue, LocalFunctionError> {
    let value = number_arg(spec, args, 0)?;
    let precision = optional_u8_arg(spec, args, 1)?.unwrap_or(0);
    Ok(JsonValue::String(format!("{value:.precision$}", precision = precision as usize)))
}

fn invoke_required(spec: &'static LocalFunctionSpec, args: &JsonValue) -> Result<JsonValue, LocalFunctionError> {
    let value = arg_at(spec, args, 0)?.unwrap_or(&JsonValue::Null);
    let present = match value {
        JsonValue::Null => false,
        JsonValue::String(value) => !value.is_empty(),
        JsonValue::Array(value) => !value.is_empty(),
        JsonValue::Object(value) => !value.is_empty(),
        _ => true,
    };
    Ok(JsonValue::Bool(present))
}

fn invoke_regex_match(spec: &'static LocalFunctionSpec, args: &JsonValue) -> Result<JsonValue, LocalFunctionError> {
    let value = string_arg(spec, args, 0)?;
    let pattern = string_arg(spec, args, 1)?;
    let regex = makepad_widgets::makepad_script::makepad_regex::Regex::new(&pattern)
        .map_err(|err| LocalFunctionError::Domain {
            function: spec.name,
            message: format!("invalid regex: {err}"),
        })?;
    Ok(JsonValue::Bool(regex.run(value.as_str(), &mut [])))
}

fn string_arg(spec: &'static LocalFunctionSpec, args: &JsonValue, index: usize) -> Result<String, LocalFunctionError> {
    arg_at(spec, args, index)?
        .and_then(JsonValue::as_str)
        .map(str::to_string)
        .ok_or_else(|| LocalFunctionError::ArgMismatch {
            function: spec.name,
            detail: format!("arg `{}` must be a string", spec.args[index].name),
        })
}

fn number_arg(spec: &'static LocalFunctionSpec, args: &JsonValue, index: usize) -> Result<f64, LocalFunctionError> {
    arg_at(spec, args, index)?
        .and_then(JsonValue::as_f64)
        .ok_or_else(|| LocalFunctionError::ArgMismatch {
            function: spec.name,
            detail: format!("arg `{}` must be a number", spec.args[index].name),
        })
}

fn optional_u8_arg(
    spec: &'static LocalFunctionSpec,
    args: &JsonValue,
    index: usize,
) -> Result<Option<u8>, LocalFunctionError> {
    let Some(value) = arg_at(spec, args, index)? else {
        return Ok(None);
    };
    let Some(number) = value.as_u64() else {
        return Err(LocalFunctionError::ArgMismatch {
            function: spec.name,
            detail: format!("arg `{}` must be an unsigned integer", spec.args[index].name),
        });
    };
    u8::try_from(number).map(Some).map_err(|_| LocalFunctionError::ArgMismatch {
        function: spec.name,
        detail: format!("arg `{}` must fit in u8", spec.args[index].name),
    })
}

fn arg_at<'a>(
    spec: &'static LocalFunctionSpec,
    args: &'a JsonValue,
    index: usize,
) -> Result<Option<&'a JsonValue>, LocalFunctionError> {
    if index >= spec.args.len() {
        return Err(LocalFunctionError::ArgMismatch {
            function: spec.name,
            detail: format!("unexpected arg index {index}"),
        });
    }

    let value = match args {
        JsonValue::Array(values) => values.get(index),
        JsonValue::Object(values) => values.get(spec.args[index].name),
        _ => None,
    };

    if value.is_none() && spec.args[index].required {
        return Err(LocalFunctionError::ArgMismatch {
            function: spec.name,
            detail: format!("missing required arg `{}`", spec.args[index].name),
        });
    }
    Ok(value)
}

/// Execution-time errors from a registered local function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalFunctionError {
    /// The supplied args did not match the declared `ArgSpec[]`.
    ArgMismatch {
        function: &'static str,
        detail: String,
    },
    /// The function ran but its domain logic failed (e.g. regex parse
    /// error, invalid URL format).
    Domain {
        function: &'static str,
        message: String,
    },
}

impl std::fmt::Display for LocalFunctionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgMismatch { function, detail } => {
                write!(f, "local function {function} arg mismatch: {detail}")
            }
            Self::Domain { function, message } => {
                write!(f, "local function {function} failed: {message}")
            }
        }
    }
}

impl std::error::Error for LocalFunctionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_registry_contains_closed_set() {
        for name in ["open_url", "format_date", "format_number", "required", "regex_match"] {
            assert!(
                is_registered(name),
                "missing required W7 function in v1: {name}"
            );
        }
    }

    #[test]
    fn is_registered_rejects_unknown_function() {
        assert!(!is_registered("exec_shell"));
        assert!(!is_registered("eval"));
        assert!(!is_registered(""));
    }

    #[test]
    fn invoke_on_unknown_returns_none_not_error() {
        // Unknown function must return None so the host can produce
        // `HostError::LocalFunctionNotAllowed` rather than confusing
        // the template with a runtime error.
        assert!(invoke("exec_shell", &JsonValue::Null).is_none());
    }

    #[test]
    fn invoke_format_number_formats_precision() {
        let result = invoke("format_number", &serde_json::json!([23.456, 1]))
            .expect("format_number is registered")
            .expect("valid args should format");

        assert_eq!(result, JsonValue::String("23.5".into()));
    }

    #[test]
    fn invoke_required_rejects_empty_values() {
        let empty = invoke("required", &serde_json::json!([""]))
            .expect("required is registered")
            .expect("valid args should execute");
        let present = invoke("required", &serde_json::json!(["hello"]))
            .expect("required is registered")
            .expect("valid args should execute");

        assert_eq!(empty, JsonValue::Bool(false));
        assert_eq!(present, JsonValue::Bool(true));
    }

    #[test]
    fn invoke_format_date_formats_rfc3339() {
        let result = invoke(
            "format_date",
            &serde_json::json!(["2026-04-23T08:09:10+00:00", "%Y-%m-%d"]),
        )
        .expect("format_date is registered")
        .expect("valid args should format");

        assert_eq!(result, JsonValue::String("2026-04-23".into()));
    }

    #[test]
    fn invoke_regex_match_returns_bool() {
        let result = invoke("regex_match", &serde_json::json!(["light rain", "rain"]))
            .expect("regex_match is registered")
            .expect("valid args should execute");

        assert_eq!(result, JsonValue::Bool(true));
    }

    #[test]
    fn invoke_open_url_rejects_unsafe_scheme() {
        let err = invoke("open_url", &serde_json::json!(["file:///etc/passwd"]))
            .expect("open_url is registered")
            .expect_err("unsafe URL scheme should fail");

        assert!(matches!(
            err,
            LocalFunctionError::Domain {
                function: "open_url",
                ..
            }
        ));
    }

    #[test]
    fn open_url_signature_is_correct() {
        let spec = lookup("open_url").expect("open_url registered");
        assert_eq!(spec.args.len(), 1);
        assert_eq!(spec.args[0].name, "url");
        assert_eq!(spec.args[0].kind, ArgKind::String);
        assert!(spec.args[0].required);
        assert_eq!(spec.returns, ReturnKind::Unit);
    }

    #[test]
    fn required_returns_bool_for_check_sites() {
        // Templates use `required(...)` inside validation `checks` arrays,
        // which demand a Bool return.
        let spec = lookup("required").expect("required registered");
        assert_eq!(spec.returns, ReturnKind::Bool);
    }

    #[test]
    fn regex_match_also_returns_bool() {
        let spec = lookup("regex_match").expect("regex_match registered");
        assert_eq!(spec.returns, ReturnKind::Bool);
    }

    #[test]
    fn format_functions_return_string_for_binding_sites() {
        // Templates that interpolate `${format_date(...)}` into a Label
        // need a String return.
        assert_eq!(lookup("format_date").unwrap().returns, ReturnKind::String);
        assert_eq!(lookup("format_number").unwrap().returns, ReturnKind::String);
    }

    #[test]
    fn iter_returns_closed_set_of_five() {
        let all: Vec<_> = iter().collect();
        assert_eq!(
            all.len(),
            5,
            "v1 closed set must have exactly 5 functions; changing this requires a design review"
        );
    }

    #[test]
    fn local_function_error_display_is_human_readable() {
        let err = LocalFunctionError::Domain {
            function: "regex_match",
            message: "invalid regex: unclosed group".into(),
        };
        let rendered = format!("{err}");
        assert!(rendered.contains("regex_match"));
        assert!(rendered.contains("unclosed group"));
    }
}
