//! Transcrição com timestamps por palavra via whisper.cpp (`whisper-cli`, Metal no Mac).
//! Saída `words.json` no formato da fábrica: [{"word","start","end"}].
use crate::util::{ffmpeg, run, tc_srt, which, write_json};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Word {
    pub word: String,
    pub start: f64,
    pub end: f64,
}

pub struct TranscribeOpts {
    pub model: String,
    pub lang: String,
    pub threads: usize,
    pub prompt: Option<String>,
    pub dtw: bool,
}

fn models_dir() -> PathBuf {
    if let Ok(d) = std::env::var("VF_MODELS") {
        return PathBuf::from(d);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".cache/vf/models")
}

/// Aceita caminho de arquivo .bin ou nome (ex.: large-v3-turbo-q5_0); baixa do Hugging Face se faltar.
pub fn resolve_model(model: &str) -> Result<PathBuf> {
    let p = PathBuf::from(model);
    if p.is_file() {
        return Ok(p);
    }
    let dir = models_dir();
    let file = dir.join(format!("ggml-{model}.bin"));
    if file.is_file() {
        return Ok(file);
    }
    std::fs::create_dir_all(&dir)?;
    let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{model}.bin");
    eprintln!("baixando modelo {model} para {} ...", file.display());
    let tmp = file.with_extension("part");
    let st = Command::new("curl").args(["-L", "--fail", "-sS", "-o"]).arg(&tmp).arg(&url).status()?;
    if !st.success() {
        bail!("download do modelo falhou: {url}");
    }
    std::fs::rename(&tmp, &file)?;
    Ok(file)
}

fn whisper_bin() -> Result<PathBuf> {
    let name = std::env::var("VF_WHISPER").unwrap_or_else(|_| "whisper-cli".into());
    which(&name).with_context(|| "whisper-cli não encontrado: instale com `brew install whisper-cpp` ou defina VF_WHISPER")
}

fn dtw_preset(model: &str) -> Option<&'static str> {
    let m = model.rsplit('/').next().unwrap_or(model).trim_start_matches("ggml-").trim_end_matches(".bin");
    let base = m.split("-q").next().unwrap_or(m);
    Some(match base {
        "tiny" => "tiny",
        "tiny.en" => "tiny.en",
        "base" => "base",
        "base.en" => "base.en",
        "small" => "small",
        "small.en" => "small.en",
        "medium" => "medium",
        "medium.en" => "medium.en",
        "large-v1" => "large.v1",
        "large-v2" => "large.v2",
        "large-v3" => "large.v3",
        "large-v3-turbo" => "large.v3.turbo",
        _ => return None,
    })
}

/// Junta segmentos de `-ml 1 -sow` em palavras: pontuação solta vai para a palavra anterior.
pub fn segments_to_words(j: &Value) -> Vec<Word> {
    let mut words: Vec<Word> = Vec::new();
    for s in j["transcription"].as_array().into_iter().flatten() {
        let text = s["text"].as_str().unwrap_or("");
        let t = text.trim();
        if t.is_empty() || (t.starts_with('[') && t.ends_with(']')) {
            continue;
        }
        let a = s["offsets"]["from"].as_f64().unwrap_or(0.0) / 1000.0;
        let b = s["offsets"]["to"].as_f64().unwrap_or(0.0) / 1000.0;
        let is_punct = t.chars().all(|c| !c.is_alphanumeric());
        let continues = !text.starts_with(' ') && !words.is_empty();
        if (is_punct || continues) && !words.is_empty() {
            let w = words.last_mut().unwrap();
            w.word.push_str(t);
            w.end = w.end.max(b);
            continue;
        }
        words.push(Word { word: t.to_string(), start: a, end: b.max(a) });
    }
    words
}

/// Converte o JSON do Speech-to-Text do ElevenLabs.
pub fn from_elevenlabs(j: &Value) -> Vec<Word> {
    let mut out: Vec<Word> = Vec::new();
    for w in j["words"].as_array().into_iter().flatten() {
        let ty = w["type"].as_str().unwrap_or("word");
        if ty != "word" {
            continue;
        }
        let t = w["text"].as_str().unwrap_or("").trim().to_string();
        if t.is_empty() {
            continue;
        }
        let (a, b) = (w["start"].as_f64().unwrap_or(0.0), w["end"].as_f64().unwrap_or(0.0));
        if t.chars().all(|c| !c.is_alphanumeric()) {
            if let Some(l) = out.last_mut() {
                l.word.push_str(&t);
                continue;
            }
        }
        out.push(Word { word: t, start: a, end: b });
    }
    out
}

pub fn transcribe(input: &Path, out_dir: &Path, o: &TranscribeOpts) -> Result<Vec<Word>> {
    let bin = whisper_bin()?;
    let model = resolve_model(&o.model)?;
    let wav = out_dir.join(".vf_16k.wav");
    run(Command::new(ffmpeg())
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-i"])
        .arg(input)
        .args(["-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le"])
        .arg(&wav))?;
    let base = out_dir.join(".vf_whisper");
    let mut cmd = Command::new(bin);
    cmd.arg("-m").arg(&model).arg("-f").arg(&wav);
    cmd.args(["-l", &o.lang, "-t", &o.threads.to_string(), "-ml", "1", "-sow", "-oj", "-np"]);
    if let Some(p) = &o.prompt {
        cmd.args(["--prompt", p]);
    }
    if o.dtw {
        if let Some(d) = dtw_preset(&model.display().to_string()) {
            cmd.args(["-dtw", d, "-nfa"]);
        }
    }
    cmd.arg("-of").arg(&base);
    run(&mut cmd)?;
    let jpath = base.with_extension("json");
    let raw = std::fs::read(&jpath)?;
    // whisper.cpp pode emitir UTF-8 inválido quando um caractere é cortado entre tokens
    let j: Value = serde_json::from_str(&String::from_utf8_lossy(&raw))?;
    let _ = std::fs::remove_file(&wav);
    let _ = std::fs::remove_file(&jpath);
    Ok(segments_to_words(&j))
}

/// Legendas: quebra em pontuação final, pausa > 0,6 s ou `max_words` palavras.
pub fn to_srt(words: &[Word], max_words: usize) -> String {
    let mut out = String::new();
    let mut line: Vec<&Word> = Vec::new();
    let mut n = 0;
    let mut flush = |line: &mut Vec<&Word>, out: &mut String| {
        if line.is_empty() {
            return;
        }
        n += 1;
        let text: Vec<&str> = line.iter().map(|w| w.word.as_str()).collect();
        out.push_str(&format!("{n}\n{} --> {}\n{}\n\n", tc_srt(line[0].start), tc_srt(line.last().unwrap().end), text.join(" ")));
        line.clear();
    };
    for (k, w) in words.iter().enumerate() {
        line.push(w);
        let end_sent = w.word.ends_with(['.', '!', '?', '…']);
        let gap = words.get(k + 1).map(|nx| nx.start - w.end > 0.6).unwrap_or(true);
        if end_sent || gap || line.len() >= max_words {
            flush(&mut line, &mut out);
        }
    }
    flush(&mut line, &mut out);
    out
}

pub fn write_words(words: &[Word], out_dir: &Path) -> Result<()> {
    write_json(&out_dir.join("words.json"), words)?;
    std::fs::write(out_dir.join("transcript.srt"), to_srt(words, 7))?;
    let text: Vec<&str> = words.iter().map(|w| w.word.as_str()).collect();
    std::fs::write(out_dir.join("transcript.txt"), text.join(" ") + "\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merge_punct_and_subwords() {
        let j: Value = serde_json::json!({"transcription": [
            {"text": " Um", "offsets": {"from": 0, "to": 200}},
            {"text": " mare", "offsets": {"from": 200, "to": 400}},
            {"text": "chal", "offsets": {"from": 400, "to": 600}},
            {"text": ",", "offsets": {"from": 600, "to": 610}},
            {"text": " [_BEG_]", "offsets": {"from": 610, "to": 610}},
            {"text": " doente.", "offsets": {"from": 700, "to": 1100}}
        ]});
        let w = segments_to_words(&j);
        assert_eq!(w.iter().map(|w| w.word.as_str()).collect::<Vec<_>>(), vec!["Um", "marechal,", "doente."]);
        assert_eq!(w[1].end, 0.61);
        let srt = to_srt(&w, 7);
        assert!(srt.starts_with("1\n00:00:00,000 --> 00:00:01,100\nUm marechal, doente."));
    }
    #[test]
    fn elevenlabs() {
        let j = serde_json::json!({"words": [
            {"text": "Olá", "start": 0.0, "end": 0.3, "type": "word"},
            {"text": " ", "start": 0.3, "end": 0.35, "type": "spacing"},
            {"text": "mundo", "start": 0.35, "end": 0.7, "type": "word"},
            {"text": "(risos)", "start": 0.7, "end": 1.0, "type": "audio_event"}
        ]});
        assert_eq!(from_elevenlabs(&j).len(), 2);
    }
}
