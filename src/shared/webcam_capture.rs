//! Standalone webcam capture widget for efficient video display.
//!
//! This widget wraps Makepad's native Video widget for GPU-accelerated
//! camera preview with a clean, simplified API.

use makepad_widgets::*;
use makepad_widgets::makepad_platform::video::{
    VideoInputId, VideoFormatId, VideoInputsEvent, VideoPixelFormat,
};
use makepad_widgets::video::VideoCameraPreviewMode;

use crate::voip::VoipGlobalState;

script_mod! {
    use mod.prelude.widgets.*

    mod.widgets.WebcamCaptureBase = #(WebcamCapture::register_widget(vm))

    mod.widgets.WebcamCapture = set_type_default() do mod.widgets.WebcamCaptureBase {
        width: Fill
        height: Fill

        flow: Overlay

        // Native video preview (GPU-accelerated)
        video := Video {
            width: Fill
            height: Fill
        }

        // Placeholder shown when camera is inactive
        placeholder := View {
            width: Fill
            height: Fill
            visible: true
            align: Align{ x: 0.5, y: 0.5 }
            show_bg: true
            draw_bg +: { color: #1a1a1a }

            placeholder_label := Label {
                text: "Camera Off"
                draw_text +: { color: #666 }
            }
        }
    }
}

/// Camera format choice for capture configuration.
#[derive(Clone, Debug)]
pub struct CameraChoice {
    pub input_id: VideoInputId,
    pub format_id: VideoFormatId,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub pixel_format: VideoPixelFormat,
}

impl CameraChoice {
    /// Pick the best camera format from available video inputs.
    pub fn pick_best(ev: &VideoInputsEvent) -> Option<Self> {
        let desc = ev.descs.first()?;

        fn pixel_rank(pf: VideoPixelFormat) -> usize {
            match pf {
                VideoPixelFormat::NV12 => 4,
                VideoPixelFormat::YUY2 => 3,
                VideoPixelFormat::YUV420 => 2,
                VideoPixelFormat::RGB24 => 1,
                _ => 0,
            }
        }

        fn better(
            a: &makepad_widgets::makepad_platform::video::VideoFormat,
            b: &makepad_widgets::makepad_platform::video::VideoFormat,
        ) -> bool {
            let ar = pixel_rank(a.pixel_format);
            let br = pixel_rank(b.pixel_format);
            if ar != br { return ar > br; }
            let ap = a.width * a.height;
            let bp = b.width * b.height;
            if ap != bp { return ap > bp; }
            a.frame_rate.unwrap_or(0.0) > b.frame_rate.unwrap_or(0.0)
        }

        let mut best: Option<makepad_widgets::makepad_platform::video::VideoFormat> = None;

        // Pass 1: NV12 at <= 1080p
        for fmt in &desc.formats {
            if fmt.pixel_format == VideoPixelFormat::NV12
                && fmt.width <= 1920 && fmt.height <= 1080
                && best.as_ref().is_none_or(|b| better(fmt, b))
            {
                best = Some(*fmt);
            }
        }

        // Pass 2: any NV12
        if best.is_none() {
            for fmt in &desc.formats {
                if fmt.pixel_format == VideoPixelFormat::NV12
                    && best.as_ref().is_none_or(|b| better(fmt, b))
                {
                    best = Some(*fmt);
                }
            }
        }

        // Pass 3: YUY2 or YUV420
        if best.is_none() {
            for fmt in &desc.formats {
                if matches!(fmt.pixel_format, VideoPixelFormat::YUY2 | VideoPixelFormat::YUV420)
                    && best.as_ref().is_none_or(|b| better(fmt, b))
                {
                    best = Some(*fmt);
                }
            }
        }

        // Pass 4: fallback
        if best.is_none() {
            best = desc.formats.first().copied();
        }

        let format = best?;
        Some(CameraChoice {
            input_id: desc.input_id,
            format_id: format.format_id,
            name: desc.name.clone(),
            width: format.width,
            height: format.height,
            pixel_format: format.pixel_format,
        })
    }
}

/// Convert from voip::CameraChoice to shared::CameraChoice
impl From<crate::voip::camera::CameraChoice> for CameraChoice {
    fn from(c: crate::voip::camera::CameraChoice) -> Self {
        Self {
            input_id: c.input_id,
            format_id: c.format_id,
            name: c.name,
            width: c.width,
            height: c.height,
            pixel_format: c.pixel_format,
        }
    }
}

/// Actions emitted by the WebcamCapture widget.
#[derive(Clone, Debug, Default)]
pub enum WebcamCaptureAction {
    #[default]
    None,
    CaptureStarted { width: u32, height: u32 },
    CaptureStopped,
    NoCameraAvailable,
    Error(String),
}

/// Capture state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CaptureState {
    #[default]
    Idle,
    Starting,
    Active,
    Stopping,
}

/// Standalone webcam capture widget using native GPU-accelerated preview.
#[derive(Script, ScriptHook, Widget)]
pub struct WebcamCapture {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    #[walk] walk: Walk,
    #[layout] layout: Layout,

    #[visible]
    #[live(true)]
    visible: bool,

    #[rust] state: CaptureState,
    #[rust] camera_choice: Option<CameraChoice>,
}

impl Widget for WebcamCapture {
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        match event {
            Event::VideoPlaybackPrepared(_) if self.state == CaptureState::Starting => {
                self.state = CaptureState::Active;
                self.view(cx, ids!(placeholder)).set_visible(cx, false);
                if let Some(ref choice) = self.camera_choice {
                    cx.widget_action(self.widget_uid(), WebcamCaptureAction::CaptureStarted {
                        width: choice.width as u32,
                        height: choice.height as u32,
                    });
                }
            }
            Event::VideoPlaybackResourcesReleased(_) if self.state == CaptureState::Stopping => {
                self.state = CaptureState::Idle;
                self.update_placeholder(cx, "Camera Off");
                cx.widget_action(self.widget_uid(), WebcamCaptureAction::CaptureStopped);
            }
            // Note: Event::VideoInputs is handled at the App level and stored in VoipGlobalState.
            // WebcamCapture reads from global state via start_capture_from_global().
            _ => {}
        }
    }
}

impl WebcamCapture {
    /// Update the placeholder text
    fn update_placeholder(&mut self, cx: &mut Cx, text: &str) {
        self.view(cx, ids!(placeholder)).set_visible(cx, true);
        self.view.label(cx, ids!(placeholder_label)).set_text(cx, text);
    }

    /// Internal: actually start the camera
    fn start_camera_internal(&mut self, cx: &mut Cx) {
        let Some(choice) = self.camera_choice.clone() else {
            return;
        };

        let video = self.view.video(cx, ids!(video));
        if !video.is_unprepared() {
            log!("WebcamCapture: Video not unprepared, cannot start");
            return;
        }

        log!("WebcamCapture: Starting camera {} ({}x{} {:?})",
            choice.name, choice.width, choice.height, choice.pixel_format);

        self.state = CaptureState::Starting;
        self.update_placeholder(cx, "Starting Camera...");

        video.set_camera_preview_mode(cx, VideoCameraPreviewMode::Native);
        video.set_source_camera(cx, choice.input_id, choice.format_id);
        video.begin_playback(cx);
    }

    /// Start camera capture using global camera choice from VoipGlobalState.
    /// VoipGlobalState must be initialized (via VoipGlobalState::initialize) before calling this.
    pub fn start_capture_from_global(&mut self, cx: &mut Cx) {
        if self.state != CaptureState::Idle {
            log!("WebcamCapture: Already in state {:?}, not starting", self.state);
            return;
        }

        // Read camera choice from global state (populated by App handling Event::VideoInputs)
        if let Some(c) = VoipGlobalState::get_camera_choice(cx) {
            log!("WebcamCapture: Got camera from global state: {}", c.name);
            self.camera_choice = Some(c.into());
            self.start_camera_internal(cx);
        } else {
            // No camera available in global state - VoipGlobalState may not have received VideoInputs yet
            log!("WebcamCapture: No camera in global state");
            self.update_placeholder(cx, "No Camera Available");
            cx.widget_action(self.widget_uid(), WebcamCaptureAction::NoCameraAvailable);
        }
    }

    /// Start camera capture with the given camera choice.
    pub fn start_capture(&mut self, cx: &mut Cx, choice: CameraChoice) {
        if self.state != CaptureState::Idle {
            return;
        }

        self.camera_choice = Some(choice);
        self.start_camera_internal(cx);
    }

    /// Stop camera capture.
    pub fn stop_capture(&mut self, cx: &mut Cx) {
        if self.state == CaptureState::Idle {
            return;
        }

        let video = self.view.video(cx, ids!(video));
        if !video.is_unprepared() && !video.is_cleaning_up() {
            self.state = CaptureState::Stopping;
            video.stop_and_cleanup_resources(cx);
        }

        self.camera_choice = None;
    }

    /// Get the current capture state.
    pub fn capture_state(&self) -> CaptureState {
        self.state
    }

    /// Check if capture is active.
    pub fn is_capturing(&self) -> bool {
        self.state == CaptureState::Active
    }

    /// Get the current frame dimensions.
    pub fn frame_size(&self) -> Option<(u32, u32)> {
        if self.state == CaptureState::Active {
            self.camera_choice.as_ref().map(|c| (c.width as u32, c.height as u32))
        } else {
            None
        }
    }

    /// Get the camera choice.
    pub fn camera_choice(&self) -> Option<&CameraChoice> {
        self.camera_choice.as_ref()
    }
}

/// Reference to a WebcamCapture widget.
impl WebcamCaptureRef {
    pub fn start_capture_from_global(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.start_capture_from_global(cx);
        }
    }

    pub fn start_capture(&self, cx: &mut Cx, choice: CameraChoice) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.start_capture(cx, choice);
        }
    }

    pub fn stop_capture(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.stop_capture(cx);
        }
    }

    pub fn is_capturing(&self) -> bool {
        self.borrow().map(|i| i.is_capturing()).unwrap_or(false)
    }

    pub fn capture_state(&self) -> CaptureState {
        self.borrow().map(|i| i.capture_state()).unwrap_or(CaptureState::Idle)
    }

    pub fn frame_size(&self) -> Option<(u32, u32)> {
        self.borrow().and_then(|i| i.frame_size())
    }
}
