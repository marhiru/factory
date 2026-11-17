//! Argumentos no estilo `vf init type="short,long" platform="yt,tiktok"`.
//! Aceita `chave=valor`, `--chave valor`, `--chave=valor` e `--flag` (vira "1").
use anyhow::{bail, Result};
use std::collections::BTreeMap;

pub struct Kv {
    map: BTreeMap<String, String>,
}

impl Kv {
    pub fn parse(args: &[String], flags: &[&str]) -> Result<Kv> {
        let mut map = BTreeMap::new();
        let mut i = 0;
        while i < args.len() {
            let a = &args[i];
            let body = a.trim_start_matches('-');
            if let Some((k, v)) = body.split_once('=') {
                map.insert(norm(k), v.trim_matches('"').to_string());
            } else if a.starts_with('-') && a.len() > 1 {
                let k = norm(body);
                if flags.contains(&k.as_str()) || i + 1 >= args.len() || args[i + 1].starts_with('-') {
                    map.insert(k, "1".into());
                } else {
                    i += 1;
                    map.insert(k, args[i].clone());
                }
            } else {
                bail!("argumento inesperado: {a} (use chave=valor, ex.: platform=yt)");
            }
            i += 1;
        }
        Ok(Kv { map })
    }
    pub fn get(&self, k: &str) -> Option<&str> {
        self.map.get(k).map(|s| s.as_str())
    }
    pub fn flag(&self, k: &str) -> bool {
        matches!(self.get(k), Some("1" | "true" | "sim" | "yes"))
    }
    /// lista separada por vírgula, em minúsculas e sem vazios
    pub fn list(&self, k: &str) -> Vec<String> {
        self.get(k)
            .map(|v| v.split(',').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default()
    }
    /// pares na forma normalizada (para regravar os argumentos, ex. na agenda)
    pub fn pairs(&self) -> impl Iterator<Item = (&String, &String)> {
        self.map.iter()
    }
    pub fn check_known(&self, known: &[&str]) -> Result<()> {
        for k in self.map.keys() {
            if !known.contains(&k.as_str()) {
                bail!("opção desconhecida: {k} (conhecidas: {})", known.join(", "));
            }
        }
        Ok(())
    }
}

fn norm(k: &str) -> String {
    k.trim().to_lowercase().replace('-', "_")
}

/// yt | youtube -> "yt"; tiktok | tt -> "tiktok"
pub fn platform(p: &str) -> Result<&'static str> {
    Ok(match p {
        "yt" | "youtube" => "yt",
        "tiktok" | "tt" => "tiktok",
        "tiktok-biz" | "tiktok_biz" | "ttb" | "tiktok-business" => "ttb",
        _ => bail!("plataforma inválida: {p} (yt | tiktok | tiktok-biz)"),
    })
}

pub fn platforms(kv: &Kv) -> Result<Vec<&'static str>> {
    let mut out = Vec::new();
    for p in kv.list("platform").iter().chain(kv.list("platforms").iter()) {
        let p = platform(p)?;
        if !out.contains(&p) {
            out.push(p);
        }
    }
    Ok(out)
}
