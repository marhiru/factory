#!/usr/bin/env bun
// Cartões de plano: a unidade de trabalho da esteira. Um cartão por item de `visual` de cada beat, com a janela
// de tempo cortada em início de palavra, o texto falado nessa janela e os critérios de aceite do motor.
// Cada sub-agente recebe um LOTE de cartões contíguos e só escreve nos arquivos que o cartão lista em `saida`.
// uso: bun planos.mjs [roteiro/beats.json] [--lote 5]   (precisa do `vf align` antes)
//      bun planos.mjs status                           (passe/estado de cada cartão)
import fs from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf(`--${k}`); return i >= 0 ? args[i + 1] : d; };
const DIR = "roteiro/planos";
const PASSES = ["animatic", "layout", "animacao", "luz", "fx", "som", "final"]; // v2: "luz" entrou antes de "fx"

if (args[0] === "status") {
  const cs = fs.existsSync(DIR) ? fs.readdirSync(DIR).filter((f) => f.endsWith(".json")).sort().map((f) => JSON.parse(fs.readFileSync(path.join(DIR, f), "utf8"))) : [];
  const by = {};
  for (const c of cs) by[c.lote] = [...(by[c.lote] ?? []), c];
  for (const [l, v] of Object.entries(by)) console.log(`lote ${l}: ${v.map((c) => `${c.id}[${PASSES[c.passe] ?? c.passe}${c.status === "ok" ? "" : `:${c.status}`}]`).join(" ")}`);
  const min = Math.min(...cs.map((c) => c.passe));
  console.log(`${cs.length} cartões · passe mínimo: ${PASSES[min] ?? min} · pendentes no passe: ${cs.filter((c) => c.status !== "ok").length}`);
  process.exit(0);
}

const bp = args.find((a, i) => !a.startsWith("--") && !(args[i - 1] ?? "").startsWith("--")) ?? "roteiro/beats.json";
const B = JSON.parse(fs.readFileSync(bp, "utf8"));
const W = JSON.parse(fs.readFileSync("data/words.json", "utf8"));
const motor = B.motor ?? "cutout-2d";
const lote = Number(opt("lote", 5));
const LEAD = 2 / 30;
const w = (i) => +(W[i].start - LEAD).toFixed(3);
if (!B.beats?.every((b) => "w0" in b)) { console.error("rode antes: vf align roteiro/beats.json data/words.json"); process.exit(1); }

const CRIT = {
  "cutout-2d": ["toda ação cai numa palavra (at_word ou w(i))", "câmera nunca parada (push/pan)", "≥ 1 ator agindo (entra, anda, troca de estado ou ciclo)", "pés no chão do plate e escala coerente", "still conferido na safe zone"],
  estatico: ["imagem curada sem texto/defeito", "zoom/pan perceptível (0,6-3,5%/s)", "corte em início de palavra"],
  "blender-3d": ["câmera em glide contínuo (vida ≥ 0,08)", "personagem estala na pose na palavra e congela respirando", "1 ideia por plano", "luma média 0,35-0,5"],
};
fs.mkdirSync(DIR, { recursive: true });
const old = Object.fromEntries((fs.existsSync(DIR) ? fs.readdirSync(DIR) : []).filter((f) => f.endsWith(".json")).map((f) => { const c = JSON.parse(fs.readFileSync(path.join(DIR, f), "utf8")); return [c.id, c]; }));
const cards = [];
B.beats.forEach((b, i) => {
  const a0 = i === 0 ? 0 : w(b.w0);
  const end = i + 1 < B.beats.length ? w(B.beats[i + 1].w0) : +(W.at(-1).end + 0.4).toFixed(3);
  const vis = String(b.visual ?? "").split(";").map((s) => s.trim()).filter(Boolean);
  const n = Math.max(1, vis.length);
  const ws = []; for (let k = b.w0; k <= b.w1; k++) ws.push(k);
  const cut = (j) => (j === 0 ? a0 : j === n ? end : Math.max(a0, w(ws[Math.min(ws.length - 1, Math.round((j * ws.length) / n))])));
  for (let j = 0; j < n; j++) {
    const id = `s${String(cards.length + 1).padStart(2, "0")}`;
    const [s, e] = [cut(j), cut(j + 1)];
    const wr = ws.filter((k) => W[k].start >= s - 0.01 && W[k].start < e - 0.01);
    const o = old[id];
    cards.push({
      id, beat: b.id, tipo: b.tipo, lote: Math.floor(cards.length / lote) + 1, start: s, end: e, dur: +(e - s).toFixed(3),
      palavras: wr.length ? [wr[0], wr.at(-1)] : [], texto: wr.map((k) => W[k].word).join(" "),
      visual: vis[j] ?? b.visual ?? "", camera: b.camera ?? "", cenario: b.cenario ?? null, sfx: b.sfx ?? [], card: j === 0 ? b.card ?? null : null,
      saida: { cena: `roteiro/cenas/${id}.py`, still: `out/planos/${id}.png` },
      criterios: CRIT[motor] ?? CRIT["cutout-2d"],
      luz: o?.luz ?? b.luz ?? [],          // cues do iluminador: {at_word, spot|rays|exposure|temp, ...}
      fx: o?.fx ?? b.fx ?? [],             // FX do artista de FX: {at_word, tipo, fonte: shader|pixelfx, ...}
      continuo_com: o?.continuo_com ?? null, // diretor de câmera: id do cartão vizinho na mesma scene (zoom-out -> zoom-in)
      // cartões antigos (v1, sem o passe "luz"): fx/som/final sobem um índice
      passe: o ? (o.passes_v === 2 ? o.passe : o.passe >= 3 ? o.passe + 1 : o.passe) : 0, passes_v: 2,
      status: o?.status ?? "pendente", notas: o?.notas ?? [],
    });
  }
});
// continuidade de câmera (QA etapa 5): cartões vizinhos no mesmo cenário viram sugestão de junção
// (sem corte: zoom-out do enquadramento atual -> zoom-in no novo assunto). O diretor de câmera confirma
// em `continuo_com` ou recusa (motivo em notas) quando muda tempo/assunto.
let juncoes = 0;
cards.forEach((c, i) => {
  const prev = cards[i - 1];
  c.continuo_sugerido = prev && c.cenario && c.cenario === prev.cenario && c.continuo_com == null ? prev.id : null;
  if (c.continuo_sugerido) juncoes++;
});
for (const c of cards) fs.writeFileSync(path.join(DIR, `${c.id}.json`), JSON.stringify(c, null, 1));
for (const f of fs.readdirSync(DIR)) if (f.endsWith(".json") && !cards.some((c) => `${c.id}.json` === f)) fs.unlinkSync(path.join(DIR, f));
const lotes = Math.max(...cards.map((c) => c.lote));
console.log(`${cards.length} cartões em ${lotes} lote(s) de até ${lote} -> ${DIR}/ (motor ${motor}) · ${juncoes} junção(ões) de câmera sugerida(s) no mesmo cenário`);
