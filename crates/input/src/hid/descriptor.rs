//! Minimal HID report-descriptor parser: byte length of each feature report,
//! so feature requests go out with exactly the size the device declares
//! (Windows rejects anything else).

use std::collections::HashMap;

/// Feature report id → total length in bytes, including the report-id byte.
pub fn feature_report_lengths(desc: &[u8]) -> HashMap<u8, usize> {
    let mut bits: HashMap<u8, usize> = HashMap::new();
    let mut report_size = 0usize;
    let mut report_count = 0usize;
    let mut report_id = 0u8;
    let mut stack: Vec<(usize, usize, u8)> = Vec::new();
    let mut i = 0;
    while i < desc.len() {
        let prefix = desc[i];
        if prefix == 0xFE {
            // Long item: skip it.
            let size = *desc.get(i + 1).unwrap_or(&0) as usize;
            i += 3 + size;
            continue;
        }
        let size = match prefix & 0x03 {
            3 => 4,
            n => n as usize,
        };
        let kind = (prefix >> 2) & 0x03;
        let tag = prefix >> 4;
        let mut value = 0u32;
        for k in 0..size {
            value |= (*desc.get(i + 1 + k).unwrap_or(&0) as u32) << (8 * k);
        }
        match (kind, tag) {
            (1, 0x7) => report_size = value as usize,
            (1, 0x9) => report_count = value as usize,
            (1, 0x8) => report_id = value as u8,
            (1, 0xA) => stack.push((report_size, report_count, report_id)),
            (1, 0xB) => {
                if let Some((s, c, id)) = stack.pop() {
                    report_size = s;
                    report_count = c;
                    report_id = id;
                }
            }
            (0, 0xB) => *bits.entry(report_id).or_default() += report_size * report_count,
            _ => {}
        }
        i += 1 + size;
    }
    bits.into_iter()
        .map(|(id, b)| (id, b.div_ceil(8) + usize::from(id != 0)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_per_feature_report() {
        let desc = [
            0x06, 0x00, 0xFF, // Usage Page (vendor)
            0x09, 0x01, // Usage
            0xA1, 0x01, // Collection
            0x85, 0x80, // Report ID 0x80
            0x75, 0x08, // Report Size 8
            0x95, 0x3F, // Report Count 63
            0x09, 0x02, // Usage
            0xB1, 0x02, // Feature
            0x85, 0x91, // Report ID 0x91
            0x95, 0x03, // Report Count 3
            0xB1, 0x02, // Feature
            0xA4, // Push
            0x85, 0x92, //   Report ID 0x92
            0x75, 0x01, //   Report Size 1
            0x95, 0x0C, //   Report Count 12 → 12 bits → 2 bytes
            0xB1, 0x02, //   Feature
            0xB4, // Pop
            0x85, 0x01, // Report ID 1
            0x81, 0x02, // Input (ignored)
            0xC0, // End collection
        ];
        let l = feature_report_lengths(&desc);
        assert_eq!(l.get(&0x80), Some(&64));
        assert_eq!(l.get(&0x91), Some(&4));
        assert_eq!(l.get(&0x92), Some(&3));
        assert_eq!(l.get(&0x01), None, "input reports are not features");
    }

    #[test]
    fn tolerates_truncated_and_long_items() {
        let l = feature_report_lengths(&[0xFE, 0x02, 0x10, 0xAA, 0xBB, 0x85]);
        assert!(l.is_empty());
    }
}
