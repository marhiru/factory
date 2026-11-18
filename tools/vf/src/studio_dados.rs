//! Resumo limpo das capturas do Studio (extensão vf studio): um valor atual por cartão, identificado pelo
//! accessibilityId do cartão (não pelo texto, que muda com o idioma do Studio), com a hora da captura, mais a
//! série de visualizações por hora/minuto juntada de todas as capturas (cada uma traz as últimas ~48 h).
//!
//! Estado "Não existem dados suficientes" vira ausência de valor, nunca zero.
use crate::studio::{dados_dir, numero_tela};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

/// cartão do hub de análises, já achatado
#[derive(Debug, Clone)]
pub struct Cartao {
    pub id: String,
    pub quando: String,
    pub textos: Vec<String>,
    /// barras: (rótulo, fração 0-1)
    pub barras: Vec<(String, f64)>,
}

fn limpa(s: &str) -> String {
    s.replace(['\u{200e}', '\u{200f}', '\u{2068}', '\u{2069}'], "").replace(['\u{2009}', '\u{a0}', '\u{202f}'], " ").trim().to_string()
}

const SEM_DADOS: &[&str] = &["não existem dados suficientes", "not enough data", "no hay suficientes datos", "dados insuficientes"];

pub fn cartoes(v: &Value, quando: &str, out: &mut Vec<Cartao>) {
    match v {
        Value::Object(m) => {
            if let Some(cv) = m.get("cardViewModel").filter(|c| c.get("accessibilityId").is_some()) {
                let id = cv["accessibilityId"].as_str().unwrap_or("").to_string();
                let mut c = Cartao { id, quando: quando.to_string(), textos: Vec::new(), barras: Vec::new() };
                achatar(cv, &mut c);
                out.push(c);
            }
            m.values().for_each(|x| cartoes(x, quando, out));
        }
        Value::Array(a) => a.iter().for_each(|x| cartoes(x, quando, out)),
        _ => {}
    }
}

fn achatar(v: &Value, c: &mut Cartao) {
    match v {
        Value::Object(m) => {
            if let Some(b) = m.get("horizontalBarViewModel") {
                let segs = b["segments"].as_array().cloned().unwrap_or_default();
                let rotulo = b["accessibilityLabel"].as_str().or(segs.first().and_then(|s| s["label"].as_str())).unwrap_or("");
                if let Some(p) = segs.first().and_then(|s| s["percentage"].as_f64()) {
                    c.barras.push((limpa(rotulo), p));
                    // barra de dois lados sem rótulo (ex.: continuaram × ignorado): guarda o segundo lado também
                    if rotulo.is_empty() {
                        if let Some(p2) = segs.get(1).and_then(|s| s["percentage"].as_f64()) {
                            c.barras.push((String::new(), p2));
                        }
                    }
                }
                return;
            }
            for (k, x) in m {
                if matches!(k.as_str(), "styleRuns" | "commandRuns" | "loggingDirectives" | "rendererContext" | "onTap" | "accessibility" | "accessibilityLabel" | "trackingParams") {
                    continue;
                }
                if k == "content" {
                    if let Some(t) = x.as_str() {
                        let t = limpa(t);
                        if !t.is_empty() {
                            c.textos.push(t);
                        }
                        continue;
                    }
                }
                achatar(x, c);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| achatar(x, c)),
        _ => {}
    }
}

/// "0:42 (63%)" -> (42, Some(63))
fn valor_e_pct(t: &str) -> Option<(f64, Option<f64>)> {
    if let Some((a, b)) = t.split_once(" (") {
        let p = numero_tela(b.trim_end_matches(')'));
        return numero_tela(a).map(|x| (x, p));
    }
    numero_tela(t).map(|x| (x, None))
}

/// "Desde a publicação", "Últimos 28 dias", "Visualizações • 48 horas", "60 minutos" (não "Tempo de visualização (horas)")
fn eh_periodo(t: &str) -> bool {
    let l = t.to_lowercase();
    if ["desde", "since", "últimos", "ultimos", "last "].iter().any(|p| l.starts_with(p)) || l.contains(" • ") {
        return true;
    }
    let w: Vec<&str> = l.split_whitespace().collect();
    w.len() == 2 && w[0].chars().all(|c| c.is_ascii_digit()) && ["horas", "hours", "minutos", "minutes", "dias", "days"].contains(&w[1])
}

/// chave curta do cartão: sem prefixos do hub e sem ".updating_live"
pub fn chave(id: &str) -> String {
    let mut k = id.trim_end_matches(".updating_live").to_string();
    for p in ["analytics_hub.", "FEanalytics_hub_video_overview.", "FEanalytics_hub_channel_overview.", "FEanalytics_hub_content.", "FEanalytics_hub_"] {
        if let Some(r) = k.strip_prefix(p) {
            k = r.to_string();
        }
    }
    k.replace("video_overview_", "").replace("VideoOverview", "")
}

/// interpreta um cartão: {titulo, valor, valor_pct, periodo, pares[[rótulo, valor]], barras[[rótulo, %]], sem_dados}
pub fn interpretar(c: &Cartao) -> Value {
    let sem = c.textos.iter().any(|t| SEM_DADOS.iter().any(|s| t.to_lowercase().contains(s)));
    let periodo = c.textos.iter().find(|t| eh_periodo(t) && numero_tela(t).is_none()).cloned();
    let titulo = c.textos.iter().find(|t| numero_tela(t).is_none() && valor_e_pct(t).is_none() && !eh_periodo(t) && t.chars().count() > 3 && !SEM_DADOS.iter().any(|s| t.to_lowercase().contains(s))).cloned();
    let valor = c.textos.iter().find_map(|t| valor_e_pct(t).map(|v| (t.clone(), v)));
    // pares rótulo → número em sequência (ex.: "Continuaram assistindo", "53,1%")
    let mut pares = Vec::new();
    for w in c.textos.windows(2) {
        if numero_tela(&w[0]).is_none() && !eh_periodo(&w[0]) {
            if let Some(x) = numero_tela(&w[1]) {
                pares.push(json!([w[0], x]));
            }
        }
    }
    json!({
        "id": c.id, "quando": c.quando, "titulo": titulo, "periodo": periodo, "sem_dados": sem,
        "texto": valor.as_ref().map(|v| v.0.clone()),
        "valor": if sem { Value::Null } else { valor.as_ref().map(|v| json!(v.1 .0)).unwrap_or(Value::Null) },
        "valor_pct": valor.as_ref().and_then(|v| v.1 .1),
        "pares": pares,
        "barras": c.barras.iter().map(|(l, p)| json!([l, (p * 100.0 * 1e4).round() / 1e4])).collect::<Vec<_>>(),
    })
}

/// séries de views por hora/minuto: { ts (s) -> views }, o maior valor visto para cada hora
fn series(v: &Value, hora: &mut BTreeMap<i64, f64>, minuto: &mut BTreeMap<i64, f64>) {
    for r in v["results"].as_array().into_iter().flatten() {
        let k = r["key"].as_str().unwrap_or("");
        let alvo = if k.contains("HOURLY") { &mut *hora } else if k.contains("MINUTELY") { &mut *minuto } else { continue };
        let t = &r["value"]["resultTable"];
        let ts = t.pointer("/dimensionColumns/0/timestamps/values").and_then(|x| x.as_array()).cloned().unwrap_or_default();
        let vs = t.pointer("/metricColumns/0/counts/values").and_then(|x| x.as_array()).cloned().unwrap_or_default();
        for (a, b) in ts.iter().zip(vs.iter()) {
            if let (Some(a), Some(b)) = (a.as_i64(), b.as_f64()) {
                let e = alvo.entry(a / 1000).or_insert(0.0);
                if b > *e {
                    *e = b;
                }
            }
        }
    }
}

/// resumo de um vídeo (id de 11) ou canal ("canal_UC..."); None se não houver captura
pub fn resumo(id: &str) -> Option<Value> {
    let dir = dados_dir().join(id);
    let mut caps: Vec<Value> = std::fs::read_dir(&dir).ok()?.flatten().filter_map(|e| serde_json::from_slice::<Value>(&std::fs::read(e.path()).ok()?).ok()).collect();
    if caps.is_empty() {
        return None;
    }
    caps.sort_by(|a, b| a["quando"].as_str().unwrap_or("").cmp(b["quando"].as_str().unwrap_or("")));
    let mut atuais: BTreeMap<String, Value> = BTreeMap::new();
    let (mut hora, mut minuto) = (BTreeMap::new(), BTreeMap::new());
    for c in &caps {
        let q = c["quando"].as_str().unwrap_or("");
        let mut cs = Vec::new();
        cartoes(&c["dados"], q, &mut cs);
        for card in cs {
            let i = interpretar(&card);
            let k = chave(&card.id);
            // o mais recente com dado vence; "sem dados" só fica se nunca houve dado
            let manter_antigo = atuais.get(&k).map(|a| !a["sem_dados"].as_bool().unwrap_or(false) && i["sem_dados"].as_bool().unwrap_or(false)).unwrap_or(false);
            if !manter_antigo {
                atuais.insert(k, i);
            }
        }
        series(&c["dados"], &mut hora, &mut minuto);
    }
    let get = |k: &str| atuais.get(k);
    let val = |k: &str| get(k).and_then(|c| c["valor"].as_f64());
    let mut m = Map::new();
    let mut put = |nome: &str, v: Option<f64>, fonte: &str| {
        if let Some(x) = v {
            m.insert(nome.into(), json!({ "valor": x, "fonte": fonte, "quando": get(fonte).map(|c| c["quando"].clone()).unwrap_or(Value::Null), "periodo": get(fonte).map(|c| c["periodo"].clone()).unwrap_or(Value::Null) }));
        }
    };
    put("impressoes", val("impressions"), "impressions");
    put("ctr", val("ctr"), "ctr");
    put("avd", val("average_view_duration"), "average_view_duration");
    put("avd_pct", get("average_view_duration").and_then(|c| c["valor_pct"].as_f64()), "average_view_duration");
    put("horas", val("simple_metric_watch_time"), "simple_metric_watch_time");
    put("inscritos", val("simple_metric_subscribers"), "simple_metric_subscribers");
    put("gostei", val("shorts_likes").or(val("vod_likes")), if val("shorts_likes").is_some() { "shorts_likes" } else { "vod_likes" });
    put("views", val("KeyMetricViews"), "KeyMetricViews");
    // continuaram × ignorado: a barra tem o valor exato
    let mut achou_cont = false;
    for k in ["shorts_how_viewers_engaged", "channel_shorts_how_viewers_engaged", "shorts_viewers_engagement"] {
        if let Some(c) = get(k).filter(|c| !c["sem_dados"].as_bool().unwrap_or(false)) {
            let b = c["barras"].as_array().cloned().unwrap_or_default();
            let p0 = b.first().and_then(|x| x[1].as_f64()).or_else(|| c["pares"].get(0).and_then(|p| p[1].as_f64()));
            let p1 = b.get(1).and_then(|x| x[1].as_f64()).or_else(|| c["pares"].get(1).and_then(|p| p[1].as_f64()));
            achou_cont |= p0.is_some();
            put("continuaram", p0, k);
            put("ignorado", p1, k);
        }
    }
    // cartões de engajamento do canal/conteúdo têm outros ids: procura pelo par "continuaram"/"stayed"
    if !achou_cont {
        if let Some((k, c)) = atuais.iter().find(|(_, c)| c["pares"].as_array().into_iter().flatten().any(|p| p[0].as_str().map(|s| s.to_lowercase()).is_some_and(|s| s.contains("continuaram") || s.contains("stayed to watch")))) {
            let p = c["pares"].as_array().cloned().unwrap_or_default();
            let achar = |alvos: &[&str]| p.iter().find(|x| x[0].as_str().map(|s| s.to_lowercase()).is_some_and(|s| alvos.iter().any(|a| s.contains(a)))).and_then(|x| x[1].as_f64());
            let k = k.clone();
            put("continuaram", achar(&["continuaram", "stayed"]), &k);
            put("ignorado", achar(&["ignorado", "deslizaram", "swiped"]), &k);
        }
    }
    let lista = |k: &str| -> Value {
        get(k).filter(|c| !c["sem_dados"].as_bool().unwrap_or(false)).map(|c| {
            let barras = c["barras"].as_array().cloned().unwrap_or_default();
            if barras.iter().any(|b| b[0].as_str().is_some_and(|s| !s.is_empty())) { json!(barras) } else { c["pares"].clone() }
        }).unwrap_or(Value::Null)
    };
    // o Studio mostra 0 em impressões/CTR/duração/horas/inscritos até processar (horas depois das views):
    // com views na série, 0 nessas métricas é "processando", não zero
    let teve_views = hora.values().sum::<f64>() > 0.0;
    let mut processando = Vec::new();
    if teve_views {
        for k in ["impressoes", "ctr", "avd", "avd_pct", "horas", "inscritos", "views"] {
            if m.get(k).and_then(|x| x["valor"].as_f64()) == Some(0.0) {
                m.remove(k);
                processando.push(k.to_string());
            }
        }
    }
    let sem_dados: Vec<String> = atuais.iter().filter(|(_, c)| c["sem_dados"].as_bool().unwrap_or(false)).map(|(k, _)| k.clone()).collect();
    Some(json!({
        "id": id, "capturas": caps.len(),
        "primeira": caps.first().map(|c| c["quando"].clone()), "atualizado": caps.last().map(|c| c["quando"].clone()),
        "metricas": m,
        "trafego": lista("how_viewers_found"), "trafego_48h": lista("realtime_traffic_sources"),
        "geografias": lista("top_geographies"), "demografia": lista("top_demographics"),
        "cartoes": atuais, "sem_dados": sem_dados, "processando": processando,
        "por_hora": hora.iter().map(|(t, v)| json!([t, v])).collect::<Vec<_>>(),
        "por_minuto": minuto.iter().rev().take(180).rev().map(|(t, v)| json!([t, v])).collect::<Vec<_>>(),
    }))
}

/// views acumuladas nas primeiras N horas desde a publicação (pela série por hora do Studio).
/// "cobre" = a série começa antes da publicação (senão as primeiras horas não foram capturadas).
pub fn primeiras_horas(res: &Value, publicado: i64, agora: i64) -> Value {
    let serie: Vec<(i64, f64)> = res["por_hora"].as_array().into_iter().flatten().filter_map(|p| Some((p[0].as_i64()?, p[1].as_f64()?))).collect();
    let Some(inicio) = serie.first().map(|p| p.0) else { return Value::Null };
    let h0 = publicado - publicado.rem_euclid(3600);
    if inicio > h0 + 3600 {
        return json!({ "cobre": false, "motivo": "as capturas começam depois da publicação: abra o vídeo no Studio nas primeiras 48 h" });
    }
    let mut out = Map::new();
    for n in [1i64, 3, 6, 12, 24, 48] {
        let fim = h0 + n * 3600;
        let soma: f64 = serie.iter().filter(|p| p.0 >= h0 && p.0 < fim).map(|p| p.1).sum();
        let ultimo = serie.last().map(|p| p.0 + 3600).unwrap_or(0);
        if fim <= agora && fim <= ultimo {
            out.insert(format!("h{n}"), json!(soma));
        } else {
            out.insert(format!("h{n}"), Value::Null);
        }
    }
    // pico: hora com mais views desde a publicação
    let pico = serie.iter().filter(|p| p.0 >= h0).max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    out.insert("cobre".into(), json!(true));
    out.insert("pico_hora".into(), json!(pico.map(|p| (p.0 - h0) / 3600 + 1)));
    out.insert("pico_views".into(), json!(pico.map(|p| p.1)));
    out.insert("serie".into(), json!(serie.iter().filter(|p| p.0 >= h0).take(72).map(|p| json!([(p.0 - h0) / 3600 + 1, p.1])).collect::<Vec<_>>()));
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn engaged() -> Value {
        json!({ "cardViewModel": { "accessibilityId": "analytics_hub.video_overview_shorts_how_viewers_engaged", "bodyContent": { "items": [
            { "horizontalBarViewModel": { "segments": [{ "percentage": 0.5307 }, { "percentage": 0.4693 }] } },
            { "simpleMetricViewModel": { "subtitle": { "content": "Continuaram assistindo" }, "value": { "content": "53,1%" } } },
            { "simpleMetricViewModel": { "subtitle": { "content": "Ignorado" }, "value": { "content": "46,9%" } } },
            { "sectionHeaderViewModel": { "headline": { "content": "Engajamento dos espectadores" }, "sublabel": { "content": "Desde a publicação" } } } ] } } })
    }
    #[test]
    fn cartoes_e_chaves() {
        let mut cs = Vec::new();
        cartoes(&json!({ "a": [engaged(), { "cardViewModel": { "accessibilityId": "analytics_hub.video_overview_ctr", "x": [{ "content": "4,2%" }, { "content": "Taxa de cliques" }, { "content": "Desde o carregamento" }] } },
            { "cardViewModel": { "accessibilityId": "analytics_hub.FEanalytics_hub_video_overview.top_geographies", "x": [{ "content": "Principais geografias" }, { "content": "Não existem dados suficientes" }] } }] }), "2026-11-17T22:59:24Z", &mut cs);
        assert_eq!(cs.len(), 3);
        let e = interpretar(&cs[0]);
        assert_eq!(chave(&cs[0].id), "shorts_how_viewers_engaged");
        assert_eq!(e["barras"][0][1].as_f64().unwrap(), 53.07);
        assert_eq!(e["periodo"], "Desde a publicação");
        let ctr = interpretar(&cs[1]);
        assert_eq!(chave(&cs[1].id), "ctr");
        assert_eq!(ctr["valor"].as_f64().unwrap(), 4.2);
        assert_eq!(ctr["titulo"], "Taxa de cliques");
        let g = interpretar(&cs[2]);
        assert!(g["sem_dados"].as_bool().unwrap() && g["valor"].is_null());
        assert_eq!(valor_e_pct("0:42 (63%)"), Some((42.0, Some(63.0))));
        assert!(eh_periodo("Desde a publicação") && eh_periodo("Visualizações • 48 horas") && eh_periodo("60 minutos"));
        assert!(!eh_periodo("Tempo de visualização (horas)") && !eh_periodo("Taxa de cliques"));
    }
    #[test]
    fn horas() {
        let pubd = 1_794_945_600; // hora cheia
        let serie: Vec<Value> = (0..30).map(|k| json!([pubd - 3600 + k * 3600, if k == 0 { 0.0 } else { 100.0 }])).collect();
        let r = json!({ "por_hora": serie });
        let p = primeiras_horas(&r, pubd + 600, pubd + 40 * 3600);
        assert_eq!(p["h1"], 100.0);
        assert_eq!(p["h24"], 2400.0);
        assert!(p["h48"].is_null(), "a série não cobre 48 h");
        let tarde = json!({ "por_hora": [[pubd + 10 * 3600, 5.0]] });
        assert_eq!(primeiras_horas(&tarde, pubd, pubd + 20 * 3600)["cobre"], false);
    }
}

#[cfg(test)]
mod real {
    /// VF_STUDIO_REAL=<id> cargo test resumo_real -- --ignored --nocapture : resumo das capturas reais
    #[test]
    #[ignore]
    fn resumo_real() {
        let id = std::env::var("VF_STUDIO_REAL").unwrap();
        let mut r = super::resumo(&id).unwrap();
        r["cartoes"] = serde_json::json!("(omitido)");
        let n = r["por_hora"].as_array().map(|a| a.len()).unwrap_or(0);
        r["por_hora"] = serde_json::json!(format!("{n} horas"));
        r["por_minuto"] = serde_json::json!("(omitido)");
        println!("{}", serde_json::to_string_pretty(&r).unwrap());
        let pubd = crate::tempo::parse_iso("2026-11-17T19:40:00Z").unwrap().unwrap();
        println!("{}", super::primeiras_horas(&super::resumo(&id).unwrap(), pubd, crate::tempo::now()));
    }
}
