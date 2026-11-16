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
/// Regras do QA estrito por formato (bloco `qa:` do formats.yaml).
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct QaSpec {
    /// hold mínimo (frames) para acusar congelamento
    pub freeze_frames: usize,
    /// faixa de duração aceita na entrega [min, max] (mais apertada que `duration`)
    pub duration_strict: Option<[f64; 2]>,
    /// "vida" mínima de um plano (média do diff de ~0,5 s); abaixo disso o plano está parado
    pub min_life: f32,
    /// plano mais longo aceito (s)
    pub max_shot_s: f64,
    /// bitrate total máximo (Mb/s)
    pub max_bitrate_mbps: f64,
    /// a voz/som precisa começar até aqui (s)
    pub audio_start_s: f64,
    /// precisa haver corte ou movimento forte até aqui (s)
    pub hook_s: f64,
    /// silêncio total (ar morto) máximo no mix final (s)
    pub max_silence_s: f64,
    /// buraco máximo na trilha de voz (s), checado com --voice
    pub max_voice_gap_s: f64,
    /// a mesma imagem não pode voltar antes disso (s); 0 desliga
    pub repeat_window_s: f64,
}

impl Default for QaSpec {
    fn default() -> Self {
        QaSpec {
            freeze_frames: 12,
            duration_strict: None,
            min_life: 0.004,
            max_shot_s: 10.0,
            max_bitrate_mbps: 40.0,
            audio_start_s: 0.3,
            hook_s: 3.0,
            max_silence_s: 1.0,
            max_voice_gap_s: 1.5,
            repeat_window_s: 0.0,
        }
    }
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
    #[serde(default)]
    pub qa: QaSpec,
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
