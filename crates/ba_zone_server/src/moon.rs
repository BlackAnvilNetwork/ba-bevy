//! USNO lunar table for the zone. One HTTP call per missing year, then the
//! file is the moon until it no longer brackets the clock.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use ba_protocol::moon::{
    self, cache_path, covered, lunation_at, unix_now, MoonPhaseTable, USNO_PHASES_URL,
};
use ba_protocol::ServerMessage;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub type SharedMoon = Arc<RwLock<MoonPhaseTable>>;

/// Don't hit USNO again if the last attempt was recent, even when it failed.
const MIN_REFETCH_SECS: i64 = 12 * 60 * 60;

pub async fn load_or_fetch() -> MoonPhaseTable {
    let path = cache_path();
    let mut table = load_file(&path);
    if refresh(&mut table).await {
        if let Err(err) = write_cache(&path, &table) {
            warn!("could not write moon cache {}: {err}", path.display());
        } else {
            info!("wrote moon cache {}", path.display());
        }
    } else if covered(unix_now(), &table.new_moons_unix) {
        info!("moon phase already known; not calling USNO");
    }
    log_phase(&table);
    table
}

pub async fn refresh_shared(shared: &SharedMoon) {
    let mut table = shared.read().await.clone();
    if !refresh(&mut table).await {
        return;
    }
    let path = cache_path();
    if let Err(err) = write_cache(&path, &table) {
        warn!("could not write moon cache {}: {err}", path.display());
    } else {
        info!("wrote moon cache {}", path.display());
    }
    log_phase(&table);
    *shared.write().await = table;
}

pub fn server_message(table: &MoonPhaseTable) -> ServerMessage {
    let state = lunation_at(unix_now(), &table.new_moons_unix);
    ServerMessage::Moon {
        new_moon_unix: state.new_moon_unix,
        next_new_moon_unix: state.next_new_moon_unix,
    }
}

fn log_phase(table: &MoonPhaseTable) {
    let now = unix_now();
    let state = lunation_at(now, &table.new_moons_unix);
    let age = state.age_at(now);
    let source = if moon::bracketed(now, &table.new_moons_unix).is_some() {
        "USNO cache"
    } else if moon::bracketed(now, moon::BAKED_NEW_MOONS_UNIX).is_some() {
        "baked USNO 2026"
    } else {
        "mean synodic month"
    };
    info!("moon {} age={age:.3} ({source})", moon::phase_name(age));
}

fn load_file(path: &Path) -> MoonPhaseTable {
    match std::fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<MoonPhaseTable>(&text) {
            Ok(mut table) => {
                table.new_moons_unix.sort_unstable();
                table.new_moons_unix.dedup();
                table.years.sort_unstable();
                table.years.dedup();
                table
            }
            Err(err) => {
                warn!("moon cache {} is unreadable: {err}", path.display());
                MoonPhaseTable::default()
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => MoonPhaseTable::default(),
        Err(err) => {
            warn!("moon cache {}: {err}", path.display());
            MoonPhaseTable::default()
        }
    }
}

/// Returns true when `table` changed and should be written.
async fn refresh(table: &mut MoonPhaseTable) -> bool {
    let now = unix_now();
    if covered(now, &table.new_moons_unix) {
        return false;
    }
    if table.fetched_at_unix > 0 && now.saturating_sub(table.fetched_at_unix) < MIN_REFETCH_SECS {
        warn!("moon table does not cover now; skipping USNO until the 12h quiet period ends");
        return false;
    }

    let year = moon::utc_year(now);
    let client = match reqwest::Client::builder()
        .user_agent("BlackAnvil/0.1 (zone moon cache; one request per missing year)")
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            warn!("moon HTTP client: {err}");
            table.fetched_at_unix = now;
            table.source = USNO_PHASES_URL.to_string();
            return true;
        }
    };

    for y in [year, year - 1, year + 1] {
        if covered(now, &table.new_moons_unix) {
            break;
        }
        if table.years.contains(&y) {
            continue;
        }
        match fetch_year(&client, y).await {
            Ok(moons) => {
                info!("USNO {y}: {} new moons", moons.len());
                table.years.push(y);
                table.years.sort_unstable();
                table.years.dedup();
                table.new_moons_unix.extend(moons);
                table.new_moons_unix.sort_unstable();
                table.new_moons_unix.dedup();
            }
            Err(err) => warn!("USNO moon phases {y}: {err}"),
        }
    }

    table.fetched_at_unix = now;
    table.source = USNO_PHASES_URL.to_string();
    true
}

async fn fetch_year(client: &reqwest::Client, year: i32) -> Result<Vec<i64>, String> {
    let url = format!("{USNO_PHASES_URL}?year={year}");
    let response = client.get(&url).send().await.map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    let body = response.text().await.map_err(|err| err.to_string())?;
    new_moons_from_usno(&body)
}

fn new_moons_from_usno(body: &str) -> Result<Vec<i64>, String> {
    let parsed: UsnoYear = serde_json::from_str(body).map_err(|err| err.to_string())?;
    let mut moons = Vec::new();
    for row in parsed.phasedata {
        if row.phase != "New Moon" {
            continue;
        }
        let Some((hour, min)) = parse_hhmm(&row.time) else {
            continue;
        };
        if !(1..=12).contains(&row.month) || !(1..=31).contains(&row.day) {
            continue;
        }
        moons.push(moon::unix_from_utc(row.year, row.month, row.day, hour, min));
    }
    if moons.is_empty() {
        return Err("no new moons in USNO response".into());
    }
    moons.sort_unstable();
    moons.dedup();
    Ok(moons)
}

fn parse_hhmm(time: &str) -> Option<(u32, u32)> {
    let (hour, min) = time.split_once(':')?;
    let hour = hour.parse().ok()?;
    let min = min.parse().ok()?;
    (hour < 24 && min < 60).then_some((hour, min))
}

fn write_cache(path: &Path, table: &MoonPhaseTable) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let json = serde_json::to_vec_pretty(table)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Windows rename does not replace an existing file.
            let _ = std::fs::remove_file(path);
            std::fs::rename(&tmp, path)
        }
    }
}

#[derive(Deserialize)]
struct UsnoYear {
    phasedata: Vec<UsnoPhase>,
}

#[derive(Deserialize)]
struct UsnoPhase {
    phase: String,
    year: i32,
    month: u32,
    day: u32,
    time: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usno_json_keeps_new_moons_only() {
        let body = r#"{
            "phasedata": [
                {"phase":"Full Moon","year":2026,"month":11,"day":24,"time":"14:53"},
                {"phase":"New Moon","year":2026,"month":12,"day":9,"time":"00:52"},
                {"phase":"New Moon","year":2026,"month":11,"day":9,"time":"07:02"}
            ]
        }"#;
        let moons = new_moons_from_usno(body).unwrap();
        assert_eq!(
            moons,
            vec![
                moon::unix_from_utc(2026, 11, 9, 7, 2),
                moon::unix_from_utc(2026, 12, 9, 0, 52),
            ]
        );
    }

    #[test]
    fn empty_usno_body_is_not_a_successful_year() {
        let body = r#"{"phasedata":[{"phase":"Full Moon","year":2026,"month":11,"day":24,"time":"14:53"}]}"#;
        assert!(new_moons_from_usno(body).is_err());
    }
}
