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
    // editar título/descrição/tags de vídeo já enviado (vf meta) e ler comentários (vf analytics comentarios=1)
    "https://www.googleapis.com/auth/youtube.force-ssl",
];
pub const TT_SCOPES: &[&str] = &["user.info.basic", "user.info.stats", "video.list", "video.upload"];
/// post direto (vf publish platform=tiktok mode=publico): só com `vf auth platform=tiktok direct=1`, porque o app
/// precisa do produto Direct Post aprovado; sem ele o TikTok recusa o login inteiro.
pub const TT_DIRECT_SCOPE: &str = "video.publish";
/// TikTok Business (Accounts API, business-api.tiktok.com): retenção, tempo médio, alcance. EXPERIMENTAL:
/// app no TikTok for Business + conta Business/Creator. Escopos sobrescrevíveis em ttb-client.json > "scopes".
pub const TTB_SCOPES: &str = "user.info.basic,user.insights,video.list,video.insights";
pub const TTB_API: &str = "https://business-api.tiktok.com/open_api/v1.3";

/// resposta da Business API: {code: 0, message, data}
pub fn ttb_data(v: &serde_json::Value) -> Result<serde_json::Value> {
    match v.get("code").and_then(|c| c.as_i64()) {
        Some(0) => Ok(v.get("data").cloned().unwrap_or(serde_json::Value::Null)),
        _ => bail!("TikTok Business: {} (resposta: {})", v.get("message").and_then(|m| m.as_str()).unwrap_or("erro"), v.to_string().chars().take(400).collect::<String>()),
    }
}

/// o token foi concedido com este escopo? (o Google devolve URLs separadas por espaço; o TikTok, por vírgula)
pub fn has_scope(t: &Token, s: &str) -> bool {
    t.scope.split([' ', ',']).any(|x| x == s)
}
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
    /// nome do canal/conta mostrado pela plataforma (preenchido no login)
    #[serde(default)]
    pub nome: String,
    /// id do canal (YouTube) ou open_id (TikTok)
    #[serde(default)]
    pub conta_id: String,
}

pub struct Client {
    pub id: String,
    pub secret: String,
    pub redirect: Option<String>,
    /// escopos pedidos no login (só TikTok Business; padrão TTB_SCOPES)
    pub scopes: Option<String>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn cfg_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"));
    base.join("vf")
}

// ---------------- várias contas por plataforma ----------------
// ~/.config/vf/auth/<p>/<apelido>.json  (um token por conta)  +  ~/.config/vf/auth/<p>/padrao (apelido padrão)
// O app (client) é um só por plataforma; cada conta/canal autoriza esse app separadamente.

fn plat_dir(p: &str) -> PathBuf {
    cfg_dir().join("auth").join(p)
}

fn token_path(p: &str, conta: &str) -> PathBuf {
    plat_dir(p).join(format!("{conta}.json"))
}

/// apelido válido: minúsculas, números, - e _
pub fn slug(s: &str) -> String {
    let mut o = String::new();
    for c in s.trim().to_lowercase().chars() {
        let c = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'ê' | 'è' => 'e',
            'í' | 'î' => 'i',
            'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() || c == '_' {
            o.push(c);
        } else if !o.ends_with('-') {
            o.push('-');
        }
    }
    let o = o.trim_matches('-').chars().take(40).collect::<String>();
    if o.is_empty() { "principal".into() } else { o }
}

/// migra o token antigo de conta única (~/.config/vf/auth/<p>.json) para <p>/principal.json
fn migrate(p: &str) {
    let old = cfg_dir().join("auth").join(format!("{p}.json"));
    if old.is_file() {
        let _ = std::fs::create_dir_all(plat_dir(p));
        let dst = token_path(p, "principal");
        if !dst.exists() && std::fs::rename(&old, &dst).is_ok() && default_account(p).is_none() {
            let _ = set_default(p, "principal");
        }
    }
}

pub fn accounts(p: &str) -> Vec<String> {
    migrate(p);
    let mut v: Vec<String> = std::fs::read_dir(plat_dir(p))
        .map(|d| d.filter_map(|e| e.ok()).filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".json")).map(String::from)).collect())
        .unwrap_or_default();
    v.sort();
    v
}

fn default_account(p: &str) -> Option<String> {
    std::fs::read_to_string(plat_dir(p).join("padrao")).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && token_path(p, s).exists())
}

fn set_default(p: &str, conta: &str) -> Result<()> {
    std::fs::create_dir_all(plat_dir(p))?;
    std::fs::write(plat_dir(p).join("padrao"), conta)?;
    Ok(())
}

/// Resolve a conta: explícita > `factory.yaml > contas.<p>` do projeto atual > padrão > única existente.
pub fn resolve(p: &str, conta: Option<&str>) -> Result<String> {
    migrate(p);
    let all = accounts(p);
    if all.is_empty() {
        bail!("{} não autorizado: rode vf auth platform={p} [account=<apelido>]", label(p));
    }
    let pick = conta.map(slug).or_else(|| project_account(p)).or_else(|| default_account(p)).or_else(|| (all.len() == 1).then(|| all[0].clone()));
    match pick {
        Some(c) if all.contains(&c) => Ok(c),
        Some(c) => bail!("{}: conta \"{c}\" não existe (contas: {}). Autorize com vf auth platform={p} account={c}", label(p), all.join(", ")),
        None => bail!("{}: várias contas ({}) e nenhuma padrão: use account=<apelido> ou vf auth use platform={p} account=<apelido>", label(p), all.join(", ")),
    }
}

/// `contas: { yt: canal-x, tiktok: canal-x }` no factory.yaml da pasta atual (o projeto escolhe a conta)
fn project_account(p: &str) -> Option<String> {
    let y: serde_yaml::Value = serde_yaml::from_str(&std::fs::read_to_string("factory.yaml").ok()?).ok()?;
    y.get("contas")?.get(p)?.as_str().map(slug).filter(|s| !s.is_empty())
}

fn label(p: &str) -> &'static str {
    match p {
        "yt" => "YouTube",
        "ttb" => "TikTok Business",
        _ => "TikTok",
    }
}

pub fn client(p: &str) -> Result<Client> {
    let (ki, ks) = match p {
        "yt" => ("VF_YT_CLIENT_ID", "VF_YT_CLIENT_SECRET"),
        "ttb" => ("VF_TTB_CLIENT_KEY", "VF_TTB_CLIENT_SECRET"),
        _ => ("VF_TIKTOK_CLIENT_KEY", "VF_TIKTOK_CLIENT_SECRET"),
    };
    if let (Ok(id), Ok(secret)) = (std::env::var(ki), std::env::var(ks)) {
        let redirect = match p {
            "tiktok" => std::env::var("VF_TIKTOK_REDIRECT").ok(),
            "ttb" => std::env::var("VF_TTB_REDIRECT").ok(),
            _ => None,
        };
        return Ok(Client { id, secret, redirect, scopes: None });
    }
    let f = cfg_dir().join(format!("{p}-client.json"));
    let raw = match std::fs::read(&f) {
        Ok(r) => r,
        Err(_) => {
            let doc = crate::init::plugin_root().map(|r| r.join("skills/edicao/references/plataformas.md").display().to_string()).unwrap_or_else(|_| "skills/edicao/references/plataformas.md".into());
            let how = if p == "ttb" {
                "1. business-api.tiktok.com → My Apps → crie um app com acesso a \"TikTok Accounts\" (dados de conta orgânica)\n  \
                 2. Permissões: dados do perfil, lista de posts e insights de vídeo (user.insights, video.insights)\n  \
                 3. Redirect URI: http://localhost:8724/callback/\n  \
                 4. vf auth platform=tiktok-biz client=<json com client_key (app id) e client_secret>\n  \
                 A conta TikTok precisa ser Business ou Creator. Recurso EXPERIMENTAL."
            } else if p == "yt" {
                "1. console.cloud.google.com → crie um projeto → ative \"YouTube Data API v3\" e \"YouTube Analytics API\"\n  \
                 2. Tela de consentimento OAuth: Externo, adicione seu e-mail como usuário de teste\n  \
                 3. Credenciais → Criar ID do cliente OAuth → tipo \"App para computador\" → baixe o JSON\n  \
                 4. vf auth platform=yt client=~/Downloads/client_secret_....json"
            } else {
                "1. developers.tiktok.com → crie um app com Login Kit (Desktop) e Content Posting API\n  \
                 2. Escopos: user.info.basic, user.info.stats, video.list, video.upload\n  \
                 3. Redirect URI: http://localhost:8723/callback/\n  \
                 4. vf auth platform=tiktok client=<json com client_key e client_secret>"
            };
            anyhow::bail!(
                "o {} ainda não tem um app autorizado nesta máquina (é uma vez só).\n  {how}\n  (ou defina {ki}/{ks})\n  passo a passo: {doc}",
                label(p)
            );
        }
    };
    let v: serde_json::Value = serde_json::from_slice(&raw).with_context(|| format!("{} inválido", f.display()))?;
    let v = v.get("installed").cloned().unwrap_or(v);
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(String::from);
    Ok(Client {
        id: s("client_id").or(s("client_key")).context("client_id/client_key ausente")?,
        secret: s("client_secret").context("client_secret ausente")?,
        redirect: s("redirect_uri"),
        scopes: s("scopes"),
    })
}

/// copia o JSON de credenciais baixado (Google "App para computador" ou {client_key, client_secret} do TikTok)
/// para ~/.config/vf/<p>-client.json (modo 600), validando antes
fn install_client(p: &str, src: &str) -> Result<()> {
    let src = if let Some(rest) = src.strip_prefix("~/") { PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(rest) } else { PathBuf::from(src) };
    let raw = std::fs::read(&src).with_context(|| format!("não consegui ler {}", src.display()))?;
    let v: serde_json::Value = serde_json::from_slice(&raw).with_context(|| format!("{} não é JSON", src.display()))?;
    let inner = v.get("installed").or(v.get("web")).cloned().unwrap_or(v.clone());
    if v.get("web").is_some() {
        bail!("esse JSON é de um cliente \"Aplicativo da Web\"; crie um ID do cliente do tipo \"App para computador\"");
    }
    let has = |k: &str| inner.get(k).and_then(|x| x.as_str()).is_some();
    if !(has("client_secret") && (has("client_id") || has("client_key"))) {
        bail!("{} não tem client_id/client_key e client_secret", src.display());
    }
    let dst = cfg_dir().join(format!("{p}-client.json"));
    std::fs::create_dir_all(dst.parent().unwrap())?;
    std::fs::write(&dst, &raw)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o600))?;
    }
    eprintln!("credenciais do app {} salvas em {} (pode apagar o arquivo baixado)", label(p), dst.display());
    Ok(())
}

fn save(p: &str, conta: &str, t: &Token) -> Result<()> {
    let path = token_path(p, conta);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, serde_json::to_vec_pretty(t)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn load(p: &str, conta: &str) -> Option<Token> {
    serde_json::from_slice(&std::fs::read(token_path(p, conta)).ok()?).ok()
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
        nome: old.map(|o| o.nome.clone()).unwrap_or_default(),
        conta_id: old.map(|o| o.conta_id.clone()).unwrap_or_default(),
    })
}

/// Token válido da conta escolhida (renova sozinho com o refresh_token). Devolve (apelido, token).
pub fn token(p: &str, conta: Option<&str>) -> Result<(String, Token)> {
    let conta = resolve(p, conta)?;
    let t = load(p, &conta).with_context(|| format!("token de {}/{conta} ilegível: rode vf auth platform={p} account={conta}", label(p)))?;
    if t.expires_at > now() + 60 {
        return Ok((conta, t));
    }
    if t.refresh_token.is_empty() {
        bail!("token de {}/{conta} expirou e não há refresh_token: rode vf auth platform={p} account={conta}", label(p));
    }
    let c = client(p)?;
    let r = if p == "ttb" {
        let r = net::request("POST", &format!("{TTB_API}/tt_user/oauth2/refresh_token/"), &[], Body::Json(&serde_json::json!({ "client_id": c.id, "client_secret": c.secret, "grant_type": "refresh_token", "refresh_token": t.refresh_token })))?;
        let d = ttb_data(&r.json()?).with_context(|| format!("renovar token de {}/{conta}", label(p)))?;
        let nt = parse_token(p, &d, Some(&t))?;
        save(p, &conta, &nt)?;
        return Ok((conta, nt));
    } else if p == "yt" {
        net::request("POST", "https://oauth2.googleapis.com/token", &[], Body::Form(&[("client_id", &c.id), ("client_secret", &c.secret), ("refresh_token", &t.refresh_token), ("grant_type", "refresh_token")]))?
    } else {
        net::request("POST", "https://open.tiktokapis.com/v2/oauth/token/", &[], Body::Form(&[("client_key", &c.id), ("client_secret", &c.secret), ("refresh_token", &t.refresh_token), ("grant_type", "refresh_token")]))?
    };
    let nt = parse_token(p, &r.json()?, Some(&t)).with_context(|| format!("renovar token de {}/{conta} (rode vf auth platform={p} account={conta})", label(p)))?;
    save(p, &conta, &nt)?;
    Ok((conta, nt))
}

/// Quem é a conta autorizada: (nome, id). YouTube: canal; TikTok: display_name/open_id.
fn whoami(p: &str, t: &Token) -> (String, String) {
    let b = format!("Bearer {}", t.access_token);
    if p == "ttb" {
        let url = format!("{TTB_API}/business/get/?{}", net::query(&[("business_id", &t.open_id), ("fields", "[\"display_name\",\"username\",\"followers_count\"]")]));
        let nome = match net::get(&url, &[("Access-Token", &t.access_token)]).and_then(|r| r.json()).and_then(|v| ttb_data(&v)) {
            Ok(d) => d.get("display_name").or(d.get("username")).and_then(|x| x.as_str()).unwrap_or("").to_string(),
            Err(e) => {
                eprintln!("aviso: não consegui ler o perfil Business: {e:#}");
                String::new()
            }
        };
        return (nome, t.open_id.clone());
    }
    if p == "yt" {
        let v = match net::get("https://www.googleapis.com/youtube/v3/channels?part=snippet&mine=true", &[("Authorization", &b)]).and_then(|r| r.ok()).and_then(|r| r.json()) {
            Ok(v) => Some(v),
            Err(e) => {
                // sem isso o login "dá certo" mas o canal fica sem nome e publish/analytics falham depois
                eprintln!("aviso: não consegui ler o canal desta conta: {e:#}");
                None
            }
        };
        let it = v.as_ref().and_then(|v| v.pointer("/items/0"));
        let nome = it.and_then(|i| i.pointer("/snippet/customUrl").or(i.pointer("/snippet/title"))).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let id = it.and_then(|i| i.get("id")).and_then(|x| x.as_str()).unwrap_or("").to_string();
        (nome, id)
    } else {
        let v = net::get("https://open.tiktokapis.com/v2/user/info/?fields=open_id,display_name", &[("Authorization", &b)]).and_then(|r| r.json()).ok();
        let nome = v.as_ref().and_then(|v| v.pointer("/data/user/display_name")).and_then(|x| x.as_str()).unwrap_or("").to_string();
        (nome, t.open_id.clone())
    }
}

pub fn run(args: &[String]) -> Result<()> {
    let (action, rest) = match args.first().map(|s| s.as_str()) {
        Some(a @ ("status" | "logout" | "login" | "use" | "list")) => (if a == "list" { "status" } else { a }, &args[1..]),
        _ => ("login", args),
    };
    let kv = Kv::parse(rest, &[])?;
    kv.check_known(&["platform", "platforms", "port", "client", "account", "conta", "direct", "direto"])?;
    let conta = kv.get("account").or(kv.get("conta"));
    let mut plats = kv::platforms(&kv)?;
    if plats.is_empty() {
        if action != "status" {
            bail!("diga a plataforma: vf auth {}platform=\"yt,tiktok\"", if action == "login" { "" } else { "<ação> " });
        }
        plats = vec!["yt", "tiktok", "ttb"];
    }
    for p in plats {
        match action {
            "status" => {
                let all = accounts(p);
                if all.is_empty() {
                    println!("{:8} nenhuma conta autorizada", label(p));
                    continue;
                }
                let def = default_account(p);
                let proj = project_account(p);
                for c in all {
                    let Some(t) = load(p, &c) else { continue };
                    let left = t.expires_at as i64 - now() as i64;
                    println!(
                        "{:8} {}{:<20} {:<28} access {} · refresh {}{}",
                        label(p),
                        if def.as_deref() == Some(c.as_str()) { "* " } else { "  " },
                        c,
                        if t.nome.is_empty() { "(nome não lido)".to_string() } else { t.nome.clone() },
                        if left > 0 { format!("válido {} min", left / 60) } else { "expirado (renova sozinho)".into() },
                        if t.refresh_token.is_empty() { "ausente" } else { "ok" },
                        if proj.as_deref() == Some(c.as_str()) { " · usada por este projeto" } else { "" }
                    );
                }
            }
            "use" => {
                let c = slug(conta.context("diga a conta: vf auth use platform=yt account=<apelido>")?);
                if !token_path(p, &c).exists() {
                    bail!("{}: conta \"{c}\" não existe (contas: {})", label(p), accounts(p).join(", "));
                }
                set_default(p, &c)?;
                println!("{}: conta padrão = {c}", label(p));
            }
            "logout" => {
                let c = resolve(p, conta)?;
                std::fs::remove_file(token_path(p, &c))?;
                if default_account(p).is_none() {
                    let _ = std::fs::remove_file(plat_dir(p).join("padrao"));
                    if let Some(first) = accounts(p).first() {
                        set_default(p, first)?;
                    }
                }
                println!("{}: conta {c} removida desta máquina (revogue também o acesso na conta, se quiser)", label(p));
            }
            _ => {
                if let Some(c) = kv.get("client") {
                    install_client(p, c)?;
                }
                login(p, conta, kv.get("port").and_then(|x| x.parse().ok()), kv.flag("direct") || kv.flag("direto"))?
            }
        }
    }
    Ok(())
}

fn login(p: &str, conta: Option<&str>, port: Option<u16>, direct: bool) -> Result<()> {
    let c = client(p)?;
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(net::rand_bytes(48));
    let digest = Sha256::digest(verifier.as_bytes());
    let state = net::rand_hex(16);

    let (listener, redirect, url) = if p == "ttb" {
        // Accounts API do TikTok for Business: mesmo authorize do TikTok, troca do código na business-api, sem PKCE
        let redirect = c.redirect.clone().unwrap_or_else(|| "http://localhost:8724/callback/".into());
        let rport = redirect.split("://").nth(1).and_then(|h| h.split('/').next()).and_then(|h| h.rsplit(':').next()).and_then(|x| x.parse().ok()).unwrap_or(8724u16);
        let l = TcpListener::bind(("127.0.0.1", port.unwrap_or(rport))).with_context(|| format!("abrir a porta {rport} do redirect {redirect}"))?;
        let q = net::query(&[("client_key", &c.id), ("redirect_uri", &redirect), ("response_type", "code"), ("scope", c.scopes.as_deref().unwrap_or(TTB_SCOPES)), ("state", &state)]);
        (l, redirect, format!("https://www.tiktok.com/v2/auth/authorize/?{q}"))
    } else if p == "yt" {
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
            ("prompt", "consent select_account"), // escolher a conta/canal a cada login: várias contas
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
            ("scope", &TT_SCOPES.iter().copied().chain(direct.then_some(TT_DIRECT_SCOPE)).collect::<Vec<_>>().join(",")),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("state", &state),
        ]);
        (l, redirect, format!("https://www.tiktok.com/v2/auth/authorize/?{q}"))
    };

    eprintln!("{}: abrindo o navegador para autorizar. Se não abrir, acesse:\n{url}\n", label(p));
    let _ = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" }).arg(&url).status();
    let code = wait_code(&listener, &state, Duration::from_secs(300))?;

    let r = if p == "ttb" {
        let r = net::request("POST", &format!("{TTB_API}/tt_user/oauth2/token/"), &[], Body::Json(&serde_json::json!({ "client_id": c.id, "client_secret": c.secret, "grant_type": "authorization_code", "auth_code": code, "redirect_uri": redirect })))?;
        let d = ttb_data(&r.json()?).context("trocar o código pelo token")?;
        // a business-api devolve o token dentro de data; reempacota no formato que parse_token lê
        net::Resp { status: 200, headers: Default::default(), body: serde_json::to_vec(&d)? }
    } else if p == "yt" {
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
    let mut t = parse_token(p, &r.json()?, None)?;
    let (nome, id) = whoami(p, &t);
    t.nome = nome.clone();
    t.conta_id = id.clone();
    // mesma conta já autorizada com outro apelido? reaproveita o apelido (re-login atualiza o token)
    let existente = (!id.is_empty()).then(|| accounts(p).into_iter().find(|c| load(p, c).map(|o| o.conta_id == id).unwrap_or(false))).flatten();
    let apelido = match (conta.map(slug), existente) {
        (Some(a), Some(e)) if a != e => {
            eprintln!("aviso: essa conta já estava salva como \"{e}\"; renomeando para \"{a}\"");
            let _ = std::fs::remove_file(token_path(p, &e));
            a
        }
        (Some(a), _) => a,
        (None, Some(e)) => e,
        (None, None) => slug(if nome.is_empty() { "principal" } else { nome.trim_start_matches('@') }),
    };
    save(p, &apelido, &t)?;
    if default_account(p).is_none() {
        set_default(p, &apelido)?;
    }
    println!(
        "{}: conta \"{apelido}\" autorizada{} -> {}{}",
        label(p),
        if nome.is_empty() { String::new() } else { format!(" ({nome})") },
        token_path(p, &apelido).display(),
        if default_account(p).as_deref() == Some(apelido.as_str()) { " · padrão" } else { "" }
    );
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
        } else if let Some(code) = get("code").or_else(|| get("auth_code")) {
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
