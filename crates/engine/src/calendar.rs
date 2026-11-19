//! Original turn-to-year conversion (`0x46da89`), including scenario dates.
//! Native comparisons and the decoded table: `tools/calendar_check.py`.
//! [`date_text`] is the original's date formatter (`0x46dca5`).
use tot_kit::{CalendarSettings, Labels, MONTH_LABEL};

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

/// How a date number reads (`0x46dca5`): scenario flag 0x80 with a negative
/// calendar increment counts months, everything else counts years.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateStyle {
    Years,
    Months,
}

impl DateStyle {
    /// The style for these settings; `None` when the date cannot be read
    /// (a scenario whose calendar parameters are not located, 0x33 files).
    pub fn of(flags: u16, settings: Option<CalendarSettings>) -> Option<Self> {
        if flags & 0x80 == 0 {
            return Some(Self::Years);
        }
        let settings = settings?;
        Some(if settings.increment < 0 {
            Self::Months
        } else {
            Self::Years
        })
    }
}

/// English label fallbacks: the standard `Labels.txt` values.
const ERA: [&str; 2] = ["B.C.", "A.D."];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "June", "July", "Aug", "Sept", "Oct", "Nov", "Dec",
];

/// The date as the original writes it (`0x46dca5`, English layout:
/// language preference 0 at `0x5d158c`).
///
/// - Months: label `420 + month` and the year, `"Dec 429"`. For a positive
///   date the month is `date % 12`; otherwise it is `11 + date % 12`
///   (truncating remainder), and the year is `|date / 12|` with no era.
/// - Years: `"A.D. 1500"` for a date >= 0, `"4000 B.C."` below zero
///   (labels 1 and 0).
///
/// `labels` is the `Labels.txt` in effect; a missing label falls back to
/// the standard English text.
pub fn date_text(date: i32, style: DateStyle, labels: Option<&Labels>) -> String {
    let label = |i: usize, fallback: &str| -> String {
        labels
            .and_then(|l| l.get(i))
            .unwrap_or(fallback)
            .to_string()
    };
    match style {
        DateStyle::Months => {
            let month = if date > 0 { date % 12 } else { 11 + date % 12 } as usize;
            format!(
                "{} {}",
                label(MONTH_LABEL + month, MONTHS[month]),
                (date / 12).unsigned_abs()
            )
        }
        DateStyle::Years if date >= 0 => format!("{} {date}", label(1, ERA[1])),
        DateStyle::Years => format!("{} {}", date.unsigned_abs(), label(0, ERA[0])),
    }
}

impl crate::WorldState {
    /// The current date and how it reads; `None` when the scenario's
    /// calendar is not located. The number is recomputed from the turn: the
    /// original's display ignored an edited year field (evidence README
    /// §T1-8 save writer).
    pub fn date(&self) -> Option<(i32, DateStyle)> {
        let flags = self.research_settings.flags;
        let style = DateStyle::of(flags, self.calendar)?;
        let settings = self.calendar?;
        Some((year(self.turn, self.difficulty, flags, settings), style))
    }
}

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
    fn dates_read_like_the_original() {
        let y = DateStyle::Years;
        assert_eq!(date_text(-4000, y, None), "4000 B.C.");
        assert_eq!(date_text(1500, y, None), "A.D. 1500");
        assert_eq!(date_text(0, y, None), "A.D. 0");
        let m = DateStyle::Months;
        // FallRome turn 0: raw 5159 is December 429 (owner-observed "Dec 429").
        assert_eq!(date_text(5159, m, None), "Dec 429");
        assert_eq!(date_text(5160, m, None), "Jan 430");
        assert_eq!(date_text(12, m, None), "Jan 1");
        // Zero and below take the second branch: 11 + remainder.
        assert_eq!(date_text(0, m, None), "Dec 0");
        assert_eq!(date_text(-1, m, None), "Nov 0");
        assert_eq!(date_text(-13, m, None), "Nov 1");
        let fantasy = Labels::parse("@LABELS\n2\nFirst Age\nSecond Age \n").unwrap();
        assert_eq!(date_text(-10, y, Some(&fantasy)), "10 First Age");
        assert_eq!(date_text(10, y, Some(&fantasy)), "Second Age 10");
        // Short files fall back to the English month.
        assert_eq!(date_text(5159, m, Some(&fantasy)), "Dec 429");
    }

    #[test]
    fn style_follows_the_scenario_flag_and_increment() {
        let s = |increment| {
            Some(CalendarSettings {
                turn_limit: -1,
                increment,
                start_year: 5160,
            })
        };
        assert_eq!(DateStyle::of(0, None), Some(DateStyle::Years));
        assert_eq!(DateStyle::of(0, s(-1)), Some(DateStyle::Years));
        assert_eq!(DateStyle::of(0x80, s(-5)), Some(DateStyle::Months));
        assert_eq!(DateStyle::of(0x80, s(10)), Some(DateStyle::Years));
        assert_eq!(DateStyle::of(0x80, None), None);
    }

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
