#!/usr/bin/env bun
// Trava da etapa de animação: mede quanto do vídeo tem animação de verdade no data/edl.json.
// "Animado" = clipe `scene` com pelo menos um ator que FAZ algo (entra/sai, se move por keys, troca de estado,
// ou tem estado em ciclo), ou um clipe `video` (clipe de IA/stock já é movimento). Ken Burns (`image` com zoom)
// e cartelas (`text`/`color`) contam como "estático com câmera".
// uso: bun edl-check.mjs data/edl.json [--min 0.7] [--max-gap 6] [--sprites public]
//      bun edl-check.mjs data/edl.json --perfil estatico [--beats roteiro/beats.json]   (vídeo longo estático com tema)
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
if ((opt("perfil", edl.perfil ?? "animado")) === "estatico") {
  estatico();
  process.exit(0);
}
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
// ---------- continuidade de câmera (QA visual etapa 5; medido em 3 referências de 19-35M de views) ----------
// mesmo plate em clipes vizinhos = mesmo contexto: sem corte seco (zoom-out -> zoom-in numa scene só, ou dissolve
// de 0,2-0,3 s com a câmera seguindo). Corte seco no total: no máximo 25 % das transições. Exceção: `corte_motivo`.
const camWarns = [];
const base = (edl.clips ?? []).filter((c) => (c.layer ?? 0) === 0 && ["scene", "image", "video"].includes(c.type)).sort((a, b) => a.start - b.start);
const plateOf = (c) => c.scene?.bg?.src ?? (c.type === "image" ? c.src : null);
let trans = 0, secas = 0;
for (let i = 1; i < base.length; i++) {
  const a = base[i - 1], b = base[i];
  if (Math.abs(b.start - a.end) > 0.1 && b.start > a.end) continue; // não são vizinhos
  trans++;
  const kind = b.transition_in?.kind ?? "cut", dt = b.transition_in?.dur ?? 0;
  if (kind === "cut") secas++;
  const mesmo = plateOf(a) && plateOf(a) === plateOf(b);
  if (mesmo && kind === "cut" && !b.corte_motivo) problems.push(`${b.id}: corte seco no mesmo cenário de ${a.id} (${plateOf(b)}): junte numa scene com zoom-out -> zoom-in, ou dissolve 0,2-0,3 s com a câmera seguindo (ou registre "corte_motivo")`);
  if (kind === "fade" && (dt < 0.15 || dt > 0.45)) camWarns.push(`${b.id}: dissolve de ${dt}s (referência: 0,2-0,3 s)`);
}
for (const c of base.filter((c) => c.type === "scene")) {
  const cam = c.scene?.camera;
  const moves = cam && (JSON.stringify(cam.from ?? {}) !== JSON.stringify(cam.to ?? {}) || (cam.keys ?? []).length > 1);
  if (!moves && c.end - c.start > 1) camWarns.push(`${c.id}: câmera parada em ${(c.end - c.start).toFixed(1)} s (referências: 0,05 % de frames parados)`);
}
if (trans) console.log(`câmera: ${trans} transições, ${secas} secas (${((100 * secas) / trans).toFixed(0)} %; máximo 25 %)`);
if (trans >= 4 && secas / trans > 0.25) camWarns.push(`cortes secos em ${((100 * secas) / trans).toFixed(0)} % das transições (referências: 0-25 %): troque por dissolves com câmera andando`);
camWarns.forEach((w) => console.log("AVISO", w));

const fail = ratio < MIN || gap > MAXGAP || problems.length > 0;
problems.forEach((p) => console.log("ERRO", p));
console.log(!fail ? "OK: animação aprovada." : ratio < MIN || gap > MAXGAP ? "REPROVADO: a etapa de animação NÃO está feita (Ken Burns/cartelas não são animação)." : "REPROVADO: corrija os ERROs acima (sprites/continuidade de câmera).");
process.exit(fail ? 1 : 0);

// ---------- perfil estático (yt-long com tema): o movimento é o zoom/pan leve, o capricho é medido aqui ----------
function estatico() {
  const P = {
    minShot: Number(opt("min-shot", 1.5)), maxShot: Number(opt("max-shot", 9)), hookMean: Number(opt("hook-mean", 3.2)),
    interrupt: Number(opt("interrupt", 40)), repeat: Number(opt("repeat", 120)), minRate: 0.006, maxRate: 0.035,
  };
  const errs = [], warns = [];
  const base = (edl.clips ?? []).filter((c) => (c.layer ?? 0) === 0 && ["image", "card", "video", "color"].includes(c.type)).sort((a, b) => a.start - b.start);
  if (!base.length) errs.push("nenhum clipe visual na camada 0");
  // 1) cobertura sem buracos
  let t = 0;
  for (const c of base) {
    if (c.start - t > 1 / (edl.fps ?? 30) + 1e-3) errs.push(`buraco ${t.toFixed(2)}-${c.start.toFixed(2)} s (vira tela preta)`);
    t = Math.max(t, c.end);
  }
  if (dur - t > 0.05) errs.push(`buraco no fim ${t.toFixed(2)}-${dur.toFixed(2)} s`);
  // 2) todo still se mexe, num ritmo perceptível mas sem enjoar
  for (const c of base.filter((c) => c.type === "image")) {
    const m = c.motion, d = c.end - c.start;
    if (!m) { errs.push(`${c.id}: still sem movimento`); continue; }
    const ds = Math.abs((m.to?.scale ?? m.from?.scale ?? 1) - (m.from?.scale ?? 1));
    const dx = Math.hypot((m.to?.x ?? m.from?.x ?? 0) - (m.from?.x ?? 0), (m.to?.y ?? m.from?.y ?? 0) - (m.from?.y ?? 0));
    const rate = (ds + dx) / Math.max(d, 0.1);
    if (rate < P.minRate) errs.push(`${c.id}: movimento imperceptível (${(rate * 100).toFixed(2)}%/s, mínimo ${P.minRate * 100}%/s): zoom 1,00→1,07 em 6 s = 1,2%/s`);
    if (rate > P.maxRate && d > 1) warns.push(`${c.id}: movimento rápido demais para o estilo (${(rate * 100).toFixed(1)}%/s)`);
  }
  // 3) ritmo: planos entre min e max; gancho mais rápido
  for (const c of base) {
    const d = c.end - c.start;
    if (d > P.maxShot + 0.3 + (c.transition_in?.dur ?? 0)) errs.push(`${c.id}: plano de ${d.toFixed(1)} s (máximo ${P.maxShot} s)`);
    if (d < P.minShot && c.type === "image") warns.push(`${c.id}: plano de ${d.toFixed(1)} s (curto: o zoom nem aparece)`);
  }
  const hook = base.filter((c) => c.start < 30);
  const hookMean = hook.reduce((s, c) => s + Math.min(c.end, 30) - c.start, 0) / Math.max(1, hook.length);
  if (hookMean > P.hookMean) errs.push(`gancho (0-30 s): plano médio de ${hookMean.toFixed(1)} s (máximo ${P.hookMean} s: o gancho corta mais rápido)`);
  // 4) interrupção de padrão: cartela, título, vídeo ou punch a cada <= 40 s
  const ints = (edl.clips ?? []).filter((c) => c.type === "card" || c.type === "video" || (c.type === "text" && c.style !== "lower") || typeof c.punch_at === "number" || c.transition_in?.kind === "punch")
    .map((c) => c.start + (typeof c.punch_at === "number" ? c.punch_at : 0)).sort((a, b) => a - b);
  let prev = 0, worst = 0, worstAt = 0;
  for (const x of [...ints, dur]) { if (x - prev > worst) { worst = x - prev; worstAt = prev; } prev = x; }
  if (worst > P.interrupt) errs.push(`${worst.toFixed(0)} s sem interrupção de padrão a partir de ${worstAt.toFixed(0)} s (máximo ${P.interrupt} s: card dado/citação/mapa/comparação, título ou punch)`);
  // 5) imagem repetida cedo demais
  const seen = {};
  for (const c of base.filter((c) => c.type === "image")) {
    const k = c.src ?? JSON.stringify(c.layers);
    if (k in seen && c.start - seen[k] < P.repeat) errs.push(`${c.id}: ${k} repetida ${(c.start - seen[k]).toFixed(0)} s depois (mínimo ${P.repeat} s)`);
    seen[k] = c.start;
  }
  // 6) acabamento: grão + vinheta (+ graduação recomendada)
  const ov = new Set((edl.overlays ?? []).map((o) => o.kind));
  for (const k of ["grain", "vignette"]) if (!ov.has(k)) errs.push(`acabamento: overlay '${k}' ausente`);
  if (!ov.has("grade")) warns.push("acabamento: sem 'grade' (graduação única do tema)");
  // 7) capítulos do roteiro têm cartela
  const bp = opt("beats", path.join(path.dirname(path.dirname(path.resolve(edlPath))), "roteiro/beats.json"));
  let caps = [];
  try { caps = (JSON.parse(fs.readFileSync(bp, "utf8")).beats ?? []).filter((b) => b.tipo === "capitulo" && typeof b.start === "number"); } catch {}
  const cards = (edl.clips ?? []).filter((c) => c.type === "card" && c.kind === "capitulo");
  for (const b of caps) if (!cards.some((c) => Math.abs(c.start - b.start) < 3)) errs.push(`capítulo "${b.titulo ?? b.id}" (${b.start.toFixed(1)} s) sem card capitulo`);
  const imgs = base.filter((c) => c.type === "image").length;
  console.log(`perfil estático: ${base.length} planos (${imgs} stills, ${cards.length} capítulos, ${ints.length} interrupções), gancho ${hookMean.toFixed(1)} s/plano, maior trecho sem interrupção ${worst.toFixed(0)} s, ${dur.toFixed(0)} s`);
  warns.forEach((w) => console.log("AVISO", w));
  errs.forEach((e) => console.log("ERRO", e));
  if (errs.length) { console.log("REPROVADO: a edição estática não está no padrão da fábrica."); process.exit(1); }
  console.log("OK: perfil estático aprovado.");
}
