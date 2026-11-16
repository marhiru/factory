//! QA frame a frame do vídeo final: specs vs formato, loudness, pretos/brancos/congelados/pops,
//! estatística por plano e contact sheets densas (das miniaturas da mesma passada de decode).
use crate::formats::Format;
use crate::frames::FrameAnalysis;
use crate::sheet::{self, Tile};
use crate::util::{ffmpeg, tc, write_json};
use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

#[derive(Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    OK,
    WARN,
    FAIL,
}

#[derive(Serialize, Debug)]
pub struct Check {
    pub level: Level,
    pub name: String,
    pub detail: String,
}

#[derive(Serialize, Default, Debug, Clone, Copy)]
pub struct Loudness {
    pub integrated: Option<f64>,
    pub lra: Option<f64>,
    pub true_peak: Option<f64>,
}

pub fn loudness(input: &Path) -> Result<Loudness> {
    let out = Command::new(ffmpeg())
        .args(["-hide_banner", "-nostdin", "-nostats", "-i"])
        .arg(input)
        .args(["-map", "0:a:0", "-af", "ebur128=peak=true", "-f", "null", "-"])
        .output()?;
    let e = String::from_utf8_lossy(&out.stderr);
    let tail = &e[e.rfind("Summary:").unwrap_or(0)..];
    let grab = |key: &str| -> Option<f64> {
        tail.lines()
            .map(str::trim)
            .find(|l| l.starts_with(key))
            .and_then(|l| l[key.len()..].split_whitespace().next()?.parse().ok())
    };
    Ok(Loudness { integrated: grab("I:"), lra: grab("LRA:"), true_peak: grab("Peak:") })
}

/// Lê um shots.json/plan.json no formato do short_qa.py: shots[{a,b}], flash[a,b], xfade[[a,b]], planned[[a,b]].
pub fn planned_frames(plan: Option<&Value>) -> HashSet<usize> {
    let mut s = HashSet::new();
    let Some(p) = plan else { return s };
    let mut add = |a: i64, b: i64| {
        for f in (a - 1).max(0)..=(b + 1).max(0) {
            s.insert(f as usize);
        }
    };
    let pair = |v: &Value| -> Option<(i64, i64)> { Some((v.get(0)?.as_i64()?, v.get(1)?.as_i64()?)) };
    if let Some((a, b)) = p.get("flash").and_then(pair) {
        add(a, b);
    }
    for key in ["xfade", "planned"] {
        for v in p.get(key).and_then(Value::as_array).into_iter().flatten() {
            if let Some((a, b)) = pair(v) {
                add(a, b);
            }
        }
    }
    // fronteiras de plano: o frame do corte e o anterior
    for sh in p.get("shots").and_then(Value::as_array).into_iter().flatten() {
        if let Some(a) = sh.get("a").and_then(Value::as_i64) {
            add(a, a - 1);
        }
    }
    s
}

fn runs(flags: &[bool], min_len: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < flags.len() {
        if flags[i] {
            let a = i;
            while i < flags.len() && flags[i] {
                i += 1;
            }
            if i - a >= min_len {
                out.push((a, i));
            }
        } else {
            i += 1;
        }
    }
    out
}

fn fmt_runs(r: &[(usize, usize)], fps: f64) -> String {
    let v: Vec<String> = r.iter().take(12).map(|&(a, b)| format!("f{a}-{} ({})", b - 1, tc(a as f64 / fps))).collect();
    let more = if r.len() > 12 { format!(" … +{}", r.len() - 12) } else { String::new() };
    v.join(", ") + &more
}

#[derive(Serialize)]
pub struct QaReport<'a> {
    pub source: &'a str,
    pub format: &'a str,
    pub worst: Level,
    pub checks: &'a [Check],
    pub loudness: Loudness,
    pub shots: &'a [crate::frames::Shot],
    pub sheets: Vec<String>,
}

pub fn run_qa(fa: &FrameAnalysis, fmt_name: &str, fmt: &Format, plan: Option<&Value>, out: &Path, freeze_frames: usize) -> Result<Level> {
    let p = &fa.probe;
    let fps = p.fps;
    let f = &fa.frames;
    let mut planned = planned_frames(plan);
    // transições graduais detectadas e cortes também não contam como defeito
    for &(a, b) in &fa.xfades {
        planned.extend(a.saturating_sub(1)..=b);
    }
    for &c in &fa.cuts {
        planned.extend([c.saturating_sub(1), c]);
    }
    let mut checks = Vec::new();
    let mut ck = |level, name: &str, detail: String| checks.push(Check { level, name: name.into(), detail });

    let size_ok = (p.width == fmt.width && p.height == fmt.height) || fmt.alt_sizes.iter().any(|s| s[0] == p.width && s[1] == p.height);
    let aspect_ok = (p.width as f64 / p.height as f64 - fmt.width as f64 / fmt.height as f64).abs() < 0.01;
    ck(
        if size_ok { Level::OK } else if aspect_ok { Level::WARN } else { Level::FAIL },
        "resolução",
        format!("{}x{} (esperado {}x{} {})", p.width, p.height, fmt.width, fmt.height, fmt.aspect),
    );
    ck(
        if fmt.fps.iter().any(|&x| (x - fps).abs() < 0.05) { Level::OK } else { Level::WARN },
        "fps",
        format!("{fps:.3} (aceitos {:?})", fmt.fps),
    );
    let dur = f.len() as f64 / fps;
    ck(
        if dur >= fmt.duration.min && dur <= fmt.duration.max { Level::OK } else { Level::FAIL },
        "duração",
        format!("{:.2} s, {} frames (faixa {}-{} s, alvo {} s)", dur, f.len(), fmt.duration.min, fmt.duration.max, fmt.duration.target),
    );
    ck(
        if p.vcodec == fmt.video.codec && p.pix_fmt == fmt.video.pix_fmt { Level::OK } else { Level::WARN },
        "codec de vídeo",
        format!("{} {} (esperado {} {})", p.vcodec, p.pix_fmt, fmt.video.codec, fmt.video.pix_fmt),
    );
    let mut ld = Loudness::default();
    match &p.audio {
        None => ck(Level::FAIL, "áudio", "sem trilha de áudio (o Remotion grava trilha muda: faça o mux)".into()),
        Some(a) => {
            ck(
                if a.sample_rate == fmt.audio.sample_rate { Level::OK } else { Level::WARN },
                "áudio",
                format!("{} {} Hz {}ch", a.codec, a.sample_rate, a.channels),
            );
            ld = loudness(Path::new(&p.path))?;
            if let Some(i) = ld.integrated {
                let d = (i - fmt.lufs).abs();
                ck(
                    if d <= fmt.lufs_tolerance { Level::OK } else { Level::WARN },
                    "loudness",
                    format!("{i:.1} LUFS (alvo {} ±{}) LRA {:.1}", fmt.lufs, fmt.lufs_tolerance, ld.lra.unwrap_or(0.0)),
                );
                if i < -60.0 {
                    ck(Level::FAIL, "áudio mudo", "loudness integrado abaixo de -60 LUFS".into());
                }
            }
            if let Some(tp) = ld.true_peak {
                ck(if tp <= fmt.true_peak + 0.1 { Level::OK } else { Level::WARN }, "true peak", format!("{tp:.1} dBTP (máx {})", fmt.true_peak));
            }
        }
    }

    let free = |i: usize| !planned.contains(&i);
    let black: Vec<bool> = f.iter().map(|x| x.luma < 0.035 && free(x.i)).collect();
    let white: Vec<bool> = f.iter().map(|x| x.luma > 0.95 && x.sat < 0.03 && free(x.i)).collect();
    let frozen: Vec<bool> = f.iter().map(|x| x.i > 0 && x.motion < 0.0015 && free(x.i)).collect();
    let br = runs(&black, 1);
    let wr = runs(&white, 1);
    let fr = runs(&frozen, freeze_frames);
    ck(
        if br.is_empty() { Level::OK } else if br.iter().all(|r| r.1 - r.0 <= 2) { Level::WARN } else { Level::FAIL },
        "frames pretos",
        if br.is_empty() { "nenhum".into() } else { fmt_runs(&br, fps) },
    );
    ck(if wr.is_empty() { Level::OK } else { Level::WARN }, "frames brancos", if wr.is_empty() { "nenhum".into() } else { fmt_runs(&wr, fps) });
    ck(
        if fr.is_empty() { Level::OK } else { Level::WARN },
        "congelados",
        if fr.is_empty() { format!("nenhum hold ≥ {freeze_frames} f") } else { format!("≥{freeze_frames} f: {}", fmt_runs(&fr, fps)) },
    );
    let mut pops = Vec::new();
    for i in 1..f.len().saturating_sub(1) {
        let (m0, m1) = (f[i].motion, f[i + 1].motion);
        if m0 > 0.06 && m1 > 0.06 && f[i + 1].skip2 < 0.3 * m0.min(m1) && free(i) {
            pops.push((i, i + 1));
        }
    }
    ck(if pops.is_empty() { Level::OK } else { Level::FAIL }, "pops de 1 frame", if pops.is_empty() { "nenhum".into() } else { fmt_runs(&pops, fps) });

    // sheets densas
    let (w, h) = (fa.opts.width, fa.opts.height);
    let tiles: Vec<Tile> = fa
        .thumbs
        .iter()
        .map(|t| Tile { img: sheet::rgb_to_image(w, h, t.rgb.clone()), label: format!("{} {}", t.i, tc(t.t)) })
        .collect();
    let (cols, rows) = if h > w { (8, 3) } else { (6, 6) };
    let sheets_dir = out.join("sheets");
    crate::util::ensure_dir(&sheets_dir)?;
    let sheets = sheet::contact_sheets(tiles, cols, rows, (w as u32).min(240), &sheets_dir.join("qa_"))?;

    let worst = checks.iter().map(|c| c.level).max().unwrap_or(Level::OK);
    let mut md = format!("# QA — {}\n\nFormato **{fmt_name}** ({}) · resultado **{:?}** · decode {:.1} s\n\n| | Checagem | Detalhe |\n|---|---|---|\n", p.path, fmt.label, worst, fa.decode_secs);
    for c in &checks {
        md += &format!("| {:?} | {} | {} |\n", c.level, c.name, c.detail);
    }
    md += "\n## Planos detectados\n\n| # | frames | início | dur | luma | croma | movimento |\n|---|---|---|---|---|---|---|\n";
    for s in &fa.shots {
        md += &format!("| {} | {}-{} | {} | {:.2} s | {:.2} | {:.2} | {:.3} |\n", s.id, s.a, s.b - 1, tc(s.ta), s.dur, s.luma, s.sat, s.motion);
    }
    md += "\n## Contact sheets\n\n";
    for s in &sheets {
        md += &format!("- {}\n", s.display());
    }
    std::fs::write(out.join("qa.md"), &md)?;
    write_json(
        &out.join("qa.json"),
        &QaReport {
            source: &p.path,
            format: fmt_name,
            worst,
            checks: &checks,
            loudness: ld,
            shots: &fa.shots,
            sheets: sheets.iter().map(|s| s.display().to_string()).collect(),
        },
    )?;
    print!("{md}");
    Ok(worst)
}
