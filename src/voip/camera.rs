//! Camera handling and video management

use makepad_widgets::*;
use makepad_widgets::makepad_platform::video::{VideoInputId, VideoFormatId, VideoInputsEvent, VideoPixelFormat};
use makepad_widgets::video::VideoCameraPreviewMode;

/// Camera format choice
#[derive(Clone)]
pub struct CameraChoice {
    pub input_id: VideoInputId,
    pub format_id: VideoFormatId,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub pixel_format: VideoPixelFormat,
}

/// Camera manager handles camera selection and video playback
pub struct CameraManager;

impl CameraManager {
    /// Pick the best camera format from available options
    pub fn pick_camera_choice(ev: &VideoInputsEvent) -> Option<CameraChoice> {
        let desc = ev.descs.first()?;

        log!("Camera: {} has {} formats", desc.name, desc.formats.len());
        for (i, fmt) in desc.formats.iter().enumerate() {
            log!(
                "  Format {}: {}x{} {:?} fps={:?}",
                i, fmt.width, fmt.height, fmt.pixel_format, fmt.frame_rate
            );
        }

        fn pixel_rank(pixel_format: VideoPixelFormat) -> usize {
            match pixel_format {
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
            let a_rank = pixel_rank(a.pixel_format);
            let b_rank = pixel_rank(b.pixel_format);
            if a_rank != b_rank {
                return a_rank > b_rank;
            }
            let a_pixels = a.width * a.height;
            let b_pixels = b.width * b.height;
            if a_pixels != b_pixels {
                return a_pixels > b_pixels;
            }
            let a_fps = a.frame_rate.unwrap_or(0.0);
            let b_fps = b.frame_rate.unwrap_or(0.0);
            a_fps > b_fps
        }

        let mut best: Option<makepad_widgets::makepad_platform::video::VideoFormat> = None;

        // Pass 1: NV12 at <= 1080p (preferred)
        for fmt in &desc.formats {
            if fmt.pixel_format != VideoPixelFormat::NV12 {
                continue;
            }
            if fmt.width > 1920 || fmt.height > 1080 {
                continue;
            }
            if best.as_ref().map_or(true, |b| better(fmt, b)) {
                best = Some(*fmt);
            }
        }

        // Pass 2: any NV12
        if best.is_none() {
            for fmt in &desc.formats {
                if fmt.pixel_format != VideoPixelFormat::NV12 {
                    continue;
                }
                if best.as_ref().map_or(true, |b| better(fmt, b)) {
                    best = Some(*fmt);
                }
            }
        }

        // Pass 3: YUY2 or YUV420
        if best.is_none() {
            for fmt in &desc.formats {
                if !matches!(fmt.pixel_format, VideoPixelFormat::YUY2 | VideoPixelFormat::YUV420) {
                    continue;
                }
                if best.as_ref().map_or(true, |b| better(fmt, b)) {
                    best = Some(*fmt);
                }
            }
        }

        // Pass 4: Any format (fallback)
        if best.is_none() {
            log!("No preferred format found, taking first available format...");
            best = desc.formats.first().copied();
        }

        let format = match best {
            Some(f) => f,
            None => {
                log!("No camera format available!");
                return None;
            }
        };
        log!("Selected format: {}x{} {:?}", format.width, format.height, format.pixel_format);

        Some(CameraChoice {
            input_id: desc.input_id,
            format_id: format.format_id,
            name: desc.name.clone(),
            width: format.width,
            height: format.height,
            pixel_format: format.pixel_format,
        })
    }

    /// Start camera for lobby preview
    pub fn start_lobby_camera(ui: &View, cx: &mut Cx, choice: &CameraChoice) -> bool {
        let video = ui.video(cx, &[live_id!(lobby_camera_video)]);

        if !video.is_unprepared() {
            return false;
        }

        log!("Starting lobby camera: {} ({}x{} {:?})",
            choice.name, choice.width, choice.height, choice.pixel_format);

        ui.view(cx, ids!(lobby_video_host)).set_visible(cx, true);
        ui.view(cx, ids!(lobby_camera_placeholder)).set_visible(cx, false);

        video.set_camera_preview_mode(cx, VideoCameraPreviewMode::Native);
        video.set_source_camera(cx, choice.input_id, choice.format_id);
        video.begin_playback(cx);
        true
    }

    /// Start camera for in-call video
    pub fn start_call_camera(ui: &View, cx: &mut Cx, choice: &CameraChoice) -> bool {
        let video = ui.video(cx, &[live_id!(local_camera_video)]);

        if !video.is_unprepared() {
            return false;
        }

        log!("Starting call camera...");

        ui.view(cx, ids!(local_video_host)).set_visible(cx, true);
        ui.view(cx, ids!(local_avatar_view)).set_visible(cx, false);

        video.set_camera_preview_mode(cx, VideoCameraPreviewMode::Native);
        video.set_source_camera(cx, choice.input_id, choice.format_id);
        video.begin_playback(cx);
        true
    }

    /// Stop lobby camera
    pub fn stop_lobby_camera(ui: &View, cx: &mut Cx) {
        let video = ui.video(cx, &[live_id!(lobby_camera_video)]);
        if !video.is_unprepared() && !video.is_cleaning_up() {
            video.stop_and_cleanup_resources(cx);
        }
        ui.view(cx, ids!(lobby_video_host)).set_visible(cx, false);
        ui.view(cx, ids!(lobby_camera_placeholder)).set_visible(cx, true);
        ui.view(cx, ids!(join_call_button_view)).set_visible(cx, true);
    }

    /// Stop in-call camera
    pub fn stop_call_camera(ui: &View, cx: &mut Cx) {
        let video = ui.video(cx, &[live_id!(local_camera_video)]);
        if !video.is_unprepared() && !video.is_cleaning_up() {
            video.stop_and_cleanup_resources(cx);
        }
        ui.view(cx, ids!(local_video_host)).set_visible(cx, false);
        ui.view(cx, ids!(local_avatar_view)).set_visible(cx, true);
    }

    /// Show video view for lobby
    pub fn show_lobby_video(ui: &View, cx: &mut Cx) {
        ui.view(cx, ids!(lobby_video_host)).set_visible(cx, true);
        ui.view(cx, ids!(lobby_camera_placeholder)).set_visible(cx, false);
    }

    /// Show video view for call
    pub fn show_call_video(ui: &View, cx: &mut Cx) {
        ui.view(cx, ids!(local_video_host)).set_visible(cx, true);
        ui.view(cx, ids!(local_avatar_view)).set_visible(cx, false);
    }
}
