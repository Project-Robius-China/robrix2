//! This module defines a badge that shows the count of unread mentions
//! (`@`-prefixed, in red) or unread messages (bare count, in gray).
//!
//! Color alone does not carry the mention/message distinction: the `@` prefix
//! is the redundant channel that keeps the two readable for color-blind users
//! and in isolation, where there is no neighbouring badge to compare against.

use makepad_widgets::*;


script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.UnreadBadge = #(UnreadBadge::register_widget(vm)) {

        width: 30, height: 20,
        align: Align{ x: 0.5, y: 0.5 }
        flow: Overlay,

        rounded_view := View {
            width: Fill,
            height: Fill,
            show_bg: true,
            draw_bg +: {
                badge_color: instance((COLOR_UNREAD_BADGE_MESSAGES)),
                border_radius: instance(4.0)
                // Set this border_size to a larger value to make the oval smaller 
                border_size: instance(2.0)

                pixel: fn() {
                    let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                    sdf.box(
                        self.border_size,
                        1.0,
                        self.rect_size.x - (self.border_size * 2.0),
                        self.rect_size.y - 2.0,
                        max(1.0, self.border_radius)
                    )
                    sdf.fill_keep(self.badge_color);
                    return sdf.result;
                }
            }
        }
        // Label that displays the unread message count
        label_count := Label {
            padding: 0,
            width: Fit,
            height: Fit,
            flow: Right, // do not wrap
            text: "",
            draw_text +: {
                color: #ffffff,
                text_style: theme.font_regular {font_size: 8.0},
            }
        }
    }
}


#[derive(Script, ScriptHook, Widget)]
pub struct UnreadBadge {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    #[live] is_marked_unread: bool,
    #[live] unread_mentions: u64,
    #[live] unread_messages: u64,
}

impl Widget for UnreadBadge {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {

        /// Helper function to format the badge's rounded rectangle.
        ///
        /// The rounded rectangle needs to be wider for longer text.
        /// It also adds a plus sign at the end if the unread count is greater than 99.
        ///
        /// `extra_glyphs` accounts for characters drawn beyond the digits
        /// themselves — the mention badge's leading `@` — so the pill grows to
        /// fit them instead of clipping.
        fn format_border_and_truncation(count: u64, extra_glyphs: u64) -> (f64, &'static str) {
            let (border_size, plus_sign) = if count > 99 {
                (0.0, "+")
            } else if count > 9 {
                (2.0, "")
            } else {
                (5.0, "")
            };
            // Each extra glyph costs roughly one digit's worth of width, which
            // the border insets by ~3px per step.
            (
                (border_size - 3.0 * extra_glyphs as f64).max(0.0),
                plus_sign,
            )
        }

        // If there are unread mentions, show red badge and the number of unread mentions.
        //
        // The `@` matters: mentions and plain unreads were previously identical
        // glyphs distinguished only by badge color, so a red "3" and a gray "3"
        // are the same badge to anyone who can't separate the two hues — and to
        // anyone glancing at a list where no gray badge is nearby to compare
        // against. The prefix carries the same signal in a second channel.
        if self.unread_mentions > 0 {
            let (border_size, plus_sign) = format_border_and_truncation(self.unread_mentions, 1);
            self.label(cx, ids!(label_count))
                .set_text(cx, &format!("@{}{plus_sign}", std::cmp::min(self.unread_mentions, 99)));
            let mut rounded_view = self.view(cx, ids!(rounded_view));
            script_apply_eval!(cx, rounded_view, {
                draw_bg +: {
                    border_size: #(border_size),
                    badge_color: mod.widgets.COLOR_UNREAD_BADGE_MENTIONS
                }
            });
            self.visible = true;
        }
        // If there are no unread mentions but this is marked as unread, show the badge as a dot.
        else if self.is_marked_unread {
            self.label(cx, ids!(label_count)).set_text(cx, "");
            let mut rounded_view = self.view(cx, ids!(rounded_view));
            script_apply_eval!(cx, rounded_view, {
                draw_bg +: {
                    border_size: 6.0, // larger value = smaller dot
                    badge_color: mod.widgets.COLOR_UNREAD_BADGE_MARKED
                }
            });
            self.visible = true;
        }
        // If there are no unread mentions but there are unread messages, show gray badge and the number of unread messages
        else if self.unread_messages > 0 {
            let (border_size, plus_sign) = format_border_and_truncation(self.unread_messages, 0);
            self.label(cx, ids!(label_count))
                .set_text(cx, &format!("{}{plus_sign}", std::cmp::min(self.unread_messages, 99)));
            let mut rounded_view = self.view(cx, ids!(rounded_view));
            script_apply_eval!(cx, rounded_view, {
                draw_bg +: {
                    border_size: #(border_size),
                    badge_color: mod.widgets.COLOR_UNREAD_BADGE_MESSAGES
                }
            });
            self.visible = true;
        }
        else {
            // If there are no unreads of any kind, hide the badge
            self.visible = false;
        }

        self.view.draw_walk(cx, scope, walk)
    }
}

impl UnreadBadgeRef {
    /// Sets the unread mentions and messages counts without explicitly redrawing the badge.
    pub fn update_counts(&self, is_marked_unread: bool, num_unread_mentions: u64, num_unread_messages: u64) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.is_marked_unread = is_marked_unread;
            inner.unread_mentions = num_unread_mentions;
            inner.unread_messages = num_unread_messages;
            inner.visible = is_marked_unread || num_unread_mentions > 0 || num_unread_messages > 0;
        }
    }
}
