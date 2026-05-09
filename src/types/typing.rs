// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Authoring-time stage and edge type contracts (FLOWIP-114b).
//!
//! Stage typing captures the declared input/output (and join leg) types for
//! a stage. Edge typing is a per-edge projection that names which payload
//! type flows across one connection, derived from the surrounding stage
//! contracts and topology metadata.
//!
//! These types are structural authoring metadata. They are read by clients
//! (Studio, UI) but are ignored by graph validation and traversal.

use serde::{Deserialize, Serialize};

/// Three-way model for declared type positions.
///
/// `Unspecified` means no contract was declared. `Exact` carries the raw
/// captured type identifier (from `stringify!($ty)` or `type_name::<T>()`).
/// `Mixed` indicates that more than one type can flow through this position.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeHintInfo {
    Unspecified,
    Exact { name: String },
    Mixed,
}

impl TypeHintInfo {
    /// Convenience constructor for an exact type hint.
    pub fn exact(name: impl Into<String>) -> Self {
        Self::Exact { name: name.into() }
    }

    /// Render the raw type name for UI display.
    ///
    /// Strips Rust path qualifiers (`crate::module::Type` -> `Type`) and splits
    /// CamelCase boundaries into spaces. Leaves generics intact. Returns
    /// `None` for `Unspecified` and `Mixed`.
    ///
    /// This is the conservative v0.5 formatter. Studio overlay feedback may
    /// drive future refinement; clients must treat the result as render-only
    /// and never compare it for compatibility.
    pub fn display_name(&self) -> Option<String> {
        match self {
            TypeHintInfo::Exact { name } => Some(format_display_name(name)),
            TypeHintInfo::Unspecified | TypeHintInfo::Mixed => None,
        }
    }
}

/// Authoring-time stage typing contract (FLOWIP-114b).
///
/// All positions default to `Unspecified` for stages that did not declare a
/// type at the relevant slot. Sources only fill `output_type`; sinks only
/// fill `input_type`; transforms/stateful fill `input_type` and
/// `output_type`; joins fill `reference_type`, `stream_type`, and
/// `output_type`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageTypingInfo {
    pub input_type: TypeHintInfo,
    pub output_type: TypeHintInfo,
    pub boundary_in_type: TypeHintInfo,
    pub boundary_out_type: TypeHintInfo,
    pub reference_type: TypeHintInfo,
    pub stream_type: TypeHintInfo,
    pub is_placeholder: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder_message: Option<String>,
}

impl StageTypingInfo {
    /// All positions `Unspecified`, not a placeholder.
    pub fn empty() -> Self {
        Self {
            input_type: TypeHintInfo::Unspecified,
            output_type: TypeHintInfo::Unspecified,
            boundary_in_type: TypeHintInfo::Unspecified,
            boundary_out_type: TypeHintInfo::Unspecified,
            reference_type: TypeHintInfo::Unspecified,
            stream_type: TypeHintInfo::Unspecified,
            is_placeholder: false,
            placeholder_message: None,
        }
    }
}

/// Per-edge payload typing projection (FLOWIP-114b).
///
/// Built at flow-build time from the surrounding stage contracts and
/// topology role information. The endpoint serves it as-is; clients label
/// edges with `payload_type` and key colour/role on `role`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeTypingInfo {
    pub role: EdgeTypingRole,
    pub label_source: EdgeTypingLabelSource,
    pub payload_type: TypeHintInfo,
}

impl EdgeTypingInfo {
    pub fn new(
        role: EdgeTypingRole,
        label_source: EdgeTypingLabelSource,
        payload_type: TypeHintInfo,
    ) -> Self {
        Self {
            role,
            label_source,
            payload_type,
        }
    }
}

/// Role of an edge within a typed stage's input space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeTypingRole {
    Input,
    Reference,
    Stream,
}

/// Which contract slot the edge label was projected from.
///
/// Clients use this to disambiguate why a particular type appears on the
/// edge (e.g., it was derived from the upstream stage's `output_type` vs.
/// the downstream stage's `reference_type`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeTypingLabelSource {
    UpstreamOutputType,
    DownstreamInputType,
    DownstreamReferenceType,
    DownstreamStreamType,
}

/// Render a captured Rust type name for UI display.
///
/// Strips Rust path qualifiers (`crate::module::Type` -> `Type`) so
/// fully qualified `type_name::<T>()` strings collapse to the same form
/// `stringify!($ty)` would produce. PascalCase type names are kept
/// intact — splitting them on word boundaries makes tooltips
/// unrecognisable against the underlying source code.
fn format_display_name(name: &str) -> String {
    strip_rust_path_qualifiers(name)
}

fn strip_rust_path_qualifiers(name: &str) -> String {
    let mut result = String::new();
    let mut token = String::new();

    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' {
            token.push(ch);
        } else {
            push_final_path_segment(&mut result, &token);
            token.clear();
            result.push(ch);
        }
    }
    push_final_path_segment(&mut result, &token);

    result
}

fn push_final_path_segment(result: &mut String, token: &str) {
    if token.is_empty() {
        return;
    }

    result.push_str(token.rsplit("::").next().unwrap_or(token));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_strips_path_qualifiers() {
        let hint = TypeHintInfo::exact("product_catalog::domain::EnrichedOrder");
        assert_eq!(hint.display_name().as_deref(), Some("EnrichedOrder"));
    }

    #[test]
    fn display_name_preserves_pascal_case() {
        // Programmer-recognisable PascalCase is kept intact so the
        // tooltip text matches the source-code spelling.
        let hint = TypeHintInfo::exact("EnrichedOrderWithPromo");
        assert_eq!(hint.display_name().as_deref(), Some("EnrichedOrderWithPromo"));
    }

    #[test]
    fn display_name_preserves_generics() {
        let hint = TypeHintInfo::exact("Vec<OrderEvent>");
        assert_eq!(hint.display_name().as_deref(), Some("Vec<OrderEvent>"));
    }

    #[test]
    fn display_name_preserves_acronyms() {
        // Acronyms stay glued to neighbouring words; no synthesised
        // space splits.
        let hint = TypeHintInfo::exact("HTTPRequest");
        assert_eq!(hint.display_name().as_deref(), Some("HTTPRequest"));
    }

    #[test]
    fn display_name_strips_paths_inside_generics() {
        let hint = TypeHintInfo::exact("Vec<product_catalog::domain::OrderEvent>");
        assert_eq!(hint.display_name().as_deref(), Some("Vec<OrderEvent>"));
    }

    #[test]
    fn unspecified_and_mixed_render_to_none() {
        assert_eq!(TypeHintInfo::Unspecified.display_name(), None);
        assert_eq!(TypeHintInfo::Mixed.display_name(), None);
    }

    #[test]
    fn type_hint_info_round_trips_via_serde() {
        for hint in [
            TypeHintInfo::Unspecified,
            TypeHintInfo::exact("Foo::Bar"),
            TypeHintInfo::Mixed,
        ] {
            let json = serde_json::to_string(&hint).unwrap();
            let back: TypeHintInfo = serde_json::from_str(&json).unwrap();
            assert_eq!(back, hint);
        }
    }

    #[test]
    fn stage_typing_info_round_trips_with_defaults() {
        let info = StageTypingInfo {
            input_type: TypeHintInfo::Unspecified,
            output_type: TypeHintInfo::exact("Foo"),
            boundary_in_type: TypeHintInfo::Unspecified,
            boundary_out_type: TypeHintInfo::Unspecified,
            reference_type: TypeHintInfo::exact("Bar"),
            stream_type: TypeHintInfo::exact("Baz"),
            is_placeholder: false,
            placeholder_message: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        // placeholder_message must be omitted when None.
        assert!(!json.contains("placeholder_message"));
        let back: StageTypingInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back, info);
    }

    #[test]
    fn edge_typing_round_trips_with_snake_case_enums() {
        let typing = EdgeTypingInfo::new(
            EdgeTypingRole::Stream,
            EdgeTypingLabelSource::DownstreamStreamType,
            TypeHintInfo::exact("OrderEvent"),
        );
        let json = serde_json::to_string(&typing).unwrap();
        assert!(json.contains("\"role\":\"stream\""));
        assert!(json.contains("\"label_source\":\"downstream_stream_type\""));
        let back: EdgeTypingInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back, typing);
    }
}
