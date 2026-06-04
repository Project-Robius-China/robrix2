//! Participants list widget for VoIP calls
//!
//! This module provides a participants list widget that displays remote
//! participants in a VoIP call, including their video feeds rendered as textures.

use std::collections::HashMap;
use makepad_widgets::*;
use crate::shared::webrtc_video::{WebRtcVideoWidgetRefExt, WebRtcVideoFrame};

/// Represents a remote participant in a VoIP call
#[derive(Clone, Debug)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub avatar_letter: String,
    pub is_muted: bool,
    pub is_speaking: bool,
    pub is_video_on: bool,
}

impl Default for Participant {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::from("Unknown"),
            avatar_letter: String::from("?"),
            is_muted: false,
            is_speaking: false,
            is_video_on: false,
        }
    }
}

/// Internal state for a participant's video frame data
struct ParticipantVideoFrame {
    /// RGBA pixel data
    data: Vec<u8>,
    width: u32,
    height: u32,
}

#[derive(Script, ScriptHook, Widget)]
pub struct ParticipantsList {
    #[deref]
    view: View,
    #[rust]
    participants: Vec<Participant>,
    /// Video frames for each participant, keyed by participant ID
    #[rust]
    video_frames: HashMap<String, ParticipantVideoFrame>,
}

impl Widget for ParticipantsList {
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = item.as_flat_list().borrow_mut() {
                for (i, participant) in self.participants.iter().enumerate() {
                    let item_id = LiveId::from_num(0, i as u64);
                    if let Some(widget) = list.item(cx, item_id, live_id!(ParticipantItem)) {
                        widget.label(cx, ids!(avatar_letter)).set_text(cx, &participant.avatar_letter);
                        widget.label(cx, ids!(name_label)).set_text(cx, &participant.name);

                        // Update mute icon color based on mute status
                        let mut mute_btn = widget.button(cx, ids!(mute_icon));
                        if participant.is_muted {
                            script_apply_eval!(cx, mute_btn, {
                                draw_icon +: { color: #e53935 }
                            });
                        } else {
                            script_apply_eval!(cx, mute_btn, {
                                draw_icon +: { color: #aaa }
                            });
                        }

                        widget.label(cx, ids!(status_label)).set_text(cx, if participant.is_speaking { "Speaking" } else { "" });

                        // Toggle video/avatar visibility based on is_video_on
                        let has_frame = self.video_frames.contains_key(&participant.id);
                        let has_video = participant.is_video_on && has_frame;

                        let video_widget = widget.web_rtc_video(cx, ids!(participant_video));
                        video_widget.set_visible(cx, has_video);
                        widget.view(cx, ids!(avatar_container)).set_visible(cx, !has_video);

                        // If video is on and we have frame data, set it on the WebRtcVideo widget
                        if has_video {
                            if let Some(video_frame) = self.video_frames.get(&participant.id) {
                                let webrtc_frame = WebRtcVideoFrame {
                                    data: video_frame.data.clone(),
                                    width: video_frame.width,
                                    height: video_frame.height,
                                    participant_id: Some(participant.id.clone()),
                                };
                                video_widget.set_frame(cx, webrtc_frame);
                            }
                        }

                        widget.draw_all(cx, scope);
                    }
                }
            }
        }
        DrawStep::done()
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }
}

impl ParticipantsList {
    /// Add a new participant to the list
    pub fn add_participant(&mut self, cx: &mut Cx, participant: Participant) {
        self.participants.push(participant);
        self.redraw(cx);
    }

    /// Remove a participant from the list
    pub fn remove_participant(&mut self, cx: &mut Cx, id: &str) {
        self.participants.retain(|p| p.id != id);
        self.video_frames.remove(id);
        self.redraw(cx);
    }

    /// Update a participant's properties
    pub fn update_participant(&mut self, cx: &mut Cx, id: &str, updater: impl FnOnce(&mut Participant)) {
        if let Some(participant) = self.participants.iter_mut().find(|p| p.id == id) {
            updater(participant);
            self.redraw(cx);
        }
    }

    /// Clear all participants from the list
    pub fn clear(&mut self, cx: &mut Cx) {
        self.participants.clear();
        // Don't clear video_frames - they will be reused when participants are re-added
        // This preserves video streams across participant list refreshes
        self.redraw(cx);
    }

    /// Clear all participants and their video frames
    pub fn clear_all(&mut self, cx: &mut Cx) {
        self.participants.clear();
        self.video_frames.clear();
        self.redraw(cx);
    }

    /// Get a reference to the participants list
    pub fn participants(&self) -> &[Participant] {
        &self.participants
    }

    /// Push an I420 video frame to a participant's video
    ///
    /// This converts the I420 YUV data to RGBA and stores it for rendering.
    ///
    /// # Arguments
    /// * `livekit_participant_id` - The LiveKit identity (may include session suffix)
    /// * `y` - Y plane data
    /// * `u` - U plane data
    /// * `v` - V plane data
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `_pts_ms` - Presentation timestamp in milliseconds (currently unused)
    pub fn push_video_frame(
        &mut self,
        cx: &mut Cx,
        livekit_participant_id: &str,
        y: Vec<u8>,
        u: Vec<u8>,
        v: Vec<u8>,
        width: u32,
        height: u32,
        _pts_ms: u64,
    ) {
        // LiveKit identity format: "@user:server.tld:session_id"
        // Matrix user_id format: "@user:server.tld"
        // We need to match by user_id prefix since LiveKit adds session suffix

        // Find matching participant - try exact match first, then prefix match
        let storage_key = self.participants.iter()
            .find(|p| p.id == livekit_participant_id)
            .or_else(|| self.participants.iter().find(|p| livekit_participant_id.starts_with(&p.id)))
            .map(|p| p.id.clone());

        let storage_key = match storage_key {
            Some(key) => key,
            None => {
                // No matching participant found - store under LiveKit ID anyway
                // (participant might be added later)
                livekit_participant_id.to_string()
            }
        };

        // Convert I420 YUV to RGBA
        let rgba_data = i420_to_rgba(&y, &u, &v, width, height);

        // Store the RGBA frame data
        self.video_frames.insert(
            storage_key.clone(),
            ParticipantVideoFrame {
                data: rgba_data,
                width,
                height,
            },
        );

        // Mark participant as having video
        if let Some(participant) = self.participants.iter_mut().find(|p| p.id == storage_key) {
            participant.is_video_on = true;
        }

        self.redraw(cx);
    }

    /// Check if a participant has an active video frame
    /// Checks both exact match and prefix match (for LiveKit session IDs)
    pub fn has_video_frame(&self, participant_id: &str) -> bool {
        // Exact match
        if self.video_frames.contains_key(participant_id) {
            return true;
        }
        // Check if any frame key starts with this participant_id
        // (for when frame was stored under LiveKit ID before participant was matched)
        self.video_frames.keys().any(|k| k.starts_with(participant_id))
    }

    /// Check if a participant has an active video texture (alias for has_video_frame)
    pub fn has_video_texture(&self, participant_id: &str) -> bool {
        self.has_video_frame(participant_id)
    }
}

/// Convert I420 YUV to RGBA
fn i420_to_rgba(y: &[u8], u: &[u8], v: &[u8], width: u32, height: u32) -> Vec<u8> {
    let width = width as usize;
    let height = height as usize;
    let mut rgba = vec![0u8; width * height * 4];

    for j in 0..height {
        for i in 0..width {
            let y_idx = j * width + i;
            let uv_idx = (j / 2) * (width / 2) + (i / 2);

            let y_val = y[y_idx] as f32;
            let u_val = u[uv_idx] as f32 - 128.0;
            let v_val = v[uv_idx] as f32 - 128.0;

            // BT.601 YUV to RGB conversion
            let r = (y_val + 1.402 * v_val).clamp(0.0, 255.0) as u8;
            let g = (y_val - 0.344 * u_val - 0.714 * v_val).clamp(0.0, 255.0) as u8;
            let b = (y_val + 1.772 * u_val).clamp(0.0, 255.0) as u8;

            let rgba_idx = (j * width + i) * 4;
            rgba[rgba_idx] = r;
            rgba[rgba_idx + 1] = g;
            rgba[rgba_idx + 2] = b;
            rgba[rgba_idx + 3] = 255; // Alpha
        }
    }

    rgba
}

impl ParticipantsListRef {
    pub fn add_participant(&self, cx: &mut Cx, participant: Participant) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.add_participant(cx, participant);
        }
    }

    pub fn remove_participant(&self, cx: &mut Cx, id: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.remove_participant(cx, id);
        }
    }

    pub fn update_participant(&self, cx: &mut Cx, id: &str, updater: impl FnOnce(&mut Participant)) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.update_participant(cx, id, updater);
        }
    }

    pub fn clear(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.clear(cx);
        }
    }

    pub fn clear_all(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.clear_all(cx);
        }
    }

    pub fn get_participants(&self) -> Vec<Participant> {
        if let Some(inner) = self.borrow() {
            inner.participants.clone()
        } else {
            Vec::new()
        }
    }

    /// Check if a participant has an active video texture
    pub fn has_video_texture(&self, participant_id: &str) -> bool {
        if let Some(inner) = self.borrow() {
            inner.has_video_texture(participant_id)
        } else {
            false
        }
    }

    /// Push an I420 video frame to a participant's video texture
    pub fn push_video_frame(
        &self,
        cx: &mut Cx,
        participant_id: &str,
        y: Vec<u8>,
        u: Vec<u8>,
        v: Vec<u8>,
        width: u32,
        height: u32,
        pts_ms: u64,
    ) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.push_video_frame(cx, participant_id, y, u, v, width, height, pts_ms);
        }
    }
}
