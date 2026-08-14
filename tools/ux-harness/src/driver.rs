//! Spawns the headless app and drives it through makepad's stdin/stdout event
//! channel.
//!
//! The headless backend is *pull*-driven: it only advances timers, next-frames
//! and repaints when it receives a `Tick`. It answers with
//! `RequestAnimationFrame` whenever it still has work pending. So the driver's
//! job is to keep ticking until the app goes quiet, then take measurements —
//! that is what makes a run deterministic instead of a race against wall-clock.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use crate::proto::{decode, Incoming, Key, Modifiers, Msg, WidgetSnapshot};

pub struct Driver {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Incoming>,
    pending: VecDeque<Incoming>,
    next_request_id: u64,
    /// Virtual clock handed to the app. Real elapsed time would make runs
    /// unreproducible; a fixed step makes every animation land identically.
    clock: f64,
    frames_dir: PathBuf,
    pub last_focus_rect: Option<(f64, f64, f64, f64)>,
    pub logs: Vec<String>,
    /// Set UX_HARNESS_TRACE=1 to mirror the whole protocol exchange to stderr.
    trace: bool,
}

const TICK_STEP: f64 = 1.0 / 60.0;
/// How long to wait for any single line from the app before declaring it hung.
const LINE_TIMEOUT: Duration = Duration::from_secs(60);
/// How long a Tick may go unanswered before the UI counts as idle. Generous
/// because the headless backend rasterises on the CPU.
const IDLE_WINDOW: Duration = Duration::from_millis(2500);
/// Wall-clock ceiling for any single settle, so a permanently-animating UI
/// (blinking caret) cannot stall a run.
const SETTLE_BUDGET: Duration = Duration::from_secs(8);

impl Driver {
    /// Launch `app_bin` in headless stdin-loop mode with frames landing in
    /// `frames_dir`.
    pub fn launch(app_bin: &Path, frames_dir: &Path, extra_env: &[(String, String)]) -> Result<Driver, String> {
        std::fs::create_dir_all(frames_dir)
            .map_err(|e| format!("cannot create frames dir {}: {e}", frames_dir.display()))?;

        // The headless rasteriser defaults to at most 4 threads to stay polite
        // on a developer's machine. An audit is a batch job — every frame is
        // something the harness is blocked on — so give it the box.
        let threads = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(2).max(1))
            .unwrap_or(4);

        let mut cmd = Command::new(app_bin);
        cmd.arg("--stdin-loop")
            .env("MAKEPAD_HEADLESS_OUT_DIR", frames_dir)
            .env("MAKEPAD_STDIN_LOOP", "1")
            .env("MAKEPAD_HEADLESS_THREADS", threads.to_string())
            // Keep the audit off the developer's real Matrix session/state.
            .env("RUST_BACKTRACE", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in extra_env {
            cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("cannot spawn {}: {e}", app_bin.display()))?;

        let stdin = child.stdin.take().ok_or("no stdin on child")?;
        let stdout = child.stdout.take().ok_or("no stdout on child")?;
        let stderr = child.stderr.take().ok_or("no stderr on child")?;

        let trace = std::env::var("UX_HARNESS_TRACE").is_ok();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if trace {
                    eprintln!("← {}", &line[..line.len().min(220)]);
                }
                if let Some(msg) = decode(&line) {
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
            }
        });
        // Drain stderr so a chatty app can never fill the pipe and deadlock.
        let (etx, erx) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if trace {
                    eprintln!("!! {line}");
                }
                if etx.send(line).is_err() {
                    break;
                }
            }
        });
        thread::spawn(move || {
            // Keep the receiver alive; stderr is only surfaced on failure.
            while erx.recv().is_ok() {}
        });

        Ok(Driver {
            child,
            stdin,
            rx,
            pending: VecDeque::new(),
            next_request_id: 1,
            clock: 0.0,
            frames_dir: frames_dir.to_path_buf(),
            last_focus_rect: None,
            logs: Vec::new(),
            trace: std::env::var("UX_HARNESS_TRACE").is_ok(),
        })
    }

    fn send(&mut self, msg: Msg) -> Result<(), String> {
        let line = msg.to_line();
        if self.trace {
            eprintln!("→ {}", line.trim());
        }
        self.stdin
            .write_all(line.as_bytes())
            .map_err(|e| format!("write to app failed (did it crash?): {e}"))?;
        self.stdin.flush().map_err(|e| format!("flush failed: {e}"))
    }

    fn recv(&mut self) -> Result<Incoming, String> {
        if let Some(msg) = self.pending.pop_front() {
            return Ok(msg);
        }
        match self.rx.recv_timeout(LINE_TIMEOUT) {
            Ok(msg) => {
                if let Incoming::KeyFocusRect { x, y, width, height } = &msg {
                    self.last_focus_rect = match (x, y, width, height) {
                        (Some(x), Some(y), Some(w), Some(h)) => Some((*x, *y, *w, *h)),
                        _ => None,
                    };
                }
                Ok(msg)
            }
            Err(RecvTimeoutError::Timeout) => Err("app went silent (30s)".to_string()),
            Err(RecvTimeoutError::Disconnected) => Err("app exited".to_string()),
        }
    }

    /// Block until the app finishes its startup draw.
    pub fn wait_for_startup(&mut self) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            if Instant::now() > deadline {
                return Err("timed out waiting for AfterStartup".to_string());
            }
            match self.recv()? {
                Incoming::AfterStartup => return Ok(()),
                Incoming::Log(l) => self.logs.push(l),
                _ => {}
            }
        }
    }

    /// Tick until the app stops asking for animation frames, or `max_ticks` is
    /// reached. This is the "let the UI settle" primitive every step uses.
    ///
    /// Returns the number of ticks actually spent, which doubles as a cheap
    /// *responsiveness* measurement: a control that settles in 2 ticks feels
    /// instant, one that churns for 200 does not.
    pub fn settle(&mut self, max_ticks: usize) -> Result<usize, String> {
        let mut ticks = 0;
        let mut quiet_rounds = 0;
        // A UI with a focused text field never truly idles — the caret blink
        // keeps asking for frames forever. Tick budgets alone would then let a
        // single settle run for minutes on the CPU rasteriser, so cap the wall
        // clock too and treat "still animating" as a normal outcome.
        let budget = Instant::now() + SETTLE_BUDGET;
        while ticks < max_ticks && Instant::now() < budget {
            self.clock += TICK_STEP;
            self.send(Msg::Tick)?;
            ticks += 1;

            // One Tick == at most one RequestAnimationFrame, and the app only
            // sends it once the frame is fully rasterised. Rendering a full
            // window on the CPU takes real time, so wait for that answer
            // instead of pipelining more Ticks — queueing them just makes the
            // app render frames nobody asked for.
            let mut wants_more = false;
            let deadline = Instant::now() + IDLE_WINDOW;
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                match self.rx.recv_timeout(remaining) {
                    Ok(Incoming::RequestAnimationFrame) => {
                        wants_more = true;
                        break;
                    }
                    Ok(Incoming::Log(l)) => self.logs.push(l),
                    Ok(Incoming::KeyFocusRect { x, y, width, height }) => {
                        self.last_focus_rect = match (x, y, width, height) {
                            (Some(x), Some(y), Some(w), Some(h)) => Some((x, y, w, h)),
                            _ => None,
                        };
                    }
                    Ok(other) => self.pending.push_back(other),
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return Err("app exited".to_string()),
                }
            }
            if wants_more {
                quiet_rounds = 0;
            } else {
                quiet_rounds += 1;
                if quiet_rounds >= 2 {
                    break;
                }
            }
        }
        Ok(ticks)
    }

    pub fn set_viewport(&mut self, width: f64, height: f64, dpi: f64) -> Result<(), String> {
        self.send(Msg::WindowGeomChange {
            window_id: 0,
            dpi_factor: dpi,
            left: 0.0,
            top: 0.0,
            width,
            height,
        })?;
        self.settle(60)?;
        Ok(())
    }

    pub fn mouse_move(&mut self, x: f64, y: f64) -> Result<(), String> {
        let time = self.clock;
        self.send(Msg::MouseMove { x, y, time, modifiers: Modifiers::default() })?;
        self.settle(60)?;
        Ok(())
    }

    /// A full press/release at a point, returning how many ticks the UI needed
    /// to settle afterwards.
    pub fn click(&mut self, x: f64, y: f64) -> Result<usize, String> {
        let time = self.clock;
        self.send(Msg::MouseMove { x, y, time, modifiers: Modifiers::default() })?;
        self.settle(30)?;
        let time = self.clock;
        self.send(Msg::MouseDown { x, y, time, modifiers: Modifiers::default() })?;
        self.settle(30)?;
        let time = self.clock;
        self.send(Msg::MouseUp { x, y, time, modifiers: Modifiers::default() })?;
        self.settle(24)
    }

    pub fn key(&mut self, key: Key, modifiers: Modifiers) -> Result<usize, String> {
        let time = self.clock;
        self.send(Msg::KeyDown { key, time, modifiers })?;
        self.settle(60)?;
        let time = self.clock;
        self.send(Msg::KeyUp { key, time, modifiers })?;
        self.settle(120)
    }

    pub fn type_text(&mut self, text: &str) -> Result<(), String> {
        self.send(Msg::TextInput { input: text.to_string() })?;
        self.settle(120)?;
        Ok(())
    }

    pub fn scroll(&mut self, x: f64, y: f64, sx: f64, sy: f64) -> Result<usize, String> {
        let time = self.clock;
        self.send(Msg::Scroll { x, y, sx, sy, time })?;
        self.settle(60)
    }

    /// Ask the app for its post-layout widget tree.
    pub fn widget_snapshot(&mut self) -> Result<Vec<WidgetSnapshot>, String> {
        let id = self.next_request_id;
        self.next_request_id += 1;
        self.send(Msg::WidgetSnapshot { request_id: id })?;

        // The app answers this straight out of its message loop — no Tick
        // needed — so just wait for it.
        let mut found = None;
        let mut keep = VecDeque::new();
        while let Some(msg) = self.pending.pop_front() {
            match msg {
                Incoming::WidgetSnapshot { request_id, widgets } if request_id == id => {
                    found = Some(widgets);
                }
                other => keep.push_back(other),
            }
        }
        self.pending = keep;
        if let Some(widgets) = found {
            return Ok(widgets);
        }

        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("no WidgetSnapshot response within 180s".to_string());
            }
            match self.rx.recv_timeout(remaining) {
                Ok(Incoming::WidgetSnapshot { request_id, widgets }) if request_id == id => {
                    return Ok(widgets)
                }
                Ok(Incoming::Log(l)) => self.logs.push(l),
                Ok(other) => self.pending.push_back(other),
                Err(RecvTimeoutError::Timeout) => {
                    return Err("no WidgetSnapshot response within 180s".to_string())
                }
                Err(RecvTimeoutError::Disconnected) => return Err("app exited".to_string()),
            }
        }
    }

    /// Force a repaint and return the newest frame file on disk.
    ///
    /// The headless backend writes every drawn frame to `MAKEPAD_HEADLESS_OUT_DIR`,
    /// so "capture" is really "make it draw, then take the latest file".
    pub fn capture(&mut self, label: &str) -> Result<Option<PathBuf>, String> {
        let id = self.next_request_id;
        self.next_request_id += 1;
        let before = latest_frame(&self.frames_dir);
        self.send(Msg::Screenshot { request_id: id })?;
        self.settle(120)?;

        // The frame is written by the render pass, which lands after the Tick
        // that triggered it — and encoding a full-window PNG on the CPU is not
        // instant. Wait for a file that is actually newer than the one we had,
        // instead of assuming settle() outran the encoder.
        let deadline = Instant::now() + Duration::from_secs(25);
        let mut path = None;
        while Instant::now() < deadline {
            match latest_frame(&self.frames_dir) {
                Some(p) if Some(&p) != before.as_ref() => {
                    path = Some(p);
                    break;
                }
                Some(p) => {
                    path = Some(p);
                    thread::sleep(Duration::from_millis(150));
                }
                None => thread::sleep(Duration::from_millis(150)),
            }
        }
        let Some(path) = path else { return Ok(None) };
        // Give the frame a stable, human-meaningful name next to the raw one.
        let named = self.frames_dir.join(format!("scene_{label}.png"));
        let _ = std::fs::copy(&path, &named);
        Ok(Some(named))
    }

    pub fn shutdown(mut self) {
        let _ = self.send(Msg::Kill);
        let _ = self.stdin.flush();
        thread::sleep(Duration::from_millis(200));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Newest `window_*_frame_*.png` in a directory, by modified time.
pub fn latest_frame(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(dir).ok()? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let name = path.file_name()?.to_string_lossy().to_string();
        if !name.starts_with("window_") || !name.ends_with(".png") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(mtime) = meta.modified() else { continue };
        match &best {
            Some((t, _)) if *t >= mtime => {}
            _ => best = Some((mtime, path)),
        }
    }
    best.map(|(_, p)| p)
}
