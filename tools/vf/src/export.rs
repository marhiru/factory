//! Export por formato: enquadramento (pad/crop/blur), encode, loudnorm em duas passadas e mux.
use crate::formats::Format;
use crate::probe::{probe, Probe};
use crate::util::{ffmpeg, run};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Fit {
    /// barras (letterbox/pillarbox)
    Pad,
    /// corta o excesso (centro)
    Crop,
    /// fundo com cópia desfocada (bom para 16:9 -> 9:16)
    Blur,
}

pub struct ExportOpts {
    pub audio: Option<PathBuf>,
    pub fit: Fit,
    pub fast: bool,
    pub loudnorm: bool,
    pub reencode: bool,
    pub size: Option<(u32, u32)>,
}

fn measure_loudness(src: &Path, map: &str, fmt: &Format) -> Result<Value> {
    let af = format!("loudnorm=I={}:TP={}:LRA=11:print_format=json", fmt.lufs, fmt.true_peak);
    let out = Command::new(ffmpeg())
        .args(["-hide_banner", "-nostdin", "-nostats", "-i"])
        .arg(src)
        .args(["-map", map, "-af", &af, "-f", "null", "-"])
        .output()?;
    let e = String::from_utf8_lossy(&out.stderr);
    let a = e.rfind('{').context("loudnorm não retornou medidas")?;
    let b = e.rfind('}').context("loudnorm json")?;
    Ok(serde_json::from_str(&e[a..=b])?)
}

pub fn export(input: &Path, out: &Path, fmt: &Format, o: &ExportOpts) -> Result<()> {
    let p: Probe = probe(input)?;
    let (tw, th) = o.size.unwrap_or_else(|| {
        if fmt.alt_sizes.iter().any(|s| s[0] == p.width && s[1] == p.height) {
            (p.width, p.height)
        } else {
            (fmt.width, fmt.height)
        }
    });
    let fps_ok = fmt.fps.iter().any(|&x| (x - p.fps).abs() < 0.05);
    let geometry_ok = p.width == tw && p.height == th && fps_ok;
    let copy_video = geometry_ok && !o.reencode && p.vcodec == fmt.video.codec && p.pix_fmt == fmt.video.pix_fmt;

    let (audio_src, audio_map): (&Path, String) = match &o.audio {
        Some(a) => (a.as_path(), "1:a:0".into()),
        None => {
            if p.audio.is_none() {
                bail!("{} não tem áudio; passe --audio mix.wav", input.display());
            }
            (input, "0:a:0".into())
        }
    };
    let mut af = format!("aresample={}", fmt.audio.sample_rate);
    if o.loudnorm {
        let m = measure_loudness(audio_src, "0:a:0", fmt)?;
        let g = |k: &str| m[k].as_str().unwrap_or("0").to_string();
        af = format!(
            "loudnorm=I={}:TP={}:LRA=11:measured_I={}:measured_TP={}:measured_LRA={}:measured_thresh={}:offset={}:linear=true,aresample={}",
            fmt.lufs,
            fmt.true_peak,
            g("input_i"),
            g("input_tp"),
            g("input_lra"),
            g("input_thresh"),
            g("target_offset"),
            fmt.audio.sample_rate
        );
    }

    let mut cmd = Command::new(ffmpeg());
    cmd.args(["-hide_banner", "-loglevel", "error", "-stats", "-nostdin", "-y", "-i"]).arg(input);
    if let Some(a) = &o.audio {
        cmd.arg("-i").arg(a);
    }
    let fps = if fps_ok { String::new() } else { ",fps=30".to_string() };
    if copy_video {
        cmd.args(["-map", "0:v:0", "-c:v", "copy"]);
    } else {
        let graph = match o.fit {
            Fit::Pad => format!("[0:v]scale={tw}:{th}:force_original_aspect_ratio=decrease:flags=lanczos,pad={tw}:{th}:(ow-iw)/2:(oh-ih)/2:black,setsar=1{fps},scale=out_range=tv,format=yuv420p[v]"),
            Fit::Crop => format!("[0:v]scale={tw}:{th}:force_original_aspect_ratio=increase:flags=lanczos,crop={tw}:{th},setsar=1{fps},scale=out_range=tv,format=yuv420p[v]"),
            Fit::Blur => format!(
                "[0:v]split[a][b];[a]scale={tw}:{th}:force_original_aspect_ratio=increase,crop={tw}:{th},gblur=sigma=40,eq=brightness=-0.08[bg];\
                 [b]scale={tw}:{th}:force_original_aspect_ratio=decrease:flags=lanczos[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2,setsar=1{fps},scale=out_range=tv,format=yuv420p[v]"
            ),
        };
        cmd.args(["-filter_complex", &graph, "-map", "[v]", "-color_range", "tv", "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709"]);
        if o.fast {
            cmd.args(["-c:v", "h264_videotoolbox", "-q:v", "70", "-profile:v", "high"]);
        } else {
            cmd.args(["-c:v", "libx264", "-crf", &fmt.video.crf.to_string(), "-preset", &fmt.video.preset, "-profile:v", "high"]);
        }
    }
    cmd.args(["-map", &audio_map, "-af", &af, "-c:a", "aac", "-b:a", &fmt.audio.bitrate, "-ar", &fmt.audio.sample_rate.to_string()]);
    cmd.args(["-shortest", "-movflags", "+faststart"]).arg(out);
    run(&mut cmd)?;
    let dur = probe(out)?.duration;
    eprintln!(
        "export {} -> {} ({}x{}, {:.2} s, vídeo {})",
        input.display(),
        out.display(),
        tw,
        th,
        dur,
        if copy_video { "copiado" } else { "recodificado" }
    );
    if dur > fmt.duration.max {
        eprintln!("ATENÇÃO: {:.1} s passa do máximo de {} s para {}", dur, fmt.duration.max, fmt.label);
    }
    Ok(())
}
