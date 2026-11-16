use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn ffmpeg() -> String {
    std::env::var("VF_FFMPEG").unwrap_or_else(|_| "ffmpeg".into())
}
pub fn ffprobe() -> String {
    std::env::var("VF_FFPROBE").unwrap_or_else(|_| "ffprobe".into())
}

/// Roda um comando e falha com o stderr se o status não for 0.
pub fn run(cmd: &mut Command) -> Result<Output> {
    let out = cmd.output().with_context(|| format!("não consegui executar {:?}", cmd.get_program()))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail: String = err.chars().rev().take(1500).collect::<Vec<_>>().into_iter().rev().collect();
        bail!("{:?} falhou ({}):\n{}", cmd.get_program(), out.status, tail);
    }
    Ok(out)
}

pub fn which(name: &str) -> Option<PathBuf> {
    if name.contains('/') {
        let p = PathBuf::from(name);
        return p.is_file().then_some(p);
    }
    let path = std::env::var_os("PATH")?;
    let extra = ["/opt/homebrew/bin", "/usr/local/bin"];
    std::env::split_paths(&path)
        .chain(extra.iter().map(PathBuf::from))
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

pub fn jobs(n: Option<usize>) -> usize {
    n.unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)).max(1)
}

/// 83.4 -> "01:23.4"; acima de 1 h -> "1:01:23.4"
pub fn tc(t: f64) -> String {
    let t = t.max(0.0);
    let h = (t / 3600.0) as u64;
    let m = ((t % 3600.0) / 60.0) as u64;
    let s = t % 60.0;
    if h > 0 { format!("{h}:{m:02}:{s:04.1}") } else { format!("{m:02}:{s:04.1}") }
}

/// 83.456 -> "00:01:23,456" (SRT)
pub fn tc_srt(t: f64) -> String {
    let ms = (t.max(0.0) * 1000.0).round() as u64;
    format!("{:02}:{:02}:{:02},{:03}", ms / 3_600_000, (ms / 60_000) % 60, (ms / 1000) % 60, ms % 1000)
}

pub fn write_json<T: Serialize + ?Sized>(path: &Path, v: &T) -> Result<()> {
    let f = std::fs::File::create(path).with_context(|| format!("criar {}", path.display()))?;
    serde_json::to_writer_pretty(std::io::BufWriter::new(f), v)?;
    Ok(())
}

pub fn ensure_dir(p: &Path) -> Result<()> {
    std::fs::create_dir_all(p).with_context(|| format!("criar pasta {}", p.display()))
}

pub fn mean(v: &[f32]) -> f32 {
    if v.is_empty() { 0.0 } else { v.iter().sum::<f32>() / v.len() as f32 }
}

pub fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}
