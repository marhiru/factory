// Espelho de formats.yaml (plugin video-factory). Safe zones em px no tamanho nativo.
export type FormatId = "yt-long" | "yt-short" | "tiktok";
export type Safe = { top: number; bottom: number; left: number; right: number };
export type Fmt = { id: FormatId; width: number; height: number; safe: Safe };

export const FORMATS: Record<FormatId, Fmt> = {
  "yt-long": { id: "yt-long", width: 1920, height: 1080, safe: { top: 0, bottom: 0, left: 0, right: 0 } },
  "yt-short": { id: "yt-short", width: 1080, height: 1920, safe: { top: 140, bottom: 380, left: 40, right: 120 } },
  tiktok: { id: "tiktok", width: 1080, height: 1920, safe: { top: 160, bottom: 480, left: 60, right: 140 } },
};
