// Construtores genéricos de packs para elencos desenhados em código (Canvas2D).
// Você fornece UMA função de desenho por tipo de personagem; os construtores geram os itens:
// expressões com frame de piscada, ciclos de N frames (marcha, passeio, aceno, dança) e efeitos animados.
//
//   type Opts = { expr: string; blink?: 0 | 1; phase?: number; mode?: string; wave?: number; tilt?: number };
//   const meuBoneco: Draw<Opts> = (g, o) => { ... };
//   front("heroi", meuBoneco, { neutro: { expr: "neutro" }, susto: { expr: "susto" }, marcha: marchFrames({ expr: "neutro" }) })
import type { Pack, SpriteItem } from "./types";

export type Draw<O> = (g: CanvasRenderingContext2D, o: O) => void;
type Base = { blink?: number; phase?: number; mode?: string; wave?: number; tilt?: number };
export const N = 8; // frames por ciclo (padrão)
export const cyc = (k: number, n = N) => k / n;
const pad = (k: number) => String(k).padStart(2, "0");

type PackOpts = { w?: number; h?: number; scale?: number; anchor?: [number, number]; fps?: Record<string, number> };

/** Personagem de frente. Estado com objeto = pose única + frame `_blink` (blink: 1). Estado com lista = ciclo. */
export const front = <O extends Base>(name: string, draw: Draw<O>, states: Record<string, O | O[]>, opts: PackOpts = {}): Pack => {
  const items: SpriteItem[] = [];
  for (const [state, spec] of Object.entries(states)) {
    if (Array.isArray(spec)) spec.forEach((o, k) => items.push({ file: `${state}_${pad(k)}`, state, draw: (g) => draw(g, o) }));
    else {
      items.push({ file: state, state, draw: (g) => draw(g, spec) });
      items.push({ file: `${state}_blink`, state, role: "blink", draw: (g) => draw(g, { ...spec, blink: 1 }) });
    }
  }
  return { name, w: opts.w ?? 600, h: opts.h ?? 1000, scale: opts.scale ?? 1.6, anchor: opts.anchor ?? [0.5, 0.975], fps: { walk: 10, march: 8, cheer: 10, dance: 6, ...opts.fps }, items };
};

/** Personagem de perfil num único ciclo (marcha/caminhada lateral). */
export const side = <O extends Base>(name: string, draw: Draw<O>, state: string, o: O, opts: PackOpts & { frames?: number } = {}): Pack => {
  const n = opts.frames ?? N;
  return {
    name, w: opts.w ?? 600, h: opts.h ?? 1000, scale: opts.scale ?? 1.4, anchor: opts.anchor ?? [0.5, 0.975], fps: { [state]: 8, ...opts.fps },
    items: Array.from({ length: n }, (_, k) => ({ file: `${state}_${pad(k)}`, state, draw: (g: CanvasRenderingContext2D) => draw(g, { ...o, phase: cyc(k, n) }) })),
  };
};

/** Efeito/prop: estado "play" estático (frames=1) ou ciclo com t = 0..1. */
export const fx = (name: string, w: number, h: number, draw: (g: CanvasRenderingContext2D, t: number) => void,
  opts: { frames?: number; fps?: number; anchor?: [number, number]; scale?: number } = {}): Pack => {
  const n = opts.frames ?? 1;
  return {
    name, w, h, scale: opts.scale ?? 1.5, anchor: opts.anchor ?? [0.5, 0.5], fps: { play: opts.fps ?? 12 },
    items: Array.from({ length: n }, (_, k) => ({ file: n > 1 ? `play_${pad(k)}` : "play", state: "play", draw: (g: CanvasRenderingContext2D) => draw(g, n > 1 ? k / (n - 1) : 0) })),
  };
};

// ciclos prontos: recebem a pose base e variam fase/onda/inclinação (valores arredondados = PNGs estáveis entre exports)
export const marchFrames = <O extends Base>(o: O, n = N): O[] => Array.from({ length: n }, (_, k) => ({ ...o, mode: "march", phase: cyc(k, n) }));
export const strollFrames = <O extends Base>(o: O, n = N): O[] => Array.from({ length: n }, (_, k) => ({ ...o, mode: "stroll", phase: cyc(k, n) }));
export const cheerFrames = <O extends Base>(o: O, n = N): O[] => Array.from({ length: n }, (_, k) => ({ ...o, wave: Math.round(Math.sin(2 * Math.PI * cyc(k, n)) * 30) / 30 }));
export const danceFrames = <O extends Base>(o: O, n = N, deg = 6): O[] => Array.from({ length: n }, (_, k) => ({ ...o, tilt: Math.round(deg * Math.sin(2 * Math.PI * cyc(k, n)) * 10) / 10 }));
