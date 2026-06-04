//! Audio level detection and speaking indicator

use makepad_widgets::*;
use makepad_widgets::View;
use makepad_widgets::makepad_platform::audio::{AudioDeviceId, AudioDevicesEvent};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Threshold for speaking detection (RMS level)
const SPEAKING_THRESHOLD: f32 = 0.01;

/// Speaking detector handles audio level monitoring
pub struct SpeakingDetector {
    pub audio_device: Option<AudioDeviceId>,
    pub audio_level: Arc<AtomicU32>,
    pub is_speaking: bool,
}

impl Default for SpeakingDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeakingDetector {
    pub fn new() -> Self {
        Self {
            audio_device: None,
            audio_level: Arc::new(AtomicU32::new(0)),
            is_speaking: false,
        }
    }

    /// Handle audio devices event and start monitoring
    pub fn handle_audio_devices(&mut self, cx: &mut Cx, ev: &AudioDevicesEvent) {
        log!("AudioDevices event: {} devices found", ev.descs.len());

        let inputs = ev.default_input();
        if let Some(device_id) = inputs.first() {
            log!("Using audio input device: {:?}", device_id);
            self.audio_device = Some(*device_id);
            cx.use_audio_inputs(&[*device_id]);

            let audio_level = self.audio_level.clone();
            cx.audio_input(0, move |_info, buffer| {
                let rms = Self::calculate_rms(&buffer.data);
                audio_level.store(rms.to_bits(), Ordering::Relaxed);
            });
        }
    }

    /// Calculate RMS (root mean square) of audio samples
    fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let mut sum = 0.0f32;
        for sample in samples {
            sum += sample * sample;
        }
        (sum / samples.len() as f32).sqrt()
    }

    /// Get current audio level
    pub fn get_level(&self) -> f32 {
        let level_bits = self.audio_level.load(Ordering::Relaxed);
        f32::from_bits(level_bits)
    }

    /// Check if user is currently speaking (level above threshold)
    pub fn check_speaking(&mut self, is_muted: bool) -> bool {
        if is_muted {
            self.is_speaking = false;
            return false;
        }

        let level = self.get_level();
        let was_speaking = self.is_speaking;
        self.is_speaking = level > SPEAKING_THRESHOLD;

        was_speaking != self.is_speaking
    }

    /// Update the speaking indicator in the UI
    pub fn update_indicator(ui: &View, cx: &mut Cx, is_speaking: bool) {
        ui.view(cx, ids!(local_speaking_border)).set_visible(cx, is_speaking);
    }
}
