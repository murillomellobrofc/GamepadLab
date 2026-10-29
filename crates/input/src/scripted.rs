//! Scripted mock backend: tests push fake events and inspect outputs, so
//! the whole service can run in CI without a physical controller.

use crate::backend::{BackendEvent, Clock, GamepadBackend, InputError, OutputCommand};
use crate::model::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Inner {
    queue: VecDeque<BackendEvent>,
    sent: Vec<(ControllerId, OutputCommand)>,
    connected: Vec<ControllerId>,
}

/// Handle kept by the test to drive a `ScriptedBackend`.
#[derive(Clone, Default)]
pub struct ScriptHandle(Arc<Mutex<Inner>>);

impl ScriptHandle {
    pub fn push(&self, ev: BackendEvent) {
        self.0.lock().unwrap().queue.push_back(ev);
    }

    pub fn connect(&self, info: ControllerInfo) {
        self.push(BackendEvent::Connected(info));
    }

    pub fn disconnect(&self, id: &ControllerId) {
        self.push(BackendEvent::Disconnected(id.clone()));
    }

    pub fn state(&self, id: &ControllerId, st: DeviceState) {
        self.push(BackendEvent::State(id.clone(), st));
    }

    pub fn sent(&self) -> Vec<(ControllerId, OutputCommand)> {
        self.0.lock().unwrap().sent.clone()
    }
}

pub struct ScriptedBackend {
    name: &'static str,
    priority: u8,
    handle: ScriptHandle,
}

impl ScriptedBackend {
    pub fn new(name: &'static str, priority: u8) -> (Self, ScriptHandle) {
        let handle = ScriptHandle::default();
        (
            Self {
                name,
                priority,
                handle: handle.clone(),
            },
            handle,
        )
    }

    /// A plausible controller description for tests.
    pub fn controller(id: &str, vid: u16, pid: u16, backend: &str) -> ControllerInfo {
        ControllerInfo {
            id: ControllerId::new(id),
            name: format!("Test pad {id}"),
            vendor_id: vid,
            product_id: pid,
            serial: None,
            kind: crate::mapping::detect_kind(vid, pid, ""),
            connection: ConnectionType::Usb,
            button_count: 17,
            axis_count: 6,
            capabilities: Capabilities {
                rumble: true,
                ..Capabilities::default()
            },
            battery: None,
            backend: backend.into(),
            is_virtual: false,
            is_demo: false,
            profile_key: ControllerInfo::make_profile_key(vid, pid, None),
        }
    }
}

impl GamepadBackend for ScriptedBackend {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn poll(&mut self, _clock: &Clock, out: &mut Vec<BackendEvent>) {
        let mut inner = self.handle.0.lock().unwrap();
        while let Some(ev) = inner.queue.pop_front() {
            match &ev {
                BackendEvent::Connected(info) => inner.connected.push(info.id.clone()),
                BackendEvent::Disconnected(id) => inner.connected.retain(|c| c != id),
                _ => {}
            }
            out.push(ev);
        }
    }

    fn send(&mut self, id: &ControllerId, cmd: &OutputCommand) -> Result<(), InputError> {
        let mut inner = self.handle.0.lock().unwrap();
        if !inner.connected.contains(id) {
            return Err(InputError::Disconnected(id.to_string()));
        }
        inner.sent.push((id.clone(), cmd.clone()));
        Ok(())
    }
}
