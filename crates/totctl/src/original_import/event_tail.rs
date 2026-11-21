//! Framing checks for the plain single-player Original save examined by
//! tools/import_event_tail_check.py.

use serde::Serialize;

const PREFIX_BYTES: usize = 1853;
const RECORD_BYTES: usize = 276;
// The inspection command targets a stock reference with two records.
// Use a fixed reader budget for other inputs until that scope expands.
const MAX_RECORDS: usize = 64;
const MAX_STRING_BYTES: usize = 255;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct EventTail {
    prefix_bytes: usize,
    record_count: usize,
    string_count: usize,
    string_bytes: usize,
    event_section_bytes: usize,
}

/// Check extents without exposing embedded strings or interpreting saved flags.
pub(super) fn inspect(tail: &[u8]) -> Result<EventTail, String> {
    let section = tail
        .get(PREFIX_BYTES..)
        .ok_or("event trailer: truncated post-city prefix")?;
    let data = section
        .strip_prefix(b"EVNT")
        .ok_or("event trailer: expected EVNT at post-city offset 1853")?;
    let (count, data) = data
        .split_first_chunk::<4>()
        .ok_or("event trailer: truncated record count")?;
    let count = u32::from_le_bytes(*count);
    if count > MAX_RECORDS as u32 {
        return Err(format!(
            "event trailer: record count {count} exceeds inspection limit {MAX_RECORDS}"
        ));
    }
    let count = count as usize;
    let (records, mut strings) = data
        .split_at_checked(count * RECORD_BYTES)
        .ok_or("event trailer: truncated event records")?;
    let string_bytes = strings.len();
    let mut string_count = 0;
    for (index, record) in records.as_chunks::<RECORD_BYTES>().0.iter().enumerate() {
        // The native outer reader visits these dword slots in this order
        // (0x461e98 to 0x4622c4). A nonzero word selects a string.
        for offset in (0x0c..=0x18)
            .step_by(4)
            .chain((0x34..=0x58).step_by(4))
            .chain((0x60..=0x84).step_by(4))
        {
            if record[offset..offset + 4] == [0; 4] {
                continue;
            }
            let end = strings
                .iter()
                .take(MAX_STRING_BYTES)
                .position(|&byte| byte == 0)
                .ok_or_else(|| {
                    format!(
                        "event trailer: unterminated string for record {index}, slot {offset:#x}"
                    )
                })?;
            strings = &strings[end + 1..];
            string_count += 1;
        }
    }
    if !strings.is_empty() {
        return Err(format!(
            "event trailer: {} unrecognized bytes after the strings",
            strings.len()
        ));
    }
    Ok(EventTail {
        prefix_bytes: PREFIX_BYTES,
        record_count: count,
        string_count,
        string_bytes,
        event_section_bytes: section.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_truncated_and_unrecognized_tail_data() {
        let mut tail = vec![0; PREFIX_BYTES];
        assert!(inspect(&tail[..PREFIX_BYTES - 1])
            .unwrap_err()
            .contains("prefix"));
        assert!(inspect(&tail).unwrap_err().contains("EVNT"));
        tail.extend(b"EVNT");
        assert!(inspect(&tail).unwrap_err().contains("record count"));
        tail.extend(u32::MAX.to_le_bytes());
        assert!(inspect(&tail).unwrap_err().contains("inspection limit"));
        tail[PREFIX_BYTES + 4..].copy_from_slice(&1u32.to_le_bytes());
        assert!(inspect(&tail)
            .unwrap_err()
            .contains("truncated event records"));
        let start = tail.len();
        tail.resize(start + RECORD_BYTES, 0);
        tail[start + 0x84] = 1;
        tail.extend([b'x'; MAX_STRING_BYTES]);
        tail.push(0);
        assert!(inspect(&tail).unwrap_err().contains("unterminated string"));
        tail.pop();
        tail.pop();
        tail.push(0);
        assert_eq!(inspect(&tail).unwrap().string_count, 1);
        tail.push(0);
        assert!(inspect(&tail).unwrap_err().contains("unrecognized bytes"));
    }
}
