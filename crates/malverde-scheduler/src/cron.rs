use chrono::{DateTime, Datelike, Duration, Timelike, Utc};

use malverde_core::{MalverdeError, Result};

/// Expresión cron de 5 campos: `min hora día-del-mes mes día-de-la-semana`.
/// Soporta `*`, listas (`,`), rangos (`-`) y pasos (`*/n`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cron {
    raw: String,
    minutes: Vec<u32>,
    hours: Vec<u32>,
    days_of_month: Vec<u32>,
    months: Vec<u32>,
    days_of_week: Vec<u32>,
}

impl Cron {
    pub fn parse(expr: &str) -> Result<Self> {
        let fields: Vec<&str> = expr.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(MalverdeError::scheduler(format!(
                "cron inválido '{expr}': se requieren 5 campos (min hora dom mes dow)"
            )));
        }
        Ok(Self {
            raw: expr.to_string(),
            minutes: parse_field(fields[0], 0, 59)?,
            hours: parse_field(fields[1], 0, 23)?,
            days_of_month: parse_field(fields[2], 1, 31)?,
            months: parse_field(fields[3], 1, 12)?,
            days_of_week: parse_field(fields[4], 0, 6)?,
        })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Siguiente ejecución estrictamente posterior a `after` (UTC).
    pub fn next_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        // Búsqueda minutal acotada: ~366 días máx (527k iteraciones como mucho).
        let mut t = after
            .with_second(0)
            .and_then(|t| t.with_nanosecond(0))
            .unwrap_or(after)
            + Duration::minutes(1);
        let limit = after + Duration::days(366);
        while t <= limit {
            if self.matches(t) {
                return Some(t);
            }
            t += Duration::minutes(1);
        }
        None
    }

    pub fn matches(&self, t: DateTime<Utc>) -> bool {
        let dom_star = self.days_of_month.len() == 31;
        let dow_star = self.days_of_week.len() == 7;
        if !self.months.contains(&t.month()) {
            return false;
        }
        let dom_ok = self.days_of_month.contains(&t.day());
        // chrono: weekday().num_days_from_sunday() → 0=domingo.
        let dow_ok = self.days_of_week.contains(&t.weekday().num_days_from_sunday());
        let day_ok = match (dom_star, dow_star) {
            (true, true) => true,
            (false, true) => dom_ok,
            (true, false) => dow_ok,
            // Regla clásica de Vixie cron: si ambos están restringidos, vale cualquiera.
            (false, false) => dom_ok || dow_ok,
        };
        day_ok
            && self.hours.contains(&t.hour())
            && self.minutes.contains(&t.minute())
    }
}

fn parse_field(field: &str, min: u32, max: u32) -> Result<Vec<u32>> {
    let mut values = Vec::new();
    for part in field.split(',') {
        let (range, step) = match part.split_once('/') {
            Some((r, s)) => (r, s.parse::<u32>().map_err(|_| bad(field))?),
            None => (part, 1),
        };
        if step == 0 {
            return Err(bad(field));
        }
        let (lo, hi) = if range == "*" {
            (min, max)
        } else if let Some((a, b)) = range.split_once('-') {
            (
                a.parse::<u32>().map_err(|_| bad(field))?,
                b.parse::<u32>().map_err(|_| bad(field))?,
            )
        } else {
            let v: u32 = range.parse().map_err(|_| bad(field))?;
            (v, v)
        };
        if lo < min || hi > max || lo > hi {
            return Err(bad(field));
        }
        let mut v = lo;
        while v <= hi {
            values.push(v);
            v += step;
        }
    }
    values.sort_unstable();
    values.dedup();
    if values.is_empty() {
        return Err(bad(field));
    }
    Ok(values)
}

fn bad(field: &str) -> MalverdeError {
    MalverdeError::scheduler(format!("campo cron inválido '{field}'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    #[test]
    fn parses_wildcard() {
        let c = Cron::parse("* * * * *").unwrap();
        assert_eq!(c.minutes.len(), 60);
        assert!(c.matches(at(2026, 1, 1, 0, 0)));
    }

    #[test]
    fn parses_steps_and_lists() {
        let c = Cron::parse("*/15 9-17 * * 1-5").unwrap();
        assert_eq!(c.minutes, vec![0, 15, 30, 45]);
        assert!(c.hours.contains(&9) && c.hours.contains(&17));
        assert!(c.matches(at(2026, 3, 2, 9, 30))); // lunes
        assert!(!c.matches(at(2026, 3, 1, 9, 30))); // domingo
        assert!(!c.matches(at(2026, 3, 2, 18, 30)));
    }

    #[test]
    fn dom_and_dow_follow_vixie_either_rule() {
        // dom y dow restringidos: coincide si CUALQUIERA de los dos coincide.
        let c = Cron::parse("0 9 1,15 * 1-5").unwrap();
        assert!(c.matches(at(2026, 3, 1, 9, 0))); // domingo 1 → por dom
        assert!(c.matches(at(2026, 3, 2, 9, 0))); // lunes 2 → por dow
        assert!(!c.matches(at(2026, 3, 7, 9, 0))); // sábado 7 → dom=7 y dow=6, ninguno
    }

    #[test]
    fn next_after_advances() {
        let c = Cron::parse("30 9 * * *").unwrap();
        let next = c.next_after(at(2026, 6, 1, 10, 0)).unwrap();
        assert_eq!(next, at(2026, 6, 2, 9, 30));
        let next2 = c.next_after(at(2026, 6, 1, 8, 0)).unwrap();
        assert_eq!(next2, at(2026, 6, 1, 9, 30));
    }

    #[test]
    fn rejects_bad_expressions() {
        assert!(Cron::parse("61 * * * *").is_err());
        assert!(Cron::parse("* * * *").is_err());
        assert!(Cron::parse("a * * * *").is_err());
    }
}
