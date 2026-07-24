//! A modal dialog for viewing and editing room settings.

use std::path::PathBuf;

use makepad_widgets::*;
use ruma::{OwnedRoomAliasId, OwnedRoomId, RoomAliasId, RoomId, ServerName};

use crate::i18n::{AppLanguage, tr_key};
use crate::shared::avatar::AvatarWidgetExt;
use crate::utils::load_png_or_jpg;

// ─────────────────────────────────────────────────────────────────────────────
// Room-alias management: pure logic (no UI / no network), unit-tested below.
//
// These functions back the "Room Aliases" section of the room settings modal.
// They are deliberately pure so their behaviour can be verified without a
// Makepad context or a live Matrix connection (see `specs/task-room-aliases.spec.md`).
// ─────────────────────────────────────────────────────────────────────────────

/// Why a user-entered alias string was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasInputError {
    /// Input was empty (after trimming).
    Empty,
    /// Input contained whitespace, which is never valid in a room alias.
    ContainsWhitespace,
    /// Input did not parse as a valid `#localpart:server` room alias.
    InvalidFormat,
}

/// Normalize and validate a user-entered room alias.
///
/// - `#localpart:server` (or any string containing `#`/`:`) is parsed as an
///   explicit alias and must be well-formed.
/// - A bare `localpart` (no `#` and no `:`) is completed to
///   `#{localpart}:{homeserver}`, matching how [`parse_address`](super) treats
///   bare room addresses against the current homeserver.
///
/// Returns [`AliasInputError`] instead of panicking on any malformed input.
pub fn normalize_and_validate_alias(
    input: &str,
    homeserver: &ServerName,
) -> Result<OwnedRoomAliasId, AliasInputError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AliasInputError::Empty);
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err(AliasInputError::ContainsWhitespace);
    }
    // A bare localpart (no sigil, no server delimiter) is resolved against the
    // current homeserver; anything else is treated as an explicit alias.
    let candidate = if trimmed.starts_with('#') || trimmed.contains(':') {
        trimmed.to_string()
    } else {
        format!("#{trimmed}:{homeserver}")
    };
    let parsed = OwnedRoomAliasId::try_from(candidate.as_str())
        .map_err(|_| AliasInputError::InvalidFormat)?;
    // ruma leniently accepts an empty localpart (e.g. "#:server"); a usable room
    // alias must have a non-empty localpart, so reject it explicitly.
    if parsed.alias().is_empty() {
        return Err(AliasInputError::InvalidFormat);
    }
    Ok(parsed)
}

/// A single alias-management operation requested from the UI.
#[derive(Debug, Clone)]
pub enum AliasOp {
    /// Promote an already-published alias to be the room's canonical alias.
    SetCanonical(OwnedRoomAliasId),
    /// Remove an alias from the room (from canonical and/or the alt list).
    Remove(OwnedRoomAliasId),
}

/// The `(canonical, alt_aliases)` pair to write into the `m.room.canonical_alias`
/// state event after applying an [`AliasOp`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAliasState {
    pub canonical: Option<OwnedRoomAliasId>,
    pub alt_aliases: Vec<OwnedRoomAliasId>,
}

/// Why an [`AliasOp`] could not be reconciled into a new canonical-alias state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalReconcileError {
    /// Tried to set an alias canonical that is neither the current canonical nor
    /// a published alt alias — it must be published to the directory first.
    NotPublished,
}

/// Compute the new `(canonical, alt_aliases)` after applying `op`, enforcing the
/// invariants of `m.room.canonical_alias`:
///
/// - Setting an alias canonical requires it to already be published (canonical ∪ alts).
/// - The previous canonical (if different) is demoted into `alt_aliases`.
/// - The canonical alias never also appears in `alt_aliases` (deduped).
/// - Removing the current canonical clears it; removing an alt just drops it.
pub fn reconcile_canonical_alias(
    current_canonical: Option<&RoomAliasId>,
    current_alts: &[OwnedRoomAliasId],
    op: AliasOp,
) -> Result<CanonicalAliasState, CanonicalReconcileError> {
    // Compare via canonical string form to avoid borrowed/owned PartialEq ambiguity.
    let target = match &op {
        AliasOp::SetCanonical(a) | AliasOp::Remove(a) => a.clone(),
    };
    let target_str = target.as_str();
    match op {
        AliasOp::SetCanonical(_) => {
            let is_published = current_canonical.is_some_and(|c| c.as_str() == target_str)
                || current_alts.iter().any(|a| a.as_str() == target_str);
            if !is_published {
                return Err(CanonicalReconcileError::NotPublished);
            }
            let mut alts: Vec<OwnedRoomAliasId> = Vec::new();
            // Demote the old canonical (when it differs from the new one).
            if let Some(old) = current_canonical {
                if old.as_str() != target_str {
                    alts.push(old.to_owned());
                }
            }
            // Keep the remaining alts, minus the new canonical, without duplicates.
            for a in current_alts {
                if a.as_str() != target_str && !alts.iter().any(|x| x.as_str() == a.as_str()) {
                    alts.push(a.clone());
                }
            }
            Ok(CanonicalAliasState { canonical: Some(target), alt_aliases: alts })
        }
        AliasOp::Remove(_) => {
            let canonical = match current_canonical {
                Some(c) if c.as_str() == target_str => None,
                other => other.map(RoomAliasId::to_owned),
            };
            let alt_aliases = current_alts
                .iter()
                .filter(|a| a.as_str() != target_str)
                .cloned()
                .collect();
            Ok(CanonicalAliasState { canonical, alt_aliases })
        }
    }
}

/// Compute the `alt_aliases` list to advertise after publishing `new_alias`.
///
/// Used by the optimistic "publish → auto-advertise" flow: the freshly
/// published alias is appended to the room's existing alt aliases so it shows
/// up as advertised immediately. The result preserves the `m.room.canonical_alias`
/// invariants:
///
/// - The canonical alias is never duplicated into `alt_aliases`.
/// - An alias already present (canonical or alt) is not added twice.
///
/// The canonical alias itself is passed only so it can be excluded; it is never
/// added or removed here (that stays with [`reconcile_canonical_alias`]).
pub fn advertise_alias_into_alts(
    current_canonical: Option<&RoomAliasId>,
    current_alts: &[OwnedRoomAliasId],
    new_alias: &RoomAliasId,
) -> Vec<OwnedRoomAliasId> {
    let new_str = new_alias.as_str();
    let is_canonical = current_canonical.is_some_and(|c| c.as_str() == new_str);
    let already_alt = current_alts.iter().any(|a| a.as_str() == new_str);
    let mut alts = current_alts.to_vec();
    if !is_canonical && !already_alt {
        alts.push(new_alias.to_owned());
    }
    alts
}

/// The next step of a sequenced alias operation after its directory write
/// (`create_alias` / `delete_alias`) returns.
///
/// A publish/remove is two server writes — the room-directory write and the
/// `m.room.canonical_alias` write — that must run in sequence, not in parallel:
/// the canonical write happens only if the directory write succeeded. This
/// prevents a partial failure from leaving inconsistent server state (e.g. an
/// alias advertised in `alt_aliases` but never registered in the directory).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequencedAliasStep {
    /// Directory write succeeded — proceed to the `m.room.canonical_alias` write.
    WriteCanonical,
    /// Directory write failed — abort; leave `m.room.canonical_alias` untouched.
    Abort,
}

/// Decide the next step after the directory write of a sequenced publish/remove.
pub fn next_step_after_directory_write(directory_ok: bool) -> SequencedAliasStep {
    if directory_ok {
        SequencedAliasStep::WriteCanonical
    } else {
        SequencedAliasStep::Abort
    }
}

/// Serializes alias mutations for the modal's room: at most one may be in
/// flight at a time. Overlapping mutations are the cross-operation race that
/// can resurrect an unbound alias — each write snapshots the *full*
/// canonical/alt state, so a late-completing write clobbers a newer one. This
/// gate keeps the edit controls disabled from submit until the operation fully
/// settles, so the next mutation always builds on reconciled state.
///
/// A successful write holds the gate until the authoritative refresh confirms
/// server state; a *failed* write releases immediately, since the modal has
/// already rolled back to the pre-edit snapshot (and the refresh may never
/// arrive, e.g. the client was torn down).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AliasWriteGate {
    /// No mutation in flight — edit controls are live and submits are allowed.
    #[default]
    Idle,
    /// A mutation was submitted; awaiting its server write result.
    AwaitingResult,
    /// The write succeeded; awaiting the authoritative `FetchRoomSettings` refresh.
    AwaitingRefresh,
}

impl AliasWriteGate {
    /// Whether a new mutation may be submitted (only when fully idle).
    pub fn can_submit(self) -> bool {
        matches!(self, AliasWriteGate::Idle)
    }

    /// Record a submitted mutation. Returns `false` (and does nothing) if a
    /// mutation is already in flight, so callers can reject the overlap.
    pub fn on_submit(&mut self) -> bool {
        if self.can_submit() {
            *self = AliasWriteGate::AwaitingResult;
            true
        } else {
            false
        }
    }

    /// Record the write result, keyed on whether the server was actually
    /// *attempted*:
    /// - `attempted == true` (a request was sent — success OR server-side
    ///   failure): hold the gate until this op's own reconciliation fetch lands,
    ///   because a `FetchRoomSettings` is in flight for it. Releasing now would
    ///   let a new op start while that fetch is outstanding — the stray-refresh
    ///   race codex flagged.
    /// - `attempted == false` (preflight failure, e.g. no client — nothing was
    ///   sent, no fetch spawned, state unchanged): release straight to `Idle`.
    ///
    /// Ignores stray results (not in `AwaitingResult`).
    pub fn on_result(&mut self, attempted: bool) {
        if matches!(self, AliasWriteGate::AwaitingResult) {
            *self = if attempted {
                AliasWriteGate::AwaitingRefresh
            } else {
                AliasWriteGate::Idle
            };
        }
    }

    /// Whether an incoming authoritative refresh should be applied to the modal.
    /// A refresh arriving while `AwaitingResult` is a stray fetch (this op's own
    /// fetch is only spawned after its result) — applying it would clobber the
    /// optimistic state mid-flight, so it is rejected. `Idle` accepts the
    /// open/initial fetch; `AwaitingRefresh` accepts this op's own reconcile.
    pub fn should_accept_refresh(self) -> bool {
        !matches!(self, AliasWriteGate::AwaitingResult)
    }

    /// The authoritative refresh landed — release only from `AwaitingRefresh`
    /// (this op's own reconciliation). Callers must gate the state overwrite on
    /// [`Self::should_accept_refresh`] and call this to advance the enum. Room
    /// switches reset to `Idle` directly in `show`, not via this method.
    pub fn on_refresh(&mut self) {
        if matches!(self, AliasWriteGate::AwaitingRefresh) {
            *self = AliasWriteGate::Idle;
        }
    }
}

/// Per-room stage of an in-flight alias write, tracked by [`PendingAliasWrites`].
/// Absence from the map means idle. Mirrors [`AliasWriteGate`] but at app level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAliasStage {
    /// Submitted; the server write result has not returned yet. A settings fetch
    /// arriving now is an unrelated open-fetch and must NOT clear the entry (the
    /// write outcome is still unknown).
    Submitted,
    /// The write completed server-side (result returned) and a reconcile fetch is
    /// expected. Any settings fetch now reflects that completed write, so it
    /// clears the entry.
    AwaitingReconcile,
}

/// App-level (per-room) registry of in-flight alias writes, kept *outside* the
/// singleton modal so a modal close/switch/reopen cannot silently re-enable
/// the edit controls while a write is still settling (P1-2). A room is
/// "pending" from the moment a mutation is submitted until that op reconciles.
///
/// Ownership: `app.rs` holds the single instance; `show` consults it to decide
/// whether to open a room locked. Transitions survive the modal being torn down
/// and reopened.
#[derive(Debug, Default, Clone)]
pub struct PendingAliasWrites {
    rooms: std::collections::HashMap<OwnedRoomId, PendingAliasStage>,
}

impl PendingAliasWrites {
    /// A mutation was submitted for `room_id` — mark it pending (`Submitted`).
    pub fn register(&mut self, room_id: OwnedRoomId) {
        self.rooms.insert(room_id, PendingAliasStage::Submitted);
    }

    /// The write result arrived. `attempted == false` (preflight failure) is
    /// terminal — clear immediately (no reconcile fetch will follow).
    /// `attempted == true` advances to `AwaitingReconcile`: the write completed
    /// server-side and a reconcile fetch is expected.
    pub fn on_result(&mut self, room_id: &RoomId, attempted: bool) {
        if attempted {
            if self.rooms.contains_key(room_id) {
                self.rooms.insert(room_id.to_owned(), PendingAliasStage::AwaitingReconcile);
            }
        } else {
            self.rooms.remove(room_id);
        }
    }

    /// A settings fetch for `room_id` landed. It clears the entry only from
    /// `AwaitingReconcile` (the write already completed server-side, so the fetch
    /// reflects it). While still `Submitted`, an open-fetch from a reopen must
    /// not clear the pending write — its outcome is not yet known.
    pub fn on_reconciled(&mut self, room_id: &RoomId) {
        if self.rooms.get(room_id) == Some(&PendingAliasStage::AwaitingReconcile) {
            self.rooms.remove(room_id);
        }
    }

    /// Whether `room_id` has an alias write still settling (open it locked).
    pub fn is_pending(&self, room_id: &RoomId) -> bool {
        self.rooms.contains_key(room_id)
    }

    /// The in-flight stage for `room_id`, if any — so `show` can open the modal
    /// in the matching locked state (reject open-fetches while `Submitted`,
    /// accept the reconcile while `AwaitingReconcile`).
    pub fn stage(&self, room_id: &RoomId) -> Option<PendingAliasStage> {
        self.rooms.get(room_id).copied()
    }
}

/// Decide whether a failed `create_alias` (directory publish) should be treated
/// as an idempotent success: only when the server rejected it as a conflict AND
/// the alias already resolves to *this* room. This makes a retry after a
/// step-2 (advertise) failure repair the divergence instead of dying on
/// "alias already in use" (`FetchRoomSettings` can't see directory mappings, so
/// the two-phase write is not otherwise self-healing).
pub fn publish_alias_treat_as_success(
    directory_conflict: bool,
    existing_maps_to_this_room: bool,
) -> bool {
    directory_conflict && existing_maps_to_this_room
}

/// Decide whether a failed `delete_alias` (directory unbind) should be treated
/// as an idempotent success: a "not found" means the mapping is already gone,
/// so a retry after a partial Remove can proceed to de-advertise instead of
/// dying on "alias not found".
pub fn remove_alias_treat_as_success(directory_not_found: bool) -> bool {
    directory_not_found
}

#[cfg(test)]
mod alias_logic_tests {
    use super::*;

    fn server() -> ruma::OwnedServerName {
        ruma::OwnedServerName::try_from("example.org").expect("valid server name")
    }

    fn alias(s: &str) -> OwnedRoomAliasId {
        OwnedRoomAliasId::try_from(s).expect("valid alias in test")
    }

    #[test]
    fn test_normalize_alias_accepts_full_alias() {
        let got = normalize_and_validate_alias("#general:example.org", &server()).unwrap();
        assert_eq!(got, alias("#general:example.org"));
    }

    #[test]
    fn test_normalize_alias_completes_bare_localpart() {
        let got = normalize_and_validate_alias("general", &server()).unwrap();
        assert_eq!(got, alias("#general:example.org"));
    }

    #[test]
    fn test_normalize_alias_rejects_invalid() {
        for bad in ["", "#:example.org", "#has space:example.org", "#general"] {
            assert!(
                normalize_and_validate_alias(bad, &server()).is_err(),
                "expected {bad:?} to be rejected",
            );
        }
    }

    #[test]
    fn test_reconcile_promote_alias_to_canonical() {
        let out = reconcile_canonical_alias(
            Some(&alias("#old:example.org")),
            &[alias("#new:example.org")],
            AliasOp::SetCanonical(alias("#new:example.org")),
        )
        .unwrap();
        assert_eq!(out.canonical, Some(alias("#new:example.org")));
        assert!(out.alt_aliases.contains(&alias("#old:example.org")));
        assert!(!out.alt_aliases.contains(&alias("#new:example.org")));
    }

    #[test]
    fn test_reconcile_rejects_unpublished_canonical() {
        let err = reconcile_canonical_alias(
            Some(&alias("#old:example.org")),
            &[],
            AliasOp::SetCanonical(alias("#ghost:example.org")),
        )
        .unwrap_err();
        assert_eq!(err, CanonicalReconcileError::NotPublished);
    }

    #[test]
    fn test_reconcile_remove_canonical_clears_it() {
        let out = reconcile_canonical_alias(
            Some(&alias("#main:example.org")),
            &[alias("#alt:example.org")],
            AliasOp::Remove(alias("#main:example.org")),
        )
        .unwrap();
        assert_eq!(out.canonical, None);
        assert!(out.alt_aliases.contains(&alias("#alt:example.org")));
    }

    #[test]
    fn test_reconcile_dedups_canonical_from_alts() {
        let out = reconcile_canonical_alias(
            Some(&alias("#old:example.org")),
            &[alias("#dup:example.org")],
            AliasOp::SetCanonical(alias("#dup:example.org")),
        )
        .unwrap();
        assert!(!out.alt_aliases.contains(&alias("#dup:example.org")));
    }

    #[test]
    fn test_advertise_alias_appends_new_alt() {
        let alts = advertise_alias_into_alts(
            Some(&alias("#main:example.org")),
            &[alias("#one:example.org")],
            &alias("#two:example.org"),
        );
        assert_eq!(
            alts,
            vec![alias("#one:example.org"), alias("#two:example.org")],
        );
    }

    #[test]
    fn test_advertise_alias_never_duplicates_canonical() {
        // Advertising the canonical alias must not push it into alt_aliases.
        let alts = advertise_alias_into_alts(
            Some(&alias("#main:example.org")),
            &[alias("#one:example.org")],
            &alias("#main:example.org"),
        );
        assert!(!alts.contains(&alias("#main:example.org")));
        assert_eq!(alts, vec![alias("#one:example.org")]);
    }

    #[test]
    fn test_advertise_alias_is_idempotent_for_existing_alt() {
        // Re-advertising an already-published alt does not create a duplicate.
        let alts = advertise_alias_into_alts(
            None,
            &[alias("#one:example.org")],
            &alias("#one:example.org"),
        );
        assert_eq!(alts, vec![alias("#one:example.org")]);
    }

    #[test]
    fn test_sequenced_op_writes_canonical_only_after_directory_success() {
        assert_eq!(
            next_step_after_directory_write(true),
            SequencedAliasStep::WriteCanonical,
        );
    }

    #[test]
    fn test_sequenced_op_aborts_when_directory_write_fails() {
        // A failed directory write must NOT trigger the canonical_alias write —
        // this is the invariant that stops partial/parallel-write divergence.
        assert_eq!(
            next_step_after_directory_write(false),
            SequencedAliasStep::Abort,
        );
    }

    #[test]
    fn test_alias_gate_blocks_overlapping_mutation() {
        let mut gate = AliasWriteGate::default();
        assert!(gate.can_submit());
        assert!(gate.on_submit()); // first submit accepted
        assert!(!gate.can_submit()); // controls now gated
        assert!(!gate.on_submit()); // overlapping submit rejected
        assert_eq!(gate, AliasWriteGate::AwaitingResult);
    }

    #[test]
    fn test_alias_gate_attempted_result_holds_until_refresh() {
        // Both success AND server-attempted failure hold the gate until this
        // op's own reconcile fetch lands (a fetch is in flight for it).
        for attempted in [true, true] {
            let mut gate = AliasWriteGate::default();
            gate.on_submit();
            gate.on_result(attempted); // attempted → awaiting its own refresh
            assert_eq!(gate, AliasWriteGate::AwaitingRefresh);
            assert!(!gate.can_submit());
            gate.on_refresh(); // this op's authoritative refresh releases it
            assert!(gate.can_submit());
        }
    }

    #[test]
    fn test_alias_gate_preflight_failure_releases_immediately() {
        // A preflight failure (attempted == false: nothing sent, no fetch
        // spawned, state unchanged) releases straight to Idle.
        let mut gate = AliasWriteGate::default();
        gate.on_submit();
        gate.on_result(false);
        assert!(gate.can_submit());
        assert_eq!(gate, AliasWriteGate::Idle);
    }

    #[test]
    fn test_alias_gate_ignores_stray_result() {
        let mut gate = AliasWriteGate::default();
        gate.on_result(true); // no submit in flight → ignored
        assert_eq!(gate, AliasWriteGate::Idle);
    }

    #[test]
    fn test_alias_gate_stray_refresh_while_awaiting_result_does_not_release() {
        // The op's own fetch is only spawned after its result, so a refresh
        // arriving during AwaitingResult is a stray from a prior op: it must
        // neither release the gate nor be accepted (would clobber optimism).
        let mut gate = AliasWriteGate::default();
        gate.on_submit();
        assert_eq!(gate, AliasWriteGate::AwaitingResult);
        assert!(!gate.should_accept_refresh()); // reject stray
        gate.on_refresh();
        assert_eq!(gate, AliasWriteGate::AwaitingResult);
        assert!(!gate.can_submit());
    }

    #[test]
    fn test_alias_gate_accepts_refresh_when_idle_or_awaiting_refresh() {
        assert!(AliasWriteGate::Idle.should_accept_refresh()); // open/initial fetch
        assert!(AliasWriteGate::AwaitingRefresh.should_accept_refresh()); // own reconcile
        assert!(!AliasWriteGate::AwaitingResult.should_accept_refresh()); // stray
    }

    // ── PendingAliasWrites (app-level per-room registry, survives reopen) ──

    fn room(s: &str) -> OwnedRoomId {
        OwnedRoomId::try_from(s).expect("valid room id in test")
    }

    #[test]
    fn test_pending_registry_survives_reopen() {
        // Submitting marks the room pending; that state must persist across any
        // number of reads (a modal close/reopen just re-reads is_pending).
        let mut reg = PendingAliasWrites::default();
        let r = room("!a:example.org");
        reg.register(r.clone());
        assert!(reg.is_pending(&r));
        assert!(reg.is_pending(&r)); // reopen consults it again → still pending
    }

    #[test]
    fn test_pending_registry_cleared_on_reconcile() {
        let mut reg = PendingAliasWrites::default();
        let r = room("!a:example.org");
        reg.register(r.clone());
        reg.on_result(&r, true); // attempted → still pending, awaiting reconcile
        assert!(reg.is_pending(&r));
        reg.on_reconciled(&r);
        assert!(!reg.is_pending(&r));
    }

    #[test]
    fn test_pending_registry_open_fetch_does_not_clear_submitted() {
        // A settings fetch (e.g. an open-fetch from a reopen) that lands BEFORE
        // the write result must not clear a still-Submitted write — the outcome
        // is unknown, so the room must stay locked.
        let mut reg = PendingAliasWrites::default();
        let r = room("!a:example.org");
        reg.register(r.clone()); // Submitted
        reg.on_reconciled(&r); // stray/open fetch while Submitted
        assert!(reg.is_pending(&r)); // still pending
        reg.on_result(&r, true); // now AwaitingReconcile
        reg.on_reconciled(&r); // the real reconcile clears it
        assert!(!reg.is_pending(&r));
    }

    #[test]
    fn test_pending_registry_preflight_failure_clears_immediately() {
        let mut reg = PendingAliasWrites::default();
        let r = room("!a:example.org");
        reg.register(r.clone());
        reg.on_result(&r, false); // preflight failure is terminal
        assert!(!reg.is_pending(&r));
    }

    #[test]
    fn test_pending_registry_is_per_room() {
        let mut reg = PendingAliasWrites::default();
        let a = room("!a:example.org");
        let b = room("!b:example.org");
        reg.register(a.clone());
        assert!(reg.is_pending(&a));
        assert!(!reg.is_pending(&b)); // other rooms unaffected
    }

    // ── idempotent two-phase repair decisions ──

    #[test]
    fn test_publish_treat_as_success_only_when_conflict_maps_here() {
        assert!(publish_alias_treat_as_success(true, true)); // conflict + maps here → repair
        assert!(!publish_alias_treat_as_success(true, false)); // conflict, maps elsewhere → real fail
        assert!(!publish_alias_treat_as_success(false, true)); // not a conflict → real fail
        assert!(!publish_alias_treat_as_success(false, false));
    }

    #[test]
    fn test_remove_treat_as_success_on_not_found() {
        assert!(remove_alias_treat_as_success(true)); // already gone → success (de-advertise)
        assert!(!remove_alias_treat_as_success(false)); // other error → real fail
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AliasRow — one published-alias row in the "Room Aliases" section.
//
// Each row shows the alias string plus, for users with manage permission, a
// "Set as main" button (hidden on the current canonical) and a "Remove" button.
// The row is a self-contained widget: it stores its own alias and emits an
// [`AliasRowAction`] carrying that alias, so the parent modal routes per-row
// clicks without tracking slot indices (mirrors `DeviceCard` in
// `settings/devices_settings.rs`). Its DSL lives in the shared `script_mod!`
// block below, alongside `RoomSettingsModal`.
// ─────────────────────────────────────────────────────────────────────────────

/// Per-row action emitted by an [`AliasRow`], carrying the row's alias value so
/// the parent modal can act without knowing which slot fired.
#[derive(Clone, Debug, Default)]
pub enum AliasRowAction {
    /// "Set as main" clicked — promote this alias to canonical.
    SetCanonical(OwnedRoomAliasId),
    /// "Remove" clicked — unpublish this alias / drop it from canonical+alts.
    Remove(OwnedRoomAliasId),
    #[default]
    None,
}

/// The data for one alias row, handed to an [`AliasRow`] PortalList item via
/// its draw scope's props. Carries everything the row needs to render and to
/// route its clicks.
#[derive(Clone, Debug)]
pub struct AliasRowProps {
    pub alias: OwnedRoomAliasId,
    /// Whether this is the room's canonical (main) alias.
    pub is_canonical: bool,
    /// Whether the edit controls (Set-as-main / Remove) are interactive.
    pub edit_enabled: bool,
    pub language: AppLanguage,
}

#[derive(Script, ScriptHook, Widget)]
pub struct AliasRow {
    #[deref] view: View,
    /// The alias this row currently represents (mirrored from props at draw).
    #[rust] alias: Option<OwnedRoomAliasId>,
    /// Whether this row is the room's canonical (main) alias.
    #[rust] is_canonical: bool,
    /// Whether this row's edit controls are interactive (mirrored from props).
    #[rust] edit_enabled: bool,
}

impl Widget for AliasRow {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        if let Event::Actions(actions) = event {
            // Ignore clicks unless this row's controls are interactive (they are
            // hidden otherwise; the guard is belt-and-suspenders against a queued
            // click while a write is in flight or permission is absent).
            if !self.edit_enabled {
                return;
            }
            // "Set as main" is a no-op on the current canonical (its button is
            // hidden anyway; the guard keeps it correct if that ever changes).
            if self.view.button(cx, ids!(alias_row_set_main_button)).clicked(actions)
                && !self.is_canonical
            {
                if let Some(alias) = self.alias.clone() {
                    cx.action(AliasRowAction::SetCanonical(alias));
                }
            }
            if self.view.button(cx, ids!(alias_row_remove_button)).clicked(actions) {
                if let Some(alias) = self.alias.clone() {
                    cx.action(AliasRowAction::Remove(alias));
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Populate from the item scope's props (set by the parent modal's draw
        // loop). Values are mirrored into `#[rust]` fields for `handle_event`.
        if let Some(props) = scope.props.get::<AliasRowProps>() {
            self.alias = Some(props.alias.clone());
            self.is_canonical = props.is_canonical;
            self.edit_enabled = props.edit_enabled;

            self.view.label(cx, ids!(alias_row_label)).set_text(cx, props.alias.as_str());
            // Canonical rows get a "Main" badge (reuses the localized label).
            self.view.view(cx, ids!(alias_row_main_badge)).set_visible(cx, props.is_canonical);
            if props.is_canonical {
                self.view.label(cx, ids!(alias_row_main_badge_label))
                    .set_text(cx, tr_key(props.language, "room_settings.aliases.canonical_label"));
            }
            self.view.button(cx, ids!(alias_row_set_main_button))
                .set_text(cx, tr_key(props.language, "room_settings.aliases.set_canonical_button"));
            self.view.button(cx, ids!(alias_row_remove_button))
                .set_text(cx, tr_key(props.language, "room_settings.aliases.remove_button"));
            // "Set as main" only when interactive and not already canonical.
            self.view.button(cx, ids!(alias_row_set_main_button))
                .set_visible(cx, props.edit_enabled && !props.is_canonical);
            self.view.button(cx, ids!(alias_row_remove_button))
                .set_visible(cx, props.edit_enabled);
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // One published-alias row (see the AliasRow Rust widget above).
    // PortalList item: a fixed-height row so the modal can size the list to fit
    // its content deterministically (see ALIAS_ROW_PX). Data comes from
    // `AliasRowProps` via the item scope at draw time.
    mod.widgets.AliasRow = #(AliasRow::register_widget(vm)) {
        width: Fill
        height: 40
        flow: Right
        align: Align{y: 0.5}
        margin: Inset{bottom: 6}
        spacing: 8

        alias_row_label := Label {
            width: Fill
            height: Fit
            draw_text +: {
                text_style: REGULAR_TEXT {font_size: 10.5}
                color: (RBX_FG_PRIMARY)
            }
            text: ""
        }

        alias_row_main_badge := RoundedView {
            visible: false
            width: Fit
            height: Fit
            align: Align{y: 0.5}
            padding: Inset{left: 8, right: 8, top: 2, bottom: 2}
            show_bg: true
            draw_bg +: {
                color: (RBX_ACCENT_SOFT)
                border_radius: (RBX_RADIUS_PILL)
            }
            alias_row_main_badge_label := Label {
                width: Fit
                height: Fit
                draw_text +: {
                    text_style: RBX_TEXT_BADGE {}
                    color: (RBX_ACCENT)
                }
                text: ""
            }
        }

        alias_row_set_main_button := RobrixNeutralIconButton {
            width: Fit
            height: (RBX_CONTROL_H_SM)
            padding: Inset{top: 6, bottom: 6, left: 10, right: 10}
            icon_walk: Walk{width: 0, height: 0}
            draw_bg +: { border_radius: (RBX_RADIUS_XS) }
            text: "Set as main"
        }

        alias_row_remove_button := RobrixNegativeIconButton {
            width: Fit
            height: (RBX_CONTROL_H_SM)
            padding: Inset{top: 6, bottom: 6, left: 10, right: 10}
            icon_walk: Walk{width: 0, height: 0}
            draw_bg +: { border_radius: (RBX_RADIUS_XS) }
            text: "Remove"
        }
    }

    mod.widgets.RoomSettingsModal = #(RoomSettingsModal::register_widget(vm)) {
        width: Fill { max: 680 }
        height: Fit
        margin: Inset{left: 12, right: 12}

        RoundedShadowView {
            width: Fill
            height: Fit
            flow: Down
            padding: Inset{top: 0, right: 0, bottom: 0, left: 0}
            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_SM)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
                shadow_color: (RBX_SHADOW_STRONG)
                shadow_radius: 10.0
                shadow_offset: vec2(0.0, 3.0)
            }

            // ── Title bar ────────────────────────────────────────────────
            title_bar := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}
                padding: Inset{left: 20, right: 12, top: 14, bottom: 14}
                spacing: 8

                title_label := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        text_style: TITLE_TEXT {font_size: 13}
                        color: (RBX_FG_PRIMARY)
                    }
                    text: "Room Settings"
                }

                close_button := RobrixNeutralIconButton {
                    width: 28
                    height: 28
                    padding: 4
                    draw_icon.svg: (ICON_CLOSE)
                    icon_walk: Walk{width: 14, height: 14}
                    text: ""
                }
            }

            // ── Separator ────────────────────────────────────────────────
            View {
                width: Fill
                height: 1
                show_bg: true
                draw_bg +: { color: (COLOR_SECONDARY) }
            }

            // ── Main area ────────────────────────────────────────────────
            main_area := View {
                width: Fill
                height: Fit
                flow: Right

                // Sidebar
                sidebar := View {
                    width: 130
                    height: Fit
                    flow: Down
                    padding: Inset{top: 12, left: 0, right: 0, bottom: 12}
                    show_bg: true
                    draw_bg +: { color: #F3F5F8 }

                    general_tab_button := RobrixNeutralIconButton {
                        width: Fill
                        height: 36
                        padding: Inset{left: 12, right: 8, top: 8, bottom: 8}
                        align: Align{x: 0.0, y: 0.5}
                        icon_walk: Walk{width: 0, height: 0}
                        draw_bg +: {
                            color: #E8EEF5
                            color_hover: #DDE6F0
                            color_down: #D0DBE8
                            border_radius: 0.0
                        }
                        draw_text +: {
                            color: #000
                            color_hover: #000
                            color_down: #000
                            text_style: REGULAR_TEXT {font_size: 11}
                        }
                        text: "General"
                    }
                }

                // Content area
                content_scroll := ScrollYView {
                    width: Fill
                    height: 520
                    flow: Down
                    spacing: 0
                    padding: Inset{left: 24, right: 24, top: 20, bottom: 20}

                    // ── General heading ──────────────────────────────
                    general_heading := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 16}
                        draw_text +: {
                            text_style: TITLE_TEXT {font_size: 13}
                            color: #000
                        }
                        text: "General"
                    }

                    // ── Form row (inputs + avatar) ───────────────────
                    form_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        spacing: 16

                        // Inputs column
                        inputs_col := View {
                            width: Fill
                            height: Fit
                            flow: Down
                            spacing: 6

                            room_name_label := Label {
                                width: Fill
                                height: Fit
                                margin: Inset{bottom: 2}
                                draw_text +: {
                                    text_style: REGULAR_TEXT {font_size: 10.5}
                                    color: #333
                                }
                                text: "Room Name"
                            }

                            room_name_input := RobrixTextInput {
                                width: Fill
                                height: 44
                                empty_text: "Room name"
                            }

                            room_topic_label := Label {
                                width: Fill
                                height: Fit
                                margin: Inset{top: 10, bottom: 2}
                                draw_text +: {
                                    text_style: REGULAR_TEXT {font_size: 10.5}
                                    color: #333
                                }
                                text: "Room Topic"
                            }

                            room_topic_input := RobrixTextInput {
                                width: Fill
                                height: 120
                                empty_text: "Room topic (optional)"
                                is_multiline: true
                            }

                            name_error_label := Label {
                                visible: false
                                width: Fill
                                height: Fit
                                margin: Inset{top: 2}
                                draw_text +: {
                                    text_style: REGULAR_TEXT {font_size: 10}
                                    color: (COLOR_FG_DANGER_RED)
                                }
                                text: ""
                            }

                            buttons_row := View {
                                width: Fill
                                height: Fit
                                flow: Right
                                align: Align{x: 1.0, y: 0.5}
                                margin: Inset{top: 12}
                                spacing: 10

                                cancel_button := RobrixNeutralIconButton {
                                    width: 90
                                    height: 32
                                    padding: 6
                                    icon_walk: Walk{width: 0, height: 0}
                                    draw_icon.svg: (ICON_FORBIDDEN)
                                    text: "Cancel"
                                }

                                save_button := RobrixIconButton {
                                    width: 90
                                    height: 32
                                    padding: 6
                                    icon_walk: Walk{width: 0, height: 0}
                                    draw_icon.svg: (ICON_CHECKMARK)
                                    text: "Save"
                                }
                            }
                        }

                        // Avatar column
                        avatar_col := View {
                            width: 80
                            height: Fit
                            flow: Down
                            align: Align{x: 0.5}
                            spacing: 6

                            room_avatar := Avatar {
                                width: 60
                                height: 60
                            }

                            pencil_button := RobrixNeutralIconButton {
                                width: 60
                                height: 24
                                padding: 4
                                align: Align{x: 0.5, y: 0.5}
                                draw_icon.svg: (ICON_EDIT)
                                icon_walk: Walk{width: 12, height: 12}
                                text: ""
                            }
                        }
                    }

                    // ── Section separator ────────────────────────────
                    View {
                        width: Fill
                        height: 1
                        margin: Inset{top: 20, bottom: 16}
                        show_bg: true
                        draw_bg +: { color: (COLOR_SECONDARY) }
                    }

                    // ── Advanced ────────────────────────────────────
                    advanced_heading := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 10}
                        draw_text +: {
                            text_style: RBX_TEXT_SECTION_TITLE {}
                            color: (RBX_FG_PRIMARY)
                        }
                        text: "Advanced"
                    }

                    room_id_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 4}
                        draw_text +: {
                            text_style: RBX_TEXT_BODY {}
                            color: (RBX_FG_SECONDARY)
                        }
                        text: "Room ID"
                    }

                    room_id_input := RobrixTextInput {
                        width: Fill
                        height: 36
                        is_read_only: true
                        empty_text: "!room:server"
                    }

                    // ── Section separator ────────────────────────────
                    View {
                        width: Fill
                        height: 1
                        margin: Inset{top: 20, bottom: 16}
                        show_bg: true
                        draw_bg +: { color: (COLOR_SECONDARY) }
                    }

                    // ── Room Addresses ───────────────────────────────
                    addresses_heading := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 10}
                        draw_text +: {
                            text_style: TITLE_TEXT {font_size: 12}
                            color: #000
                        }
                        text: "Room Addresses"
                    }

                    published_addresses_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 4}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 11}
                            color: #333
                        }
                        text: "Published Addresses"
                    }

                    published_desc := Label {
                        width: Fill
                        height: Fit
                        flow: Flow.Right{wrap: true}
                        margin: Inset{bottom: 8}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 10}
                            color: #666
                        }
                        text: "These are the addresses that are published on the room directory for others to find this room."
                    }

                    main_alias_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        align: Align{y: 0.5}
                        margin: Inset{bottom: 8}
                        spacing: 8

                        main_alias_label := Label {
                            width: Fill
                            height: Fit
                            draw_text +: {
                                text_style: REGULAR_TEXT {font_size: 10.5}
                                color: #444
                            }
                            text: "No main address set"
                        }
                    }

                    // ── Alias rows (canonical + alts) ────────────────
                    // A PortalList so EVERY alias gets a real, actionable row
                    // (Remove / Set-as-main) with no fixed cap — the modal drives
                    // it from `alias_entries` and sizes its height to fit the
                    // content (see `render_alias_section`), so short lists don't
                    // scroll and long ones (rare) scroll internally.
                    alias_list := PortalList {
                        width: Fill
                        height: 0
                        flow: Down
                        grab_key_focus: false
                        max_pull_down: 0.0
                        auto_tail: false
                        keep_invisible: false

                        alias_item := mod.widgets.AliasRow {}
                    }

                    publish_toggle_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        align: Align{y: 0.5}
                        margin: Inset{bottom: 8}
                        spacing: 8

                        publish_toggle := Toggle {
                            width: Fit
                            height: Fit
                            padding: Inset{top: 2, right: 4, bottom: 2, left: 2}
                            text: ""
                            active: false
                            draw_bg +: {
                                size: 18.0
                                color_active: (COLOR_ACTIVE_PRIMARY)
                                border_color_active: (COLOR_ACTIVE_PRIMARY)
                                mark_color_active: #fff
                            }
                        }

                        publish_toggle_label := Label {
                            width: Fill
                            height: Fit
                            flow: Flow.Right{wrap: true}
                            draw_text +: {
                                text_style: REGULAR_TEXT {font_size: 10}
                                color: #333
                            }
                            text: "Publish this room to the public in matrix.org's room directory?"
                        }
                    }

                    no_published_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 8}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 10}
                            color: #888
                        }
                        text: "No other published addresses yet, add one below"
                    }

                    // Hidden by default (P1-A): the add control only appears once
                    // the room's power-level fetch confirms manage permission,
                    // via `render_alias_section`. Never visible before that.
                    add_address_row := View {
                        visible: false
                        width: Fill
                        height: Fit
                        flow: Right
                        align: Align{y: 0.5}
                        spacing: 8
                        margin: Inset{bottom: 12}

                        add_address_input := RobrixTextInput {
                            width: Fill
                            height: 36
                            empty_text: "# e.g. my-room"
                        }

                        add_address_button := RobrixIconButton {
                            width: 60
                            height: 36
                            padding: 6
                            icon_walk: Walk{width: 0, height: 0}
                            text: "Add"
                        }
                    }

                    local_addresses_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 4}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 11}
                            color: #333
                        }
                        text: "Local Addresses"
                    }

                    local_desc := Label {
                        width: Fill
                        height: Fit
                        flow: Flow.Right{wrap: true}
                        margin: Inset{bottom: 8}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 10}
                            color: #666
                        }
                        text: "Set addresses for this room so users can find this room. As an admin, you can set local addresses for this room."
                    }

                    // ── Section separator ────────────────────────────
                    View {
                        width: Fill
                        height: 1
                        margin: Inset{top: 12, bottom: 16}
                        show_bg: true
                        draw_bg +: { color: (COLOR_SECONDARY) }
                    }

                    // ── Other / Moderation ───────────────────────────
                    other_heading := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 10}
                        draw_text +: {
                            text_style: TITLE_TEXT {font_size: 12}
                            color: #000
                        }
                        text: "Other"
                    }

                    moderation_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 6}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 11}
                            color: #333
                        }
                        text: "Moderation and safety"
                    }

                    show_media_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 2}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 10.5}
                            color: #333
                        }
                        text: "Show media in timeline"
                    }

                    show_media_desc := Label {
                        width: Fill
                        height: Fit
                        flow: Flow.Right{wrap: true}
                        margin: Inset{bottom: 6}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 10}
                            color: #666
                        }
                        text: "A hidden media can always be shown by tapping on it"
                    }

                    media_hide_radio := RadioButton {
                        width: Fit
                        height: Fit
                        align: Align{y: 0.5}
                        padding: Inset{top: 4, bottom: 4, left: 6, right: 4}
                        draw_text +: {
                            color: (MESSAGE_TEXT_COLOR)
                            color_hover: (MESSAGE_TEXT_COLOR)
                            color_focus: (MESSAGE_TEXT_COLOR)
                            color_active: (MESSAGE_TEXT_COLOR)
                            color_down: (MESSAGE_TEXT_COLOR)
                            color_disabled: (MESSAGE_TEXT_COLOR)
                            text_style: REGULAR_TEXT {font_size: 10.5}
                        }
                        draw_bg +: {
                            color: (COLOR_PRIMARY)
                            border_color: (COLOR_SECONDARY_DARKER)
                            border_color_active: (COLOR_ACTIVE_PRIMARY_DARKER)
                            mark_color: vec4(0.0, 0.0, 0.0, 0.0)
                            mark_color_active: (COLOR_ACTIVE_PRIMARY_DARKER)
                        }
                        text: "Always hide"
                    }

                    media_show_radio := RadioButton {
                        width: Fit
                        height: Fit
                        align: Align{y: 0.5}
                        padding: Inset{top: 4, bottom: 4, left: 6, right: 4}
                        draw_text +: {
                            color: (MESSAGE_TEXT_COLOR)
                            color_hover: (MESSAGE_TEXT_COLOR)
                            color_focus: (MESSAGE_TEXT_COLOR)
                            color_active: (MESSAGE_TEXT_COLOR)
                            color_down: (MESSAGE_TEXT_COLOR)
                            color_disabled: (MESSAGE_TEXT_COLOR)
                            text_style: REGULAR_TEXT {font_size: 10.5}
                        }
                        draw_bg +: {
                            color: (COLOR_PRIMARY)
                            border_color: (COLOR_SECONDARY_DARKER)
                            border_color_active: (COLOR_ACTIVE_PRIMARY_DARKER)
                            mark_color: vec4(0.0, 0.0, 0.0, 0.0)
                            mark_color_active: (COLOR_ACTIVE_PRIMARY_DARKER)
                        }
                        text: "Always show"
                    }

                    // ── Section separator ────────────────────────────
                    View {
                        width: Fill
                        height: 1
                        margin: Inset{top: 16, bottom: 16}
                        show_bg: true
                        draw_bg +: { color: (COLOR_SECONDARY) }
                    }

                    // ── Leave Room ───────────────────────────────────
                    leave_room_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 10}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 11}
                            color: #333
                        }
                        text: "Leave room"
                    }

                    leave_button := RobrixNegativeIconButton {
                        width: Fit
                        height: 32
                        padding: Inset{left: 12, right: 12, top: 6, bottom: 6}
                        icon_walk: Walk{width: 0, height: 0}
                        text: "Leave room"
                    }
                }
            }
        }
    }
}

/// Actions emitted by the `RoomSettingsModal`.
#[derive(Clone, Debug, Default)]
pub enum RoomSettingsAction {
    /// Open the modal for the given room.
    Open { room_id: OwnedRoomId },
    /// Close the modal (user clicked close/X).
    Close,
    /// Save room name and topic.
    Save { room_id: OwnedRoomId, room_name: String, room_topic: String },
    /// Cancel edits without saving.
    Cancel,
    /// Toggle publishing this room to the directory.
    SetDirectoryPublish { room_id: OwnedRoomId, enabled: bool },
    /// Publish a new (already-validated) alias and advertise it into
    /// `m.room.canonical_alias`'s `alt_aliases`. `canonical`/`alt_aliases` are
    /// the reconciled state to write (alt_aliases already includes `alias`).
    PublishAlias {
        room_id: OwnedRoomId,
        alias: OwnedRoomAliasId,
        canonical: Option<OwnedRoomAliasId>,
        alt_aliases: Vec<OwnedRoomAliasId>,
    },
    /// Promote an existing alias to canonical. `canonical`/`alt_aliases` are the
    /// reconciled target of the `m.room.canonical_alias` state event.
    SetCanonicalAlias {
        room_id: OwnedRoomId,
        canonical: Option<OwnedRoomAliasId>,
        alt_aliases: Vec<OwnedRoomAliasId>,
    },
    /// Remove an alias: unbind it from the room directory and drop it from
    /// `m.room.canonical_alias` (reconciled `canonical`/`alt_aliases`).
    RemoveAlias {
        room_id: OwnedRoomId,
        alias: OwnedRoomAliasId,
        canonical: Option<OwnedRoomAliasId>,
        alt_aliases: Vec<OwnedRoomAliasId>,
    },
    /// Change media visibility preference.
    SetMediaVisibility { room_id: OwnedRoomId, always_show: bool },
    /// Leave the room.
    LeaveRoom { room_id: OwnedRoomId },
    /// Upload a new room avatar from the given local file path.
    UploadRoomAvatar { room_id: OwnedRoomId, avatar_path: PathBuf },
    #[default]
    None,
}

/// Per-row height (px) used to size the alias `PortalList` to fit its content.
/// Slightly over the DSL row advance (height 40 + margin 6 = 46) so a fitted
/// list has a hair of slack rather than clipping the last row into a scroll.
const ALIAS_ROW_PX: f64 = 48.0;
/// Above this many aliases the list stops growing and scrolls internally (every
/// row stays a real, actionable AliasRow — nothing is stranded).
const ALIAS_LIST_MAX_ROWS: usize = 10;

#[derive(Script, ScriptHook, Widget)]
pub struct RoomSettingsModal {
    #[deref] view: View,
    #[source] source: ScriptObjectRef,
    #[rust] room_id: Option<OwnedRoomId>,
    #[rust] original_name: String,
    #[rust] original_topic: String,
    #[rust] always_show_media: bool,
    /// Language used to (re-)render the alias section after optimistic updates.
    #[rust] language: AppLanguage,
    /// Current canonical alias (authoritative, plus optimistic edits).
    #[rust] current_canonical: Option<OwnedRoomAliasId>,
    /// Current alt aliases (authoritative, plus optimistic edits).
    #[rust] current_alts: Vec<OwnedRoomAliasId>,
    /// Whether the user may manage aliases (gates the per-row edit controls).
    #[rust] can_manage_aliases: bool,
    /// Snapshot of `(canonical, alts)` taken before an in-flight optimistic
    /// write, restored if the server reports failure.
    #[rust] alias_snapshot: Option<(Option<OwnedRoomAliasId>, Vec<OwnedRoomAliasId>)>,
    /// Serializes alias mutations: at most one write in flight per room. Gates
    /// the edit controls from submit until the operation fully settles.
    #[rust] alias_gate: AliasWriteGate,
    /// The alias rows to render, in display order (canonical first). Drives the
    /// alias `PortalList` in `draw_walk`; rebuilt by `render_alias_section`.
    #[rust] alias_entries: Vec<AliasRowProps>,
}

impl Widget for RoomSettingsModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Drive the alias PortalList: every entry gets a real actionable row, so
        // no alias is stranded regardless of count (mirrors the DevicesScreen
        // PortalList pattern).
        while let Some(widget) = self.view.draw_walk(cx, scope, walk).step() {
            let plist = widget.as_portal_list();
            let Some(mut list) = plist.borrow_mut() else {
                continue;
            };
            let n = self.alias_entries.len();
            list.set_item_range(cx, 0, n);
            while let Some(index) = list.next_visible_item(cx) {
                if index < n {
                    let props = self.alias_entries[index].clone();
                    let item = list.item(cx, index, id!(alias_item));
                    let mut item_scope = Scope::with_props(&props);
                    item.draw_all(cx, &mut item_scope);
                }
            }
        }
        DrawStep::done()
    }
}

impl WidgetMatchEvent for RoomSettingsModal {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        // Close button
        if self.view.button(cx, ids!(close_button)).clicked(actions) {
            cx.action(RoomSettingsAction::Close);
            return;
        }

        // Cancel button
        if self.view.button(cx, ids!(cancel_button)).clicked(actions) {
            cx.action(RoomSettingsAction::Cancel);
            return;
        }

        // Save button – validate name not empty
        if self.view.button(cx, ids!(save_button)).clicked(actions) {
            let name = self.view.text_input(cx, ids!(room_name_input)).text();
            let topic = self.view.text_input(cx, ids!(room_topic_input)).text();
            if name.trim().is_empty() {
                self.view.label(cx, ids!(name_error_label))
                    .set_text(cx, "Room name cannot be empty");
                self.view.label(cx, ids!(name_error_label)).set_visible(cx, true);
                self.view.redraw(cx);
            } else {
                self.view.label(cx, ids!(name_error_label)).set_visible(cx, false);
                if let Some(room_id) = self.room_id.clone() {
                    cx.action(RoomSettingsAction::Save {
                        room_id,
                        room_name: name.trim().to_string(),
                        room_topic: topic.trim().to_string(),
                    });
                }
            }
            return;
        }

        // Publish toggle
        let publish_toggle = self.view.check_box(cx, ids!(publish_toggle));
        if let Some(enabled) = publish_toggle.changed(actions) {
            if let Some(room_id) = self.room_id.clone() {
                cx.action(RoomSettingsAction::SetDirectoryPublish { room_id, enabled });
            }
        }

        // Add address button — validate, optimistically advertise, then publish.
        if self.view.button(cx, ids!(add_address_button)).clicked(actions) {
            let raw = self.view.text_input(cx, ids!(add_address_input)).text();
            let raw = raw.trim().to_string();
            if !raw.is_empty() {
                if let Some(room_id) = self.room_id.clone() {
                    self.add_alias(cx, room_id, &raw);
                }
            }
        }

        // Per-row actions from AliasRow widgets (Set as main / Remove).
        for action in actions {
            match action.downcast_ref::<AliasRowAction>() {
                Some(AliasRowAction::SetCanonical(alias)) => {
                    self.set_canonical_alias(cx, alias.clone());
                }
                Some(AliasRowAction::Remove(alias)) => {
                    self.remove_alias(cx, alias.clone());
                }
                _ => {}
            }
        }

        // Server outcome of an alias write: commit on success, roll back the
        // optimistic UI and surface the server error on failure.
        for action in actions {
            if let Some(result) = action.downcast_ref::<crate::sliding_sync::RoomAliasWriteResultAction>() {
                if self.room_id.as_deref() == Some(result.room_id.as_ref()) {
                    self.apply_write_result(cx, result);
                }
            }
        }

        // Media radio buttons
        let radios = self.view.radio_button_set(cx, ids_array!(media_hide_radio, media_show_radio));
        if let Some(selected) = radios.selected(cx, actions) {
            let always_show = selected == 1;
            self.always_show_media = always_show;
            if let Some(room_id) = self.room_id.clone() {
                cx.action(RoomSettingsAction::SetMediaVisibility { room_id, always_show });
            }
        }

        // Leave button
        if self.view.button(cx, ids!(leave_button)).clicked(actions) {
            if let Some(room_id) = self.room_id.clone() {
                cx.action(RoomSettingsAction::LeaveRoom { room_id });
            }
        }

        // Pencil / edit avatar button — open native file picker
        if self.view.button(cx, ids!(pencil_button)).clicked(actions) {
            #[cfg(any(target_os = "macos", target_os = "windows", all(target_os = "linux", not(target_env = "ohos"))))]
            if let Some(room_id) = self.room_id.clone() {
                use rfd::FileDialog;
                if let Some(path) = FileDialog::new()
                    .add_filter("Image", &["png", "jpg", "jpeg"])
                    .pick_file()
                {
                    cx.action(RoomSettingsAction::UploadRoomAvatar { room_id, avatar_path: path });
                }
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows", all(target_os = "linux", not(target_env = "ohos")))))]
            if let Some(_room_id) = self.room_id.clone() {
                use crate::shared::popup_list::{PopupKind, enqueue_popup_notification};
                enqueue_popup_notification(
                    "Avatar upload not supported on this platform",
                    PopupKind::Warning,
                    Some(4.0),
                );
            }
        }
    }
}

impl RoomSettingsModal {
    /// Populate the modal with room data and prepare for display. `alias_stage`
    /// comes from the app-level [`PendingAliasWrites`] registry: `Some(_)` means
    /// this room has an alias write still settling (submitted in a prior modal
    /// session), so the section opens locked in the matching gate state until
    /// that op's reconcile fetch lands.
    pub fn show(
        &mut self,
        cx: &mut Cx,
        room_id: OwnedRoomId,
        room_name: &str,
        room_topic: &str,
        canonical_alias: Option<&str>,
        alias_stage: Option<PendingAliasStage>,
    ) {
        let room_id_text = room_id.as_str().to_string();
        self.room_id = Some(room_id);
        self.original_name = room_name.to_string();
        self.original_topic = room_topic.to_string();
        self.always_show_media = false;

        // Update title
        self.view.label(cx, ids!(title_label))
            .set_text(cx, &format!("Room Settings – {room_name}"));

        // Populate inputs
        self.view.text_input(cx, ids!(room_name_input))
            .set_text(cx, room_name);
        self.view.text_input(cx, ids!(room_topic_input))
            .set_text(cx, room_topic);
        self.view.text_input(cx, ids!(room_id_input))
            .set_text(cx, &room_id_text);
        self.view.text_input(cx, ids!(room_id_input))
            .set_is_read_only(cx, true);

        // P1-A: reset the alias section to a read-only "loading" state for the
        // NEW room. Without this, the singleton modal keeps the previous room's
        // aliases, permission, snapshot, and rendered rows — so a click landing
        // before this room's fetch returns would mutate the wrong room with a
        // stale alias. Edit controls stay disabled (can_manage=false) until the
        // matching `FetchRoomSettings` refresh arrives via `apply_alias_settings`.
        self.current_canonical = canonical_alias
            .and_then(|s| OwnedRoomAliasId::try_from(s).ok());
        self.current_alts = Vec::new();
        self.can_manage_aliases = false;
        self.alias_snapshot = None;
        // P1-2: if a write for this room is still settling (registry), open locked
        // in the matching gate state so a close→reopen can't re-enable controls
        // mid-flight. `Submitted` → AwaitingResult (reject an unrelated open-fetch
        // until the write result returns); `AwaitingReconcile` → AwaitingRefresh
        // (the op's reconcile fetch unlocks it). No pending write → Idle.
        self.alias_gate = match alias_stage {
            Some(PendingAliasStage::Submitted) => AliasWriteGate::AwaitingResult,
            Some(PendingAliasStage::AwaitingReconcile) => AliasWriteGate::AwaitingRefresh,
            None => AliasWriteGate::Idle,
        };
        self.render_alias_section(cx);

        // Avatar fallback text (first char of name)
        let avatar_char = room_name.chars().next().unwrap_or('?').to_string();
        self.view.avatar(cx, ids!(room_avatar))
            .show_text(cx, None, None, &avatar_char);

        // Reset error label
        self.view.label(cx, ids!(name_error_label)).set_visible(cx, false);
        self.view.label(cx, ids!(name_error_label)).set_text(cx, "");

        self.view.redraw(cx);
    }

    /// Update the avatar widget with freshly uploaded image bytes.
    pub fn apply_avatar(&mut self, cx: &mut Cx, image_data: &[u8]) {
        let _ = self.view.avatar(cx, ids!(room_avatar))
            .show_image(cx, None, |cx, img| load_png_or_jpg(&img, cx, image_data));
        self.view.redraw(cx);
    }

    /// Whether `room_id` matches the room this modal is currently showing.
    /// Used to drop stale/out-of-order async responses for a previous room
    /// (P1-B), so they never overwrite the current room's modal.
    fn is_current_room(&self, room_id: &RoomId) -> bool {
        self.room_id.as_deref() == Some(room_id)
    }

    /// Apply fetched settings (topic, is_public) that arrived asynchronously.
    /// Ignored if `room_id` is not the room currently shown (stale response).
    pub fn apply_fetched_settings(
        &mut self,
        cx: &mut Cx,
        room_id: &RoomId,
        topic: Option<String>,
        is_public: bool,
    ) {
        if !self.is_current_room(room_id) {
            return;
        }
        if let Some(t) = topic {
            self.original_topic = t.clone();
            self.view.text_input(cx, ids!(room_topic_input)).set_text(cx, &t);
        }
        // Update publish toggle state (active == is_public)
        // Toggle widget: set via script_apply_eval on check_box
        let _ = is_public; // reflected by the toggle's current state
        self.view.redraw(cx);
    }

    /// Apply the room's alias data (canonical + alt aliases) and permission
    /// gating to the "Room Aliases" section. Labels use the localized strings
    /// from `resources/i18n/**` so the section follows the app language.
    ///
    /// This is the authoritative refresh: it overwrites any optimistic state
    /// and clears the rollback snapshot. When `can_manage` is false the user
    /// lacks the power level to send the `m.room.canonical_alias` state event,
    /// so the add-address control is hidden and a read-only hint is shown.
    ///
    /// Ignored if `room_id` is not the room currently shown (P1-B): out-of-order
    /// fetches for a previous room must never overwrite the current modal.
    pub fn apply_alias_settings(
        &mut self,
        cx: &mut Cx,
        room_id: &RoomId,
        language: AppLanguage,
        canonical_alias: Option<OwnedRoomAliasId>,
        alt_aliases: Vec<OwnedRoomAliasId>,
        can_manage: bool,
    ) {
        if !self.is_current_room(room_id) {
            return;
        }
        // P1-1: route the gate decision BEFORE touching state. A refresh arriving
        // while a mutation's result is still pending (AwaitingResult) is a stray
        // fetch from a prior op — reject it so it can't clobber optimistic state
        // or the rollback snapshot. Otherwise accept and advance the gate.
        if !self.alias_gate.should_accept_refresh() {
            return;
        }
        self.alias_gate.on_refresh();
        // Store authoritative state; a fresh fetch supersedes optimism.
        self.language = language;
        self.current_canonical = canonical_alias;
        self.current_alts = alt_aliases;
        self.can_manage_aliases = can_manage;
        self.alias_snapshot = None;
        self.render_alias_section(cx);
    }

    /// Release a waiting alias write's gate when its reconcile fetch could not
    /// produce data (no client / room unavailable). Unlike `apply_alias_settings`
    /// this does NOT overwrite state — it keeps the current (optimistic) aliases
    /// and just re-enables the controls, so the gate can never strand disabled.
    /// Guarded like a refresh: a stray release during `AwaitingResult` is ignored.
    pub fn release_alias_lock(&mut self, cx: &mut Cx, room_id: &RoomId) {
        if !self.is_current_room(room_id) || !self.alias_gate.should_accept_refresh() {
            return;
        }
        self.alias_gate.on_refresh();
        self.render_alias_section(cx);
    }

    /// Render the whole alias section (labels, per-row list, gating) from the
    /// modal's current stored state. Called on authoritative refresh and after
    /// every optimistic edit.
    fn render_alias_section(&mut self, cx: &mut Cx) {
        let language = self.language;
        let can_manage = self.can_manage_aliases;
        // Edit controls are interactive only when the user has permission AND no
        // alias write is in flight (P1-C: one mutation per room). `can_manage`
        // still drives the read-only-vs-editable hint text, so a manager who is
        // mid-write doesn't briefly see the "no permission" message.
        let edit_enabled = can_manage && self.alias_gate.can_submit();

        // Localized section labels.
        self.view.label(cx, ids!(addresses_heading))
            .set_text(cx, tr_key(language, "room_settings.aliases.section_title"));
        self.view.label(cx, ids!(published_addresses_label))
            .set_text(cx, tr_key(language, "room_settings.aliases.canonical_label"));

        // The canonical alias is now shown as a badged, actionable row in the
        // list below, so hide the old separate summary line to avoid showing it
        // twice (review finding P2-cosmetic).
        self.view.view(cx, ids!(main_alias_row)).set_visible(cx, false);

        // Build the ordered row list for the PortalList: canonical first
        // (flagged), then alts. Every entry becomes a real, actionable row — no
        // cap, so nothing is stranded (P2).
        let mut entries: Vec<AliasRowProps> = Vec::new();
        if let Some(c) = self.current_canonical.clone() {
            entries.push(AliasRowProps { alias: c, is_canonical: true, edit_enabled, language });
        }
        for a in self.current_alts.clone() {
            entries.push(AliasRowProps { alias: a, is_canonical: false, edit_enabled, language });
        }
        let row_count = entries.len();
        self.alias_entries = entries;

        // Size the list to fit its content (up to a cap, beyond which it scrolls
        // internally). A fitted list never scroll-captures inside the modal.
        let visible_rows = row_count.min(ALIAS_LIST_MAX_ROWS);
        let list_height = visible_rows as f64 * ALIAS_ROW_PX;
        let mut alias_list = self.view.portal_list(cx, ids!(alias_list));
        script_apply_eval!(cx, alias_list, {
            height: #(list_height)
        });

        // Empty-state hint when there are no aliases at all.
        let no_aliases = row_count == 0;
        self.view.label(cx, ids!(no_published_label)).set_visible(cx, no_aliases);
        if no_aliases {
            self.view.label(cx, ids!(no_published_label))
                .set_text(cx, tr_key(language, "room_settings.aliases.none_published"));
        }

        // Localized add control.
        self.view.text_input(cx, ids!(add_address_input))
            .set_empty_text(cx, tr_key(language, "room_settings.aliases.add_placeholder").to_string());
        self.view.button(cx, ids!(add_address_button))
            .set_text(cx, tr_key(language, "room_settings.aliases.add_button"));

        // Permission gating: only users who can send `m.room.canonical_alias`
        // see the add control; it is also hidden while a write is in flight.
        self.view.view(cx, ids!(add_address_row)).set_visible(cx, edit_enabled);
        if can_manage {
            self.view.label(cx, ids!(local_desc))
                .set_text(cx, tr_key(language, "room_settings.aliases.alt_label"));
        } else {
            self.view.label(cx, ids!(local_desc))
                .set_text(cx, tr_key(language, "room_settings.aliases.readonly_hint"));
        }

        self.view.redraw(cx);
    }

    /// Validate a raw address string and, on success, optimistically advertise
    /// it and emit [`RoomSettingsAction::PublishAlias`].
    fn add_alias(&mut self, cx: &mut Cx, room_id: OwnedRoomId, raw: &str) {
        use crate::shared::popup_list::{PopupKind, enqueue_popup_notification};

        // P1-C: reject if a mutation is already in flight (controls are hidden
        // while gated, but a queued click could still reach here).
        if !self.alias_gate.can_submit() {
            return;
        }

        let Some(server_name) = crate::sliding_sync::current_user_id()
            .map(|u| u.server_name().to_owned())
        else {
            enqueue_popup_notification(
                tr_key(self.language, "room_settings.aliases.sign_in_required").to_string(),
                PopupKind::Error,
                Some(4.0),
            );
            return;
        };

        let valid_alias = match normalize_and_validate_alias(raw, &server_name) {
            Ok(alias) => alias,
            Err(_) => {
                enqueue_popup_notification(
                    tr_key(self.language, "room_settings.aliases.invalid_format").to_string(),
                    PopupKind::Error,
                    Some(4.0),
                );
                return;
            }
        };

        // Optimistically advertise the new alias into alt_aliases.
        let new_alts = advertise_alias_into_alts(
            self.current_canonical.as_deref(),
            &self.current_alts,
            &valid_alias,
        );
        self.snapshot_alias_state();
        self.current_alts = new_alts.clone();
        self.alias_gate.on_submit();
        self.render_alias_section(cx);
        self.view.text_input(cx, ids!(add_address_input)).set_text(cx, "");

        cx.action(RoomSettingsAction::PublishAlias {
            room_id,
            alias: valid_alias,
            canonical: self.current_canonical.clone(),
            alt_aliases: new_alts,
        });
    }

    /// Promote `alias` to canonical: reconcile, optimistically update, emit.
    fn set_canonical_alias(&mut self, cx: &mut Cx, alias: OwnedRoomAliasId) {
        use crate::shared::popup_list::{PopupKind, enqueue_popup_notification};
        let Some(room_id) = self.room_id.clone() else { return };
        if !self.alias_gate.can_submit() {
            return;
        }

        match reconcile_canonical_alias(
            self.current_canonical.as_deref(),
            &self.current_alts,
            AliasOp::SetCanonical(alias),
        ) {
            Ok(state) => {
                self.snapshot_alias_state();
                self.current_canonical = state.canonical.clone();
                self.current_alts = state.alt_aliases.clone();
                self.alias_gate.on_submit();
                self.render_alias_section(cx);
                cx.action(RoomSettingsAction::SetCanonicalAlias {
                    room_id,
                    canonical: state.canonical,
                    alt_aliases: state.alt_aliases,
                });
            }
            Err(CanonicalReconcileError::NotPublished) => {
                enqueue_popup_notification(
                    tr_key(self.language, "room_settings.aliases.publish_failed").to_string(),
                    PopupKind::Error,
                    Some(4.0),
                );
            }
        }
    }

    /// Remove `alias`: reconcile out of canonical/alts, optimistically update, emit.
    fn remove_alias(&mut self, cx: &mut Cx, alias: OwnedRoomAliasId) {
        let Some(room_id) = self.room_id.clone() else { return };
        if !self.alias_gate.can_submit() {
            return;
        }
        // Defensive: only act on an alias that belongs to the room currently
        // shown, so a stale per-row click that somehow survives a room switch
        // can't unbind a foreign alias from the directory. (Set-as-main is
        // already covered by `reconcile`'s `NotPublished`.)
        let known = self.current_canonical.as_deref().is_some_and(|c| c.as_str() == alias.as_str())
            || self.current_alts.iter().any(|a| a.as_str() == alias.as_str());
        if !known {
            return;
        }

        // Remove never fails (see `reconcile_canonical_alias`).
        if let Ok(state) = reconcile_canonical_alias(
            self.current_canonical.as_deref(),
            &self.current_alts,
            AliasOp::Remove(alias.clone()),
        ) {
            self.snapshot_alias_state();
            self.current_canonical = state.canonical.clone();
            self.current_alts = state.alt_aliases.clone();
            self.alias_gate.on_submit();
            self.render_alias_section(cx);
            cx.action(RoomSettingsAction::RemoveAlias {
                room_id,
                alias,
                canonical: state.canonical,
                alt_aliases: state.alt_aliases,
            });
        }
    }

    /// Snapshot the current alias state before an optimistic write, so it can
    /// be restored if the server reports failure. Captured just before each
    /// user-initiated edit as its pre-edit baseline. A single publish fans out
    /// into two writes (directory + canonical_alias) that share this one
    /// baseline, so a failure from either rolls back to the same pre-edit state.
    ///
    /// This is a single overwritable slot: if a user starts a second edit before
    /// the first write's result returns, a late failure would roll back to the
    /// wrong baseline. That transient case is self-healing — every write result
    /// triggers a `FetchRoomSettings` in `app.rs`, whose authoritative
    /// `apply_alias_settings` refresh overwrites the optimistic state and clears
    /// this snapshot regardless of which write failed.
    fn snapshot_alias_state(&mut self) {
        self.alias_snapshot =
            Some((self.current_canonical.clone(), self.current_alts.clone()));
    }

    /// React to a server outcome for an alias write. On failure, roll back the
    /// optimistic state to the pre-write snapshot; on success, commit it. The
    /// user-facing error toast is raised by `app.rs` so it fires even when this
    /// modal has already been closed.
    fn apply_write_result(
        &mut self,
        cx: &mut Cx,
        result: &crate::sliding_sync::RoomAliasWriteResultAction,
    ) {
        // Advance the in-flight gate keyed on whether the server was attempted.
        // Attempted (success OR server-side failure): hold until this op's own
        // reconcile fetch lands. Preflight failure: release now (no fetch coming).
        self.alias_gate.on_result(result.attempted);
        if result.error.is_some() {
            // Roll back optimistic UI to the pre-write baseline. For an attempted
            // failure the gate stays held (AwaitingRefresh) so controls remain
            // locked until this op's reconcile fetch; for a preflight failure the
            // gate is now Idle so the re-render re-enables the controls.
            if let Some((canonical, alts)) = self.alias_snapshot.clone() {
                self.current_canonical = canonical;
                self.current_alts = alts;
            }
            self.render_alias_section(cx);
        }
        // On success we leave the optimistic state in place with the gate still
        // held (AwaitingRefresh); this op's own `FetchRoomSettings` refresh
        // reconciles it with authoritative server state and releases the gate.
    }
}

impl RoomSettingsModalRef {
    /// Populate the modal with room data and prepare for display.
    pub fn show_settings(
        &self,
        cx: &mut Cx,
        room_id: OwnedRoomId,
        room_name: &str,
        room_topic: &str,
        canonical_alias: Option<&str>,
        alias_stage: Option<PendingAliasStage>,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show(cx, room_id, room_name, room_topic, canonical_alias, alias_stage);
    }

    /// Apply asynchronously-fetched settings (topic, is_public). Dropped if the
    /// response is for a room other than the one currently shown (P1-B).
    pub fn apply_fetched_settings(
        &self,
        cx: &mut Cx,
        room_id: &RoomId,
        topic: Option<String>,
        is_public: bool,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.apply_fetched_settings(cx, room_id, topic, is_public);
    }

    /// Apply fetched alias data (canonical + alt aliases) and permission gating.
    /// Dropped if the response is for a room other than the one shown (P1-B).
    pub fn apply_alias_settings(
        &self,
        cx: &mut Cx,
        room_id: &RoomId,
        language: AppLanguage,
        canonical_alias: Option<OwnedRoomAliasId>,
        alt_aliases: Vec<OwnedRoomAliasId>,
        can_manage: bool,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.apply_alias_settings(cx, room_id, language, canonical_alias, alt_aliases, can_manage);
    }

    /// Release a stranded alias gate when its reconcile fetch was unavailable.
    pub fn release_alias_lock(&self, cx: &mut Cx, room_id: &RoomId) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.release_alias_lock(cx, room_id);
    }

    /// Update the avatar widget after a successful upload.
    pub fn apply_avatar(&self, cx: &mut Cx, image_data: &[u8]) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.apply_avatar(cx, image_data);
    }
}

#[cfg(test)]
mod tests {
    const SOURCE: &str = include_str!("room_settings_modal.rs");

    #[test]
    fn advanced_section_declares_read_only_room_id_input() {
        assert!(SOURCE.contains(concat!("advanced_", "heading := Label")));
        assert!(SOURCE.contains(concat!("text: \"", "Advanced", "\"")));
        assert!(SOURCE.contains(concat!("room_id_", "label := Label")));
        assert!(SOURCE.contains(concat!("text: \"", "Room ID", "\"")));
        assert!(SOURCE.contains(concat!("room_id_", "input := RobrixTextInput")));
        assert!(SOURCE.contains(concat!("is_read_", "only: true")));
        assert!(SOURCE.contains(concat!("empty_text: \"", "!room:server", "\"")));
    }

    #[test]
    fn show_populates_room_id_input_from_room_id() {
        assert!(SOURCE.contains(concat!("let room_id_", "text = room_id.as_str().to_string();")));
        assert!(SOURCE.contains(concat!("self.room_id = Some(room_id", ");")));
        assert!(SOURCE.contains(concat!("ids!(room_id_", "input))")));
        assert!(SOURCE.contains(concat!(".set_text(cx, &room_id_", "text);")));
    }
}
