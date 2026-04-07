//! Participants list widget for VoIP calls

use makepad_widgets::*;

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

#[derive(Script, ScriptHook, Widget)]
pub struct ParticipantsList {
    #[deref]
    view: View,
    #[rust]
    participants: Vec<Participant>,
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
                        widget.label(cx, ids!(mute_icon)).set_text(cx, if participant.is_muted { "M" } else { "" });
                        widget.label(cx, ids!(status_label)).set_text(cx, if participant.is_speaking { "Speaking" } else { "" });

                        // Toggle video/avatar visibility based on is_video_on
                        widget.view(cx, ids!(participant_video_host)).set_visible(cx, participant.is_video_on);
                        widget.view(cx, ids!(avatar_container)).set_visible(cx, !participant.is_video_on);

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
    pub fn add_participant(&mut self, cx: &mut Cx, participant: Participant) {
        self.participants.push(participant);
        self.redraw(cx);
    }

    pub fn remove_participant(&mut self, cx: &mut Cx, id: &str) {
        self.participants.retain(|p| p.id != id);
        self.redraw(cx);
    }

    pub fn update_participant(&mut self, cx: &mut Cx, id: &str, updater: impl FnOnce(&mut Participant)) {
        if let Some(participant) = self.participants.iter_mut().find(|p| p.id == id) {
            updater(participant);
            self.redraw(cx);
        }
    }

    pub fn clear(&mut self, cx: &mut Cx) {
        self.participants.clear();
        self.redraw(cx);
    }

    pub fn participants(&self) -> &[Participant] {
        &self.participants
    }
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

    pub fn get_participants(&self) -> Vec<Participant> {
        if let Some(inner) = self.borrow() {
            inner.participants.clone()
        } else {
            Vec::new()
        }
    }
}
