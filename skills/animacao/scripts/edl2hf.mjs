#!/usr/bin/env bun
// Compila data/edl.json numa composição HyperFrames (HTML + GSAP, sem CDN) em <projeto>/hf/.
// Renderer dos vídeos longos: stills com zoom/pan (mesma matemática do mygo/Remotion), vídeo, texto,
// cor, cartelas de interrupção de padrão (`card`) e acabamento de filme (`overlays`: grão, vinheta,
// light leak, graduação). Sai sem áudio: o som entra no `vf export --audio` (igual ao mygo).
// uso: bun edl2hf.mjs [data/edl.json] [--out hf] [--formato yt-long] [--scale 1]
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";

const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf(`--${k}`); return i >= 0 ? args[i + 1] : d; };
const pos = args.filter((a, i) => !a.startsWith("--") && !(args[i - 1] ?? "").startsWith("--"));
const edlPath = path.resolve(pos[0] ?? "data/edl.json");
const proj = path.dirname(path.dirname(edlPath));
const out = path.resolve(proj, opt("out", "hf"));
const SIZES = { "yt-long": [1920, 1080], "yt-short": [1080, 1920], tiktok: [1080, 1920] };
const formato = opt("formato", "yt-long");
const scale = Number(opt("scale", "1"));
const [W, H] = (SIZES[formato] ?? SIZES["yt-long"]).map((v) => Math.round(v * scale));
const edl = JSON.parse(fs.readFileSync(edlPath, "utf8"));
const FPS = edl.fps ?? 30;
const D = edl.duration;
const U = Math.min(W, H) / 1080; // escala de px

const bad = (edl.clips ?? []).filter((c) => c.type === "scene");
if (bad.length) {
  console.error(`edl2hf: ${bad.length} clipe(s) \`scene\` (cutout) não são suportados pelo HyperFrames: use renderer mygo para esse projeto`);
  process.exit(1);
}

// ---------- assets ----------
fs.mkdirSync(out, { recursive: true });
const pub = path.join(out, "public");
try { fs.unlinkSync(pub); } catch {}
fs.symlinkSync(path.relative(out, path.join(proj, "public")), pub);
let gsapSrc;
for (const base of [proj, out, path.dirname(new URL(import.meta.url).pathname)]) {
  try { gsapSrc = createRequire(path.join(base, "x.js")).resolve("gsap/dist/gsap.min.js"); break; } catch {}
}
if (!gsapSrc) { console.error("edl2hf: gsap não encontrado: `bun add -d gsap hyperframes` no projeto"); process.exit(1); }
fs.copyFileSync(gsapSrc, path.join(out, "gsap.min.js"));
fs.mkdirSync(path.join(out, "fx"), { recursive: true });
const grain = path.join(out, "fx", "grain.png");
if (!fs.existsSync(grain)) {
  // ruído monocromático 512x512 em torno de cinza médio (para mix-blend overlay)
  execFileSync("ffmpeg", ["-v", "error", "-y", "-f", "lavfi", "-i", "color=c=0x808080:s=512x512,noise=alls=60:allf=u,format=gray", "-frames:v", "1", grain]);
}

// ---------- helpers ----------
const esc = (s) => String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
const r3 = (v) => Math.round(v * 1000) / 1000;
// "1.889" / "12.000" / "3,5" -> número (separador de milhar some; vírgula decimal vira ponto)
const num = (t) => {
  let s = String(t).match(/-?[\d.,]+/)?.[0] ?? "0";
  if (/\d[.,]\d{3}(?!\d)/.test(s)) s = s.replace(/[.,](?=\d{3}(?!\d))/g, "");
  return Number(s.replace(",", ".")) || 0;
};
const src = (p) => `public/${String(p).replace(/^\/+/, "")}`;
const font = edl.font?.family ? `'${edl.font.family}', ` : "";
const FONT = `${font}'Inter', 'Helvetica Neue', Arial, sans-serif`;
const SERIF = edl.font_serif?.family ? `'${edl.font_serif.family}', Georgia, serif` : "Georgia, 'Times New Roman', serif";

const clips = [...(edl.clips ?? [])].map((c, k) => ({ ...c, _k: k }));
// fade: estende o clipe anterior da mesma camada para cobrir a entrada (como no Remotion/mygo)
for (const c of clips) {
  if (c.transition_in?.kind === "fade" || c.transition_in?.kind === "blur") {
    const prev = clips.filter((p) => (p.layer ?? 0) === (c.layer ?? 0) && p !== c && Math.abs(p.end - c.start) < 0.05).at(-1);
    if (prev) prev._extend = Math.max(prev._extend ?? 0, c.transition_in.dur ?? 0.3);
  }
}

const els = [];
const tl = []; // linhas JS da timeline
const T = (s) => r3(s);
const dur = (c) => r3(c.end - c.start + (c._extend ?? 0));
const z = (c) => (c.layer ?? 0) * 100 + (c.type === "card" ? 50 : 0) + Math.min(49, c._k % 50);

for (const c of clips) {
  const id = `c_${String(c.id ?? c._k).replace(/[^\w-]/g, "_")}`;
  const attrs = `id="${id}" class="clip" data-start="${T(c.start)}" data-duration="${dur(c)}" style="z-index:${z(c)}"`;
  const frames = Math.max(1, Math.round(dur(c) * FPS)); // inclui a extensão do fade, como no mygo/Remotion
  const tdur = r3(Math.max(1, frames - 1) / FPS); // interpolate([0, dur-1]) dos outros renderers
  const motion = (sel, m, layerDepth = 0) => {
    if (!m) return;
    const amp = 1 + layerDepth * 0.8;
    const P = (p) => ({ s: 1 + ((p?.scale ?? 1) - 1) * amp, x: (p?.x ?? 0) * amp * W, y: (p?.y ?? 0) * amp * H, r: p?.rotate ?? 0 });
    const a = P(m.from), b = P({ scale: m.to?.scale ?? m.from?.scale, x: m.to?.x ?? m.from?.x, y: m.to?.y ?? m.from?.y, rotate: m.to?.rotate ?? m.from?.rotate });
    tl.push(`tl.fromTo("${sel}", {x:${r3(a.x)}, y:${r3(a.y)}, scale:${r3(a.s)}, rotation:${a.r}}, {x:${r3(b.x)}, y:${r3(b.y)}, scale:${r3(b.s)}, rotation:${b.r}, duration:${tdur}, ease:E("${m.ease ?? "linear"}")}, ${T(c.start)});`);
  };
  let inner = "";
  if (c.type === "image") {
    const layers = c.layers?.length ? c.layers : [{ src: c.src, depth: 0 }];
    const m = c.motion ?? { from: { scale: 1 }, to: { scale: 1.1 }, ease: "linear" };
    inner = layers.map((l, j) => `<div class="kb" id="${id}_l${j}"><img src="${esc(src(l.src))}" style="object-fit:${c.fit ?? "cover"}"></div>`).join("");
    layers.forEach((l, j) => motion(`#${id}_l${j}`, m, l.depth ?? 0));
  } else if (c.type === "video") {
    inner = `<div class="kb" id="${id}_v"><video id="${id}_vid" src="${esc(src(c.src))}" muted data-start="${T(c.start)}" data-duration="${dur(c)}" data-media-start="${c.trim ?? 0}"${c.playbackRate ? ` data-playback-rate="${c.playbackRate}"` : ""} style="object-fit:${c.fit ?? "cover"}"></video></div>`;
    motion(`#${id}_v`, c.motion);
  } else if (c.type === "color") {
    inner = `<div class="fill" style="background:${esc(c.color ?? "#000")}"></div>`;
  } else if (c.type === "text") {
    const st = c.style ?? "title";
    const col = esc(c.color ?? "#fff");
    if (st === "number") {
      const target = num(c.text);
      const grouped = /\d[.,]\d{3}/.test(c.text);
      inner = `<div class="center"><div class="t-number" id="${id}_n" style="color:${col}">${esc(c.text)}</div></div>`;
      tl.push(`count("#${id}_n", ${JSON.stringify(c.text)}, ${target}, ${grouped}, ${T(c.start)}, ${r3(Math.min(c.end - c.start, 1.2))});`);
      tl.push(`tl.fromTo("#${id}_n", {scale:0.6, opacity:0}, {scale:1, opacity:1, duration:0.35, ease:"back.out(1.7)"}, ${T(c.start)});`);
    } else if (st === "lower") {
      inner = `<div class="lower"><div class="t-lower" id="${id}_t" style="color:${col}">${esc(c.text)}</div></div>`;
      tl.push(`tl.fromTo("#${id}_t", {xPercent:-120}, {xPercent:0, duration:0.45, ease:"power3.out"}, ${T(c.start)});`);
    } else if (st === "label") {
      inner = `<div class="label"><div class="t-label" id="${id}_t">${esc(c.text)}</div></div>`;
      tl.push(`tl.fromTo("#${id}_t", {scale:0}, {scale:1, duration:0.35, ease:"back.out(2)"}, ${T(c.start)});`);
    } else {
      inner = `<div class="center"><div class="t-title" id="${id}_t" style="color:${col}">${words(c.text, `${id}_w`)}</div></div>`;
      tl.push(`tl.fromTo("#${id}_t .w", {yPercent:60, opacity:0}, {yPercent:0, opacity:1, duration:0.4, stagger:0.06, ease:"power3.out"}, ${T(c.start)});`);
    }
    tl.push(`tl.to("#${id} > div", {opacity:0, duration:0.2}, ${T(c.end - 0.2)});`);
  } else if (c.type === "card") {
    inner = card(c, id);
  } else {
    console.error(`edl2hf: tipo de clipe desconhecido '${c.type}' em ${c.id}`);
    process.exit(1);
  }
  // transições de entrada
  const tr = c.transition_in;
  if (tr?.kind === "fade") tl.push(`tl.fromTo("#${id}", {opacity:0}, {opacity:1, duration:${tr.dur ?? 0.3}, ease:"none"}, ${T(c.start)});`);
  if (tr?.kind === "blur") tl.push(`tl.fromTo("#${id}", {opacity:0, filter:"blur(${Math.round(24 * U)}px)"}, {opacity:1, filter:"blur(0px)", duration:${tr.dur ?? 0.5}, ease:"power2.out"}, ${T(c.start)});`);
  if (tr?.kind === "slide") tl.push(`tl.fromTo("#${id}", {x:${W}}, {x:0, duration:${tr.dur ?? 0.3}, ease:"power3.out"}, ${T(c.start)});`);
  if (tr?.kind === "flash" || tr?.kind === "dip") {
    const color = tr.kind === "flash" ? "#fff" : "#000";
    inner += `<div class="fill" id="${id}_fl" style="background:${color}"></div>`;
    tl.push(`tl.fromTo("#${id}_fl", {opacity:1}, {opacity:0, duration:${tr.dur ?? 0.2}, ease:"none"}, ${T(c.start)});`);
  }
  if (tr?.kind === "punch") tl.push(`tl.fromTo("#${id}", {scale:1.06}, {scale:1, duration:${tr.dur ?? 0.18}, ease:"power2.out"}, ${T(c.start)});`);
  // zoom punch no meio do plano (revelação): {"punch_at": s}
  if (typeof c.punch_at === "number") tl.push(`tl.fromTo("#${id}", {scale:1}, {scale:1.04, duration:0.15, ease:"power2.out", yoyo:false}, ${T(c.start + c.punch_at)});`);
  els.push(`<div ${attrs}>${inner}</div>`);
}

function words(text, pre) {
  return String(text).split(/\s+/).filter(Boolean).map((w, k) => `<span class="w" id="${pre}${k}">${esc(w)}</span>`).join(" ");
}

// cartelas de interrupção de padrão: kind = capitulo | dado | citacao | comparacao | mapa
function card(c, id) {
  const d = c.data ?? {};
  const t0 = T(c.start), len = c.end - c.start;
  // sem imagem: gradiente que deriva devagar (fundo chapado parado reprova no QA como congelado/preto)
  const bg = d.bg ? `<div class="kb" id="${id}_bg"><img src="${esc(src(d.bg))}"></div><div class="fill shade"></div>`
    : `<div class="kb" id="${id}_bg" style="background:radial-gradient(ellipse at 30% 30%, ${esc(d.color ?? "#2a3140")} 0%, #0b0d12 70%)"></div>`;
  if (!d.bg) tl.push(`tl.fromTo("#${id}_bg", {scale:1.0, x:0}, {scale:1.25, x:${r3(-80 * U)}, duration:${r3(len)}, ease:"none"}, ${t0});`);
  // o conteúdo do card respira (push lento) do começo ao fim
  tl.push(`tl.fromTo("#${id} .card", {scale:1}, {scale:1.04, duration:${r3(len)}, ease:"none"}, ${t0});`);
  if (d.bg) tl.push(`tl.fromTo("#${id}_bg", {scale:1.08, filter:"blur(${Math.round(10 * U)}px)"}, {scale:1.16, filter:"blur(${Math.round(10 * U)}px)", duration:${r3(len)}, ease:"none"}, ${t0});`);
  tl.push(`tl.to("#${id} .card", {opacity:0, duration:0.25, ease:"none"}, ${T(c.end - 0.25)});`);
  if (c.kind === "capitulo") {
    tl.push(`tl.fromTo("#${id} .k-num", {opacity:0, y:${r3(-18 * U)}}, {opacity:1, y:0, duration:0.6, ease:"power2.out"}, ${t0});`);
    tl.push(`tl.fromTo("#${id} .k-line", {scaleX:0}, {scaleX:1, duration:0.5, ease:"power3.inOut"}, ${T(c.start + 0.15)});`);
    tl.push(`tl.fromTo("#${id} .k-title .w", {yPercent:110}, {yPercent:0, duration:0.55, stagger:0.07, ease:"power3.out"}, ${T(c.start + 0.3)});`);
    return `${bg}<div class="card center col"><div class="k-num">${esc(d.numero ?? "")}</div><div class="k-line"></div><div class="k-title"><span class="mask">${words(d.titulo ?? c.text ?? "", `${id}_w`)}</span></div></div>`;
  }
  if (c.kind === "dado") {
    const target = num(d.valor);
    tl.push(`count("#${id}_n", ${JSON.stringify(String(d.valor))}, ${target}, ${/\d[.,]\d{3}/.test(String(d.valor))}, ${t0}, ${r3(Math.min(len * 0.5, 1.4))});`);
    tl.push(`tl.fromTo("#${id} .k-big", {scale:0.85, opacity:0}, {scale:1, opacity:1, duration:0.5, ease:"power3.out"}, ${t0});`);
    tl.push(`tl.fromTo("#${id} .k-sub", {y:${r3(30 * U)}, opacity:0}, {y:0, opacity:1, duration:0.5, ease:"power3.out"}, ${T(c.start + 0.35)});`);
    return `${bg}<div class="card center col"><div class="k-big" id="${id}_n">${esc(d.valor)}</div><div class="k-sub">${esc(d.legenda ?? "")}</div></div>`;
  }
  if (c.kind === "citacao") {
    const ws = String(d.texto ?? "").split(/\s+/).filter(Boolean);
    const per = Math.min(0.12, (len * 0.6) / Math.max(1, ws.length));
    tl.push(`tl.fromTo("#${id} .k-quote .w", {opacity:0.12}, {opacity:1, duration:0.2, stagger:${r3(per)}, ease:"none"}, ${t0});`);
    tl.push(`tl.fromTo("#${id} .k-autor", {opacity:0}, {opacity:1, duration:0.5}, ${T(c.start + Math.min(len * 0.7, ws.length * per + 0.2))});`);
    return `${bg}<div class="card center col"><div class="k-mark">“</div><div class="k-quote">${words(d.texto, `${id}_w`)}</div><div class="k-autor">${esc(d.autor ?? "")}</div></div>`;
  }
  if (c.kind === "comparacao") {
    const [a, b] = d.lados ?? [{}, {}];
    tl.push(`tl.fromTo("#${id} .k-a", {xPercent:-100}, {xPercent:0, duration:0.55, ease:"power3.out"}, ${t0});`);
    tl.push(`tl.fromTo("#${id} .k-b", {xPercent:100}, {xPercent:0, duration:0.55, ease:"power3.out"}, ${T(c.start + 0.12)});`);
    tl.push(`tl.fromTo("#${id} .k-side img", {scale:1.0}, {scale:1.08, duration:${r3(len)}, ease:"none"}, ${t0});`);
    const side = (s, cls) => `<div class="k-side ${cls}"><img src="${esc(src(s.img))}"><div class="k-cap">${esc(s.rotulo ?? "")}</div></div>`;
    return `<div class="fill" style="background:#07080b"></div><div class="card split">${side(a, "k-a")}${side(b, "k-b")}</div>`;
  }
  if (c.kind === "mapa") {
    const [px, py] = d.ponto ?? [0.5, 0.5];
    tl.push(`tl.fromTo("#${id}_map", {scale:1.0}, {scale:${d.zoom ?? 1.25}, transformOrigin:"${px * 100}% ${py * 100}%", duration:${r3(len)}, ease:E("inOut")}, ${t0});`);
    tl.push(`tl.fromTo("#${id} .k-pin", {scale:0}, {scale:1, duration:0.4, ease:"back.out(2.5)"}, ${T(c.start + 0.4)});`);
    tl.push(`tl.fromTo("#${id} .k-ring", {scale:0.4, opacity:0.9}, {scale:2.6, opacity:0, duration:1.2, repeat:${Math.max(0, Math.floor((len - 0.6) / 1.2) - 1)}, ease:"power1.out"}, ${T(c.start + 0.6)});`);
    tl.push(`tl.fromTo("#${id} .k-plabel", {opacity:0, x:${r3(-20 * U)}}, {opacity:1, x:0, duration:0.4}, ${T(c.start + 0.7)});`);
    return `<div class="kb" id="${id}_map"><img src="${esc(src(d.img))}"><div class="k-pinwrap" style="left:${px * 100}%;top:${py * 100}%"><div class="k-ring"></div><div class="k-pin"></div><div class="k-plabel">${esc(d.rotulo ?? "")}</div></div></div>`;
  }
  console.error(`edl2hf: card '${c.kind}' desconhecido em ${c.id}`);
  process.exit(1);
}

// ---------- acabamento (overlays) ----------
const ov = edl.overlays ?? [];
const ovEls = [];
let grade = "";
for (const [k, o] of ov.entries()) {
  if (o.kind === "grain") {
    const steps = Math.max(1, Math.round((D * FPS) / 2)); // grão troca a cada 2 frames (~15 Hz, cara de película)
    ovEls.push(`<div class="ov grain" id="ov${k}" style="opacity:${o.opacity ?? 0.07};background-size:${Math.round(512 * U * (o.size ?? 1))}px"></div>`);
    tl.push(`tl.fromTo("#ov${k}", {backgroundPosition:"0px 0px"}, {backgroundPosition:"${steps * 373}px ${steps * 211}px", duration:${r3(D)}, ease:"steps(${steps})"}, 0);`);
  } else if (o.kind === "vignette") {
    const s = o.strength ?? 0.4;
    ovEls.push(`<div class="ov" style="background:radial-gradient(ellipse at center, rgba(0,0,0,0) ${Math.round(55 - s * 20)}%, rgba(0,0,0,${s}) 100%)"></div>`);
  } else if (o.kind === "leak") {
    // light leak sutil nos instantes de `at` (ex.: inícios de capítulo), cor quente
    ovEls.push(`<div class="ov leak" id="ov${k}" style="opacity:0;background:radial-gradient(circle at 85% 20%, ${o.color ?? "rgba(255,170,90,.9)"} 0%, rgba(255,120,60,0) 55%)"></div>`);
    for (const t of o.at ?? []) {
      tl.push(`tl.fromTo("#ov${k}", {opacity:0, x:${r3(-120 * U)}}, {opacity:${o.opacity ?? 0.35}, x:0, duration:0.6, ease:"power1.out"}, ${T(t)});`);
      tl.push(`tl.to("#ov${k}", {opacity:0, x:${r3(120 * U)}, duration:1.4, ease:"power1.in"}, ${T(t + 0.6)});`);
    }
  } else if (o.kind === "grade") {
    grade = `filter: contrast(${o.contrast ?? 1}) saturate(${o.saturate ?? 1}) sepia(${o.sepia ?? 0}) brightness(${o.brightness ?? 1}) hue-rotate(${o.hue ?? 0}deg);`;
  } else if (o.kind === "letterbox") {
    console.error("edl2hf: overlay letterbox reprova no QA (barras pretas); não use");
    process.exit(1);
  }
}
if ((edl.captions?.enabled)) console.error("edl2hf: aviso: legendas não são desenhadas pelo HyperFrames (vídeo longo da fábrica é sem legenda queimada; use SRT no YouTube)");

const fontFace = edl.font?.src ? `@font-face{font-family:'${edl.font.family}';src:url('${src(edl.font.src)}');font-display:block}` : "";
const fontFace2 = edl.font_serif?.src ? `@font-face{font-family:'${edl.font_serif.family}';src:url('${src(edl.font_serif.src)}');font-display:block}` : "";
const px = (v) => `${r3(v * U)}px`;

const html = `<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="UTF-8">
<!-- gerado por video-factory/skills/animacao/scripts/edl2hf.mjs a partir de data/edl.json: não edite à mão -->
<script src="gsap.min.js"></script>
<style>
${fontFace}${fontFace2}
html,body{margin:0;background:#000}
#root{position:relative;width:${W}px;height:${H}px;overflow:hidden;background:${esc(edl.background ?? "#000")};isolation:isolate}
#stage{position:absolute;inset:0;${grade}}
.clip{position:absolute;inset:0;overflow:hidden}
.kb{position:absolute;inset:0;transform-origin:50% 50%;will-change:transform}
.kb img,.kb video{width:100%;height:100%;display:block}
.fill{position:absolute;inset:0}
.shade{background:linear-gradient(180deg,rgba(0,0,0,.55),rgba(0,0,0,.35) 40%,rgba(0,0,0,.65))}
.center{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;text-align:center}
.col{flex-direction:column}
.w{display:inline-block}
.t-title{font-family:${FONT};font-weight:900;font-size:${px(110)};line-height:1.05;text-transform:uppercase;max-width:82%;text-shadow:0 ${px(6)} ${px(30)} rgba(0,0,0,.6)}
.t-number{font-family:${FONT};font-weight:900;font-size:${px(190)};text-shadow:0 ${px(8)} ${px(40)} rgba(0,0,0,.6)}
.lower{position:absolute;left:${px(90)};bottom:${px(110)}}
.t-lower{font-family:${FONT};font-weight:800;font-size:${px(46)};background:rgba(8,8,12,.78);padding:${px(16)} ${px(28)};border-left:${px(8)} solid #f2c14e}
.label{position:absolute;left:${px(90)};top:${px(80)}}
.t-label{font-family:${FONT};font-weight:900;font-size:${px(40)};background:#f2c14e;color:#111;padding:${px(8)} ${px(20)};border-radius:${px(8)};transform-origin:0 50%}
.card{position:absolute;inset:0;color:#fff}
.k-num{font-family:${FONT};font-weight:700;font-size:${px(40)};letter-spacing:.35em;text-transform:uppercase;color:#f2c14e}
.k-line{width:${px(160)};height:${px(4)};background:#f2c14e;margin:${px(26)} 0 ${px(30)};transform-origin:50% 50%}
.k-title{font-family:${SERIF};font-weight:700;font-size:${px(124)};line-height:1.06;max-width:80%}
.mask{display:inline-block;overflow:hidden;padding-bottom:.08em}
.k-big{font-family:${FONT};font-weight:900;font-size:${px(230)};line-height:1;color:#fff;text-shadow:0 ${px(10)} ${px(50)} rgba(0,0,0,.5)}
.k-sub{font-family:${FONT};font-weight:600;font-size:${px(54)};max-width:70%;margin-top:${px(24)};color:#e9e4da}
.k-mark{font-family:${SERIF};font-size:${px(220)};line-height:.6;color:#f2c14e;height:${px(110)}}
.k-quote{font-family:${SERIF};font-style:italic;font-size:${px(70)};line-height:1.25;max-width:72%}
.k-autor{font-family:${FONT};font-weight:600;font-size:${px(34)};letter-spacing:.12em;text-transform:uppercase;margin-top:${px(36)};color:#f2c14e}
.split{display:flex;gap:${px(14)};padding:${px(14)};box-sizing:border-box}
.k-side{position:relative;flex:1;overflow:hidden}
.k-side img{width:100%;height:100%;object-fit:cover;transform-origin:50% 50%}
.k-cap{position:absolute;left:0;right:0;bottom:${px(60)};text-align:center;font-family:${FONT};font-weight:900;font-size:${px(60)};text-transform:uppercase;text-shadow:0 ${px(4)} ${px(24)} rgba(0,0,0,.8)}
.k-pinwrap{position:absolute;width:0;height:0}
.k-pin{position:absolute;left:${px(-14)};top:${px(-14)};width:${px(28)};height:${px(28)};border-radius:50%;background:#f2c14e;box-shadow:0 0 0 ${px(5)} rgba(0,0,0,.45)}
.k-ring{position:absolute;left:${px(-30)};top:${px(-30)};width:${px(60)};height:${px(60)};border-radius:50%;border:${px(4)} solid #f2c14e}
.k-plabel{position:absolute;left:${px(34)};top:${px(-30)};white-space:nowrap;font-family:${FONT};font-weight:800;font-size:${px(42)};color:#fff;background:rgba(8,8,12,.75);padding:${px(6)} ${px(16)}}
.ov{position:absolute;inset:0;pointer-events:none;z-index:100000}
.grain{background-image:url('fx/grain.png');background-repeat:repeat;mix-blend-mode:overlay}
.leak{mix-blend-mode:screen}
</style>
</head>
<body>
<div id="root" data-composition-id="main" data-start="0" data-duration="${r3(D)}" data-width="${W}" data-height="${H}" data-fps="${FPS}">
<div id="stage">
${els.join("\n")}
</div>
${ovEls.join("\n")}
</div>
<script>
// eases iguais aos do mygo/Remotion (src/lib/ease.ts): linear | in | out | inOut (bezier .45,0,.55,1)
function bez(x1,y1,x2,y2){const cx=3*x1,bx=3*(x2-x1)-cx,ax=1-cx-bx,cy=3*y1,by=3*(y2-y1)-cy,ay=1-cy-by;
  const X=t=>((ax*t+bx)*t+cx)*t, Y=t=>((ay*t+by)*t+cy)*t, dX=t=>(3*ax*t+2*bx)*t+cx;
  return x=>{let t=x;for(let i=0;i<8;i++){const d=dX(t);if(Math.abs(d)<1e-6)break;t-=(X(t)-x)/d;}t=Math.min(1,Math.max(0,t));return Y(t);};}
const EASES={linear:t=>t,in:t=>t*t*t,out:t=>1-Math.pow(1-t,3),inOut:bez(.45,0,.55,1),inOutSine:t=>-(Math.cos(Math.PI*t)-1)/2};
function E(n){return EASES[n]||EASES.linear;}
window.__timelines = window.__timelines || {};
const tl = gsap.timeline({ paused: true });
// contador: "1.889" mantém o separador de milhar do texto original
function count(sel, text, target, grouped, at, d){const o={v:0};const el=document.querySelector(sel);
  const fmt=n=>text.replace(/[\\d.,-]+/, grouped? Math.round(n).toLocaleString("pt-BR") : String(Math.round(n)));
  tl.fromTo(o,{v:0},{v:target,duration:d,ease:"power2.out",onUpdate:()=>{el.textContent=fmt(o.v);}},at);}
${tl.join("\n")}
tl.set({}, {}, ${r3(D)});
const ready = () => { window.__timelines["main"] = tl; };
(document.fonts && document.fonts.ready ? document.fonts.ready : Promise.resolve()).then(ready);
</script>
</body>
</html>
`;
fs.writeFileSync(path.join(out, "index.html"), html);
const n = (t) => clips.filter((c) => c.type === t).length;
console.log(`hf: ${clips.length} clipes (image ${n("image")}, card ${n("card")}, text ${n("text")}, video ${n("video")}), ${ov.length} overlays, ${r3(D)} s ${W}x${H}@${FPS} -> ${path.relative(proj, out)}/index.html`);
