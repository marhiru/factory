//! Estatísticas por frame + detecção de cortes. Decodifica o vídeo em N segmentos paralelos
//! (um ffmpeg por segmento, com 2 frames de sobreposição para as diferenças entre frames).
use crate::decode::{self, DecodeOpts};
use crate::probe::{self, Probe};
use crate::util::{mean, write_json};
use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

#[derive(Serialize, Deserialize, Clone, Copy, Default, Debug)]
pub struct FrameStat {
    pub i: usize,
    pub t: f64,
    /// luma média 0-1 (Rec.709)
    pub luma: f32,
    /// croma média 0-1 (max-min do RGB)
    pub sat: f32,
    /// diferença média absoluta de luma para o frame anterior, 0-1
    pub motion: f32,
    /// diferença para o frame i-2 (detecta "pops" de um frame)
    pub skip2: f32,
    /// distância de histograma RGB 8x8x8 para o frame anterior, 0-1
    pub hdiff: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Shot {
    pub id: usize,
    /// primeiro frame (inclusivo)
    pub a: usize,
    /// fim (exclusivo) = `a` do próximo plano
    pub b: usize,
    pub ta: f64,
    pub tb: f64,
    pub dur: f64,
    pub luma: f32,
    pub sat: f32,
    pub motion: f32,
}

/// Miniatura capturada durante a mesma passada de decodificação (sem custo extra de decode).
pub struct Thumb {
    pub i: usize,
    pub t: f64,
    pub rgb: Vec<u8>,
}

pub struct FrameAnalysis {
    pub probe: Probe,
    pub opts: DecodeOpts,
    pub frames: Vec<FrameStat>,
    pub cuts: Vec<usize>,
    pub shots: Vec<Shot>,
    /// transições graduais detectadas [a, b) (dissolves/crossfades)
    pub xfades: Vec<(usize, usize)>,
    pub thumbs: Vec<Thumb>,
    pub decode_secs: f64,
}

#[derive(Clone, Debug)]
pub struct FramesOpts {
    pub width: usize,
    pub jobs: usize,
    pub hwaccel: bool,
    pub threshold: f32,
    pub min_shot: usize,
    /// capturar miniatura a cada N frames (0 = não capturar)
    pub thumb_every: usize,
}

const BINS: usize = 512;

fn stats(rgb: &[u8], gray: &mut [u8], hist: &mut [u32; BINS]) -> (f32, f32) {
    hist.fill(0);
    let (mut ls, mut cs) = (0u64, 0u64);
    for (px, g) in rgb.chunks_exact(3).zip(gray.iter_mut()) {
        let (r, gg, b) = (px[0] as u32, px[1] as u32, px[2] as u32);
        let y = (54 * r + 183 * gg + 19 * b) >> 8;
        *g = y as u8;
        ls += y as u64;
        let mx = r.max(gg).max(b);
        let mn = r.min(gg).min(b);
        cs += (mx - mn) as u64;
        hist[((r >> 5) << 6 | (gg >> 5) << 3 | (b >> 5)) as usize] += 1;
    }
    let n = (gray.len() as f64) * 255.0;
    ((ls as f64 / n) as f32, (cs as f64 / n) as f32)
}

fn mad(a: &[u8], b: &[u8]) -> f32 {
    let s: u64 = a.iter().zip(b).map(|(&x, &y)| x.abs_diff(y) as u64).sum();
    (s as f64 / (a.len() as f64 * 255.0)) as f32
}

fn hist_dist(a: &[u32; BINS], b: &[u32; BINS], npx: usize) -> f32 {
    let s: u64 = a.iter().zip(b.iter()).map(|(&x, &y)| x.abs_diff(y) as u64).sum();
    (s as f64 / (2.0 * npx as f64)) as f32
}

/// histograma RGB 4x4x4 por frame (para testar transições graduais)
pub type Coarse = [u32; 64];

fn coarsen(h: &[u32; BINS]) -> Coarse {
    let mut c = [0u32; 64];
    for (k, &v) in h.iter().enumerate() {
        let (r, g, b) = (k >> 6, (k >> 3) & 7, k & 7);
        c[(r >> 1) << 4 | (g >> 1) << 2 | (b >> 1)] += v;
    }
    c
}

fn coarse_dist(a: &Coarse, b: &Coarse) -> f32 {
    let s: u64 = a.iter().zip(b).map(|(&x, &y)| x.abs_diff(y) as u64).sum();
    let n: u64 = a.iter().map(|&x| x as u64).sum::<u64>().max(1);
    (s as f64 / (2.0 * n as f64)) as f32
}

struct SegOut {
    coarse: Vec<Coarse>,
    frames: Vec<FrameStat>,
    thumbs: Vec<Thumb>,
}

fn segment(input: &Path, o: &DecodeOpts, s: usize, e: Option<usize>, thumb_every: usize) -> Result<SegOut> {
    let lead = s.min(2);
    let start = s - lead;
    let n = e.map(|e| e - start);
    let npx = o.width * o.height;
    let mut grays: [Vec<u8>; 3] = [vec![0; npx], vec![0; npx], vec![0; npx]];
    let mut hists: [Box<[u32; BINS]>; 2] = [Box::new([0; BINS]), Box::new([0; BINS])];
    let mut frames = Vec::with_capacity(n.unwrap_or(1024));
    let mut thumbs = Vec::new();
    let mut coarse = Vec::with_capacity(n.unwrap_or(1024));
    let mut k = 0usize; // frames lidos neste segmento
    decode::stream(input, start, n, o, |gi, rgb| {
        let cur = k % 3;
        let (h0, h1) = hists.split_at_mut(1);
        let (hc, hp) = if k % 2 == 0 { (&mut h0[0], &h1[0]) } else { (&mut h1[0], &h0[0]) };
        let (luma, sat) = {
            let g = &mut grays[cur];
            stats(rgb, g, hc)
        };
        if k >= lead {
            let (motion, hdiff) = if k >= 1 {
                (mad(&grays[cur], &grays[(k + 2) % 3]), hist_dist(hc, hp, npx))
            } else {
                (0.0, 0.0)
            };
            let skip2 = if k >= 2 { mad(&grays[cur], &grays[(k + 1) % 3]) } else { motion };
            coarse.push(coarsen(hc));
            frames.push(FrameStat { i: gi, t: gi as f64 / o.fps, luma, sat, motion, skip2, hdiff });
            if thumb_every > 0 && gi % thumb_every == 0 {
                thumbs.push(Thumb { i: gi, t: gi as f64 / o.fps, rgb: rgb.to_vec() });
            }
        }
        k += 1;
        Ok(())
    })?;
    Ok(SegOut { coarse, frames, thumbs })
}

/// Detecção de cortes: salto de histograma forte, ou salto relativo ao contexto local
/// (histograma ou luma), com duração mínima de plano.
pub fn detect_cuts(f: &[FrameStat], threshold: f32, min_shot: usize) -> Vec<usize> {
    let n = f.len();
    let mut cuts = Vec::new();
    let win = 12usize;
    let mut last = 0usize;
    for i in 1..n {
        let lo = i.saturating_sub(win);
        let hi = (i + win + 1).min(n);
        let (mut sh, mut sm, mut c) = (0f32, 0f32, 0f32);
        for j in lo..hi {
            if j + 1 >= i && j <= i + 1 {
                continue;
            }
            sh += f[j].hdiff;
            sm += f[j].motion;
            c += 1.0;
        }
        let (lh, lm) = if c > 0.0 { (sh / c, sm / c) } else { (0.0, 0.0) };
        let h = f[i].hdiff;
        let m = f[i].motion;
        let is_cut = h >= threshold
            || (h >= threshold * 0.5 && h >= 3.0 * lh + 0.04)
            || (m >= 0.10 && m >= 4.0 * lm + 0.03 && h >= threshold * 0.25);
        if is_cut && (i - last >= min_shot || cuts.is_empty() && i >= min_shot) {
            cuts.push(i);
            last = i;
        }
    }
    cuts
}

/// Transições graduais: sequência de 3-60 frames com histograma mudando o tempo todo, cercada de
/// frames calmos, em que o antes e o depois são imagens diferentes.
pub fn detect_xfades(f: &[FrameStat], coarse: &[Coarse]) -> Vec<(usize, usize)> {
    let n = f.len();
    let mut out = Vec::new();
    let mut i = 1;
    while i < n {
        if f[i].hdiff < 0.12 {
            i += 1;
            continue;
        }
        let a = i;
        while i < n && f[i].hdiff >= 0.12 {
            i += 1;
        }
        let b = i; // primeiro frame calmo
        let len = b - a;
        if !(3..=60).contains(&len) {
            continue;
        }
        let run: f32 = f[a..b].iter().map(|x| x.hdiff).sum::<f32>() / len as f32;
        let side = |r: std::ops::Range<usize>| -> f32 {
            let v: Vec<f32> = f[r].iter().map(|x| x.hdiff).collect();
            mean(&v)
        };
        let pre = side(a.saturating_sub(6)..a);
        let post = side(b..(b + 6).min(n));
        if pre < run / 3.0 && post < run / 3.0 && b < n && coarse_dist(&coarse[a - 1], &coarse[b]) >= 0.25 {
            out.push((a, b));
        }
    }
    out
}

/// Junta cortes secos e transições: cortes dentro de uma transição viram um só, no meio dela.
pub fn merge_cuts(hard: &[usize], xf: &[(usize, usize)], min_shot: usize) -> Vec<usize> {
    let mut v: Vec<usize> = hard.iter().copied().filter(|&c| !xf.iter().any(|&(a, b)| c + 1 >= a && c <= b)).collect();
    v.extend(xf.iter().map(|&(a, b)| (a + b) / 2));
    v.sort_unstable();
    let mut out: Vec<usize> = Vec::new();
    for c in v {
        if out.last().map_or(c > 0, |&l| c - l >= min_shot) {
            out.push(c);
        }
    }
    out
}

pub fn build_shots(f: &[FrameStat], cuts: &[usize], fps: f64) -> Vec<Shot> {
    let mut bounds = vec![0usize];
    bounds.extend_from_slice(cuts);
    bounds.push(f.len());
    bounds
        .windows(2)
        .enumerate()
        .filter(|(_, w)| w[1] > w[0])
        .map(|(k, w)| {
            let s = &f[w[0]..w[1]];
            let l: Vec<f32> = s.iter().map(|x| x.luma).collect();
            let c: Vec<f32> = s.iter().map(|x| x.sat).collect();
            let m: Vec<f32> = s.iter().skip(1).map(|x| x.motion).collect();
            Shot {
                id: k + 1,
                a: w[0],
                b: w[1],
                ta: w[0] as f64 / fps,
                tb: w[1] as f64 / fps,
                dur: (w[1] - w[0]) as f64 / fps,
                luma: mean(&l),
                sat: mean(&c),
                motion: mean(&m),
            }
        })
        .collect()
}

pub fn analyze(input: &Path, o: &FramesOpts) -> Result<FrameAnalysis> {
    let p = probe::probe(input)?;
    let dopts = DecodeOpts::new(&p, o.width, o.hwaccel);
    let total = p.nb_frames.max(1) as usize;
    // segmentos de pelo menos ~6 s para amortizar o custo de abrir/buscar
    let min_seg = (p.fps * 6.0).max(60.0) as usize;
    let nseg = o.jobs.min(total / min_seg).max(1);
    let t0 = Instant::now();
    let pool = rayon::ThreadPoolBuilder::new().num_threads(nseg).build()?;
    let outs: Vec<SegOut> = pool.install(|| {
        (0..nseg)
            .into_par_iter()
            .map(|k| {
                let s = k * total / nseg;
                let e = if k + 1 == nseg { None } else { Some((k + 1) * total / nseg) };
                segment(input, &dopts, s, e, o.thumb_every)
            })
            .collect::<Result<Vec<_>>>()
    })?;
    let decode_secs = t0.elapsed().as_secs_f64();
    let mut frames = Vec::with_capacity(total);
    let mut thumbs = Vec::new();
    let mut coarse = Vec::with_capacity(total);
    for so in outs {
        frames.extend(so.frames);
        coarse.extend(so.coarse);
        thumbs.extend(so.thumbs);
    }
    // reindexa de forma contínua (protege contra estimativa de nb_frames imprecisa)
    for (k, fr) in frames.iter_mut().enumerate() {
        fr.i = k;
        fr.t = k as f64 / p.fps;
    }
    let hard = detect_cuts(&frames, o.threshold, o.min_shot);
    let xfades = detect_xfades(&frames, &coarse);
    let cuts = merge_cuts(&hard, &xfades, o.min_shot);
    let shots = build_shots(&frames, &cuts, p.fps);
    Ok(FrameAnalysis { probe: p, opts: dopts, frames, cuts, shots, xfades, thumbs, decode_secs })
}

#[derive(Serialize)]
pub struct ShotsFile<'a> {
    pub source: &'a str,
    pub fps: f64,
    pub frames: usize,
    pub duration: f64,
    pub shots: &'a [Shot],
    /// mesmo formato do plan.json do QA: transições a ignorar
    pub xfade: Vec<[usize; 2]>,
}

pub fn write_outputs(fa: &FrameAnalysis, out: &Path) -> Result<()> {
    let f = std::fs::File::create(out.join("frames.jsonl"))?;
    let mut w = std::io::BufWriter::new(f);
    for fr in &fa.frames {
        serde_json::to_writer(&mut w, fr)?;
        w.write_all(b"\n")?;
    }
    write_json(
        &out.join("shots.json"),
        &ShotsFile {
            source: &fa.probe.path,
            fps: fa.probe.fps,
            frames: fa.frames.len(),
            duration: fa.frames.len() as f64 / fa.probe.fps,
            shots: &fa.shots,
            xfade: fa.xfades.iter().map(|&(a, b)| [a, b]).collect(),
        },
    )?;
    write_json(&out.join("probe.json"), &fa.probe)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fs(h: &[f32]) -> Vec<FrameStat> {
        h.iter().enumerate().map(|(i, &x)| FrameStat { i, hdiff: x, motion: x * 0.3, ..Default::default() }).collect()
    }
    #[test]
    fn cuts_basic() {
        let mut h = vec![0.01f32; 100];
        h[30] = 0.8;
        h[31] = 0.5; // flash de 1 frame não vira plano novo (min_shot)
        h[70] = 0.3;
        let c = detect_cuts(&fs(&h), 0.4, 6);
        assert_eq!(c, vec![30, 70]);
        let s = build_shots(&fs(&h), &c, 30.0);
        assert_eq!(s.len(), 3);
        assert_eq!((s[1].a, s[1].b), (30, 70));
    }
    #[test]
    fn hist_and_mad() {
        let a = vec![0u8; 30];
        let b = vec![255u8; 30];
        assert!((mad(&a, &b) - 1.0).abs() < 1e-6);
        let mut g = vec![0u8; 10];
        let mut h1 = [0u32; BINS];
        let mut h2 = [0u32; BINS];
        stats(&a, &mut g, &mut h1);
        stats(&b, &mut g, &mut h2);
        assert!((hist_dist(&h1, &h2, 10) - 1.0).abs() < 1e-6);
    }
}
