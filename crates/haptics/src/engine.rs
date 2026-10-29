//! Plays rumble programs on a 100 Hz timer thread.

use crate::pattern::RumbleRequest;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(10);
/// Re-send unchanged values this often (some drivers time out).
const KEEPALIVE: Duration = Duration::from_millis(400);

/// Where motor intensities go (the input service in the app).
pub trait RumbleSink: Send + Sync {
    fn set(&self, controller: &str, weak: f32, strong: f32) -> Result<(), String>;
}

/// Current intensity, reported for on-screen feedback.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RumbleState {
    pub weak: f32,
    pub strong: f32,
    pub active: bool,
}

pub type RumbleObserver = Arc<dyn Fn(&str, RumbleState) + Send + Sync>;

enum Cmd {
    Play(String, Vec<RumbleRequest>),
    Stop(String),
    Shutdown,
}

struct Playback {
    program: Vec<RumbleRequest>,
    started: Instant,
    last_sent: (f32, f32),
    last_sent_at: Instant,
}

impl Playback {
    fn total_ms(&self) -> f32 {
        self.program.iter().map(|r| r.duration_ms as f32).sum()
    }

    fn at(&self, t_ms: f32) -> (f32, f32) {
        let mut offset = 0.0;
        for r in &self.program {
            let d = r.duration_ms as f32;
            if t_ms < offset + d {
                return r.intensity_at(t_ms - offset);
            }
            offset += d;
        }
        (0.0, 0.0)
    }
}

pub struct HapticsEngine {
    tx: Sender<Cmd>,
    handle: Option<JoinHandle<()>>,
}

impl HapticsEngine {
    pub fn start(sink: Arc<dyn RumbleSink>, observer: RumbleObserver) -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("gamepadlab-haptics".into())
            .spawn(move || run(rx, sink, observer))
            .expect("spawn haptics thread");
        Self {
            tx,
            handle: Some(handle),
        }
    }

    /// Plays segments back to back, replacing whatever was playing.
    pub fn play(&self, controller: &str, program: Vec<RumbleRequest>) {
        let program = program.into_iter().map(RumbleRequest::sanitized).collect();
        let _ = self.tx.send(Cmd::Play(controller.to_string(), program));
    }

    pub fn stop(&self, controller: &str) {
        let _ = self.tx.send(Cmd::Stop(controller.to_string()));
    }
}

impl Drop for HapticsEngine {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn run(rx: Receiver<Cmd>, sink: Arc<dyn RumbleSink>, observer: RumbleObserver) {
    let mut playing: HashMap<String, Playback> = HashMap::new();
    let stop = |id: &str, sink: &Arc<dyn RumbleSink>, observer: &RumbleObserver| {
        if let Err(e) = sink.set(id, 0.0, 0.0) {
            log::debug!("rumble stop on {id}: {e}");
        }
        observer(
            id,
            RumbleState {
                weak: 0.0,
                strong: 0.0,
                active: false,
            },
        );
    };
    loop {
        let wait = if playing.is_empty() {
            Duration::from_secs(3600)
        } else {
            TICK
        };
        match rx.recv_timeout(wait) {
            Ok(Cmd::Play(id, program)) => {
                let now = Instant::now();
                playing.insert(
                    id,
                    Playback {
                        program,
                        started: now,
                        last_sent: (-1.0, -1.0),
                        last_sent_at: now,
                    },
                );
            }
            Ok(Cmd::Stop(id)) => {
                if playing.remove(&id).is_some() {
                    stop(&id, &sink, &observer);
                }
            }
            Ok(Cmd::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                for id in playing.keys() {
                    stop(id, &sink, &observer);
                }
                return;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }

        let mut finished = Vec::new();
        for (id, p) in playing.iter_mut() {
            let t = p.started.elapsed().as_secs_f32() * 1000.0;
            if t >= p.total_ms() {
                finished.push(id.clone());
                continue;
            }
            let (w, s) = p.at(t);
            let changed = (w - p.last_sent.0).abs() > 0.004 || (s - p.last_sent.1).abs() > 0.004;
            if changed || p.last_sent_at.elapsed() >= KEEPALIVE {
                match sink.set(id, w, s) {
                    Ok(()) => {}
                    Err(e) => {
                        log::warn!("rumble on {id} failed: {e}");
                        finished.push(id.clone());
                        continue;
                    }
                }
                p.last_sent = (w, s);
                p.last_sent_at = Instant::now();
                observer(
                    id,
                    RumbleState {
                        weak: w,
                        strong: s,
                        active: true,
                    },
                );
            }
        }
        for id in finished {
            playing.remove(&id);
            stop(&id, &sink, &observer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::{motor_test_program, PatternKind};
    use std::sync::Mutex;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<(String, f32, f32)>>);

    impl RumbleSink for Recorder {
        fn set(&self, c: &str, w: f32, s: f32) -> Result<(), String> {
            self.0.lock().unwrap().push((c.into(), w, s));
            Ok(())
        }
    }

    struct Failing;
    impl RumbleSink for Failing {
        fn set(&self, _: &str, _: f32, _: f32) -> Result<(), String> {
            Err("unsupported".into())
        }
    }

    fn no_observer() -> RumbleObserver {
        Arc::new(|_, _| {})
    }

    #[test]
    fn plays_then_stops_with_zero() {
        let rec = Arc::new(Recorder::default());
        let engine = HapticsEngine::start(rec.clone(), no_observer());
        engine.play("pad", vec![RumbleRequest::constant(0.3, 0.6, 60)]);
        std::thread::sleep(Duration::from_millis(200));
        let log = rec.0.lock().unwrap().clone();
        assert!(log
            .iter()
            .any(|(_, w, s)| (*w - 0.3).abs() < 1e-6 && (*s - 0.6).abs() < 1e-6));
        let last = log.last().unwrap();
        assert_eq!((last.1, last.2), (0.0, 0.0), "always ends silent");
    }

    #[test]
    fn stop_interrupts() {
        let rec = Arc::new(Recorder::default());
        let engine = HapticsEngine::start(rec.clone(), no_observer());
        engine.play("pad", vec![RumbleRequest::constant(1.0, 1.0, 5000)]);
        std::thread::sleep(Duration::from_millis(40));
        engine.stop("pad");
        std::thread::sleep(Duration::from_millis(40));
        let log = rec.0.lock().unwrap().clone();
        assert_eq!(log.last().map(|l| (l.1, l.2)), Some((0.0, 0.0)));
    }

    #[test]
    fn observer_sees_active_then_idle() {
        let seen = Arc::new(Mutex::new(Vec::<RumbleState>::new()));
        let s2 = seen.clone();
        let engine = HapticsEngine::start(
            Arc::new(Recorder::default()),
            Arc::new(move |_, st| s2.lock().unwrap().push(st)),
        );
        engine.play(
            "pad",
            vec![RumbleRequest {
                weak: 1.0,
                strong: 0.0,
                duration_ms: 80,
                pattern: PatternKind::RampDown,
                period_ms: 100,
            }],
        );
        std::thread::sleep(Duration::from_millis(250));
        let seen = seen.lock().unwrap();
        assert!(seen.iter().any(|s| s.active && s.weak > 0.3));
        assert!(!seen.last().unwrap().active);
    }

    #[test]
    fn sink_failure_ends_playback() {
        let seen = Arc::new(Mutex::new(Vec::<RumbleState>::new()));
        let s2 = seen.clone();
        let engine = HapticsEngine::start(
            Arc::new(Failing),
            Arc::new(move |_, st| s2.lock().unwrap().push(st)),
        );
        engine.play("pad", motor_test_program(1.0));
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(seen.lock().unwrap().last().map(|s| s.active), Some(false));
    }

    #[test]
    fn program_segments_in_order() {
        let p = Playback {
            program: motor_test_program(1.0),
            started: Instant::now(),
            last_sent: (0.0, 0.0),
            last_sent_at: Instant::now(),
        };
        assert_eq!(p.at(500.0), (1.0, 0.0));
        assert_eq!(p.at(1200.0), (0.0, 0.0));
        assert_eq!(p.at(1900.0), (0.0, 1.0));
        assert_eq!(p.at(3300.0), (1.0, 1.0));
        assert_eq!(p.total_ms(), 3800.0);
    }
}
