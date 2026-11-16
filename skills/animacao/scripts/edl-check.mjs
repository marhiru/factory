#!/usr/bin/env node
// Trava da etapa de animação: mede quanto do vídeo tem animação de verdade no data/edl.json.
// "Animado" = clipe `scene` com pelo menos um ator que FAZ algo (entra/sai, se move por keys, troca de estado,
// ou tem estado em ciclo), ou um clipe `video` (clipe de IA/stock já é movimento). Ken Burns (`image` com zoom)
// e cartelas (`text`/`color`) contam como "estático com câmera".
// uso: node edl-check.mjs data/edl.json [--min 0.7] [--max-gap 6] [--sprites public]
// sai com código 1 se reprovar: a skill factory não pode marcar `animacao: feito` sem passar aqui.
import fs from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf(`--${k}`); return i >= 0 ? args[i + 1] : d; };
const edlPath = args.find((a) => !a.startsWith("--") && !args[args.indexOf(a) - 1]?.startsWith("--")) ?? "data/edl.json";
const MIN = Number(opt("min", 0.7));
const MAXGAP = Number(opt("max-gap", 6));
const root = opt("sprites", path.join(path.dirname(path.dirname(path.resolve(edlPath))), "public"));
const edl = JSON.parse(fs.readFileSync(edlPath, "utf8"));
const spriteCache = {};
const sprite = (dir) => {
  if (!(dir in spriteCache)) {
    try { spriteCache[dir] = JSON.parse(fs.readFileSync(path.join(root, dir, "sprite.json"), "utf8")); } catch { spriteCache[dir] = null; }
  }
  return spriteCache[dir];
};

const actorActs = (a) => {
  const why = [];
  if (a.enter) why.push("entra");
  if (a.exit) why.push("sai");
  const keys = a.keys ?? [];
  if (keys.some((k) => ["x", "y", "scale", "rot", "opacity"].some((p) => p in k))) why.push("move");
  if (keys.some((k) => "state" in k)) why.push("troca estado");
  const sp = sprite(a.sprite);
  const states = new Set([a.state, ...keys.map((k) => k.state)].filter(Boolean));
  if (sp && [...states].some((s) => (sp.states?.[s]?.frames?.length ?? 0) > 1)) why.push("ciclo");
  if (sp === null) why.push("SPRITE AUSENTE");
  return why;
};

const dur = edl.duration;
const rows = [];
const animated = [];
const problems = [];
for (const c of edl.clips ?? []) {
  let kind = "estático", detail = c.type;
  if (c.type === "scene") {
    const acts = (c.scene?.actors ?? []).map((a) => ({ id: a.id, why: actorActs(a) }));
    acts.filter((x) => x.why.includes("SPRITE AUSENTE")).forEach((x) => problems.push(`${c.id}: sprite de ${x.id} não encontrado em ${root}`));
    const live = acts.filter((x) => x.why.some((w) => w !== "SPRITE AUSENTE"));
    if (live.length) { kind = "animado"; animated.push([c.start, c.end]); }
    detail = `${acts.length} atores, ${live.length} agindo (${[...new Set(live.flatMap((x) => x.why))].join(", ") || "nenhuma ação"})`;
  } else if (c.type === "video") {
    kind = "animado"; animated.push([c.start, c.end]); detail = `vídeo ${c.src}`;
  } else if (c.type === "image") detail = `still ${c.src ?? "(camadas)"}${c.motion ? " + Ken Burns" : ""}`;
  rows.push({ id: c.id, a: c.start, b: c.end, kind, detail });
}
// cobertura (união dos intervalos animados) e maior trecho sem animação
animated.sort((x, y) => x[0] - y[0]);
const merged = [];
for (const [a, b] of animated) {
  const l = merged.at(-1);
  if (l && a <= l[1] + 1e-3) l[1] = Math.max(l[1], b); else merged.push([a, b]);
}
const covered = merged.reduce((s, [a, b]) => s + (Math.min(b, dur) - Math.max(a, 0)), 0);
let gap = 0, gapAt = 0, prev = 0;
for (const [a, b] of [...merged, [dur, dur]]) { if (a - prev > gap) { gap = a - prev; gapAt = prev; } prev = Math.max(prev, b); }
const ratio = covered / Math.max(dur, 1e-6);

for (const r of rows) console.log(`${r.kind === "animado" ? "ANIM " : "----"} ${String(r.id ?? "").padEnd(16)} ${r.a.toFixed(2).padStart(7)}-${r.b.toFixed(2).padEnd(7)} ${r.detail}`);
console.log(`\nanimação real: ${(ratio * 100).toFixed(0)}% do vídeo (mínimo ${(MIN * 100).toFixed(0)}%) · maior trecho sem animação: ${gap.toFixed(1)} s em ${gapAt.toFixed(1)} s (máximo ${MAXGAP} s)`);
const fail = ratio < MIN || gap > MAXGAP || problems.length > 0;
problems.forEach((p) => console.log("ERRO", p));
console.log(fail ? "REPROVADO: a etapa de animação NÃO está feita (Ken Burns/cartelas não são animação)." : "OK: animação aprovada.");
process.exit(fail ? 1 : 0);
