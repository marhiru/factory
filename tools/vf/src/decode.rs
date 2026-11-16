//! Decodificação via ffmpeg -> rawvideo rgb24 por pipe, já reduzido no tamanho de análise.
use crate::probe::Probe;
use crate::util::ffmpeg;
use anyhow::{bail, Context, Result};
use std::io::{BufReader, ErrorKind, Read};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Clone, Debug)]
pub struct DecodeOpts {
    pub width: usize,
    pub height: usize,
    pub hwaccel: bool,
    pub fps_str: String,
    pub fps: f64,
}

impl DecodeOpts {
    pub fn new(p: &Probe, target_w: usize, hwaccel: bool) -> Self {
        let tw = target_w.min(p.width.max(2) as usize).max(16);
        let w = tw & !1;
        let h = (((w as f64) * p.height.max(2) as f64 / p.width.max(2) as f64).round() as usize & !1).max(2);
        DecodeOpts { width: w, height: h, hwaccel, fps_str: p.fps_str.clone(), fps: p.fps }
    }
    pub fn frame_bytes(&self) -> usize {
        self.width * self.height * 3
    }
}

/// Lê `n` frames (None = até o fim) a partir do frame `start`, chamando `f(indice_global, rgb)`.
pub fn stream<F>(input: &Path, start: usize, n: Option<usize>, o: &DecodeOpts, mut f: F) -> Result<usize>
where
    F: FnMut(usize, &[u8]) -> Result<()>,
{
    let mut cmd = Command::new(ffmpeg());
    cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-threads", "0"]);
    if o.hwaccel {
        cmd.args(["-hwaccel", "videotoolbox"]);
    }
    if start > 0 {
        cmd.args(["-ss", &format!("{:.6}", start as f64 / o.fps)]);
    }
    cmd.arg("-i").arg(input);
    cmd.args(["-an", "-sn", "-dn", "-map", "0:v:0"]);
    if let Some(n) = n {
        cmd.args(["-frames:v", &n.to_string()]);
    }
    let vf = format!("fps={},scale={}:{}:flags=area,format=rgb24", o.fps_str, o.width, o.height);
    cmd.args(["-vf", &vf, "-f", "rawvideo", "-"]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().context("não consegui iniciar o ffmpeg (está no PATH?)")?;
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });

    let mut rd = BufReader::with_capacity(o.frame_bytes() * 4, stdout);
    let mut buf = vec![0u8; o.frame_bytes()];
    let mut count = 0usize;
    loop {
        match rd.read_exact(&mut buf) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
        f(start + count, &buf)?;
        count += 1;
    }
    let status = child.wait()?;
    let err = err_thread.join().unwrap_or_default();
    if !status.success() && count == 0 {
        bail!("ffmpeg falhou ao decodificar {}:\n{}", input.display(), err);
    }
    Ok(count)
}
