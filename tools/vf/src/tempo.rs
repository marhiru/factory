//! Datas sem dependências: epoch <-> civil (UTC), RFC 3339 e o `at=` do `vf publish`.
//! `at=` aceita: `+30m`, `+2h`, `+1d`; `2026-11-20 18:00` (hora local); `2026-11-20T18:00:00-03:00` ou `...Z`.
use anyhow::{bail, Context, Result};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// dias desde 1970-01-01 -> (ano, mês, dia)   (algoritmo de Howard Hinnant)
pub fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// epoch -> "2026-11-20T21:00:00Z"
pub fn rfc3339(t: i64) -> String {
    let (y, m, d) = civil(t.div_euclid(86_400));
    let s = t.rem_euclid(86_400);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", s / 3600, s / 60 % 60, s % 60)
}

/// epoch -> "2026-11-20" (UTC)
pub fn date(t: i64) -> String {
    rfc3339(t)[..10].to_string()
}

/// epoch -> hora local legível ("2026-11-20 18:00 -0300"), pelo `date` do sistema
pub fn local(t: i64) -> String {
    let fmt = "+%Y-%m-%d %H:%M %z";
    let o = std::process::Command::new("date").args(["-r", &t.to_string(), fmt]).output().ok().filter(|o| o.status.success())
        .or_else(|| std::process::Command::new("date").args(["-d", &format!("@{t}"), fmt]).output().ok().filter(|o| o.status.success()));
    o.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_else(|| rfc3339(t))
}

/// RFC 3339 / ISO 8601 com fuso (Z ou ±hh:mm) -> epoch. Sem fuso: None (é hora local).
pub fn parse_iso(s: &str) -> Option<Result<i64>> {
    let s = s.trim().replace(' ', "T");
    let (dt, off) = if let Some(x) = s.strip_suffix('Z').or_else(|| s.strip_suffix('z')) {
        (x.to_string(), 0i64)
    } else if s.len() > 16 && matches!(&s[s.len() - 6..s.len() - 5], "+" | "-") && &s[s.len() - 3..s.len() - 2] == ":" {
        let o = &s[s.len() - 6..];
        let sign = if o.starts_with('-') { -1 } else { 1 };
        let h: i64 = o[1..3].parse().ok()?;
        let m: i64 = o[4..6].parse().ok()?;
        (s[..s.len() - 6].to_string(), sign * (h * 3600 + m * 60))
    } else {
        return None;
    };
    Some(civil_secs(&dt).map(|t| t - off))
}

/// "2026-11-20T18:00[:00][.000]" lido como UTC -> epoch
fn civil_secs(dt: &str) -> Result<i64> {
    let dt = dt.split('.').next().unwrap_or(dt);
    let (d, h) = dt.split_once('T').unwrap_or((dt, "00:00"));
    let dp: Vec<&str> = d.split('-').collect();
    let hp: Vec<&str> = h.split(':').collect();
    if dp.len() != 3 || hp.len() < 2 {
        bail!("data inválida: {dt} (use AAAA-MM-DD HH:MM)");
    }
    let n = |x: &str| x.parse::<i64>().with_context(|| format!("data inválida: {dt}"));
    let (y, mo, da) = (n(dp[0])?, n(dp[1])?, n(dp[2])?);
    let (hh, mi, ss) = (n(hp[0])?, n(hp[1])?, hp.get(2).map(|x| n(x)).transpose()?.unwrap_or(0));
    if !(1..=12).contains(&mo) || !(1..=31).contains(&da) || hh > 23 || mi > 59 || ss > 60 {
        bail!("data inválida: {dt}");
    }
    Ok(days_from_civil(y, mo as u32, da as u32) * 86_400 + hh * 3600 + mi * 60 + ss)
}

/// `at=` -> epoch. Hora sem fuso = hora local desta máquina.
pub fn parse_when(s: &str) -> Result<i64> {
    let s = s.trim();
    if let Some(rel) = s.strip_prefix('+') {
        let (num, unit) = rel.split_at(rel.find(|c: char| !c.is_ascii_digit()).unwrap_or(rel.len()));
        let n: i64 = num.parse().with_context(|| format!("at={s}: use +30m, +2h ou +1d"))?;
        let mult = match unit {
            "m" | "min" => 60,
            "h" | "" => 3600,
            "d" => 86_400,
            _ => bail!("at={s}: unidade {unit} (use m, h ou d)"),
        };
        return Ok(now() + n * mult);
    }
    if let Some(r) = parse_iso(s) {
        return r;
    }
    // hora local: o `date` do sistema converte (macOS: -j -f; GNU: -d)
    let norm = {
        let x = s.replace(' ', "T");
        let colons = x.split_once('T').map(|(_, h)| h.matches(':').count()).unwrap_or(0);
        match (x.contains('T'), colons) {
            (false, _) => format!("{x}T00:00:00"),
            (true, 1) => format!("{x}:00"),
            _ => x,
        }
    };
    civil_secs(&norm)?; // valida o formato antes de chamar o date
    let out = std::process::Command::new("date").args(["-j", "-f", "%Y-%m-%dT%H:%M:%S", &norm, "+%s"]).output().ok().filter(|o| o.status.success())
        .or_else(|| std::process::Command::new("date").args(["-d", &norm.replace('T', " "), "+%s"]).output().ok().filter(|o| o.status.success()))
        .with_context(|| format!("não consegui ler a data {s}"))?;
    String::from_utf8_lossy(&out.stdout).trim().parse().with_context(|| format!("não consegui ler a data {s}"))
}

/// ISO 8601 de duração do YouTube ("PT1M5S", "PT2H3M", "P1DT2S") -> segundos
pub fn iso_duration(s: &str) -> f64 {
    let mut total = 0.0;
    let mut num = String::new();
    let mut in_time = false;
    for c in s.chars() {
        match c {
            'P' => {}
            'T' => in_time = true,
            '0'..='9' | '.' => num.push(c),
            u => {
                let v: f64 = num.parse().unwrap_or(0.0);
                num.clear();
                total += v * match (u, in_time) {
                    ('D', _) => 86_400.0,
                    ('W', _) => 604_800.0,
                    ('H', true) => 3600.0,
                    ('M', true) => 60.0,
                    ('S', true) => 1.0,
                    _ => 0.0,
                };
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        let t = parse_iso("2026-11-20T18:00:00-03:00").unwrap().unwrap();
        assert_eq!(rfc3339(t), "2026-11-20T21:00:00Z");
        assert_eq!(parse_iso("2024-02-29 23:59Z").unwrap().unwrap(), days_from_civil(2024, 2, 29) * 86_400 + 23 * 3600 + 59 * 60);
        assert!(parse_iso("2026-11-20 18:00").is_none());
        for d in [-1000, 0, 19_000, 20_774] {
            let (y, m, dd) = civil(d);
            assert_eq!(days_from_civil(y, m, dd), d);
        }
    }
    #[test]
    fn relative_and_local() {
        let n = now();
        assert!((parse_when("+2h").unwrap() - n - 7200).abs() < 5);
        assert!((parse_when("+1d").unwrap() - n - 86_400).abs() < 5);
        assert!(parse_when("+3x").is_err());
        assert!(parse_when("2026-13-01 10:00").is_err());
        // hora local: a diferença para UTC é no máximo ±14 h
        let l = parse_when("2026-11-20 18:00").unwrap();
        let u = parse_iso("2026-11-20T18:00Z").unwrap().unwrap();
        assert!((l - u).abs() <= 14 * 3600);
    }
    #[test]
    fn durations() {
        assert_eq!(iso_duration("PT1M5S"), 65.0);
        assert_eq!(iso_duration("PT2H3M"), 7380.0);
        assert_eq!(iso_duration("P1DT2S"), 86_402.0);
    }
}
