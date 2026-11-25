// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A stage's external interface and the system selected for this flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalPortInfo {
    pub relation: PortRelation,
    pub port: String,
    pub system: String,
    #[serde(flatten)]
    pub connection: SystemConnectionInfo,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requests: Vec<PortRequest>,
}

/// Operator-facing connection details declared by the selected system.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemConnectionInfo {
    /// Operator-facing name of the system bound to this port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
    /// Host, optional port and path. Excludes credentials, query and fragment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
}

impl ExternalPortInfo {
    /// The binding's display name, falling back to its declared system ID.
    pub fn system_label(&self) -> &str {
        self.connection
            .display_name
            .as_deref()
            .unwrap_or(&self.system)
    }
}

impl SystemConnectionInfo {
    pub fn valid_address(value: &str) -> bool {
        if !single_line(value)
            || value.contains(['@', '?', '#', '\\'])
            || value.chars().any(char::is_whitespace)
        {
            return false;
        }
        let authority = value.split('/').next().unwrap_or_default();
        let valid_port = |port: &str| {
            !port.is_empty()
                && port.bytes().all(|b| b.is_ascii_digit())
                && port.parse::<u16>().is_ok()
        };
        if let Some(v6) = authority.strip_prefix('[') {
            return v6.split_once(']').is_some_and(|(ip, suffix)| {
                ip.parse::<std::net::Ipv6Addr>().is_ok()
                    && (suffix.is_empty() || suffix.strip_prefix(':').is_some_and(valid_port))
            });
        }
        let (host, port) = authority
            .split_once(':')
            .map_or((authority, None), |(h, p)| (h, Some(p)));
        !host.is_empty()
            && host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
            && port.is_none_or(valid_port)
    }
}

/// How the stage interacts with the external system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortRelation {
    Asks,
    Reads,
    Receives,
    Delivers,
}

/// A request the stage may perform through its port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortRequest {
    #[serde(rename = "type")]
    pub effect_type: String,
    /// Declared port method, when the producer supplies its name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    pub guarantee: RequestGuarantee,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<RequestOutcome>,
}

/// The execution guarantee declared for a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestGuarantee {
    ExactlyOnce,
    AtLeastOnce,
    AtMostOnce,
}

/// The public facts a request may author, or a reply recorded for the handler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RequestOutcome {
    DomainFacts { types: Vec<String> },
    RecordedReply,
}

pub(crate) fn validate_ports(ports: &[ExternalPortInfo]) -> Result<(), &'static str> {
    let mut names = HashSet::new();
    let mut effects = HashSet::new();
    for row in ports {
        if !single_line(&row.port) || !names.insert(&row.port) {
            return Err("port names must be distinct, nonempty single-line labels");
        }
        let system = row.system.as_bytes();
        if system.len() > 64
            || !system.first().is_some_and(u8::is_ascii_alphanumeric)
            || !system
                .iter()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
        {
            return Err("invalid system name");
        }
        if matches!(row.relation, PortRelation::Reads | PortRelation::Receives)
            && !row.requests.is_empty()
        {
            return Err("source ports cannot declare effect requests");
        }
        if row
            .connection
            .address
            .as_deref()
            .is_some_and(|a| !SystemConnectionInfo::valid_address(a))
        {
            return Err("system addresses must contain only a host, optional port and path");
        }
        if [
            &row.connection.display_name,
            &row.connection.transport,
            &row.connection.resource,
        ]
        .into_iter()
        .flatten()
        .any(|s| !single_line(s))
        {
            return Err("system descriptions must be nonempty single-line labels");
        }
        for request in &row.requests {
            if request
                .method
                .as_deref()
                .is_some_and(|name| !single_line(name))
            {
                return Err("request methods must be nonempty single-line labels");
            }
            if !valid_effect_type(&request.effect_type) || !effects.insert(&request.effect_type) {
                return Err(
                    "request types must be valid, distinct effect identifiers within the stage",
                );
            }
            if let Some(RequestOutcome::DomainFacts { types }) = &request.outcome {
                if row.relation == PortRelation::Delivers {
                    return Err("a delivery request records a reply");
                }
                let mut facts = HashSet::new();
                if types
                    .iter()
                    .any(|name| !single_line(name) || !facts.insert(name))
                {
                    return Err("outcome types must be distinct, nonempty single-line labels");
                }
            }
        }
    }
    Ok(())
}

fn single_line(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}

fn valid_effect_type(value: &str) -> bool {
    value.len() <= 255
        && value.split('.').all(|part| {
            let bytes = part.as_bytes();
            bytes.len() <= 64
                && bytes
                    .first()
                    .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
                && bytes
                    .iter()
                    .all(|c| c.is_ascii_alphanumeric() || *c == b'_')
        })
}
