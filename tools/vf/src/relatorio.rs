//! `vf relatorio`: relatórios no estilo do YouTube Studio ("Desempenho inicial"), por vídeo e de todos os
//! vídeos com filtros, em Markdown + JSON + HTML (gráficos SVG sem dependências).
//!
//!   vf relatorio video=ID [janela=d7|d1|d3|d28|total|todas|AAAA-MM-DD..AAAA-MM-DD] [beats=roteiro/beats.json]
//!   vf relatorio [tipo=short|longo] [privacidade=public] [desde=AAAA-MM-DD] [ate=AAAA-MM-DD] [busca=texto]
//!                [tag=x] [min_views=N] [ordenar=views|intencionais|retencao|avd|pct|horas|inscritos|likes|comentarios|shares|engajamento|publicado]
//!                [ordem=desc|asc] [limite=N] [max=200] [janela=d7]
//!   platform=yt (padrão) | tiktok   account=<apelido>|all   html=0   o=<pasta>
//!
//! Janela "dN" = do dia da publicação até o N-ésimo dia (a Analytics API é diária, fuso do Pacífico); vídeo mais
//! novo que a janela sai como "parcial" e fica fora da mediana. A mediana compara com vídeos do MESMO tipo.
//! "Continuaram assistindo" (Shorts: assistiram × deslizaram) não existe na API pública: só no Studio.
//! TikTok: Display API (views, curtidas, comentários, compartilhamentos, duração) + TikTok Business API
//! (EXPERIMENTAL; tempo médio, retenção, até o fim, alcance) quando houver `vf auth platform=tiktok-biz`.
use crate::auth;
use crate::kv::{self, Kv};
use crate::net;
use crate::platform::{self as pf, median, num};
use crate::tempo;
use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------- janelas

#[derive(Clone, Debug, PartialEq)]
pub enum Janela {
    Dias(u32),
    Total,
    Faixa(String, String),
}

impl Janela {
    fn parse(s: &str) -> Result<Vec<Janela>> {
        let s = s.trim().to_lowercase();
        if s == "todas" || s == "all" {
            return Ok(vec![Janela::Dias(1), Janela::Dias(3), Janela::Dias(7), Janela::Dias(28), Janela::Total]);
        }
        if s == "total" || s == "vida" || s == "lifetime" {
            return Ok(vec![Janela::Total]);
        }
        if let Some((a, b)) = s.split_once("..") {
            for d in [a, b] {
                if tempo::parse_iso(&format!("{d}T00:00Z")).map(|r| r.is_err()).unwrap_or(true) {
                    bail!("janela={s}: use AAAA-MM-DD..AAAA-MM-DD");
                }
            }
            return Ok(vec![Janela::Faixa(a.to_string(), b.to_string())]);
        }
        let n: u32 = s.trim_start_matches('d').parse().with_context(|| format!("janela={s}: use d1, d3, d7, d28, total, todas ou AAAA-MM-DD..AAAA-MM-DD"))?;
        if n == 0 || n > 3650 {
            bail!("janela={s}: entre d1 e d3650");
        }
        Ok(vec![Janela::Dias(n)])
    }
    pub fn chave(&self) -> String {
        match self {
            Janela::Dias(n) => format!("d{n}"),
            Janela::Total => "total".into(),
            Janela::Faixa(a, b) => format!("{a}..{b}"),
        }
    }
    pub fn nome(&self) -> String {
        match self {
            Janela::Dias(1) => "primeiro dia".into(),
            Janela::Dias(n) => format!("{n} primeiros dias"),
            Janela::Total => "desde a publicação".into(),
            Janela::Faixa(a, b) => format!("{a} a {b}"),
        }
    }
    /// (início, fim, parcial: Some((dias com dado, dias da janela)))
    pub fn faixa(&self, publicado: i64, agora: i64) -> (String, String, Option<(i64, i64)>) {
        let dia0 = publicado - publicado.rem_euclid(86_400);
        match self {
            Janela::Dias(n) => {
                let fim = dia0 + (*n as i64 - 1) * 86_400;
                if fim > agora {
                    let tidos = (agora - dia0) / 86_400 + 1;
                    (tempo::date(dia0), tempo::date(agora), Some((tidos, *n as i64)))
                } else {
                    (tempo::date(dia0), tempo::date(fim), None)
                }
            }
            Janela::Total => (tempo::date(dia0), tempo::date(agora), None),
            Janela::Faixa(a, b) => (a.clone(), b.clone(), None),
        }
    }
}

// ---------------------------------------------------------------- formatação pt-BR

pub fn fmt_n(v: f64) -> String {
    let neg = v < 0.0;
    let s = format!("{:.0}", v.abs());
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    if neg { format!("−{out}") } else { out }
}

pub fn fmt_dec(v: f64, casas: usize) -> String {
    let s = format!("{:.*}", casas, v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let int = fmt_n(int.parse::<f64>().unwrap_or(0.0));
    let sinal = if v < 0.0 && v.abs() >= 0.5 * 10f64.powi(-(casas as i32)) { "−" } else { "" };
    if frac.is_empty() { format!("{sinal}{int}") } else { format!("{sinal}{int},{frac}") }
}

pub fn fmt_pct(v: f64) -> String {
    format!("{}%", fmt_dec(v, 2))
}

pub fn fmt_dur(s: f64) -> String {
    let s = s.round() as i64;
    if s < 60 {
        format!("{s} segundo{}", if s == 1 { "" } else { "s" })
    } else if s < 3600 {
        format!("{}:{:02}", s / 60, s % 60)
    } else {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    }
}

fn fmt_subs(v: f64) -> String {
    if v > 0.0 { format!("+{}", fmt_n(v)) } else { fmt_n(v) }
}

fn delta(v: f64, med: Option<f64>) -> String {
    match med {
        Some(m) if m.abs() > 1e-9 => {
            let d = (v - m) / m.abs() * 100.0;
            if d.abs() < 0.5 { "=".into() } else if d > 0.0 { format!("+{:.0}%", d) } else { format!("−{:.0}%", d.abs()) }
        }
        _ => "—".into(),
    }
}

#[derive(Clone, Copy)]
enum Fmt {
    Int,
    Pct,
    Dur,
    Horas,
    Subs,
    /// fração 0-1 (ou >1 com replays) mostrada como %
    Frac,
}

fn fmtv(v: Option<f64>, f: Fmt) -> String {
    let Some(v) = v else { return "—".into() };
    match f {
        Fmt::Int => fmt_n(v),
        Fmt::Pct => fmt_pct(v),
        Fmt::Dur => fmt_dur(v),
        Fmt::Horas => fmt_dec(v, 2),
        Fmt::Subs => fmt_subs(v),
        Fmt::Frac => fmt_pct(v * 100.0),
    }
}

fn esc_md(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// linhas da tabela "Desempenho" (chave no JSON, rótulo como no Studio, formato)
const YT_LINHAS: &[(&str, &str, Fmt)] = &[
    ("views", "Visualizações", Fmt::Int),
    ("impressoes", "Impressões", Fmt::Int),
    ("ctr", "Taxa de cliques das impressões", Fmt::Pct),
    ("intencionais", "Visualizações intencionais", Fmt::Int),
    ("continuaram", "Continuaram assistindo", Fmt::Pct),
    ("avd", "Duração média da visualização", Fmt::Dur),
    ("pct", "Porcentagem visualizada média", Fmt::Pct),
    ("retencao_media", "Retenção média (curva)", Fmt::Frac),
    ("horas", "Tempo de exibição (horas)", Fmt::Horas),
    ("likes", "Marcações com \"Gostei\"", Fmt::Int),
    ("inscritos", "Inscritos gerados", Fmt::Subs),
    ("comentarios", "Comentários adicionados", Fmt::Int),
    ("shares", "Compartilhamentos", Fmt::Int),
];

const TT_LINHAS: &[(&str, &str, Fmt)] = &[
    ("views", "Visualizações", Fmt::Int),
    ("alcance", "Espectadores alcançados", Fmt::Int),
    ("avd", "Tempo médio assistido", Fmt::Dur),
    ("retencao_media", "Retenção média (tempo médio ÷ duração)", Fmt::Frac),
    ("ate_o_fim", "Assistiram até o fim", Fmt::Pct),
    ("horas", "Tempo total assistido (horas)", Fmt::Horas),
    ("likes", "Curtidas", Fmt::Int),
    ("comentarios", "Comentários", Fmt::Int),
    ("shares", "Compartilhamentos", Fmt::Int),
    ("engajamento", "Engajamento ((curt.+coment.+compart.) ÷ views)", Fmt::Frac),
];

fn get(v: &Value, k: &str) -> Option<f64> {
    match v.get(k) {
        Some(Value::Number(n)) => n.as_f64(),
        _ => None,
    }
}

// ---------------------------------------------------------------- consultas YouTube

const METRICAS: &str = "views,engagedViews,estimatedMinutesWatched,averageViewDuration,averageViewPercentage,likes,comments,shares,subscribersGained,subscribersLost";

fn report(b: &str, inicio: &str, fim: &str, extra: &[(&str, &str)]) -> Result<Vec<Value>> {
    let mut p: Vec<(&str, &str)> = vec![("ids", "channel==MINE"), ("startDate", inicio), ("endDate", fim)];
    p.extend_from_slice(extra);
    Ok(pf::rows(&pf::yt_report(b, &p)?))
}

/// métricas de um vídeo numa faixa de datas, já com os nomes do relatório
fn yt_metricas(b: &str, id: &str, inicio: &str, fim: &str) -> Result<Value> {
    let f = format!("video=={id}");
    let Some(r) = report(b, inicio, fim, &[("metrics", METRICAS), ("filters", &f)])?.into_iter().next() else {
        // sem linha = a Analytics ainda não processou (atraso de 24-72 h), não "zero"
        return Ok(json!({ "sem_analytics": true, "continuaram": Value::Null, "inicio": inicio, "fim": fim }));
    };
    Ok(json!({
        "views": num(&r, "views"), "intencionais": num(&r, "engagedViews"), "avd": num(&r, "averageViewDuration"),
        "pct": num(&r, "averageViewPercentage"), "horas": num(&r, "estimatedMinutesWatched") / 60.0,
        "likes": num(&r, "likes"), "comentarios": num(&r, "comments"), "shares": num(&r, "shares"),
        "inscritos": num(&r, "subscribersGained") - num(&r, "subscribersLost"),
        "continuaram": Value::Null, "inicio": inicio, "fim": fim
    }))
}

/// curva de retenção, média, marcos e maiores quedas
pub fn retencao(curva: &[(f64, f64, f64)], dur: f64) -> Value {
    // curva: (fração do vídeo, audienceWatchRatio, relativeRetentionPerformance)
    if curva.is_empty() {
        return Value::Null;
    }
    let media = curva.iter().map(|c| c.1).sum::<f64>() / curva.len() as f64;
    let rel = curva.iter().map(|c| c.2).sum::<f64>() / curva.len() as f64;
    let at = |x: f64| curva.iter().min_by(|a, b| (a.0 - x).abs().partial_cmp(&(b.0 - x).abs()).unwrap()).map(|c| c.1);
    // marcos em ordem de tempo (lista, não mapa: o JSON não guarda ordem de chaves)
    let mut marcos = Vec::new();
    if dur > 0.0 {
        marcos.push((3.0 / dur, "3 s".to_string()));
        if dur > 45.0 {
            marcos.push((30.0 / dur, "30 s".to_string()));
        }
    }
    for (k, x) in [("25%", 0.25), ("50%", 0.5), ("75%", 0.75), ("100%", 1.0)] {
        marcos.push((x, k.to_string()));
    }
    marcos.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let marcos: Vec<Value> = marcos.into_iter().map(|(x, k)| json!({ "ponto": k, "assistindo": at(x) })).collect();
    // quedas que importam: ≥ 1 p.p. entre pontos vizinhos, e no máximo uma a cada 3 % do vídeo
    let mut todas: Vec<(f64, f64, f64)> = curva.windows(2).map(|w| (w[1].0 * dur, w[0].1 - w[1].1, w[1].0)).filter(|q| q.1 >= 0.01).collect();
    todas.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut quedas: Vec<(f64, f64, f64)> = Vec::new();
    for q in todas {
        if quedas.iter().all(|x| (x.2 - q.2).abs() > 0.03) {
            quedas.push(q);
        }
    }
    json!({
        "media": media, "relativa_media": rel, "marcos": marcos,
        "maiores_quedas": quedas.iter().take(5).map(|q| json!({ "s": (q.0 * 10.0).round() / 10.0, "fracao": q.2, "queda": q.1 })).collect::<Vec<_>>(),
        "curva": curva.iter().map(|c| json!({ "s": (c.0 * dur * 10.0).round() / 10.0, "fracao": c.0, "assistindo": c.1, "relativa": c.2 })).collect::<Vec<_>>()
    })
}

fn yt_retencao(b: &str, id: &str, inicio: &str, fim: &str, dur: f64) -> Result<Value> {
    let f = format!("video=={id}");
    let r = report(b, inicio, fim, &[("metrics", "audienceWatchRatio,relativeRetentionPerformance"), ("dimensions", "elapsedVideoTimeRatio"), ("filters", &f)])?;
    let c: Vec<(f64, f64, f64)> = r.iter().map(|x| (num(x, "elapsedVideoTimeRatio"), num(x, "audienceWatchRatio"), num(x, "relativeRetentionPerformance"))).collect();
    Ok(retencao(&c, dur))
}

fn yt_trafego(b: &str, id: &str, inicio: &str, fim: &str) -> Vec<Value> {
    let f = format!("video=={id}");
    report(b, inicio, fim, &[("metrics", "views,estimatedMinutesWatched"), ("dimensions", "insightTrafficSourceType"), ("filters", &f), ("sort", "-views")]).unwrap_or_default()
}

/// vídeo completo numa janela: métricas + retenção (opcional); erro vira aviso, não derruba o relatório
fn yt_linha(b: &str, v: &Value, j: &Janela, agora: i64, com_retencao: bool) -> Value {
    let id = v["id"].as_str().unwrap_or("");
    let publicado = v["publicado"].as_str().and_then(tempo::parse_iso).and_then(|r| r.ok()).unwrap_or(agora);
    let (ini, fim, parcial) = j.faixa(publicado, agora);
    let mut m = match yt_metricas(b, id, &ini, &fim) {
        Ok(m) => m,
        Err(e) => return json!({ "erro": format!("{e:#}") }),
    };
    if let Some((t, n)) = parcial {
        m["parcial"] = json!(format!("{t} de {n} dias"));
    }
    // janela que chega até hoje e Analytics sem dado: usa os contadores em tempo real da Data API
    if m["sem_analytics"] == true && (parcial.is_some() || *j == Janela::Total) {
        for (k, src) in [("views", "views"), ("likes", "likes"), ("comentarios", "comentarios")] {
            if let Some(x) = v["total"].get(src).and_then(|x| x.as_f64()) {
                m[k] = json!(x);
            }
        }
        m["tempo_real"] = json!(true);
    }
    if com_retencao && num(&m, "views") > 0.0 {
        match yt_retencao(b, id, &ini, &fim, v["duracao_s"].as_f64().unwrap_or(0.0)) {
            Ok(r) if !r.is_null() => {
                m["retencao_media"] = r["media"].clone();
                m["relativa_media"] = r["relativa_media"].clone();
                m["retencao"] = r;
            }
            Ok(_) => {}
            Err(e) => m["erro_retencao"] = json!(format!("{e:#}")),
        }
    }
    m
}

// ---------------------------------------------------------------- filtros

struct Filtros {
    tipo: Option<String>,
    privacidade: Option<String>,
    desde: Option<i64>,
    ate: Option<i64>,
    busca: Option<String>,
    tag: Option<String>,
    min_views: f64,
    ordenar: String,
    asc: bool,
    limite: usize,
}

impl Filtros {
    fn de(kv: &Kv) -> Result<Filtros> {
        let dia = |k: &str| -> Result<Option<i64>> {
            kv.get(k).map(|d| tempo::parse_iso(&format!("{d}T00:00Z")).context("data inválida").and_then(|r| r).with_context(|| format!("{k}={d}: use AAAA-MM-DD"))).transpose()
        };
        let tipo = kv.get("tipo").map(|t| t.to_lowercase()).filter(|t| t != "todos");
        if let Some(t) = &tipo {
            if !matches!(t.as_str(), "short" | "shorts" | "longo" | "longos" | "long") {
                bail!("tipo={t}: use short, longo ou todos");
            }
        }
        Ok(Filtros {
            tipo: tipo.map(|t| if t.starts_with("short") { "short".into() } else { "longo".into() }),
            privacidade: kv.get("privacidade").map(|s| s.to_lowercase()),
            desde: dia("desde")?,
            ate: dia("ate")?.map(|t| t + 86_399),
            busca: kv.get("busca").map(|s| s.to_lowercase()),
            tag: kv.get("tag").map(|s| s.to_lowercase().trim_start_matches('#').to_string()),
            min_views: kv.get("min_views").and_then(|x| x.parse().ok()).unwrap_or(0.0),
            ordenar: kv.get("ordenar").unwrap_or("views").to_lowercase(),
            asc: matches!(kv.get("ordem"), Some("asc" | "crescente")),
            limite: kv.get("limite").and_then(|x| x.parse().ok()).unwrap_or(usize::MAX),
        })
    }
    /// filtros que não dependem das métricas
    fn passa(&self, v: &Value) -> bool {
        let tipo = v["tipo"].as_str().unwrap_or("");
        if let Some(t) = &self.tipo {
            if !tipo.starts_with(t.as_str()) {
                return false;
            }
        }
        if let Some(p) = &self.privacidade {
            if v["privacidade"].as_str().map(|x| x.to_lowercase()) != Some(p.clone()) {
                return false;
            }
        }
        let publicado = v["publicado"].as_str().and_then(tempo::parse_iso).and_then(|r| r.ok());
        if let (Some(d), Some(p)) = (self.desde, publicado) {
            if p < d {
                return false;
            }
        }
        if let (Some(a), Some(p)) = (self.ate, publicado) {
            if p > a {
                return false;
            }
        }
        let low = |k: &str| v[k].as_str().unwrap_or("").to_lowercase();
        if let Some(b) = &self.busca {
            if !(low("titulo").contains(b.as_str()) || low("descricao").contains(b.as_str())) {
                return false;
            }
        }
        if let Some(t) = &self.tag {
            let tags: Vec<String> = v["tags"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_lowercase())).collect()).unwrap_or_default();
            let hashtag = format!("#{t}");
            if !(tags.iter().any(|x| x == t) || low("descricao").contains(&hashtag) || low("titulo").contains(&hashtag)) {
                return false;
            }
        }
        true
    }
    fn chave(&self) -> &'static str {
        match self.ordenar.as_str() {
            "intencionais" => "intencionais",
            "retencao" | "retenção" => "retencao_media",
            "avd" | "duracao" => "avd",
            "pct" | "porcentagem" => "pct",
            "horas" => "horas",
            "inscritos" => "inscritos",
            "likes" | "gostei" | "curtidas" => "likes",
            "comentarios" => "comentarios",
            "shares" | "compartilhamentos" => "shares",
            "engajamento" => "engajamento",
            "publicado" | "data" => "publicado_ts",
            "alcance" => "alcance",
            _ => "views",
        }
    }
    fn descricao(&self) -> String {
        let mut f = Vec::new();
        if let Some(t) = &self.tipo { f.push(format!("tipo={t}")); }
        if let Some(t) = &self.privacidade { f.push(format!("privacidade={t}")); }
        if let Some(t) = self.desde { f.push(format!("desde={}", tempo::date(t))); }
        if let Some(t) = self.ate { f.push(format!("até={}", tempo::date(t))); }
        if let Some(t) = &self.busca { f.push(format!("busca=\"{t}\"")); }
        if let Some(t) = &self.tag { f.push(format!("tag={t}")); }
        if self.min_views > 0.0 { f.push(format!("min_views={}", self.min_views)); }
        f.push(format!("ordenar={} {}", self.ordenar, if self.asc { "↑" } else { "↓" }));
        if self.limite != usize::MAX { f.push(format!("limite={}", self.limite)); }
        f.join(" · ")
    }
}

fn ordenar(itens: &mut [Value], chave: &str, asc: bool) {
    itens.sort_by(|a, b| {
        let x = get(&a["m"], chave).or_else(|| get(a, chave)).unwrap_or(f64::MIN);
        let y = get(&b["m"], chave).or_else(|| get(b, chave)).unwrap_or(f64::MIN);
        let o = x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal);
        if asc { o } else { o.reverse() }
    });
}

/// medianas por métrica entre itens completos (sem "parcial" nem erro)
fn medianas(itens: &[&Value], linhas: &[(&str, &str, Fmt)]) -> Value {
    let ok: Vec<&&Value> = itens.iter().filter(|x| x["m"].get("parcial").is_none() && x["m"].get("erro").is_none()).collect();
    let mut m = serde_json::Map::new();
    for (k, _, _) in linhas {
        m.insert(k.to_string(), json!(median(ok.iter().filter_map(|x| get(&x["m"], k)).collect())));
    }
    m.insert("n".into(), json!(ok.len()));
    Value::Object(m)
}

// ---------------------------------------------------------------- comando

pub fn run(args: &[String]) -> Result<()> {
    let kv = Kv::parse(args, &[])?;
    kv.check_known(&[
        "platform", "platforms", "account", "conta", "video", "janela", "beats", "tipo", "privacidade", "desde", "ate", "busca", "tag", "min_views",
        "ordenar", "ordem", "limite", "max", "html", "o", "out", "studio",
    ])?;
    let mut plats = kv::platforms(&kv)?;
    if plats.is_empty() {
        plats = vec!["yt"];
    }
    let janelas = Janela::parse(kv.get("janela").unwrap_or(if kv.get("video").is_some() { "todas" } else { "d7" }))?;
    let videos = kv.get("video").map(pf::split_list).unwrap_or_default();
    if videos.is_empty() && janelas.len() > 1 {
        bail!("no relatório geral use uma janela só (janela=d7, d28, total...) para comparar os vídeos entre si");
    }
    let filtros = Filtros::de(&kv)?;
    let html = !matches!(kv.get("html"), Some("0" | "false" | "nao" | "não"));
    let dir = kv.get("o").or(kv.get("out")).map(PathBuf::from).unwrap_or_else(|| if Path::new("out").is_dir() { PathBuf::from("out/relatorios") } else { PathBuf::from("relatorios") });
    std::fs::create_dir_all(&dir)?;
    let max = kv.get("max").and_then(|x| x.parse().ok()).unwrap_or(200usize);
    let mut falhas = 0;
    for p in plats {
        let p = if p == "ttb" { "tiktok" } else { p };
        for acct in pf::contas_de(&kv, if p == "yt" { "yt" } else { "tiktok" }, true) {
            let r = if p == "yt" { yt(&kv, acct.as_deref(), &videos, &janelas, &filtros, max) } else { tt(acct.as_deref(), &videos, &filtros, max).map(|r| vec![r]) };
            match r.and_then(|rels| rels.iter().try_for_each(|rel| salvar(&dir, rel, html))) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("erro em {p}{}: {e:#}\n", acct.map(|x| format!("/{x}")).unwrap_or_default());
                    falhas += 1;
                }
            }
        }
    }
    if falhas > 0 {
        bail!("{falhas} relatório(s) com erro; veja acima");
    }
    Ok(())
}

fn salvar(dir: &Path, rel: &Value, html: bool) -> Result<()> {
    let base = rel["arquivo"].as_str().unwrap_or("relatorio").to_string();
    let md = markdown(rel);
    crate::util::write_json(&dir.join(format!("{base}.json")), rel)?;
    std::fs::write(dir.join(format!("{base}.md")), &md)?;
    println!("{md}");
    eprintln!("→ {}", dir.join(format!("{base}.md")).display());
    if html {
        std::fs::write(dir.join(format!("{base}.html")), pagina(rel))?;
        eprintln!("→ {}", dir.join(format!("{base}.html")).display());
    }
    Ok(())
}

fn slug(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect()
}

fn yt(kv: &Kv, acct: Option<&str>, videos: &[String], janelas: &[Janela], f: &Filtros, max: usize) -> Result<Vec<Value>> {
    let (conta, t) = auth::token("yt", acct)?;
    // studio=1: completa com a API interna do Studio (vf studio), sessão do mesmo apelido
    // dados do Studio (extensão vf studio) entram sozinhos quando existem; studio=0 desliga
    let studio = !matches!(kv.get("studio"), Some("0" | "false" | "nao" | "não"));
    let b = pf::bearer(&t);
    let agora = tempo::now();
    let mut avisos = vec!["\"Continuaram assistindo\" (assistiram × deslizaram) só existe no YouTube Studio: a API pública não fornece.".to_string()];
    let ch = pf::yt_get(&b, "channels", &[("part", "snippet,contentDetails"), ("mine", "true")]).context("YouTube Data API: ler o canal")?;
    let it = ch.pointer("/items/0").cloned().context("essa conta Google não tem canal do YouTube")?;
    let canal = it.pointer("/snippet/title").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let canal_id = it["id"].as_str().unwrap_or("").to_string();
    let uploads = it.pointer("/contentDetails/relatedPlaylists/uploads").and_then(|x| x.as_str()).unwrap_or("").to_string();

    // uploads (base dos filtros e das medianas)
    let mut ids: Vec<String> = Vec::new();
    let mut token = String::new();
    while ids.len() < max && !uploads.is_empty() {
        let mut q = vec![("part", "contentDetails"), ("playlistId", uploads.as_str()), ("maxResults", "50")];
        if !token.is_empty() {
            q.push(("pageToken", token.as_str()));
        }
        let v = pf::yt_get(&b, "playlistItems", &q)?;
        ids.extend(v["items"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x.pointer("/contentDetails/videoId").and_then(|v| v.as_str()).map(String::from)));
        match v.get("nextPageToken").and_then(|x| x.as_str()) {
            Some(n) => token = n.to_string(),
            None => break,
        }
    }
    ids.truncate(max);
    for v in videos {
        if !ids.contains(v) {
            ids.push(v.clone());
        }
    }
    let mut todos = pf::yt_videos(&b, &ids)?;
    // Short × longo pela Analytics (creatorContentType); sem views, fica a estimativa pela duração
    let inicio = todos.iter().filter_map(|v| v["publicado"].as_str().and_then(tempo::parse_iso).and_then(|r| r.ok())).min().unwrap_or(agora);
    let shorts: HashSet<String> = report(&b, &tempo::date(inicio), &tempo::date(agora), &[("metrics", "views"), ("dimensions", "video"), ("filters", "creatorContentType==shorts"), ("sort", "-views"), ("maxResults", "200")])
        .map(|r| r.iter().filter_map(|x| x["video"].as_str().map(String::from)).collect())
        .unwrap_or_default();
    for v in todos.iter_mut() {
        if shorts.contains(v["id"].as_str().unwrap_or("")) {
            v["tipo"] = json!("short");
        }
        v["publicado_ts"] = json!(v["publicado"].as_str().and_then(tempo::parse_iso).and_then(|r| r.ok()));
    }
    let publico = |v: &Value| matches!(v["privacidade"].as_str(), Some("public" | "unlisted"));

    if !videos.is_empty() {
        // ---------------- individual (um relatório por vídeo pedido; o JSON principal é o primeiro)
        let mut rels = Vec::new();
        for id in videos {
            let v = todos.iter().find(|x| x["id"] == id.as_str()).cloned().with_context(|| format!("vídeo {id} não encontrado no canal {canal} ({conta})"))?;
            let tipo = v["tipo"].as_str().unwrap_or("").trim_end_matches('?').to_string();
            let principal = janelas.iter().find(|j| **j == Janela::Total).or(janelas.first()).unwrap().clone();
            let mut colunas: Vec<Value> = janelas.par_iter().map(|j| {
                let mut m = yt_linha(&b, &v, j, agora, *j == principal);
                m["janela"] = json!(j.chave());
                m["janela_nome"] = json!(j.nome());
                m
            }).collect();
            let pub_ts = v["publicado_ts"].as_i64().unwrap_or(agora);
            let res = if studio { crate::studio_dados::resumo(id) } else { None };
            let mut horas_iniciais = Value::Null;
            if let Some(r) = &res {
                // Studio completa a janela principal: o que a API não tem (impressões, CTR, continuaram) e o que
                // ela ainda não processou (gostei, inscritos, horas, duração média)
                let mut usados = Vec::new();
                if let Some(c) = colunas.iter_mut().find(|c| c["janela"] == principal.chave().as_str()) {
                    for (de, para, sempre) in [("continuaram", "continuaram", true), ("impressoes", "impressoes", true), ("ctr", "ctr", true), ("gostei", "likes", false), ("inscritos", "inscritos", false), ("horas", "horas", false), ("avd", "avd", false)] {
                        if let Some(x) = r["metricas"][de]["valor"].as_f64() {
                            if sempre || get(c, para).is_none() {
                                c[para] = json!(x);
                                usados.push(para);
                            }
                        }
                    }
                    for (k, val) in crate::studio::linhas_mapeadas(&conta, id) {
                        if get(c, &k).is_none() {
                            c[k.as_str()] = val;
                        }
                    }
                }
                avisos.retain(|a| !a.starts_with("\"Continuaram assistindo\""));
                avisos.push(format!(
                    "Studio (extensão vf studio, não oficial; capturado {}): {} na coluna \"{}\".{}",
                    r["atualizado"].as_str().unwrap_or("").get(..16).unwrap_or("").replace('T', " "),
                    if usados.is_empty() { "nenhuma linha".to_string() } else { usados.join(", ") },
                    principal.nome(),
                    r["processando"].as_array().filter(|a| !a.is_empty()).map(|a| format!(" Ainda processando no Studio: {}.", a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", "))).unwrap_or_default()
                ));
                horas_iniciais = crate::studio_dados::primeiras_horas(r, pub_ts, agora);
                if horas_iniciais["cobre"] == false {
                    avisos.push(format!("primeiras horas: {}", horas_iniciais["motivo"].as_str().unwrap_or("")));
                }
            } else if studio {
                avisos.push(format!("sem dados do Studio para {id}: com a extensão vf studio, abra as análises do vídeo no Studio (de preferência nas primeiras 48 h, para ter a curva por hora)"));
            }
            // pares: mesmo tipo, públicos, os 30 mais recentes
            let pares: Vec<Value> = todos.iter().filter(|x| x["id"] != id.as_str() && publico(x) && x["tipo"].as_str().unwrap_or("").trim_end_matches('?') == tipo).take(30).cloned().collect();
            let mut meds = serde_json::Map::new();
            for j in janelas {
                let itens: Vec<Value> = pares.par_iter().map(|x| json!({ "m": yt_linha(&b, x, j, agora, *j == principal) })).collect();
                meds.insert(j.chave(), medianas(&itens.iter().collect::<Vec<_>>(), YT_LINHAS));
            }
            let pid = v["publicado"].as_str().unwrap_or("");
            // primeiras horas dos pares (mesmo tipo) com captura do Studio: mediana para comparar
            let mut horas_med = serde_json::Map::new();
            if studio && horas_iniciais.is_object() {
                let ph: Vec<Value> = pares.iter().filter_map(|x| {
                    let r = crate::studio_dados::resumo(x["id"].as_str()?)?;
                    Some(crate::studio_dados::primeiras_horas(&r, x["publicado_ts"].as_i64()?, agora))
                }).filter(|p| p["cobre"] == true).collect();
                for k in ["h1", "h3", "h6", "h12", "h24", "h48"] {
                    horas_med.insert(k.into(), json!(median(ph.iter().filter_map(|p| p[k].as_f64()).collect())));
                }
                horas_med.insert("n".into(), json!(ph.len()));
            }
            let (ini, fim, _) = principal.faixa(pub_ts, agora);
            let trafego = yt_trafego(&b, id, &ini, &fim);
            let beats = kv.get("beats").and_then(|p| std::fs::read(p).ok()).and_then(|r| serde_json::from_slice::<Value>(&r).ok());
            if colunas.iter().any(|c| c["tempo_real"] == true) {
                avisos.push("Analytics ainda sem dados para este vídeo (atraso de 24-72 h): visualizações, gostei e comentários vêm da Data API em tempo real; as demais linhas ficam \"—\" até a Analytics processar.".into());
            }
            if !publico(&v) {
                avisos.push(format!("{id} está {}: vídeo não público não soma views", v["privacidade"].as_str().unwrap_or("privado")));
            }
            if agora - pub_ts < 3 * 86_400 {
                avisos.push("vídeo com menos de 3 dias: a Analytics atrasa 24-72 h, números ainda parciais".into());
            }
            rels.push(json!({
                "tipo_relatorio": "video", "plataforma": "yt", "conta": conta, "canal": canal, "gerado": tempo::rfc3339(agora),
                "arquivo": format!("yt-{}-{}-{}-{}", slug(&conta), slug(id), janelas.iter().map(|j| j.chave()).collect::<Vec<_>>().join("_").replace("..", "-"), today()),
                "video": v, "publicado": pid, "janelas": colunas, "principal": principal.chave(), "medianas": meds,
                "pares": pares.len(), "tipo_pares": tipo, "trafego": trafego, "beats": beats, "avisos": avisos.clone(),
                "studio": res.as_ref().map(enxuto), "primeiras_horas": horas_iniciais, "primeiras_horas_mediana": Value::Object(horas_med)
            }));
        }
        return Ok(rels);
    }

    // ---------------- geral
    let j = janelas[0].clone();
    // sem privacidade= explícita, só vídeos públicos/não listados (privado não soma views)
    let base: Vec<Value> = todos.into_iter().filter(|v| f.passa(v) && (f.privacidade.is_some() || publico(v))).collect();
    if base.is_empty() {
        avisos.push("nenhum vídeo público passou nos filtros".into());
    }
    let mut itens: Vec<Value> = base.par_iter().map(|v| {
        let mut m = yt_linha(&b, v, &j, agora, true);
        // Studio: continuaram, impressões, CTR e views nas primeiras 24 h (quando houver captura)
        if studio {
            if let Some(r) = v["id"].as_str().and_then(crate::studio_dados::resumo) {
                for k in ["continuaram", "impressoes", "ctr"] {
                    if let Some(x) = r["metricas"][k]["valor"].as_f64() {
                        m[k] = json!(x);
                    }
                }
                if let Some(pt) = v["publicado_ts"].as_i64() {
                    let ph = crate::studio_dados::primeiras_horas(&r, pt, agora);
                    m["h1"] = ph["h1"].clone();
                    m["h24"] = ph["h24"].clone();
                }
            }
        }
        json!({ "id": v["id"], "titulo": v["titulo"], "tipo": v["tipo"], "publicado": v["publicado"], "publicado_ts": v["publicado_ts"], "duracao_s": v["duracao_s"], "url": v["url"], "m": m })
    }).collect();
    for x in &itens {
        if let Some(e) = x["m"]["erro"].as_str() {
            avisos.push(format!("{}: {e}", x["id"].as_str().unwrap_or("")));
        }
    }
    let rt = itens.iter().filter(|x| x["m"]["tempo_real"] == true).count();
    if rt > 0 {
        avisos.push(format!("{rt} vídeo(s) ainda sem dados na Analytics (atraso de 24-72 h): visualizações, gostei e comentários em tempo real (Data API); demais colunas \"—\""));
    }
    itens.retain(|x| num(&x["m"], "views") >= f.min_views);
    ordenar(&mut itens, f.chave(), f.asc);
    itens.truncate(f.limite);
    let parciais = itens.iter().filter(|x| x["m"].get("parcial").is_some()).count();
    if parciais > 0 {
        avisos.push(format!("{parciais} vídeo(s) mais novos que a janela {}: marcados \"parcial\" e fora das medianas", j.chave()));
    }
    let totais = {
        let mut t = serde_json::Map::new();
        for k in ["views", "intencionais", "horas", "likes", "inscritos", "comentarios", "shares", "impressoes", "h1", "h24"] {
            // só soma colunas que algum vídeo tem (vazio continua "—", não 0)
            if itens.iter().any(|x| get(&x["m"], k).is_some()) {
                t.insert(k.into(), json!(itens.iter().filter_map(|x| get(&x["m"], k)).sum::<f64>()));
            }
        }
        Value::Object(t)
    };
    let por_tipo = |tp: &str| {
        let sel: Vec<&Value> = itens.iter().filter(|x| x["tipo"].as_str().unwrap_or("").starts_with(tp)).collect();
        let mut m = medianas(&sel, YT_LINHAS);
        for k in ["h1", "h24"] {
            m[k] = json!(median(sel.iter().filter_map(|x| get(&x["m"], k)).collect()));
        }
        m
    };
    let canal_studio = if studio && !canal_id.is_empty() { crate::studio_dados::resumo(&format!("canal_{canal_id}")).map(|r| enxuto(&r)) } else { None };
    if studio && itens.iter().any(|x| get(&x["m"], "continuaram").is_some() || get(&x["m"], "h24").is_some()) {
        avisos.retain(|a| !a.starts_with("\"Continuaram assistindo\""));
        avisos.push("colunas Continuaram, Impressões, CTR, 1ª hora e 24 h vêm do Studio (extensão vf studio, não oficial) quando há captura do vídeo.".into());
    }
    Ok(vec![json!({
        "tipo_relatorio": "geral", "plataforma": "yt", "conta": conta, "canal": canal, "gerado": tempo::rfc3339(agora),
        "arquivo": format!("yt-{}-geral-{}-{}", slug(&conta), j.chave().replace("..", "-"), today()),
        "janela": j.chave(), "janela_nome": j.nome(), "filtros": f.descricao(), "itens": itens, "totais": totais,
        "medianas": { "short": por_tipo("short"), "longo": por_tipo("longo") }, "avisos": avisos,
        "studio_canal": canal_studio, "colunas_extra": [["h1", "Views 1ª h"], ["h24", "Views 24 h"]]
    })])
}

/// resumo do Studio sem o que pesa (cartões crus e série por minuto)
fn enxuto(r: &Value) -> Value {
    let mut r = r.clone();
    if let Some(o) = r.as_object_mut() {
        let cartoes: Vec<Value> = o.get("cartoes").and_then(|c| c.as_object()).map(|c| c.iter().filter(|(_, x)| !x["sem_dados"].as_bool().unwrap_or(false)).map(|(k, x)| json!({ "chave": k, "titulo": x["titulo"], "texto": x["texto"], "periodo": x["periodo"], "pares": x["pares"], "barras": x["barras"] })).collect()).unwrap_or_default();
        o.insert("cartoes".into(), json!(cartoes));
        o.remove("por_minuto");
    }
    r
}

fn today() -> String {
    tempo::date(tempo::now())
}

// ---------------------------------------------------------------- TikTok

/// TikTok Business API (EXPERIMENTAL): métricas de retenção por post. Campos conforme a doc v1.3 de
/// /business/video/list/; se o TikTok recusar algum, a resposta crua sai no erro para ajustar.
fn ttb_videos(acct: Option<&str>, max: usize) -> Result<(String, Vec<Value>)> {
    let (conta, t) = auth::token("ttb", acct)?;
    let fields = r#"["item_id","create_time","caption","share_url","video_duration","video_views","likes","comments","shares","reach","full_video_watched_rate","total_time_watched","average_time_watched","impression_sources","video_view_retention"]"#;
    let mut out = Vec::new();
    let mut cursor: Option<i64> = None;
    while out.len() < max {
        let mut q: Vec<(&str, String)> = vec![("business_id", t.open_id.clone()), ("fields", fields.into()), ("max_count", "20".into())];
        if let Some(c) = cursor {
            q.push(("cursor", c.to_string()));
        }
        let qs: Vec<(&str, &str)> = q.iter().map(|(a, b)| (*a, b.as_str())).collect();
        let url = format!("{}/business/video/list/?{}", auth::TTB_API, net::query(&qs));
        let d = auth::ttb_data(&net::get(&url, &[("Access-Token", &t.access_token)])?.json()?)?;
        out.extend(d["videos"].as_array().cloned().unwrap_or_default());
        match (d["has_more"].as_bool(), d["cursor"].as_i64()) {
            (Some(true), Some(c)) => cursor = Some(c),
            _ => break,
        }
    }
    out.truncate(max);
    Ok((conta, out))
}

fn tt(acct: Option<&str>, videos: &[String], f: &Filtros, max: usize) -> Result<Value> {
    let agora = tempo::now();
    let mut avisos = vec!["TikTok: a API dá totais desde a publicação (sem janela d1/d7); \"intencionais\" e \"continuaram\" não existem para o TikTok.".to_string()];
    // Display API (perfil pessoal autorizado com vf auth platform=tiktok)
    let display = match auth::token("tiktok", acct) {
        Ok((conta, t)) => Some((conta, pf::tt_list(&pf::bearer(&t), videos, max)?)),
        Err(e) => {
            avisos.push(format!("Display API indisponível: {e:#}"));
            None
        }
    };
    // Business API (retenção) se houver conta tiktok-biz
    // a conta Business usa o mesmo apelido quando existir; senão a padrão do tiktok-biz
    let biz_acct = acct.filter(|a| auth::accounts("ttb").iter().any(|x| x == a));
    let biz = if auth::accounts("ttb").is_empty() {
        avisos.push("sem conta TikTok Business: tempo médio, retenção, até o fim e alcance ficam \"—\". Para ter: vf auth platform=tiktok-biz (conta Business/Creator; EXPERIMENTAL)".into());
        None
    } else {
        match ttb_videos(biz_acct, max.max(videos.len())) {
            Ok(x) => Some(x),
            Err(e) => {
                avisos.push(format!("TikTok Business: {e:#}"));
                None
            }
        }
    };
    if display.is_none() && biz.is_none() {
        bail!("nenhuma conta TikTok utilizável (vf auth platform=tiktok e/ou platform=tiktok-biz)");
    }
    let conta = display.as_ref().map(|d| d.0.clone()).or(biz.as_ref().map(|b| b.0.clone())).unwrap_or_default();
    // une por id do post
    let mut itens: Vec<Value> = Vec::new();
    let mut add = |id: String, base: Value| {
        if let Some(x) = itens.iter_mut().find(|x| x["id"] == id.as_str()) {
            for (k, v) in base.as_object().cloned().unwrap_or_default() {
                if !v.is_null() {
                    x[k.as_str()] = v;
                }
            }
        } else {
            let mut b = base;
            b["id"] = json!(id);
            itens.push(b);
        }
    };
    for v in display.as_ref().map(|d| d.1.clone()).unwrap_or_default() {
        add(v["id"].as_str().unwrap_or("").to_string(), json!({
            "titulo": v["title"].as_str().filter(|s| !s.is_empty()).or(v["video_description"].as_str()).unwrap_or(""),
            "descricao": v["video_description"], "url": v["share_url"], "duracao_s": num(&v, "duration"),
            "publicado_ts": num(&v, "create_time") as i64, "publicado": tempo::rfc3339(num(&v, "create_time") as i64), "tipo": "tiktok",
            "views": num(&v, "view_count"), "likes": num(&v, "like_count"), "comentarios": num(&v, "comment_count"), "shares": num(&v, "share_count"),
        }));
    }
    for v in biz.as_ref().map(|b| b.1.clone()).unwrap_or_default() {
        let dur = num(&v, "video_duration");
        let avg = get(&v, "average_time_watched");
        let fim = get(&v, "full_video_watched_rate").map(|x| if x <= 1.0 { x * 100.0 } else { x });
        let ts = num(&v, "create_time") as i64;
        add(v["item_id"].as_str().map(String::from).unwrap_or_else(|| v["item_id"].to_string()), json!({
            "titulo": v["caption"], "url": v["share_url"], "duracao_s": if dur > 0.0 { json!(dur) } else { Value::Null },
            "publicado_ts": if ts > 0 { json!(ts) } else { Value::Null }, "publicado": if ts > 0 { json!(tempo::rfc3339(ts)) } else { Value::Null }, "tipo": "tiktok",
            "views": get(&v, "video_views"), "likes": get(&v, "likes"), "comentarios": get(&v, "comments"), "shares": get(&v, "shares"),
            "alcance": get(&v, "reach"), "avd": avg, "ate_o_fim": fim, "horas": get(&v, "total_time_watched").map(|s| s / 3600.0),
            "retencao_media": match (avg, dur > 0.0) { (Some(a), true) => json!(a / dur), _ => Value::Null },
            "impression_sources": v["impression_sources"], "video_view_retention": v["video_view_retention"],
        }));
    }
    // métricas no formato comum ("m") + engajamento
    for x in itens.iter_mut() {
        let views = num(x, "views");
        let eng = num(x, "likes") + num(x, "comentarios") + num(x, "shares");
        let mut m = serde_json::Map::new();
        for (k, _, _) in TT_LINHAS {
            if let Some(v) = get(x, k) {
                m.insert(k.to_string(), json!(v));
            }
        }
        m.insert("engajamento".into(), json!(if views > 0.0 { eng / views } else { 0.0 }));
        // TikTok Business: 0 costuma ser "sem dado ainda" (atraso de 24-48 h), não zero real
        for k in ["avd", "retencao_media", "ate_o_fim", "alcance", "horas"] {
            if m.get(k).and_then(|v| v.as_f64()) == Some(0.0) && views > 0.0 {
                m.insert(k.into(), Value::Null);
            }
        }
        // curva por segundo se vier no formato [{second, percentage}]
        if let Some(arr) = x["video_view_retention"].as_array() {
            let dur = num(x, "duracao_s");
            let c: Vec<(f64, f64, f64)> = arr.iter().filter_map(|p| {
                let s = get(p, "second").or(get(p, "time"))?;
                let pc = get(p, "percentage").or(get(p, "rate"))?;
                Some((if dur > 0.0 { s / dur } else { 0.0 }, if pc > 1.0 { pc / 100.0 } else { pc }, 0.5))
            }).collect();
            if !c.is_empty() {
                m.insert("retencao".into(), retencao(&c, dur));
            }
        }
        x["m"] = Value::Object(m);
    }
    let base_f = |x: &Value| f.passa(x);

    if !videos.is_empty() {
        let id = &videos[0];
        let v = itens.iter().find(|x| x["id"] == id.as_str()).cloned().with_context(|| format!("post {id} não encontrado (a Display API só lista posts públicos)"))?;
        let pares: Vec<&Value> = itens.iter().filter(|x| x["id"] != id.as_str()).collect();
        let med = medianas(&pares, TT_LINHAS);
        let mut col = v["m"].clone();
        col["janela"] = json!("total");
        col["janela_nome"] = json!("desde a publicação");
        return Ok(json!({
            "tipo_relatorio": "video", "plataforma": "tiktok", "conta": conta, "gerado": tempo::rfc3339(agora),
            "arquivo": format!("tiktok-{}-{}-{}", slug(&conta), slug(id), today()),
            "video": v, "janelas": [col], "principal": "total", "medianas": { "total": med }, "pares": pares.len(), "tipo_pares": "tiktok",
            "trafego": v["impression_sources"], "avisos": avisos
        }));
    }
    let mut lista: Vec<Value> = itens.into_iter().filter(|x| base_f(x)).filter(|x| num(&x["m"], "views") >= f.min_views).collect();
    ordenar(&mut lista, f.chave(), f.asc);
    lista.truncate(f.limite);
    let mut totais = serde_json::Map::new();
    for k in ["views", "likes", "comentarios", "shares", "horas", "alcance"] {
        totais.insert(k.into(), json!(lista.iter().map(|x| num(&x["m"], k)).sum::<f64>()));
    }
    let med = medianas(&lista.iter().collect::<Vec<_>>(), TT_LINHAS);
    Ok(json!({
        "tipo_relatorio": "geral", "plataforma": "tiktok", "conta": conta, "gerado": tempo::rfc3339(agora),
        "arquivo": format!("tiktok-{}-geral-{}", slug(&conta), today()),
        "janela": "total", "janela_nome": "desde a publicação", "filtros": f.descricao(), "itens": lista, "totais": Value::Object(totais),
        "medianas": { "tiktok": med }, "avisos": avisos
    }))
}

// ---------------------------------------------------------------- Markdown

fn linhas_de(rel: &Value) -> &'static [(&'static str, &'static str, Fmt)] {
    if rel["plataforma"] == "tiktok" { TT_LINHAS } else { YT_LINHAS }
}

pub fn markdown(rel: &Value) -> String {
    let mut o = String::new();
    let linhas = linhas_de(rel);
    let plat = if rel["plataforma"] == "tiktok" { "TikTok" } else { "YouTube" };
    if rel["tipo_relatorio"] == "video" {
        let v = &rel["video"];
        let titulo = v["titulo"].as_str().unwrap_or("");
        o += &format!("# {}\n\n", esc_md(if titulo.is_empty() { v["id"].as_str().unwrap_or("") } else { titulo }));
        o += &format!(
            "{} · {} · {} · publicado {} · duração {} · {}{}\n\n",
            plat,
            v["url"].as_str().unwrap_or(""),
            v["tipo"].as_str().unwrap_or(""),
            v["publicado"].as_str().unwrap_or("").get(..10).unwrap_or(""),
            fmt_dur(num(v, "duracao_s")),
            rel["conta"].as_str().unwrap_or(""),
            rel["canal"].as_str().map(|c| format!(" ({c})")).unwrap_or_default()
        );
        let cols = rel["janelas"].as_array().cloned().unwrap_or_default();
        let tipo = rel["tipo_pares"].as_str().unwrap_or("");
        if cols.len() == 1 {
            let c = &cols[0];
            let j = c["janela"].as_str().unwrap_or("");
            let med = &rel["medianas"][j];
            o += &format!("## Desempenho · {}{}\n\n", c["janela_nome"].as_str().unwrap_or(""), periodo(c));
            o += &format!("| Métrica | Resultado | Mediana do canal ({tipo}, n={}) | Δ |\n|---|---|---|---|\n", med["n"]);
            for (k, label, f) in linhas {
                if *k == "continuaram" && get(c, k).is_none() {
                    o += &format!("| {label} | — | — | só no YouTube Studio (vf studio) |\n");
                    continue;
                }
                let val = get(c, k);
                o += &format!("| {} | {} | {} | {} |\n", esc_md(label), fmtv(val, *f), fmtv(get(med, k), *f), val.map(|x| delta(x, get(med, k))).unwrap_or("—".into()));
            }
        } else {
            o += "## Desempenho por janela\n\n| Métrica |";
            for c in &cols {
                o += &format!(" {}{} |", c["janela_nome"].as_str().unwrap_or(""), if c.get("parcial").is_some() { " (parcial)" } else { "" });
            }
            o += &format!(" Mediana {tipo} ({}) |\n|---|", rel["principal"].as_str().unwrap_or(""));
            o += &"---|".repeat(cols.len() + 1);
            o += "\n";
            let pj = rel["principal"].as_str().unwrap_or("");
            for (k, label, f) in linhas {
                o += &format!("| {} |", esc_md(label));
                for c in &cols {
                    o += &format!(" {} |", fmtv(get(c, k), *f));
                }
                o += &format!(" {} |\n", if *k == "continuaram" { "só no Studio".into() } else { fmtv(get(&rel["medianas"][pj], k), *f) });
            }
            o += "\nMedianas por janela (vídeos do mesmo tipo, completos):\n\n| Janela | n | Views | Intencionais | % vista | Retenção média |\n|---|---|---|---|---|---|\n";
            for c in &cols {
                let j = c["janela"].as_str().unwrap_or("");
                let m = &rel["medianas"][j];
                o += &format!("| {} | {} | {} | {} | {} | {} |\n", c["janela_nome"].as_str().unwrap_or(""), m["n"], fmtv(get(m, "views"), Fmt::Int), fmtv(get(m, "intencionais"), Fmt::Int), fmtv(get(m, "pct"), Fmt::Pct), fmtv(get(m, "retencao_media"), Fmt::Frac));
            }
        }
        // retenção
        let principal = cols.iter().find(|c| c["janela"] == rel["principal"]).cloned().unwrap_or(Value::Null);
        let ret = if principal["retencao"].is_object() { principal["retencao"].clone() } else { principal["m"]["retencao"].clone() };
        o += "\n## Retenção\n\n";
        if ret.is_object() {
            o += &format!(
                "Retenção média (curva): **{}** · desempenho relativo médio **{}** (0,50 = igual a vídeos de mesma duração no YouTube; acima é melhor).\n\n",
                fmtv(get(&ret, "media"), Fmt::Frac),
                fmtv(get(&ret, "relativa_media"), Fmt::Horas)
            );
            o += "| Ponto | Ainda assistindo |\n|---|---|\n";
            for m in ret["marcos"].as_array().cloned().unwrap_or_default() {
                o += &format!("| {} | {} |\n", m["ponto"].as_str().unwrap_or(""), fmtv(get(&m, "assistindo"), Fmt::Frac));
            }
            let quedas = ret["maiores_quedas"].as_array().cloned().unwrap_or_default();
            if !quedas.is_empty() {
                o += "\nMaiores quedas:\n\n| Em | Queda | Beat do roteiro |\n|---|---|---|\n";
                for q in quedas {
                    let s = num(&q, "s");
                    o += &format!("| {} | −{} p.p. | {} |\n", fmt_dur(s), fmt_dec(num(&q, "queda") * 100.0, 1), beat_em(&rel["beats"], s));
                }
            }
        } else if rel["plataforma"] == "tiktok" {
            o += &format!("Retenção média: **{}** · assistiram até o fim: **{}** (TikTok Business; \"—\" = sem conta Business ou dado ainda não disponível).\n", fmtv(get(&principal, "retencao_media"), Fmt::Frac), fmtv(get(&principal, "ate_o_fim"), Fmt::Pct));
        } else {
            o += "Sem curva de retenção (vídeo sem views na janela, ou a Analytics ainda não tem dados).\n";
        }
        // primeiras horas (série por hora do Studio)
        let ph = &rel["primeiras_horas"];
        if ph["cobre"] == true {
            let med = &rel["primeiras_horas_mediana"];
            let n = med["n"].as_u64().unwrap_or(0);
            o += &format!("\n## Primeiras horas (Studio)\n\nVisualizações acumuladas desde a publicação{}.\n\n| Desde a publicação | Views | Mediana {} (n={n}) | Δ |\n|---|---|---|---|\n",
                ph["pico_hora"].as_i64().map(|h| format!("; pico na {h}ª hora ({} views)", fmt_n(num(ph, "pico_views")))).unwrap_or_default(), rel["tipo_pares"].as_str().unwrap_or(""));
            for (k, nome) in [("h1", "1ª hora"), ("h3", "3 horas"), ("h6", "6 horas"), ("h12", "12 horas"), ("h24", "24 horas"), ("h48", "48 horas")] {
                let v = get(ph, k);
                if v.is_none() && get(med, k).is_none() {
                    continue;
                }
                o += &format!("| {nome} | {} | {} | {} |\n", fmtv(v, Fmt::Int), fmtv(get(med, k), Fmt::Int), v.map(|x| delta(x, get(med, k))).unwrap_or("—".into()));
            }
        }
        // tráfego
        let tr_studio = rel["studio"]["trafego"].as_array().filter(|a| !a.is_empty()).or(rel["studio"]["trafego_48h"].as_array().filter(|a| !a.is_empty()));
        if rel["trafego"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
            if let Some(tr) = tr_studio {
                let per = if rel["studio"]["trafego"].as_array().is_some_and(|a| !a.is_empty()) { "desde a publicação" } else { "últimas 48 h" };
                o += &format!("\n## Fontes de tráfego (Studio, {per})\n\n| Fonte | % |\n|---|---|\n");
                for x in tr {
                    o += &format!("| {} | {} |\n", esc_md(x[0].as_str().unwrap_or("")), fmtv(x[1].as_f64(), Fmt::Pct));
                }
            }
        }
        if let Some(tr) = rel["trafego"].as_array().filter(|a| !a.is_empty()) {
            o += "\n## Fontes de tráfego\n\n| Fonte | Views | % |\n|---|---|---|\n";
            if tr[0].get("insightTrafficSourceType").is_some() {
                let total: f64 = tr.iter().map(|x| num(x, "views")).sum();
                for x in tr {
                    let vv = num(x, "views");
                    o += &format!("| {} | {} | {} |\n", fonte(x["insightTrafficSourceType"].as_str().unwrap_or("")), fmt_n(vv), if total > 0.0 { fmt_pct(vv / total * 100.0) } else { "—".into() });
                }
            } else {
                // TikTok Business: [{impression_source, percentage}] (formato a confirmar; o bruto fica no JSON)
                for x in tr {
                    let nome = x.get("impression_source").or(x.get("source")).and_then(|v| v.as_str()).unwrap_or("?");
                    let pc = get(x, "percentage").map(|p| if p <= 1.0 { p * 100.0 } else { p });
                    o += &format!("| {} | — | {} |\n", esc_md(nome), pc.map(fmt_pct).unwrap_or("—".into()));
                }
            }
        }
        for (k, nome) in [("geografias", "Principais geografias"), ("demografia", "Principal grupo demográfico")] {
            if let Some(l) = rel["studio"][k].as_array().filter(|a| !a.is_empty()) {
                o += &format!("\n## {nome} (Studio)\n\n| | % |\n|---|---|\n");
                for x in l.iter().take(10) {
                    o += &format!("| {} | {} |\n", esc_md(x[0].as_str().unwrap_or("")), fmtv(x[1].as_f64(), Fmt::Pct));
                }
            }
        }
    } else {
        let titulo = format!("Relatório geral — {} · {}", rel["canal"].as_str().or(rel["conta"].as_str()).unwrap_or(""), plat);
        o += &format!("# {titulo}\n\nJanela: **{}** · gerado {} · {} · filtros: {}\n\n", rel["janela_nome"].as_str().unwrap_or(""), rel["gerado"].as_str().unwrap_or("").get(..10).unwrap_or(""), rel["conta"].as_str().unwrap_or(""), rel["filtros"].as_str().unwrap_or(""));
        let itens = rel["itens"].as_array().cloned().unwrap_or_default();
        // colunas que só existem com Studio aparecem só se algum vídeo tiver o dado
        let tem = |k: &str| itens.iter().any(|x| get(&x["m"], k).is_some());
        let mut cols: Vec<(String, String, Fmt)> = linhas.iter().filter(|l| !matches!(l.0, "continuaram" | "impressoes" | "ctr" | "alcance" | "ate_o_fim") || tem(l.0)).map(|l| (l.0.to_string(), l.1.to_string(), l.2)).collect();
        for e in rel["colunas_extra"].as_array().into_iter().flatten() {
            let k = e[0].as_str().unwrap_or("");
            if tem(k) {
                cols.push((k.to_string(), e[1].as_str().unwrap_or(k).to_string(), Fmt::Int));
            }
        }
        o += &format!("{} vídeo(s)\n\n| # | Vídeo | Tipo | Publicado |", itens.len());
        for (_, l, _) in &cols {
            o += &format!(" {} |", curto(l));
        }
        o += "\n|---|---|---|---|";
        o += &"---|".repeat(cols.len());
        o += "\n";
        for (i, x) in itens.iter().enumerate() {
            let t: String = x["titulo"].as_str().unwrap_or("").chars().take(48).collect();
            o += &format!("| {} | [{}]({}){} | {} | {} |", i + 1, esc_md(&t), x["url"].as_str().unwrap_or(""), if x["m"].get("parcial").is_some() { " *(parcial)*" } else { "" }, x["tipo"].as_str().unwrap_or(""), x["publicado"].as_str().unwrap_or("").get(..10).unwrap_or(""));
            for (k, _, f) in &cols {
                o += &format!(" {} |", fmtv(get(&x["m"], k), *f));
            }
            o += "\n";
        }
        o += "| | **Total** | | |";
        for (k, _, f) in &cols {
            o += &format!(" {} |", get(&rel["totais"], k).map(|v| fmtv(Some(v), *f)).unwrap_or_default());
        }
        o += "\n";
        for (nome, m) in rel["medianas"].as_object().cloned().unwrap_or_default() {
            if m["n"].as_u64().unwrap_or(0) == 0 {
                continue;
            }
            o += &format!("| | **Mediana {nome}** (n={}) | | |", m["n"]);
            for (k, _, f) in &cols {
                o += &format!(" {} |", fmtv(get(&m, k), *f));
            }
            o += "\n";
        }
        // canal no Studio (páginas de análise do canal capturadas pela extensão)
        if let Some(c) = rel["studio_canal"].as_object() {
            o += &format!("\n## Canal no Studio (capturado {})\n\n", c.get("atualizado").and_then(|x| x.as_str()).unwrap_or("").get(..16).unwrap_or("").replace('T', " "));
            for (k, nome, f) in [("continuaram", "Continuaram assistindo (Shorts)", Fmt::Pct), ("ignorado", "Ignorado (Shorts)", Fmt::Pct), ("impressoes", "Impressões", Fmt::Int), ("ctr", "CTR", Fmt::Pct)] {
                if let Some(x) = c.get("metricas").and_then(|m| m[k]["valor"].as_f64()) {
                    o += &format!("- **{nome}:** {}{}\n", fmtv(Some(x), f), c["metricas"][k]["periodo"].as_str().map(|p| format!(" ({p})")).unwrap_or_default());
                }
            }
            for card in c.get("cartoes").and_then(|x| x.as_array()).into_iter().flatten() {
                let titulo = card["titulo"].as_str().unwrap_or("");
                if titulo.is_empty() {
                    continue;
                }
                let lista: Vec<String> = card["barras"].as_array().filter(|b| b.iter().any(|x| x[0].as_str().is_some_and(|s| !s.is_empty()))).or(card["pares"].as_array()).into_iter().flatten().take(6)
                    .map(|x| format!("{} {}", x[0].as_str().unwrap_or(""), x[1].as_f64().map(|v| fmt_dec(v, 1)).unwrap_or_default())).collect();
                let valor = card["texto"].as_str().unwrap_or("");
                if lista.is_empty() && valor.is_empty() {
                    continue;
                }
                o += &format!("- **{}**{}: {}{}\n", esc_md(titulo), card["periodo"].as_str().map(|p| format!(" ({p})")).unwrap_or_default(), esc_md(valor), if lista.is_empty() { String::new() } else { format!(" — {}", esc_md(&lista.join(" · "))) });
            }
        }
        // melhores e piores em retenção
        let mut com_ret: Vec<&Value> = itens.iter().filter(|x| get(&x["m"], "retencao_media").is_some()).collect();
        if com_ret.len() >= 2 {
            com_ret.sort_by(|a, b| get(&b["m"], "retencao_media").partial_cmp(&get(&a["m"], "retencao_media")).unwrap_or(std::cmp::Ordering::Equal));
            let n = 5.min(com_ret.len() / 2).max(1);
            o += "\n## Retenção média: melhores e piores\n\n| | Vídeo | Retenção média | % vista | Views |\n|---|---|---|---|---|\n";
            for (lado, grupo) in [("melhor", com_ret[..n].to_vec()), ("pior", com_ret[com_ret.len() - n..].to_vec())] {
                for x in grupo {
                    o += &format!("| {lado} | {} | {} | {} | {} |\n", esc_md(&x["titulo"].as_str().unwrap_or("").chars().take(60).collect::<String>()), fmtv(get(&x["m"], "retencao_media"), Fmt::Frac), fmtv(get(&x["m"], "pct"), Fmt::Pct), fmtv(get(&x["m"], "views"), Fmt::Int));
                }
            }
        }
    }
    let avisos = rel["avisos"].as_array().cloned().unwrap_or_default();
    if !avisos.is_empty() {
        o += "\n## Notas\n\n";
        for a in avisos {
            o += &format!("- {}\n", a.as_str().unwrap_or(""));
        }
    }
    o
}

fn periodo(c: &Value) -> String {
    let p = c.get("parcial").and_then(|x| x.as_str()).map(|x| format!(", parcial: {x}")).unwrap_or_default();
    match (c["inicio"].as_str(), c["fim"].as_str()) {
        (Some(a), Some(b)) => format!(" ({a} → {b}{p})"),
        _ => String::new(),
    }
}

fn curto(l: &str) -> &str {
    match l {
        "Visualizações intencionais" => "Intencionais",
        "Duração média da visualização" | "Tempo médio assistido" => "Duração média",
        "Porcentagem visualizada média" => "% vista",
        "Retenção média (curva)" | "Retenção média (tempo médio ÷ duração)" => "Retenção média",
        "Taxa de cliques das impressões" => "CTR",
        "Continuaram assistindo" => "Continuaram",
        "Tempo de exibição (horas)" | "Tempo total assistido (horas)" => "Horas",
        "Marcações com \"Gostei\"" => "Gostei",
        "Inscritos gerados" => "Inscritos",
        "Comentários adicionados" => "Comentários",
        "Espectadores alcançados" => "Alcance",
        "Engajamento ((curt.+coment.+compart.) ÷ views)" => "Engajamento",
        x => x,
    }
}

fn fonte(k: &str) -> String {
    match k {
        "SHORTS" => "Feed do Shorts",
        "YT_SEARCH" => "Pesquisa do YouTube",
        "RELATED_VIDEO" => "Vídeos sugeridos",
        "SUBSCRIBER" => "Inscrições / página inicial de inscritos",
        "YT_CHANNEL" => "Páginas do canal",
        "BROWSE" | "YT_OTHER_PAGE" => "Recursos de navegação",
        "EXT_URL" => "Externo",
        "NO_LINK_OTHER" | "NO_LINK_EMBEDDED" => "Direto ou desconhecido",
        "PLAYLIST" | "YT_PLAYLIST_PAGE" => "Playlists",
        "NOTIFICATION" => "Notificações",
        "END_SCREEN" => "Telas finais",
        "ANNOTATION" | "CAMPAIGN_CARD" => "Cards",
        "HASHTAGS" => "Hashtags",
        "SOUND_PAGE" => "Páginas de som",
        "ADVERTISING" => "Publicidade",
        x => return x.to_string(),
    }
    .to_string()
}

fn beat_em(beats: &Value, s: f64) -> String {
    let lista = beats.get("beats").or(Some(beats)).and_then(|b| b.as_array()).cloned().unwrap_or_default();
    lista
        .iter()
        .find(|b| matches!((get(b, "start"), get(b, "end")), (Some(a), Some(e)) if a <= s && s <= e + 0.25))
        .map(|b| format!("beat {} ({}): {}", b["id"], b["tipo"].as_str().unwrap_or(""), esc_md(&b.get("texto").or(b.get("text")).and_then(|x| x.as_str()).unwrap_or("").chars().take(80).collect::<String>())))
        .unwrap_or_else(|| "—".into())
}

// ---------------------------------------------------------------- HTML

/// página única, sem dependências: tabelas + curva de retenção (individual) ou barras de retenção (geral).
/// Uma série só por gráfico (azul do slot 1), modo escuro selecionado, tooltip no hover, tabela como visão acessível.
pub fn pagina(rel: &Value) -> String {
    let md = markdown(rel);
    let dados = serde_json::to_string(rel).unwrap_or_default().replace("</", "<\\/");
    let titulo = md.lines().next().unwrap_or("Relatório").trim_start_matches("# ").to_string();
    // gráficos logo depois do cabeçalho (título + linha de identificação)
    let mut corpo = md_para_html(&md);
    let pos = corpo.find("</p>").map(|p| p + 4).unwrap_or(0);
    corpo.insert_str(pos, "\n<div id=\"graf\"></div>\n");
    format!(
        r##"<!DOCTYPE html>
<html lang="pt-BR"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{t}</title>
<style>
.viz-root{{color-scheme:light;--surface-1:#fcfcfb;--surface-2:#f3f2ef;--text-primary:#0b0b0b;--text-secondary:#52514e;--text-muted:#76756f;--grid:#e4e3de;--series-1:#2a78d6;--series-1-soft:#cde2fb}}
@media (prefers-color-scheme:dark){{:root:where(:not([data-theme="light"])) .viz-root{{color-scheme:dark;--surface-1:#1a1a19;--surface-2:#242423;--text-primary:#f0efec;--text-secondary:#c3c2b7;--text-muted:#9a998f;--grid:#33332f;--series-1:#3987e5;--series-1-soft:#1c3556}}}}
:root[data-theme="dark"] .viz-root{{color-scheme:dark;--surface-1:#1a1a19;--surface-2:#242423;--text-primary:#f0efec;--text-secondary:#c3c2b7;--text-muted:#9a998f;--grid:#33332f;--series-1:#3987e5;--series-1-soft:#1c3556}}
html,body{{margin:0}}
.viz-root{{background:var(--surface-1);color:var(--text-primary);font:15px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif;min-height:100vh}}
main{{max-width:1180px;margin:0 auto;padding:32px 24px 64px}}
h1{{font-size:24px;margin:0 0 8px}} h2{{font-size:18px;margin:32px 0 12px}}
p{{color:var(--text-secondary);margin:8px 0}} a{{color:var(--series-1)}}
.tw{{overflow-x:auto}} table{{border-collapse:collapse;width:100%;font-variant-numeric:tabular-nums;font-size:14px}}
th,td{{text-align:left;padding:7px 10px;border-bottom:1px solid var(--grid);white-space:nowrap}}
th{{color:var(--text-secondary);font-weight:600;cursor:pointer;user-select:none;position:sticky;top:0;background:var(--surface-1)}}
td:not(:first-child),th:not(:first-child){{text-align:right}}
.lista td:nth-child(2),.lista th:nth-child(2){{text-align:left;white-space:normal;min-width:220px}}
tbody tr:hover{{background:var(--surface-2)}}
.chart{{position:relative;margin:8px 0 24px}} .chart svg{{display:block;width:100%;height:auto;overflow:visible}}
.chart text{{fill:var(--text-muted);font-size:12px}} .axis line,.grid line{{stroke:var(--grid)}}
.tip{{position:absolute;pointer-events:none;background:var(--surface-2);color:var(--text-primary);border:1px solid var(--grid);border-radius:6px;padding:6px 9px;font-size:13px;white-space:nowrap;display:none;box-shadow:0 2px 8px rgba(0,0,0,.12)}}
.tip b{{font-weight:650}} .tip span{{color:var(--text-secondary)}}
ul{{color:var(--text-secondary)}} code{{font-size:13px}}
</style></head>
<body><div class="viz-root"><main>
{corpo}
</main></div>
<script type="application/json" id="dados">{dados}</script>
<script>
(function(){{
const R=JSON.parse(document.getElementById('dados').textContent);
const NS='http://www.w3.org/2000/svg';
const pct=v=>(v*100).toLocaleString('pt-BR',{{maximumFractionDigits:1}})+'%';
const seg=s=>{{s=Math.round(s);return s<60?s+' s':Math.floor(s/60)+':'+String(s%60).padStart(2,'0')}};
function el(n,a,p){{const e=document.createElementNS(NS,n);for(const k in a)e.setAttribute(k,a[k]);if(p)p.appendChild(e);return e}}
function tipBox(wrap){{const t=document.createElement('div');t.className='tip';wrap.appendChild(t);return t}}
function fill(t,valor,rotulo){{t.textContent='';const b=document.createElement('b');b.textContent=valor;const s=document.createElement('span');s.textContent=' '+rotulo;t.append(b,s)}}
const g=document.getElementById('graf');
// curva de retenção (individual)
let ret=null;
if(R.tipo_relatorio==='video'){{const c=(R.janelas||[]).find(j=>j.janela===R.principal)||{{}};ret=c.retencao||(c.m&&c.m.retencao)||null}}
if(ret&&ret.curva&&ret.curva.length>1){{
  const h=document.createElement('h2');h.textContent='Curva de retenção';g.appendChild(h);
  const wrap=document.createElement('div');wrap.className='chart';g.appendChild(wrap);
  const W=900,H=300,L=48,B=28,T=10,Rr=12,pts=ret.curva;
  const maxY=Math.max(1,...pts.map(p=>p.assistindo))*1.05,maxX=pts[pts.length-1].s||1;
  const x=s=>L+(W-L-Rr)*s/maxX,y=v=>T+(H-T-B)*(1-v/maxY);
  const svg=el('svg',{{viewBox:`0 0 ${{W}} ${{H}}`,role:'img','aria-label':'Curva de retenção: fração da audiência ainda assistindo em cada ponto do vídeo'}},wrap);
  const gr=el('g',{{class:'grid'}},svg);
  for(let k=0;k<=4;k++){{const v=maxY*k/4;el('line',{{x1:L,x2:W-Rr,y1:y(v),y2:y(v)}},gr);const tx=el('text',{{x:L-8,y:y(v)+4,'text-anchor':'end'}},svg);tx.textContent=pct(v)}}
  for(let k=0;k<=5;k++){{const s=maxX*k/5;const tx=el('text',{{x:x(s),y:H-8,'text-anchor':'middle'}},svg);tx.textContent=seg(s)}}
  const d=pts.map((p,i)=>(i?'L':'M')+x(p.s).toFixed(1)+' '+y(p.assistindo).toFixed(1)).join(' ');
  el('path',{{d:d+` L${{x(maxX)}} ${{y(0)}} L${{x(0)}} ${{y(0)}} Z`,fill:'var(--series-1-soft)',opacity:.6}},svg);
  el('path',{{d,fill:'none',stroke:'var(--series-1)','stroke-width':2,'stroke-linejoin':'round'}},svg);
  const hair=el('line',{{y1:T,y2:H-B,stroke:'var(--text-muted)','stroke-width':1,visibility:'hidden'}},svg);
  const dot=el('circle',{{r:5,fill:'var(--series-1)',stroke:'var(--surface-1)','stroke-width':2,visibility:'hidden'}},svg);
  const tip=tipBox(wrap);
  const hit=el('rect',{{x:L,y:T,width:W-L-Rr,height:H-T-B,fill:'transparent'}},svg);
  hit.addEventListener('pointermove',ev=>{{const r=svg.getBoundingClientRect();const sx=(ev.clientX-r.left)*W/r.width;const s=(sx-L)/(W-L-Rr)*maxX;
    let best=pts[0];for(const p of pts)if(Math.abs(p.s-s)<Math.abs(best.s-s))best=p;
    hair.setAttribute('x1',x(best.s));hair.setAttribute('x2',x(best.s));dot.setAttribute('cx',x(best.s));dot.setAttribute('cy',y(best.assistindo));
    hair.setAttribute('visibility','visible');dot.setAttribute('visibility','visible');
    fill(tip,pct(best.assistindo),'ainda assistindo em '+seg(best.s)+(best.relativa!=null?' · relativa '+best.relativa.toLocaleString('pt-BR',{{maximumFractionDigits:2}}):''));
    tip.style.display='block';const px=x(best.s)*r.width/W,py=y(best.assistindo)*r.height/H;tip.style.left=Math.min(px+12,r.width-tip.offsetWidth)+'px';tip.style.top=Math.max(0,py-40)+'px'}});
  hit.addEventListener('pointerleave',()=>{{tip.style.display='none';hair.setAttribute('visibility','hidden');dot.setAttribute('visibility','hidden')}});
}}
// visualizações por hora desde a publicação (Studio, individual)
const ph=R.primeiras_horas&&R.primeiras_horas.serie;
if(ph&&ph.length>1){{
  const h=document.createElement('h2');h.textContent='Visualizações por hora desde a publicação (Studio)';g.appendChild(h);
  const wrap=document.createElement('div');wrap.className='chart';g.appendChild(wrap);
  const W=900,H=220,L=48,B=28,T=10,Rr=12,n=ph.length,maxV=Math.max(1,...ph.map(p=>p[1]));
  // escala fixa de no mínimo 24 h: vídeos diferentes ficam comparáveis a olho
  const bw=(W-L-Rr)/Math.max(n,24),y=v=>T+(H-T-B)*(1-v/maxV);
  const svg=el('svg',{{viewBox:`0 0 ${{W}} ${{H}}`,role:'img','aria-label':'Visualizações em cada hora desde a publicação'}},wrap);
  const gr=el('g',{{class:'grid'}},svg);
  for(let k=0;k<=3;k++){{const v=maxV*k/3;el('line',{{x1:L,x2:W-Rr,y1:y(v),y2:y(v)}},gr);const tx=el('text',{{x:L-8,y:y(v)+4,'text-anchor':'end'}},svg);tx.textContent=Math.round(v).toLocaleString('pt-BR')}}
  const tip=tipBox(wrap);
  ph.forEach((p,k)=>{{const x=L+k*bw,hgt=Math.max(1,y(0)-y(p[1]));
    el('rect',{{x:x+1,y:y(0)-hgt,width:Math.max(1,bw-2),height:hgt,rx:Math.min(4,bw/3),fill:'var(--series-1)'}},svg);
    if(n<=24||k%Math.ceil(n/12)===0){{const t=el('text',{{x:x+bw/2,y:H-8,'text-anchor':'middle'}},svg);t.textContent=p[0]+'h'}}
    const hit=el('rect',{{x,y:T,width:bw,height:H-T-B,fill:'transparent'}},svg);
    hit.addEventListener('pointermove',ev=>{{const r=wrap.getBoundingClientRect();fill(tip,p[1].toLocaleString('pt-BR')+' views',p[0]+'ª hora desde a publicação');
      tip.style.display='block';tip.style.left=Math.min(ev.clientX-r.left+12,r.width-tip.offsetWidth)+'px';tip.style.top=(ev.clientY-r.top-36)+'px'}});
    hit.addEventListener('pointerleave',()=>{{tip.style.display='none'}});
  }});
}}
// barras de retenção média por vídeo (geral)
if(R.tipo_relatorio==='geral'){{
  const its=(R.itens||[]).filter(i=>i.m&&i.m.retencao_media!=null).sort((a,b)=>b.m.retencao_media-a.m.retencao_media).slice(0,30);
  if(its.length){{
    const h=document.createElement('h2');h.textContent='Retenção média por vídeo'+(its.length===30?' (30 primeiros)':'');g.appendChild(h);
    const wrap=document.createElement('div');wrap.className='chart';g.appendChild(wrap);
    const W=900,row=24,L=300,Rr=70,H=its.length*row+8,maxV=Math.max(...its.map(i=>i.m.retencao_media),0.01);
    const svg=el('svg',{{viewBox:`0 0 ${{W}} ${{H}}`,role:'img','aria-label':'Retenção média de cada vídeo, do maior para o menor'}},wrap);
    const tip=tipBox(wrap);
    its.forEach((it,k)=>{{const yy=k*row+4,w=Math.max(2,(W-L-Rr)*it.m.retencao_media/maxV);
      const lb=el('text',{{x:L-10,y:yy+15,'text-anchor':'end'}},svg);const tt=(it.titulo||it.id||'');lb.textContent=tt.length>42?tt.slice(0,41)+'…':tt;
      const bar=el('rect',{{x:L,y:yy+2,width:w,height:row-8,rx:4,fill:'var(--series-1)'}},svg);
      const v=el('text',{{x:L+w+6,y:yy+15}},svg);v.textContent=pct(it.m.retencao_media);
      const hit=el('rect',{{x:0,y:yy,width:W,height:row,fill:'transparent'}},svg);
      hit.addEventListener('pointermove',ev=>{{const r=wrap.getBoundingClientRect();fill(tip,pct(it.m.retencao_media),'retenção média · '+(it.m.views||0).toLocaleString('pt-BR')+' views · '+tt);
        tip.style.display='block';tip.style.left=Math.min(ev.clientX-r.left+12,r.width-tip.offsetWidth)+'px';tip.style.top=(ev.clientY-r.top-36)+'px'}});
      hit.addEventListener('pointerleave',()=>{{tip.style.display='none'}});
    }});
  }}
}}
// tabelas ordenáveis (clique no cabeçalho)
const num=s=>{{const t=s.replace(/[^0-9,−+\-:]/g,'');if(t.includes(':')){{return t.split(':').reduce((a,b)=>a*60+(+b||0),0)}}const n=parseFloat(t.replace(/\./g,'').replace(',','.').replace('−','-'));return isNaN(n)?null:n}};
document.querySelectorAll('table').forEach(tb=>{{const ths=tb.querySelectorAll('th');ths.forEach((th,ci)=>{{let asc=false;th.title='ordenar';th.addEventListener('click',()=>{{
  const body=tb.tBodies[0];const rows=[...body.rows];const fixos=rows.filter(r=>/Total|Mediana/.test(r.cells[1]?r.cells[1].textContent:''));const mov=rows.filter(r=>!fixos.includes(r));
  asc=!asc;mov.sort((a,b)=>{{const x=a.cells[ci].textContent,y=b.cells[ci].textContent,nx=num(x),ny=num(y);const c=(nx!=null&&ny!=null)?nx-ny:x.localeCompare(y,'pt-BR');return asc?c:-c}});
  [...mov,...fixos].forEach(r=>body.appendChild(r))}})}})}});
}})();
</script></body></html>
"##,
        t = esc_html(&titulo),
        corpo = corpo,
        dados = dados
    )
}

/// Markdown mínimo do próprio relatório (títulos, parágrafos, tabelas, listas, links, negrito) -> HTML
fn md_para_html(md: &str) -> String {
    let inline = |s: &str| -> String {
        let mut o = esc_html(s);
        // [texto](url)
        while let (Some(a), Some(m)) = (o.find('['), o.find("](")) {
            let Some(e) = o[m..].find(')').map(|x| x + m) else { break };
            if a > m {
                break;
            }
            let txt = o[a + 1..m].to_string();
            let url = o[m + 2..e].to_string();
            o = format!("{}<a href=\"{}\">{}</a>{}", &o[..a], url, txt, &o[e + 1..]);
        }
        let mut out = String::new();
        let mut on = false;
        for (i, part) in o.split("**").enumerate() {
            if i > 0 {
                out += if on { "</strong>" } else { "<strong>" };
                on = !on;
            }
            out += part;
        }
        out.replace("\\|", "|")
    };
    let mut o = String::new();
    let mut lines = md.lines().peekable();
    while let Some(l) = lines.next() {
        if let Some(h) = l.strip_prefix("## ") {
            o += &format!("<h2>{}</h2>\n", inline(h));
        } else if let Some(h) = l.strip_prefix("# ") {
            o += &format!("<h1>{}</h1>\n", inline(h));
        } else if l.starts_with('|') {
            let cells = |r: &str| -> Vec<String> {
                let r = r.trim().trim_start_matches('|').trim_end_matches('|');
                let mut v = Vec::new();
                let mut cur = String::new();
                let mut prev = ' ';
                for c in r.chars() {
                    if c == '|' && prev != '\\' {
                        v.push(cur.trim().to_string());
                        cur.clear();
                    } else {
                        cur.push(c);
                    }
                    prev = c;
                }
                v.push(cur.trim().to_string());
                v
            };
            let cab = cells(l);
            let lista = cab.get(1).map(|c| c == "Vídeo").unwrap_or(false);
            o += if lista { "<div class=\"tw\"><table class=\"lista\"><thead><tr>" } else { "<div class=\"tw\"><table><thead><tr>" };
            for c in cab {
                o += &format!("<th>{}</th>", inline(&c));
            }
            o += "</tr></thead><tbody>\n";
            if lines.peek().map(|x| x.starts_with("|---")).unwrap_or(false) {
                lines.next();
            }
            while lines.peek().map(|x| x.starts_with('|')).unwrap_or(false) {
                o += "<tr>";
                for c in cells(lines.next().unwrap()) {
                    o += &format!("<td>{}</td>", inline(&c));
                }
                o += "</tr>\n";
            }
            o += "</tbody></table></div>\n";
        } else if let Some(it) = l.strip_prefix("- ") {
            o += "<ul>";
            o += &format!("<li>{}</li>", inline(it));
            while lines.peek().map(|x| x.starts_with("- ")).unwrap_or(false) {
                o += &format!("<li>{}</li>", inline(&lines.next().unwrap()[2..]));
            }
            o += "</ul>\n";
        } else if !l.trim().is_empty() {
            o += &format!("<p>{}</p>\n", inline(l));
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formatos() {
        assert_eq!(fmt_n(1234567.0), "1.234.567");
        assert_eq!(fmt_n(365.0), "365");
        assert_eq!(fmt_pct(63.24), "63,24%");
        assert_eq!(fmt_dec(2.3412, 2), "2,34");
        assert_eq!(fmt_dur(42.0), "42 segundos");
        assert_eq!(fmt_dur(192.0), "3:12");
        assert_eq!(fmt_dur(3725.0), "1:02:05");
        assert_eq!(fmt_subs(1.0), "+1");
        assert_eq!(fmt_subs(-2.0), "−2");
        assert_eq!(delta(110.0, Some(100.0)), "+10%");
        assert_eq!(delta(89.0, Some(100.0)), "−11%");
        assert_eq!(delta(1.0, None), "—");
    }
    #[test]
    fn janelas() {
        let pubd = tempo::parse_iso("2026-11-10T15:00:00Z").unwrap().unwrap();
        let agora = tempo::parse_iso("2026-11-14T12:00:00Z").unwrap().unwrap();
        assert_eq!(Janela::Dias(1).faixa(pubd, agora), ("2026-11-10".into(), "2026-11-10".into(), None));
        assert_eq!(Janela::Dias(3).faixa(pubd, agora), ("2026-11-10".into(), "2026-11-12".into(), None));
        assert_eq!(Janela::Dias(7).faixa(pubd, agora), ("2026-11-10".into(), "2026-11-14".into(), Some((5, 7))));
        assert_eq!(Janela::parse("todas").unwrap().len(), 5);
        assert_eq!(Janela::parse("2026-11-01..2026-11-15").unwrap()[0], Janela::Faixa("2026-11-01".into(), "2026-11-15".into()));
        assert!(Janela::parse("d0").is_err() && Janela::parse("xx").is_err());
    }
    #[test]
    fn curva() {
        let c: Vec<(f64, f64, f64)> = (0..=100).map(|i| (i as f64 / 100.0, 1.0 - i as f64 / 200.0 - if i > 20 { 0.2 } else { 0.0 }, 0.5)).collect();
        let r = retencao(&c, 60.0);
        // média de 1 − i/200 (0,75) menos 0,2 em 80 de 101 pontos
        assert!((r["media"].as_f64().unwrap() - 0.5916).abs() < 0.005, "{}", r["media"]);
        assert_eq!(r["maiores_quedas"][0]["s"].as_f64().unwrap(), 12.6);
        assert_eq!(r["marcos"][0]["ponto"], "3 s");
        assert!(r["marcos"][0]["assistindo"].as_f64().unwrap() > 0.97);
        assert_eq!(r["maiores_quedas"].as_array().unwrap().len(), 1, "só a queda real de 20 p.p.");
    }
    fn rel_exemplo() -> Value {
        let curva: Vec<(f64, f64, f64)> = (0..=100).map(|i| (i as f64 / 100.0, 1.0 - i as f64 / 250.0 - if i > 30 { 0.1 } else { 0.0 }, 0.55)).collect();
        let ret = retencao(&curva, 66.0);
        json!({
            "tipo_relatorio": "video", "plataforma": "yt", "conta": "canal", "canal": "Canal <Teste>", "principal": "d7", "tipo_pares": "short", "pares": 3,
            "video": { "id": "ABC", "titulo": "Por que | isso", "url": "https://youtu.be/ABC", "tipo": "short", "publicado": "2026-11-10T15:00:00Z", "duracao_s": 66.0 },
            "janelas": [{ "janela": "d7", "janela_nome": "7 primeiros dias", "inicio": "2026-11-10", "fim": "2026-11-16", "views": 365.0, "intencionais": 164.0, "avd": 42.0, "pct": 63.24, "horas": 2.34, "likes": 8.0, "inscritos": 1.0, "comentarios": 1.0, "shares": 0.0, "retencao_media": ret["media"], "retencao": ret }],
            "medianas": { "d7": { "n": 3, "views": 300.0, "intencionais": 150.0, "pct": 60.0 } },
            "beats": { "beats": [{ "id": 1, "tipo": "gancho", "texto": "Gancho", "start": 0.0, "end": 100.0 }] },
            "trafego": [{ "insightTrafficSourceType": "SHORTS", "views": 300.0 }, { "insightTrafficSourceType": "YT_SEARCH", "views": 65.0 }],
            "avisos": ["nota"]
        })
    }
    #[test]
    fn markdown_individual() {
        let md = markdown(&rel_exemplo());
        assert!(md.contains("| Visualizações | 365 | 300 | +22% |"), "{md}");
        assert!(md.contains("| Visualizações intencionais | 164 | 150 | +9% |"));
        assert!(md.contains("| Continuaram assistindo | — | — | só no YouTube Studio (vf studio) |"), "{md}");
        // com studio=1 e métrica mapeada, a linha ganha o número
        let mut r = rel_exemplo();
        r["janelas"][0]["continuaram"] = json!(49.55);
        assert!(markdown(&r).contains("| Continuaram assistindo | 49,55% |"));
        assert!(md.contains("| Duração média da visualização | 42 segundos |"));
        assert!(md.contains("| Porcentagem visualizada média | 63,24% | 60,00% | +5% |"));
        assert!(md.contains("| Tempo de exibição (horas) | 2,34 |"));
        assert!(md.contains("| Inscritos gerados | +1 |"));
        assert!(md.contains("Por que \\| isso") && md.contains("Feed do Shorts") && md.contains("beat 1 (gancho)"));
    }
    #[test]
    fn html_seguro() {
        let h = pagina(&rel_exemplo());
        assert!(h.starts_with("<!DOCTYPE html>") && h.contains("<table>") && h.contains("Curva de retenção"));
        assert!(h.find("<h1>").unwrap() < h.find("id=\"graf\"").unwrap(), "gráfico depois do título");
        // no corpo o texto sai escapado; no JSON embutido só "</" importa (vira "<\/")
        let corpo = &h[..h.find("<script type=\"application/json\"").unwrap()];
        assert!(!corpo.contains("Canal <Teste>") && corpo.contains("Canal &lt;Teste&gt;"));
        let mut r = rel_exemplo();
        r["video"]["titulo"] = json!("x</script><b>");
        let h2 = pagina(&r);
        assert_eq!(h2.matches("</script>").count(), 2, "o JSON não pode fechar o <script>");
        assert!(h.contains("<a href=\"https://youtu.be/ABC\">") || h.contains("youtu.be/ABC"));
    }
    #[test]
    fn filtros_e_ordem() {
        let kv = Kv::parse(&["tipo=short".into(), "busca=brasil".into(), "desde=2026-11-01".into(), "ordenar=retencao".into()], &[]).unwrap();
        let f = Filtros::de(&kv).unwrap();
        let v = |t: &str, tipo: &str, d: &str| json!({ "titulo": t, "tipo": tipo, "publicado": d, "privacidade": "public" });
        assert!(f.passa(&v("História do Brasil", "short", "2026-11-05T10:00:00Z")));
        assert!(!f.passa(&v("História do Brasil", "longo", "2026-11-05T10:00:00Z")));
        assert!(!f.passa(&v("Roma", "short", "2026-11-05T10:00:00Z")));
        assert!(!f.passa(&v("Brasil", "short", "2026-10-05T10:00:00Z")));
        let mut its = vec![json!({"m": {"retencao_media": 0.4}}), json!({"m": {"retencao_media": 0.9}}), json!({"m": {}})];
        ordenar(&mut its, f.chave(), false);
        assert_eq!(its[0]["m"]["retencao_media"], 0.9);
        assert!(its[2]["m"].get("retencao_media").is_none());
    }
}

#[cfg(test)]
mod demo {
    use super::*;
    /// VF_DEMO_OUT=<pasta> cargo test demo_paginas -- --ignored : grava exemplos de MD/HTML para conferência visual
    #[test]
    #[ignore]
    fn demo_paginas() {
        let dir = PathBuf::from(std::env::var("VF_DEMO_OUT").expect("VF_DEMO_OUT"));
        std::fs::create_dir_all(&dir).unwrap();
        let curva: Vec<(f64, f64, f64)> = (0..=100).map(|i| { let x = i as f64 / 100.0; (x, (1.0 - 0.35 * x - if x > 0.08 { 0.12 } else { 0.0 } - if x > 0.55 { 0.08 } else { 0.0 }).max(0.0), 0.45 + 0.1 * x) }).collect();
        let ret = retencao(&curva, 58.0);
        let col = |j: &str, nome: &str, v: f64| json!({ "janela": j, "janela_nome": nome, "inicio": "2026-11-10", "fim": "2026-11-16", "views": v, "intencionais": v * 0.45, "avd": 42.0, "pct": 63.24, "horas": v * 42.0 / 3600.0, "likes": (v / 45.0).round(), "inscritos": 1.0, "comentarios": 1.0, "shares": 2.0, "retencao_media": ret["media"], "retencao": ret });
        let ind = json!({
            "tipo_relatorio": "video", "plataforma": "yt", "conta": "history-in-minutes", "canal": "History in Minutes", "principal": "total", "tipo_pares": "short", "pares": 12,
            "video": { "id": "dQw4w9WgXcQ", "titulo": "O império que acabou numa manhã", "url": "https://youtu.be/dQw4w9WgXcQ", "tipo": "short", "publicado": "2026-11-10T15:00:00Z", "duracao_s": 58.0 },
            "janelas": [col("d1", "primeiro dia", 120.0), col("d3", "3 primeiros dias", 260.0), col("d7", "7 primeiros dias", 365.0), col("d28", "28 primeiros dias", 410.0), col("total", "desde a publicação", 418.0)],
            "medianas": { "d1": {"n": 12, "views": 90.0, "intencionais": 40.0, "pct": 58.0, "retencao_media": 0.55}, "d3": {"n": 12, "views": 200.0}, "d7": {"n": 11, "views": 300.0}, "d28": {"n": 9, "views": 380.0}, "total": {"n": 12, "views": 390.0, "intencionais": 170.0, "avd": 38.0, "pct": 59.1, "retencao_media": 0.58, "horas": 3.1, "likes": 7.0, "inscritos": 1.0, "comentarios": 1.0, "shares": 1.0} },
            "trafego": [{ "insightTrafficSourceType": "SHORTS", "views": 340.0 }, { "insightTrafficSourceType": "YT_SEARCH", "views": 50.0 }, { "insightTrafficSourceType": "SUBSCRIBER", "views": 28.0 }],
            "avisos": ["\"Continuaram assistindo\" só existe no YouTube Studio: a API pública não fornece."]
        });
        let itens: Vec<Value> = (0..14).map(|k| {
            let v = 1200.0 / (k as f64 + 1.0);
            json!({ "id": format!("v{k}"), "titulo": format!("Vídeo de teste número {k} com um título razoavelmente comprido"), "tipo": if k % 3 == 0 { "longo" } else { "short" }, "publicado": "2026-11-01T10:00:00Z", "url": "https://youtu.be/x",
                "m": { "views": v, "intencionais": v * 0.5, "avd": 30.0 + k as f64, "pct": 50.0 + k as f64, "retencao_media": 0.35 + (k as f64 * 0.37) % 0.5, "horas": v / 100.0, "likes": v / 40.0, "inscritos": 1.0, "comentarios": 2.0, "shares": 3.0 } })
        }).collect();
        let ger = json!({ "tipo_relatorio": "geral", "plataforma": "yt", "conta": "history-in-minutes", "canal": "History in Minutes", "gerado": "2026-11-17T20:00:00Z", "janela": "d7", "janela_nome": "7 primeiros dias", "filtros": "tipo=short · ordenar=views ↓", "itens": itens, "totais": { "views": 4000.0 }, "medianas": { "short": { "n": 9, "views": 150.0, "pct": 57.0, "retencao_media": 0.5 } }, "avisos": [] });
        for (n, r) in [("individual", ind), ("geral", ger)] {
            std::fs::write(dir.join(format!("{n}.md")), markdown(&r)).unwrap();
            std::fs::write(dir.join(format!("{n}.html")), pagina(&r)).unwrap();
        }
    }
}
