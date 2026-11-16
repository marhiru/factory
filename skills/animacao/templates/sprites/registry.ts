// Registro de sprites do projeto: troque pelos desenhos do seu elenco.
// Para elencos grandes use os construtores de packs.ts (front/side/fx + ciclos de marcha, passeio, aceno e dança).
import type { Pack, SpriteItem } from "./types";

/** boneco de exemplo (600 x 1000 unidades, pés em y = 975): cabeça, corpo e olhos que fecham na piscada */
const boneco = (g: CanvasRenderingContext2D, o: { boca: "sorri" | "o"; olhos: "abertos" | "fechados"; braco?: number }) => {
  g.lineWidth = 12; g.strokeStyle = "#241726"; g.lineJoin = "round";
  g.fillStyle = "#3f6fb5"; g.beginPath(); g.ellipse(300, 640, 120, 190, 0, 0, Math.PI * 2); g.fill(); g.stroke();
  g.beginPath(); g.moveTo(260, 820); g.lineTo(250, 970); g.moveTo(340, 820); g.lineTo(350, 970); g.stroke();
  const a = o.braco ?? 0;
  g.beginPath(); g.moveTo(190, 600); g.lineTo(120, 760); g.moveTo(410, 600); g.lineTo(480 + 40 * a, 760 - 260 * Math.abs(a)); g.stroke();
  g.fillStyle = "#ffe2c2"; g.beginPath(); g.ellipse(300, 330, 130, 140, 0, 0, Math.PI * 2); g.fill(); g.stroke();
  g.fillStyle = "#241726";
  for (const x of [255, 345]) {
    if (o.olhos === "abertos") { g.beginPath(); g.ellipse(x, 320, 12, 16, 0, 0, Math.PI * 2); g.fill(); }
    else { g.beginPath(); g.moveTo(x - 16, 322); g.lineTo(x + 16, 322); g.stroke(); }
  }
  g.beginPath();
  if (o.boca === "sorri") g.arc(300, 380, 40, 0.15 * Math.PI, 0.85 * Math.PI); else g.ellipse(300, 395, 16, 22, 0, 0, Math.PI * 2);
  g.stroke();
};
const item = (file: string, state: string, draw: (g: CanvasRenderingContext2D) => void, role?: "blink"): SpriteItem => ({ file, state, draw, role });

export const PACKS: Pack[] = [
  {
    name: "exemplo", w: 600, h: 1000, scale: 1.5, anchor: [0.5, 0.975], fps: { aceno: 10 },
    items: [
      item("neutro", "neutro", (g) => boneco(g, { boca: "sorri", olhos: "abertos" })),
      item("neutro_blink", "neutro", (g) => boneco(g, { boca: "sorri", olhos: "fechados" }), "blink"),
      item("susto", "susto", (g) => boneco(g, { boca: "o", olhos: "abertos" })),
      item("susto_blink", "susto", (g) => boneco(g, { boca: "o", olhos: "fechados" }), "blink"),
      ...Array.from({ length: 8 }, (_, k) => item(`aceno_${k}`, "aceno", (g) => boneco(g, { boca: "sorri", olhos: "abertos", braco: Math.sin((2 * Math.PI * k) / 8) }))),
    ],
  },
];
