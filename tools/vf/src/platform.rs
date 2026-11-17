//! `vf publish`, `vf meta` e `vf analytics` no YouTube e no TikTok, com os tokens do `vf auth`.
//! Várias contas: `account=<apelido>` (lista com vírgula; `account=all` no analytics); sem account vale
//! `factory.yaml > contas.<plataforma>` do projeto, depois a conta padrão (`vf auth use`).
//!
//! publish: `mode=` rascunho (padrão) | privado | nao-listado | amigos | publico, `at=` agenda.
//!   YouTube: upload resumível. rascunho/privado = private; at= usa o publishAt nativo (sobe agora como
//!            privado e o YouTube publica sozinho na hora). Longo e Short são o mesmo endpoint; o formato
//!            (detectado pelo arquivo ou `formato=`) só muda a checagem de metadata.
//!   TikTok:  rascunho = caixa de entrada do app (inbox, a legenda é feita no app). Os outros modos usam o
//!            Direct Post (escopo video.publish: `vf auth platform=tiktok direct=1`). A API não agenda:
//!            com at= o pedido vai para a agenda local (`vf agenda`), que posta na hora.
//! analytics: Data API (canal, uploads, metadata, views em tempo real) + Analytics API (período, retenção,
//!   tráfego, Shorts × longos, curva por idade) + comentários. O JSON alimenta a skill `metricas`.
use crate::auth;
use crate::kv::{self, Kv};
use crate::net::{self, Body};
use crate::tempo;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

fn bearer(t: &auth::Token) -> String {
    format!("Bearer {}", t.access_token)
}

/// data local de hoje (nome dos arquivos)
fn today() -> String {
    std::process::Command::new("date").arg("+%F").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_else(|| tempo::date(tempo::now()))
}

fn out_dir(kv: &Kv, sub: &str) -> Result<PathBuf> {
    let d = kv.get("o").or(kv.get("out")).map(PathBuf::from).unwrap_or_else(|| if Path::new("out").is_dir() { PathBuf::from("out").join(sub) } else { PathBuf::from(sub) });
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

fn tt_check(v: &Value) -> Result<()> {
    let code = v.pointer("/error/code").and_then(|c| c.as_str()).unwrap_or("ok");
    if code != "ok" {
        bail!("TikTok: {code}: {}", v.pointer("/error/message").and_then(|m| m.as_str()).unwrap_or(""));
    }
    Ok(())
}

/// contas pedidas: account=a,b (lista), account=all (todas; só analytics) ou nenhuma (resolve: projeto > padrão)
fn contas_de(kv: &Kv, p: &str, allow_all: bool) -> Vec<Option<String>> {
    let raw = kv.get("account").or(kv.get("conta"));
    match raw {
        Some("all" | "todas") if allow_all => auth::accounts(p).into_iter().map(Some).collect(),
        Some(list) => list.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| Some(s.to_string())).collect(),
        None => vec![None],
    }
}

fn factory_yaml() -> Option<serde_yaml::Value> {
    serde_yaml::from_str(&std::fs::read_to_string("factory.yaml").ok()?).ok()
}

/// plataformas do projeto atual (factory.yaml > plataformas), se houver
fn project_platforms() -> Option<Vec<&'static str>> {
    let y = factory_yaml()?;
    let v: Vec<&'static str> = y.get("plataformas")?.as_sequence()?.iter().filter_map(|x| x.as_str()).filter_map(|x| kv::platform(x).ok()).collect();
    (!v.is_empty()).then_some(v)
}

fn split_list(s: &str) -> Vec<String> {
    s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

// ---------------------------------------------------------------- publish

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Modo {
    Rascunho,
    Privado,
    NaoListado,
    Amigos,
    Publico,
}

impl Modo {
    fn parse(s: &str) -> Result<Modo> {
        Ok(match s.to_lowercase().replace(['_', ' '], "-").as_str() {
            "rascunho" | "draft" | "inbox" => Modo::Rascunho,
            "privado" | "private" | "self" | "so-eu" => Modo::Privado,
            "nao-listado" | "não-listado" | "unlisted" => Modo::NaoListado,
            "amigos" | "friends" => Modo::Amigos,
            "publico" | "público" | "public" | "postar" | "post" => Modo::Publico,
            x => bail!("mode={x}: use rascunho | privado | nao-listado | amigos | publico"),
        })
    }
    fn nome(self) -> &'static str {
        match self {
            Modo::Rascunho => "rascunho",
            Modo::Privado => "privado",
            Modo::NaoListado => "nao-listado",
            Modo::Amigos => "amigos",
            Modo::Publico => "publico",
        }
    }
}

const PUBLISH_KEYS: &[&str] = &[
    "platform", "platforms", "title", "description", "caption", "legenda", "tags", "category", "meta", "thumb", "ia", "o", "out", "account", "conta",
    "mode", "modo", "at", "agendar", "formato", "idioma", "lang", "comentarios", "duet", "stitch", "capa_ms", "kids",
];

/// metadados do envio: meta=<json> e/ou chaves avulsas (as avulsas vencem)
pub struct Meta {
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub caption: Option<String>,
    pub category: String,
    pub idioma: Option<String>,
}

fn meta_de(kv: &Kv, fallback_title: Option<String>) -> Result<Meta> {
    let meta: Value = match kv.get("meta") {
        Some(m) => serde_json::from_slice(&std::fs::read(m).with_context(|| format!("ler {m}"))?).with_context(|| format!("{m} não é JSON"))?,
        None => json!({}),
    };
    let ms = |k: &str| kv.get(k).map(String::from).or_else(|| meta.get(k).and_then(|v| v.as_str()).map(String::from));
    let tags: Vec<String> = kv.get("tags").map(split_list)
        .or_else(|| meta.get("tags").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.trim().to_string())).filter(|s| !s.is_empty()).collect()))
        .unwrap_or_default();
    let idioma = ms("idioma").or_else(|| ms("lang")).or_else(|| factory_yaml().and_then(|y| y.get("idioma")?.as_str().map(String::from)));
    Ok(Meta {
        title: ms("title").or(fallback_title).unwrap_or_default(),
        description: ms("description").unwrap_or_default(),
        tags,
        caption: ms("caption").or_else(|| ms("legenda")),
        category: ms("category").or_else(|| meta.get("category").and_then(|v| v.as_u64()).map(|n| n.to_string())).unwrap_or_else(|| "22".into()),
        idioma,
    })
}

/// formato do arquivo: `formato=` ou pelo vídeo (vertical e até 3 min = short)
fn formato_de(kv: &Kv, file: &Path) -> String {
    if let Some(f) = kv.get("formato") {
        return f.to_string();
    }
    match crate::probe::probe(file) {
        Ok(p) if p.height > p.width && p.duration <= 180.5 => "short".into(),
        Ok(_) => "yt-long".into(),
        Err(_) => "desconhecido".into(),
    }
}

/// Regras duras do YouTube (a API recusa) e avisos de boas práticas. Corrige o que é seguro corrigir.
pub fn yt_check_meta(m: &mut Meta, formato: &str) -> Result<Vec<String>> {
    let mut avisos = Vec::new();
    if m.title.contains(['<', '>']) || m.description.contains(['<', '>']) {
        m.title = m.title.replace(['<', '>'], "");
        m.description = m.description.replace(['<', '>'], "");
        avisos.push("< e > removidos do título/descrição (o YouTube recusa)".into());
    }
    let tl = m.title.chars().count();
    if m.title.trim().is_empty() {
        bail!("título vazio: passe title= ou meta=");
    }
    if tl > 100 {
        bail!("título com {tl} caracteres: o YouTube aceita até 100 (o ideal é que a ideia caiba nos primeiros ~60)");
    }
    if m.description.len() > 5000 {
        bail!("descrição com {} bytes: o YouTube aceita até 5000", m.description.len());
    }
    // tags: até 500 caracteres somando vírgulas, e tag com espaço conta as aspas
    let mut total = 0;
    let mut keep = Vec::new();
    for t in m.tags.drain(..) {
        let t = t.replace(['<', '>', ','], "");
        let c = t.chars().count() + if t.contains(' ') { 2 } else { 0 } + if keep.is_empty() { 0 } else { 1 };
        if total + c > 500 {
            avisos.push(format!("tag \"{t}\" e as seguintes cortadas: limite de 500 caracteres"));
            break;
        }
        total += c;
        keep.push(t);
    }
    m.tags = keep;
    let low = format!("{} {}", m.title, m.description).to_lowercase();
    let short = formato.contains("short");
    if short && !low.contains("#shorts") {
        m.description = format!("{}{}#Shorts", m.description.trim_end(), if m.description.trim().is_empty() { "" } else { "\n\n" });
    }
    if !short {
        if !(m.description.contains("0:00") || m.description.contains("00:00")) {
            avisos.push("vídeo longo sem capítulos na descrição (00:00 Abertura, ≥3 capítulos de ≥10 s)".into());
        }
        if tl > 70 {
            avisos.push(format!("título com {tl} caracteres: pode ser cortado na busca/sugeridos; ponha o essencial no começo"));
        }
    }
    if m.description.chars().take(150).count() < 50 && !short {
        avisos.push("descrição curta: as 2 primeiras linhas (~150 caracteres) aparecem na busca, use-as".into());
    }
    Ok(avisos)
}

pub fn publish(args: &[String]) -> Result<()> {
    publish_run(args, false).map(|_| ())
}

/// `da_agenda`: chamado pela `vf agenda run` (não reagenda e posta já).
pub fn publish_run(args: &[String], da_agenda: bool) -> Result<Vec<Value>> {
    let (file, rest) = match args.first() {
        Some(f) if !f.contains('=') && !f.starts_with("--") => (PathBuf::from(f), &args[1..]),
        _ => bail!("uso: vf publish <video.mp4> platform=yt|tiktok [mode=rascunho|privado|nao-listado|publico] [at=\"2026-11-20 18:00\"] [meta=out/publicacao.json title=... thumb=capa.jpg]"),
    };
    let kv = Kv::parse(rest, &[])?;
    kv.check_known(PUBLISH_KEYS)?;
    if !file.is_file() {
        bail!("{} não existe", file.display());
    }
    let plats = kv::platforms(&kv)?;
    if plats.is_empty() {
        bail!("diga a plataforma: platform=yt ou platform=tiktok (ou platform=yt,tiktok)");
    }
    let modo_explicito = kv.get("mode").or(kv.get("modo")).map(Modo::parse).transpose()?;
    let at = if da_agenda { None } else { kv.get("at").or(kv.get("agendar")).map(tempo::parse_when).transpose()? };
    let modo = match (modo_explicito, at) {
        (None, Some(_)) => Modo::Publico, // agendar = publicar na hora marcada
        (Some(Modo::Rascunho | Modo::Privado), Some(_)) => bail!("at= agenda a PUBLICAÇÃO; com mode=rascunho/privado não há o que agendar (tire o at= ou use mode=publico)"),
        (m, _) => m.unwrap_or(Modo::Rascunho),
    };
    if let Some(t) = at {
        if t < tempo::now() + 15 * 60 {
            bail!("at={}: precisa ser pelo menos 15 min no futuro (agora: {})", tempo::local(t), tempo::local(tempo::now()));
        }
        if plats.contains(&"yt") && modo != Modo::Publico {
            bail!("no YouTube, at= só agenda vídeo público (publishAt); para não-listado, suba e mude no Studio");
        }
    }
    let formato = formato_de(&kv, &file);
    let ia = kv.get("ia").map(|v| !matches!(v, "0" | "false" | "nao" | "não")).unwrap_or(true);
    let dir = out_dir(&kv, "publicacao")?;
    let fallback = file.file_stem().map(|s| s.to_string_lossy().into_owned());
    let mut resultados = Vec::new();

    for p in plats {
        let mut m = meta_de(&kv, fallback.clone())?;
        for acct in contas_de(&kv, p, false) {
            let a = acct.as_deref();
            let res = if p == "yt" {
                for w in yt_check_meta(&mut m, &formato)? {
                    eprintln!("aviso: {w}");
                }
                let privacy = match modo {
                    Modo::Rascunho | Modo::Privado | Modo::Amigos => "private",
                    Modo::NaoListado => "unlisted",
                    Modo::Publico if at.is_some() => "private",
                    Modo::Publico => "public",
                };
                if modo == Modo::Amigos {
                    eprintln!("aviso: o YouTube não tem modo amigos; enviado como privado");
                }
                yt_upload(a, &file, &m, &YtOpts { privacy, publish_at: at, ia, kids: kv.flag("kids"), thumb: kv.get("thumb").map(Path::new), modo, formato: &formato })?
            } else if let (Some(t), false) = (at, da_agenda) {
                // TikTok não agenda pela API: a agenda local posta na hora
                let mut job: Vec<String> = vec![std::fs::canonicalize(&file)?.display().to_string()];
                for (k, v) in kv.pairs() {
                    if !matches!(k.as_str(), "at" | "agendar" | "platform" | "platforms" | "account" | "conta" | "mode" | "modo") {
                        job.push(format!("{k}={v}"));
                    }
                }
                job.push("platform=tiktok".into());
                job.push(format!("mode={}", modo.nome()));
                if let Some(c) = a {
                    job.push(format!("account={c}"));
                }
                let (conta, tk) = auth::token("tiktok", a)?;
                // falhar agora, e não na hora do post
                if !auth::has_scope(&tk, auth::TT_DIRECT_SCOPE) {
                    bail!("TikTok/{conta}: agendar post exige o post direto (escopo video.publish): vf auth platform=tiktok account={conta} direct=1, com o Direct Post aprovado no app");
                }
                crate::agenda::enqueue(&job, t, "tiktok", &conta)?
            } else if modo == Modo::Rascunho {
                tt_inbox(a, &file)?
            } else {
                if modo == Modo::NaoListado {
                    bail!("o TikTok não tem não-listado: use privado (só você), amigos ou publico");
                }
                let caption = m.caption.clone().unwrap_or_else(|| tt_caption(&m));
                let privacy = match modo {
                    Modo::Privado => "SELF_ONLY",
                    Modo::Amigos => "MUTUAL_FOLLOW_FRIENDS",
                    _ => "PUBLIC_TO_EVERYONE",
                };
                let flag_off = |k: &str| kv.get(k).map(|v| matches!(v, "0" | "false" | "nao" | "não")).unwrap_or(false);
                tt_direct(a, &file, &TtOpts {
                    caption: &caption,
                    privacy,
                    ia,
                    no_comment: flag_off("comentarios"),
                    no_duet: flag_off("duet"),
                    no_stitch: flag_off("stitch"),
                    cover_ms: kv.get("capa_ms").and_then(|x| x.parse().ok()),
                })?
            };
            let conta = res.get("conta").and_then(|c| c.as_str()).unwrap_or("conta");
            let path = dir.join(format!("{p}-{conta}-{}.json", today()));
            crate::util::write_json(&path, &res)?;
            println!("{}", serde_json::to_string_pretty(&res)?);
            eprintln!("registrado em {} — anote em factory.yaml > publicacao.enviados", path.display());
            resultados.push(res);
        }
    }
    Ok(resultados)
}

/// legenda do TikTok sem caption= explícita: título + hashtags das tags (no máx. 5)
fn tt_caption(m: &Meta) -> String {
    let tags: Vec<String> = m.tags.iter().take(5).map(|t| format!("#{}", t.split_whitespace().collect::<String>().trim_start_matches('#'))).collect();
    let base = m.title.trim();
    if tags.is_empty() { base.to_string() } else { format!("{base} {}", tags.join(" ")) }
}

struct YtOpts<'a> {
    privacy: &'a str,
    publish_at: Option<i64>,
    ia: bool,
    kids: bool,
    thumb: Option<&'a Path>,
    modo: Modo,
    formato: &'a str,
}

fn yt_snippet(m: &Meta) -> Value {
    let mut s = json!({ "title": m.title, "description": m.description, "tags": m.tags, "categoryId": m.category });
    if let Some(l) = &m.idioma {
        s["defaultLanguage"] = json!(l);
        s["defaultAudioLanguage"] = json!(l);
    }
    s
}

fn yt_upload(acct: Option<&str>, file: &Path, m: &Meta, o: &YtOpts) -> Result<Value> {
    let (conta, t) = auth::token("yt", acct)?;
    let size = std::fs::metadata(file)?.len().to_string();
    let mut status = json!({ "privacyStatus": o.privacy, "selfDeclaredMadeForKids": o.kids, "containsSyntheticMedia": o.ia });
    if let Some(at) = o.publish_at {
        status["publishAt"] = json!(tempo::rfc3339(at));
    }
    let body = json!({ "snippet": yt_snippet(m), "status": status });
    let b = bearer(&t);
    let init = net::request(
        "POST",
        "https://www.googleapis.com/upload/youtube/v3/videos?uploadType=resumable&part=snippet,status",
        &[("Authorization", &b), ("X-Upload-Content-Type", "video/mp4"), ("X-Upload-Content-Length", &size)],
        Body::Json(&body),
    )?
    .ok()
    .context("YouTube: iniciar upload")?;
    let loc = init.headers.get("location").context("YouTube não devolveu a URL de upload")?.clone();
    let estado = match (o.modo, o.publish_at) {
        (_, Some(at)) => format!("agendado: privado até {} (o YouTube publica sozinho)", tempo::local(at)),
        (Modo::Rascunho, _) => "privado (rascunho)".into(),
        (Modo::Publico, _) => "PÚBLICO".into(),
        (Modo::NaoListado, _) => "não listado".into(),
        _ => "privado".into(),
    };
    eprintln!("YouTube: enviando {} ({} MB, {}) como {estado}…", file.display(), std::fs::metadata(file)?.len() / 1_000_000, o.formato);
    let v = net::request("PUT", &loc, &[("Authorization", &b), ("Content-Type", "video/mp4")], Body::File(file))?.ok().context("YouTube: upload")?.json()?;
    let id = v.get("id").and_then(|x| x.as_str()).context("YouTube: upload sem id")?.to_string();
    // projeto de API não verificado: o YouTube trava o vídeo como privado mesmo pedindo público
    let final_privacy = v.pointer("/status/privacyStatus").and_then(|x| x.as_str()).unwrap_or(o.privacy).to_string();
    if final_privacy != o.privacy {
        eprintln!("aviso: pedido {} mas o YouTube deixou {final_privacy}: projeto do Google Cloud não verificado só envia privado (auditoria em https://support.google.com/youtube/contact/yt_api_form)", o.privacy);
    }
    let mut thumb_ok = Value::Null;
    if let Some(th) = o.thumb {
        thumb_ok = json!(yt_thumb(&b, &id, th));
    }
    Ok(json!({
        "plataforma": "yt", "conta": conta, "canal": t.nome, "id": id, "modo": o.modo.nome(), "estado": estado, "privacidade": final_privacy,
        "agendado_para": o.publish_at.map(tempo::rfc3339), "formato": o.formato,
        "url": format!("https://youtu.be/{id}"), "studio": format!("https://studio.youtube.com/video/{id}/edit"),
        "titulo": m.title, "thumb": thumb_ok, "arquivo": file.display().to_string(), "data": today()
    }))
}

fn yt_thumb(b: &str, id: &str, th: &Path) -> bool {
    let url = format!("https://www.googleapis.com/upload/youtube/v3/thumbnails/set?videoId={}", net::enc(id));
    let ct = if th.extension().map(|e| e == "png").unwrap_or(false) { "image/png" } else { "image/jpeg" };
    match net::request("POST", &url, &[("Authorization", b), ("Content-Type", ct)], Body::File(th)).and_then(|r| r.ok()) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("aviso: thumbnail não enviada ({e:#}); contas não verificadas por telefone não podem usar thumbnail personalizada");
            false
        }
    }
}

/// regras da API: arquivo ≤ 64 MB vai inteiro; acima, pedaços de 10 MB e o último absorve o resto
fn tt_chunks(size: u64) -> (u64, u64) {
    const MB64: u64 = 64 * 1024 * 1024;
    if size <= MB64 { (size, 1) } else { (10_000_000u64, size / 10_000_000) }
}

fn tt_send(upload: &str, file: &Path, size: u64) -> Result<()> {
    let (chunk, count) = tt_chunks(size);
    let tmp = std::env::temp_dir().join(format!("vf-tt-{}.part", std::process::id()));
    let mut f = std::fs::File::open(file)?;
    for i in 0..count {
        let a = i * chunk;
        let bnd = if i + 1 == count { size } else { a + chunk };
        let mut buf = vec![0u8; (bnd - a) as usize];
        f.seek(SeekFrom::Start(a))?;
        f.read_exact(&mut buf)?;
        std::fs::write(&tmp, &buf)?;
        let range = format!("bytes {}-{}/{}", a, bnd - 1, size);
        let r = net::request("PUT", upload, &[("Content-Type", "video/mp4"), ("Content-Range", &range)], Body::File(&tmp)).and_then(|r| r.ok());
        if let Err(e) = r {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.context(format!("TikTok: parte {}/{count}", i + 1)));
        }
    }
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

fn tt_source(size: u64) -> Value {
    let (chunk, count) = tt_chunks(size);
    json!({ "source": "FILE_UPLOAD", "video_size": size, "chunk_size": chunk, "total_chunk_count": count })
}

fn tt_status(b: &str, publish_id: &str) -> Option<Value> {
    net::request("POST", "https://open.tiktokapis.com/v2/post/publish/status/fetch/", &[("Authorization", b)], Body::Json(&json!({ "publish_id": publish_id })))
        .and_then(|r| r.json())
        .ok()
        .and_then(|v| v.get("data").cloned())
}

fn tt_inbox(acct: Option<&str>, file: &Path) -> Result<Value> {
    let (conta, t) = auth::token("tiktok", acct)?;
    let b = bearer(&t);
    let size = std::fs::metadata(file)?.len();
    let init = net::request("POST", "https://open.tiktokapis.com/v2/post/publish/inbox/video/init/", &[("Authorization", &b)], Body::Json(&json!({ "source_info": tt_source(size) })))?.json()?;
    tt_check(&init)?;
    let upload = init.pointer("/data/upload_url").and_then(|x| x.as_str()).context("TikTok: sem upload_url")?.to_string();
    let publish_id = init.pointer("/data/publish_id").and_then(|x| x.as_str()).context("TikTok: sem publish_id")?.to_string();
    eprintln!("TikTok: enviando {} ({} MB) para a caixa de entrada…", file.display(), size / 1_000_000);
    tt_send(&upload, file, size)?;
    let st = tt_status(&b, &publish_id);
    Ok(json!({
        "plataforma": "tiktok", "conta": conta, "nome": t.nome, "publish_id": publish_id, "modo": "rascunho",
        "estado": "rascunho na caixa de entrada do app (finalize legenda e poste pelo TikTok)",
        "status_api": st.as_ref().and_then(|v| v.get("status")).cloned().unwrap_or(Value::Null),
        "arquivo": file.display().to_string(), "data": today()
    }))
}

struct TtOpts<'a> {
    caption: &'a str,
    privacy: &'a str,
    ia: bool,
    no_comment: bool,
    no_duet: bool,
    no_stitch: bool,
    cover_ms: Option<u64>,
}

/// Direct Post: publica direto no perfil (exige o escopo video.publish e, para público, app auditado).
fn tt_direct(acct: Option<&str>, file: &Path, o: &TtOpts) -> Result<Value> {
    let (conta, t) = auth::token("tiktok", acct)?;
    if !auth::has_scope(&t, auth::TT_DIRECT_SCOPE) {
        bail!(
            "TikTok/{conta}: post direto precisa do escopo video.publish. No developers.tiktok.com, adicione o Direct Post ao app \
             (Content Posting API → Direct Post) e rode: vf auth platform=tiktok account={conta} direct=1\n  \
             Sem isso, mode=rascunho continua funcionando (caixa de entrada do app)."
        );
    }
    let b = bearer(&t);
    let ci = net::request("POST", "https://open.tiktokapis.com/v2/post/publish/creator_info/query/", &[("Authorization", &b)], Body::Json(&json!({})))?.json()?;
    tt_check(&ci)?;
    let d = ci.get("data").cloned().unwrap_or(Value::Null);
    let opts: Vec<String> = d.get("privacy_level_options").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    if !opts.iter().any(|x| x == o.privacy) {
        bail!(
            "TikTok/{conta}: a conta/app não permite {} agora (permitido: {}). Apps não auditados só postam SELF_ONLY (privado): \
             use mode=privado, ou peça a auditoria do app no TikTok for Developers.",
            o.privacy,
            if opts.is_empty() { "nenhum".into() } else { opts.join(", ") }
        );
    }
    let size = std::fs::metadata(file)?.len();
    if let (Some(max), Ok(p)) = (d.get("max_video_post_duration_sec").and_then(|x| x.as_f64()), crate::probe::probe(file)) {
        if p.duration > max {
            bail!("TikTok/{conta}: vídeo de {:.0} s e esta conta posta até {max:.0} s", p.duration);
        }
    }
    if o.caption.encode_utf16().count() > 2200 {
        bail!("legenda do TikTok com mais de 2200 caracteres");
    }
    let bool_or = |k: &str, mine: bool| mine || d.get(k).and_then(|x| x.as_bool()).unwrap_or(false);
    let mut post = json!({
        "title": o.caption, "privacy_level": o.privacy,
        "disable_comment": bool_or("comment_disabled", o.no_comment),
        "disable_duet": bool_or("duet_disabled", o.no_duet),
        "disable_stitch": bool_or("stitch_disabled", o.no_stitch),
        "brand_content_toggle": false, "brand_organic_toggle": false, "is_aigc": o.ia
    });
    if let Some(ms) = o.cover_ms {
        post["video_cover_timestamp_ms"] = json!(ms);
    }
    let init = net::request("POST", "https://open.tiktokapis.com/v2/post/publish/video/init/", &[("Authorization", &b)], Body::Json(&json!({ "post_info": post, "source_info": tt_source(size) })))?.json()?;
    tt_check(&init)?;
    let upload = init.pointer("/data/upload_url").and_then(|x| x.as_str()).context("TikTok: sem upload_url")?.to_string();
    let publish_id = init.pointer("/data/publish_id").and_then(|x| x.as_str()).context("TikTok: sem publish_id")?.to_string();
    eprintln!("TikTok: postando {} ({} MB) como {}…", file.display(), size / 1_000_000, o.privacy);
    tt_send(&upload, file, size)?;
    // o TikTok processa depois do upload; espera até ~3 min pelo resultado
    let mut st = Value::Null;
    for _ in 0..36 {
        std::thread::sleep(std::time::Duration::from_secs(5));
        if let Some(s) = tt_status(&b, &publish_id) {
            let fim = matches!(s.get("status").and_then(|x| x.as_str()), Some("PUBLISH_COMPLETE" | "FAILED"));
            st = s;
            if fim {
                break;
            }
        }
    }
    let status = st.get("status").and_then(|x| x.as_str()).unwrap_or("PROCESSANDO").to_string();
    if status == "FAILED" {
        bail!("TikTok recusou o post: {}", st.get("fail_reason").and_then(|x| x.as_str()).unwrap_or("sem motivo"));
    }
    let ids = st.get("publicaly_available_post_id").cloned().unwrap_or(Value::Null);
    Ok(json!({
        "plataforma": "tiktok", "conta": conta, "nome": t.nome, "publish_id": publish_id, "modo": "direto", "privacidade": o.privacy,
        "estado": if status == "PUBLISH_COMPLETE" { "postado".to_string() } else { format!("{status} (confira no app)") },
        "post_ids": ids, "legenda": o.caption, "arquivo": file.display().to_string(), "data": today()
    }))
}

// ---------------------------------------------------------------- meta (corrigir metadata de vídeo já enviado)

/// `vf meta <id> platform=yt meta=out/meta_corrigida.json [title= description= tags= category=] [aplicar=1]`
/// Sem aplicar=1 só mostra o antes × depois. TikTok não permite editar legenda pela API.
pub fn meta(args: &[String]) -> Result<()> {
    let (id, rest) = match args.first() {
        Some(f) if !f.contains('=') && !f.starts_with("--") => (f.clone(), &args[1..]),
        _ => bail!("uso: vf meta <ID do vídeo> platform=yt meta=<json {{title, description, tags}}> [aplicar=1] [thumb=capa.jpg]"),
    };
    let kv = Kv::parse(rest, &["aplicar", "apply"])?;
    kv.check_known(&["platform", "platforms", "meta", "title", "description", "tags", "category", "idioma", "lang", "thumb", "aplicar", "apply", "account", "conta", "formato"])?;
    let plats = kv::platforms(&kv)?;
    if plats.iter().any(|p| *p == "tiktok") {
        bail!("o TikTok não deixa editar a legenda pela API: aplique a sugestão no app (Editar post)");
    }
    let acct = kv.get("account").or(kv.get("conta"));
    let (conta, t) = auth::token("yt", acct)?;
    let b = bearer(&t);
    let url = format!("https://www.googleapis.com/youtube/v3/videos?{}", net::query(&[("part", "snippet,contentDetails"), ("id", &id)]));
    let cur = net::get(&url, &[("Authorization", &b)])?.ok().context("YouTube Data API: ler o vídeo")?.json()?;
    let it = cur.pointer("/items/0").with_context(|| format!("vídeo {id} não encontrado nesta conta ({conta})"))?;
    let sn = it.get("snippet").cloned().unwrap_or(Value::Null);
    let s = |k: &str| sn.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let antes = Meta {
        title: s("title"),
        description: s("description"),
        tags: sn.get("tags").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(),
        caption: None,
        category: s("categoryId"),
        idioma: sn.get("defaultLanguage").and_then(|x| x.as_str()).map(String::from),
    };
    // só muda o que foi pedido (meta= e/ou chaves avulsas); o resto fica como está no YouTube
    let mj: Value = match kv.get("meta") {
        Some(m) => serde_json::from_slice(&std::fs::read(m).with_context(|| format!("ler {m}"))?).with_context(|| format!("{m} não é JSON"))?,
        None => json!({}),
    };
    let pick = |k: &str| kv.get(k).map(String::from).or_else(|| mj.get(k).and_then(|v| v.as_str().map(String::from).or_else(|| v.as_u64().map(|n| n.to_string()))));
    let tags_novas = kv.get("tags").map(split_list).or_else(|| mj.get("tags").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()));
    let dur = tempo::iso_duration(it.pointer("/contentDetails/duration").and_then(|x| x.as_str()).unwrap_or(""));
    let formato = kv.get("formato").map(String::from).unwrap_or_else(|| if dur <= 180.0 { "short".into() } else { "yt-long".into() });
    let mut depois = Meta {
        title: pick("title").unwrap_or_else(|| antes.title.clone()),
        description: pick("description").unwrap_or_else(|| antes.description.clone()),
        tags: tags_novas.unwrap_or_else(|| antes.tags.clone()),
        caption: None,
        category: pick("category").unwrap_or_else(|| antes.category.clone()),
        idioma: pick("idioma").or_else(|| pick("lang")).or_else(|| antes.idioma.clone()),
    };
    for w in yt_check_meta(&mut depois, &formato)? {
        eprintln!("aviso: {w}");
    }
    let diff = |k: &str, a: &str, d: &str| {
        if a == d {
            println!("= {k}: (igual)");
        } else {
            println!("- {k}: {}\n+ {k}: {}", a.replace('\n', "⏎ "), d.replace('\n', "⏎ "));
        }
    };
    println!("== {id} · {conta} ({formato})");
    diff("título", &antes.title, &depois.title);
    diff("descrição", &antes.description, &depois.description);
    diff("tags", &antes.tags.join(", "), &depois.tags.join(", "));
    diff("categoria", &antes.category, &depois.category);
    diff("idioma", antes.idioma.as_deref().unwrap_or(""), depois.idioma.as_deref().unwrap_or(""));
    if !(kv.flag("aplicar") || kv.flag("apply")) {
        eprintln!("\nsó prévia: rode de novo com aplicar=1 para gravar no YouTube");
        return Ok(());
    }
    if !auth::has_scope(&t, "https://www.googleapis.com/auth/youtube.force-ssl") && !auth::has_scope(&t, "https://www.googleapis.com/auth/youtube") {
        bail!("a conta {conta} foi autorizada sem permissão de edição: rode vf auth platform=yt account={conta} de novo (o vf agora pede youtube.force-ssl)");
    }
    let body = json!({ "id": id, "snippet": yt_snippet(&depois) });
    net::request("PUT", "https://www.googleapis.com/youtube/v3/videos?part=snippet", &[("Authorization", &b)], Body::Json(&body))?.ok().context("YouTube: atualizar metadata")?;
    if let Some(th) = kv.get("thumb") {
        yt_thumb(&b, &id, Path::new(th));
    }
    let log = json!({ "id": id, "conta": conta, "data": today(), "antes": { "title": antes.title, "description": antes.description, "tags": antes.tags }, "depois": { "title": depois.title, "description": depois.description, "tags": depois.tags } });
    let dir = out_dir(&kv, "analytics")?;
    let path = dir.join(format!("meta-{id}-{}.json", today()));
    crate::util::write_json(&path, &log)?;
    println!("metadata atualizada no YouTube; antes × depois em {} (compare as métricas em 7-14 dias)", path.display());
    Ok(())
}

// ---------------------------------------------------------------- analytics

struct AOpts {
    videos: Vec<String>,
    dias: u32,
    max: usize,
    comentarios: bool,
    curva: bool,
    retencao: usize,
}

pub fn analytics(args: &[String]) -> Result<()> {
    let kv = Kv::parse(args, &["comentarios", "comments", "curva"])?;
    kv.check_known(&["platform", "platforms", "video", "dias", "days", "o", "out", "account", "conta", "max", "comentarios", "comments", "curva", "retencao"])?;
    let mut plats = kv::platforms(&kv)?;
    if plats.is_empty() {
        // sem platform=: as do projeto (factory.yaml > plataformas), senão as que têm conta autorizada
        plats = project_platforms().unwrap_or_else(|| vec!["yt", "tiktok"]);
        plats.retain(|p| !auth::accounts(p).is_empty());
        if plats.is_empty() {
            bail!("nenhuma conta autorizada: rode vf auth platform=yt (ou tiktok)");
        }
    }
    // sem kv.list: ids do YouTube diferenciam maiúsculas
    let videos = kv.get("video").map(split_list).unwrap_or_default();
    let o = AOpts {
        dias: kv.get("dias").or(kv.get("days")).and_then(|d| d.parse().ok()).unwrap_or(28),
        max: kv.get("max").and_then(|d| d.parse().ok()).unwrap_or(50),
        comentarios: kv.flag("comentarios") || kv.flag("comments"),
        curva: kv.flag("curva") || !videos.is_empty(),
        retencao: kv.get("retencao").and_then(|d| d.parse().ok()).unwrap_or(if videos.is_empty() { 5 } else { usize::MAX }),
        videos,
    };
    let dir = out_dir(&kv, "analytics")?;
    // uma conta/plataforma com problema não derruba as outras; o código de saída indica falha no fim
    let mut falhas = Vec::new();
    for p in plats {
        for acct in contas_de(&kv, p, true) {
            let a = acct.as_deref();
            let r = if p == "yt" { yt_analytics(a, &o) } else { tt_analytics(a, &o) };
            match r {
                Ok(v) => {
                    let conta = v.get("conta").and_then(|c| c.as_str()).unwrap_or("conta");
                    let path = dir.join(format!("{p}-{conta}-{}.json", today()));
                    crate::util::write_json(&path, &v)?;
                    for w in v.get("avisos").and_then(|x| x.as_array()).cloned().unwrap_or_default() {
                        eprintln!("  ! {}", w.as_str().unwrap_or(""));
                    }
                    eprintln!("{p}/{conta}: {}\n", path.display());
                }
                Err(e) => {
                    eprintln!("erro em {p}{}: {e:#}\n", a.map(|x| format!("/{x}")).unwrap_or_default());
                    falhas.push(p);
                }
            }
        }
    }
    if !falhas.is_empty() {
        bail!("analíticos incompletos ({} falha(s)); veja as mensagens acima", falhas.len());
    }
    Ok(())
}

fn yt_report(b: &str, params: &[(&str, &str)]) -> Result<Value> {
    let url = format!("https://youtubeanalytics.googleapis.com/v2/reports?{}", net::query(params));
    net::get(&url, &[("Authorization", b)])?.ok()?.json()
}

/// linhas de um relatório como objetos {coluna: valor}
fn rows(v: &Value) -> Vec<Value> {
    let cols: Vec<String> = v.get("columnHeaders").and_then(|c| c.as_array()).map(|a| a.iter().filter_map(|h| h.get("name").and_then(|n| n.as_str()).map(String::from)).collect()).unwrap_or_default();
    v.get("rows").and_then(|r| r.as_array()).map(|rs| rs.iter().filter_map(|r| r.as_array()).map(|r| Value::Object(cols.iter().cloned().zip(r.iter().cloned()).collect())).collect()).unwrap_or_default()
}

fn num(v: &Value, k: &str) -> f64 {
    match v.get(k) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    Some(crate::util::median(&mut v))
}

const YT_METRICS: &str = "views,engagedViews,estimatedMinutesWatched,averageViewDuration,averageViewPercentage,likes,comments,shares,subscribersGained";

fn yt_get(b: &str, path: &str, q: &[(&str, &str)]) -> Result<Value> {
    let url = format!("https://www.googleapis.com/youtube/v3/{path}?{}", net::query(q));
    net::get(&url, &[("Authorization", b)])?.ok()?.json()
}

/// detalhes (Data API) normalizados: metadata + estado + views em tempo real
fn yt_videos(b: &str, ids: &[String]) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for chunk in ids.chunks(50) {
        let v = yt_get(b, "videos", &[("part", "snippet,status,statistics,contentDetails"), ("id", &chunk.join(","))])?;
        for it in v.get("items").and_then(|i| i.as_array()).cloned().unwrap_or_default() {
            let g = |p: &str| it.pointer(p).cloned().unwrap_or(Value::Null);
            let dur = tempo::iso_duration(g("/contentDetails/duration").as_str().unwrap_or(""));
            let st = g("/statistics");
            out.push(json!({
                "id": g("/id"), "titulo": g("/snippet/title"), "descricao": g("/snippet/description"), "tags": g("/snippet/tags"),
                "categoria": g("/snippet/categoryId"), "idioma": g("/snippet/defaultLanguage"), "idioma_audio": g("/snippet/defaultAudioLanguage"),
                "publicado": g("/snippet/publishedAt"), "privacidade": g("/status/privacyStatus"), "agendado_para": g("/status/publishAt"),
                "duracao_s": dur, "tipo": if dur <= 180.0 { "short?" } else { "longo" },
                "thumb": it.pointer("/snippet/thumbnails/maxres/url").or(it.pointer("/snippet/thumbnails/high/url")).cloned().unwrap_or(Value::Null),
                "total": { "views": num(&st, "viewCount"), "likes": num(&st, "likeCount"), "comentarios": num(&st, "commentCount") },
                "url": format!("https://youtu.be/{}", g("/id").as_str().unwrap_or(""))
            }));
        }
    }
    Ok(out)
}

fn yt_analytics(acct: Option<&str>, o: &AOpts) -> Result<Value> {
    let (conta, t) = auth::token("yt", acct)?;
    let b = bearer(&t);
    let now = tempo::now();
    let (start, end) = (tempo::date(now - o.dias as i64 * 86_400), tempo::date(now));
    let mut avisos: Vec<String> = Vec::new();

    // 1. canal pela Data API: falha aqui = API desativada / conta sem canal (antes isso ficava mudo)
    let ch = yt_get(&b, "channels", &[("part", "snippet,statistics,contentDetails"), ("mine", "true")]).context("YouTube Data API: ler o canal")?;
    let it = ch.pointer("/items/0").cloned().context("essa conta Google não tem canal do YouTube (ou escolheu a conta errada no login): rode vf auth platform=yt account=<apelido>")?;
    let canal_nome = it.pointer("/snippet/title").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let uploads = it.pointer("/contentDetails/relatedPlaylists/uploads").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let st = it.get("statistics").cloned().unwrap_or(Value::Null);
    println!("== YouTube · {conta} ({canal_nome}) · {} inscritos · {} views · {} vídeos · {start} → {end}", num(&st, "subscriberCount"), num(&st, "viewCount"), num(&st, "videoCount"));

    // 2. Analytics do período (atraso de 24-72 h)
    let base = |extra: &[(&'static str, &str)]| -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = vec![("ids".into(), "channel==MINE".into()), ("startDate".into(), start.clone()), ("endDate".into(), end.clone())];
        v.extend(extra.iter().map(|(k, x)| (k.to_string(), x.to_string())));
        v
    };
    let rep = |extra: &[(&'static str, &str)]| -> Result<Vec<Value>> {
        let p = base(extra);
        let r: Vec<(&str, &str)> = p.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        Ok(rows(&yt_report(&b, &r)?))
    };
    let canal_periodo = rep(&[("metrics", &format!("{YT_METRICS},subscribersLost"))])?.into_iter().next().unwrap_or(Value::Null);
    let por_video = rep(&[("metrics", YT_METRICS), ("dimensions", "video"), ("sort", "-views"), ("maxResults", "200")])?;
    let shorts_ids: HashSet<String> = rep(&[("metrics", "views"), ("dimensions", "video"), ("filters", "creatorContentType==shorts"), ("sort", "-views"), ("maxResults", "200")])
        .map(|r| r.iter().filter_map(|x| x.get("video").and_then(|v| v.as_str()).map(String::from)).collect())
        .unwrap_or_default();
    let por_tipo = rep(&[("metrics", "views,engagedViews,estimatedMinutesWatched,averageViewPercentage,subscribersGained"), ("dimensions", "creatorContentType")]).unwrap_or_default();
    let trafego = rep(&[("metrics", "views,estimatedMinutesWatched"), ("dimensions", "insightTrafficSourceType"), ("sort", "-views")]).unwrap_or_default();
    if por_video.is_empty() {
        avisos.push(format!(
            "Analytics sem views de {start} a {end}: o YouTube Analytics atrasa 24-72 h, e vídeos privados/agendados não somam views. \
             As views em tempo real (Data API) estão em videos[].total."
        ));
    }

    // 3. quais vídeos: os pedidos; senão os últimos `max` uploads + os que tiveram views no período
    let mut ids: Vec<String> = o.videos.clone();
    if ids.is_empty() {
        let mut token = String::new();
        while ids.len() < o.max && !uploads.is_empty() {
            let mut q = vec![("part", "contentDetails"), ("playlistId", uploads.as_str()), ("maxResults", "50")];
            if !token.is_empty() {
                q.push(("pageToken", token.as_str()));
            }
            let v = yt_get(&b, "playlistItems", &q)?;
            ids.extend(v.get("items").and_then(|i| i.as_array()).cloned().unwrap_or_default().iter().filter_map(|x| x.pointer("/contentDetails/videoId").and_then(|v| v.as_str()).map(String::from)));
            match v.get("nextPageToken").and_then(|x| x.as_str()) {
                Some(n) => token = n.to_string(),
                None => break,
            }
        }
        ids.truncate(o.max);
        for r in &por_video {
            if let Some(id) = r.get("video").and_then(|v| v.as_str()) {
                if !ids.iter().any(|x| x == id) {
                    ids.push(id.to_string());
                }
            }
        }
    }
    let mut vids = yt_videos(&b, &ids)?;
    let periodo: BTreeMap<String, Value> = por_video.iter().filter_map(|r| Some((r.get("video")?.as_str()?.to_string(), r.clone()))).collect();
    for v in vids.iter_mut() {
        let id = v["id"].as_str().unwrap_or("").to_string();
        if shorts_ids.contains(&id) {
            v["tipo"] = json!("short");
        } else if periodo.contains_key(&id) && v["tipo"] == "short?" && !shorts_ids.is_empty() {
            v["tipo"] = json!("longo");
        }
        v["periodo"] = periodo.get(&id).cloned().unwrap_or(Value::Null);
    }
    // ordem: views no período, depois views totais
    vids.sort_by(|a, b| (num(&b["periodo"], "views"), num(&b["total"], "views")).partial_cmp(&(num(&a["periodo"], "views"), num(&a["total"], "views"))).unwrap_or(std::cmp::Ordering::Equal));

    // 4. retenção, tráfego por vídeo, curva por idade e comentários nos principais
    let n_ret = o.retencao.min(vids.len());
    let n_cmt = if o.videos.is_empty() { 5.min(vids.len()) } else { vids.len() };
    let n_curva = if o.curva { 25.min(vids.len()) } else { 0 };
    for (k, v) in vids.iter_mut().enumerate() {
        let id = v["id"].as_str().unwrap_or("").to_string();
        let publico = v["privacidade"] == "public" || v["privacidade"] == "unlisted";
        let f = format!("video=={id}");
        let tem_views = num(&v["periodo"], "views") > 0.0;
        if k < n_ret && tem_views {
            match rep(&[("metrics", "audienceWatchRatio,relativeRetentionPerformance"), ("dimensions", "elapsedVideoTimeRatio"), ("filters", &f)]) {
                Ok(ret) => {
                    let dur = v["duracao_s"].as_f64().unwrap_or(0.0);
                    let curva: Vec<Value> = ret.iter().map(|r| {
                        let x = num(r, "elapsedVideoTimeRatio");
                        json!({ "fracao": x, "s": (x * dur * 10.0).round() / 10.0, "assistindo": num(r, "audienceWatchRatio"), "relativa": num(r, "relativeRetentionPerformance") })
                    }).collect();
                    // maiores quedas entre pontos vizinhos (onde o roteiro perde gente)
                    let mut quedas: Vec<(f64, f64, f64)> = curva.windows(2).map(|w| (w[1]["s"].as_f64().unwrap_or(0.0), w[0]["assistindo"].as_f64().unwrap_or(0.0) - w[1]["assistindo"].as_f64().unwrap_or(0.0), w[1]["fracao"].as_f64().unwrap_or(0.0))).collect();
                    quedas.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                    let at = |x: f64| curva.iter().min_by(|a, b| (num(a, "fracao") - x).abs().partial_cmp(&(num(b, "fracao") - x).abs()).unwrap()).map(|r| num(r, "assistindo"));
                    v["retencao"] = json!({
                        "marcos": { "10%": at(0.1), "25%": at(0.25), "50%": at(0.5), "100%": at(1.0) },
                        "maiores_quedas": quedas.iter().take(5).filter(|q| q.1 > 0.0).map(|q| json!({ "s": q.0, "fracao": q.2, "queda": q.1 })).collect::<Vec<_>>(),
                        "curva": curva
                    });
                }
                Err(e) => avisos.push(format!("retenção de {id}: {e:#}")),
            }
            if !o.videos.is_empty() {
                v["trafego"] = json!(rep(&[("metrics", "views,estimatedMinutesWatched,averageViewDuration"), ("dimensions", "insightTrafficSourceType"), ("filters", &f), ("sort", "-views")]).unwrap_or_default());
            }
        }
        if k < n_curva && publico {
            if let Some(pubd) = v["publicado"].as_str().and_then(|s| tempo::parse_iso(s)).and_then(|r| r.ok()) {
                let fim = (pubd + 28 * 86_400).min(now);
                let (ini, fim) = (tempo::date(pubd), tempo::date(fim));
                let p: [(&str, &str); 7] = [("ids", "channel==MINE"), ("startDate", &ini), ("endDate", &fim), ("metrics", "views,engagedViews,averageViewPercentage"), ("dimensions", "day"), ("filters", &f), ("sort", "day")];
                if let Ok(r) = yt_report(&b, &p) {
                    let dias = rows(&r);
                    let mut acc = 0.0;
                    let mut idade = serde_json::Map::new();
                    let marcos = [1usize, 2, 3, 7, 14, 28];
                    for (i, d) in dias.iter().enumerate() {
                        acc += num(d, "views");
                        if marcos.contains(&(i + 1)) {
                            idade.insert(format!("d{}", i + 1), json!(acc));
                        }
                    }
                    v["idade"] = Value::Object(idade);
                    v["por_dia"] = json!(dias);
                }
            }
        }
        if o.comentarios && k < n_cmt && publico {
            match yt_get(&b, "commentThreads", &[("part", "snippet"), ("videoId", &id), ("maxResults", "100"), ("order", "relevance"), ("textFormat", "plainText")]) {
                // sem nome de autor: só o texto interessa para a análise (e não guarda dado pessoal)
                Ok(c) => {
                    v["comentarios"] = json!(c.get("items").and_then(|i| i.as_array()).cloned().unwrap_or_default().iter().map(|x| {
                        let s = x.pointer("/snippet/topLevelComment/snippet").cloned().unwrap_or(Value::Null);
                        json!({ "texto": s.get("textDisplay"), "likes": s.get("likeCount"), "respostas": x.pointer("/snippet/totalReplyCount"), "data": s.get("publishedAt") })
                    }).collect::<Vec<_>>())
                }
                Err(e) => avisos.push(format!("comentários de {id}: {e:#}")),
            }
        }
    }

    // 5. tabela e medianas (a skill compara cada vídeo com a mediana do PRÓPRIO canal)
    println!("{:<12} {:<6} {:<9} {:<10} {:>8} {:>8} {:>6} {:>6}  título", "vídeo", "tipo", "estado", "publicado", "views", "período", "méd s", "% vis");
    for v in &vids {
        let p = &v["periodo"];
        println!(
            "{:<12} {:<6} {:<9} {:<10} {:>8} {:>8} {:>6} {:>6}  {}",
            v["id"].as_str().unwrap_or(""),
            v["tipo"].as_str().unwrap_or(""),
            if v["agendado_para"].is_string() { "agendado" } else { v["privacidade"].as_str().unwrap_or("") },
            v["publicado"].as_str().unwrap_or("").get(..10).unwrap_or(""),
            num(&v["total"], "views"),
            if p.is_null() { "-".into() } else { num(p, "views").to_string() },
            if p.is_null() { "-".into() } else { format!("{:.0}", num(p, "averageViewDuration")) },
            if p.is_null() { "-".into() } else { format!("{:.0}", num(p, "averageViewPercentage")) },
            v["titulo"].as_str().unwrap_or("").chars().take(60).collect::<String>()
        );
    }
    let med = |tipo: &str, k: &str| median(vids.iter().filter(|v| v["tipo"].as_str().unwrap_or("").starts_with(tipo) && !v["periodo"].is_null()).map(|v| num(&v["periodo"], k)).collect());
    let mut medianas = json!({});
    for tipo in ["short", "longo"] {
        medianas[tipo] = json!({ "views": med(tipo, "views"), "averageViewDuration": med(tipo, "averageViewDuration"), "averageViewPercentage": med(tipo, "averageViewPercentage") });
    }
    for d in ["d1", "d3", "d7", "d28"] {
        for tipo in ["short", "longo"] {
            medianas[tipo][format!("views_{d}")] = json!(median(vids.iter().filter(|v| v["tipo"].as_str().unwrap_or("").starts_with(tipo)).filter_map(|v| v["idade"].get(d).and_then(|x| x.as_f64())).collect()));
        }
    }
    if num(&st, "viewCount") == 0.0 {
        avisos.push("o canal tem 0 views no total: nada a analisar ainda (vídeos privados/rascunho não contam)".into());
    }
    let privados = vids.iter().filter(|v| v["privacidade"] == "private" && !v["agendado_para"].is_string()).count();
    if privados > 0 {
        avisos.push(format!("{privados} vídeo(s) ainda privados (rascunho): publique no Studio ou com vf publish mode=publico"));
    }
    Ok(json!({
        "plataforma": "yt", "conta": conta, "canal": canal_nome, "canal_id": it.get("id"), "gerado": tempo::rfc3339(now),
        "periodo": { "inicio": start, "fim": end, "dias": o.dias },
        "canal_total": { "inscritos": num(&st, "subscriberCount"), "views": num(&st, "viewCount"), "videos": num(&st, "videoCount") },
        "canal_periodo": canal_periodo, "por_tipo": por_tipo, "trafego": trafego, "medianas": medianas,
        "videos": vids, "avisos": avisos
    }))
}

fn tt_analytics(acct: Option<&str>, o: &AOpts) -> Result<Value> {
    let (conta, t) = auth::token("tiktok", acct)?;
    let b = bearer(&t);
    let user = net::get("https://open.tiktokapis.com/v2/user/info/?fields=display_name,follower_count,following_count,likes_count,video_count", &[("Authorization", &b)])?.json()?;
    tt_check(&user)?;
    let u = user.pointer("/data/user").cloned().unwrap_or(Value::Null);
    println!("== TikTok · {conta} ({}) · {} seguidores · {} curtidas · {} vídeos", u["display_name"].as_str().unwrap_or(""), num(&u, "follower_count"), num(&u, "likes_count"), num(&u, "video_count"));
    let fields = "id,title,video_description,create_time,duration,cover_image_url,share_url,view_count,like_count,comment_count,share_count";
    let mut vids: Vec<Value> = Vec::new();
    if o.videos.is_empty() {
        // Display API: páginas de até 20; só posts públicos
        let mut cursor: Option<i64> = None;
        while vids.len() < o.max {
            let mut body = json!({ "max_count": 20 });
            if let Some(c) = cursor {
                body["cursor"] = json!(c);
            }
            let v = net::request("POST", &format!("https://open.tiktokapis.com/v2/video/list/?fields={fields}"), &[("Authorization", &b)], Body::Json(&body))?.json()?;
            tt_check(&v)?;
            vids.extend(v.pointer("/data/videos").and_then(|x| x.as_array()).cloned().unwrap_or_default());
            match (v.pointer("/data/has_more").and_then(|x| x.as_bool()), v.pointer("/data/cursor").and_then(|x| x.as_i64())) {
                (Some(true), Some(c)) => cursor = Some(c),
                _ => break,
            }
        }
        vids.truncate(o.max);
    } else {
        for chunk in o.videos.chunks(20) {
            let v = net::request("POST", &format!("https://open.tiktokapis.com/v2/video/query/?fields={fields}"), &[("Authorization", &b)], Body::Json(&json!({ "filters": { "video_ids": chunk } })))?.json()?;
            tt_check(&v)?;
            vids.extend(v.pointer("/data/videos").and_then(|x| x.as_array()).cloned().unwrap_or_default());
        }
    }
    let now = tempo::now();
    for v in vids.iter_mut() {
        let views = num(v, "view_count");
        let eng = num(v, "like_count") + num(v, "comment_count") + num(v, "share_count");
        v["engajamento"] = json!(if views > 0.0 { eng / views } else { 0.0 });
        v["compart_por_mil"] = json!(if views > 0.0 { num(v, "share_count") * 1000.0 / views } else { 0.0 });
        v["idade_dias"] = json!(((now - num(v, "create_time") as i64) as f64 / 86_400.0).max(0.0).round());
        v["publicado"] = json!(tempo::rfc3339(num(v, "create_time") as i64));
    }
    println!("{:<20} {:>5} {:>9} {:>7} {:>6} {:>6} {:>6}  título", "vídeo", "dias", "views", "likes", "coment", "shares", "eng%");
    for x in &vids {
        println!(
            "{:<20} {:>5} {:>9} {:>7} {:>6} {:>6} {:>6.1}  {}",
            x["id"].as_str().unwrap_or(""), num(x, "idade_dias"), num(x, "view_count"), num(x, "like_count"), num(x, "comment_count"), num(x, "share_count"),
            num(x, "engajamento") * 100.0, x["title"].as_str().unwrap_or("").chars().take(60).collect::<String>()
        );
    }
    let mut avisos = vec!["a API do TikTok não entrega retenção, fontes de tráfego nem comentários: exporte do TikTok Studio (Analytics → vídeo) se precisar".to_string()];
    if vids.is_empty() {
        avisos.push("nenhum post público encontrado: rascunhos da caixa de entrada e posts privados não aparecem na Display API".into());
    }
    let medianas = json!({
        "views": median(vids.iter().map(|v| num(v, "view_count")).collect()),
        "engajamento": median(vids.iter().map(|v| num(v, "engajamento")).collect()),
        "compart_por_mil": median(vids.iter().map(|v| num(v, "compart_por_mil")).collect()),
        "duracao": median(vids.iter().map(|v| num(v, "duration")).collect()),
    });
    Ok(json!({ "plataforma": "tiktok", "conta": conta, "gerado": tempo::rfc3339(now), "perfil": u, "medianas": medianas, "videos": vids, "avisos": avisos }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn m(title: &str, desc: &str, tags: &[&str]) -> Meta {
        Meta { title: title.into(), description: desc.into(), tags: tags.iter().map(|s| s.to_string()).collect(), caption: None, category: "22".into(), idioma: None }
    }
    #[test]
    fn modos() {
        assert_eq!(Modo::parse("público").unwrap(), Modo::Publico);
        assert_eq!(Modo::parse("unlisted").unwrap(), Modo::NaoListado);
        assert!(Modo::parse("xyz").is_err());
    }
    #[test]
    fn checa_meta() {
        let mut a = m("Por que <isso> acontece", "texto", &[]);
        let w = yt_check_meta(&mut a, "short").unwrap();
        assert_eq!(a.title, "Por que isso acontece");
        assert!(a.description.ends_with("#Shorts") && w.iter().any(|x| x.contains("removidos")));
        let mut b = m(&"x".repeat(101), "", &[]);
        assert!(yt_check_meta(&mut b, "yt-long").is_err());
        let long_tags: Vec<String> = (0..60).map(|i| format!("tag numero {i}")).collect();
        let mut c = Meta { tags: long_tags, ..m("ok", "00:00 a", &[]) };
        yt_check_meta(&mut c, "yt-long").unwrap();
        let total: usize = c.tags.iter().map(|t| t.len() + 2).sum::<usize>() + c.tags.len() - 1;
        assert!(total <= 500 && c.tags.len() < 60);
        let mut d = m("ok", "00:00 abertura", &[]);
        assert!(!yt_check_meta(&mut d, "yt-long").unwrap().iter().any(|x| x.contains("capítulos")));
    }
    #[test]
    fn legenda_tt() {
        let x = m("Título", "", &["história do brasil", "#curiosidades"]);
        assert_eq!(tt_caption(&x), "Título #históriadobrasil #curiosidades");
    }
    #[test]
    fn chunks() {
        assert_eq!(tt_chunks(1000), (1000, 1));
        assert_eq!(tt_chunks(100_000_000), (10_000_000, 10));
    }
}
