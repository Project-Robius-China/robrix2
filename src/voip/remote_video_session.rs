//! Remote video session for displaying LiveKit participant video

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use makepad_widgets::makepad_platform::{
    VideoFrameSession, VideoSessionState,
    MseDecodedFrame,
};
use makepad_widgets::makepad_platform::video_decode::yuv::{YuvPlaneData, YuvLayout, YuvColorMatrix};

/// A video session that receives I420 frames from LiveKit remote participants
pub struct RemoteVideoSession {
    frames: Arc<Mutex<VecDeque<MseDecodedFrame>>>,
    dimensions: Arc<Mutex<Option<(u32, u32)>>>,
    state: Arc<Mutex<VideoSessionState>>,
}

impl RemoteVideoSession {
    pub fn new() -> Self {
        Self {
            frames: Arc::new(Mutex::new(VecDeque::new())),
            dimensions: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(VideoSessionState::Connecting)),
        }
    }

    /// Get a handle to push frames into this session
    pub fn get_pusher(&self) -> RemoteVideoFramePusher {
        RemoteVideoFramePusher {
            frames: self.frames.clone(),
            dimensions: self.dimensions.clone(),
            state: self.state.clone(),
        }
    }
}

impl VideoFrameSession for RemoteVideoSession {
    fn take_frames(&mut self) -> Vec<MseDecodedFrame> {
        let mut frames = self.frames.lock().unwrap();
        frames.drain(..).collect()
    }

    fn dimensions(&self) -> Option<(u32, u32)> {
        *self.dimensions.lock().unwrap()
    }

    fn state(&self) -> VideoSessionState {
        self.state.lock().unwrap().clone()
    }
}

/// Handle for pushing frames into a RemoteVideoSession from another thread
#[derive(Clone)]
pub struct RemoteVideoFramePusher {
    frames: Arc<Mutex<VecDeque<MseDecodedFrame>>>,
    dimensions: Arc<Mutex<Option<(u32, u32)>>>,
    state: Arc<Mutex<VideoSessionState>>,
}

impl RemoteVideoFramePusher {
    /// Push an I420 frame into the session
    pub fn push_i420_frame(&self, y: Vec<u8>, u: Vec<u8>, v: Vec<u8>, width: u32, height: u32, pts_ms: u64) {
        // Update dimensions
        *self.dimensions.lock().unwrap() = Some((width, height));

        // Set state to active
        *self.state.lock().unwrap() = VideoSessionState::Active;

        // Create the decoded frame
        let frame = MseDecodedFrame {
            track_id: 0,
            pts_ms,
            yuv: YuvPlaneData {
                y,
                u,
                v,
                width,
                height,
                layout: YuvLayout::I420,
                matrix: YuvColorMatrix::BT709,
            },
        };

        // Push to queue (limit queue size to avoid memory buildup)
        let mut frames = self.frames.lock().unwrap();
        if frames.len() >= 3 {
            frames.pop_front();
        }
        frames.push_back(frame);
    }

    /// Mark the session as ended
    #[allow(dead_code)]
    pub fn set_ended(&self) {
        *self.state.lock().unwrap() = VideoSessionState::Ended;
    }

    /// Mark the session as errored
    #[allow(dead_code)]
    pub fn set_error(&self, error: String) {
        *self.state.lock().unwrap() = VideoSessionState::Error(error);
    }
}
