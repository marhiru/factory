#!/usr/bin/env bun
// Trava do roteiro do vídeo longo (esquemática de hook rate da fábrica). Roda ANTES da aprovação humana
// (tempos estimados pelo wpm) e de novo depois do `vf align` (tempos reais da voz).
// uso: bun roteiro-check.mjs [roteiro/beats.json] [--min 600 --max 900] [--wpm 160] [--piloto]
//   --piloto: trecho de 2-4 min (gancho + capítulo 1 + loop final): só as regras que cabem no trecho
// sai com código 1 se reprovar.
import fs from "node:fs";

const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf(`--${k}`); return i >= 0 ? args[i + 1] : d; };
const p = args.find((a, i) => !a.startsWith("--") && !(args[i - 1] ?? "").startsWith("--")) ?? "roteiro/beats.json";
const B = JSON.parse(fs.readFileSync(p, "utf8"));
const piloto = args.includes("--piloto");
const WPM = Number(opt("wpm", B.wpm ?? 160));
const MIN = Number(opt("min", piloto ? 100 : 600)), MAX = Number(opt("max", piloto ? 240 : 900));
const beats = B.beats ?? [];
const errs = [], warns = [];
const words = (t) => String(t ?? "").trim().split(/\s+/).filter(Boolean);

// tempos: reais (vf align) ou estimados pelo wpm
const real = beats.every((b) => typeof b.start === "number");
let t = 0;
for (const b of beats) {
  if (real) { b._a = b.start; b._b = b.end; } else { b._a = t; t += (words(b.texto).length / WPM) * 60 + 0.25; b._b = t; }
}
const D = beats.at(-1)?._b ?? 0;
const at = (tipo) => beats.filter((b) => [].concat(tipo).includes(b.tipo));
const fmt = (s) => `${Math.floor(s / 60)}:${String(Math.round(s % 60)).padStart(2, "0")}`;

// 1) duração
if (D < MIN || D > MAX) errs.push(`duração ${fmt(D)} fora de ${fmt(MIN)}-${fmt(MAX)} (${real ? "real" : `estimada a ${WPM} wpm`})`);
// 2) primeira frase: sem preparação, curta, afirmação forte
const f0 = beats[0];
if (!f0) errs.push("sem beats");
else {
  const first = String(f0.texto).split(/(?<=[.!?])\s+/)[0];
  if (/^(ol[áa]|oi\b|e a[ií]\b|fala\b|bem[- ]vind|neste v[ií]deo|hoje (eu )?(vou|vamos)|voc[eê] sabia que|antes de come[çc]ar|se inscrev)/i.test(first.trim()))
    errs.push(`primeira frase começa com preparação/saudação: "${first}"`);
  if (words(first).length > 14) errs.push(`primeira frase com ${words(first).length} palavras (máx 14): "${first}"`);
  if (f0.tipo !== "gancho") errs.push(`o 1º beat precisa ser tipo "gancho" (é "${f0.tipo}")`);
}
// 3) frases curtas no gancho (0-30 s)
const hook = beats.filter((b) => b._a < 30);
const sent = hook.flatMap((b) => String(b.texto).split(/(?<=[.!?])\s+/)).filter(Boolean);
const avg = sent.reduce((s, x) => s + words(x).length, 0) / Math.max(1, sent.length);
if (avg > 12) errs.push(`gancho (0-30 s) com frases de ${avg.toFixed(1)} palavras em média (máx 12)`);
// 4) promessa + loop aberto cedo; re-hook antes de 75 s
const prom = at("promessa")[0];
if (!prom || prom._a > 40) errs.push(`falta beat "promessa" até 0:40 (${prom ? `está em ${fmt(prom._a)}` : "nenhum"})`);
const loop1 = at("loop")[0];
if (!loop1 || loop1._a > 40) errs.push(`falta o primeiro loop aberto (tipo "loop") até 0:40 (${loop1 ? `está em ${fmt(loop1._a)}` : "nenhum"})`);
if (!beats.some((b) => ["reganho", "virada", "revelacao"].includes(b.tipo) && b._a >= 35 && b._a <= 75)) errs.push(`falta re-hook (reganho/virada/revelação) entre 0:35 e 1:15`);
// 5) tensão contínua: pergunta/virada/loop a cada <= 90 s; curiosidade nova a cada <= 60 s (aviso)
const tens = beats.filter((b) => ["gancho", "loop", "reganho", "virada", "revelacao", "pergunta", "capitulo"].includes(b.tipo)).map((b) => b._a);
let prev = 0, worst = 0, worstAt = 0;
for (const x of [...tens, D]) { if (x - prev > worst) { worst = x - prev; worstAt = prev; } prev = x; }
if (worst > 90) errs.push(`${Math.round(worst)} s sem pergunta/virada/loop a partir de ${fmt(worstAt)} (máx 90 s)`);
else if (worst > 60) warns.push(`${Math.round(worst)} s sem pergunta nova a partir de ${fmt(worstAt)} (ideal ≤ 60 s)`);
// 6) capítulos: cada um fecha com loop aberto para o próximo
const caps = at("capitulo");
caps.forEach((c, k) => {
  if (!c.titulo) errs.push(`capítulo do beat ${c.id} sem "titulo"`);
  const next = caps[k + 1]?._a ?? D;
  if (!beats.some((b) => b.tipo === "loop" && b._a >= next - 60 && b._a < next + 1)) {
    errs.push(`capítulo "${c.titulo ?? c.id}" não termina com loop aberto (tipo "loop" no último minuto antes de ${fmt(next)})`);
  }
});
if (!piloto) {
  if (caps.length < 3) errs.push(`só ${caps.length} capítulo(s) (mín 3 para vídeo de 10-15 min)`);
  if (!beats.some((b) => ["reganho", "revelacao", "virada"].includes(b.tipo) && b._a >= D * 0.4 && b._a <= D * 0.6)) errs.push("falta o re-hook do meio (reganho/revelação/virada entre 40% e 60%)");
  const pay = at("payoff").at(-1);
  if (!pay || pay._a < D * 0.65 || pay._a > D * 0.92) errs.push(`payoff principal precisa cair entre 65% e 92% (${pay ? fmt(pay._a) : "nenhum"})`);
  const fecho = beats.at(-1);
  if (fecho?.tipo !== "fecho") errs.push('o último beat precisa ser tipo "fecho" (callback ao gancho + teaser do próximo)');
  else if (fecho._b - fecho._a > 20) errs.push(`fecho de ${Math.round(fecho._b - fecho._a)} s (máx 20 s)`);
}
// 7) sem CTA pedinte e sem visual vazio
for (const b of beats) {
  if (/(se inscrev|inscreva|deixa (o|seu) like|deixe (o|seu) like|ative o sininho|ativa o sininho)/i.test(b.texto)) errs.push(`beat ${b.id}: CTA pedinte ("se inscreva/like/sininho") derruba a retenção`);
  if (!String(b.visual ?? "").trim()) errs.push(`beat ${b.id}: sem "visual"`);
  if (B.motor === "estatico" && !b.cenario) errs.push(`beat ${b.id}: sem "cenario" (agrupa as imagens nos lotes do Flow)`);
}
// 8) densidade visual: 1 imagem a cada 4-7 s
const nVis = beats.reduce((s, b) => s + String(b.visual ?? "").split(";").filter((x) => x.trim()).length, 0);
const per = D / Math.max(1, nVis);
if (per > 7.5) errs.push(`${nVis} visuais para ${fmt(D)}: 1 a cada ${per.toFixed(1)} s (máx 7 s; gancho ~2-3 s)`);
const hookVis = hook.reduce((s, b) => s + String(b.visual ?? "").split(";").filter((x) => x.trim()).length, 0);
if (hook.length && 30 / Math.max(1, hookVis) > 3.5) errs.push(`gancho com ${hookVis} visuais em 30 s (mín 9: plano de ~3 s)`);

console.log(`roteiro: ${beats.length} beats, ${fmt(D)} (${real ? "voz real" : `estimado a ${WPM} wpm`}), ${caps.length} capítulos, ${nVis} visuais (1/${per.toFixed(1)} s), gancho ${avg.toFixed(1)} palavras/frase, maior trecho sem tensão ${Math.round(worst)} s`);
warns.forEach((w) => console.log("AVISO", w));
errs.forEach((e) => console.log("ERRO", e));
console.log(errs.length ? "REPROVADO: reescreva antes de mostrar ao usuário." : "OK: esquemática de retenção aprovada.");
process.exit(errs.length ? 1 : 0);
