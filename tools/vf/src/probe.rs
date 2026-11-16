use crate::util::{ffprobe, run};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::process::Command;

#[derive(Serialize, Clone, Debug)]
pub struct AudioInfo {
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct Probe {
    pub path: String,
    pub duration: f64,
    /// Já considerando rotação (largura/altura de exibição).
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub fps_str: String,
    pub nb_frames: u64,
    pub vcodec: String,
    pub pix_fmt: String,
    pub bitrate: Option<u64>,
    pub size_bytes: u64,
    pub audio: Option<AudioInfo>,
}

fn rate(s: &str) -> Option<f64> {
    let (a, b) = s.split_once('/').unwrap_or((s, "1"));
    let (a, b): (f64, f64) = (a.parse().ok()?, b.parse().ok()?);
    (a > 0.0 && b > 0.0).then(|| a / b)
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str()?.parse().ok())
}

pub fn probe(path: &Path) -> Result<Probe> {
    let out = run(Command::new(ffprobe()).args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"]).arg(path))?;
    let j: Value = serde_json::from_slice(&out.stdout).context("ffprobe json")?;
    let streams = j["streams"].as_array().cloned().unwrap_or_default();
    let v = streams
        .iter()
        .find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"].as_i64() != Some(1))
        .with_context(|| format!("{} não tem stream de vídeo", path.display()))?;
    let a = streams.iter().find(|s| s["codec_type"] == "audio");

    let (mut fps_str, mut fps) = ("30/1".to_string(), 30.0);
    for key in ["avg_frame_rate", "r_frame_rate"] {
        if let Some(s) = v[key].as_str() {
            if let Some(r) = rate(s) {
                if r < 500.0 {
                    fps_str = s.to_string();
                    fps = r;
                    break;
                }
            }
        }
    }
    let duration = num(&j["format"]["duration"]).or_else(|| num(&v["duration"])).unwrap_or(0.0);
    let nb_frames = num(&v["nb_frames"]).map(|n| n as u64).filter(|&n| n > 0).unwrap_or((duration * fps).round() as u64);

    let mut rot = num(&v["tags"]["rotate"]).unwrap_or(0.0);
    if let Some(sd) = v["side_data_list"].as_array() {
        for d in sd {
            if let Some(r) = num(&d["rotation"]) {
                rot = r;
            }
        }
    }
    let (mut w, mut h) = (v["width"].as_u64().unwrap_or(0) as u32, v["height"].as_u64().unwrap_or(0) as u32);
    if (rot.abs() as i64) % 180 == 90 {
        std::mem::swap(&mut w, &mut h);
    }

    Ok(Probe {
        path: path.display().to_string(),
        duration,
        width: w,
        height: h,
        fps,
        fps_str,
        nb_frames,
        vcodec: v["codec_name"].as_str().unwrap_or("?").into(),
        pix_fmt: v["pix_fmt"].as_str().unwrap_or("?").into(),
        bitrate: num(&j["format"]["bit_rate"]).map(|b| b as u64),
        size_bytes: num(&j["format"]["size"]).map(|b| b as u64).unwrap_or(0),
        audio: a.map(|a| AudioInfo {
            codec: a["codec_name"].as_str().unwrap_or("?").into(),
            sample_rate: num(&a["sample_rate"]).unwrap_or(0.0) as u32,
            channels: a["channels"].as_u64().unwrap_or(0) as u32,
        }),
    })
}
