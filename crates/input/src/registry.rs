//! Merges devices from several backends and hides duplicates.
//!
//! The same physical controller can show up twice: through gilrs (OS gamepad
//! API) and through raw HID, or as XInput + DirectInput on Windows. Devices
//! sharing VID:PID are paired, and only the one from the highest-priority
//! backend stays visible.

use crate::model::{ControllerId, ControllerInfo};

#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Shown(ControllerInfo),
    Hidden(ControllerInfo),
}

#[derive(Debug, Clone)]
struct Entry {
    info: ControllerInfo,
    priority: u8,
    visible: bool,
    order: u64,
}

#[derive(Debug, Default)]
pub struct Registry {
    entries: Vec<Entry>,
    next_order: u64,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, info: ControllerInfo, priority: u8) -> Vec<Change> {
        if let Some(e) = self.entries.iter_mut().find(|e| e.info.id == info.id) {
            e.info = info;
            return Vec::new();
        }
        self.entries.push(Entry {
            info,
            priority,
            visible: false,
            order: self.next_order,
        });
        self.next_order += 1;
        self.recompute()
    }

    pub fn remove(&mut self, id: &ControllerId) -> Vec<Change> {
        let Some(pos) = self.entries.iter().position(|e| &e.info.id == id) else {
            return Vec::new();
        };
        let removed = self.entries.remove(pos);
        let mut changes = Vec::new();
        if removed.visible {
            changes.push(Change::Hidden(removed.info));
        }
        changes.extend(self.recompute());
        changes
    }

    /// Replaces metadata; returns the new info when the device is visible.
    pub fn update(&mut self, info: ControllerInfo) -> Option<ControllerInfo> {
        let e = self.entries.iter_mut().find(|e| e.info.id == info.id)?;
        e.info = info;
        e.visible.then(|| e.info.clone())
    }

    pub fn is_visible(&self, id: &ControllerId) -> bool {
        self.entries.iter().any(|e| &e.info.id == id && e.visible)
    }

    pub fn get(&self, id: &ControllerId) -> Option<&ControllerInfo> {
        self.entries
            .iter()
            .find(|e| &e.info.id == id)
            .map(|e| &e.info)
    }

    pub fn visible(&self) -> Vec<ControllerInfo> {
        let mut v: Vec<&Entry> = self.entries.iter().filter(|e| e.visible).collect();
        v.sort_by_key(|e| e.order);
        v.into_iter().map(|e| e.info.clone()).collect()
    }

    pub fn all(&self) -> impl Iterator<Item = &ControllerInfo> {
        self.entries.iter().map(|e| &e.info)
    }

    fn recompute(&mut self) -> Vec<Change> {
        let mut wanted = vec![true; self.entries.len()];
        let mut groups: Vec<(u16, u16)> = self
            .entries
            .iter()
            .filter(|e| dedupable(&e.info))
            .map(|e| (e.info.vendor_id, e.info.product_id))
            .collect();
        groups.sort_unstable();
        groups.dedup();

        for (vid, pid) in groups {
            let mut members: Vec<usize> = (0..self.entries.len())
                .filter(|&i| {
                    let info = &self.entries[i].info;
                    dedupable(info) && info.vendor_id == vid && info.product_id == pid
                })
                .collect();
            let top = members
                .iter()
                .map(|&i| self.entries[i].priority)
                .max()
                .unwrap_or(0);
            let mut budget = members
                .iter()
                .filter(|&&i| self.entries[i].priority == top)
                .count();
            members.sort_by_key(|&i| self.entries[i].order);
            for i in members {
                if self.entries[i].priority < top && budget > 0 {
                    wanted[i] = false;
                    budget -= 1;
                }
            }
        }

        let mut changes = Vec::new();
        for (e, want) in self.entries.iter_mut().zip(wanted) {
            if e.visible != want {
                e.visible = want;
                changes.push(if want {
                    Change::Shown(e.info.clone())
                } else {
                    Change::Hidden(e.info.clone())
                });
            }
        }
        // Hide before show so the UI never briefly lists both copies.
        changes.sort_by_key(|c| matches!(c, Change::Shown(_)));
        changes
    }
}

fn dedupable(info: &ControllerInfo) -> bool {
    !info.is_demo && !info.is_virtual && (info.vendor_id != 0 || info.product_id != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Capabilities, ConnectionType, ControllerKind};

    fn info(id: &str, vid: u16, pid: u16, backend: &str) -> ControllerInfo {
        ControllerInfo {
            id: ControllerId::new(id),
            name: id.into(),
            vendor_id: vid,
            product_id: pid,
            serial: None,
            kind: ControllerKind::Generic,
            connection: ConnectionType::Unknown,
            button_count: 0,
            axis_count: 0,
            capabilities: Capabilities::default(),
            battery: None,
            backend: backend.into(),
            is_virtual: false,
            is_demo: false,
            profile_key: format!("{vid:04x}:{pid:04x}"),
        }
    }

    fn ids(changes: &[Change]) -> Vec<String> {
        changes
            .iter()
            .map(|c| match c {
                Change::Shown(i) => format!("+{}", i.id),
                Change::Hidden(i) => format!("-{}", i.id),
            })
            .collect()
    }

    #[test]
    fn hid_copy_hides_gilrs_copy() {
        let mut r = Registry::new();
        assert_eq!(ids(&r.add(info("g1", 0x054c, 0x0ce6, "gilrs"), 1)), ["+g1"]);
        assert_eq!(
            ids(&r.add(info("h1", 0x054c, 0x0ce6, "hid"), 2)),
            ["-g1", "+h1"]
        );
        assert_eq!(r.visible().len(), 1);
        assert!(r.is_visible(&ControllerId::new("h1")));
        // Unplugging the HID side brings the gilrs copy back.
        assert_eq!(ids(&r.remove(&ControllerId::new("h1"))), ["-h1", "+g1"]);
    }

    #[test]
    fn pairs_one_to_one() {
        let mut r = Registry::new();
        r.add(info("g1", 0x054c, 0x0ce6, "gilrs"), 1);
        r.add(info("g2", 0x054c, 0x0ce6, "gilrs"), 1);
        r.add(info("h1", 0x054c, 0x0ce6, "hid"), 2);
        let visible: Vec<String> = r.visible().into_iter().map(|i| i.id.0).collect();
        assert_eq!(visible, ["g2", "h1"]);
    }

    #[test]
    fn different_devices_are_untouched() {
        let mut r = Registry::new();
        r.add(info("xbox", 0x045e, 0x0b13, "gilrs"), 1);
        r.add(info("ds", 0x054c, 0x0ce6, "hid"), 2);
        assert_eq!(r.visible().len(), 2);
    }

    #[test]
    fn demo_and_virtual_are_never_hidden() {
        let mut r = Registry::new();
        let mut demo = info("demo", 0x054c, 0x0ce6, "demo");
        demo.is_demo = true;
        r.add(demo, 0);
        r.add(info("h1", 0x054c, 0x0ce6, "hid"), 2);
        let mut steam = info("steam", 0x28de, 0x11ff, "gilrs");
        steam.is_virtual = true;
        r.add(steam, 1);
        assert_eq!(r.visible().len(), 3);
    }

    #[test]
    fn re_adding_same_id_only_updates() {
        let mut r = Registry::new();
        r.add(info("g1", 1, 2, "gilrs"), 1);
        assert!(r.add(info("g1", 1, 2, "gilrs"), 1).is_empty());
        assert_eq!(r.visible().len(), 1);
    }
}
