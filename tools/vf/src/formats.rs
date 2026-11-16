use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

const DEFAULT: &str = include_str!("../../../formats.yaml");

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Range {
    pub min: f64,
    pub max: f64,
    pub target: f64,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct VideoSpec {
    pub codec: String,
    pub crf: u32,
    pub preset: String,
    pub pix_fmt: String,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct AudioSpec {
    pub codec: String,
    pub bitrate: String,
    pub sample_rate: u32,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Safe {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Format {
    pub platform: String,
    pub label: String,
    pub aspect: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub alt_sizes: Vec<[u32; 2]>,
    pub fps: Vec<f64>,
    pub duration: Range,
    pub wpm: f64,
    pub lufs: f64,
    pub true_peak: f64,
    pub lufs_tolerance: f64,
    pub video: VideoSpec,
    pub audio: AudioSpec,
    pub safe_zone: Safe,
}

pub fn load(path: Option<&Path>) -> Result<BTreeMap<String, Format>> {
    let env = std::env::var("VF_FORMATS").ok();
    let text = match path.map(|p| p.to_path_buf()).or(env.map(Into::into)) {
        Some(p) => std::fs::read_to_string(&p).with_context(|| format!("ler {}", p.display()))?,
        None => DEFAULT.to_string(),
    };
    Ok(serde_yaml::from_str(&text)?)
}

pub fn get(path: Option<&Path>, name: &str) -> Result<Format> {
    let all = load(path)?;
    let keys: Vec<_> = all.keys().cloned().collect();
    all.get(name).cloned().with_context(|| format!("formato '{name}' desconhecido; use um de {keys:?}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_parses() {
        let f = super::load(None).unwrap();
        assert!(f.contains_key("yt-long") && f.contains_key("yt-short") && f.contains_key("tiktok"));
        assert_eq!(f["tiktok"].height, 1920);
    }
}
