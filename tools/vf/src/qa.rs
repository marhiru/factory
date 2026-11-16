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

#[derive(Serialize, Default, Debug, Clone)]
pub struct Loudness {
    pub integrated: Option<f64>,
    pub lra: Option<f64>,
    pub true_peak: Option<f64>,
    /// pico de amostra (dBFS), do astats
    pub sample_peak: Option<f64>,
    /// trechos de silêncio [início, fim] em s
    pub silences: Vec<(f64, f64)>,
}

/// silencedetect: pares (início, fim) dos trechos abaixo de `noise_db` por `min_s` ou mais
fn parse_silences(e: &str, total: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut open: Option<f64> = None;
    for l in e.lines() {
        if let Some(i) = l.find("silence_start: ") {
            open = l[i + 15..].split_whitespace().next().and_then(|v| v.parse().ok());
        } else if let Some(i) = l.find("silence_end: ") {
            let end: f64 = l[i + 13..].split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(total);
            out.push((open.take().unwrap_or(0.0).max(0.0), end));
        }
    }
    if let Some(a) = open {
        out.push((a.max(0.0), total));
    }
    out
}

/// Uma passada de ffmpeg: silêncio + pico de amostra + loudness EBU R128.
pub fn audio_analysis(input: &Path, total: f64, noise_db: f64, min_s: f64) -> Result<Loudness> {
    let af = format!("silencedetect=n={noise_db}dB:d={min_s},astats=measure_perchannel=none,ebur128=peak=true");
    let out = Command::new(ffmpeg())
        .args(["-hide_banner", "-nostdin", "-nostats", "-i"])
        .arg(input)
        .args(["-map", "0:a:0", "-af", &af, "-f", "null", "-"])
        .output()?;
    let e = String::from_utf8_lossy(&out.stderr);
    let tail = &e[e.rfind("Summary:").unwrap_or(0)..];
    let grab = |key: &str| -> Option<f64> {
        tail.lines()
            .map(str::trim)
            .find(|l| l.starts_with(key))
            .and_then(|l| l[key.len()..].split_whitespace().next()?.parse().ok())
    };
    let sample_peak = e
        .lines()
        .filter(|l| l.contains("Peak level dB:"))
        .filter_map(|l| l.rsplit(':').next()?.trim().parse::<f64>().ok())
        .fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v))));
    Ok(Loudness {
        integrated: grab("I:"),
        lra: grab("LRA:"),
        true_peak: grab("Peak:"),
        sample_peak,
        silences: parse_silences(&e, total),
    })
}

/// Exceções aceitas do QA estrito: `qa_excecoes` do factory.yaml, lista de {check, motivo}.
pub fn load_exceptions(factory: &Path) -> Vec<(String, String)> {
    let Ok(t) = std::fs::read_to_string(factory) else { return vec![] };
    let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(&t) else { return vec![] };
    v.get("qa_excecoes")
        .and_then(|x| x.as_sequence())
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let c = e.get("check")?.as_str()?.to_string();
            let m = e.get("motivo").and_then(|m| m.as_str()).unwrap_or("").to_string();
            (!m.trim().is_empty()).then_some((c, m))
        })
        .collect()
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
    pub strict: bool,
    pub loudness: Loudness,
    pub shots: &'a [crate::frames::Shot],
    pub sheets: Vec<String>,
}

pub struct QaOpts<'a> {
    pub plan: Option<&'a Value>,
    /// sobrescreve qa.freeze_frames do formato
    pub freeze_frames: Option<usize>,
    /// estrito: WARN vira FAIL (exceto exceções com motivo)
    pub strict: bool,
    pub exceptions: Vec<(String, String)>,
    /// trilha de voz isolada, para checar buracos na narração
    pub voice: Option<&'a Path>,
}

pub fn run_qa(fa: &FrameAnalysis, fmt_name: &str, fmt: &Format, o: &QaOpts, out: &Path) -> Result<Level> {
    let plan = o.plan;
    let q = &fmt.qa;
    let freeze_frames = o.freeze_frames.unwrap_or(q.freeze_frames);
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
    if let Some([a, b]) = q.duration_strict {
        ck(
            if dur >= a && dur <= b { Level::OK } else { Level::FAIL },
            "duração × alvo",
            format!("{dur:.1} s (entrega aceita {a}-{b} s para {})", fmt.label),
        );
    }
    let mbps = p.bitrate.map(|b| b as f64 / 1e6).unwrap_or_else(|| p.size_bytes as f64 * 8.0 / dur.max(0.01) / 1e6);
    ck(
        if mbps <= q.max_bitrate_mbps { Level::OK } else { Level::FAIL },
        "bitrate",
        format!("{mbps:.1} Mb/s, {:.0} MB (máx {} Mb/s: reencode com `vf export --reencode`)", p.size_bytes as f64 / 1e6, q.max_bitrate_mbps),
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
            ld = audio_analysis(Path::new(&p.path), dur, -50.0, q.max_silence_s.min(0.5))?;
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
            if let Some(sp) = ld.sample_peak {
                ck(if sp < -0.1 { Level::OK } else { Level::FAIL }, "clipping", format!("pico de amostra {sp:.2} dBFS"));
            }
            // abertura: som começa já no primeiro instante
            let start = ld.silences.first().filter(|s| s.0 <= 0.01).map_or(0.0, |s| s.1);
            ck(
                if start <= q.audio_start_s { Level::OK } else { Level::FAIL },
                "abertura do áudio",
                format!("som começa em {start:.2} s (máx {} s: sem silêncio antes do gancho)", q.audio_start_s),
            );
            let dead: Vec<String> = ld
                .silences
                .iter()
                .filter(|s| s.1 - s.0 > q.max_silence_s && s.0 > 0.01 && s.1 < dur - 0.6)
                .map(|s| format!("{}-{} ({:.1} s)", tc(s.0), tc(s.1), s.1 - s.0))
                .collect();
            ck(
                if dead.is_empty() { Level::OK } else { Level::FAIL },
                "ar morto",
                if dead.is_empty() { format!("nenhum silêncio > {} s", q.max_silence_s) } else { dead.join(", ") },
            );
        }
    }
    if let Some(v) = o.voice {
        let vd = crate::probe::probe(v).map(|x| x.duration).unwrap_or(dur);
        let va = audio_analysis(v, vd, -40.0, q.max_voice_gap_s)?;
        let gaps: Vec<String> = va
            .silences
            .iter()
            .filter(|s| s.0 > 0.01 && s.1 < vd - 0.3)
            .map(|s| format!("{}-{} ({:.1} s)", tc(s.0), tc(s.1), s.1 - s.0))
            .collect();
        ck(
            if gaps.is_empty() { Level::OK } else { Level::WARN },
            "buracos na voz",
            if gaps.is_empty() { format!("nenhuma pausa > {} s", q.max_voice_gap_s) } else { gaps.join(", ") },
        );
    }

    let free = |i: usize| !planned.contains(&i);
    let black: Vec<bool> = f.iter().map(|x| x.luma < 0.035 && free(x.i)).collect();
    let white: Vec<bool> = f.iter().map(|x| x.luma > 0.95 && x.sat < 0.03 && free(x.i)).collect();
    // hold de verdade: quase nada muda frame a frame E nada muda em ~0,5 s (zoom lento não é hold)
    let frozen: Vec<bool> = f.iter().map(|x| x.i > 0 && x.motion < 0.0015 && x.slow < 0.002 && free(x.i)).collect();
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
        if fr.is_empty() { Level::OK } else { Level::FAIL },
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

    // planos: vida, duração, início, repetição, bordas
    let dead: Vec<String> = fa
        .shots
        .iter()
        .filter(|s| s.dur >= 1.0 && s.slow < q.min_life)
        .map(|s| format!("#{} {} {:.1} s (vida {:.4})", s.id, tc(s.ta), s.dur, s.slow))
        .collect();
    ck(
        if dead.is_empty() { Level::OK } else { Level::FAIL },
        "planos sem vida",
        if dead.is_empty() { format!("todos com vida ≥ {}", q.min_life) } else { dead.join(", ") },
    );
    let long: Vec<String> = fa
        .shots
        .iter()
        .filter(|s| s.dur > q.max_shot_s)
        .map(|s| format!("#{} {} {:.1} s", s.id, tc(s.ta), s.dur))
        .collect();
    ck(
        if long.is_empty() { Level::OK } else { Level::WARN },
        "planos longos",
        if long.is_empty() { format!("nenhum acima de {} s", q.max_shot_s) } else { format!("> {} s: {}", q.max_shot_s, long.join(", ")) },
    );
    let hook_f = (q.hook_s * fps) as usize;
    let hook_cut = fa.cuts.iter().any(|&c| c <= hook_f);
    let hook_mv = f.iter().take(hook_f).skip(1).map(|x| x.slow).fold(0f32, f32::max);
    ck(
        if hook_cut || hook_mv >= 0.02 { Level::OK } else { Level::FAIL },
        "gancho visual",
        format!("primeiros {} s: {} corte(s), movimento máx {:.3} (precisa corte ou movimento ≥ 0,02)", q.hook_s, fa.cuts.iter().filter(|&&c| c <= hook_f).count(), hook_mv),
    );
    if q.repeat_window_s > 0.0 {
        let mut rep = Vec::new();
        for (i, a) in fa.shots.iter().enumerate() {
            for b in fa.shots.iter().skip(i + 2) {
                if b.ta - a.ta >= q.repeat_window_s {
                    break;
                }
                if (a.ahash ^ b.ahash).count_ones() <= 3 && a.slow > 0.0 {
                    rep.push(format!("#{} {} = #{} {}", a.id, tc(a.ta), b.id, tc(b.ta)));
                }
            }
        }
        ck(
            if rep.is_empty() { Level::OK } else { Level::WARN },
            "imagem repetida",
            if rep.is_empty() { format!("nenhuma repetição em < {} s", q.repeat_window_s) } else { rep.into_iter().take(12).collect::<Vec<_>>().join(", ") },
        );
    }
    let names = ["topo", "base", "esquerda", "direita"];
    let lit: Vec<&crate::frames::FrameStat> = f.iter().filter(|x| x.luma > 0.08).collect();
    let mut sides = Vec::new();
    for k in 0..4 {
        let n = lit.iter().filter(|x| x.bars[k]).count();
        if !lit.is_empty() && n as f64 / lit.len() as f64 > 0.3 {
            sides.push(format!("{} ({:.0}% dos frames)", names[k], 100.0 * n as f64 / lit.len() as f64));
        }
    }
    ck(
        if sides.is_empty() { Level::OK } else { Level::FAIL },
        "barras pretas",
        if sides.is_empty() { "nenhuma".into() } else { format!("{}: enquadramento errado (use --fit crop/blur ou refaça no tamanho do formato)", sides.join(", ")) },
    );

    // estrito: WARN vira FAIL, a não ser que haja exceção com motivo no factory.yaml
    if o.strict {
        for c in checks.iter_mut() {
            if let Some((_, m)) = o.exceptions.iter().find(|(n, _)| *n == c.name) {
                if c.level != Level::OK {
                    c.detail = format!("{} · EXCEÇÃO: {m}", c.detail);
                    c.level = Level::WARN;
                }
            } else if c.level == Level::WARN {
                c.level = Level::FAIL;
                c.detail = format!("{} · (estrito: WARN = FAIL)", c.detail);
            }
        }
    }

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
    let mut md = format!(
        "# QA — {}\n\nFormato **{fmt_name}** ({}) · modo **{}** · resultado **{:?}** · decode {:.1} s\n\n| | Checagem | Detalhe |\n|---|---|---|\n",
        p.path,
        fmt.label,
        if o.strict { "estrito" } else { "leniente" },
        worst,
        fa.decode_secs
    );
    for c in &checks {
        md += &format!("| {:?} | {} | {} |\n", c.level, c.name, c.detail);
    }
    md += "\n## Planos detectados\n\n| # | frames | início | dur | luma | croma | movimento | vida |\n|---|---|---|---|---|---|---|---|\n";
    for s in &fa.shots {
        md += &format!("| {} | {}-{} | {} | {:.2} s | {:.2} | {:.2} | {:.3} | {:.4} |\n", s.id, s.a, s.b - 1, tc(s.ta), s.dur, s.luma, s.sat, s.motion, s.slow);
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
            strict: o.strict,
            loudness: ld,
            shots: &fa.shots,
            sheets: sheets.iter().map(|s| s.display().to_string()).collect(),
        },
    )?;
    print!("{md}");
    Ok(worst)
}
