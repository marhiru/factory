#!/usr/bin/env bun
// Curadoria dos lotes do Flow -> imagens finais por plano.
// assets/flow/curadoria.json: { "b1-1": "assets/flow/porto-noite-1/porto-noite-1_07.png", ... , "_rejeitadas": {"<arquivo>": "motivo"} }
// aplica: copia cada escolha para public/img/planos/<plano>.jpg, com upscale (lanczos + nitidez leve) quando a
// largura < --min-w (o zoom-in come ~10% das bordas), e registra tudo em assets/manifest.yaml (origem: flow).
// uso: curadoria.mjs aplicar [--min-w 2150] [--w 2560]  |  curadoria.mjs faltando
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf(`--${k}`); return i >= 0 ? args[i + 1] : d; };
const C = JSON.parse(fs.readFileSync("assets/flow/curadoria.json", "utf8"));
const L = JSON.parse(fs.readFileSync("assets/flow/lotes.json", "utf8"));
const planos = L.lotes.flatMap((l) => l.planos.map((p) => ({ p, lote: l })));
const escolhas = Object.entries(C).filter(([k]) => !k.startsWith("_"));

if (args[0] === "faltando") {
  const falta = planos.filter(({ p }) => !C[p]).map(({ p }) => p);
  console.log(falta.length ? `faltam ${falta.length} planos: ${falta.join(", ")}` : "todos os planos têm imagem");
  process.exit(falta.length ? 1 : 0);
}
if (args[0] !== "aplicar") { console.error("uso: curadoria.mjs aplicar | faltando"); process.exit(2); }

const minW = Number(opt("min-w", 2150)), W = Number(opt("w", 2560));
fs.mkdirSync("public/img/planos", { recursive: true });
const usados = new Map();
let up = 0;
const man = [];
for (const [p, src] of escolhas) {
  if (!fs.existsSync(src)) { console.error(`ERRO ${p}: ${src} não existe`); process.exit(1); }
  if (usados.has(src)) console.error(`aviso: ${src} usado em ${usados.get(src)} e ${p} (o edl-check reprova repetição < 120 s)`);
  usados.set(src, p);
  const w = Number(execFileSync("ffprobe", ["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width", "-of", "csv=p=0", src]).toString().trim());
  const out = `public/img/planos/${p}.jpg`;
  const vf = w < minW ? `scale=${W}:-2:flags=lanczos,unsharp=5:5:0.6:5:5:0.0` : "null";
  if (w < minW) up++;
  execFileSync("ffmpeg", ["-v", "error", "-y", "-i", src, "-vf", vf, "-q:v", "2", out]);
  const lote = planos.find((x) => x.p === p)?.lote;
  man.push(`  - id: ${p}\n    tipo: imagem\n    origem: flow\n    lote: ${lote?.id ?? "?"}\n    arquivo: ${out}\n    fonte: ${src}\n    upscale: ${w < minW}\n    licenca: "gerada pelo usuário no Google Flow (Nano Banana)"\n    status: pronto`);
}
const mp = "assets/manifest.yaml";
let m = fs.existsSync(mp) ? fs.readFileSync(mp, "utf8") : "assets:\n";
m = m.replace(/\n# --- curadoria flow ---[\s\S]*$/, "");
fs.writeFileSync(mp, `${m.trimEnd()}\n# --- curadoria flow --- (gerado por curadoria.mjs; não edite abaixo)\nflow_planos:\n${man.join("\n")}\n`);
for (const l of L.lotes) if (l.planos.every((p) => C[p])) l.status = "curado";
fs.writeFileSync("assets/flow/lotes.json", JSON.stringify(L, null, 2));
const falta = planos.filter(({ p }) => !C[p]).length;
console.log(`${escolhas.length} planos -> public/img/planos (${up} com upscale)${falta ? ` · faltam ${falta}` : ""}`);
