//! Agenda local de posts: `vf publish ... platform=tiktok at=...` grava o pedido aqui, porque a API do TikTok
//! não agenda. `vf agenda run` posta o que venceu; `vf agenda install` liga um LaunchAgent (macOS) que roda
//! isso a cada 5 min. O YouTube não passa por aqui: usa o publishAt nativo e publica mesmo com o Mac desligado.
//!
//!   ~/.config/vf/agenda/pendente/<id>.json   {id, quando, plataforma, conta, cwd, args}
//!   ~/.config/vf/agenda/feito/<id>.json      + resultado
//!   ~/.config/vf/agenda/falhou/<id>.json     + erro (não tenta de novo sozinho: `vf agenda retry id=`)
use crate::kv::Kv;
use crate::tempo;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

const LABEL: &str = "com.video-factory.agenda";

fn dir(sub: &str) -> PathBuf {
    crate::auth::cfg_dir().join("agenda").join(sub)
}

fn read(p: &std::path::Path) -> Option<Value> {
    serde_json::from_slice(&std::fs::read(p).ok()?).ok()
}

fn jobs(sub: &str) -> Vec<(PathBuf, Value)> {
    let mut v: Vec<(PathBuf, Value)> = std::fs::read_dir(dir(sub))
        .map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).filter_map(|p| Some((p.clone(), read(&p)?))).collect())
        .unwrap_or_default();
    v.sort_by_key(|(_, j)| j["quando"].as_i64().unwrap_or(0));
    v
}

fn plist_path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/LaunchAgents").join(format!("{LABEL}.plist"))
}

fn runner_on() -> bool {
    if cfg!(target_os = "macos") {
        plist_path().exists()
    } else {
        std::process::Command::new("crontab").arg("-l").output().map(|o| String::from_utf8_lossy(&o.stdout).contains("vf agenda run") || String::from_utf8_lossy(&o.stdout).contains("agenda run")).unwrap_or(false)
    }
}

/// grava o pedido; devolve o registro que o `vf publish` imprime
pub fn enqueue(args: &[String], quando: i64, plataforma: &str, conta: &str) -> Result<Value> {
    let d = dir("pendente");
    std::fs::create_dir_all(&d)?;
    let id = format!("{}-{}", tempo::rfc3339(quando).replace([':', '-'], "").trim_end_matches('Z'), crate::net::rand_hex(3));
    let job = json!({
        "id": id, "quando": quando, "quando_local": tempo::local(quando), "plataforma": plataforma, "conta": conta,
        "cwd": std::env::current_dir()?.display().to_string(), "args": args, "criado": tempo::rfc3339(tempo::now())
    });
    crate::util::write_json(&d.join(format!("{id}.json")), &job)?;
    if !runner_on() {
        eprintln!("ATENÇÃO: nada está rodando a agenda nesta máquina. Rode uma vez: vf agenda install\n  (o post sai na hora só se o computador estiver ligado; com ele desligado, sai na próxima vez que ligar)");
    }
    eprintln!("não mova nem apague {} até a hora do post", args.first().map(|s| s.as_str()).unwrap_or("o vídeo"));
    Ok(json!({ "plataforma": plataforma, "conta": conta, "estado": format!("agendado localmente para {}", tempo::local(quando)), "agenda_id": id, "cancelar": format!("vf agenda cancel id={id}") }))
}

pub fn run(args: &[String]) -> Result<()> {
    let (action, rest) = match args.first().map(|s| s.as_str()) {
        Some(a @ ("list" | "ls" | "run" | "cancel" | "retry" | "install" | "uninstall")) => (a, &args[1..]),
        Some(x) if !x.contains('=') => bail!("ação desconhecida: {x} (list | run | cancel id= | retry id= | install | uninstall)"),
        _ => ("list", args),
    };
    let kv = Kv::parse(rest, &[])?;
    kv.check_known(&["id"])?;
    match action {
        "list" | "ls" => list(),
        "run" => run_due(),
        "cancel" | "retry" => {
            let id = kv.get("id").context("diga qual: id=<id> (vf agenda list)")?;
            let (from, to) = if action == "cancel" { ("pendente", "cancelado") } else { ("falhou", "pendente") };
            let src = dir(from).join(format!("{id}.json"));
            if !src.exists() {
                bail!("{id} não está em {from} (vf agenda list)");
            }
            std::fs::create_dir_all(dir(to))?;
            std::fs::rename(&src, dir(to).join(format!("{id}.json")))?;
            println!("{id}: {from} -> {to}");
            Ok(())
        }
        "install" => install(),
        _ => uninstall(),
    }
}

fn list() -> Result<()> {
    let now = tempo::now();
    for sub in ["pendente", "falhou", "feito"] {
        let js = jobs(sub);
        if js.is_empty() {
            continue;
        }
        println!("== {sub}");
        for (_, j) in js.iter().rev().take(if sub == "feito" { 10 } else { usize::MAX }).collect::<Vec<_>>().into_iter().rev() {
            let q = j["quando"].as_i64().unwrap_or(0);
            println!(
                "{:<28} {} {:<8} {:<16} {}{}",
                j["id"].as_str().unwrap_or(""),
                j["quando_local"].as_str().unwrap_or(""),
                j["plataforma"].as_str().unwrap_or(""),
                j["conta"].as_str().unwrap_or(""),
                j["args"].get(0).and_then(|x| x.as_str()).unwrap_or(""),
                match sub {
                    "pendente" if q <= now => "  (vencido: sai no próximo vf agenda run)".to_string(),
                    "falhou" => format!("  erro: {}", j["erro"].as_str().unwrap_or("")),
                    _ => String::new(),
                }
            );
        }
    }
    println!("executor: {}", if runner_on() { "ligado" } else { "DESLIGADO (vf agenda install)" });
    Ok(())
}

fn run_due() -> Result<()> {
    std::fs::create_dir_all(dir("pendente"))?;
    // trava simples contra duas execuções ao mesmo tempo (launchd + manual)
    let lock = dir("").join(".lock");
    if let Ok(m) = std::fs::metadata(&lock) {
        let idade = m.modified().ok().and_then(|t| t.elapsed().ok()).map(|d| d.as_secs()).unwrap_or(0);
        if idade < 3600 {
            eprintln!("outra vf agenda run em andamento ({lock:?}); saindo");
            return Ok(());
        }
    }
    std::fs::write(&lock, std::process::id().to_string())?;
    let now = tempo::now();
    let original = std::env::current_dir()?;
    let mut n = 0;
    for (path, mut j) in jobs("pendente") {
        if j["quando"].as_i64().unwrap_or(i64::MAX) > now {
            continue;
        }
        n += 1;
        let id = j["id"].as_str().unwrap_or("").to_string();
        let args: Vec<String> = j["args"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        eprintln!("[{}] postando {id}…", tempo::local(now));
        let r = std::env::set_current_dir(j["cwd"].as_str().unwrap_or(".")).context("pasta do projeto sumiu").and_then(|_| crate::platform::publish_run(&args, true));
        let _ = std::env::set_current_dir(&original);
        let to = match r {
            Ok(res) => {
                j["resultado"] = json!(res);
                "feito"
            }
            Err(e) => {
                eprintln!("{id}: {e:#}");
                j["erro"] = json!(format!("{e:#}"));
                "falhou"
            }
        };
        j["executado"] = json!(tempo::rfc3339(tempo::now()));
        std::fs::create_dir_all(dir(to))?;
        crate::util::write_json(&dir(to).join(format!("{id}.json")), &j)?;
        let _ = std::fs::remove_file(&path);
    }
    let _ = std::fs::remove_file(&lock);
    if n == 0 {
        eprintln!("nada vencido na agenda");
    }
    Ok(())
}

fn install() -> Result<()> {
    let exe = std::env::current_exe()?.canonicalize()?;
    let log = dir("").join("agenda.log");
    std::fs::create_dir_all(dir(""))?;
    if !cfg!(target_os = "macos") {
        println!("adicione ao crontab (crontab -e):\n*/5 * * * * {} agenda run >> {} 2>&1", exe.display(), log.display());
        return Ok(());
    }
    let path_env = std::env::var("PATH").unwrap_or_default();
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{LABEL}</string>
  <key>ProgramArguments</key><array><string>{exe}</string><string>agenda</string><string>run</string></array>
  <key>StartInterval</key><integer>300</integer>
  <key>RunAtLoad</key><true/>
  <key>EnvironmentVariables</key><dict><key>PATH</key><string>{path}</string></dict>
  <key>StandardOutPath</key><string>{log}</string>
  <key>StandardErrorPath</key><string>{log}</string>
</dict>
</plist>
"#,
        exe = xml(&exe.display().to_string()),
        path = xml(&path_env),
        log = xml(&log.display().to_string())
    );
    let p = plist_path();
    std::fs::create_dir_all(p.parent().unwrap())?;
    let _ = std::process::Command::new("launchctl").args(["unload", &p.display().to_string()]).output();
    std::fs::write(&p, plist)?;
    let st = std::process::Command::new("launchctl").args(["load", "-w", &p.display().to_string()]).status()?;
    if !st.success() {
        bail!("launchctl load falhou ({})", p.display());
    }
    println!("agenda ligada: {} roda `vf agenda run` a cada 5 min (log: {})", p.display(), log.display());
    Ok(())
}

fn uninstall() -> Result<()> {
    let p = plist_path();
    if p.exists() {
        let _ = std::process::Command::new("launchctl").args(["unload", &p.display().to_string()]).output();
        std::fs::remove_file(&p)?;
        println!("agenda desligada ({} removido); pedidos pendentes continuam em {}", p.display(), dir("pendente").display());
    } else {
        println!("a agenda não estava ligada");
    }
    Ok(())
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
