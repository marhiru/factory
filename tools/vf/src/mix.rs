//! `vf mix`: mixa as faixas de áudio do edl.json numa passada de ffmpeg.
//! role: voice (chave do ducking) | music (abaixa sob a voz) | sfx. Saída WAV 48 kHz 24-bit com a duração exata.
use crate::util::{ffmpeg, run};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

#[derive(Deserialize, Debug)]
pub struct Track {
    pub src: String,
    #[serde(default)]
    pub gain_db: f64,
    #[serde(default)]
    pub start: f64,
    #[serde(default)]
    pub trim: f64,
    #[serde(default)]
    pub r#loop: bool,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub fade_in: f64,
    #[serde(default)]
    pub fade_out: f64,
}

#[derive(Deserialize)]
struct EdlAudio {
    duration: f64,
    #[serde(default)]
    audio: Vec<Track>,
}

pub struct MixOpts {
    /// dB de redução da música enquanto há voz (aprox.)
    pub duck_db: f64,
}

fn role(t: &Track) -> &str {
    match t.role.as_deref() {
        Some(r) => r,
        None if t.src.contains("voice") || t.src.contains("voz") => "voice",
        None if t.src.contains("music") || t.src.contains("musica") || t.src.contains("trilha") => "music",
        None => "sfx",
    }
}

pub fn mix(edl_path: &Path, root: &Path, out: &Path, o: &MixOpts) -> Result<()> {
    let e: EdlAudio = serde_json::from_slice(&std::fs::read(edl_path)?).context("edl.json")?;
    if e.audio.is_empty() {
        bail!("edl.json não tem faixas em \"audio\"");
    }
    let dur = e.duration;
    let mut cmd = Command::new(ffmpeg());
    cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
    for t in &e.audio {
        let p = root.join(&t.src);
        if !p.is_file() {
            bail!("faixa não encontrada: {}", p.display());
        }
        if t.r#loop {
            cmd.args(["-stream_loop", "-1"]);
        }
        cmd.arg("-i").arg(p);
    }
    let mut g = String::new();
    let (mut voices, mut music, mut sfx) = (vec![], vec![], vec![]);
    for (k, t) in e.audio.iter().enumerate() {
        let len = (dur - t.start).max(0.01);
        let delay = (t.start * 1000.0).round() as i64;
        let mut chain = format!(
            "[{k}:a]atrim=start={:.4}:duration={len:.4},asetpts=PTS-STARTPTS,aresample=48000,aformat=sample_fmts=fltp:channel_layouts=stereo,volume={:.2}dB",
            t.trim, t.gain_db
        );
        if t.fade_in > 0.0 {
            chain += &format!(",afade=t=in:d={:.3}", t.fade_in);
        }
        if t.fade_out > 0.0 {
            chain += &format!(",afade=t=out:st={:.3}:d={:.3}", (len - t.fade_out).max(0.0), t.fade_out);
        }
        if delay > 0 {
            chain += &format!(",adelay={delay}|{delay}");
        }
        let lbl = format!("t{k}");
        g += &format!("{chain}[{lbl}];");
        match role(t) {
            "voice" => voices.push(lbl),
            "music" => music.push(lbl),
            _ => sfx.push(lbl),
        }
    }
    let join = |v: &[String], name: &str, g: &mut String| -> Option<String> {
        match v.len() {
            0 => None,
            1 => Some(v[0].clone()),
            n => {
                *g += &format!("{}amix=inputs={n}:normalize=0:duration=longest[{name}];", v.iter().map(|x| format!("[{x}]")).collect::<String>());
                Some(name.to_string())
            }
        }
    };
    let v = join(&voices, "vsum", &mut g);
    let m = join(&music, "msum", &mut g);
    let s = join(&sfx, "ssum", &mut g);
    let mut finals = vec![];
    match (&v, &m) {
        (Some(v), Some(m)) => {
            // ratio alta + threshold baixo: música desce ~duck_db quando a voz está ativa
            let ratio = (o.duck_db / 3.0).clamp(2.0, 20.0);
            g += &format!("[{v}]asplit=2[vout][vkey];[{m}][vkey]sidechaincompress=threshold=0.02:ratio={ratio:.1}:attack=15:release=350:knee=4[mduck];");
            finals.push("vout".to_string());
            finals.push("mduck".to_string());
        }
        (Some(v), None) => finals.push(v.clone()),
        (None, Some(m)) => finals.push(m.clone()),
        _ => {}
    }
    if let Some(s) = s {
        finals.push(s);
    }
    let n = finals.len();
    g += &format!(
        "{}amix=inputs={n}:normalize=0:duration=longest,apad,atrim=duration={dur:.4},alimiter=limit=0.89:level=false[out]",
        finals.iter().map(|x| format!("[{x}]")).collect::<String>()
    );
    cmd.args(["-filter_complex", &g, "-map", "[out]", "-ar", "48000", "-c:a", "pcm_s24le"]).arg(out);
    run(&mut cmd)?;
    eprintln!("mix: {} faixas (voz {}, música {}, sfx {}) -> {} ({dur:.2} s)", e.audio.len(), voices.len(), music.len(), sfx.len(), out.display());
    Ok(())
}
