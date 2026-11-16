// Matemática da animação cutout: espelho de tools/vf-mygo/scene.go (os dois renderers dão o mesmo resultado).
import { continueRender, delayRender, staticFile } from "remotion";
import { EDL, FPS, WORDS } from "./edl";
import type { Actor, AnimKey, AnimMove, Clip, Ease, SpriteDef } from "./edl";
import { EASE_FNS } from "./ease";

export const hh = (i: number, s: number) => {
  const v = Math.sin(i * 127.1 + s * 311.7) * 43758.5453;
  return v - Math.floor(v);
};
export const strHash = (s: string) => {
  let h = 0;
  for (const ch of s) h = (h * 31 + (ch.codePointAt(0) ?? 0)) % 9973;
  return h;
};
const clamp01 = (x: number) => Math.max(0, Math.min(1, x));
const lerp = (a: number, b: number, p: number) => a + (b - a) * p;

// ---- sprites: sprite.json de cada pasta usada, carregados antes do 1º frame ----
export const SPRITES: Record<string, SpriteDef> = {};
const dirs = new Set<string>();
for (const c of EDL.clips) if (c.type === "scene") for (const a of c.scene.actors) dirs.add(a.sprite);
if (dirs.size && typeof window !== "undefined") {
  const h = delayRender("sprites");
  Promise.all(
    [...dirs].map((d) =>
      fetch(staticFile(`${d}/sprite.json`))
        .then((r) => r.json())
        .then((j: SpriteDef) => (SPRITES[d] = j)),
    ),
  )
    .then(() => continueRender(h))
    .catch((e) => {
      console.error(e);
      continueRender(h);
    });
}
export const firstState = (sp: SpriteDef) => ("neutral" in sp.states ? "neutral" : Object.keys(sp.states).sort()[0]);

// at_word -> segundos relativos ao clipe (2 frames antes da palavra)
const resolve = (clip: Clip, w: number | undefined, t: number | undefined, def: number) =>
  w !== undefined && WORDS.length ? WORDS[Math.max(0, Math.min(WORDS.length - 1, w))].start - clip.start - 2 / FPS : t ?? def;

export type Prepared = Actor & { _keys: (AnimKey & { _t: number })[]; _enter?: AnimMove & { _t: number; _d: number }; _exit?: AnimMove & { _t: number; _d: number }; _seed: number; _swaps: number[] };
export const prepare = (clip: Clip, a: Actor): Prepared => {
  const keys = (a.keys ?? []).map((k) => ({ ...k, _t: resolve(clip, k.at_word, k.t, 0) })).sort((x, y) => x._t - y._t);
  return {
    ...a,
    _keys: keys,
    _enter: a.enter && { ...a.enter, _t: resolve(clip, a.enter.at_word, a.enter.at, 0), _d: a.enter.dur || 0.35 },
    _exit: a.exit && { ...a.exit, _t: resolve(clip, a.exit.at_word, a.exit.at, clip.end - clip.start), _d: a.exit.dur || 0.3 },
    _seed: strHash(a.id + a.sprite),
    _swaps: keys.filter((k) => k.state !== undefined).map((k) => k._t),
  };
};

const numAt = (t: number, base: number, keys: (AnimKey & { _t: number })[], get: (k: AnimKey) => number | undefined) => {
  let v = base, pt = -Infinity;
  for (const k of keys) {
    const p = get(k);
    if (p === undefined) continue;
    if (t >= k._t) { v = p; pt = k._t; continue; }
    if (pt === -Infinity) return v;
    const e = EASE_FNS[(k.ease ?? "inOut") as Ease] ?? EASE_FNS.inOut;
    return lerp(v, p, e(clamp01((t - pt) / Math.max(1e-6, k._t - pt))));
  }
  return v;
};
const backOut = (u: number) => { const c1 = 1.4, c3 = c1 + 1; return 1 + c3 * Math.pow(u - 1, 3) + c1 * Math.pow(u - 1, 2); };
export const springAt = (frame: number, fps: number, damping: number, stiffness: number, mass: number) => {
  if (frame <= 0) return 0;
  const t = frame / fps, w0 = Math.sqrt(stiffness / mass), z = damping / (2 * Math.sqrt(stiffness * mass));
  if (z < 1) { const wd = w0 * Math.sqrt(1 - z * z); return 1 - Math.exp(-z * w0 * t) * (Math.cos(wd * t) + ((z * w0) / wd) * Math.sin(wd * t)); }
  return 1 - Math.exp(-w0 * t) * (1 + w0 * t);
};

const blinking = (a: Prepared, f: number) => {
  for (const ts of a._swaps) { const d = f - Math.round(ts * FPS) + 1; if (d >= 0 && d < 3) return true; }
  let s = Math.round(-60 * hh(a._seed, 9));
  for (let n = 0; s <= f && n < 2000; n++) {
    const d = f - s;
    if (d >= 0 && d < 4) return true;
    s += Math.round(15 + 54 * hh(a._seed * 31 + n, 13));
  }
  return false;
};

export type ActorPose = { x: number; y: number; sx: number; sy: number; rot: number; op: number; flip: boolean; img: string; visible: boolean };
export const actorAt = (a: Prepared, sp: SpriteDef, t: number, frame: number): ActorPose => {
  const p: ActorPose = { x: 0, y: 0, sx: 1, sy: 1, rot: 0, op: 1, flip: false, img: "", visible: true };
  p.x = numAt(t, a.x ?? 0, a._keys, (k) => k.x);
  p.y = numAt(t, a.y ?? 0, a._keys, (k) => k.y);
  const s = numAt(t, a.scale ?? 1, a._keys, (k) => k.scale);
  p.rot = numAt(t, a.rot ?? 0, a._keys, (k) => k.rot);
  p.op = numAt(t, a.opacity ?? 1, a._keys, (k) => k.opacity);
  let state = a.state ?? firstState(sp), flip = a.flip ?? false;
  for (const k of a._keys) if (k._t <= t) { if (k.state !== undefined) state = k.state; if (k.flip !== undefined) flip = k.flip; }
  p.flip = flip;
  let sx = s, sy = s;
  for (const mv of [a._enter, a._exit]) {
    if (!mv) continue;
    const isIn = mv === a._enter;
    let u: number;
    if (isIn) { if (t < mv._t) return { ...p, visible: false }; u = clamp01((t - mv._t) / mv._d); }
    else { if (t >= mv._t + mv._d) return { ...p, visible: false }; if (t < mv._t) continue; u = 1 - clamp01((t - mv._t) / mv._d); }
    switch (mv.kind) {
      case "pop": { const g = isIn ? springAt(Math.round((t - mv._t) * FPS), FPS, 11, 210, 0.7) : EASE_FNS.in(u); sx *= g; sy *= g; break; }
      case "slide-left": p.x -= (1 - backOut(u)) * 0.75; break;
      case "slide-right": p.x += (1 - backOut(u)) * 0.75; break;
      case "drop": p.y -= (1 - backOut(u)) * 0.7; break;
      case "rise": p.y += (1 - EASE_FNS.out(u)) * 0.5; break;
      default: p.op *= u;
    }
  }
  const ph = hh(a._seed, 3);
  switch (a.idle ?? "breathe") {
    case "breathe": { const b = Math.sin(2 * Math.PI * (t / 2.6 + ph)); sy *= 1 + 0.012 * b; sx *= 1 - 0.006 * b; break; }
    case "bob": p.y -= 0.008 * Math.abs(Math.sin(Math.PI * (t / 0.45 + ph))); break;
    case "sway": p.rot += 1.6 * Math.sin(2 * Math.PI * (t / 3.2 + ph)); break;
    case "float": p.y += 0.006 * Math.sin(2 * Math.PI * (t / 2.2 + ph)); break;
    case "jitter": p.x += 0.0015 * Math.sin(t * 47 + ph * 9); p.rot += 0.8 * Math.sin(t * 39 + ph * 5); break;
  }
  p.sx = sx; p.sy = sy;
  const st = sp.states[state] ?? sp.states[firstState(sp)];
  if (!st || !st.frames.length) return { ...p, visible: false };
  const idx = st.frames.length > 1 ? Math.floor(t * (st.fps || 12)) % st.frames.length : 0;
  p.img = `${a.sprite}/${st.frames[idx]}`;
  if (st.blink && a.blink !== false && blinking(a, frame)) p.img = `${a.sprite}/${st.blink}`;
  return p;
};
