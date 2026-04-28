//! Lightweight wrapper for CPU-bound tasks.
//!
//! Currently each job is handled by spawning a detached native thread via
//! Makepad's `cx.spawn_thread`. This keeps the implementation simple while
//! still moving CPU-heavy work off the UI thread.

use makepad_widgets::{Cx, CxOsApi};
use std::sync::{atomic::AtomicBool, mpsc::Sender, Arc};
use crate::{
    room::member_search::{self, search_room_members_streaming_with_sort, PrecomputedMemberSort},
    shared::mentionable_text_input::SearchResult,
    sliding_sync::TimelineKind,
};
use matrix_sdk::room::RoomMember;

pub enum CpuJob {
    SearchRoomMembers(SearchRoomMembersJob),
    PrecomputeMemberSort(PrecomputeMemberSortJob),
}

/// Action posted back to UI thread when precomputed sort is ready.
#[derive(Debug)]
pub struct PrecomputedMemberSortReady {
    pub timeline_kind: TimelineKind,
    pub sort: Arc<PrecomputedMemberSort>,
    /// The Arc<Vec<RoomMember>> this sort was computed for.
    /// Held alive to prevent ABA via address reuse; compared by Arc::ptr_eq.
    pub members_arc: Arc<Vec<RoomMember>>,
}

pub struct PrecomputeMemberSortJob {
    pub timeline_kind: TimelineKind,
    pub members: Arc<Vec<RoomMember>>,
}

pub struct SearchRoomMembersJob {
    pub members: Arc<Vec<RoomMember>>,
    pub search_text: String,
    pub max_results: usize,
    pub sender: Sender<SearchResult>,
    pub search_id: u64,
    pub precomputed_sort: Option<Arc<PrecomputedMemberSort>>,
    pub cancel_token: Option<Arc<AtomicBool>>,
}

fn run_member_search(params: SearchRoomMembersJob) {
    let SearchRoomMembersJob {
        members,
        search_text,
        max_results,
        sender,
        search_id,
        precomputed_sort,
        cancel_token,
    } = params;

    search_room_members_streaming_with_sort(
        members,
        search_text,
        max_results,
        sender,
        search_id,
        precomputed_sort,
        cancel_token,
    );
}

fn run_precompute_sort(params: PrecomputeMemberSortJob) {
    let sort = member_search::precompute_member_sort(&params.members);
    Cx::post_action(PrecomputedMemberSortReady {
        timeline_kind: params.timeline_kind,
        sort: Arc::new(sort),
        members_arc: params.members, // keep alive to prevent ABA
    });
}

/// Spawns a CPU-bound job on a detached native thread.
pub fn spawn_cpu_job(cx: &mut Cx, job: CpuJob) {
    cx.spawn_thread(move || match job {
        CpuJob::SearchRoomMembers(params) => run_member_search(params),
        CpuJob::PrecomputeMemberSort(params) => run_precompute_sort(params),
    });
}
