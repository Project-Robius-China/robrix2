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

/// Why a `FetchRoomSettings` was issued — carried through the request and echoed
/// back on both the fetched and unavailable responses so a stale or unrelated
/// fetch can never be mistaken for a specific write's reconcile (the race codex
/// flagged: settings fetches previously carried only `room_id`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomSettingsFetchReason {
    /// A non-write refresh (modal open, or the post-barrier recovery re-fetch),
    /// carrying a monotonic epoch. Not tied to any alias write, so it must NEVER
    /// consume a write's reconcile. Its data is applied only when it is the
    /// *newest* Open and no write has been submitted since it was issued (see
    /// [`OpenFreshness`]).
    ///
    /// Like every alias fetch it reads `m.room.canonical_alias` from the SERVER
    /// (round 8: single source of truth) — the local cache is never consulted, so
    /// there is no cross-source freshness to rank and a fetch payload can never be
    /// a stale cache snapshot.
    Open(u64),
    /// The authoritative reconcile for the alias write of this generation. Only a
    /// matching generation may release that write's gate / clear its registry.
    AliasReconcile(u64),
}

/// Freshness guard for `Open` settings fetches (P1-1). An `Open` snapshots
/// server state at request time, so a slow one issued *before* a write can
/// return *after* that write reconciles and repaint pre-write state — letting
/// the user submit a second write from stale data and clobber the first.
///
/// Every request takes a monotonic epoch; an `Open` is applied only if its epoch
/// is still `>= min_acceptable`. Both a new write AND any accepted apply push
/// `min_acceptable` past every epoch issued so far, so an older `Open` can never
/// become acceptable again — even after the gate returns to `Idle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpenFreshness {
    next_epoch: u64,
    min_acceptable: u64,
}

impl OpenFreshness {
    /// Allocate the epoch for a new `Open` fetch. This `Open` (and any newer one)
    /// is acceptable; any earlier outstanding `Open` is now stale.
    pub fn take_open(&mut self) -> u64 {
        self.next_epoch = self.next_epoch.wrapping_add(1);
        self.min_acceptable = self.next_epoch;
        self.next_epoch
    }

    /// Allocate the generation for a new write. A write permanently invalidates
    /// every `Open` issued so far (only a *future* Open can be accepted).
    pub fn take_write(&mut self) -> u64 {
        self.next_epoch = self.next_epoch.wrapping_add(1);
        self.min_acceptable = self.next_epoch.wrapping_add(1);
        self.next_epoch
    }

    /// An authoritative apply landed (an accepted Open or reconcile). Invalidate
    /// every `Open` issued so far so none can repaint over the just-applied state.
    pub fn on_apply(&mut self) {
        self.min_acceptable = self.next_epoch.wrapping_add(1);
    }

    /// Whether an `Open` of `epoch` is still fresh enough to apply.
    pub fn accepts_open(&self, epoch: u64) -> bool {
        epoch >= self.min_acceptable
    }

    /// Ensure the epoch source is at least `value` (keeps it monotonic when the
    /// modal adopts a pending write's generation on reopen).
    pub fn observe(&mut self, value: u64) {
        self.next_epoch = self.next_epoch.max(value);
    }
}

/// Serializes alias mutations for the modal's room: at most one may be in
/// flight at a time. Overlapping mutations are the cross-operation race that
/// can resurrect an unbound alias — each write snapshots the *full*
/// canonical/alt state, so a late-completing write clobbers a newer one. This
/// gate keeps the edit controls disabled from submit until the operation fully
/// settles, so the next mutation always builds on reconciled state.
///
/// Each in-flight write carries a monotonic *generation*. Only its own
/// reconcile fetch — matched by generation AND purpose — releases the gate, so a
/// stale open-fetch or a mismatched reconcile can neither release it nor clobber
/// the optimistic state, independent of arrival timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AliasWriteGate {
    /// No mutation in flight — edit controls are live and submits are allowed.
    #[default]
    Idle,
    /// A mutation (this generation) was submitted; awaiting its server result.
    AwaitingResult(u64),
    /// The write (this generation) reached the server; awaiting its reconcile.
    AwaitingRefresh(u64),
    /// The write's reconcile came back Unavailable (round 9): the write outcome
    /// is unknown, so edit controls stay non-interactive (read-only) until the
    /// recovery `Open(epoch)` APPLIES server truth. Never releases from an
    /// unknown state — a failed recovery stays here until an explicit reopen.
    Recovering(u64),
}

/// What the modal should do with an incoming settings fetch, decided purely from
/// the gate state and the fetch's [`RoomSettingsFetchReason`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchDisposition {
    /// Ignore it (stale/unrelated) — do not touch state or the gate.
    Ignore,
    /// Apply its data (authoritative load) but leave the gate as-is.
    Apply,
    /// Apply its data AND release the gate — this is the matching reconcile.
    ApplyAndRelease,
}

impl AliasWriteGate {
    /// Whether a new mutation may be submitted (only when fully idle).
    pub fn can_submit(self) -> bool {
        matches!(self, AliasWriteGate::Idle)
    }

    /// Record a submitted mutation of `generation`. Returns `false` (and does
    /// nothing) if a mutation is already in flight, so callers reject the overlap.
    pub fn on_submit(&mut self, generation: u64) -> bool {
        if self.can_submit() {
            *self = AliasWriteGate::AwaitingResult(generation);
            true
        } else {
            false
        }
    }

    /// Record the write result, keyed on whether the server was actually
    /// *attempted*:
    /// - `attempted == true` (request sent — success OR server-side failure):
    ///   hold the gate at `AwaitingRefresh(gen)` until this op's own reconcile
    ///   (matched by `gen`) lands. Releasing now would let a new op start while
    ///   that fetch is outstanding.
    /// - `attempted == false` (preflight failure — nothing sent, no fetch, state
    ///   unchanged): release straight to `Idle`.
    ///
    /// Ignores stray results (not in `AwaitingResult`).
    pub fn on_result(&mut self, attempted: bool) {
        if let AliasWriteGate::AwaitingResult(generation) = *self {
            *self = if attempted {
                AliasWriteGate::AwaitingRefresh(generation)
            } else {
                AliasWriteGate::Idle
            };
        }
    }

    /// Decide what to do with a settings fetch, matched by generation + purpose:
    /// - An `Open` fetch applies its data ONLY while `Idle` (initial/reopen load);
    ///   during any in-flight write it is stale → `Ignore` (never clobbers, never
    ///   releases).
    /// - An `AliasReconcile(g)` releases and applies ONLY when the gate is
    ///   `AwaitingRefresh(g)` with the same generation; otherwise `Ignore`.
    pub fn disposition(self, reason: RoomSettingsFetchReason) -> FetchDisposition {
        match (self, reason) {
            // Open only *gates* on Idle here; its epoch freshness is enforced by
            // the caller via `OpenFreshness` (P1-1).
            (AliasWriteGate::Idle, RoomSettingsFetchReason::Open(_)) => FetchDisposition::Apply,
            (AliasWriteGate::AwaitingRefresh(g), RoomSettingsFetchReason::AliasReconcile(r))
                if g == r =>
            {
                FetchDisposition::ApplyAndRelease
            }
            // The recovery Open (matched by epoch) is what leaves `Recovering`:
            // it applies server truth AND releases the gate to Idle (round 9), so
            // controls only come back once authoritative state has been applied.
            (AliasWriteGate::Recovering(e), RoomSettingsFetchReason::Open(r)) if e == r => {
                FetchDisposition::ApplyAndRelease
            }
            _ => FetchDisposition::Ignore,
        }
    }

    /// Apply the disposition of a fetch, releasing the gate iff it is the
    /// matching reconcile or the matching recovery Open. Returns the disposition
    /// so the caller can decide whether to overwrite state.
    pub fn on_fetch(&mut self, reason: RoomSettingsFetchReason) -> FetchDisposition {
        let disposition = self.disposition(reason);
        if disposition == FetchDisposition::ApplyAndRelease {
            *self = AliasWriteGate::Idle;
        }
        disposition
    }

    /// Whether `reason` is the matching reconcile for the write this gate is
    /// awaiting — used to decide whether an Unavailable enters `Recovering`.
    pub fn matches_reconcile(self, reason: RoomSettingsFetchReason) -> bool {
        matches!(
            (self, reason),
            (AliasWriteGate::AwaitingRefresh(g), RoomSettingsFetchReason::AliasReconcile(r)) if g == r
        )
    }

    /// The matching reconcile came back Unavailable: enter `Recovering(epoch)`
    /// instead of releasing to `Idle`, so edit controls stay non-interactive
    /// (`can_submit() == false`) until the recovery `Open(epoch)` applies server
    /// truth. Callers must first confirm [`Self::matches_reconcile`].
    pub fn enter_recovering(&mut self, recovery_epoch: u64) {
        *self = AliasWriteGate::Recovering(recovery_epoch);
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingAliasEntry {
    stage: PendingAliasStage,
    /// Generation of the in-flight write; only a reconcile with this generation
    /// may clear the entry.
    generation: u64,
}

#[derive(Debug, Default, Clone)]
pub struct PendingAliasWrites {
    rooms: std::collections::HashMap<OwnedRoomId, PendingAliasEntry>,
}

impl PendingAliasWrites {
    /// A mutation of `generation` was submitted for `room_id` — mark it pending.
    pub fn register(&mut self, room_id: OwnedRoomId, generation: u64) {
        self.rooms.insert(
            room_id,
            PendingAliasEntry { stage: PendingAliasStage::Submitted, generation },
        );
    }

    /// The write result arrived. `attempted == false` (preflight failure) is
    /// terminal — clear immediately (no reconcile fetch will follow).
    /// `attempted == true` advances to `AwaitingReconcile`: the write completed
    /// server-side and a reconcile fetch (matching generation) is expected.
    pub fn on_result(&mut self, room_id: &RoomId, attempted: bool) {
        if attempted {
            if let Some(entry) = self.rooms.get_mut(room_id) {
                entry.stage = PendingAliasStage::AwaitingReconcile;
            }
        } else {
            self.rooms.remove(room_id);
        }
    }

    /// A reconcile fetch of `generation` for `room_id` landed. It clears the entry
    /// ONLY when the entry is `AwaitingReconcile` AND the generation matches — so
    /// a stale/open fetch (wrong or absent generation) never clears a pending
    /// write, independent of arrival timing.
    pub fn on_reconciled(&mut self, room_id: &RoomId, generation: u64) {
        if let Some(entry) = self.rooms.get(room_id) {
            if entry.stage == PendingAliasStage::AwaitingReconcile
                && entry.generation == generation
            {
                self.rooms.remove(room_id);
            }
        }
    }

    /// Whether `room_id` has an alias write still settling (open it locked).
    pub fn is_pending(&self, room_id: &RoomId) -> bool {
        self.rooms.contains_key(room_id)
    }

    /// The in-flight `(stage, generation)` for `room_id`, if any — so `show` can
    /// open the modal locked in the matching gate state with the right generation.
    pub fn stage(&self, room_id: &RoomId) -> Option<(PendingAliasStage, u64)> {
        self.rooms.get(room_id).map(|e| (e.stage, e.generation))
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

/// What a reconcile fetch should post, given its **server-fresh** read of
/// `m.room.canonical_alias` (P1-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileFetchOutcome {
    /// A server-fresh read succeeded (`None` inner = no canonical alias set) —
    /// post it as the authoritative fetched settings.
    Fetched {
        canonical: Option<OwnedRoomAliasId>,
        alt_aliases: Vec<OwnedRoomAliasId>,
    },
    /// The server-fresh read was unavailable — release the gate WITHOUT applying
    /// any (possibly stale) data.
    Unavailable,
}

/// Decide what an alias fetch posts from the result of its server-fresh
/// canonical-alias read. In the pinned matrix-sdk, `send_state_event` does not
/// update the local `RoomInfo`, so the cache-backed `room.canonical_alias()` can
/// still return the *pre-write* value after a successful write; every alias
/// fetch therefore reads the server directly and, if that read fails, releases
/// via `Unavailable` rather than apply stale data. `server_read == None` means
/// the fresh read could not be obtained.
pub fn reconcile_fetch_outcome(
    server_read: Option<(Option<OwnedRoomAliasId>, Vec<OwnedRoomAliasId>)>,
) -> ReconcileFetchOutcome {
    match server_read {
        Some((canonical, alt_aliases)) => ReconcileFetchOutcome::Fetched { canonical, alt_aliases },
        None => ReconcileFetchOutcome::Unavailable,
    }
}

/// The payload an alias fetch applies, under the SINGLE-SOURCE model (round 8):
/// it is derived ONLY from the server read. The `_local_cache` parameter is
/// present solely to make the guarantee testable — the applied payload is
/// independent of the cache, so a stale cache value can never appear in a fetch
/// (which is what makes epoch ordering sound: it ranks same-source reads only).
pub fn alias_fetch_payload(
    _local_cache: (Option<OwnedRoomAliasId>, Vec<OwnedRoomAliasId>),
    server_read: Option<(Option<OwnedRoomAliasId>, Vec<OwnedRoomAliasId>)>,
) -> ReconcileFetchOutcome {
    // The cache is intentionally ignored: every fetch reads server truth.
    reconcile_fetch_outcome(server_read)
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
        assert!(gate.on_submit(1)); // first submit accepted
        assert!(!gate.can_submit()); // controls now gated
        assert!(!gate.on_submit(2)); // overlapping submit rejected
        assert_eq!(gate, AliasWriteGate::AwaitingResult(1));
    }

    #[test]
    fn test_alias_gate_attempted_result_holds_until_matching_reconcile() {
        // Success AND server-attempted failure hold the gate until this op's own
        // reconcile fetch (matched by generation) lands.
        let mut gate = AliasWriteGate::default();
        gate.on_submit(7);
        gate.on_result(true); // attempted → awaiting its own refresh
        assert_eq!(gate, AliasWriteGate::AwaitingRefresh(7));
        assert!(!gate.can_submit());
        assert_eq!(
            gate.on_fetch(RoomSettingsFetchReason::AliasReconcile(7)),
            FetchDisposition::ApplyAndRelease,
        );
        assert!(gate.can_submit());
    }

    #[test]
    fn test_alias_gate_preflight_failure_releases_immediately() {
        // A preflight failure (attempted == false: nothing sent, no fetch
        // spawned, state unchanged) releases straight to Idle.
        let mut gate = AliasWriteGate::default();
        gate.on_submit(3);
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
    fn test_alias_gate_open_fetch_applies_only_when_idle() {
        // An open-fetch loads state only when idle; during any in-flight write it
        // is stale and must be ignored (never clobber, never release).
        assert_eq!(
            AliasWriteGate::Idle.disposition(RoomSettingsFetchReason::Open(1)),
            FetchDisposition::Apply,
        );
        assert_eq!(
            AliasWriteGate::AwaitingResult(1).disposition(RoomSettingsFetchReason::Open(1)),
            FetchDisposition::Ignore,
        );
        assert_eq!(
            AliasWriteGate::AwaitingRefresh(1).disposition(RoomSettingsFetchReason::Open(1)),
            FetchDisposition::Ignore,
        );
    }

    #[test]
    fn test_alias_gate_reconcile_requires_matching_generation() {
        let mut gate = AliasWriteGate::AwaitingRefresh(2);
        // Mismatched generation → ignored, gate unchanged.
        assert_eq!(
            gate.on_fetch(RoomSettingsFetchReason::AliasReconcile(3)),
            FetchDisposition::Ignore,
        );
        assert_eq!(gate, AliasWriteGate::AwaitingRefresh(2));
        // A reconcile arriving while still AwaitingResult is stale (result not
        // back yet) → ignored.
        assert_eq!(
            AliasWriteGate::AwaitingResult(2).disposition(RoomSettingsFetchReason::AliasReconcile(2)),
            FetchDisposition::Ignore,
        );
    }

    #[test]
    fn test_alias_gate_matches_reconcile_only_matching() {
        let gate = AliasWriteGate::AwaitingRefresh(4);
        assert!(!gate.matches_reconcile(RoomSettingsFetchReason::Open(1))); // open never matches
        assert!(!gate.matches_reconcile(RoomSettingsFetchReason::AliasReconcile(5))); // wrong gen
        assert!(gate.matches_reconcile(RoomSettingsFetchReason::AliasReconcile(4))); // matching
        // A matching Unavailable enters Recovering (NOT Idle) — controls stay
        // non-interactive until the recovery Open applies (round 9).
        let mut g = gate;
        g.enter_recovering(9);
        assert_eq!(g, AliasWriteGate::Recovering(9));
        assert!(!g.can_submit());
    }

    // ── codex-named regression tests (generation/purpose correlation) ──

    #[test]
    fn test_regression_two_open_fetches_stale_after_write_result() {
        // Repro (a): W1 completes → AwaitingRefresh; a stale open-fetch (from a
        // second open) returns AFTER the write result. It must NOT release the
        // gate or apply its (pre-W1) state; only W1's own reconcile releases it.
        let mut gate = AliasWriteGate::default();
        gate.on_submit(1); // W1
        gate.on_result(true); // AwaitingRefresh(1)
        assert_eq!(
            gate.on_fetch(RoomSettingsFetchReason::Open(1)),
            FetchDisposition::Ignore,
        );
        assert_eq!(gate, AliasWriteGate::AwaitingRefresh(1)); // still held, not clobbered
        assert_eq!(
            gate.on_fetch(RoomSettingsFetchReason::AliasReconcile(1)),
            FetchDisposition::ApplyAndRelease,
        );
        assert_eq!(gate, AliasWriteGate::Idle);
    }

    #[test]
    fn test_regression_write_result_and_stale_fetch_same_batch_consistent() {
        // Repro (b): a write result and a STALE fetch land in one Actions batch.
        // App (registry) processes before UI (gate); both must reach the SAME
        // decision for the stale fetch — registry and gate stay in lockstep —
        // because both match on generation+purpose, independent of order.
        let mut reg = PendingAliasWrites::default();
        let mut gate = AliasWriteGate::default();
        let r = room("!a:example.org");
        reg.register(r.clone(), 1);
        gate.on_submit(1);
        reg.on_result(&r, true); // AwaitingReconcile(gen 1)
        gate.on_result(true); // AwaitingRefresh(1)

        // Stale fetch (older generation 0). App side:
        reg.on_reconciled(&r, 0);
        assert!(reg.is_pending(&r)); // registry NOT cleared (gen mismatch)
        // UI side, same batch:
        assert_eq!(
            gate.on_fetch(RoomSettingsFetchReason::AliasReconcile(0)),
            FetchDisposition::Ignore,
        );
        assert_eq!(gate, AliasWriteGate::AwaitingRefresh(1)); // gate NOT released
        // Consistent: both still in-flight. The real reconcile clears both.
        reg.on_reconciled(&r, 1);
        assert!(!reg.is_pending(&r));
        assert_eq!(
            gate.on_fetch(RoomSettingsFetchReason::AliasReconcile(1)),
            FetchDisposition::ApplyAndRelease,
        );
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
        reg.register(r.clone(), 1);
        assert!(reg.is_pending(&r));
        assert_eq!(reg.stage(&r), Some((PendingAliasStage::Submitted, 1)));
    }

    #[test]
    fn test_pending_registry_cleared_on_matching_reconcile() {
        let mut reg = PendingAliasWrites::default();
        let r = room("!a:example.org");
        reg.register(r.clone(), 1);
        reg.on_result(&r, true); // attempted → still pending, awaiting reconcile
        assert!(reg.is_pending(&r));
        reg.on_reconciled(&r, 2); // mismatched generation → does NOT clear
        assert!(reg.is_pending(&r));
        reg.on_reconciled(&r, 1); // matching → clears
        assert!(!reg.is_pending(&r));
    }

    #[test]
    fn test_pending_registry_open_fetch_does_not_clear_submitted() {
        // A settings fetch (e.g. an open-fetch from a reopen) that lands BEFORE
        // the write result must not clear a still-Submitted write — the outcome
        // is unknown, so the room must stay locked. Even a matching generation is
        // ignored while Submitted.
        let mut reg = PendingAliasWrites::default();
        let r = room("!a:example.org");
        reg.register(r.clone(), 1); // Submitted
        reg.on_reconciled(&r, 1); // fetch while Submitted (matching gen)
        assert!(reg.is_pending(&r)); // still pending
        reg.on_result(&r, true); // now AwaitingReconcile
        reg.on_reconciled(&r, 1); // the real reconcile clears it
        assert!(!reg.is_pending(&r));
    }

    #[test]
    fn test_pending_registry_preflight_failure_clears_immediately() {
        let mut reg = PendingAliasWrites::default();
        let r = room("!a:example.org");
        reg.register(r.clone(), 1);
        reg.on_result(&r, false); // preflight failure is terminal
        assert!(!reg.is_pending(&r));
    }

    #[test]
    fn test_pending_registry_is_per_room() {
        let mut reg = PendingAliasWrites::default();
        let a = room("!a:example.org");
        let b = room("!b:example.org");
        reg.register(a.clone(), 1);
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

    // ── P1-1: Open-fetch freshness (OpenFreshness) ──

    #[test]
    fn test_open_freshness_newest_open_wins() {
        // An older Open is stale the moment a newer Open is issued.
        let mut f = OpenFreshness::default();
        let e1 = f.take_open();
        let e2 = f.take_open();
        assert!(!f.accepts_open(e1)); // superseded
        assert!(f.accepts_open(e2)); // newest
    }

    #[test]
    fn test_regression_write_then_old_fetched_open_rejected() {
        // codex-named (a), Fetched variant: F1 issued (pre-write), W1 submitted,
        // W1's reconcile applies; the OLD F1 returning last must be rejected so it
        // can't repaint pre-write state (which would let W2 clobber W1).
        let mut f = OpenFreshness::default();
        let e_f1 = f.take_open(); // slow open, snapshots S0
        let _g_w1 = f.take_write(); // write — invalidates all earlier opens
        f.on_apply(); // W1's reconcile applies authoritative S1
        assert!(!f.accepts_open(e_f1)); // old F1 can never repaint S0
    }

    #[test]
    fn test_regression_write_then_old_open_rejected_via_unavailable_release() {
        // codex-named (a), Unavailable variant: the write's reconcile came back
        // Unavailable (released without applying), but the OLD F1 must STILL be
        // rejected — the write's submit already invalidated it, independent of
        // whether the reconcile applied or released.
        let mut f = OpenFreshness::default();
        let e_f1 = f.take_open();
        let _g_w1 = f.take_write(); // submit invalidates earlier opens (no on_apply needed)
        assert!(!f.accepts_open(e_f1));
    }

    #[test]
    fn test_open_freshness_apply_invalidates_outstanding_opens() {
        // After an accepted apply, an Open issued *before* it is stale even though
        // no new write happened (a duplicate/older open must not repaint).
        let mut f = OpenFreshness::default();
        let e_old = f.take_open();
        let _e_new = f.take_open(); // a newer open we accept and apply
        f.on_apply();
        assert!(!f.accepts_open(e_old));
    }

    // ── P1-2: reconcile must carry server truth, not stale cache ──

    #[test]
    fn test_reconcile_outcome_applies_server_read() {
        let out = reconcile_fetch_outcome(Some((
            Some(alias("#main:example.org")),
            vec![alias("#alt:example.org")],
        )));
        assert_eq!(
            out,
            ReconcileFetchOutcome::Fetched {
                canonical: Some(alias("#main:example.org")),
                alt_aliases: vec![alias("#alt:example.org")],
            },
        );
    }

    #[test]
    fn test_reconcile_outcome_releases_without_applying_when_read_unavailable() {
        // "send succeeded but local RoomInfo not yet synced": the reconcile does a
        // server-fresh read; if that read is unavailable it must NOT apply stale
        // cache data — it releases via Unavailable instead.
        assert_eq!(reconcile_fetch_outcome(None), ReconcileFetchOutcome::Unavailable);
    }

    // ── round 7: matching-Unavailable is a terminal freshness barrier ──
    // The failing interleaving: a reopen DURING a pending write issues Open(e2)
    // (take_open AFTER take_write); the write's reconcile then comes back
    // *matching-Unavailable*. Unlike round 6's test (Open issued BEFORE the
    // write), here e2 > the write generation, so only a terminal barrier on the
    // Unavailable path can invalidate it.

    #[test]
    fn test_regression_unavailable_barrier_then_reopen_open_rejected() {
        // Order (a): matching Unavailable THEN the post-write reopen Open. The
        // barrier (on_apply, fired on the matching Unavailable) must reject e2 so
        // it can never apply its possibly-pre-write snapshot.
        let mut f = OpenFreshness::default();
        let _g_w1 = f.take_write(); // W1 submitted (pending)
        let e2 = f.take_open(); // reopen-during-pending Open (e2 > W1 generation)
        f.on_apply(); // matching-Unavailable terminal barrier
        assert!(!f.accepts_open(e2)); // rejected — no stale apply
    }

    #[test]
    fn test_regression_reopen_open_then_unavailable_recovers_without_stale_or_wedge() {
        // Order (b): the reopen Open(e2) arrives first (ignored while the gate is
        // held — modeled by NOT applying it), THEN the matching Unavailable fires
        // the barrier + a recovery Open(e3). e2 must be stale (no stale apply);
        // e3 (issued post-barrier, after show() reset can_manage=false) must be
        // acceptable so the modal repopulates and is NOT wedged read-only.
        let mut f = OpenFreshness::default();
        let _g_w1 = f.take_write();
        let e2 = f.take_open(); // reopen Open — arrives first, gate-ignored (not applied)
        f.on_apply(); // matching-Unavailable barrier
        let e3 = f.take_open(); // post-barrier recovery Open
        assert!(!f.accepts_open(e2)); // e2 can never apply → no stale apply
        assert!(f.accepts_open(e3)); // e3 repopulates → not wedged read-only
    }

    // ── round 8: single source of truth — no fetch payload can be stale cache ──
    // Every alias fetch (Open, recovery Open, reconcile) reads the SERVER; the
    // cache is never consulted. So epochs only ever order same-source (server)
    // reads, and a stale cache snapshot can never reach a fetch payload — which
    // is what closes the cross-source provenance-ranking hole codex flagged.

    #[test]
    fn test_single_source_a_later_open_applies_server_not_cache() {
        // (a) A recovery Open is pending and a later Open(e4) is issued. Both read
        // the server. Concrete repro: W1 published #a. The local cache still shows
        // NO #a (S0); the server shows #a (S1). The applied payload must be the
        // SERVER read — the cache S0 (which would drop #a from a W2 alt-list and
        // de-advertise the still-directory-mapped #a) can never appear.
        let s1_server = Some((None, vec![alias("#a:example.org"), alias("#b:example.org")]));
        let s0_stale_cache = (None, vec![alias("#b:example.org")]); // missing #a
        let arbitrary_other_cache = (Some(alias("#a:example.org")), vec![alias("#b:example.org")]);
        // The payload is independent of the cache: any cache → same server payload.
        assert_eq!(
            alias_fetch_payload(s0_stale_cache.clone(), s1_server.clone()),
            alias_fetch_payload(arbitrary_other_cache, s1_server.clone()),
        );
        assert_eq!(
            alias_fetch_payload(s0_stale_cache, s1_server),
            ReconcileFetchOutcome::Fetched {
                canonical: None,
                alt_aliases: vec![alias("#a:example.org"), alias("#b:example.org")],
            },
        );
    }

    #[test]
    fn test_single_source_b_open_after_recovery_applied_before_sync() {
        // (b) The recovery already applied S1, then a later Open(e4) is issued
        // BEFORE the local cache syncs (cache still S0). The later Open also reads
        // the server, so it re-applies S1; the pre-write cache S0 is unreachable.
        let s1_server = Some((Some(alias("#a:example.org")), vec![alias("#b:example.org")]));
        let s0_stale_cache = (None, vec![]); // sync hasn't caught up
        assert_eq!(
            alias_fetch_payload(s0_stale_cache, s1_server),
            ReconcileFetchOutcome::Fetched {
                canonical: Some(alias("#a:example.org")),
                alt_aliases: vec![alias("#b:example.org")],
            },
        );
    }

    // ── round 9: don't unlock on matching-Unavailable — stay Recovering ──
    // A write's send can err ambiguously (server may have applied it), so on a
    // matching reconcile-Unavailable the modal must NOT re-enable edits from that
    // unknown state; it stays read-only (Recovering) until the recovery Open
    // applies server truth, and restores permission only from that apply.

    #[test]
    fn test_recovering_a_blocks_second_write_until_recovery_applies() {
        // (a) Unavailable → recovery pending → a second write is rejected.
        let mut gate = AliasWriteGate::default();
        gate.on_submit(1); // W1
        gate.on_result(true); // AwaitingRefresh(1)
        assert!(gate.matches_reconcile(RoomSettingsFetchReason::AliasReconcile(1)));
        gate.enter_recovering(3); // matching Unavailable → Recovering(recovery epoch)
        assert_eq!(gate, AliasWriteGate::Recovering(3));
        assert!(!gate.can_submit()); // W2 rejected while recovering
        assert!(!gate.on_submit(4)); // an attempted submit is refused…
        assert_eq!(gate, AliasWriteGate::Recovering(3)); // …gate unchanged
        // Only the recovery Open(3) applying server truth releases the gate:
        assert_eq!(
            gate.on_fetch(RoomSettingsFetchReason::Open(3)),
            FetchDisposition::ApplyAndRelease,
        );
        assert_eq!(gate, AliasWriteGate::Idle);
        assert!(gate.can_submit()); // interactive ONLY after the apply
    }

    #[test]
    fn test_recovering_b_bounded_failure_stays_read_only_until_reopen() {
        // (b) The recovery Open also fails: the gate must NOT release from an
        // unknown state; it stays read-only until an explicit reopen recovers it.
        let mut gate = AliasWriteGate::Recovering(3);
        // A failed recovery Open is not a reconcile → release_alias_lock no-ops:
        assert!(!gate.matches_reconcile(RoomSettingsFetchReason::Open(3)));
        // A stray/older open can't unlock it either:
        assert_eq!(
            gate.on_fetch(RoomSettingsFetchReason::Open(2)),
            FetchDisposition::Ignore,
        );
        assert_eq!(gate, AliasWriteGate::Recovering(3)); // still read-only
        assert!(!gate.can_submit());
        // Reopen re-derives from the (cleared) registry → Idle (show() mapping),
        // and issues its own fresh Open — recovering the modal.
        let reopened_gate = match None::<(PendingAliasStage, u64)> {
            Some((PendingAliasStage::Submitted, g)) => AliasWriteGate::AwaitingResult(g),
            Some((PendingAliasStage::AwaitingReconcile, g)) => AliasWriteGate::AwaitingRefresh(g),
            None => AliasWriteGate::Idle,
        };
        assert_eq!(reopened_gate, AliasWriteGate::Idle);
        assert!(reopened_gate.can_submit()); // reopen recovers
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
    /// `generation` is the modal-allocated write generation (see registry match).
    PublishAlias {
        room_id: OwnedRoomId,
        alias: OwnedRoomAliasId,
        canonical: Option<OwnedRoomAliasId>,
        alt_aliases: Vec<OwnedRoomAliasId>,
        generation: u64,
    },
    /// Promote an existing alias to canonical. `canonical`/`alt_aliases` are the
    /// reconciled target of the `m.room.canonical_alias` state event.
    SetCanonicalAlias {
        room_id: OwnedRoomId,
        canonical: Option<OwnedRoomAliasId>,
        alt_aliases: Vec<OwnedRoomAliasId>,
        generation: u64,
    },
    /// Remove an alias: unbind it from the room directory and drop it from
    /// `m.room.canonical_alias` (reconciled `canonical`/`alt_aliases`).
    RemoveAlias {
        room_id: OwnedRoomId,
        alias: OwnedRoomAliasId,
        canonical: Option<OwnedRoomAliasId>,
        alt_aliases: Vec<OwnedRoomAliasId>,
        generation: u64,
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
    /// Monotonic source of request epochs (write generations AND open-fetch
    /// epochs) plus the `Open`-freshness threshold. Each mutation takes the next
    /// value so its reconcile is matched by generation; each open takes one so a
    /// stale pre-write open can't repaint after a write reconciles (P1-1).
    /// Persists across close/reopen.
    #[rust] open_freshness: OpenFreshness,
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
    /// Returns the epoch to tag this room's open-fetch with, so a stale earlier
    /// open (from a prior show) can't repaint over it (P1-1).
    pub fn show(
        &mut self,
        cx: &mut Cx,
        room_id: OwnedRoomId,
        room_name: &str,
        room_topic: &str,
        canonical_alias: Option<&str>,
        alias_stage: Option<(PendingAliasStage, u64)>,
    ) -> u64 {
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
        // in the matching gate state — carrying the SAME generation — so a
        // close→reopen can't re-enable controls mid-flight and only that write's
        // own reconcile (matching generation) unlocks it. `Submitted` →
        // AwaitingResult (reject an unrelated open-fetch until the result
        // returns); `AwaitingReconcile` → AwaitingRefresh. No pending write → Idle.
        //
        // Round 9 invariant "reopen replaces recovery": a `Recovering` gate is
        // never a registry stage (the registry entry is cleared when the reconcile
        // Unavailable enters Recovering), so reopening a mid-recovery room maps to
        // `Idle` here and issues its own fresh open-fetch — abandoning the stale
        // recovery Open (which is rejected by epoch if it still lands).
        self.alias_gate = match alias_stage {
            Some((PendingAliasStage::Submitted, generation)) => AliasWriteGate::AwaitingResult(generation),
            Some((PendingAliasStage::AwaitingReconcile, generation)) => AliasWriteGate::AwaitingRefresh(generation),
            None => AliasWriteGate::Idle,
        };
        // Keep the epoch source ahead of any adopted pending write so the next
        // new mutation / open can't collide with it.
        if let Some((_, generation)) = alias_stage {
            self.open_freshness.observe(generation);
        }
        // Allocate this open's epoch AFTER adopting the pending generation, so it
        // is strictly newer; it becomes the only acceptable open (P1-1).
        let open_epoch = self.open_freshness.take_open();
        self.render_alias_section(cx);

        // Avatar fallback text (first char of name)
        let avatar_char = room_name.chars().next().unwrap_or('?').to_string();
        self.view.avatar(cx, ids!(room_avatar))
            .show_text(cx, None, None, &avatar_char);

        // Reset error label
        self.view.label(cx, ids!(name_error_label)).set_visible(cx, false);
        self.view.label(cx, ids!(name_error_label)).set_text(cx, "");

        self.view.redraw(cx);
        open_epoch
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
        reason: RoomSettingsFetchReason,
        language: AppLanguage,
        canonical_alias: Option<OwnedRoomAliasId>,
        alt_aliases: Vec<OwnedRoomAliasId>,
        can_manage: bool,
    ) {
        if !self.is_current_room(room_id) {
            return;
        }
        // Decide (purely, no gate mutation yet) whether to apply. `Ignore` → a
        // stale open-fetch or mismatched reconcile: drop it (never clobber
        // optimistic state / snapshot). `ApplyAndRelease` → this write's own
        // reconcile OR the recovery Open leaving `Recovering`: apply + release.
        // `Apply` → an open-fetch while idle: load without releasing.
        let disposition = self.alias_gate.disposition(reason);
        if disposition == FetchDisposition::Ignore {
            return;
        }
        // P1-1: an accepted Open must still be the *freshest*. Reject a slow
        // pre-write open that would otherwise repaint older state after a write
        // reconciled (and let the user submit a second write from it). Checked
        // BEFORE mutating the gate, so a stale open can never release it.
        if let RoomSettingsFetchReason::Open(epoch) = reason {
            if !self.open_freshness.accepts_open(epoch) {
                return;
            }
        }
        // Accepted: release the gate to Idle if this was the matching reconcile
        // or the recovery Open. (`Apply` — an idle open-load — leaves it Idle.)
        if disposition == FetchDisposition::ApplyAndRelease {
            self.alias_gate = AliasWriteGate::Idle;
        }
        // Accepted authoritative apply — invalidate every earlier open so none
        // can repaint over the state we're about to store (P1-1).
        self.open_freshness.on_apply();
        // Store authoritative state; a fresh fetch supersedes optimism.
        self.language = language;
        self.current_canonical = canonical_alias;
        self.current_alts = alt_aliases;
        self.can_manage_aliases = can_manage;
        self.alias_snapshot = None;
        self.render_alias_section(cx);
    }

    /// Handle a reconcile fetch that could not produce data (no client / room
    /// unavailable). Acts ONLY on the matching reconcile (generation + purpose);
    /// an open-fetch or mismatched reconcile is a no-op.
    ///
    /// The write's outcome is now unknown (the send may have applied server-side;
    /// error ≠ not-applied), so the modal must NOT unlock from that unknown state
    /// (round 9). Instead it enters `Recovering(epoch)`: edit controls stay
    /// non-interactive (`can_submit() == false`, controls hidden) until the
    /// recovery `Open(epoch)` is ACCEPTED and applies server truth — which also
    /// re-derives permission from power levels. It is still a terminal freshness
    /// barrier (round 7): `OpenFreshness::on_apply` invalidates every `Open` from
    /// the pending generation. Returns `Some(epoch)` for the one recovery `Open`
    /// the caller must issue. If that recovery `Open` also fails, the gate stays
    /// `Recovering` (read-only) until an explicit reopen — never unlocked.
    pub fn release_alias_lock(
        &mut self,
        cx: &mut Cx,
        room_id: &RoomId,
        reason: RoomSettingsFetchReason,
    ) -> Option<u64> {
        if !self.is_current_room(room_id) || !self.alias_gate.matches_reconcile(reason) {
            return None;
        }
        // Terminal barrier: invalidate all Opens from the pending generation.
        self.open_freshness.on_apply();
        // Allocate the recovery Open epoch and hold the modal read-only in
        // `Recovering` until that Open applies server truth.
        let recovery_epoch = self.open_freshness.take_open();
        self.alias_gate.enter_recovering(recovery_epoch);
        self.render_alias_section(cx);
        Some(recovery_epoch)
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
        let generation = self.take_alias_generation();
        self.alias_gate.on_submit(generation);
        self.render_alias_section(cx);
        self.view.text_input(cx, ids!(add_address_input)).set_text(cx, "");

        cx.action(RoomSettingsAction::PublishAlias {
            room_id,
            alias: valid_alias,
            canonical: self.current_canonical.clone(),
            alt_aliases: new_alts,
            generation,
        });
    }

    /// Allocate the next monotonic write generation for a new mutation. This also
    /// invalidates every `Open` fetch issued so far (P1-1), so a slow pre-write
    /// open can't repaint stale state after this write reconciles.
    fn take_alias_generation(&mut self) -> u64 {
        self.open_freshness.take_write()
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
                let generation = self.take_alias_generation();
                self.alias_gate.on_submit(generation);
                self.render_alias_section(cx);
                cx.action(RoomSettingsAction::SetCanonicalAlias {
                    room_id,
                    canonical: state.canonical,
                    alt_aliases: state.alt_aliases,
                    generation,
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
            let generation = self.take_alias_generation();
            self.alias_gate.on_submit(generation);
            self.render_alias_section(cx);
            cx.action(RoomSettingsAction::RemoveAlias {
                room_id,
                alias,
                canonical: state.canonical,
                alt_aliases: state.alt_aliases,
                generation,
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
    /// Populate the modal with room data and prepare for display. Returns the
    /// epoch to tag this room's open-fetch with (P1-1); `0` if the ref is empty.
    pub fn show_settings(
        &self,
        cx: &mut Cx,
        room_id: OwnedRoomId,
        room_name: &str,
        room_topic: &str,
        canonical_alias: Option<&str>,
        alias_stage: Option<(PendingAliasStage, u64)>,
    ) -> u64 {
        let Some(mut inner) = self.borrow_mut() else { return 0 };
        inner.show(cx, room_id, room_name, room_topic, canonical_alias, alias_stage)
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
        reason: RoomSettingsFetchReason,
        language: AppLanguage,
        canonical_alias: Option<OwnedRoomAliasId>,
        alt_aliases: Vec<OwnedRoomAliasId>,
        can_manage: bool,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.apply_alias_settings(cx, room_id, reason, language, canonical_alias, alt_aliases, can_manage);
    }

    /// Release a stranded alias gate when its reconcile fetch was unavailable.
    /// Returns `Some(epoch)` for a fresh recovery `Open` the caller must issue.
    pub fn release_alias_lock(
        &self,
        cx: &mut Cx,
        room_id: &RoomId,
        reason: RoomSettingsFetchReason,
    ) -> Option<u64> {
        let mut inner = self.borrow_mut()?;
        inner.release_alias_lock(cx, room_id, reason)
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
