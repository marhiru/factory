#!/usr/bin/env bun
// Converte uma sprite sheet de FX (JSON estilo Aseprite + PNG em grade) num pack de sprites da fábrica
// (public/sprites/<pack>/*.png + sprite.json), pronto para um ator `scene` no mygo/Remotion.
//
// uso:
//   bun fx-pack.mjs <efeito-pixelfx | sheet.json> <public/sprites/fx-nome> [--scale 6] [--state fx] [--opts '{"palette":[...]}']
//     efeito-pixelfx: id do catálogo (ex.: explosions/cartoon-pow); renderiza com ref/fx/pixelfx (ou ~/.cache/vf/ref/pixelfx)
//     sheet.json:     qualquer sheet Aseprite (pixelfx, Aseprite)
//     <pasta>:        sequência de PNGs com alpha (ex.: render da composição FxSheet do remotion-three); --fps 30 --loop
//   --scale N    ampliação inteira com vizinho mais próximo (pixel art continua nítida). Padrão: 6
//   --smooth     amplia com lanczos (FX não pixelado, ex.: sheet de shader renderizada pequena)
//   --anchor x,y âncora do ator em frações (padrão: pivot da sheet, ou 0.5,0.5)
// Precisa de ffmpeg. Licença: registre o efeito em assets/manifest.yaml (pixelfx = MIT).
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const argv = process.argv.slice(2);
const pos = [];
const opt = {};
for (let i = 0; i < argv.length; i++) {
  const a = argv[i];
  if (a.startsWith("--")) {
    const k = a.slice(2);
    if (k === "smooth" || k === "loop") opt[k] = true;
    else opt[k] = argv[++i];
  } else pos.push(a);
}
if (pos.length < 2) {
  console.error(fs.readFileSync(new URL(import.meta.url)).toString().split("\n").slice(1, 11).join("\n"));
  process.exit(2);
}
const [src, outDir] = pos;
const scale = Number(opt.scale ?? 6);
const state = opt.state ?? "fx";

function pixelfxRoot() {
  const cands = [path.resolve("ref/fx/pixelfx"), path.join(os.homedir(), ".cache/vf/ref/pixelfx")];
  const r = cands.find((c) => fs.existsSync(path.join(c, "tools/render.mjs")));
  if (!r) throw new Error("pixelfx não encontrado (rode vf init, ou git clone --depth 1 https://github.com/patch-gg/pixelfx ~/.cache/vf/ref/pixelfx)");
  return r;
}

// pasta de PNGs (sequência renderizada): monta uma sheet virtual, um arquivo por frame
if (fs.existsSync(src) && fs.statSync(src).isDirectory()) {
  const files = fs.readdirSync(src).filter((f) => f.endsWith(".png")).sort();
  if (!files.length) throw new Error(`${src} não tem PNGs`);
  const dims = execFileSync("ffprobe", ["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0", path.join(src, files[0])]).toString().trim().split(",").map(Number);
  const [w, h] = dims;
  fs.mkdirSync(outDir, { recursive: true });
  const fps = Number(opt.fps ?? 30);
  const sc = Number(opt.scale ?? 1);
  const names = files.map((f, i) => {
    const name = `${state}_${String(i).padStart(2, "0")}`;
    if (sc === 1) fs.copyFileSync(path.join(src, f), path.join(outDir, `${name}.png`));
    else execFileSync("ffmpeg", ["-v", "error", "-y", "-i", path.join(src, f), "-vf", `scale=${Math.round(w * sc)}:${Math.round(h * sc)}:flags=${opt.smooth ? "lanczos" : "neighbor"}`, path.join(outDir, `${name}.png`)]);
    return name;
  });
  const anchor = opt.anchor ? opt.anchor.split(",").map(Number) : [0.5, 0.5];
  fs.writeFileSync(path.join(outDir, "sprite.json"), JSON.stringify({ w: Math.round(w * sc), h: Math.round(h * sc), anchor, states: { [state]: { frames: names, fps } } }, null, 2));
  fs.writeFileSync(path.join(outDir, "ORIGEM.json"), JSON.stringify({ fonte: path.resolve(src), licenca: "próprio (shader da fábrica)", loop: Boolean(opt.loop), frames: names.length, fps }, null, 2));
  console.log(`${outDir}: ${names.length} frames ${Math.round(w * sc)}x${Math.round(h * sc)} @ ${fps} fps (sequência)`);
  process.exit(0);
}

let sheetJson = src;
if (!src.endsWith(".json")) {
  const root = pixelfxRoot();
  const args = ["tools/render.mjs", src, "--sheet"];
  if (opt.opts) args.push("--opts", opt.opts);
  execFileSync(process.execPath, args, { cwd: root, stdio: ["ignore", "ignore", "inherit"] });
  const base = path.basename(src);
  sheetJson = path.join(root, "records", src, `${base}.json`);
}
const sheet = JSON.parse(fs.readFileSync(sheetJson, "utf8"));
const png = path.join(path.dirname(sheetJson), sheet.meta.image);
const frames = Array.isArray(sheet.frames) ? sheet.frames : Object.values(sheet.frames);
const meta = sheet.meta.pixelfx ?? {};
const durMs = frames[0]?.duration ?? 33;
const fps = meta.fps ?? Math.round(1000 / durMs);
const [fw, fh] = meta.frameSize ?? [frames[0].frame.w, frames[0].frame.h];
const anchor = opt.anchor ? opt.anchor.split(",").map(Number) : meta.pivot ? [meta.pivot[0] / fw, meta.pivot[1] / fh] : [0.5, 0.5];

fs.mkdirSync(outDir, { recursive: true });
const flags = opt.smooth ? "lanczos" : "neighbor";
const names = [];
frames.forEach((f, i) => {
  const name = `${state}_${String(i).padStart(2, "0")}`;
  const { x, y, w, h } = f.frame;
  execFileSync("ffmpeg", ["-v", "error", "-y", "-i", png, "-vf", `crop=${w}:${h}:${x}:${y},scale=${w * scale}:${h * scale}:flags=${flags}`, "-frames:v", "1", path.join(outDir, `${name}.png`)]);
  names.push(name);
});
const spr = { w: fw * scale, h: fh * scale, anchor, states: { [state]: { frames: names, fps } } };
fs.writeFileSync(path.join(outDir, "sprite.json"), JSON.stringify(spr, null, 2));
fs.writeFileSync(
  path.join(outDir, "ORIGEM.json"),
  JSON.stringify({ fonte: meta.id ? `pixelfx:${meta.id}` : sheetJson, licenca: meta.id ? "MIT (patch-gg/pixelfx)" : "ver origem", loop: meta.loop ?? false, frames: names.length, fps, opts: opt.opts ?? null }, null, 2),
);
console.log(`${outDir}: ${names.length} frames ${fw * scale}x${fh * scale} @ ${fps} fps, loop=${meta.loop ?? false}, anchor=${anchor.map((a) => a.toFixed(2)).join(",")}`);
