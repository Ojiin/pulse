// The schedules of the app's scheduled tasks: five-field cron expressions,
// matched in local time (a fixed offset east of UTC, in minutes).

use chrono::{DateTime, Datelike, Timelike};

pub const MINUTE: i64 = 60_000;
pub const HOUR: i64 = 60 * MINUTE;
pub const DAY: i64 = 24 * HOUR;

pub struct Cron {
    minute: Vec<bool>,
    hour: Vec<bool>,
    dom: Vec<bool>,
    month: Vec<bool>,
    dow: Vec<bool>,
    is_dom_any: bool,
    is_dow_any: bool,
}

fn field(spec: &str, lo: u32, hi: u32) -> Option<Vec<bool>> {
    let mut out = vec![false; hi as usize + 1];
    for part in spec.split(',') {
        let (range, step) = match part.split_once('/') {
            Some((r, s)) => (r, Some(s.parse::<u32>().ok()?)),
            None => (part, None),
        };
        let step = step.unwrap_or(1);
        if step < 1 {
            return None;
        }
        let (from, to) = if range == "*" {
            (lo, hi)
        } else {
            match range.split_once('-') {
                Some((a, b)) => (a.parse().ok()?, b.parse().ok()?),
                None => {
                    let a = range.parse().ok()?;
                    (a, if part.contains('/') { hi } else { a })
                }
            }
        };
        if from < lo || to > hi {
            return None;
        }
        let mut v = from;
        while v <= to {
            out[v as usize] = true;
            v += step;
        }
    }
    Some(out)
}

pub fn parse(expression: &str) -> Option<Cron> {
    let parts: Vec<&str> = expression.split_whitespace().collect();
    let [mi, ho, dm, mo, dw] = parts.as_slice() else { return None };
    let mut dow = field(dw, 0, 7)?;
    if dow[7] {
        dow[0] = true;
    }
    Some(Cron {
        minute: field(mi, 0, 59)?,
        hour: field(ho, 0, 23)?,
        dom: field(dm, 1, 31)?,
        month: field(mo, 1, 12)?,
        dow,
        is_dom_any: *dm == "*",
        is_dow_any: *dw == "*",
    })
}

// the wall clock at t, as a UTC time shifted by the offset
pub fn local(t: i64, tz: i64) -> DateTime<chrono::Utc> {
    DateTime::from_timestamp_millis(t + tz * MINUTE).unwrap_or_default()
}

impl Cron {
    pub fn matches(&self, t: i64, tz: i64) -> bool {
        let d = local(t, tz);
        if !self.minute[d.minute() as usize] || !self.hour[d.hour() as usize] || !self.month[d.month() as usize] {
            return false;
        }
        let dom = self.dom[d.day() as usize];
        let dow = self.dow[d.weekday().num_days_from_sunday() as usize];
        match (self.is_dom_any, self.is_dow_any) {
            (true, true) => true,
            (true, false) => dow,
            (false, true) => dom,
            (false, false) => dom || dow,
        }
    }

    pub fn slots_between(&self, from: i64, to: i64, tz: i64) -> Vec<i64> {
        let mut out = Vec::new();
        let mut t = (from + MINUTE - 1).div_euclid(MINUTE) * MINUTE;
        while t <= to {
            if self.matches(t, tz) {
                out.push(t);
            }
            t += MINUTE;
        }
        out
    }

    pub fn next_slot(&self, after: i64, tz: i64) -> Option<i64> {
        let limit = after + 8 * DAY;
        let mut t = after.div_euclid(MINUTE) * MINUTE + MINUTE;
        while t <= limit {
            if self.matches(t, tz) {
                return Some(t);
            }
            t += MINUTE;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TZ: i64 = 120; // CEST
    // 2026-10-18 20:00 local = 18:00 UTC
    fn now() -> i64 {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 18).unwrap().and_hms_opt(18, 0, 0).unwrap().and_utc().timestamp_millis()
    }

    #[test]
    fn the_next_slot_is_found_in_local_time() {
        let cron = parse("5 9-22 * * *").expect("cron");
        // 20:00 local → 20:05 local
        assert_eq!(cron.next_slot(now(), TZ), Some(now() + 5 * MINUTE));
        // 22:30 local → 09:05 local the day after = 07:05 UTC
        let next = chrono::NaiveDate::from_ymd_opt(2026, 10, 19).unwrap().and_hms_opt(7, 5, 0).unwrap().and_utc().timestamp_millis();
        assert_eq!(cron.next_slot(now() + 150 * MINUTE, TZ), Some(next));
    }

    #[test]
    fn weekdays_and_bad_expressions() {
        let cron = parse("0 7 * * 1-5").expect("cron");
        // 2026-10-18 is a Sunday: the next slot is Monday 07:00 local
        let monday = chrono::NaiveDate::from_ymd_opt(2026, 10, 19).unwrap().and_hms_opt(5, 0, 0).unwrap().and_utc().timestamp_millis();
        assert_eq!(cron.next_slot(now(), TZ), Some(monday));
        assert!(parse("61 * * * *").is_none());
        assert!(parse("* * *").is_none());
    }

    #[test]
    fn slots_between_two_times() {
        let cron = parse("5 18-20 * * *").expect("cron");
        assert_eq!(cron.slots_between(now() - 3 * HOUR, now(), TZ).len(), 2);
    }
}
