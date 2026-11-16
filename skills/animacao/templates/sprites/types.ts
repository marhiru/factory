// Contrato do exportador de sprites da video-factory.
// Um "pack" é um personagem/prop; cada item é um PNG (frame de um estado ou a piscada desse estado).
export type SpriteItem = {
  /** nome do arquivo sem extensão, ex. "neutral", "walk_03", "neutral_blink" */
  file: string;
  /** estado a que pertence (neutral, shock, walk...) */
  state: string;
  /** frame do estado (ordem = ordem no array) ou o frame de olhos fechados do estado */
  role?: "frame" | "blink";
  /** desenha no espaço de design (w x h unidades); o canvas já vem escalado */
  draw: (g: CanvasRenderingContext2D) => void;
};
export type Pack = {
  name: string;
  /** canvas de design em unidades (ex. 600 x 1000 para um boneco com as solas em y = 975) */
  w: number;
  h: number;
  /** px por unidade no PNG final (1.5 => 900 x 1500) */
  scale: number;
  /** ponto do sprite que vai na posição (x, y) do ator, em frações do canvas; padrão [0.5, 0.975] (pés) */
  anchor?: [number, number];
  /** fps dos estados com vários frames (ciclos) */
  fps?: Record<string, number>;
  items: SpriteItem[];
};
