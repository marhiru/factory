//! `vf init type="short,long" platform="yt,tiktok"`: transforma a pasta atual num projeto da fábrica
//! (esqueleto, templates do motor, áudio procedural, bibliotecas de FX de referência e o plugin ligado
//! ao Claude Code). Nunca sobrescreve arquivos do usuário.
use crate::kv::{self, Kv};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Bibliotecas de FX de referência (clonadas uma vez em ~/.cache/vf/ref e ligadas em ref/fx/).
const FX_REFS: &[(&str, &str, &str)] = &[
    ("pixelfx", "https://github.com/patch-gg/pixelfx", "MIT"),
    ("pixel-splats", "https://github.com/patch-gg/pixel-splats", "Apache-2.0"),
];

pub const KNOWN: &[&str] = &["type", "types", "platform", "platforms", "motor", "idioma", "lang", "duracao", "canal", "dir", "install", "offline"];

/// Raiz do plugin: $VF_ROOT, $CLAUDE_PLUGIN_ROOT ou <bin/vf>/.. (o link em ~/.local/bin é resolvido).
pub fn plugin_root() -> Result<PathBuf> {
    for var in ["VF_ROOT", "CLAUDE_PLUGIN_ROOT"] {
        if let Ok(v) = std::env::var(var) {
            let p = PathBuf::from(v);
            if p.join("scripts/novo.sh").is_file() {
                return Ok(p);
            }
        }
    }
    let exe = std::env::current_exe()?.canonicalize()?;
    for anc in exe.ancestors().skip(1) {
        if anc.join("scripts/novo.sh").is_file() {
            return Ok(anc.to_path_buf());
        }
    }
    bail!("não achei a raiz do plugin video-factory (defina VF_ROOT=<pasta do plugin>)")
}

pub fn run(args: &[String]) -> Result<()> {
    let kv = Kv::parse(args, &["install", "offline"])?;
    kv.check_known(KNOWN)?;
    let root = plugin_root()?;

    let mut types = kv.list("type");
    types.extend(kv.list("types"));
    if types.is_empty() {
        bail!("diga o tipo: vf init type=short | type=long | type=\"short,long\"");
    }
    for t in &types {
        if !matches!(t.as_str(), "short" | "long") {
            bail!("type inválido: {t} (short | long)");
        }
    }
    let mut plats = kv::platforms(&kv)?;
    if plats.is_empty() {
        plats.push("yt");
        eprintln!("platform não informado: usando yt (vf init ... platform=\"yt,tiktok\")");
    }
    let long = types.iter().any(|t| t == "long");
    let short = types.iter().any(|t| t == "short");
    let mut formatos: Vec<&str> = Vec::new();
    if long {
        if plats.contains(&"yt") {
            formatos.push("yt-long");
        } else {
            bail!("vídeo longo só existe no YouTube na fábrica: use platform=yt (o TikTok recebe cortes com type=\"short,long\")");
        }
    }
    if short {
        if plats.contains(&"yt") {
            formatos.push("yt-short");
        }
        if plats.contains(&"tiktok") {
            formatos.push("tiktok");
        }
    } else if long && plats.contains(&"tiktok") {
        eprintln!("aviso: type=long com tiktok — o TikTok só recebe cortes verticais; adicionei o formato tiktok como derivado do longo");
        formatos.push("tiktok");
    }
    let motor = kv.get("motor").unwrap_or(if long { "estatico" } else { "cutout-2d" });

    let dir = match kv.get("dir") {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir()?,
    };
    std::fs::create_dir_all(&dir)?;
    let dir = dir.canonicalize()?;
    if dir.join("factory.yaml").exists() {
        bail!("{} já é um projeto da fábrica (factory.yaml existe)", dir.display());
    }

    let mut cmd = Command::new(root.join("scripts/novo.sh"));
    cmd.arg(&dir)
        .args(["--formato", &formatos.join(",")])
        .args(["--motor", motor])
        .args(["--plataformas", &plats.join(",")])
        .arg("--existente");
    if let Some(i) = kv.get("idioma").or(kv.get("lang")) {
        cmd.args(["--idioma", i]);
    }
    if let Some(d) = kv.get("duracao") {
        cmd.args(["--duracao", d]);
    }
    if let Some(c) = kv.get("canal") {
        cmd.args(["--canal", c]);
    }
    if kv.flag("install") {
        cmd.arg("--install");
    }
    let st = cmd.status().context("rodar scripts/novo.sh")?;
    if !st.success() {
        bail!("novo.sh falhou ({st})");
    }

    // sons prontos: o kit de áudio procedural (MIT) vai para o projeto
    copy_missing(&root.join("skills/assets/templates/audio-procedural"), &dir.join("audio/procedural"))?;
    claude_wiring(&root, &dir)?;
    if !kv.flag("offline") {
        fx_refs(&dir);
    }

    println!("\npronto: {}", dir.display());
    println!("  tipo: {} · plataformas: {} · formatos: {} · motor: {motor}", types.join(","), plats.join(","), formatos.join(","));
    println!("próximos passos:");
    println!("  1. vf auth platform=\"{}\"   (opcional: rascunho na plataforma e analíticos)", plats.join(","));
    println!("  2. claude   (nesta pasta) e peça: \"vamos começar o vídeo sobre <tema>\" — a skill factory conduz o resto");
    Ok(())
}

/// Liga o plugin ao Claude Code desta pasta e deixa um CLAUDE.md curto (só se ainda não existirem).
fn claude_wiring(root: &Path, dir: &Path) -> Result<()> {
    let settings = dir.join(".claude/settings.json");
    if !settings.exists() {
        std::fs::create_dir_all(settings.parent().unwrap())?;
        let v = serde_json::json!({
            "extraKnownMarketplaces": {
                "video-factory": { "source": { "source": "directory", "path": root.display().to_string() } }
            },
            "enabledPlugins": { "video-factory@video-factory": true }
        });
        std::fs::write(&settings, serde_json::to_string_pretty(&v)? + "\n")?;
    } else {
        eprintln!("aviso: {} já existe; habilite o plugin video-factory@video-factory nele se ainda não estiver", settings.display());
    }
    let md = dir.join("CLAUDE.md");
    if !md.exists() {
        std::fs::write(&md, CLAUDE_MD.replace("__ROOT__", &root.display().to_string()))?;
    }
    Ok(())
}

const CLAUDE_MD: &str = r#"# Projeto video-factory

Este vídeo é gerenciado pela video-factory (plugin em `__ROOT__`). Estado em `factory.yaml`.

- Comece e continue sempre pela skill `factory`: ela lê `factory.yaml` e conduz as etapas com a esteira de agentes.
- Plataformas e formatos estão em `factory.yaml > plataformas/formatos`. Publicação: rascunho por padrão
  (`vf publish`), só depois do QA estrito aprovado e com o OK do usuário; postar/agendar só se ele pedir.
- Analíticos: `vf analytics platform=<p>` grava em `out/analytics/`; a skill `metricas` analisa e sugere metadata.
- Bibliotecas de FX de referência em `ref/fx/` (pixelfx MIT, pixel-splats Apache-2.0); som procedural em `audio/procedural/`.
"#;

fn copy_missing(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_missing(&e.path(), &to)?;
        } else if !to.exists() {
            std::fs::copy(e.path(), &to)?;
        }
    }
    Ok(())
}

/// Clona (ou reaproveita) as bibliotecas de FX num cache compartilhado e liga em ref/fx/<nome>.
/// Falha de rede não impede o projeto: só avisa.
fn fx_refs(dir: &Path) {
    let cache = Path::new(&std::env::var("HOME").unwrap_or_default()).join(".cache/vf/ref");
    let _ = std::fs::create_dir_all(&cache);
    let fx = dir.join("ref/fx");
    let _ = std::fs::create_dir_all(&fx);
    for (name, url, lic) in FX_REFS {
        let dst = cache.join(name);
        if !dst.join(".git").exists() {
            let ok = Command::new("git").args(["clone", "-q", "--depth", "1", url]).arg(&dst).status().map(|s| s.success()).unwrap_or(false);
            if !ok {
                eprintln!("aviso: não consegui clonar {url}; rode depois: git clone --depth 1 {url} {}", dst.display());
                continue;
            }
        }
        let link = fx.join(name);
        if !link.exists() {
            #[cfg(unix)]
            let _ = std::os::unix::fs::symlink(&dst, &link);
        }
        eprintln!("ref/fx/{name} -> {} ({lic})", dst.display());
    }
}
