//! `vf studio`: métricas que só o YouTube Studio mostra. Caminho recomendado: a EXTENSÃO do Chrome
//! (`tools/vf-studio-ext`, `vf studio instalar`): os cookies nunca saem do navegador; ela captura o que o
//! Studio já carrega e entrega aqui (native messaging) em ~/.config/vf/studio/dados/<vídeo>/.
//! Caminho legado (abaixo): `vf studio login` com "Copy as cURL" e cookies no Keychain.
//!
//! Legado: (ex.: "Continuaram assistindo" dos Shorts), pela API
//! INTERNA do Studio (youtubei), com a sessão do navegador. NÃO é API pública: viola os termos de uso do
//! YouTube sobre acesso automatizado e pode quebrar sem aviso. Escolha do usuário; use com parcimônia.
//!
//!   1. No Chrome: Studio → Análises do vídeo → aba/cartão com a métrica; DevTools → Network → filtre
//!      "youtubei" → clique com o botão direito na requisição que traz o número → Copy → Copy as cURL (bash).
//!   2. pbpaste | vf studio login account=<apelido> [nome=shorts]   (ou curl=<arquivo>)
//!      -> cookies no Keychain do macOS (nunca em arquivo, nunca na linha de comando); o modelo da consulta
//!         (URL, corpo e cabeçalhos SEM cookie/autorização) em ~/.config/vf/studio/<apelido>/<nome>.json
//!   3. vf studio buscar video=ID [nome=shorts]  -> repete a consulta trocando o vídeo, salva a resposta em
//!      out/studio/ e lista as métricas encontradas (nomes em MAIÚSCULAS + valores)
//!   4. vf studio mapear continuaram=<NOME_DA_METRICA>  -> o vf relatorio studio=1 preenche a linha
//!   vf studio status | vf studio logout account=<apelido>
use crate::kv::Kv;
use crate::net::{self, Body};
use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};
use std::path::PathBuf;

const SERVICO: &str = "vf-studio";
const ORIGEM: &str = "https://studio.youtube.com";

fn dir() -> PathBuf {
    crate::auth::cfg_dir().join("studio")
}

// ---------------------------------------------------------------- segredo (Keychain no macOS)

#[cfg(target_os = "macos")]
fn guardar(conta: &str, segredo: &[u8]) -> Result<String> {
    security_framework::passwords::set_generic_password(SERVICO, conta, segredo).context("gravar no Keychain")?;
    Ok(format!("Keychain do macOS (serviço {SERVICO}, conta {conta})"))
}
#[cfg(target_os = "macos")]
fn ler(conta: &str) -> Result<Vec<u8>> {
    security_framework::passwords::get_generic_password(SERVICO, conta).with_context(|| format!("sessão do Studio de \"{conta}\" não encontrada no Keychain: rode vf studio login account={conta}"))
}
#[cfg(target_os = "macos")]
fn apagar(conta: &str) -> Result<()> {
    security_framework::passwords::delete_generic_password(SERVICO, conta).context("apagar do Keychain")
}

// fora do macOS: arquivo modo 600 (avisado no login)
#[cfg(not(target_os = "macos"))]
fn guardar(conta: &str, segredo: &[u8]) -> Result<String> {
    let p = dir().join(format!("{conta}.segredo"));
    std::fs::create_dir_all(dir())?;
    std::fs::write(&p, segredo)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))?;
    Ok(format!("{} (modo 600; sem Keychain neste sistema)", p.display()))
}
#[cfg(not(target_os = "macos"))]
fn ler(conta: &str) -> Result<Vec<u8>> {
    std::fs::read(dir().join(format!("{conta}.segredo"))).with_context(|| format!("sessão do Studio de \"{conta}\" não encontrada: rode vf studio login account={conta}"))
}
#[cfg(not(target_os = "macos"))]
fn apagar(conta: &str) -> Result<()> {
    std::fs::remove_file(dir().join(format!("{conta}.segredo"))).context("apagar sessão")
}

// ---------------------------------------------------------------- "Copy as cURL"

/// separa um comando de shell em palavras: '...', "...", $'...' (ANSI-C) e barra invertida no fim da linha
pub fn shell_words(s: &str) -> Vec<String> {
    let c: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut tem = false;
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        match ch {
            '\\' if i + 1 < c.len() && (c[i + 1] == '\n' || c[i + 1] == '\r') => {
                i += 2;
                continue;
            }
            '\\' if i + 1 < c.len() => {
                cur.push(c[i + 1]);
                tem = true;
                i += 2;
                continue;
            }
            '\'' => {
                i += 1;
                while i < c.len() && c[i] != '\'' {
                    cur.push(c[i]);
                    i += 1;
                }
                tem = true;
            }
            '$' if i + 1 < c.len() && c[i + 1] == '\'' => {
                i += 2;
                while i < c.len() && c[i] != '\'' {
                    if c[i] == '\\' && i + 1 < c.len() {
                        i += 1;
                        match c[i] {
                            'n' => cur.push('\n'),
                            't' => cur.push('\t'),
                            'r' => cur.push('\r'),
                            'x' if i + 2 < c.len() => {
                                let h: String = c[i + 1..i + 3].iter().collect();
                                if let Ok(b) = u8::from_str_radix(&h, 16) {
                                    cur.push(b as char);
                                    i += 2;
                                }
                            }
                            'u' if i + 4 < c.len() => {
                                let h: String = c[i + 1..i + 5].iter().collect();
                                if let Some(ch) = u32::from_str_radix(&h, 16).ok().and_then(char::from_u32) {
                                    cur.push(ch);
                                    i += 4;
                                }
                            }
                            o => cur.push(o),
                        }
                    } else {
                        cur.push(c[i]);
                    }
                    i += 1;
                }
                tem = true;
            }
            '"' => {
                i += 1;
                while i < c.len() && c[i] != '"' {
                    if c[i] == '\\' && i + 1 < c.len() && matches!(c[i + 1], '"' | '\\' | '$' | '`') {
                        i += 1;
                    }
                    cur.push(c[i]);
                    i += 1;
                }
                tem = true;
            }
            w if w.is_whitespace() => {
                if tem {
                    out.push(std::mem::take(&mut cur));
                    tem = false;
                }
            }
            o => {
                cur.push(o);
                tem = true;
            }
        }
        i += 1;
    }
    if tem {
        out.push(cur);
    }
    out
}

pub struct Captura {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub cookie: String,
    pub corpo: Option<String>,
}

pub fn parse_curl(s: &str) -> Result<Captura> {
    let w = shell_words(s);
    if w.first().map(|x| x != "curl").unwrap_or(true) {
        bail!("não parece um \"Copy as cURL (bash)\" do Chrome (deve começar com curl)");
    }
    let mut url = String::new();
    let mut headers = Vec::new();
    let mut cookie = String::new();
    let mut corpo = None;
    let mut i = 1;
    while i < w.len() {
        let a = w[i].as_str();
        let prox = w.get(i + 1).cloned().unwrap_or_default();
        match a {
            "-H" | "--header" => {
                if let Some((k, v)) = prox.split_once(':') {
                    let (k, v) = (k.trim().to_lowercase(), v.trim().to_string());
                    if k == "cookie" {
                        cookie = v;
                    } else {
                        headers.push((k, v));
                    }
                }
                i += 1;
            }
            "-b" | "--cookie" => {
                cookie = prox;
                i += 1;
            }
            "--data-raw" | "--data" | "-d" | "--data-binary" => {
                corpo = Some(prox);
                i += 1;
            }
            "-X" | "--request" | "-A" | "--user-agent" | "-e" | "--referer" => i += 1,
            x if x.starts_with("http") => url = x.to_string(),
            _ => {}
        }
        i += 1;
    }
    if url.is_empty() {
        bail!("URL não encontrada no cURL");
    }
    if !url.contains("youtube.com/youtubei/") {
        bail!("essa requisição não é da API interna do Studio (youtubei): {}", url.split('?').next().unwrap_or(""));
    }
    if !cookie.contains("SAPISID=") {
        bail!("o cURL não traz o cookie SAPISID: copie com \"Copy as cURL (bash)\" de dentro do Studio logado");
    }
    Ok(Captura { url, headers, cookie, corpo })
}

fn cookie_val(cookie: &str, nome: &str) -> Option<String> {
    cookie.split(';').map(|p| p.trim()).find_map(|p| p.strip_prefix(&format!("{nome}=")).map(String::from))
}

/// Authorization que o Studio manda: SAPISIDHASH <ts>_<sha1("<ts> <SAPISID> <origem>")> (+ 1P/3P quando houver)
pub fn sapisid_auth(cookie: &str, ts: i64) -> Option<String> {
    let h = |v: &str| -> String { Sha1::digest(format!("{ts} {v} {ORIGEM}").as_bytes()).iter().map(|b| format!("{b:02x}")).collect() };
    let mut partes = vec![format!("SAPISIDHASH {ts}_{}", h(&cookie_val(cookie, "SAPISID")?))];
    if let Some(v) = cookie_val(cookie, "__Secure-1PAPISID") {
        partes.push(format!("SAPISID1PHASH {ts}_{}", h(&v)));
    }
    if let Some(v) = cookie_val(cookie, "__Secure-3PAPISID") {
        partes.push(format!("SAPISID3PHASH {ts}_{}", h(&v)));
    }
    Some(partes.join(" "))
}

// ---------------------------------------------------------------- métricas na resposta

/// procura ids de vídeo (11 caracteres) nos campos cujo nome fala de vídeo
fn ids_no_corpo(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                if k.to_lowercase().contains("video") {
                    if let Some(s) = x.as_str().filter(|s| s.len() == 11) {
                        out.push(s.to_string());
                    }
                    if let Some(a) = x.as_array() {
                        out.extend(a.iter().filter_map(|y| y.as_str()).filter(|s| s.len() == 11).map(String::from));
                    }
                }
                ids_no_corpo(x, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| ids_no_corpo(x, out)),
        Value::String(s) => {
            // filtros como "video==ID" ou "VIDEO_ID:ID"
            for sep in ["video==", "VIDEO==", "videoId="] {
                if let Some(r) = s.split(sep).nth(1) {
                    let id: String = r.chars().take(11).collect();
                    if id.len() == 11 {
                        out.push(id);
                    }
                }
            }
        }
        _ => {}
    }
}

/// pares (NOME_EM_MAIUSCULAS, valor) achados na resposta: objetos com um campo de nome de métrica
/// (metric/metricId/type/key/name) e um número ao lado, mais chaves em MAIÚSCULAS com valor numérico
pub fn metricas(v: &Value) -> Vec<(String, f64, String)> {
    let mut out = Vec::new();
    fn num(x: &Value) -> Option<f64> {
        match x {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }
    fn maiusc(s: &str) -> bool {
        s.len() >= 4 && s.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()) && s.chars().any(|c| c.is_ascii_uppercase())
    }
    fn andar(v: &Value, caminho: &str, out: &mut Vec<(String, f64, String)>) {
        match v {
            Value::Object(m) => {
                // formato do yta_web/join: {"metric": {"type": "VIEWS"}, "counts": {"values": [...]}, "total": ...}
                if let Some(n) = m.get("metric").and_then(|x| x.get("type")).and_then(|x| x.as_str()).filter(|s| maiusc(s)) {
                    let cumul = m.get("metric").and_then(|x| x.get("cumulative")).and_then(|x| x.as_bool()).unwrap_or(false);
                    let vals: Vec<f64> = m.get("counts").or(m.get("values")).and_then(|c| c.get("values").or(Some(c))).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(num).collect()).unwrap_or_default();
                    let valor = ["total", "value", "lifetimeTotal"].iter().find_map(|k| m.get(*k).and_then(num))
                        .or_else(|| if vals.is_empty() { None } else if cumul { vals.last().copied() } else { Some(vals.iter().sum()) });
                    if let Some(x) = valor {
                        out.push((n.to_string(), x, caminho.to_string()));
                    }
                }
                let nome = ["metric", "metricId", "metricType", "type", "key", "name", "id"].iter().find_map(|k| m.get(*k).and_then(|x| x.as_str()).filter(|s| maiusc(s)));
                if let Some(n) = nome {
                    let valor = ["value", "total", "doubleValue", "int64Value", "metricValue", "count"].iter().find_map(|k| m.get(*k).and_then(num))
                        .or_else(|| m.get("value").and_then(|x| x.as_object()).and_then(|o| o.values().find_map(num)));
                    if let Some(x) = valor {
                        out.push((n.to_string(), x, caminho.to_string()));
                    }
                }
                for (k, x) in m {
                    if maiusc(k) {
                        if let Some(n) = num(x) {
                            out.push((k.clone(), n, caminho.to_string()));
                        }
                    }
                    andar(x, &format!("{caminho}.{k}"), out);
                }
            }
            Value::Array(a) => {
                for (i, x) in a.iter().enumerate() {
                    andar(x, &format!("{caminho}[{i}]"), out);
                }
            }
            _ => {}
        }
    }
    andar(v, "$", &mut out);
    out.retain(|x| !x.0.starts_with("USER_INTERFACE_THEME") && !x.0.starts_with("YT_SYS_"));
    out.extend(pares_texto(v));
    out
}

/// número como o Studio escreve em pt-BR/en: "49,6%", "1,2 mil", "2,3 mi", "1.234", "0:42", "+5"
pub fn numero_tela(s: &str) -> Option<f64> {
    let t = s.replace(['\u{2009}', '\u{a0}', '\u{202f}', '\u{200e}', '\u{2068}', '\u{2069}'], " ").trim().to_lowercase();
    if t.is_empty() || !t.chars().next()?.is_ascii_digit() && !t.starts_with(['+', '-', '−']) {
        return None;
    }
    if let Some((a, b)) = t.split_once(':') {
        if a.chars().all(|c| c.is_ascii_digit()) && b.len() == 2 && b.chars().all(|c| c.is_ascii_digit()) {
            return Some(a.parse::<f64>().ok()? * 60.0 + b.parse::<f64>().ok()?);
        }
    }
    let (mult, base) = [(" mil", 1e3), (" mi", 1e6), (" bi", 1e9), ("k", 1e3), ("m", 1e6)]
        .iter()
        .find_map(|(suf, m)| t.strip_suffix(suf).map(|b| (*m, b.to_string())))
        .unwrap_or((1.0, t.clone()));
    let base = base.trim_end_matches('%').trim().replace('−', "-").replace('+', "");
    // pt-BR: "1.234,5" ; en: "1,234.5"
    let n = if base.contains(',') && base.contains('.') {
        if base.rfind(',') > base.rfind('.') { base.replace('.', "").replace(',', ".") } else { base.replace(',', "") }
    } else if base.contains(',') {
        base.replace(',', ".")
    } else if base.matches('.').count() == 1 && base.split('.').nth(1).map(|d| d.len() == 3).unwrap_or(false) && mult == 1.0 {
        base.replace('.', "")
    } else {
        base
    };
    if !n.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
        return None;
    }
    n.parse::<f64>().ok().map(|x| x * mult)
}

/// cartões do hub de análises (view models da tela): pares rótulo → valor e barras com porcentagem.
/// Nome devolvido: "TEXTO:<rótulo>"; porcentagens ficam em % (0-100).
pub fn pares_texto(v: &Value) -> Vec<(String, f64, String)> {
    // sequência de folhas em ordem de documento, separada por cartão
    #[derive(Debug)]
    enum F {
        Rotulo(String),
        Valor(String),
        Fim,
    }
    fn andar(v: &Value, seq: &mut Vec<F>, out: &mut Vec<(String, f64, String)>) {
        match v {
            Value::Object(m) => {
                if let Some(b) = m.get("horizontalBarViewModel") {
                    if let (Some(l), Some(p)) = (b.get("accessibilityLabel").and_then(|x| x.as_str()), b.pointer("/segments/0/percentage").and_then(|x| x.as_f64())) {
                        out.push((format!("TEXTO:{l}"), p * 100.0, "barra".into()));
                    }
                    return;
                }
                if let Some(sm) = m.get("simpleMetricViewModel") {
                    if let Some(c) = sm.pointer("/value/content").and_then(|x| x.as_str()) {
                        // rótulo do próprio cartão (subtitle/title), quando vem junto do valor
                        let proprio = ["/subtitle/content", "/title/content", "/label/content"].iter().find_map(|k| sm.pointer(k).and_then(|x| x.as_str())).map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
                        match (proprio, numero_tela(c)) {
                            (Some(r), Some(x)) => out.push((format!("TEXTO:{r}"), x, format!("\"{c}\""))),
                            _ => seq.push(F::Valor(c.to_string())),
                        }
                    }
                    return;
                }
                // cartão "como os espectadores interagiram" dos Shorts: a barra traz o valor exato (0,5307)
                if let (Some(id), Some(Value::Object(_))) = (m.get("accessibilityId").and_then(|x| x.as_str()), m.get("bodyContent")) {
                    if id.ends_with("shorts_how_viewers_engaged") {
                        let mut ps = Vec::new();
                        fn barras(v: &Value, ps: &mut Vec<f64>) {
                            match v {
                                Value::Object(m) => {
                                    if let Some(p) = m.get("horizontalBarViewModel").and_then(|b| b.pointer("/segments/0/percentage")).and_then(|x| x.as_f64()) {
                                        ps.push(p);
                                    }
                                    m.values().for_each(|x| barras(x, ps));
                                }
                                Value::Array(a) => a.iter().for_each(|x| barras(x, ps)),
                                _ => {}
                            }
                        }
                        barras(&m["bodyContent"], &mut ps);
                        if let Some(p) = ps.first() {
                            out.push(("STUDIO:SHORTS_CONTINUARAM".into(), p * 100.0, id.to_string()));
                        }
                    }
                }
                let card = m.contains_key("cardViewModel") || m.contains_key("hoverCardViewModel");
                for (k, x) in m {
                    if matches!(k.as_str(), "content" | "simpleText" | "headline" | "title") {
                        if let Some(t) = x.as_str().or_else(|| x.get("content").and_then(|c| c.as_str())) {
                            let t = t.trim();
                            if !t.is_empty() && numero_tela(t).is_none() && t.len() < 120 {
                                seq.push(F::Rotulo(t.to_string()));
                            } else if numero_tela(t).is_some() && k == "content" {
                                seq.push(F::Valor(t.to_string()));
                            }
                        }
                        if x.is_string() {
                            continue;
                        }
                    }
                    if !matches!(k.as_str(), "accessibility" | "accessibilityLabel" | "loggingDirectives" | "trackingParams" | "styleRuns" | "commandRuns") {
                        andar(x, seq, out);
                    }
                }
                if card {
                    seq.push(F::Fim);
                }
            }
            Value::Array(a) => a.iter().for_each(|x| andar(x, seq, out)),
            _ => {}
        }
    }
    let mut seq = Vec::new();
    let mut out = Vec::new();
    andar(v, &mut seq, &mut out);
    // cada valor fica com o rótulo mais próximo antes dele no mesmo cartão
    let mut rotulo: Option<String> = None;
    for f in seq {
        match f {
            F::Rotulo(r) => rotulo = Some(r),
            F::Valor(t) => {
                if let (Some(r), Some(x)) = (&rotulo, numero_tela(&t)) {
                    out.push((format!("TEXTO:{r}"), x, format!("\"{t}\"")));
                }
            }
            F::Fim => rotulo = None,
        }
    }
    out
}

// ---------------------------------------------------------------- comando

pub fn run(args: &[String]) -> Result<()> {
    let (acao, rest) = match args.first().map(|s| s.as_str()) {
        Some(a @ ("login" | "buscar" | "status" | "logout" | "mapear" | "host" | "instalar" | "desinstalar" | "metricas" | "resumo")) => (a, &args[1..]),
        // o Chrome chama o host com a origem da extensão como argumento
        Some(x) if x.starts_with("chrome-extension://") => ("host", &args[1..]),
        _ => bail!("uso: vf studio instalar | resumo video=ID | metricas video=ID | mapear linha=METRICA | status | desinstalar  (legado: login | buscar | logout)"),
    };
    match acao {
        "mapear" => return mapear(rest),
        "resumo" => {
            // resumo limpo (um valor atual por cartão + primeiras horas): vf studio resumo video=ID|canal=UC...
            let kv = Kv::parse(rest, &[])?;
            let id = match (kv.get("video"), kv.get("canal")) {
                (Some(v), _) => v.to_string(),
                (None, Some(c)) => format!("canal_{}", c.trim_start_matches("canal_")),
                _ => bail!("vf studio resumo video=ID (ou canal=UC...)"),
            };
            let mut r = crate::studio_dados::resumo(&id).with_context(|| format!("nada capturado para {id}: abra as análises no Studio com a extensão"))?;
            if let Some(o) = r.as_object_mut() {
                o.remove("cartoes");
                o.remove("por_minuto");
            }
            println!("{}", serde_json::to_string_pretty(&r)?);
            return Ok(());
        }
        "host" => return host(),
        "instalar" => return instalar(),
        "desinstalar" => return desinstalar(),
        "metricas" => {
            let kv = Kv::parse(rest, &[])?;
            let video = kv.get("video").context("vf studio metricas video=ID")?;
            let ms = metricas_capturadas(video);
            if ms.is_empty() {
                bail!("nada capturado para {video}: abra as análises dele no Studio (ou use a coleta da extensão)");
            }
            println!("{:<48} {:>14}  resposta", "métrica", "valor");
            for (n, v, onde) in ms {
                println!("{n:<48} {v:>14}  {}", onde.chars().take(70).collect::<String>());
            }
            eprintln!("\nquando achar a métrica certa: vf studio mapear continuaram=<NOME>");
            return Ok(());
        }
        _ => {}
    }
    let kv = Kv::parse(rest, &[])?;
    kv.check_known(&["account", "conta", "nome", "curl", "video", "o", "out"])?;
    let conta = crate::auth::slug(kv.get("account").or(kv.get("conta")).unwrap_or("principal"));
    let nome = crate::auth::slug(kv.get("nome").unwrap_or("metricas"));
    let modelo = dir().join(&conta).join(format!("{nome}.json"));
    match acao {
        "login" => {
            let txt = match kv.get("curl") {
                Some(f) => std::fs::read_to_string(f).with_context(|| format!("ler {f}"))?,
                None => {
                    let mut s = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
                    s
                }
            };
            let c = parse_curl(&txt)?;
            // segredo: só o necessário para autenticar (cookie + qual conta/canal)
            let get = |k: &str| c.headers.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
            let segredo = json!({ "cookie": c.cookie, "authuser": get("x-goog-authuser"), "pageid": get("x-goog-pageid") });
            let onde = guardar(&conta, &serde_json::to_vec(&segredo)?)?;
            // modelo da consulta: sem cookie nem autorização
            let corpo: Value = c.corpo.as_deref().map(|b| serde_json::from_str(b).unwrap_or(Value::String(b.to_string()))).unwrap_or(Value::Null);
            let mut ids = Vec::new();
            ids_no_corpo(&corpo, &mut ids);
            ids.dedup();
            let headers: Vec<(String, String)> = c.headers.iter().filter(|(k, _)| !matches!(k.as_str(), "authorization" | "cookie" | "x-goog-authuser" | "x-goog-pageid" | "content-length")).cloned().collect();
            std::fs::create_dir_all(modelo.parent().unwrap())?;
            crate::util::write_json(&modelo, &json!({ "url": c.url, "headers": headers, "corpo": corpo, "video_capturado": ids.first(), "capturado": crate::tempo::rfc3339(crate::tempo::now()) }))?;
            println!("sessão do Studio de \"{conta}\" guardada em: {onde}\nmodelo da consulta \"{nome}\": {} (vídeo capturado: {})", modelo.display(), ids.first().map(|s| s.as_str()).unwrap_or("nenhum achado"));
            eprintln!("ATENÇÃO: esses cookies dão acesso à sua conta Google. Apague com: vf studio logout account={conta}. Sair da conta no navegador também os invalida.");
            Ok(())
        }
        "logout" => {
            let _ = apagar(&conta);
            let _ = std::fs::remove_dir_all(dir().join(&conta));
            println!("sessão e modelos do Studio de \"{conta}\" apagados");
            Ok(())
        }
        "status" => {
            let contas: Vec<String> = std::fs::read_dir(dir()).map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
            if contas.is_empty() {
                println!("nenhuma sessão do Studio (vf studio login)");
            }
            for c in contas {
                let ok = ler(&c).is_ok();
                let modelos: Vec<String> = std::fs::read_dir(dir().join(&c)).map(|d| d.filter_map(|e| e.ok()).filter_map(|e| e.file_name().to_str()?.strip_suffix(".json").map(String::from)).collect()).unwrap_or_default();
                println!("{c:<20} sessão {} · consultas: {}", if ok { "guardada" } else { "AUSENTE" }, modelos.join(", "));
            }
            let caps = std::fs::read_dir(dados_dir()).map(|d| d.count()).unwrap_or(0);
            println!("extensão: host {} · {caps} vídeo(s) capturado(s) em {}", if host_manifestos().iter().any(|p| p.exists()) { "instalado" } else { "NÃO instalado (vf studio instalar)" }, dados_dir().display());
            println!("mapa: {}", serde_json::to_string(&mapa()).unwrap_or_default());
            Ok(())
        }
        _ => {
            let video = kv.get("video").context("diga o vídeo: vf studio buscar video=ID")?;
            let r = buscar(&conta, &nome, video)?;
            let out = kv.get("o").or(kv.get("out")).map(PathBuf::from).unwrap_or_else(|| if std::path::Path::new("out").is_dir() { PathBuf::from("out/studio") } else { PathBuf::from("studio") });
            std::fs::create_dir_all(&out)?;
            let arq = out.join(format!("{video}-{nome}-{}.json", crate::tempo::date(crate::tempo::now())));
            crate::util::write_json(&arq, &r)?;
            let ms = metricas(&r);
            if ms.is_empty() {
                println!("nenhuma métrica reconhecida na resposta; veja o JSON bruto: {}", arq.display());
            } else {
                println!("{:<48} {:>14}  onde", "métrica", "valor");
                let mut vistos = std::collections::HashSet::new();
                for (n, v, c) in ms {
                    if vistos.insert((n.clone(), c.clone())) {
                        println!("{n:<48} {v:>14}  {}", c.chars().take(70).collect::<String>());
                    }
                }
                eprintln!("\nresposta bruta: {}\nquando achar a métrica certa: vf studio mapear continuaram=<NOME>", arq.display());
            }
            Ok(())
        }
    }
}

fn mapa_path() -> PathBuf {
    dir().join("mapa.json")
}

pub fn mapa() -> Map<String, Value> {
    let mut m: Map<String, Value> = std::fs::read(mapa_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    // padrão: o rótulo do cartão do Studio (pt-BR e en)
    m.entry("continuaram").or_insert(json!("STUDIO:SHORTS_CONTINUARAM|TEXTO:Continuaram assistindo|TEXTO:Stayed to watch"));
    m
}

/// a métrica casa com o mapa? (nome exato, ou "TEXTO:<rótulo>" contido no rótulo, sem diferenciar maiúsculas;
/// várias opções separadas por |)
fn casa(nome: &str, alvo: &str) -> bool {
    alvo.split('|').any(|a| {
        let a = a.trim();
        nome == a || (a.starts_with("TEXTO:") && nome.starts_with("TEXTO:") && nome[6..].to_lowercase().contains(&a[6..].to_lowercase()))
    })
}

fn mapear(args: &[String]) -> Result<()> {
    let mut m = mapa();
    for a in args {
        let (k, v) = a.split_once('=').context("use linha=METRICA, ex.: continuaram=SHORTS_...")?;
        if v.is_empty() {
            m.remove(k);
        } else {
            m.insert(k.to_string(), json!(v));
        }
    }
    std::fs::create_dir_all(dir())?;
    crate::util::write_json(&mapa_path(), &Value::Object(m.clone()))?;
    println!("mapa do Studio: {}", serde_json::to_string(&Value::Object(m))?);
    Ok(())
}

/// repete a consulta capturada trocando o vídeo; devolve a resposta JSON
pub fn buscar(conta: &str, nome: &str, video: &str) -> Result<Value> {
    let modelo: Value = serde_json::from_slice(&std::fs::read(dir().join(conta).join(format!("{nome}.json"))).with_context(|| format!("consulta \"{nome}\" de \"{conta}\" não capturada: vf studio login account={conta} nome={nome}"))?)?;
    let seg: Value = serde_json::from_slice(&ler(conta)?)?;
    let cookie = seg["cookie"].as_str().unwrap_or("");
    let auth = sapisid_auth(cookie, crate::tempo::now()).context("cookie sem SAPISID: capture de novo")?;
    let mut corpo = serde_json::to_string(&modelo["corpo"])?;
    if let Some(orig) = modelo["video_capturado"].as_str() {
        corpo = corpo.replace(orig, video);
    }
    let corpo: Value = serde_json::from_str(&corpo)?;
    let mut hs: Vec<(String, String)> = modelo["headers"].as_array().map(|a| a.iter().filter_map(|x| Some((x[0].as_str()?.to_string(), x[1].as_str()?.to_string()))).collect()).unwrap_or_default();
    hs.retain(|(k, _)| k != "content-type");
    hs.push(("cookie".into(), cookie.to_string()));
    hs.push(("authorization".into(), auth));
    hs.push(("x-origin".into(), ORIGEM.into()));
    if let Some(u) = seg["authuser"].as_str() {
        hs.push(("x-goog-authuser".into(), u.into()));
    }
    if let Some(p) = seg["pageid"].as_str() {
        hs.push(("x-goog-pageid".into(), p.into()));
    }
    let refs: Vec<(&str, &str)> = hs.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let r = net::request("POST", modelo["url"].as_str().unwrap_or(""), &refs, Body::Json(&corpo))?;
    if r.status == 401 || r.status == 403 {
        bail!("o Studio recusou a sessão (HTTP {}): ela expirou ou você saiu da conta. Capture de novo: vf studio login account={conta} nome={nome}", r.status);
    }
    r.ok().context("Studio (API interna)")?.json()
}

/// valores mapeados (linha do relatório -> número) para um vídeo; erros viram None
pub fn linhas_mapeadas(conta: &str, video: &str) -> Map<String, Value> {
    let m = mapa();
    let mut out = Map::new();
    if m.is_empty() {
        return out;
    }
    // 1º: o que a extensão capturou (sem rede, sem cookie)
    let caps = metricas_capturadas(video);
    for (linha, metrica) in &m {
        // a primeira alternativa do mapa que existir vence (ex.: o valor exato da barra antes do texto arredondado)
        let achado = metrica.as_str().and_then(|mm| mm.split('|').find_map(|alt| caps.iter().find(|x| casa(&x.0, alt))));
        if let Some(v) = achado {
            out.insert(linha.clone(), json!(v.1));
        }
    }
    if !out.is_empty() || ler(conta).is_err() {
        return out;
    }
    // 2º (legado): sessão por cookie
    let nomes: Vec<String> = std::fs::read_dir(dir().join(conta)).map(|d| d.filter_map(|e| e.ok()).filter_map(|e| e.file_name().to_str()?.strip_suffix(".json").map(String::from)).collect()).unwrap_or_default();
    for n in nomes {
        if let Ok(r) = buscar(conta, &n, video) {
            let ms = metricas(&r);
            for (linha, metrica) in &m {
                if let Some(v) = metrica.as_str().and_then(|mm| ms.iter().find(|x| casa(&x.0, mm))) {
                    out.insert(linha.clone(), json!(v.1));
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------- extensão do Chrome (native messaging)

pub const HOST_NOME: &str = "com.video_factory.studio";
/// id fixo da extensão (vem da "key" do manifest.json de tools/vf-studio-ext)
pub const EXT_ID: &str = "gepfeoocoahfmnnkpmlakpffedddabmm";

pub fn dados_dir() -> PathBuf {
    dir().join("dados")
}

fn host_manifestos() -> Vec<PathBuf> {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    let bases: Vec<PathBuf> = if cfg!(target_os = "macos") {
        let a = home.join("Library/Application Support");
        vec![a.join("Google/Chrome"), a.join("Google/Chrome Beta"), a.join("Chromium"), a.join("BraveSoftware/Brave-Browser"), a.join("Microsoft Edge"), a.join("Arc/User Data")]
    } else {
        let c = home.join(".config");
        vec![c.join("google-chrome"), c.join("chromium"), c.join("BraveSoftware/Brave-Browser"), c.join("microsoft-edge")]
    };
    bases.into_iter().filter(|b| b.is_dir()).map(|b| b.join("NativeMessagingHosts").join(format!("{HOST_NOME}.json"))).collect()
}

fn instalar() -> Result<()> {
    let exe = std::env::current_exe()?.canonicalize()?;
    // o Chrome executa um arquivo sem argumentos extras: um wrapper chama "vf studio host"
    let wrapper = dir().join("vf-studio-host.sh");
    std::fs::create_dir_all(dir())?;
    std::fs::write(&wrapper, format!("#!/bin/sh\nexec \"{}\" studio host \"$@\"\n", exe.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755))?;
    }
    let manifesto = json!({
        "name": HOST_NOME, "description": "vf studio: recebe as métricas capturadas no YouTube Studio",
        "path": wrapper.display().to_string(), "type": "stdio", "allowed_origins": [format!("chrome-extension://{EXT_ID}/")]
    });
    let alvos = host_manifestos();
    if alvos.is_empty() {
        bail!("nenhum navegador Chromium encontrado (Chrome, Brave, Edge, Arc)");
    }
    for p in &alvos {
        std::fs::create_dir_all(p.parent().unwrap())?;
        crate::util::write_json(p, &manifesto)?;
        println!("host registrado: {}", p.display());
    }
    let ext = crate::init::plugin_root().map(|r| r.join("tools/vf-studio-ext")).unwrap_or_else(|_| PathBuf::from("tools/vf-studio-ext"));
    println!("\nAgora no Chrome: chrome://extensions → ative o Modo do desenvolvedor → \"Carregar sem compactação\" →\n  {}\n(id esperado: {EXT_ID}). Depois abra as análises de um vídeo no Studio ou use o botão Coletar da extensão.", ext.display());
    Ok(())
}

fn desinstalar() -> Result<()> {
    for p in host_manifestos() {
        if p.exists() {
            std::fs::remove_file(&p)?;
            println!("removido: {}", p.display());
        }
    }
    let _ = std::fs::remove_file(dir().join("vf-studio-host.sh"));
    println!("host desinstalado; as capturas continuam em {} (apague se quiser)", dados_dir().display());
    Ok(())
}

fn ler_msg(r: &mut impl std::io::Read) -> Option<Value> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len).ok()?;
    let n = u32::from_le_bytes(len) as usize;
    if n > 64 * 1024 * 1024 {
        return None;
    }
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

fn escrever_msg(w: &mut impl std::io::Write, v: &Value) -> Result<()> {
    let b = serde_json::to_vec(v)?;
    w.write_all(&(b.len() as u32).to_le_bytes())?;
    w.write_all(&b)?;
    w.flush()?;
    Ok(())
}

/// id de vídeo (11) ou "canal_UC..." (páginas de análise do canal)
fn id_ok(s: &str) -> bool {
    let ok = |t: &str| t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    (s.len() == 11 && ok(s)) || (s.starts_with("canal_UC") && s.len() <= 40 && ok(s))
}

/// grava uma captura: dados/<vídeo>/<endpoint>.json (a mais recente de cada endpoint)
pub fn salvar_captura(m: &Value) -> Result<PathBuf> {
    let video = m["video"].as_str().unwrap_or("");
    if !id_ok(video) {
        bail!("id de vídeo inválido");
    }
    let ep: String = m["endpoint"].as_str().unwrap_or("x").chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).take(80).collect();
    let d = dados_dir().join(video);
    std::fs::create_dir_all(&d)?;
    // uma resposta por arquivo (o Studio chama o mesmo endpoint várias vezes, um cartão por vez)
    let corpo = serde_json::to_vec(&m["dados"])?;
    let h: String = Sha1::digest(&corpo).iter().take(4).map(|b| format!("{b:02x}")).collect();
    let p = d.join(format!("{}-{h}.json", if ep.is_empty() { "x".to_string() } else { ep }));
    crate::util::write_json(&p, &json!({ "video": video, "endpoint": m["endpoint"], "pagina": m["pagina"], "quando": m["quando"], "dados": m["dados"] }))?;
    // teto por vídeo: as 80 mais recentes
    let mut fs: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&d)?.flatten().filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path()))).collect();
    if fs.len() > 80 {
        fs.sort();
        for (_, f) in &fs[..fs.len() - 80] {
            let _ = std::fs::remove_file(f);
        }
    }
    Ok(p)
}

/// métricas de todas as respostas capturadas de um vídeo
pub fn metricas_capturadas(video: &str) -> Vec<(String, f64, String)> {
    if !id_ok(video) {
        return Vec::new();
    }
    let mut out = Vec::new();
    // mais recente primeiro: no mapeamento vale o valor mais novo
    let mut arqs: Vec<(std::time::SystemTime, std::fs::DirEntry)> = std::fs::read_dir(dados_dir().join(video)).into_iter().flatten().flatten().filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e))).collect();
    arqs.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, e) in arqs {
        if let Some(v) = std::fs::read(e.path()).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) {
            let nome = e.file_name().to_string_lossy().trim_end_matches(".json").to_string();
            for (n, x, c) in metricas(&v["dados"]) {
                out.push((n, x, format!("{nome}{}", c.trim_start_matches('$'))));
            }
        }
    }
    let mut vistos = std::collections::HashSet::new();
    out.retain(|x| vistos.insert((x.0.clone(), x.2.clone())));
    out
}

/// loop do native messaging: o Chrome abre este processo por mensagem (sendNativeMessage)
fn host() -> Result<()> {
    let mut entrada = std::io::stdin().lock();
    let mut saida = std::io::stdout().lock();
    while let Some(m) = ler_msg(&mut entrada) {
        let resp = match m["tipo"].as_str().unwrap_or("") {
            "ping" => json!({ "ok": true, "versao": env!("CARGO_PKG_VERSION") }),
            "captura" => match salvar_captura(&m) {
                Ok(_) => json!({ "ok": true }),
                Err(e) => json!({ "erro": format!("{e:#}") }),
            },
            "contas" => {
                let padrao = crate::auth::resolve("yt", None).ok();
                json!({ "contas": crate::auth::accounts("yt").into_iter().map(|c| json!({ "apelido": c, "padrao": padrao.as_deref() == Some(c.as_str()) })).collect::<Vec<_>>() })
            }
            "listar" => match listar(m["conta"].as_str(), m["max"].as_u64().unwrap_or(50) as usize) {
                Ok(v) => json!({ "videos": v }),
                Err(e) => json!({ "erro": format!("{e:#}") }),
            },
            _ => json!({ "erro": "tipo desconhecido" }),
        };
        escrever_msg(&mut saida, &resp)?;
    }
    Ok(())
}

/// vídeos públicos mais recentes do canal (Data API), para a coleta ativa
fn listar(conta: Option<&str>, max: usize) -> Result<Vec<Value>> {
    let (_, t) = crate::auth::token("yt", conta)?;
    let b = crate::platform::bearer(&t);
    let ch = crate::platform::yt_get(&b, "channels", &[("part", "contentDetails"), ("mine", "true")])?;
    let up = ch.pointer("/items/0/contentDetails/relatedPlaylists/uploads").and_then(|x| x.as_str()).context("canal sem uploads")?.to_string();
    let n = max.clamp(1, 50).to_string();
    let v = crate::platform::yt_get(&b, "playlistItems", &[("part", "contentDetails"), ("playlistId", &up), ("maxResults", &n)])?;
    let ids: Vec<String> = v["items"].as_array().cloned().unwrap_or_default().iter().filter_map(|x| x.pointer("/contentDetails/videoId").and_then(|v| v.as_str()).map(String::from)).collect();
    Ok(crate::platform::yt_videos(&b, &ids)?
        .into_iter()
        .filter(|v| v["privacidade"] == "public")
        .map(|v| json!({ "id": v["id"], "titulo": v["titulo"], "publicado": v["publicado"] }))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curl_do_chrome() {
        let c = r#"curl 'https://studio.youtube.com/youtubei/v1/yta_web/get_screen?alt=json' \
  -H 'accept: */*' \
  -H 'content-type: application/json' \
  -b 'VISITOR=1; SAPISID=abc/def; __Secure-3PAPISID=xyz' \
  -H 'x-goog-authuser: 1' \
  -H 'x-goog-pageid: 1234' \
  --data-raw $'{"screenConfig":{"entity":{"videoId":"wuXBs0A6-14"}},"q":"it\'s"}'"#;
        let p = parse_curl(c).unwrap();
        assert!(p.url.ends_with("get_screen?alt=json"));
        assert_eq!(cookie_val(&p.cookie, "SAPISID").unwrap(), "abc/def");
        assert!(p.headers.iter().any(|(k, v)| k == "x-goog-pageid" && v == "1234"));
        let corpo: Value = serde_json::from_str(p.corpo.as_deref().unwrap()).unwrap();
        assert_eq!(corpo["q"], "it's");
        let mut ids = Vec::new();
        ids_no_corpo(&corpo, &mut ids);
        assert_eq!(ids, vec!["wuXBs0A6-14"]);
        assert!(parse_curl("curl 'https://www.google.com/x' -b 'SAPISID=1'").is_err());
        assert!(parse_curl("curl 'https://studio.youtube.com/youtubei/v1/x' -b 'SID=1'").is_err());
    }
    #[test]
    fn hash() {
        let a = sapisid_auth("SAPISID=abc; __Secure-3PAPISID=xyz", 1700000000).unwrap();
        // sha1("1700000000 abc https://studio.youtube.com")
        let esperado: String = Sha1::digest(b"1700000000 abc https://studio.youtube.com").iter().map(|b| format!("{b:02x}")).collect();
        assert!(a.starts_with(&format!("SAPISIDHASH 1700000000_{esperado}")));
        assert!(a.contains("SAPISID3PHASH 1700000000_") && !a.contains("SAPISID1PHASH"));
    }
    #[test]
    fn protocolo_nativo() {
        let mut buf = Vec::new();
        escrever_msg(&mut buf, &json!({ "tipo": "ping" })).unwrap();
        assert_eq!(u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize, buf.len() - 4);
        let v = ler_msg(&mut std::io::Cursor::new(buf)).unwrap();
        assert_eq!(v["tipo"], "ping");
        assert!(ler_msg(&mut std::io::Cursor::new(vec![1u8, 2])).is_none());
    }
    #[test]
    fn captura_e_leitura() {
        let tmp = std::env::temp_dir().join(format!("vf-studio-teste-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &tmp);
        let m = json!({ "video": "wuXBs0A6-14", "endpoint": "yta_web/get_screen", "dados": { "cards": [{ "metric": "SHORTS_VIEWED_RATE", "value": 0.4955 }] } });
        let p = salvar_captura(&m).unwrap();
        let nome = p.file_name().unwrap().to_string_lossy().to_string();
        assert!(p.parent().unwrap().ends_with("wuXBs0A6-14") && nome.starts_with("yta_web_get_screen-") && nome.ends_with(".json"), "{nome}");
        // a mesma resposta não duplica; outra resposta do mesmo endpoint vira outro arquivo
        assert_eq!(salvar_captura(&m).unwrap(), p);
        let mut m2 = m.clone();
        m2["dados"]["cards"][0]["value"] = json!(0.5);
        assert_ne!(salvar_captura(&m2).unwrap(), p);
        let ms = metricas_capturadas("wuXBs0A6-14");
        assert!(ms.iter().any(|x| x.0 == "SHORTS_VIEWED_RATE"));
        assert!(salvar_captura(&json!({ "video": "../../etc/x", "dados": {} })).is_err(), "id inválido não escreve fora da pasta");
        assert!(metricas_capturadas("../x").is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }
    #[test]
    fn numeros_da_tela() {
        assert_eq!(numero_tela("49,55%"), Some(49.55));
        assert_eq!(numero_tela("1,2 mil"), Some(1200.0));
        assert_eq!(numero_tela("2,3 mi"), Some(2_300_000.0));
        assert_eq!(numero_tela("1.234"), Some(1234.0));
        assert_eq!(numero_tela("0:42"), Some(42.0));
        assert_eq!(numero_tela("359"), Some(359.0));
        assert_eq!(numero_tela("12.5K"), Some(12500.0));
        assert_eq!(numero_tela("Hoje, 18:49"), None);
        assert_eq!(numero_tela("Feed dos Shorts"), None);
    }
    #[test]
    fn hub_e_join() {
        let hub = json!({ "cardViewModel": { "items": [
            { "listItemViewModel": { "title": { "content": "Continuaram assistindo" } } },
            { "simpleMetricViewModel": { "value": { "content": "49,6%" } } },
            { "listItemViewModel": { "title": { "content": "Deslizaram" } } },
            { "simpleMetricViewModel": { "value": { "content": "50,4%" } } } ] },
            "outros": [{ "horizontalBarViewModel": { "accessibilityLabel": "Feed dos Shorts", "segments": [{ "percentage": 0.98 }] } }] });
        let real = json!({ "cardViewModel": { "accessibilityId": "analytics_hub.video_overview_shorts_how_viewers_engaged", "bodyContent": { "linearLayoutViewModel": { "items": [
            { "horizontalBarViewModel": { "segments": [{ "percentage": 0.5307 }, { "percentage": 0.4693 }] } },
            { "linearLayoutViewModel": { "items": [
                { "simpleMetricViewModel": { "subtitle": { "content": "Continuaram assistindo" }, "value": { "content": "53,1%" } } },
                { "simpleMetricViewModel": { "subtitle": { "content": "Ignorado" }, "value": { "content": "46,9%" } } } ] } } ] } } } });
        let pr = pares_texto(&real);
        assert!(pr.iter().any(|x| x.0 == "TEXTO:Continuaram assistindo" && (x.1 - 53.1).abs() < 1e-9), "{pr:?}");
        assert!(pr.iter().any(|x| x.0 == "TEXTO:Ignorado" && (x.1 - 46.9).abs() < 1e-9));
        assert!(pr.iter().any(|x| x.0 == "STUDIO:SHORTS_CONTINUARAM" && (x.1 - 53.07).abs() < 1e-9));
        let p = pares_texto(&hub);
        assert!(p.iter().any(|x| x.0 == "TEXTO:Continuaram assistindo" && (x.1 - 49.6).abs() < 1e-9), "{p:?}");
        assert!(p.iter().any(|x| x.0 == "TEXTO:Deslizaram" && (x.1 - 50.4).abs() < 1e-9));
        assert!(p.iter().any(|x| x.0 == "TEXTO:Feed dos Shorts" && (x.1 - 98.0).abs() < 1e-9));
        assert!(casa("TEXTO:Continuaram assistindo", "TEXTO:Continuaram assistindo|TEXTO:Stayed to watch"));
        assert!(casa("TEXTO:Stayed to watch", "TEXTO:continuaram assistindo|TEXTO:stayed to watch"));
        let join = json!({ "metricColumns": [{ "counts": { "values": [0, 2, 3] }, "metric": { "type": "SUBSCRIBERS_NET_CHANGE", "cumulative": true } },
                                              { "counts": { "values": [10, 20] }, "metric": { "type": "VIEWS" } }] });
        let m = metricas(&join);
        assert!(m.iter().any(|x| x.0 == "SUBSCRIBERS_NET_CHANGE" && x.1 == 3.0));
        assert!(m.iter().any(|x| x.0 == "VIEWS" && x.1 == 30.0));
    }
    #[test]
    fn acha_metricas() {
        let r = json!({ "cards": [{ "metric": "SHORTS_FEED_VIEWED_RATE", "value": 0.4955 }, { "key": "VIEWS", "total": "365" }], "ENGAGED_VIEWS": 164, "ignorar": { "type": "abc", "value": 1 } });
        let m = metricas(&r);
        assert!(m.iter().any(|x| x.0 == "SHORTS_FEED_VIEWED_RATE" && (x.1 - 0.4955).abs() < 1e-9));
        assert!(m.iter().any(|x| x.0 == "VIEWS" && x.1 == 365.0));
        assert!(m.iter().any(|x| x.0 == "ENGAGED_VIEWS" && x.1 == 164.0));
        assert!(!m.iter().any(|x| x.0 == "abc"));
    }
}
