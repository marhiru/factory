#!/usr/bin/env node
// Primeiro corte automático: roteiro/beats.json (já com start/end do `vf align`) -> data/edl.json.
// uso: node beats2edl.mjs roteiro/beats.json data/edl.json [--fps 30] [--voice voice/voice.wav] [--music audio/music.wav]
//      [--captions on|off] [--tail 0.4] [--force]
import fs from "node:fs";

const args = process.argv.slice(2);
const pos = args.filter((a, i) => !a.startsWith("--") && !(args[i - 1] ?? "").startsWith("--"));
const opt = (k, d) => { const i = args.indexOf(`--${k}`); return i >= 0 ? args[i + 1] : d; };
const [beatsPath, edlPath] = pos;
if (!beatsPath || !edlPath) { console.error("uso: beats2edl.mjs beats.json edl.json [--fps 30] [--voice voice/voice.wav] [--music audio/music.wav] [--captions on|off] [--force]"); process.exit(2); }
if (fs.existsSync(edlPath) && !args.includes("--force")) {
  const cur = JSON.parse(fs.readFileSync(edlPath, "utf8"));
  if ((cur.clips ?? []).some((c) => !c.id?.startsWith("auto-") && c.id !== "abertura" && c.id !== "titulo" && c.id !== "foto")) {
    console.error(`${edlPath} já tem clipes editados à mão; use --force para sobrescrever`); process.exit(1);
  }
}
const fps = Number(opt("fps", 30));
const tail = Number(opt("tail", 0.4));
const B = JSON.parse(fs.readFileSync(beatsPath, "utf8"));
const beats = B.beats ?? [];
const missing = beats.filter((b) => typeof b.start !== "number");
if (missing.length) { console.error(`beats sem tempo (${missing.map((b) => b.id).join(", ")}): rode antes \`vf align roteiro/beats.json data/words.json\``); process.exit(1); }
const vertical = B.formato !== "yt-long";
const duration = +(beats.at(-1).end + tail).toFixed(3);

// Movimento linear (velocidade constante) com amplitude proporcional à duração: nenhum still fica
// abaixo de ~0,6 px/frame, senão o `vf qa` acusa congelamento.
const motionFor = (i, frames) => {
  const z = Math.min(0.25, Math.max(0.08, 0.0011 * frames)); // zoom total
  const a = Math.min(0.06, Math.max(0.025, 0.00028 * frames)); // meia amplitude de pan
  const ps = +(1 + 2 * a + 0.02).toFixed(3);
  return [
    { from: { scale: 1.0 }, to: { scale: +(1 + z).toFixed(3) }, ease: "linear" },
    { from: { scale: ps, x: a }, to: { scale: ps, x: -a }, ease: "linear" },
    { from: { scale: +(1 + z).toFixed(3) }, to: { scale: 1.0 }, ease: "linear" },
    { from: { scale: ps, x: -a, y: a / 3 }, to: { scale: ps, x: a, y: -a / 3 }, ease: "linear" },
  ][i % 4];
};
const kind = (src) => (/\.(mp4|mov|webm|m4v)$/i.test(src) ? "video" : "image");
const PALETTE = ["#1d2b53", "#7e2553", "#008751", "#ab5236", "#5f574f", "#29adff"];

const clips = [];
let m = 0;
beats.forEach((b, k) => {
  // o visual troca levemente ANTES da palavra (2 frames), como no padrão da fábrica
  const a = k === 0 ? 0 : Math.max(0, +(b.start - 2 / fps).toFixed(3));
  const z = k + 1 < beats.length ? +(beats[k + 1].start - 2 / fps).toFixed(3) : duration;
  const assets = (b.assets ?? []).filter(Boolean);
  const trans = b.tipo === "capitulo" ? { kind: "fade", dur: 0.3 } : k === 1 && B.formato !== "yt-long" ? { kind: "flash", dur: 0.2 } : undefined;
  if (assets.length === 0) {
    clips.push({ id: `auto-b${b.id}-bg`, type: "color", color: PALETTE[k % PALETTE.length], start: a, end: z, ...(trans ? { transition_in: trans } : {}) });
    clips.push({ id: `auto-b${b.id}-todo`, type: "text", style: "label", text: `FALTA ASSET: ${(b.visual ?? "").slice(0, 60)}`, start: a, end: z, layer: 3 });
  } else {
    const step = (z - a) / assets.length;
    assets.forEach((src, j) => {
      const s = +(a + j * step).toFixed(3), e = +(a + (j + 1) * step).toFixed(3);
      const base = { id: `auto-b${b.id}-${j + 1}`, type: kind(src), src, start: s, end: e, ...(j === 0 && trans ? { transition_in: trans } : {}) };
      clips.push(kind(src) === "image" ? { ...base, motion: motionFor(m++, Math.round((e - s) * fps)) } : base);
    });
  }
  if (b.texto_tela) {
    const style = /^[\d.,%\s-]+$/.test(b.texto_tela) ? "number" : b.tipo === "gancho" || b.tipo === "capitulo" ? "title" : "label";
    clips.push({ id: `auto-b${b.id}-txt`, type: "text", style, text: b.texto_tela, start: +(b.start - 1 / fps).toFixed(3), end: +Math.min(z, b.start + 2.5).toFixed(3), layer: 2 });
  }
});

const audio = [{ src: opt("voice", "voice/voice.wav"), role: "voice" }];
if (opt("music")) audio.push({ src: opt("music"), role: "music", gain_db: -2, loop: true, fade_out: 1.5 });
const capOpt = opt("captions", vertical ? "on" : "off");
const edl = { fps, duration, background: "#000000", audio, clips, captions: { enabled: capOpt === "on", words_per_page: 3, style: "karaoke" } };
fs.writeFileSync(edlPath, JSON.stringify(edl, null, 2));
const todo = clips.filter((c) => c.id.endsWith("-todo")).length;
console.log(`edl: ${clips.length} clipes, ${duration}s${todo ? `, ${todo} beat(s) sem asset (marcados FALTA ASSET)` : ""} -> ${edlPath}`);
