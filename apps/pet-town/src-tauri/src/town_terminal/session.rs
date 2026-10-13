use super::{events, input, stream, TerminalEvent};
use pet_town_agent_broker::TerminalTarget;
use std::process::{Child, ChildStdin};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tauri::ipc::Channel;

pub(crate) struct Session {
    pub token: String,
    pub target: TerminalTarget,
    pub channel: Channel<TerminalEvent>,
    pub control: bool,
    pub cancelled: AtomicBool,
    pub ready: AtomicBool,
    child: Mutex<Child>,
    writer: Mutex<Option<ChildStdin>>,
    last_verify: Mutex<Instant>,
}

impl Session {
    pub fn start(
        target: TerminalTarget,
        control: bool,
        takeover: bool,
        cols: u16,
        rows: u16,
        channel: Channel<TerminalEvent>,
    ) -> Result<Arc<Self>, String> {
        target.validate_mode(control, takeover)?;
        // The caller validates the target, then serializes spawn/installation with release.
        let mut child = target
            .stream_command(control, takeover, cols, rows)?
            .spawn()
            .map_err(|_| "Herdr could not open this terminal view. Check its installed version.")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Herdr terminal output is unavailable.")?;
        let stderr = child
            .stderr
            .take()
            .ok_or("Herdr terminal diagnostics are unavailable.")?;
        let writer = child.stdin.take();
        if let Some(stdin) = &writer {
            if let Err(error) = input::nonblocking(stdin) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        }
        let session = Arc::new(Self {
            token: uuid::Uuid::new_v4().to_string(),
            target,
            channel,
            control,
            cancelled: AtomicBool::new(false),
            ready: AtomicBool::new(false),
            child: Mutex::new(child),
            writer: Mutex::new(writer),
            // The opener already verified the target and a watchdog re-verifies
            // every second, so per-keystroke checks would only add spawn latency.
            last_verify: Mutex::new(Instant::now()),
        });
        events::status(
            &session.channel,
            "connecting",
            false,
            "Connecting to the existing Herdr terminal…",
        );
        stream::start(session.clone(), stdout, stderr);
        Ok(session)
    }

    pub fn send(&self, command: input::TerminalCommand) -> Result<(), String> {
        let payload = command.payload()?;
        if !self.control
            || self.cancelled.load(Ordering::SeqCst)
            || !self.ready.load(Ordering::SeqCst)
        {
            return Err("Terminal input is off. Reconnect or choose Take control.".into());
        }
        if self.should_verify() {
            if let Err(error) = self.target.verify() {
                self.finish("stale", &error);
                return Err(error);
            }
            *self
                .last_verify
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = Instant::now();
        }
        if self.cancelled.load(Ordering::SeqCst) {
            return Err("Terminal view is closed.".into());
        }
        if matches!(command, input::TerminalCommand::Live {}) {
            return self.target.return_to_live();
        }
        let result = {
            let mut writer = self
                .writer
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if self.cancelled.load(Ordering::SeqCst) {
                return Err("Terminal view is closed.".into());
            }
            let stdin = writer
                .as_mut()
                .ok_or("Terminal input is no longer available.")?;
            input::write_bounded(stdin, &payload)
        };
        if let Err(error) = &result {
            self.finish("disconnected", error);
        }
        result
    }

    fn should_verify(&self) -> bool {
        self.last_verify
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .elapsed()
            >= Duration::from_secs(5)
    }

    pub fn finish(&self, state: &str, message: &str) {
        if !self.cancelled.load(Ordering::SeqCst) {
            events::status(&self.channel, state, false, message);
        }
        self.stop();
    }

    pub fn stop(&self) {
        if self.cancelled.swap(true, Ordering::SeqCst) {
            return;
        }
        self.ready.store(false, Ordering::SeqCst);
        let mut child = self.child.lock().unwrap_or_else(|error| error.into_inner());
        // This is only the owned CLI viewer. Never signal the agent or its PTY.
        // Its exit closes the socket and releases ownership without waiting on input.
        let _ = child.kill();
        let _ = child.wait();
        if let Ok(mut writer) = self.writer.try_lock() {
            writer.take();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}
