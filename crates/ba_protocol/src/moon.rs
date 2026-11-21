//! Real lunar age for the night sky.
//!
//! Phase comes from new-moon instants, not a looping game clock. The 2026
//! list below is the US Naval Observatory year table
//! (`https://aa.usno.navy.mil/api/moon/phases/year?year=2026`), pulled once.
//! The zone server keeps later years in `data/moon_phases.json` and only
//! calls USNO when that file and this list no longer bracket the current
//! time, and at most one attempt every 12 hours. Clients never call USNO.
//!
//! Age is the fraction of the current lunation: 0 new, 0.5 full, 1 the next new.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Mean synodic month (Meeus). Used only when no observatory lunation brackets now.
pub const SYNODIC_MONTH_SECS: f64 = 29.530588853 * 86_400.0;

/// New moon of 2000-01-06 18:14 UT. Mean-synodic epoch.
pub const MEAN_NEW_MOON_UNIX: i64 = unix_from_utc(2000, 1, 6, 18, 14);

pub const USNO_PHASES_URL: &str = "https://aa.usno.navy.mil/api/moon/phases/year";

/// USNO new moons for 2026 (UT). The server extends this on disk for other years.
pub const BAKED_NEW_MOONS_UNIX: &[i64] = &[
    1_768_765_920, // 2026-01-18 19:52
    1_771_329_660, // 2026-02-17 12:01
    1_773_883_380, // 2026-03-19 01:23
    1_776_426_720, // 2026-04-17 11:52
    1_778_961_660, // 2026-05-16 20:01
    1_781_492_040, // 2026-06-15 02:54
    1_784_022_180, // 2026-07-14 09:43
    1_786_556_220, // 2026-08-12 17:37
    1_789_097_220, // 2026-09-11 03:27
    1_791_647_400, // 2026-10-10 15:50
    1_794_207_720, // 2026-11-09 07:02
    1_796_777_520, // 2026-12-09 00:52
];

/// One lunation. Age is measured from `new_moon_unix` to `next_new_moon_unix`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MoonState {
    pub new_moon_unix: i64,
    pub next_new_moon_unix: i64,
}

impl MoonState {
    /// A real synodic month is about 29.5 days. Reject anything else on the wire.
    pub fn plausible(self) -> bool {
        let span = self.next_new_moon_unix - self.new_moon_unix;
        (20 * 86_400..40 * 86_400).contains(&span)
    }

    /// 0 at the opening new moon, 0.5 at full, wrapping into the next cycle.
    pub fn age_at(self, now: i64) -> f32 {
        let span = (self.next_new_moon_unix - self.new_moon_unix).max(1) as f64;
        let age = ((now - self.new_moon_unix) as f64 / span).rem_euclid(1.0);
        if age.is_finite() {
            age as f32
        } else {
            0.0
        }
    }
}

/// Observatory new moons persisted by the zone server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MoonPhaseTable {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub fetched_at_unix: i64,
    #[serde(default)]
    pub years: Vec<i32>,
    #[serde(default)]
    pub new_moons_unix: Vec<i64>,
}

pub fn cache_path() -> PathBuf {
    std::env::var_os("BA_MOON_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/moon_phases.json"))
}

pub fn read_cache(path: &Path) -> Option<MoonPhaseTable> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut table: MoonPhaseTable = serde_json::from_str(&text).ok()?;
    table.new_moons_unix.sort_unstable();
    table.new_moons_unix.dedup();
    table.years.sort_unstable();
    table.years.dedup();
    Some(table)
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Howard Hinnant's civil-from-days, days since 1970-01-01.
pub const fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = (if year >= 0 { year } else { year - 399 }) / 400;
    let yoe = (year - era * 400) as u32;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146_097 + doe as i64 - 719_468
}

pub const fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

pub const fn unix_from_utc(year: i32, month: u32, day: u32, hour: u32, min: u32) -> i64 {
    days_from_civil(year, month, day) * 86_400 + hour as i64 * 3600 + min as i64 * 60
}

pub fn utc_year(unix: i64) -> i32 {
    civil_from_days(unix.div_euclid(86_400)).0
}

/// Lunation containing `now`, from a sorted new-moon list.
pub fn bracketed(now: i64, new_moons_unix: &[i64]) -> Option<MoonState> {
    let mut prev = None;
    let mut next = None;
    for &t in new_moons_unix {
        if t <= now {
            prev = Some(t);
        } else {
            next = Some(t);
            break;
        }
    }
    let moon = MoonState {
        new_moon_unix: prev?,
        next_new_moon_unix: next?,
    };
    moon.plausible().then_some(moon)
}

pub fn covered(now: i64, new_moons_unix: &[i64]) -> bool {
    bracketed(now, new_moons_unix).is_some() || bracketed(now, BAKED_NEW_MOONS_UNIX).is_some()
}

/// Persisted USNO instants win, then the baked year, then the mean month.
pub fn lunation_at(now: i64, new_moons_unix: &[i64]) -> MoonState {
    if let Some(moon) = bracketed(now, new_moons_unix) {
        return moon;
    }
    if let Some(moon) = bracketed(now, BAKED_NEW_MOONS_UNIX) {
        return moon;
    }
    mean_lunation(now)
}

pub fn mean_lunation(now: i64) -> MoonState {
    let span = SYNODIC_MONTH_SECS;
    let n = ((now as f64 - MEAN_NEW_MOON_UNIX as f64) / span).floor();
    let start = MEAN_NEW_MOON_UNIX as f64 + n * span;
    MoonState {
        new_moon_unix: start.round() as i64,
        next_new_moon_unix: (start + span).round() as i64,
    }
}

pub fn phase_name(age: f32) -> &'static str {
    const NAMES: [&str; 8] = [
        "new moon",
        "waxing crescent",
        "first quarter",
        "waxing gibbous",
        "full moon",
        "waning gibbous",
        "last quarter",
        "waning crescent",
    ];
    let age = if age.is_finite() { age.rem_euclid(1.0) } else { 0.0 };
    let i = ((age * 8.0).floor() as usize).min(7);
    NAMES[i]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_unix_matches_known_instants() {
        assert_eq!(unix_from_utc(1970, 1, 1, 0, 0), 0);
        assert_eq!(unix_from_utc(2000, 1, 6, 18, 14), 947_182_440);
        assert_eq!(MEAN_NEW_MOON_UNIX, 947_182_440);
        assert_eq!(unix_from_utc(2026, 11, 9, 7, 2), 1_794_207_720);
        assert_eq!(unix_from_utc(2026, 12, 9, 0, 52), 1_796_777_520);
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(civil_from_days(days_from_civil(2026, 11, 21)), (2026, 11, 21));
        assert_eq!(utc_year(1_795_295_280), 2026);
    }

    #[test]
    fn baked_usno_november_2026_is_waxing_gibbous() {
        let now = unix_from_utc(2026, 11, 21, 21, 8);
        let moon = lunation_at(now, &[]);
        assert_eq!(moon.new_moon_unix, 1_794_207_720);
        assert_eq!(moon.next_new_moon_unix, 1_796_777_520);
        let age = moon.age_at(now);
        assert!((age - 0.4232).abs() < 0.001, "{age}");
        assert_eq!(phase_name(age), "waxing gibbous");
        assert!(covered(now, &[]));
    }

    #[test]
    fn persisted_lunation_wins_over_the_baked_year() {
        let now = unix_from_utc(2026, 11, 21, 21, 8);
        let custom = [now - 86_400, now + 28 * 86_400];
        let moon = lunation_at(now, &custom);
        assert_eq!(moon.new_moon_unix, custom[0]);
    }

    #[test]
    fn mean_month_is_new_at_the_epoch_and_used_when_uncovered() {
        let moon = mean_lunation(MEAN_NEW_MOON_UNIX);
        assert!(moon.age_at(MEAN_NEW_MOON_UNIX).abs() < 0.001);
        assert!(moon.plausible());
        let far = unix_from_utc(2035, 6, 1, 0, 0);
        assert!(!covered(far, &[]));
        assert!(lunation_at(far, &[]).plausible());
    }

    #[test]
    fn moon_message_roundtrips() {
        let msg = crate::ServerMessage::Moon {
            new_moon_unix: 1_794_207_720,
            next_new_moon_unix: 1_796_777_520,
        };
        let json = msg.to_json().unwrap();
        assert!(json.contains("\"type\":\"moon\""));
        assert_eq!(crate::ServerMessage::from_json(&json).unwrap(), msg);
    }
}
