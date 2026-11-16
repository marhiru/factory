// Tipos do edl.json (timeline neutra da fábrica). Tempos em segundos.
import edlJson from "../../data/edl.json";
import wordsJson from "../../data/words.json";

export type Word = { word: string; start: number; end: number };
export type Pose = { scale?: number; x?: number; y?: number; rotate?: number };
export type Ease = "linear" | "in" | "out" | "inOut";
export type Motion = { from?: Pose; to?: Pose; ease?: Ease };
export type Layer = { src: string; depth: number }; // depth 0 = fundo, 1 = primeiro plano
export type Transition = { kind: "cut" | "fade" | "flash" | "slide"; dur?: number };

type Base = {
  id?: string;
  start: number;
  end: number;
  layer?: number; // ordem de empilhamento (maior = por cima)
  transition_in?: Transition;
  motion?: Motion;
};
export type Clip =
  | (Base & { type: "image"; src: string; fit?: "cover" | "contain"; layers?: Layer[] })
  | (Base & { type: "video"; src: string; trim?: number; fit?: "cover" | "contain"; volume?: number; playbackRate?: number })
  | (Base & { type: "text"; text: string; style?: "title" | "lower" | "label" | "number"; color?: string })
  | (Base & { type: "color"; color: string })
  | (Base & { type: "scene"; scene: SceneSpec });

// ---- animação cutout (clip "scene"): ver templates/edl.schema.md ----
export type AnimKey = {
  t?: number; at_word?: number; x?: number; y?: number; scale?: number; rot?: number; opacity?: number;
  state?: string; flip?: boolean; ease?: Ease;
};
export type AnimMove = { kind: "pop" | "slide-left" | "slide-right" | "drop" | "rise" | "fade"; at?: number; at_word?: number; dur?: number };
export type Actor = {
  id: string; sprite: string; h?: number; depth?: number; x?: number; y?: number; scale?: number; rot?: number; opacity?: number;
  state?: string; flip?: boolean; keys?: AnimKey[]; idle?: "breathe" | "bob" | "sway" | "float" | "jitter" | "none";
  blink?: boolean; shadow?: boolean; enter?: AnimMove; exit?: AnimMove;
  /** recorta o ator abaixo desta linha (fração da altura do quadro): atrás de mesa, muro, janela */
  clip_y?: number;
};
export type SceneSpec = { bg?: { src?: string; color?: string; fit?: "cover" | "contain" }; camera?: Motion; actors: Actor[] };
export type SpriteState = { frames: string[]; fps?: number; blink?: string };
export type SpriteDef = { w: number; h: number; anchor?: [number, number]; states: Record<string, SpriteState> };

export type AudioTrack = { src: string; gain_db?: number; start?: number; trim?: number; loop?: boolean };
export type Captions = { enabled: boolean; words_per_page?: number; style?: "karaoke" | "pop" | "plain" };
export type FontSpec = { family: string; src: string }; // src relativo a public/ (ttf/otf/woff2)
export type Edl = {
  fps: number;
  font?: FontSpec;
  duration: number;
  background?: string;
  audio: AudioTrack[];
  clips: Clip[];
  captions?: Captions;
};

export const EDL = edlJson as unknown as Edl;
export const WORDS = wordsJson as Word[];
export const FPS = EDL.fps;
export const TOTAL_FRAMES = Math.max(1, Math.ceil(EDL.duration * EDL.fps));
export const sec = (s: number) => Math.round(s * FPS);

/** Tempo (s) do início da palavra de índice k: amarre eventos visuais a palavras, não a segundos. */
export const wordAt = (k: number) => WORDS[Math.max(0, Math.min(WORDS.length - 1, k))]?.start ?? 0;

/** Família de fonte do vídeo: a do edl.json (mesmo arquivo em qualquer renderer) ou a pilha padrão. */
export const FONT = EDL.font ? `'${EDL.font.family}', 'Helvetica Neue', Arial, sans-serif` : "'Inter', 'Helvetica Neue', Arial, sans-serif";
