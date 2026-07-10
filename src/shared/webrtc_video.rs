//! WebRTC video streaming widget for displaying video frames from WebRTC sources.
//!
//! This widget is designed to receive video frames as RGBA data from WebRTC
//! streams (e.g., LiveKit) and display them efficiently using GPU textures.

use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets_internal.*

    mod.widgets.WebRtcVideoBase = #(WebRtcVideo::register_widget(vm))

    mod.widgets.WebRtcVideo = set_type_default() do mod.widgets.WebRtcVideoBase{
        width: 320
        height: 240

        draw_bg +: {
            video_texture: texture_2d(float)
            opacity: instance(1.0)

            pixel: fn() {
                let color = self.video_texture.sample_as_bgra(self.pos)
                return Pal.premul(vec4(color.xyz, color.w * self.opacity))
            }
        }
    }
}

/// Video frame data structure for receiving frames from WebRTC sources.
#[derive(Clone)]
pub struct WebRtcVideoFrame {
    /// RGBA pixel data (4 bytes per pixel: R, G, B, A)
    pub data: Vec<u8>,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Optional participant identifier
    pub participant_id: Option<String>,
}

impl std::fmt::Debug for WebRtcVideoFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebRtcVideoFrame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("participant_id", &self.participant_id)
            .field("data_len", &self.data.len())
            .finish()
    }
}

/// Actions emitted by the WebRtcVideo widget.
#[derive(Clone, Debug, Default)]
pub enum WebRtcVideoAction {
    #[default]
    None,
    /// A new frame has been displayed
    FrameUpdated {
        width: u32,
        height: u32,
    },
    /// The video stream has started (first frame received)
    StreamStarted,
    /// The video stream has stopped
    StreamStopped,
}

/// WebRTC video streaming widget.
///
/// This widget displays video frames from WebRTC sources such as LiveKit.
/// Frames are provided as RGBA data and rendered using GPU textures.
///
/// # Example
///
/// ```ignore
/// // In your UI definition:
/// webrtc_video := WebRtcVideo {
///     width: 320
///     height: 240
/// }
///
/// // In your code:
/// let frame = WebRtcVideoFrame {
///     data: rgba_data,
///     width: 640,
///     height: 480,
///     participant_id: Some("user123".to_string()),
/// };
/// self.ui.webrtc_video(cx, ids!(webrtc_video)).set_frame(cx, frame);
/// ```
#[derive(Script, ScriptHook, Widget)]
pub struct WebRtcVideo {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,

    #[redraw]
    #[live]
    draw_bg: DrawColor,

    #[walk]
    walk: Walk,

    #[live]
    layout: Layout,

    #[visible]
    #[live(true)]
    visible: bool,

    /// The current video texture
    #[rust]
    texture: Option<Texture>,

    /// Current frame dimensions
    #[rust]
    frame_width: u32,
    #[rust]
    frame_height: u32,

    /// Whether the stream is active (has received at least one frame)
    #[rust]
    stream_active: bool,

    /// Participant ID of the current stream source
    #[rust]
    participant_id: Option<String>,
}

impl Widget for WebRtcVideo {
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        if !self.visible {
            return DrawStep::done();
        }

        // Set texture if available
        if let Some(texture) = &self.texture {
            self.draw_bg.draw_vars.set_texture(0, texture);
        } else {
            self.draw_bg.draw_vars.empty_texture(0);
        }

        self.draw_bg.draw_walk(cx, walk);
        DrawStep::done()
    }

    fn handle_event(&mut self, _cx: &mut Cx, _event: &Event, _scope: &mut Scope) {
        // No special event handling needed for basic video display
    }
}

impl WebRtcVideo {
    /// Sets a video frame to be displayed.
    ///
    /// The frame data should be in RGBA format (4 bytes per pixel).
    /// This method handles texture creation and updates efficiently.
    pub fn set_frame(&mut self, cx: &mut Cx, frame: WebRtcVideoFrame) {
        let width = frame.width as usize;
        let height = frame.height as usize;
        let pixel_count = width * height;

        // Convert RGBA bytes to packed u32 for VecBGRAu8_32 texture format
        // Input: RGBA bytes [R, G, B, A]
        // Output: 0xAARRGGBB format (BGRA in memory on little-endian)
        let mut data_u32: Vec<u32> = Vec::with_capacity(pixel_count);
        let rgba = &frame.data;

        for i in 0..pixel_count {
            let idx = i * 4;
            if idx + 3 < rgba.len() {
                let r = rgba[idx] as u32;
                let g = rgba[idx + 1] as u32;
                let b = rgba[idx + 2] as u32;
                let a = rgba[idx + 3] as u32;
                // Pack as 0xAARRGGBB
                data_u32.push((a << 24) | (r << 16) | (g << 8) | b);
            }
        }

        // Check if we need to create a new texture (dimension change or first frame)
        let needs_new_texture = match &self.texture {
            Some(texture) => {
                texture.get_format(cx).vec_width_height() != Some((width, height))
            }
            None => true,
        };

        let was_inactive = !self.stream_active;

        if needs_new_texture {
            // Create new texture with the frame data
            let texture = Texture::new_with_format(
                cx,
                TextureFormat::VecBGRAu8_32 {
                    width,
                    height,
                    data: Some(data_u32),
                    updated: TextureUpdated::Full,
                },
            );
            self.texture = Some(texture);
            self.frame_width = frame.width;
            self.frame_height = frame.height;
        } else if let Some(texture) = &self.texture {
            // Reuse existing texture, just update the data
            texture.set_data_u32(cx, width, height, data_u32);
        }

        // Update state
        self.stream_active = true;
        self.participant_id = frame.participant_id;

        // Emit action for first frame
        if was_inactive {
            cx.widget_action(self.uid, WebRtcVideoAction::StreamStarted);
        }

        cx.widget_action(
            self.uid,
            WebRtcVideoAction::FrameUpdated {
                width: frame.width,
                height: frame.height,
            },
        );

        // Trigger redraw
        self.redraw(cx);
    }

    /// Clears the current video frame and marks the stream as inactive.
    pub fn clear_frame(&mut self, cx: &mut Cx) {
        self.texture = None;
        self.frame_width = 0;
        self.frame_height = 0;
        self.participant_id = None;

        if self.stream_active {
            self.stream_active = false;
            cx.widget_action(self.uid, WebRtcVideoAction::StreamStopped);
        }

        self.redraw(cx);
    }

    /// Returns the current frame dimensions, or None if no frame has been set.
    pub fn frame_size(&self) -> Option<(u32, u32)> {
        if self.stream_active {
            Some((self.frame_width, self.frame_height))
        } else {
            None
        }
    }

    /// Returns true if the stream is currently active (has received frames).
    pub fn is_active(&self) -> bool {
        self.stream_active
    }

    /// Returns the participant ID of the current stream source.
    pub fn participant_id(&self) -> Option<&str> {
        self.participant_id.as_deref()
    }

    /// Sets the texture directly (advanced usage).
    pub fn set_texture(&mut self, cx: &mut Cx, texture: Option<Texture>) {
        self.texture = texture;
        self.redraw(cx);
    }
}

/// Reference to a WebRtcVideo widget.
impl WebRtcVideoRef {
    /// Sets a video frame to be displayed.
    pub fn set_frame(&self, cx: &mut Cx, frame: WebRtcVideoFrame) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_frame(cx, frame);
        }
    }

    /// Clears the current video frame.
    pub fn clear_frame(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.clear_frame(cx);
        }
    }

    /// Returns the current frame dimensions.
    pub fn frame_size(&self) -> Option<(u32, u32)> {
        if let Some(inner) = self.borrow() {
            inner.frame_size()
        } else {
            None
        }
    }

    /// Returns true if the stream is currently active.
    pub fn is_active(&self) -> bool {
        if let Some(inner) = self.borrow() {
            inner.is_active()
        } else {
            false
        }
    }

    /// Returns the participant ID of the current stream source.
    pub fn participant_id(&self) -> Option<String> {
        if let Some(inner) = self.borrow() {
            inner.participant_id().map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Sets the texture directly.
    pub fn set_texture(&self, cx: &mut Cx, texture: Option<Texture>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_texture(cx, texture);
        }
    }
}
