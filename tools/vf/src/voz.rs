//! `vf voz <texto.txt> [-o public/voice/voice.wav] [--ref voz.wav ...]`: narração local com Chatterbox.
//! Roda `skills/assets/scripts/voz-chatterbox.py` pelo `uv` (ambiente Python isolado, criado na 1ª vez).
//! Ajustes que faltarem na linha de comando vêm de `factory.yaml > voz` (provedor: chatterbox), para a voz
//! do canal sair igual em todo vídeo: ref, modelo, exaggeration, cfg_weight, temperature, seed, tempo.
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

/// (chave no factory.yaml > voz.chatterbox ou voz, flag do script)
const DO_PROJETO: &[(&str, &str)] = &[
    ("ref", "--ref"),
    ("modelo", "--modelo"),
    ("exaggeration", "--exaggeration"),
    ("cfg_weight", "--cfg"),
    ("temperature", "--temperature"),
    ("seed", "--seed"),
    ("tempo", "--tempo"),
];

/// argumentos vindos do factory.yaml que o usuário não passou
pub fn do_factory(yaml: &serde_yaml::Value, args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let voz = match yaml.get("voz") {
        Some(v) => v,
        None => return out,
    };
    if voz.get("provedor").and_then(|p| p.as_str()).is_some_and(|p| p != "chatterbox") {
        return out;
    }
    let tem = |flag: &str| args.iter().any(|a| a == flag || a.starts_with(&format!("{flag}=")));
    let src = voz.get("chatterbox").unwrap_or(voz);
    for (k, flag) in DO_PROJETO {
        if tem(flag) {
            continue;
        }
        let v = src.get(*k).or_else(|| voz.get("settings").and_then(|s| s.get(*k)));
        let txt = match v {
            Some(serde_yaml::Value::String(s)) if !s.is_empty() => s.clone(),
            Some(serde_yaml::Value::Number(n)) => n.to_string(),
            _ => continue,
        };
        out.push(flag.to_string());
        out.push(txt);
    }
    if !tem("--lang") {
        if let Some(l) = yaml.get("idioma").and_then(|x| x.as_str()) {
            out.push("--lang".into());
            out.push(l.into());
        }
    }
    out
}

pub fn run(args: &[String]) -> Result<()> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") && args.len() == 1 {
        eprintln!("uso: vf voz <texto.txt> [-o public/voice/voice.wav] [--ref voz/ref.wav] [--exaggeration 0.5 --cfg 0.5 --seed 42 --tempo 1.0 --refazer 3,7]\n(ajustes padrão: factory.yaml > voz; detalhes: skills/assets/templates/voz-chatterbox.md)");
        if args.is_empty() {
            bail!("diga o arquivo de texto");
        }
    }
    let uv = crate::util::which("uv").context("o vf voz precisa do uv (gerenciador de Python): brew install uv")?;
    let script: PathBuf = crate::init::plugin_root()?.join("skills/assets/scripts/voz-chatterbox.py");
    if !script.is_file() {
        bail!("script não encontrado: {}", script.display());
    }
    let mut full: Vec<String> = args.to_vec();
    if let Some(y) = std::fs::read_to_string("factory.yaml").ok().and_then(|s| serde_yaml::from_str::<serde_yaml::Value>(&s).ok()) {
        let extra = do_factory(&y, args);
        if !extra.is_empty() {
            eprintln!("factory.yaml > voz: {}", extra.join(" "));
        }
        full.extend(extra);
    }
    let st = std::process::Command::new(uv).arg("run").arg("--script").arg(&script).args(&full).status().context("rodar o uv")?;
    if !st.success() {
        bail!("voz-chatterbox.py saiu com erro ({st})");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn le_factory() {
        let y: serde_yaml::Value = serde_yaml::from_str(
            "idioma: pt\nvoz:\n  provedor: chatterbox\n  chatterbox: { ref: kit/voz.wav, exaggeration: 0.6, cfg_weight: 0.4, seed: 7, modelo: pt-br }\n",
        )
        .unwrap();
        let a = do_factory(&y, &["t.txt".into(), "--seed".into(), "9".into()]);
        assert!(a.windows(2).any(|w| w[0] == "--ref" && w[1] == "kit/voz.wav"));
        assert!(a.windows(2).any(|w| w[0] == "--cfg" && w[1] == "0.4"));
        assert!(a.windows(2).any(|w| w[0] == "--lang" && w[1] == "pt"));
        assert!(!a.contains(&"--seed".to_string()), "o que veio na linha de comando vence");
        let el: serde_yaml::Value = serde_yaml::from_str("voz: { provedor: elevenlabs, settings: { stability: 0.5 } }").unwrap();
        assert!(do_factory(&el, &[]).is_empty());
    }
}
