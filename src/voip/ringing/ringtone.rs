//! Ringtone playback.
//!
//! On desktop (macOS / Linux), uses [`rodio`] to play looping audio
//! through the default output device. On iOS / Android / Windows the
//! ringtone is a no-op — the platform doesn't support voice calls in
//! this version of the app, but the ringing types still need to compile
//! and the public API must not vary across platforms.
//!
//! Audio assets live under `resources/sounds/`:
//! * `ring_in.ogg`  — looping incoming-call tone (played to the callee).
//! * `ring_out.ogg` — looping outgoing dial tone (played to the caller).
//!
//! ## Ordering invariant
//!
//! The ringtone shares the system audio output device with LiveKit's
//! capture / playback pipeline. The 1:1 call orchestrator is responsible
//! for calling [`RingtonePlayer::stop`] *before* asking LiveKit to
//! connect (or publish the local microphone track) — otherwise on some
//! Linux configurations the device transition can stutter or fail.

use std::sync::mpsc::{self as std_mpsc, Sender};

use makepad_widgets::log;

/// Commands that drive the ringtone audio thread.
#[derive(Clone, Copy, Debug)]
pub enum RingtoneCmd {
    /// Start looping the incoming-call tone. Idempotent: if a tone is
    /// already playing it will be replaced.
    PlayIncoming,
    /// Start looping the outgoing dial tone. Idempotent.
    PlayOutgoing,
    /// Stop any currently-playing tone.
    Stop,
}

/// Handle to the ringtone player. Cheap to clone (only contains a
/// channel sender). Drop the handle to leave the audio thread alive
/// (it shuts down when the last sender is dropped).
#[derive(Clone)]
pub struct RingtonePlayer {
    cmd_tx: Sender<RingtoneCmd>,
}

impl RingtonePlayer {
    /// Spawn the audio worker thread and return a handle. Call this
    /// once at app startup. Subsequent calls create independent audio
    /// threads, which is rarely what you want.
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = std_mpsc::channel::<RingtoneCmd>();
        std::thread::Builder::new()
            .name("voip-ringtone".to_string())
            .spawn(move || {
                run_audio_worker(cmd_rx);
            })
            .expect("failed to spawn voip-ringtone thread");
        Self { cmd_tx }
    }

    pub fn play_incoming(&self) {
        let _ = self.cmd_tx.send(RingtoneCmd::PlayIncoming);
    }

    pub fn play_outgoing(&self) {
        let _ = self.cmd_tx.send(RingtoneCmd::PlayOutgoing);
    }

    pub fn stop(&self) {
        let _ = self.cmd_tx.send(RingtoneCmd::Stop);
    }
}

// =====================================================================
// Desktop backend (rodio).
// =====================================================================

#[cfg(not(any(target_os = "android", target_os = "ios", target_os = "windows")))]
fn run_audio_worker(cmd_rx: std_mpsc::Receiver<RingtoneCmd>) {
    use std::io::Cursor;
    use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

    // Bundled audio assets, embedded directly into the binary so they're
    // always available regardless of the working directory or packaging.
    // These files must exist under resources/sounds/ — round 1 creates
    // placeholder zero-byte files; round 3 wires up the real assets.
    const RING_IN_BYTES: &[u8] = include_bytes!("../../../resources/sounds/ring_in.ogg");
    const RING_OUT_BYTES: &[u8] = include_bytes!("../../../resources/sounds/ring_out.ogg");

    fn try_open() -> Option<(OutputStream, OutputStreamHandle)> {
        match OutputStream::try_default() {
            Ok(pair) => Some(pair),
            Err(err) => {
                log!("RingtonePlayer: failed to open audio output: {}", err);
                None
            }
        }
    }

    fn play_loop(handle: &OutputStreamHandle, bytes: &'static [u8]) -> Option<Sink> {
        let sink = Sink::try_new(handle).ok()?;
        let source = Decoder::new(Cursor::new(bytes)).ok()?.repeat_infinite();
        sink.append(source);
        Some(sink)
    }

    // We keep the `_stream` alive for the duration of playback (rodio
    // requires this), and recreate it lazily on the first play request
    // so failure to open the device doesn't crash the worker at boot.
    let mut stream_pair: Option<(OutputStream, OutputStreamHandle)> = None;
    let mut current_sink: Option<Sink> = None;

    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            RingtoneCmd::Stop => {
                if let Some(s) = current_sink.take() { s.stop() }
            }
            RingtoneCmd::PlayIncoming | RingtoneCmd::PlayOutgoing => {
                // Stop any in-flight tone before starting the next one.
                if let Some(s) = current_sink.take() { s.stop() }

                if stream_pair.is_none() {
                    stream_pair = try_open();
                }
                let Some((_, ref handle)) = stream_pair else { continue; };

                let bytes = match cmd {
                    RingtoneCmd::PlayIncoming => RING_IN_BYTES,
                    RingtoneCmd::PlayOutgoing => RING_OUT_BYTES,
                    RingtoneCmd::Stop => unreachable!(),
                };
                if bytes.is_empty() {
                    // Round 1: placeholder asset files are empty. Skip
                    // playback rather than failing the decoder.
                    log!("RingtonePlayer: ringtone asset is empty; skipping playback");
                    continue;
                }
                current_sink = play_loop(handle, bytes);
            }
        }
    }
}

// =====================================================================
// Stub backend (iOS / Android / Windows).
// =====================================================================

#[cfg(any(target_os = "android", target_os = "ios", target_os = "windows"))]
fn run_audio_worker(cmd_rx: std_mpsc::Receiver<RingtoneCmd>) {
    // Drain commands silently; voice calls aren't supported here.
    while cmd_rx.recv().is_ok() {}
}
