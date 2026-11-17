//! `vf auth platform="yt,tiktok"`: OAuth 2.0 com PKCE e redirect de loopback (o navegador abre, você autoriza,
//! o `vf` recebe o código em 127.0.0.1). Tokens em ~/.config/vf/auth/<plataforma>.json (modo 600).
//!
//! Credenciais do app (uma vez por máquina; ver skills/edicao/references/plataformas.md):
//!   YouTube: VF_YT_CLIENT_ID + VF_YT_CLIENT_SECRET, ou ~/.config/vf/yt-client.json (o JSON "Desktop app" baixado do Google Cloud)
//!   TikTok:  VF_TIKTOK_CLIENT_KEY + VF_TIKTOK_CLIENT_SECRET, ou ~/.config/vf/tiktok-client.json {client_key, client_secret, redirect_uri}
use crate::kv::{self, Kv};
use crate::net::{self, Body};
use anyhow::{bail, Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const YT_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/youtube.upload",
    "https://www.googleapis.com/auth/youtube.readonly",
    "https://www.googleapis.com/auth/yt-analytics.readonly",
];
pub const TT_SCOPES: &[&str] = &["user.info.basic", "user.info.stats", "video.list", "video.upload"];
const TT_REDIRECT: &str = "http://localhost:8723/callback/";

#[derive(Serialize, Deserialize, Clone)]
pub struct Token {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    /// unix s
    pub expires_at: u64,
    #[serde(default)]
    pub scope: String,
    /// TikTok: open_id do usuário
    #[serde(default)]
    pub open_id: String,
}

pub struct Client {
    pub id: String,
    pub secret: String,
    pub redirect: Option<String>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn cfg_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"));
    base.join("vf")
}

fn token_path(p: &str) -> PathBuf {
    cfg_dir().join("auth").join(format!("{p}.json"))
}

fn label(p: &str) -> &'static str {
    if p == "yt" { "YouTube" } else { "TikTok" }
}

pub fn client(p: &str) -> Result<Client> {
    let (ki, ks) = if p == "yt" { ("VF_YT_CLIENT_ID", "VF_YT_CLIENT_SECRET") } else { ("VF_TIKTOK_CLIENT_KEY", "VF_TIKTOK_CLIENT_SECRET") };
    if let (Ok(id), Ok(secret)) = (std::env::var(ki), std::env::var(ks)) {
        let redirect = if p == "tiktok" { std::env::var("VF_TIKTOK_REDIRECT").ok() } else { None };
        return Ok(Client { id, secret, redirect });
    }
    let f = cfg_dir().join(format!("{p}-client.json"));
    let raw = std::fs::read(&f).with_context(|| {
        format!(
            "credenciais do app {} não encontradas: defina {ki}/{ks} ou crie {} (passo a passo: skills/edicao/references/plataformas.md)",
            label(p),
            f.display()
        )
    })?;
    let v: serde_json::Value = serde_json::from_slice(&raw).with_context(|| format!("{} inválido", f.display()))?;
    let v = v.get("installed").cloned().unwrap_or(v);
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(String::from);
    Ok(Client {
        id: s("client_id").or(s("client_key")).context("client_id/client_key ausente")?,
        secret: s("client_secret").context("client_secret ausente")?,
        redirect: s("redirect_uri"),
    })
}

fn save(p: &str, t: &Token) -> Result<()> {
    let path = token_path(p);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, serde_json::to_vec_pretty(t)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn load(p: &str) -> Option<Token> {
    serde_json::from_slice(&std::fs::read(token_path(p)).ok()?).ok()
}

fn parse_token(p: &str, v: &serde_json::Value, old: Option<&Token>) -> Result<Token> {
    // TikTok devolve erro com 200 em alguns casos
    if let Some(e) = v.get("error").filter(|e| !e.is_null() && e.as_str() != Some("ok")) {
        bail!("{}: {} {}", label(p), e, v.get("error_description").unwrap_or(&serde_json::Value::Null));
    }
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(String::from);
    Ok(Token {
        access_token: s("access_token").context("access_token ausente na resposta")?,
        refresh_token: s("refresh_token").or(old.map(|o| o.refresh_token.clone())).unwrap_or_default(),
        expires_at: now() + v.get("expires_in").and_then(|x| x.as_u64()).unwrap_or(3600),
        scope: s("scope").or(old.map(|o| o.scope.clone())).unwrap_or_default(),
        open_id: s("open_id").or(old.map(|o| o.open_id.clone())).unwrap_or_default(),
    })
}

/// Token válido (renova sozinho com o refresh_token).
pub fn token(p: &str) -> Result<Token> {
    let t = load(p).with_context(|| format!("{} não autorizado: rode vf auth platform={p}", label(p)))?;
    if t.expires_at > now() + 60 {
        return Ok(t);
    }
    if t.refresh_token.is_empty() {
        bail!("token do {} expirou e não há refresh_token: rode vf auth platform={p}", label(p));
    }
    let c = client(p)?;
    let r = if p == "yt" {
        net::request("POST", "https://oauth2.googleapis.com/token", &[], Body::Form(&[("client_id", &c.id), ("client_secret", &c.secret), ("refresh_token", &t.refresh_token), ("grant_type", "refresh_token")]))?
    } else {
        net::request("POST", "https://open.tiktokapis.com/v2/oauth/token/", &[], Body::Form(&[("client_key", &c.id), ("client_secret", &c.secret), ("refresh_token", &t.refresh_token), ("grant_type", "refresh_token")]))?
    };
    let nt = parse_token(p, &r.json()?, Some(&t)).with_context(|| format!("renovar token do {} (rode vf auth platform={p} de novo)", label(p)))?;
    save(p, &nt)?;
    Ok(nt)
}

pub fn run(args: &[String]) -> Result<()> {
    let (action, rest) = match args.first().map(|s| s.as_str()) {
        Some("status") => ("status", &args[1..]),
        Some("logout") => ("logout", &args[1..]),
        Some("login") => ("login", &args[1..]),
        _ => ("login", args),
    };
    let kv = Kv::parse(rest, &[])?;
    kv.check_known(&["platform", "platforms", "port"])?;
    let mut plats = kv::platforms(&kv)?;
    if plats.is_empty() {
        if action == "login" {
            bail!("diga a plataforma: vf auth platform=\"yt,tiktok\"");
        }
        plats = vec!["yt", "tiktok"];
    }
    for p in plats {
        match action {
            "status" => match load(p) {
                None => println!("{:8} não autorizado", label(p)),
                Some(t) => {
                    let left = t.expires_at as i64 - now() as i64;
                    println!(
                        "{:8} autorizado · access {} · refresh {} · escopos: {}",
                        label(p),
                        if left > 0 { format!("válido por {} min", left / 60) } else { "expirado (renova sozinho)".into() },
                        if t.refresh_token.is_empty() { "ausente" } else { "ok" },
                        t.scope
                    );
                }
            },
            "logout" => {
                let _ = std::fs::remove_file(token_path(p));
                println!("{}: token apagado (revogue também o acesso na conta, se quiser)", label(p));
            }
            _ => login(p, kv.get("port").and_then(|x| x.parse().ok()))?,
        }
    }
    Ok(())
}

fn login(p: &str, port: Option<u16>) -> Result<()> {
    let c = client(p)?;
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(net::rand_bytes(48));
    let digest = Sha256::digest(verifier.as_bytes());
    let state = net::rand_hex(16);

    let (listener, redirect, url) = if p == "yt" {
        let l = TcpListener::bind(("127.0.0.1", port.unwrap_or(0))).context("abrir porta local para o retorno do OAuth")?;
        let redirect = format!("http://127.0.0.1:{}", l.local_addr()?.port());
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
        let q = net::query(&[
            ("client_id", &c.id),
            ("redirect_uri", &redirect),
            ("response_type", "code"),
            ("scope", &YT_SCOPES.join(" ")),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("access_type", "offline"),
            ("prompt", "consent"),
            ("state", &state),
        ]);
        (l, redirect, format!("https://accounts.google.com/o/oauth2/v2/auth?{q}"))
    } else {
        // TikTok exige o redirect cadastrado no app, idêntico; o desktop usa o challenge em HEX
        let redirect = c.redirect.clone().unwrap_or_else(|| TT_REDIRECT.into());
        let rport = redirect.split("://").nth(1).and_then(|h| h.split('/').next()).and_then(|h| h.rsplit(':').next()).and_then(|x| x.parse().ok()).unwrap_or(8723u16);
        let l = TcpListener::bind(("127.0.0.1", port.unwrap_or(rport))).with_context(|| format!("abrir a porta {rport} do redirect {redirect}"))?;
        let challenge: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let q = net::query(&[
            ("client_key", &c.id),
            ("redirect_uri", &redirect),
            ("response_type", "code"),
            ("scope", &TT_SCOPES.join(",")),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("state", &state),
        ]);
        (l, redirect, format!("https://www.tiktok.com/v2/auth/authorize/?{q}"))
    };

    eprintln!("{}: abrindo o navegador para autorizar. Se não abrir, acesse:\n{url}\n", label(p));
    let _ = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" }).arg(&url).status();
    let code = wait_code(&listener, &state, Duration::from_secs(300))?;

    let r = if p == "yt" {
        net::request(
            "POST",
            "https://oauth2.googleapis.com/token",
            &[],
            Body::Form(&[("code", &code), ("client_id", &c.id), ("client_secret", &c.secret), ("redirect_uri", &redirect), ("grant_type", "authorization_code"), ("code_verifier", &verifier)]),
        )?
    } else {
        net::request(
            "POST",
            "https://open.tiktokapis.com/v2/oauth/token/",
            &[],
            Body::Form(&[("client_key", &c.id), ("client_secret", &c.secret), ("code", &code), ("grant_type", "authorization_code"), ("redirect_uri", &redirect), ("code_verifier", &verifier)]),
        )?
    };
    let t = parse_token(p, &r.json()?, None)?;
    save(p, &t)?;
    println!("{}: autorizado ({}) -> {}", label(p), t.scope, token_path(p).display());
    Ok(())
}

/// Espera o navegador voltar em /?code=...&state=... e responde uma página simples.
fn wait_code(l: &TcpListener, state: &str, timeout: Duration) -> Result<String> {
    l.set_nonblocking(true)?;
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() > deadline {
            bail!("tempo esgotado esperando a autorização no navegador");
        }
        let (mut s, _) = match l.accept() {
            Ok(x) => x,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        s.set_nonblocking(false)?;
        s.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut buf = [0u8; 8192];
        let n = s.read(&mut buf).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("/");
        let params: Vec<(String, String)> = path
            .split_once('?')
            .map(|(_, q)| q.split('&').filter_map(|kv| kv.split_once('=')).map(|(k, v)| (net::dec(k), net::dec(v))).collect())
            .unwrap_or_default();
        let get = |k: &str| params.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
        let (msg, out) = if let Some(e) = get("error") {
            ("Autorização recusada. Pode fechar esta aba.", Some(Err(anyhow::anyhow!("autorização recusada: {e} {}", get("error_description").unwrap_or_default()))))
        } else if let Some(code) = get("code") {
            if get("state").as_deref() != Some(state) {
                ("Resposta inválida (state).", Some(Err(anyhow::anyhow!("state não confere: resposta ignorada"))))
            } else {
                ("video-factory autorizada. Pode fechar esta aba e voltar ao terminal.", Some(Ok(code)))
            }
        } else {
            ("", None) // favicon etc.
        };
        let html = format!("<!doctype html><meta charset=utf-8><title>vf auth</title><p style=\"font:16px system-ui;margin:3em\">{msg}</p>");
        let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}", html.len());
        if let Some(r) = out {
            return r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loopback_code() {
        let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        let th = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            let _ = std::process::Command::new("curl").args(["-s", &format!("http://127.0.0.1:{port}/favicon.ico")]).output();
            std::process::Command::new("curl").args(["-s", &format!("http://127.0.0.1:{port}/callback/?code=ab%2Fc&state=xyz&scopes=a")]).output().unwrap()
        });
        assert_eq!(wait_code(&l, "xyz", Duration::from_secs(5)).unwrap(), "ab/c");
        assert!(String::from_utf8_lossy(&th.join().unwrap().stdout).contains("autorizada"));
    }
}
