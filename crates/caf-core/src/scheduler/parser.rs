//! Schedule parser and human-readable description generator.
//!
//! Supports:
//! - Presets: `hourly`, `daily`, `weekly`, `monthly`
//! - Intervals: `every <n> minutes` where n >= 5
//! - 5-field cron in local time: `<min> <hour> <day_of_month> <month> <day_of_week>`

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Timelike};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetSchedule {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronField {
    pub values: BTreeSet<u32>,
    pub is_wildcard: bool,
}

impl CronField {
    pub fn matches(&self, val: u32) -> bool {
        self.values.contains(&val)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronSchedule {
    pub minute: CronField,
    pub hour: CronField,
    pub day_of_month: CronField,
    pub month: CronField,
    pub day_of_week: CronField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSchedule {
    Preset(PresetSchedule),
    IntervalMinutes(u32),
    Cron(CronSchedule),
}

pub fn parse_schedule(schedule: &str) -> Result<ParsedSchedule, String> {
    let s = schedule.trim();
    if s == "hourly" {
        return Ok(ParsedSchedule::Preset(PresetSchedule::Hourly));
    }
    if s == "daily" {
        return Ok(ParsedSchedule::Preset(PresetSchedule::Daily));
    }
    if s == "weekly" {
        return Ok(ParsedSchedule::Preset(PresetSchedule::Weekly));
    }
    if s == "monthly" {
        return Ok(ParsedSchedule::Preset(PresetSchedule::Monthly));
    }

    if let Some(rest) = s.strip_prefix("every ")
        && let Some(min_str) = rest.strip_suffix(" minutes")
    {
        let n: u32 = min_str
            .trim()
            .parse()
            .map_err(|_| format!("invalid interval minutes in '{s}'"))?;
        if n < 5 {
            return Err(format!("interval minutes must be >= 5 in '{s}'"));
        }
        return Ok(ParsedSchedule::IntervalMinutes(n));
    }

    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(format!(
            "invalid schedule '{s}': expected preset, 'every <n> minutes', or 5 cron fields"
        ));
    }

    let min_part = parts.first().copied().unwrap_or("");
    let hour_part = parts.get(1).copied().unwrap_or("");
    let dom_part = parts.get(2).copied().unwrap_or("");
    let month_part = parts.get(3).copied().unwrap_or("");
    let dow_part = parts.get(4).copied().unwrap_or("");

    let minute = parse_field(min_part, 0, 59)?;
    let hour = parse_field(hour_part, 0, 23)?;
    let day_of_month = parse_field(dom_part, 1, 31)?;
    let month = parse_field(month_part, 1, 12)?;
    let mut day_of_week = parse_field(dow_part, 0, 7)?;

    // In cron standard, 7 is also Sunday (0)
    if day_of_week.values.contains(&7) {
        day_of_week.values.insert(0);
    }

    Ok(ParsedSchedule::Cron(CronSchedule {
        minute,
        hour,
        day_of_month,
        month,
        day_of_week,
    }))
}

fn parse_field(field: &str, min: u32, max: u32) -> Result<CronField, String> {
    if field.is_empty() {
        return Err("empty cron field".to_string());
    }

    let mut values = BTreeSet::new();

    for sub in field.split(',') {
        let sub = sub.trim();
        if sub.is_empty() {
            return Err("empty sub-expression in cron field".to_string());
        }

        if let Some((base, step_str)) = sub.split_once('/') {
            let step: u32 = step_str
                .parse()
                .map_err(|_| format!("invalid step '{step_str}' in '{sub}'"))?;
            if step == 0 {
                return Err("step cannot be 0".to_string());
            }

            let (start, end) = if base == "*" {
                (min, max)
            } else if let Some((r_start, r_end)) = base.split_once('-') {
                let s: u32 = r_start
                    .parse()
                    .map_err(|_| format!("invalid range start in '{base}'"))?;
                let e: u32 = r_end
                    .parse()
                    .map_err(|_| format!("invalid range end in '{base}'"))?;
                if s > e || s < min || e > max {
                    return Err(format!("range '{base}' out of bounds [{min}-{max}]"));
                }
                (s, e)
            } else {
                let s: u32 = base
                    .parse()
                    .map_err(|_| format!("invalid base '{base}' in step"))?;
                if s < min || s > max {
                    return Err(format!("base '{base}' out of bounds [{min}-{max}]"));
                }
                (s, max)
            };

            let mut curr = start;
            while curr <= end {
                values.insert(curr);
                curr = match curr.checked_add(step) {
                    Some(next) => next,
                    None => break,
                };
            }
        } else if let Some((r_start, r_end)) = sub.split_once('-') {
            let s: u32 = r_start
                .parse()
                .map_err(|_| format!("invalid range start in '{sub}'"))?;
            let e: u32 = r_end
                .parse()
                .map_err(|_| format!("invalid range end in '{sub}'"))?;
            if s > e || s < min || e > max {
                return Err(format!("range '{sub}' out of bounds [{min}-{max}]"));
            }
            for v in s..=e {
                values.insert(v);
            }
        } else if sub == "*" {
            for v in min..=max {
                values.insert(v);
            }
        } else {
            let v: u32 = sub
                .parse()
                .map_err(|_| format!("invalid number in '{sub}'"))?;
            if v < min || v > max {
                return Err(format!("value '{v}' out of bounds [{min}-{max}]"));
            }
            values.insert(v);
        }
    }

    if values.is_empty() {
        return Err("cron field matched no valid values".to_string());
    }

    let is_wildcard = field.trim() == "*";
    Ok(CronField {
        values,
        is_wildcard,
    })
}

pub fn compute_next_run(
    parsed: &ParsedSchedule,
    after: DateTime<Local>,
) -> Option<DateTime<Local>> {
    match parsed {
        ParsedSchedule::Preset(PresetSchedule::Hourly) => {
            let candidate = (after + Duration::hours(1))
                .with_minute(0)?
                .with_second(0)?
                .with_nanosecond(0)?;
            Some(candidate)
        }
        ParsedSchedule::Preset(PresetSchedule::Daily) => {
            let candidate = (after + Duration::days(1))
                .with_hour(0)?
                .with_minute(0)?
                .with_second(0)?
                .with_nanosecond(0)?;
            Some(candidate)
        }
        ParsedSchedule::Preset(PresetSchedule::Weekly) => {
            // Monday at 02:00
            let mut candidate = after
                .with_hour(2)?
                .with_minute(0)?
                .with_second(0)?
                .with_nanosecond(0)?;
            while candidate <= after || candidate.weekday() != chrono::Weekday::Mon {
                candidate += Duration::days(1);
            }
            Some(candidate)
        }
        ParsedSchedule::Preset(PresetSchedule::Monthly) => {
            // 1st of next month at 02:00
            let mut year = after.year();
            let mut month = after.month() + 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
            let date = NaiveDate::from_ymd_opt(year, month, 1)?;
            let candidate = date
                .and_hms_opt(2, 0, 0)?
                .and_local_timezone(Local)
                .single()?;
            Some(candidate)
        }
        ParsedSchedule::IntervalMinutes(n) => Some(after + Duration::minutes(*n as i64)),
        ParsedSchedule::Cron(cron) => {
            let start = after + Duration::minutes(1);
            let mut curr = start.with_second(0)?.with_nanosecond(0)?;

            let max_year = after.year() + 5;

            while curr.year() <= max_year {
                let m = curr.month();
                if !cron.month.matches(m) {
                    let mut next_m = m + 1;
                    let mut next_y = curr.year();
                    if next_m > 12 {
                        next_m = 1;
                        next_y += 1;
                    }
                    if let Some(nd) = NaiveDate::from_ymd_opt(next_y, next_m, 1) {
                        curr = nd
                            .and_hms_opt(0, 0, 0)?
                            .and_local_timezone(Local)
                            .single()?;
                        continue;
                    }
                }

                let dom = curr.day();
                let dow = match curr.weekday() {
                    chrono::Weekday::Sun => 0,
                    chrono::Weekday::Mon => 1,
                    chrono::Weekday::Tue => 2,
                    chrono::Weekday::Wed => 3,
                    chrono::Weekday::Thu => 4,
                    chrono::Weekday::Fri => 5,
                    chrono::Weekday::Sat => 6,
                };

                let day_matches =
                    match (cron.day_of_month.is_wildcard, cron.day_of_week.is_wildcard) {
                        (true, true) => true,
                        (false, true) => cron.day_of_month.matches(dom),
                        (true, false) => cron.day_of_week.matches(dow),
                        (false, false) => {
                            cron.day_of_month.matches(dom) || cron.day_of_week.matches(dow)
                        }
                    };

                if !day_matches {
                    curr = (curr + Duration::days(1)).with_hour(0)?.with_minute(0)?;
                    continue;
                }

                let h = curr.hour();
                if !cron.hour.matches(h) {
                    curr = (curr + Duration::hours(1)).with_minute(0)?;
                    continue;
                }

                let min = curr.minute();
                if !cron.minute.matches(min) {
                    curr += Duration::minutes(1);
                    continue;
                }

                return Some(curr);
            }

            None
        }
    }
}

pub fn describe_schedule_en(schedule: &str) -> String {
    let s = schedule.trim();
    match s {
        "hourly" => "Every hour at minute 0".to_string(),
        "daily" => "Every day at 00:00".to_string(),
        "weekly" => "Every week on Monday at 02:00".to_string(),
        "monthly" => "Every month on day 1 at 02:00".to_string(),
        "0 2 * * 1" => "Every week on Monday at 02:00".to_string(),
        "0 0 * * *" => "Every day at 00:00".to_string(),
        "0 * * * *" => "Every hour at minute 0".to_string(),
        "30 14 * * *" => "Every day at 14:30".to_string(),
        "0 9 * * 1-5" => "Monday to Friday at 09:00".to_string(),
        "0 9 * * *" => "Every day at 09:00".to_string(),
        "0 3 * * 0" => "Every week on Sunday at 03:00".to_string(),
        _ => {
            if let Some(rest) = s.strip_prefix("every ")
                && let Some(min_str) = rest.strip_suffix(" minutes")
            {
                return format!("Every {} minutes", min_str.trim());
            }
            if let Ok(ParsedSchedule::Cron(c)) = parse_schedule(s) {
                format_cron_en(&c, s)
            } else {
                format!("Schedule: {s}")
            }
        }
    }
}

pub fn describe_schedule_id(schedule: &str) -> String {
    let s = schedule.trim();
    match s {
        "hourly" => "Setiap jam pada menit 0".to_string(),
        "daily" => "Setiap hari pukul 00.00".to_string(),
        "weekly" => "Setiap Senin pukul 02.00".to_string(),
        "monthly" => "Setiap tanggal 1 pukul 02.00".to_string(),
        "0 2 * * 1" => "Setiap Senin pukul 02.00".to_string(),
        "0 0 * * *" => "Setiap hari pukul 00.00".to_string(),
        "0 * * * *" => "Setiap jam pada menit 0".to_string(),
        "30 14 * * *" => "Setiap hari pukul 14.30".to_string(),
        "0 9 * * 1-5" => "Senin hingga Jumat pukul 09.00".to_string(),
        "0 9 * * *" => "Setiap hari pukul 09.00".to_string(),
        "0 3 * * 0" => "Setiap Minggu pukul 03.00".to_string(),
        _ => {
            if let Some(rest) = s.strip_prefix("every ")
                && let Some(min_str) = rest.strip_suffix(" minutes")
            {
                return format!("Setiap {} menit", min_str.trim());
            }
            if let Ok(ParsedSchedule::Cron(c)) = parse_schedule(s) {
                format_cron_id(&c, s)
            } else {
                format!("Jadwal: {s}")
            }
        }
    }
}

fn format_cron_en(c: &CronSchedule, raw: &str) -> String {
    if c.minute.values.len() == 1 && c.hour.values.len() == 1 {
        let m = c.minute.values.iter().next().copied().unwrap_or(0);
        let h = c.hour.values.iter().next().copied().unwrap_or(0);
        let time_str = format!("{h:02}:{m:02}");

        if c.day_of_month.values.len() == 31 && c.month.values.len() == 12 {
            if c.day_of_week.values.len() >= 7 {
                return format!("Every day at {time_str}");
            }
            if c.day_of_week.values.len() == 1 {
                let dow = c.day_of_week.values.iter().next().copied().unwrap_or(0);
                let day_name = match dow {
                    0 | 7 => "Sunday",
                    1 => "Monday",
                    2 => "Tuesday",
                    3 => "Wednesday",
                    4 => "Thursday",
                    5 => "Friday",
                    6 => "Saturday",
                    _ => "day",
                };
                return format!("Every week on {day_name} at {time_str}");
            }
        }
    }
    format!("Cron: {raw}")
}

fn format_cron_id(c: &CronSchedule, raw: &str) -> String {
    if c.minute.values.len() == 1 && c.hour.values.len() == 1 {
        let m = c.minute.values.iter().next().copied().unwrap_or(0);
        let h = c.hour.values.iter().next().copied().unwrap_or(0);
        let time_str = format!("{h:02}.{m:02}");

        if c.day_of_month.values.len() == 31 && c.month.values.len() == 12 {
            if c.day_of_week.values.len() >= 7 {
                return format!("Setiap hari pukul {time_str}");
            }
            if c.day_of_week.values.len() == 1 {
                let dow = c.day_of_week.values.iter().next().copied().unwrap_or(0);
                let day_name = match dow {
                    0 | 7 => "Minggu",
                    1 => "Senin",
                    2 => "Selasa",
                    3 => "Rabu",
                    4 => "Kamis",
                    5 => "Jumat",
                    6 => "Sabtu",
                    _ => "hari",
                };
                return format!("Setiap {day_name} pukul {time_str}");
            }
        }
    }
    format!("Cron: {raw}")
}
