//! CRC-32 (IEEE 802.3) used by Sony Bluetooth output reports.

const fn table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}

static TABLE: [u32; 256] = table();

pub fn crc32(parts: &[&[u8]]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for part in parts {
        for &b in *part {
            c = TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
        }
    }
    !c
}

/// Writes the CRC of `seed ‖ report[..len-4]` into the last four bytes.
pub fn seal_bt_report(seed: u8, report: &mut [u8]) {
    let n = report.len() - 4;
    let crc = crc32(&[&[seed], &report[..n]]);
    report[n..].copy_from_slice(&crc.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        assert_eq!(crc32(&[b"123456789"]), 0xCBF4_3926);
        assert_eq!(crc32(&[b"1234", b"56789"]), 0xCBF4_3926);
    }

    #[test]
    fn seal_appends_little_endian() {
        let mut r = [0x31u8, 0, 0, 0, 0, 0];
        seal_bt_report(0xA2, &mut r);
        let expect = crc32(&[&[0xA2], &[0x31, 0]]);
        assert_eq!(u32::from_le_bytes([r[2], r[3], r[4], r[5]]), expect);
    }
}
