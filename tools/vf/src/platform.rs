//! `vf publish` (sempre rascunho) e `vf analytics` no YouTube e no TikTok, com os tokens do `vf auth`.
//!   YouTube: upload resumível com privacyStatus=private (a API não tem "rascunho"; privado é o equivalente:
//!            você revisa e publica no Studio). Analíticos pela YouTube Analytics API (inclui retenção).
//!   TikTok:  Content Posting API, upload para a caixa de entrada (inbox): o vídeo chega como rascunho no app,
//!            onde legenda, som e postagem são finalizados. Analíticos pela Display API (video.list).
use crate::auth;
use crate::kv::{self, Kv};
use crate::net::{self, Body};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

fn bearer(t: &auth::Token) -> String {
    format!("Bearer {}", t.access_token)
}

fn today() -> String {
    std::process::Command::new("date").arg("+%F").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

fn days_ago(n: u32) -> String {
    std::process::Command::new("date").args(["-v", &format!("-{n}d"), "+%F"]).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .or_else(|| std::process::Command::new("date").args(["-d", &format!("{n} days ago"), "+%F"]).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()))
        .unwrap_or_default()
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

// ---------------------------------------------------------------- publish

pub fn publish(args: &[String]) -> Result<()> {
    let (file, rest) = match args.first() {
        Some(f) if !f.contains('=') && !f.starts_with("--") => (PathBuf::from(f), &args[1..]),
        _ => bail!("uso: vf publish <video.mp4> platform=yt|tiktok [title=... description=... tags=a,b meta=out/publicacao.json thumb=capa.jpg]"),
    };
    let kv = Kv::parse(rest, &[])?;
    kv.check_known(&["platform", "platforms", "title", "description", "tags", "category", "meta", "thumb", "ia", "o", "out"])?;
    if !file.is_file() {
        bail!("{} não existe", file.display());
    }
    let plats = kv::platforms(&kv)?;
    if plats.is_empty() {
        bail!("diga a plataforma: platform=yt ou platform=tiktok");
    }
    // metadados: meta=<json {title, description, tags}> e/ou chaves avulsas (as avulsas vencem)
    let meta: Value = match kv.get("meta") {
        Some(m) => serde_json::from_slice(&std::fs::read(m).with_context(|| format!("ler {m}"))?)?,
        None => json!({}),
    };
    let ms = |k: &str| kv.get(k).map(String::from).or_else(|| meta.get(k).and_then(|v| v.as_str()).map(String::from));
    let title = ms("title").or_else(|| file.file_stem().map(|s| s.to_string_lossy().into_owned())).unwrap();
    let description = ms("description").unwrap_or_default();
    let tags: Vec<String> = kv.get("tags").map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .or_else(|| meta.get("tags").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()))
        .unwrap_or_default();
    let ia = kv.get("ia").map(|v| !matches!(v, "0" | "false" | "nao" | "não")).unwrap_or(true);
    let dir = out_dir(&kv, "publicacao")?;

    for p in plats {
        let res = if p == "yt" {
            yt_upload(&file, &title, &description, &tags, kv.get("category").unwrap_or("22"), ia, kv.get("thumb").map(Path::new))?
        } else {
            tt_upload(&file)?
        };
        let path = dir.join(format!("{p}-{}.json", today()));
        crate::util::write_json(&path, &res)?;
        println!("{}", serde_json::to_string_pretty(&res)?);
        eprintln!("registrado em {} — anote em factory.yaml > publicacao.enviados", path.display());
    }
    Ok(())
}

fn yt_upload(file: &Path, title: &str, description: &str, tags: &[String], category: &str, ia: bool, thumb: Option<&Path>) -> Result<Value> {
    let t = auth::token("yt")?;
    let size = std::fs::metadata(file)?.len().to_string();
    let body = json!({
        "snippet": { "title": title.chars().take(100).collect::<String>(), "description": description, "tags": tags, "categoryId": category },
        "status": { "privacyStatus": "private", "selfDeclaredMadeForKids": false, "containsSyntheticMedia": ia }
    });
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
    eprintln!("YouTube: enviando {} ({} MB) como PRIVADO…", file.display(), std::fs::metadata(file)?.len() / 1_000_000);
    let v = net::request("PUT", &loc, &[("Authorization", &b), ("Content-Type", "video/mp4")], Body::File(file))?.ok().context("YouTube: upload")?.json()?;
    let id = v.get("id").and_then(|x| x.as_str()).context("YouTube: upload sem id")?.to_string();
    if let Some(th) = thumb {
        let url = format!("https://www.googleapis.com/upload/youtube/v3/thumbnails/set?videoId={}", net::enc(&id));
        let ct = if th.extension().map(|e| e == "png").unwrap_or(false) { "image/png" } else { "image/jpeg" };
        if let Err(e) = net::request("POST", &url, &[("Authorization", &b), ("Content-Type", ct)], Body::File(th)).and_then(|r| r.ok()) {
            eprintln!("aviso: thumbnail não enviada ({e:#}); contas não verificadas não podem usar thumbnail personalizada");
        }
    }
    Ok(json!({
        "plataforma": "yt", "id": id, "estado": "privado (rascunho)",
        "url": format!("https://youtu.be/{id}"), "studio": format!("https://studio.youtube.com/video/{id}/edit"),
        "titulo": title, "arquivo": file.display().to_string(), "data": today()
    }))
}

fn tt_upload(file: &Path) -> Result<Value> {
    let t = auth::token("tiktok")?;
    let b = bearer(&t);
    let size = std::fs::metadata(file)?.len();
    // regras da API: arquivo ≤ 64 MB vai inteiro; acima, pedaços de 10 MB e o último absorve o resto
    const MB64: u64 = 64 * 1024 * 1024;
    let (chunk, count) = if size <= MB64 { (size, 1) } else { (10_000_000u64, size / 10_000_000) };
    let init = net::request(
        "POST",
        "https://open.tiktokapis.com/v2/post/publish/inbox/video/init/",
        &[("Authorization", &b)],
        Body::Json(&json!({ "source_info": { "source": "FILE_UPLOAD", "video_size": size, "chunk_size": chunk, "total_chunk_count": count } })),
    )?
    .json()?;
    tt_check(&init)?;
    let upload = init.pointer("/data/upload_url").and_then(|x| x.as_str()).context("TikTok: sem upload_url")?.to_string();
    let publish_id = init.pointer("/data/publish_id").and_then(|x| x.as_str()).context("TikTok: sem publish_id")?.to_string();
    eprintln!("TikTok: enviando {} ({} MB, {count} parte(s)) para a caixa de entrada…", file.display(), size / 1_000_000);
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
        let r = net::request("PUT", &upload, &[("Content-Type", "video/mp4"), ("Content-Range", &range)], Body::File(&tmp));
        let r = r.and_then(|r| r.ok());
        if let Err(e) = r {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.context(format!("TikTok: parte {}/{count}", i + 1)));
        }
    }
    let _ = std::fs::remove_file(&tmp);
    let st = net::request("POST", "https://open.tiktokapis.com/v2/post/publish/status/fetch/", &[("Authorization", &b)], Body::Json(&json!({ "publish_id": publish_id })))
        .and_then(|r| r.json())
        .ok();
    Ok(json!({
        "plataforma": "tiktok", "publish_id": publish_id, "estado": "rascunho na caixa de entrada do app (finalize legenda e poste pelo TikTok)",
        "status_api": st.as_ref().and_then(|v| v.pointer("/data/status")).cloned().unwrap_or(Value::Null),
        "arquivo": file.display().to_string(), "data": today()
    }))
}

// ---------------------------------------------------------------- analytics

pub fn analytics(args: &[String]) -> Result<()> {
    let kv = Kv::parse(args, &[])?;
    kv.check_known(&["platform", "platforms", "video", "dias", "days", "o", "out"])?;
    let mut plats = kv::platforms(&kv)?;
    if plats.is_empty() {
        plats = vec!["yt", "tiktok"];
    }
    let dias: u32 = kv.get("dias").or(kv.get("days")).and_then(|d| d.parse().ok()).unwrap_or(28);
    // sem kv.list: ids do YouTube diferenciam maiúsculas
    let videos: Vec<String> = kv.get("video").map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()).unwrap_or_default();
    let dir = out_dir(&kv, "analytics")?;
    for p in plats {
        let v = if p == "yt" { yt_analytics(&videos, dias)? } else { tt_analytics(&videos)? };
        let path = dir.join(format!("{p}-{}.json", today()));
        crate::util::write_json(&path, &v)?;
        eprintln!("{p}: {}", path.display());
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

fn yt_analytics(videos: &[String], dias: u32) -> Result<Value> {
    let t = auth::token("yt")?;
    let b = bearer(&t);
    let (start, end) = (days_ago(dias), today());
    let metrics = "views,estimatedMinutesWatched,averageViewDuration,averageViewPercentage,likes,comments,shares,subscribersGained";
    let mut out = json!({ "plataforma": "yt", "periodo": { "inicio": start, "fim": end } });
    if videos.is_empty() {
        let top = rows(&yt_report(&b, &[("ids", "channel==MINE"), ("startDate", &start), ("endDate", &end), ("metrics", metrics), ("dimensions", "video"), ("sort", "-views"), ("maxResults", "25")])?);
        let ids: Vec<String> = top.iter().filter_map(|r| r.get("video").and_then(|v| v.as_str()).map(String::from)).collect();
        let titles = yt_titles(&b, &ids)?;
        println!("{:<12} {:>9} {:>7} {:>6}  título", "vídeo", "views", "média s", "% vista");
        for r in &top {
            let id = r.get("video").and_then(|v| v.as_str()).unwrap_or("");
            println!(
                "{:<12} {:>9} {:>7} {:>6.1}  {}",
                id,
                r.get("views").unwrap_or(&Value::Null),
                r.get("averageViewDuration").unwrap_or(&Value::Null),
                r.get("averageViewPercentage").and_then(|x| x.as_f64()).unwrap_or(0.0),
                titles.get(id).and_then(|x| x.as_str()).unwrap_or("")
            );
        }
        out["videos"] = Value::Array(top);
        out["titulos"] = titles;
    } else {
        let mut list = Vec::new();
        for id in videos {
            let f = format!("video=={id}");
            let total = rows(&yt_report(&b, &[("ids", "channel==MINE"), ("startDate", &start), ("endDate", &end), ("metrics", metrics), ("filters", &f)])?);
            let ret = rows(&yt_report(&b, &[("ids", "channel==MINE"), ("startDate", &start), ("endDate", &end), ("metrics", "audienceWatchRatio,relativeRetentionPerformance"), ("dimensions", "elapsedVideoTimeRatio"), ("filters", &f)])?);
            let at = |x: f64| ret.iter().min_by(|a, b| {
                let d = |r: &Value| (r.get("elapsedVideoTimeRatio").and_then(|v| v.as_f64()).unwrap_or(0.0) - x).abs();
                d(a).partial_cmp(&d(b)).unwrap()
            }).and_then(|r| r.get("audienceWatchRatio")).and_then(|v| v.as_f64());
            let s = total.first().cloned().unwrap_or(Value::Null);
            println!(
                "{id}: views {} · média {} s · {}% vista · retenção 10%/25%/50%/100%: {}",
                s.get("views").unwrap_or(&Value::Null),
                s.get("averageViewDuration").unwrap_or(&Value::Null),
                s.get("averageViewPercentage").unwrap_or(&Value::Null),
                [0.1, 0.25, 0.5, 1.0].iter().map(|&x| at(x).map(|v| format!("{:.0}%", v * 100.0)).unwrap_or("-".into())).collect::<Vec<_>>().join(" / ")
            );
            list.push(json!({ "id": id, "total": s, "retencao": ret }));
        }
        out["videos"] = Value::Array(list);
        out["titulos"] = yt_titles(&b, videos)?;
    }
    Ok(out)
}

fn yt_titles(b: &str, ids: &[String]) -> Result<Value> {
    let mut m = serde_json::Map::new();
    for chunk in ids.chunks(50) {
        let url = format!("https://www.googleapis.com/youtube/v3/videos?{}", net::query(&[("part", "snippet"), ("id", &chunk.join(","))]));
        let v = net::get(&url, &[("Authorization", b)])?.ok()?.json()?;
        for it in v.get("items").and_then(|i| i.as_array()).cloned().unwrap_or_default() {
            if let (Some(id), Some(t)) = (it.get("id").and_then(|x| x.as_str()), it.pointer("/snippet/title").and_then(|x| x.as_str())) {
                m.insert(id.into(), t.into());
            }
        }
    }
    Ok(Value::Object(m))
}

fn tt_analytics(videos: &[String]) -> Result<Value> {
    let t = auth::token("tiktok")?;
    let b = bearer(&t);
    let user = net::get("https://open.tiktokapis.com/v2/user/info/?fields=display_name,follower_count,likes_count,video_count", &[("Authorization", &b)])?.json()?;
    tt_check(&user)?;
    let fields = "id,title,video_description,create_time,duration,view_count,like_count,comment_count,share_count,share_url";
    let v = if videos.is_empty() {
        net::request("POST", &format!("https://open.tiktokapis.com/v2/video/list/?fields={fields}"), &[("Authorization", &b)], Body::Json(&json!({ "max_count": 20 })))?.json()?
    } else {
        net::request("POST", &format!("https://open.tiktokapis.com/v2/video/query/?fields={fields}"), &[("Authorization", &b)], Body::Json(&json!({ "filters": { "video_ids": videos } })))?.json()?
    };
    tt_check(&v)?;
    let vids = v.pointer("/data/videos").cloned().unwrap_or(json!([]));
    println!("{:<20} {:>9} {:>7} {:>6} {:>6}  título", "vídeo", "views", "likes", "coment", "shares");
    for x in vids.as_array().cloned().unwrap_or_default() {
        let g = |k: &str| x.get(k).cloned().unwrap_or(Value::Null);
        println!("{:<20} {:>9} {:>7} {:>6} {:>6}  {}", g("id").as_str().unwrap_or(""), g("view_count"), g("like_count"), g("comment_count"), g("share_count"), g("title").as_str().unwrap_or(""));
    }
    Ok(json!({ "plataforma": "tiktok", "conta": user.get("data").cloned().unwrap_or(Value::Null), "videos": vids }))
}
