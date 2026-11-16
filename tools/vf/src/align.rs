//! `vf align`: amarra os beats do roteiro (roteiro/beats.json) às palavras da narração (words.json)
//! por LCS de tokens normalizados. Escreve start/end/w0/w1 em cada beat.
use crate::transcribe::Word;
use anyhow::{Context, Result};
use serde_json::Value;

pub fn norm(s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            let c = c.to_lowercase().next().unwrap_or(c);
            let base = match c {
                'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'í' | 'ì' | 'î' | 'ï' => 'i',
                'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
                'ú' | 'ù' | 'û' | 'ü' => 'u',
                'ç' => 'c',
                'ñ' => 'n',
                c => c,
            };
            base.is_alphanumeric().then_some(base)
        })
        .collect()
}

pub fn tokens(text: &str) -> Vec<String> {
    text.split_whitespace().map(norm).filter(|t| !t.is_empty()).collect()
}

/// Para cada token de `a`, o índice casado em `b` (LCS).
pub fn lcs_map(a: &[String], b: &[String]) -> Vec<Option<usize>> {
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![0u32; (n + 1) * (m + 1)];
    let idx = |i: usize, j: usize| i * (m + 1) + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[idx(i, j)] = if a[i] == b[j] { dp[idx(i + 1, j + 1)] + 1 } else { dp[idx(i + 1, j)].max(dp[idx(i, j + 1)]) };
        }
    }
    let mut out = vec![None; n];
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            out[i] = Some(j);
            i += 1;
            j += 1;
        } else if dp[idx(i + 1, j)] >= dp[idx(i, j + 1)] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

pub struct BeatSpan {
    pub id: String,
    pub w0: Option<usize>,
    pub w1: Option<usize>,
    pub matched: usize,
    pub total: usize,
}

/// Atualiza `beats` (objeto com "beats": [{texto|text, ...}]) e devolve um resumo.
pub fn align(beats: &mut Value, words: &[Word]) -> Result<Vec<BeatSpan>> {
    let list = beats.get_mut("beats").and_then(Value::as_array_mut).context("beats.json precisa de {\"beats\": [...]}")?;
    let wt: Vec<String> = words.iter().map(|w| norm(&w.word)).collect();
    let mut all = Vec::new();
    let mut owner = Vec::new();
    for (k, b) in list.iter().enumerate() {
        let text = b.get("texto").or_else(|| b.get("text")).and_then(Value::as_str).unwrap_or("");
        for t in tokens(text) {
            all.push(t);
            owner.push(k);
        }
    }
    let map = lcs_map(&all, &wt);
    let mut spans = Vec::new();
    for (k, b) in list.iter_mut().enumerate() {
        let hits: Vec<usize> = owner.iter().zip(&map).filter(|(o, _)| **o == k).filter_map(|(_, m)| *m).collect();
        let total = owner.iter().filter(|o| **o == k).count();
        let (w0, w1) = (hits.first().copied(), hits.last().copied());
        if let (Some(a), Some(z), Some(obj)) = (w0, w1, b.as_object_mut()) {
            obj.insert("w0".into(), a.into());
            obj.insert("w1".into(), z.into());
            // segundos com 3 casas
            obj.insert("start".into(), serde_json::json!((words[a].start * 1000.0).round() / 1000.0));
            obj.insert("end".into(), serde_json::json!((words[z].end * 1000.0).round() / 1000.0));
        }
        let id = b.get("id").map(|v| v.to_string()).unwrap_or_else(|| (k + 1).to_string());
        spans.push(BeatSpan { id, w0, w1, matched: hits.len(), total });
    }
    Ok(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aligns_with_number_mismatch() {
        let words: Vec<Word> = "Um marechal acabou com um império de 67 anos. É 1889 no Rio."
            .split(' ')
            .enumerate()
            .map(|(i, w)| Word { word: w.into(), start: i as f64, end: i as f64 + 0.9 })
            .collect();
        let mut beats = serde_json::json!({"beats": [
            {"id": 1, "texto": "Um marechal acabou com um império de sessenta e sete anos."},
            {"id": 2, "texto": "É mil oitocentos e oitenta e nove, no Rio."}
        ]});
        let s = align(&mut beats, &words).unwrap();
        assert_eq!((s[0].w0, s[0].w1), (Some(0), Some(8)));
        assert_eq!((s[1].w0, s[1].w1), (Some(9), Some(12)));
        assert_eq!(beats["beats"][1]["start"], 9.0);
        assert_eq!(norm("Império,"), "imperio");
    }
}
