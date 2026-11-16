//! `vf analyze`: estuda um vídeo de referência. Decode/cortes (CPU) e transcrição (GPU) rodam ao mesmo tempo.
use crate::frames::{self, FrameAnalysis, FramesOpts, Shot};
use crate::sheet::{self, Tile};
use crate::transcribe::{self, TranscribeOpts, Word};
use crate::util::{ensure_dir, median, run, tc, which, write_json};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Serialize, Default, Debug)]
pub struct Pace {
    pub duration: f64,
    pub shots: usize,
    pub cuts_per_min: f64,
    pub asl_mean: f64,
    pub asl_median: f64,
    pub shortest: f64,
    pub longest: f64,
    /// planos com <1 s, 1-2, 2-4, 4-8, >8 s
    pub shot_len_buckets: [usize; 5],
    pub words: usize,
    pub wpm: f64,
    pub words_per_sec: f64,
    pub speech_start: Option<f64>,
    pub hook_end: Option<f64>,
    pub hook_text: Option<String>,
    pub pauses_total: f64,
    pub longest_pause: f64,
    pub speech_ratio: f64,
    pub motion_mean: f32,
    pub luma_mean: f32,
    pub sat_mean: f32,
}

#[derive(Serialize)]
pub struct ShotOut<'a> {
    #[serde(flatten)]
    pub shot: &'a Shot,
    pub keyframe: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct Analysis<'a> {
    pub source: String,
    pub meta: Value,
    pub probe: &'a crate::probe::Probe,
    pub pace: &'a Pace,
    pub shots: Vec<ShotOut<'a>>,
    pub sheets: Vec<String>,
    pub timings: Value,
}

pub struct AnalyzeOpts {
    pub frames: FramesOpts,
    pub transcribe: Option<TranscribeOpts>,
    pub keyframe_side: u32,
}

fn download(url: &str, out: &Path) -> Result<(PathBuf, Value)> {
    let bin = which("yt-dlp").context("yt-dlp não encontrado: `brew install yt-dlp`")?;
    run(Command::new(bin)
        .args(["-f", "bv*[height<=1080]+ba/b[height<=1080]/b", "--merge-output-format", "mp4", "--write-info-json", "--no-playlist", "-o"])
        .arg(out.join("source.%(ext)s"))
        .arg(url))?;
    let video = std::fs::read_dir(out)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("source.") && !n.ends_with(".json")))
        .context("download sem arquivo de vídeo")?;
    let info: Value = std::fs::read(out.join("source.info.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    let keep = ["title", "uploader", "channel", "upload_date", "duration", "view_count", "like_count", "comment_count", "description", "tags", "webpage_url"];
    let meta = keep.iter().filter_map(|k| info.get(*k).map(|v| (k.to_string(), v.clone()))).collect::<serde_json::Map<_, _>>();
    Ok((video, Value::Object(meta)))
}

pub fn pace(fa: &FrameAnalysis, words: &[Word]) -> Pace {
    let dur = fa.frames.len() as f64 / fa.probe.fps;
    let mut lens: Vec<f64> = fa.shots.iter().map(|s| s.dur).collect();
    let mut b = [0usize; 5];
    for &l in &lens {
        b[match l {
            l if l < 1.0 => 0,
            l if l < 2.0 => 1,
            l if l < 4.0 => 2,
            l if l < 8.0 => 3,
            _ => 4,
        }] += 1;
    }
    let mut p = Pace {
        duration: dur,
        shots: fa.shots.len(),
        cuts_per_min: fa.cuts.len() as f64 / (dur / 60.0).max(1e-9),
        asl_mean: dur / fa.shots.len().max(1) as f64,
        shortest: lens.iter().cloned().fold(f64::INFINITY, f64::min),
        longest: lens.iter().cloned().fold(0.0, f64::max),
        asl_median: median(&mut lens),
        shot_len_buckets: b,
        motion_mean: crate::util::mean(&fa.frames.iter().map(|f| f.motion).collect::<Vec<_>>()),
        luma_mean: crate::util::mean(&fa.frames.iter().map(|f| f.luma).collect::<Vec<_>>()),
        sat_mean: crate::util::mean(&fa.frames.iter().map(|f| f.sat).collect::<Vec<_>>()),
        ..Default::default()
    };
    if let (Some(first), Some(last)) = (words.first(), words.last()) {
        let span = (last.end - first.start).max(1e-6);
        p.words = words.len();
        p.wpm = words.len() as f64 / span * 60.0;
        p.words_per_sec = words.len() as f64 / span;
        p.speech_start = Some(first.start);
        let mut speech = 0.0;
        for w in words {
            speech += w.end - w.start;
        }
        p.speech_ratio = speech / dur.max(1e-6);
        for pair in words.windows(2) {
            let g = pair[1].start - pair[0].end;
            if g > 0.25 {
                p.pauses_total += g;
            }
            p.longest_pause = p.longest_pause.max(g);
        }
        if let Some(k) = words.iter().position(|w| w.word.ends_with(['.', '!', '?'])) {
            p.hook_end = Some(words[k].end);
            p.hook_text = Some(words[..=k].iter().map(|w| w.word.as_str()).collect::<Vec<_>>().join(" "));
        }
    }
    p
}

pub fn analyze(input: &str, out: &Path, o: &AnalyzeOpts) -> Result<()> {
    ensure_dir(out)?;
    let t0 = Instant::now();
    let (video, meta) = if input.starts_with("http://") || input.starts_with("https://") {
        download(input, out)?
    } else {
        (PathBuf::from(input), Value::Null)
    };
    let t_dl = t0.elapsed().as_secs_f64();

    let t1 = Instant::now();
    let (fa, words) = std::thread::scope(|s| {
        let tr = o.transcribe.as_ref().map(|to| {
            let v = &video;
            s.spawn(move || -> Result<Vec<Word>> { transcribe::transcribe(v, out, to) })
        });
        let fa = frames::analyze(&video, &o.frames);
        let words = match tr {
            Some(h) => match h.join().expect("thread de transcrição") {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("transcrição pulada: {e:#}");
                    vec![]
                }
            },
            None => vec![],
        };
        (fa, words)
    });
    let fa = fa?;
    let t_main = t1.elapsed().as_secs_f64();
    frames::write_outputs(&fa, out)?;
    if !words.is_empty() {
        transcribe::write_words(&words, out)?;
    }

    let t2 = Instant::now();
    let kdir = out.join("keyframes");
    ensure_dir(&kdir)?;
    let times: Vec<f64> = fa.shots.iter().map(|s| (s.ta + s.tb) / 2.0).collect();
    let kfs = sheet::extract_keyframes(&video, &times, &kdir, "shot_", o.keyframe_side, o.frames.hwaccel)?;
    let tiles: Vec<Tile> = kfs
        .iter()
        .zip(&fa.shots)
        .filter_map(|(p, s)| {
            let img = image::open(p).ok()?.to_rgb8();
            Some(Tile { img, label: format!("#{} {} {:.1}s", s.id, tc(s.ta), s.dur) })
        })
        .collect();
    let vertical = fa.probe.height > fa.probe.width;
    let (cols, rows) = if vertical { (6, 2) } else { (4, 5) };
    let sdir = out.join("sheets");
    ensure_dir(&sdir)?;
    let sheets = sheet::contact_sheets(tiles, cols, rows, if vertical { 220 } else { 320 }, &sdir.join("shots_"))?;
    let t_kf = t2.elapsed().as_secs_f64();

    let pc = pace(&fa, &words);
    let shot_text = |s: &Shot| -> String {
        words
            .iter()
            .filter(|w| {
                let m = (w.start + w.end) / 2.0;
                m >= s.ta && m < s.tb
            })
            .map(|w| w.word.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let shots: Vec<ShotOut> = fa
        .shots
        .iter()
        .zip(kfs.iter())
        .map(|(s, k)| ShotOut { shot: s, keyframe: k.strip_prefix(out).unwrap_or(k).display().to_string(), text: shot_text(s) })
        .collect();
    let timings = serde_json::json!({"download_s": t_dl, "decode_transcribe_s": t_main, "decode_s": fa.decode_secs, "keyframes_s": t_kf, "total_s": t0.elapsed().as_secs_f64()});
    let sheet_rel: Vec<String> = sheets.iter().map(|p| p.strip_prefix(out).unwrap_or(p).display().to_string()).collect();

    // relatório para leitura humana/Claude
    let p = &fa.probe;
    let mut md = String::new();
    md += &format!("# Análise — {}\n\n", meta["title"].as_str().unwrap_or(&p.path));
    if let Some(u) = meta["webpage_url"].as_str() {
        md += &format!("Fonte: {u} · canal: {} · views: {}\n\n", meta["channel"].as_str().or(meta["uploader"].as_str()).unwrap_or("?"), meta["view_count"]);
    }
    md += &format!(
        "**Specs:** {}x{} @ {:.2} fps, {:.1} s, {} · áudio: {}\n\n",
        p.width,
        p.height,
        p.fps,
        pc.duration,
        p.vcodec,
        p.audio.as_ref().map(|a| format!("{} {} Hz", a.codec, a.sample_rate)).unwrap_or("nenhum".into())
    );
    md += "## Ritmo\n\n";
    md += &format!(
        "- Planos: **{}** · cortes/min: **{:.1}** · duração média {:.2} s, mediana {:.2} s (min {:.2}, máx {:.2})\n",
        pc.shots, pc.cuts_per_min, pc.asl_mean, pc.asl_median, pc.shortest, pc.longest
    );
    md += &format!("- Distribuição de planos: <1 s {} · 1-2 s {} · 2-4 s {} · 4-8 s {} · >8 s {}\n", pc.shot_len_buckets[0], pc.shot_len_buckets[1], pc.shot_len_buckets[2], pc.shot_len_buckets[3], pc.shot_len_buckets[4]);
    if pc.words > 0 {
        md += &format!(
            "- Fala: {} palavras · **{:.0} WPM** ({:.2} palavras/s) · começa em {:.2} s · fala ocupa {:.0}% · pausas >0,25 s somam {:.2} s (maior {:.2} s)\n",
            pc.words,
            pc.wpm,
            pc.words_per_sec,
            pc.speech_start.unwrap_or(0.0),
            pc.speech_ratio * 100.0,
            pc.pauses_total,
            pc.longest_pause
        );
        if let (Some(h), Some(t)) = (pc.hook_end, &pc.hook_text) {
            md += &format!("- **Gancho** (1ª frase, termina em {h:.2} s): \"{t}\"\n");
        }
    } else {
        md += "- Sem transcrição.\n";
    }
    md += &format!("- Visual: luma média {:.2}, croma {:.2}, movimento médio {:.3}\n\n", pc.luma_mean, pc.sat_mean, pc.motion_mean);
    md += "## Contact sheets (leia estas imagens primeiro)\n\n";
    for s in &sheet_rel {
        md += &format!("- {s}\n");
    }
    md += "\n## Planos\n\n| # | início | dur | mov. | fala | keyframe |\n|---|---|---|---|---|---|\n";
    for s in &shots {
        md += &format!("| {} | {} | {:.2} s | {:.3} | {} | {} |\n", s.shot.id, tc(s.shot.ta), s.shot.dur, s.shot.motion, s.text.replace('|', "/"), s.keyframe);
    }
    if !words.is_empty() {
        md += "\n## Transcrição\n\n";
        md += &std::fs::read_to_string(out.join("transcript.txt")).unwrap_or_default();
    }
    md += &format!(
        "\n---\nTempos: download {:.1} s · decode+transcrição {:.1} s (decode {:.1} s) · keyframes {:.1} s · total {:.1} s\n",
        t_dl,
        t_main,
        fa.decode_secs,
        t_kf,
        t0.elapsed().as_secs_f64()
    );
    std::fs::write(out.join("analysis.md"), &md)?;
    write_json(
        &out.join("analysis.json"),
        &Analysis { source: p.path.clone(), meta, probe: p, pace: &pc, shots, sheets: sheet_rel, timings },
    )?;
    eprintln!("análise em {} ({:.1} s)", out.join("analysis.md").display(), t0.elapsed().as_secs_f64());
    Ok(())
}
