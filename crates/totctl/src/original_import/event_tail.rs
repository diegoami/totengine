//! Framing checks for the plain single-player Original save examined by
//! tools/import_event_tail_check.py.

use serde::Serialize;
use tot_kit::{EventFile, Value};

const PREFIX_BYTES: usize = 1853;
const RECORD_BYTES: usize = 276;
// The inspection command targets a stock reference with two records.
// Use a fixed reader budget for other inputs until that scope expands.
const MAX_RECORDS: usize = 64;
const MAX_STRING_BYTES: usize = 255;
// See docs/evidence/import-event-state.json.
const ACTIVE: [u32; 2] = [0x2000_0000, 0x8000_0000];
const SPENT: u32 = 0x2000_0000;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct EventTail<'a> {
    prefix_bytes: usize,
    record_count: usize,
    string_count: usize,
    string_bytes: usize,
    event_section_bytes: usize,
    #[serde(skip)]
    records: &'a [[u8; RECORD_BYTES]],
    #[serde(skip)]
    strings: Vec<(usize, usize, &'a [u8])>,
}

#[derive(Debug, Serialize)]
pub(super) struct VictoryState {
    active: [bool; 2],
    spent: [bool; 2],
}

/// Check extents and retain borrowed records for the definition comparison.
pub(super) fn inspect(tail: &[u8]) -> Result<EventTail<'_>, String> {
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
    let records = records.as_chunks::<RECORD_BYTES>().0;
    let mut slots = Vec::new();
    for (index, record) in records.iter().enumerate() {
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
            slots.push((index, offset, &strings[..end]));
            strings = &strings[end + 1..];
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
        string_count: slots.len(),
        string_bytes,
        event_section_bytes: section.len(),
        records,
        strings: slots,
    })
}

impl EventTail<'_> {
    /// Compare the stock victory definition and expose marks separately.
    /// Other record fields and later loader behavior need separate checks.
    pub(super) fn victory_state(&self, events: &EventFile) -> Result<VictoryState, String> {
        let [event] = events.events.as_slice() else {
            return Err("expected one stock victory block".into());
        };
        let Some(second) = &event.and else {
            return Err("expected the second victory trigger".into());
        };
        let Some(receiver) = event.trigger.param("receiver").and_then(Value::word) else {
            return Err("expected the stock victory receiver".into());
        };
        let [text, endgame, video] = event.actions.as_slice() else {
            return Err("expected the three stock victory actions".into());
        };
        let ([Value::Text(text_body)], [Value::Word(filename)]) =
            (text.args.as_slice(), video.args.as_slice())
        else {
            return Err("expected the stock victory text and video argument".into());
        };
        if event.trigger.keyword != "RECEIVEDTECHNOLOGY"
            || !receiver.eq_ignore_ascii_case("anybody")
            || event.trigger.param("futuretech").and_then(Value::number) != Some(5)
            || second.keyword != "RANDOMTURN"
            || second.param("denominator").and_then(Value::number) != Some(3)
            || text.keyword != "TEXT"
            || endgame.keyword != "ENDGAME"
            || endgame.param("endscreens").and_then(Value::word) != Some("yes")
            || video.keyword != "PLAYAVIFILE"
        {
            return Err("installed event differs from the measured victory rule".into());
        }
        let [first, second] = self.records else {
            return Err("expected two embedded victory records".into());
        };
        let halves = [first, second];
        let mut state = VictoryState {
            active: [false; 2],
            spent: [false; 2],
        };
        for (index, record) in halves.into_iter().enumerate() {
            let word =
                |offset: usize| u32::from_le_bytes(record[offset..offset + 4].try_into().unwrap());
            let action = word(4);
            let modifier = word(8);
            if word(0) != [0x100, 0x40][index]
                || action & !SPENT != 0x5001
                || modifier & !ACTIVE[index] != [0x1000_0108, 0x4000_0008][index]
            {
                return Err(format!(
                    "embedded victory record {index} has different flags"
                ));
            }
            state.active[index] = modifier & ACTIVE[index] != 0;
            state.spent[index] = action & SPENT != 0;
        }
        if first[0x30] != 5
            || first[0x20..0x24] != (-2i32).to_le_bytes()
            || second[0x2c..0x2e] != 3i16.to_le_bytes()
        {
            return Err("embedded victory trigger parameters differ".into());
        }
        let mut expected = vec![(0, 0x18, receiver)];
        for record in 0..2 {
            expected.extend(
                text_body
                    .split('\n')
                    .enumerate()
                    .map(|(i, line)| (record, 0x34 + i * 4, line)),
            );
            expected.push((record, 0x84, filename));
        }
        if self.strings.len() != expected.len() {
            return Err("embedded victory string count differs".into());
        }
        for (&(index, slot, bytes), (expected_index, expected_slot, value)) in
            self.strings.iter().zip(expected)
        {
            if (index, slot) != (expected_index, expected_slot) || tot_kit::latin1(bytes) != value {
                return Err(format!(
                    "embedded victory string differs at record {index}, slot {slot:#x}"
                ));
            }
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_definition_independently_of_saved_marks() {
        let events = tot_kit::parse_events(
            "@IF\nRECEIVEDTECHNOLOGY\nreceiver=anybody\nfuturetech=5\n\
             @AND\nRANDOMTURN\ndenominator=3\n@THEN\nTEXT\nTest message.  \nENDTEXT\n\
             ENDGAME\nendscreens=yes\nPLAYAVIFILE\nFixture.avi\n@ENDIF\n",
        )
        .unwrap();
        let mut tail = vec![0; PREFIX_BYTES];
        tail.extend(b"EVNT");
        tail.extend(2u32.to_le_bytes());
        let start = tail.len();
        tail.resize(start + 2 * RECORD_BYTES, 0);
        let (first, second) = tail[start..].split_at_mut(RECORD_BYTES);
        for (record, trigger, flags) in [
            (&mut *first, 0x100u32, 0x1000_0108u32 | ACTIVE[0]),
            (&mut *second, 0x40u32, 0x4000_0008u32),
        ] {
            record[..4].copy_from_slice(&trigger.to_le_bytes());
            record[4..8].copy_from_slice(&(0x5001 | SPENT).to_le_bytes());
            record[8..12].copy_from_slice(&flags.to_le_bytes());
            record[0x34] = 1;
            record[0x84] = 1;
        }
        first[0x18] = 1;
        first[0x20..0x24].copy_from_slice(&(-2i32).to_le_bytes());
        first[0x30] = 5;
        second[0x2c..0x2e].copy_from_slice(&3i16.to_le_bytes());
        let text_start = tail.len() + b"anybody\0".len();
        tail.extend(b"anybody\0Test message.\0Fixture.avi\0Test message.\0Fixture.avi\0");
        let view = inspect(&tail).unwrap();
        let state = view.victory_state(&events).unwrap();
        assert_eq!(state.active, [true, false]);
        assert_eq!(state.spent, [true, true]);
        assert!(!serde_json::to_string(&view)
            .unwrap()
            .contains("Test message"));

        tail[text_start] = b'B';
        assert_eq!(
            inspect(&tail).unwrap().victory_state(&events).unwrap_err(),
            "embedded victory string differs at record 0, slot 0x34"
        );
        tail[text_start] = b'T';
        tail[start + 0x30] = 6;
        assert_eq!(
            inspect(&tail).unwrap().victory_state(&events).unwrap_err(),
            "embedded victory trigger parameters differ"
        );
    }

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
