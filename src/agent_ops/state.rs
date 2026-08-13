//! Contract gate for the Agent Operations Panel.
//!
//! agent-chat has supplied a development manifest for a dedicated client
//! contract. Robrix2 has no released canonical artifact set or operational
//! runtime yet. The panel stays closed until the manifest says the contract is
//! *released* and names the immutable commit it was cut from.
//!
//! The gate verifies both the manifest and the exact artifact bytes compiled
//! into Robrix. Opening therefore requires an auditable diff that vendors and
//! registers the complete release; changing only the status field can never
//! make unverified contract material usable.
//!
//! See `docs/agent-system-planes.md` for why this boundary exists at all.

use std::collections::HashMap;

use serde::Deserialize;

use super::sha256;

/// The cross-repository contract this client speaks.
pub const AGENT_OPS_CLIENT_CONTRACT: &str = "io.agentchat.agent_ops.v1";

/// The only `release_status` that may open the gate.
const RELEASED_STATUS: &str = "released";

/// Artifacts that make up the accepted V1 contract. A released manifest must
/// contain every one of these paths; listing one arbitrary file is not enough
/// to claim that the contract is consumable.
const REQUIRED_ARTIFACT_PATHS: &[&str] = &[
    "cancel-dispatch-response.json",
    "client-session-grant.json",
    "client-session-request.json",
    "client-session.json",
    "contract.schema.json",
    "error-precondition-failed.json",
    "grant-exchange-request.json",
    "grant-proof-material.json",
    "invalid-empty-capability.json",
    "invalid-terminal-resolution-with-recovery.json",
    "invalidation.json",
    "outcome-inspection.json",
    "protocol-limits.json",
    "resolve-outcome-request-accept-completed.json",
    "resolve-outcome-request-continue.json",
    "resolve-outcome-request-keep-blocked.json",
    "resolve-outcome-response.json",
    "session-proof-material.json",
    "snapshot.json",
    "stable-errors.json",
];

/// One canonical artifact compiled into this Robrix build.
///
/// The development manifest has been vendored, but agent-chat has not released
/// the canonical artifact bytes yet. Keep this list empty until those exact
/// files are copied into `specs/fixtures/agent-ops-client-v1/`, then register
/// each with `include_bytes!`. A manifest-only status flip therefore cannot
/// open the gate.
#[derive(Clone, Copy, Debug)]
struct VendoredArtifact<'a> {
    path: &'a str,
    bytes: &'a [u8],
}

const VENDORED_ARTIFACTS: &[VendoredArtifact<'static>] = &[];

/// agent-chat's canonical artifact manifest.
///
/// Unknown fields are ignored so the manifest can grow, but every field the
/// gate depends on is required: a manifest that fails to parse cannot open it.
#[derive(Clone, Debug, Deserialize)]
pub struct ContractManifest {
    pub contract: String,
    pub release_status: String,
    /// The immutable commit the artifacts were cut from. `None` until the
    /// producing repository has committed them.
    #[serde(default)]
    pub source_commit: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<ManifestArtifact>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ManifestArtifact {
    pub path: String,
    pub sha256: String,
}

/// The contract material's readiness state.
///
/// Each variant names something a human can act on; none of them is a generic
/// failure, because "it doesn't work" is not a reviewable state for a gate that
/// guards privileged access.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AgentOpsAvailability {
    /// No manifest was supplied to the gate.
    #[default]
    NoContractManifest,
    /// The manifest could not be parsed, so nothing about it can be trusted.
    UnreadableManifest,
    /// The manifest describes a different contract than this client speaks.
    ContractMismatch { expected: String, found: String },
    /// The manifest does not claim a released contract; its artifacts may
    /// still change, so a client built against them would drift.
    NotReleased { release_status: String },
    /// Released, but not bound to a commit — the artifacts are still a moving
    /// target even though the status claims otherwise.
    UnboundSourceCommit,
    /// The source binding is not a full Git object id, so it does not identify
    /// one immutable producer revision.
    InvalidSourceCommit,
    /// The manifest carries no artifacts, so there is nothing to verify against.
    EmptyArtifactSet,
    /// An artifact path, digest, or uniqueness rule is malformed.
    InvalidArtifactManifest,
    /// The manifest omitted one or more artifacts required by V1.
    IncompleteArtifactManifest { missing_count: usize },
    /// The manifest and the artifact bytes compiled into this build do not name
    /// exactly the same set.
    ArtifactSetMismatch {
        missing_count: usize,
        unexpected_count: usize,
    },
    /// A compiled artifact does not match the digest published by agent-chat.
    ArtifactDigestMismatch { path: String },
    /// The contract artifacts are pinned and verified. This says nothing about
    /// runtime bootstrap or backend connectivity.
    ContractReady {
        source_commit: String,
        artifact_count: usize,
    },
}

impl AgentOpsAvailability {
    pub fn is_contract_ready(&self) -> bool {
        matches!(self, Self::ContractReady { .. })
    }

    /// An English diagnostic for logs and tests. The panel uses localized
    /// templates and only interpolates the manifest values from these variants.
    pub fn explanation(&self) -> String {
        match self {
            Self::NoContractManifest => {
                "The agent-chat client contract manifest was not found. The panel stays closed \
                 until agent-chat ships a released manifest."
                    .to_string()
            }
            Self::UnreadableManifest => {
                "The agent-chat client contract manifest could not be read. Nothing about it can \
                 be trusted, so the panel stays closed."
                    .to_string()
            }
            Self::ContractMismatch { expected, found } => format!(
                "This client speaks {expected}, but the manifest describes {found}. The panel \
                 stays closed rather than guess at a translation."
            ),
            Self::NotReleased { release_status } => format!(
                "The agent-chat client contract is marked \"{release_status}\", not released. Its \
                 artifacts can still change, so the panel stays closed."
            ),
            Self::UnboundSourceCommit => {
                "The agent-chat client contract is marked released but names no source commit, so \
                 its artifacts are still a moving target. The panel stays closed."
                    .to_string()
            }
            Self::InvalidSourceCommit => {
                "The agent-chat client contract source binding is not a full Git object id. The \
                 panel stays closed until it names one immutable producer revision."
                    .to_string()
            }
            Self::EmptyArtifactSet => {
                "The agent-chat client contract manifest lists no artifacts, so there is nothing \
                 to verify against. The panel stays closed."
                    .to_string()
            }
            Self::InvalidArtifactManifest => {
                "The agent-chat client contract manifest contains an invalid artifact path, digest, \
                 or duplicate entry. The panel stays closed."
                    .to_string()
            }
            Self::IncompleteArtifactManifest { missing_count } => format!(
                "The agent-chat client contract manifest omits {missing_count} artifacts required \
                 by V1. The panel stays closed."
            ),
            Self::ArtifactSetMismatch { missing_count, unexpected_count } => format!(
                "The compiled artifact set does not match the manifest ({missing_count} missing, \
                 {unexpected_count} unexpected). The panel stays closed."
            ),
            Self::ArtifactDigestMismatch { path } => format!(
                "The compiled artifact {path} does not match agent-chat's published SHA-256 digest. \
                 The panel stays closed."
            ),
            Self::ContractReady { source_commit, artifact_count } => format!(
                "The agent-chat client contract artifacts are pinned to {source_commit} and all \
                 {artifact_count} digests match. No runtime connection has been attempted."
            ),
        }
    }
}

/// Decides whether the contract is ready for a future runtime.
///
/// Deliberately has no configuration, credential, URL, polling, or mutation
/// API. Those belong to the client runtime, which may only be built after this
/// gate reports [`AgentOpsAvailability::ContractReady`].
#[derive(Clone, Debug, Default)]
pub struct AgentOpsContractGate {
    availability: AgentOpsAvailability,
}

/// agent-chat's manifest, vendored into this repository and compiled in.
///
/// Embedding rather than reading it at runtime is deliberate: the gate's answer
/// becomes a fact pinned by *this* repository's commit, with no filesystem
/// lookup, no environment dependence, and no network call. Updating the
/// contract is therefore an explicit, reviewable act — copy the released
/// manifest and every canonical artifact in, then register their bytes above.
const VENDORED_MANIFEST: &str =
    include_str!("../../specs/fixtures/agent-ops-client-v1/manifest.json");

impl AgentOpsContractGate {
    /// The gate as it stands in this build, judged from the vendored manifest.
    pub fn from_vendored_manifest() -> Self {
        Self::from_manifest_json_and_artifacts(VENDORED_MANIFEST, VENDORED_ARTIFACTS)
    }

    /// A gate with no manifest: closed.
    pub fn closed() -> Self {
        Self { availability: AgentOpsAvailability::NoContractManifest }
    }

    /// Evaluate raw manifest bytes. Any failure closes the gate; there is no
    /// path through this function that opens it on partial information.
    pub fn from_manifest_json(json: &str) -> Self {
        Self::from_manifest_json_and_artifacts(json, &[])
    }

    fn from_manifest_json_and_artifacts(
        json: &str,
        artifacts: &[VendoredArtifact<'_>],
    ) -> Self {
        let Ok(manifest) = serde_json::from_str::<ContractManifest>(json) else {
            return Self { availability: AgentOpsAvailability::UnreadableManifest };
        };
        Self::from_manifest_and_artifacts(&manifest, artifacts)
    }

    pub fn from_manifest(manifest: &ContractManifest) -> Self {
        Self::from_manifest_and_artifacts(manifest, &[])
    }

    fn from_manifest_and_artifacts(
        manifest: &ContractManifest,
        artifacts: &[VendoredArtifact<'_>],
    ) -> Self {
        Self { availability: Self::evaluate(manifest, artifacts) }
    }

    fn evaluate(
        manifest: &ContractManifest,
        artifacts: &[VendoredArtifact<'_>],
    ) -> AgentOpsAvailability {
        if manifest.contract != AGENT_OPS_CLIENT_CONTRACT {
            return AgentOpsAvailability::ContractMismatch {
                expected: AGENT_OPS_CLIENT_CONTRACT.to_string(),
                found: manifest.contract.clone(),
            };
        }
        if manifest.release_status != RELEASED_STATUS {
            return AgentOpsAvailability::NotReleased {
                release_status: manifest.release_status.clone(),
            };
        }
        // Released without a commit is the more dangerous case of the two: the
        // status invites trust that the binding does not support.
        let Some(commit) = manifest.source_commit.as_deref().map(str::trim).filter(|c| !c.is_empty())
        else {
            return AgentOpsAvailability::UnboundSourceCommit;
        };
        if !is_full_git_object_id(commit) {
            return AgentOpsAvailability::InvalidSourceCommit;
        }
        if manifest.artifacts.is_empty() {
            return AgentOpsAvailability::EmptyArtifactSet;
        }

        let mut manifest_by_path = HashMap::with_capacity(manifest.artifacts.len());
        for artifact in &manifest.artifacts {
            if !is_safe_artifact_path(&artifact.path)
                || !is_lowercase_hex(&artifact.sha256, 64)
                || manifest_by_path.insert(artifact.path.as_str(), artifact.sha256.as_str()).is_some()
            {
                return AgentOpsAvailability::InvalidArtifactManifest;
            }
        }

        let missing_required = REQUIRED_ARTIFACT_PATHS
            .iter()
            .filter(|path| !manifest_by_path.contains_key(**path))
            .count();
        if missing_required > 0 {
            return AgentOpsAvailability::IncompleteArtifactManifest {
                missing_count: missing_required,
            };
        }

        let mut vendored_by_path = HashMap::with_capacity(artifacts.len());
        for artifact in artifacts {
            if !is_safe_artifact_path(artifact.path)
                || vendored_by_path.insert(artifact.path, artifact.bytes).is_some()
            {
                return AgentOpsAvailability::InvalidArtifactManifest;
            }
        }
        let missing_count = manifest_by_path
            .keys()
            .filter(|path| !vendored_by_path.contains_key(**path))
            .count();
        let unexpected_count = vendored_by_path
            .keys()
            .filter(|path| !manifest_by_path.contains_key(**path))
            .count();
        if missing_count > 0 || unexpected_count > 0 {
            return AgentOpsAvailability::ArtifactSetMismatch {
                missing_count,
                unexpected_count,
            };
        }

        for (path, expected_digest) in manifest_by_path {
            let Some(bytes) = vendored_by_path.get(path) else {
                return AgentOpsAvailability::ArtifactSetMismatch {
                    missing_count: 1,
                    unexpected_count: 0,
                };
            };
            if sha256::hex_digest(bytes) != expected_digest {
                return AgentOpsAvailability::ArtifactDigestMismatch {
                    path: path.to_string(),
                };
            }
        }

        AgentOpsAvailability::ContractReady {
            source_commit: commit.to_string(),
            artifact_count: manifest.artifacts.len(),
        }
    }

    pub fn availability(&self) -> &AgentOpsAvailability {
        &self.availability
    }

    pub fn is_contract_ready(&self) -> bool {
        self.availability.is_contract_ready()
    }

    pub fn explanation(&self) -> String {
        self.availability.explanation()
    }
}

fn is_full_git_object_id(value: &str) -> bool {
    is_lowercase_hex(value, 40) || is_lowercase_hex(value, 64)
}

fn is_lowercase_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn is_safe_artifact_path(value: &str) -> bool {
    let path = std::path::Path::new(value);
    !value.is_empty()
        && !value.contains('\\')
        && !path.is_absolute()
        && path.components().all(|component| {
            matches!(component, std::path::Component::Normal(_))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE_COMMIT: &str = "9f2c1ab4d5e6f70819a2b3c4d5e6f7089a1b2c3d";

    fn manifest_json(status: &str, commit: &str) -> String {
        let commit_field =
            if commit.is_empty() { "null".to_string() } else { format!("\"{commit}\"") };
        format!(
            r#"{{"contract":"{AGENT_OPS_CLIENT_CONTRACT}","release_status":"{status}",
                "source_commit":{commit_field},"artifacts":[]}}"#,
        )
    }

    fn matching_artifacts() -> Vec<VendoredArtifact<'static>> {
        REQUIRED_ARTIFACT_PATHS
            .iter()
            .map(|path| VendoredArtifact {
                path,
                bytes: path.as_bytes(),
            })
            .collect()
    }

    fn released_manifest(artifacts: &[VendoredArtifact<'_>]) -> ContractManifest {
        ContractManifest {
            contract: AGENT_OPS_CLIENT_CONTRACT.into(),
            release_status: RELEASED_STATUS.into(),
            source_commit: Some(SOURCE_COMMIT.into()),
            artifacts: artifacts
                .iter()
                .map(|artifact| ManifestArtifact {
                    path: artifact.path.into(),
                    sha256: sha256::hex_digest(artifact.bytes),
                })
                .collect(),
        }
    }

    #[test]
    fn contract_gate_is_fail_closed() {
        let gate = AgentOpsContractGate::closed();
        assert!(!gate.is_contract_ready());
        assert_eq!(gate.availability(), &AgentOpsAvailability::NoContractManifest);
        assert!(gate.explanation().contains("stays closed"));
    }

    /// The manifest agent-chat ships today is deliberately not released. This
    /// is the exact condition the gate exists to hold shut.
    #[test]
    fn development_manifest_keeps_the_panel_closed() {
        let gate = AgentOpsContractGate::from_manifest_json(&manifest_json("development", ""));
        assert!(!gate.is_contract_ready());
        assert_eq!(
            gate.availability(),
            &AgentOpsAvailability::NotReleased { release_status: "development".into() },
        );
        assert!(gate.explanation().contains("development"));
    }

    /// Claiming released while naming no commit is the trap worth naming
    /// separately: the status invites trust the binding cannot support.
    #[test]
    fn released_without_a_source_commit_stays_closed() {
        let gate = AgentOpsContractGate::from_manifest_json(&manifest_json("released", ""));
        assert!(!gate.is_contract_ready());
        assert_eq!(gate.availability(), &AgentOpsAvailability::UnboundSourceCommit);
    }

    #[test]
    fn blank_source_commit_is_treated_as_unbound() {
        let gate = AgentOpsContractGate::from_manifest_json(&manifest_json("released", "   "));
        assert_eq!(gate.availability(), &AgentOpsAvailability::UnboundSourceCommit);
    }

    #[test]
    fn abbreviated_source_commit_stays_closed() {
        let gate = AgentOpsContractGate::from_manifest_json(&manifest_json("released", "abc123"));
        assert_eq!(gate.availability(), &AgentOpsAvailability::InvalidSourceCommit);
    }

    /// The core promise of the byte layer: a manifest can look perfect —
    /// released, fully bound, all twenty required artifacts with correct
    /// digests — yet the gate must stay shut while this build vendors no
    /// artifact bytes. A status-field flip alone can never open it.
    #[test]
    fn a_perfect_manifest_with_no_vendored_bytes_stays_closed() {
        let manifest = released_manifest(&matching_artifacts());
        // Deliberately verify against an EMPTY vendored set, exactly the state
        // this build ships in (VENDORED_ARTIFACTS == &[]).
        let gate = AgentOpsContractGate::from_manifest_and_artifacts(&manifest, &[]);
        assert!(!gate.is_contract_ready());
        assert!(
            matches!(gate.availability(), &AgentOpsAvailability::ArtifactSetMismatch { .. }),
            "empty vendored bytes must be a set mismatch, not readiness: {:?}",
            gate.availability(),
        );
    }

    /// The shipping build's own gate, with its real empty vendored set, must
    /// not be ready regardless of what the vendored manifest later becomes.
    #[test]
    fn shipping_gate_with_empty_vendored_set_is_not_ready() {
        assert!(!AgentOpsContractGate::from_vendored_manifest().is_contract_ready());
        assert!(VENDORED_ARTIFACTS.is_empty(), "guarding the assumption this test relies on");
    }

    #[test]
    fn released_manifest_and_matching_artifacts_make_the_contract_ready() {
        let artifacts = matching_artifacts();
        let manifest = released_manifest(&artifacts);
        let gate = AgentOpsContractGate::from_manifest_and_artifacts(&manifest, &artifacts);
        assert!(gate.is_contract_ready());
        assert_eq!(
            gate.availability(),
            &AgentOpsAvailability::ContractReady {
                source_commit: SOURCE_COMMIT.into(),
                artifact_count: 20,
            },
        );
        assert!(gate.explanation().contains("9f2c1ab4"));
        assert!(gate.explanation().contains("No runtime connection"));
    }

    #[test]
    fn a_released_manifest_with_no_artifacts_stays_closed() {
        let gate = AgentOpsContractGate::from_manifest_json(&manifest_json("released", SOURCE_COMMIT));
        assert_eq!(gate.availability(), &AgentOpsAvailability::EmptyArtifactSet);
    }

    #[test]
    fn manifest_only_release_cannot_open_without_compiled_artifacts() {
        let artifacts = matching_artifacts();
        let manifest = released_manifest(&artifacts);
        let gate = AgentOpsContractGate::from_manifest(&manifest);
        assert_eq!(
            gate.availability(),
            &AgentOpsAvailability::ArtifactSetMismatch {
                missing_count: REQUIRED_ARTIFACT_PATHS.len(),
                unexpected_count: 0,
            },
        );
    }

    #[test]
    fn incomplete_v1_manifest_stays_closed() {
        let artifacts = matching_artifacts();
        let mut manifest = released_manifest(&artifacts);
        manifest.artifacts.pop();
        let gate = AgentOpsContractGate::from_manifest_and_artifacts(
            &manifest,
            &artifacts[..artifacts.len() - 1],
        );
        assert_eq!(
            gate.availability(),
            &AgentOpsAvailability::IncompleteArtifactManifest { missing_count: 1 },
        );
    }

    #[test]
    fn malformed_or_duplicate_artifact_entries_stay_closed() {
        let artifacts = matching_artifacts();
        let mut manifest = released_manifest(&artifacts);
        manifest.artifacts[0].sha256 = "not-a-sha256".into();
        let gate = AgentOpsContractGate::from_manifest_and_artifacts(&manifest, &artifacts);
        assert_eq!(gate.availability(), &AgentOpsAvailability::InvalidArtifactManifest);

        let mut manifest = released_manifest(&artifacts);
        manifest.artifacts.push(manifest.artifacts[0].clone());
        let gate = AgentOpsContractGate::from_manifest_and_artifacts(&manifest, &artifacts);
        assert_eq!(gate.availability(), &AgentOpsAvailability::InvalidArtifactManifest);

        let mut manifest = released_manifest(&artifacts);
        manifest.artifacts[0].path = "../contract.schema.json".into();
        let gate = AgentOpsContractGate::from_manifest_and_artifacts(&manifest, &artifacts);
        assert_eq!(gate.availability(), &AgentOpsAvailability::InvalidArtifactManifest);

        let mut manifest = released_manifest(&artifacts);
        manifest.artifacts[0].path = r#"nested\contract.schema.json"#.into();
        let gate = AgentOpsContractGate::from_manifest_and_artifacts(&manifest, &artifacts);
        assert_eq!(gate.availability(), &AgentOpsAvailability::InvalidArtifactManifest);
    }

    #[test]
    fn artifact_set_and_digest_mismatches_stay_closed() {
        let artifacts = matching_artifacts();
        let manifest = released_manifest(&artifacts);

        let gate = AgentOpsContractGate::from_manifest_and_artifacts(
            &manifest,
            &artifacts[..artifacts.len() - 1],
        );
        assert_eq!(
            gate.availability(),
            &AgentOpsAvailability::ArtifactSetMismatch {
                missing_count: 1,
                unexpected_count: 0,
            },
        );

        let mut tampered = artifacts.clone();
        tampered[0].bytes = b"tampered";
        let gate = AgentOpsContractGate::from_manifest_and_artifacts(&manifest, &tampered);
        assert!(matches!(
            gate.availability(),
            AgentOpsAvailability::ArtifactDigestMismatch { .. },
        ));
    }

    #[test]
    fn a_different_contract_never_opens_the_gate() {
        let json = r#"{"contract":"io.agentchat.agent_ops.v2","release_status":"released",
                       "source_commit":"abc123","artifacts":[{"path":"a.json","sha256":"00"}]}"#;
        let gate = AgentOpsContractGate::from_manifest_json(json);
        assert!(!gate.is_contract_ready());
        assert!(matches!(
            gate.availability(),
            AgentOpsAvailability::ContractMismatch { .. },
        ));
        assert!(gate.explanation().contains("v2"));
    }

    #[test]
    fn unparsable_manifest_stays_closed() {
        for json in ["", "not json", "{}", r#"{"contract":"io.agentchat.agent_ops.v1"}"#] {
            let gate = AgentOpsContractGate::from_manifest_json(json);
            assert!(!gate.is_contract_ready(), "must not open on {json:?}");
        }
    }

    /// The manifest actually vendored in this build must parse and must name
    /// the contract this client speaks. Whether it opens the gate is a fact
    /// about the contract's release state, asserted separately below.
    #[test]
    fn vendored_manifest_is_parseable_and_names_this_contract() {
        let manifest: ContractManifest = serde_json::from_str(VENDORED_MANIFEST)
            .expect("the vendored manifest must parse");
        assert_eq!(manifest.contract, AGENT_OPS_CLIENT_CONTRACT);
        assert!(!manifest.artifacts.is_empty(), "a contract with no artifacts is not a contract");
    }

    /// A future release-status change must be accompanied by all canonical
    /// bytes and their registrations. Changing only the manifest must fail.
    #[test]
    fn vendored_manifest_only_becomes_ready_with_verified_artifacts() {
        let manifest: ContractManifest = serde_json::from_str(VENDORED_MANIFEST).unwrap();
        let gate = AgentOpsContractGate::from_vendored_manifest();

        if manifest.release_status == RELEASED_STATUS {
            assert!(
                gate.is_contract_ready(),
                "a released manifest must have a full commit binding and every matching artifact",
            );
        } else {
            assert!(!gate.is_contract_ready());
        }
    }

    /// Every closed state must say something a human can act on, and no closed
    /// state may claim the panel is connected.
    #[test]
    fn every_closed_state_explains_itself() {
        let closed = vec![
            AgentOpsAvailability::NoContractManifest,
            AgentOpsAvailability::UnreadableManifest,
            AgentOpsAvailability::ContractMismatch {
                expected: "a".into(),
                found: "b".into(),
            },
            AgentOpsAvailability::NotReleased { release_status: "development".into() },
            AgentOpsAvailability::UnboundSourceCommit,
            AgentOpsAvailability::InvalidSourceCommit,
            AgentOpsAvailability::EmptyArtifactSet,
            AgentOpsAvailability::InvalidArtifactManifest,
            AgentOpsAvailability::IncompleteArtifactManifest { missing_count: 1 },
            AgentOpsAvailability::ArtifactSetMismatch {
                missing_count: 1,
                unexpected_count: 0,
            },
            AgentOpsAvailability::ArtifactDigestMismatch { path: "snapshot.json".into() },
        ];
        for state in closed {
            assert!(!state.is_contract_ready());
            let text = state.explanation();
            assert!(text.len() > 40, "explanation too thin: {text}");
            assert!(!text.contains("Connected"), "closed state must not claim connection");
        }
    }
}
