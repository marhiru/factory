//! HTTP pelo `curl` do sistema. A configuração (URL, cabeçalhos, corpo) vai pelo stdin (`curl -K -`),
//! então tokens e segredos nunca aparecem na linha de comando (`ps`).
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct Resp {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl Resp {
    pub fn json(&self) -> Result<serde_json::Value> {
        serde_json::from_slice(&self.body).with_context(|| format!("resposta não é JSON (HTTP {}): {}", self.status, self.text()))
    }
    pub fn text(&self) -> String {
        let s = String::from_utf8_lossy(&self.body);
        s.chars().take(1500).collect()
    }
    pub fn ok(self) -> Result<Resp> {
        if !(200..300).contains(&self.status) {
            bail!("HTTP {}: {}", self.status, self.text());
        }
        Ok(self)
    }
}

pub enum Body<'a> {
    None,
    Form(&'a [(&'a str, &'a str)]),
    Json(&'a serde_json::Value),
    File(&'a Path),
}

fn q(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            _ => o.push(c),
        }
    }
    o.push('"');
    o
}

pub fn request(method: &str, url: &str, headers: &[(&str, &str)], body: Body) -> Result<Resp> {
    let dir = std::env::temp_dir().join(format!("vf-net-{}-{}", std::process::id(), rand_hex(4)));
    std::fs::create_dir_all(&dir)?;
    let hfile = dir.join("h");
    let bfile = dir.join("b");
    let mut cfg = String::new();
    cfg += &format!("url = {}\nrequest = {}\nsilent\nshow-error\nlocation\n", q(url), q(method));
    cfg += &format!("dump-header = {}\noutput = {}\nwrite-out = \"%{{http_code}}\"\n", q(&hfile.display().to_string()), q(&bfile.display().to_string()));
    for (k, v) in headers {
        cfg += &format!("header = {}\n", q(&format!("{k}: {v}")));
    }
    let mut _jfile = None;
    match body {
        Body::None => {}
        Body::Form(pairs) => {
            for (k, v) in pairs {
                cfg += &format!("data-urlencode = {}\n", q(&format!("{k}={v}")));
            }
        }
        Body::Json(v) => {
            let jf = dir.join("j.json");
            std::fs::write(&jf, serde_json::to_vec(v)?)?;
            cfg += "header = \"Content-Type: application/json; charset=UTF-8\"\n";
            cfg += &format!("data-binary = {}\n", q(&format!("@{}", jf.display())));
            _jfile = Some(jf);
        }
        Body::File(p) => {
            cfg += &format!("upload-file = {}\n", q(&p.display().to_string()));
        }
    }
    let mut child = Command::new("curl")
        .args(["-K", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("curl não encontrado")?;
    child.stdin.take().unwrap().write_all(cfg.as_bytes())?;
    let out = child.wait_with_output()?;
    let status: u16 = String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0);
    let res = (|| {
        if status == 0 {
            bail!("curl falhou: {}", String::from_utf8_lossy(&out.stderr).trim());
        }
        let raw = std::fs::read_to_string(&hfile).unwrap_or_default();
        // com redirects, vale o último bloco de cabeçalhos
        let last = raw.split("\r\n\r\n").filter(|b| !b.trim().is_empty()).last().unwrap_or("");
        let headers = last
            .lines()
            .skip(1)
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_string()))
            .collect();
        Ok(Resp { status, headers, body: std::fs::read(&bfile).unwrap_or_default() })
    })();
    let _ = std::fs::remove_dir_all(&dir);
    res
}

pub fn get(url: &str, headers: &[(&str, &str)]) -> Result<Resp> {
    request("GET", url, headers, Body::None)
}

/// percent-encoding de componente de URL (RFC 3986, não reservados intactos)
pub fn enc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => o.push(b as char),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

pub fn query(pairs: &[(&str, &str)]) -> String {
    pairs.iter().map(|(k, v)| format!("{}={}", enc(k), enc(v))).collect::<Vec<_>>().join("&")
}

pub fn dec(s: &str) -> String {
    let b = s.as_bytes();
    let mut o = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => o.push(b' '),
            b'%' if i + 2 < b.len() => match std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                Some(v) => {
                    o.push(v);
                    i += 3;
                    continue;
                }
                None => o.push(b'%'),
            },
            c => o.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&o).into_owned()
}

pub fn rand_bytes(n: usize) -> Vec<u8> {
    use std::io::Read;
    let mut v = vec![0u8; n];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        let _ = f.read_exact(&mut v);
    }
    v
}

pub fn rand_hex(n: usize) -> String {
    rand_bytes(n).iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enc_dec() {
        assert_eq!(enc("a b/é"), "a%20b%2F%C3%A9");
        assert_eq!(dec("a%20b+c%2Fx%"), "a b c/x%");
    }
    #[test]
    #[ignore] // precisa do servidor de eco em 127.0.0.1:8799
    fn local_echo() {
        let r = request("POST", "http://127.0.0.1:8799/t?x=1", &[("Authorization", "Bearer s\"ecret")], Body::Form(&[("code", "a&b=c"), ("k", "v")])).unwrap();
        let v = r.json().unwrap();
        assert_eq!(v["auth"], "Bearer s\"ecret");
        assert_eq!(v["body"], "code=a%26b%3Dc&k=v");
        assert_eq!(r.headers.get("location").unwrap(), "http://127.0.0.1:8799/up");
        let j = serde_json::json!({"a": "ç"});
        let v = request("POST", "http://127.0.0.1:8799/j", &[], Body::Json(&j)).unwrap().json().unwrap();
        assert_eq!(v["body"], "{\"a\":\"ç\"}");
        let f = std::env::temp_dir().join("vf-net-test.bin");
        std::fs::write(&f, b"hello").unwrap();
        let v = request("PUT", "http://127.0.0.1:8799/u", &[("Content-Range", "bytes 0-4/5")], Body::File(&f)).unwrap().json().unwrap();
        assert_eq!(v["m"], "PUT");
        assert_eq!(v["body"], "hello");
        assert_eq!(v["range"], "bytes 0-4/5");
        let v = request("POST", "http://127.0.0.1:8799/thumb", &[], Body::File(&f)).unwrap().json().unwrap();
        assert_eq!(v["m"], "POST");
        assert_eq!(v["body"], "hello");
    }
}
