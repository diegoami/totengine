//! Original turn-to-year conversion (`0x46da89`), including scenario dates.
//! Native comparisons and the decoded table: `tools/calendar_check.py`.
use tot_kit::CalendarSettings;

// (base year, span in turns, years per turn), original 0x5d4fa8.
const TABLE: [[(i32, i32, i32); 6]; 4] = [
    [
        (-4000, 250, 20),
        (1000, 50, 10),
        (1500, 50, 5),
        (1750, 50, 2),
        (1850, 50, 1),
        (1900, 100, 1),
    ],
    [
        (-4000, 60, 50),
        (-1000, 40, 25),
        (0, 150, 10),
        (1500, 50, 5),
        (1750, 50, 2),
        (1850, 50, 1),
    ],
    [
        (-4000, 60, 50),
        (-1000, 40, 25),
        (0, 50, 20),
        (1000, 50, 10),
        (1500, 50, 5),
        (1750, 50, 2),
    ],
    [
        (-4000, 60, 50),
        (-1000, 40, 25),
        (0, 75, 20),
        (1500, 25, 10),
        (1750, 50, 2),
        (1850, 50, 1),
    ],
];

pub fn year(turn: i16, difficulty: u8, flags: u16, settings: CalendarSettings) -> i32 {
    let mut turn = turn as i32;
    let mut elapsed = 0;
    if settings.turn_limit >= 0 {
        elapsed = (turn - settings.turn_limit as i32).max(0);
        turn -= elapsed;
    }
    if flags & 128 != 0 && settings.increment != 0 && settings.start_year != 0 {
        return turn * (settings.increment as i32).abs() + settings.start_year as i32 - 1 + elapsed;
    }
    let adjusted = difficulty as i32 - i32::from(flags & 4 != 0) + i32::from(flags & 8 != 0);
    let row = &TABLE[(adjusted - 1).clamp(0, 3) as usize];
    if turn != 0 {
        turn -= 1;
    }
    for &(base, span, step) in row {
        if turn <= span {
            let year = base + step * turn;
            return if year == 0 {
                1 + elapsed
            } else {
                year + elapsed
            };
        }
        turn -= span;
    }
    let (base, span, step) = row[5];
    let year = base + span * step + turn;
    if year == 0 {
        1 + elapsed
    } else {
        year + elapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_calendar_vectors() {
        let mut count = 0;
        for line in include_str!("../../../docs/evidence/calendar-vectors.txt")
            .lines()
            .filter(|l| !l.starts_with('#'))
        {
            let v: Vec<i32> = line
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            assert_eq!(
                year(
                    v[0] as i16,
                    v[1] as u8,
                    v[2] as u16,
                    CalendarSettings {
                        turn_limit: v[3] as i16,
                        increment: v[4] as i16,
                        start_year: v[5] as i16,
                    }
                ),
                v[6],
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 324);
    }
}
