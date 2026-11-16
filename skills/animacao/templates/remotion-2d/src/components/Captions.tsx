import React from "react";
import { AbsoluteFill, spring, useCurrentFrame, useVideoConfig } from "remotion";
import { FONT, type Captions as Cfg, type Word } from "../lib/edl";
import type { Safe } from "../lib/formats";


/** Páginas de N palavras; quebra também em pontuação final e pausas > 0,5 s. */
export const paginate = (words: Word[], n: number): Word[][] => {
  const pages: Word[][] = [];
  let cur: Word[] = [];
  words.forEach((w, k) => {
    cur.push(w);
    const next = words[k + 1];
    if (cur.length >= n || /[.!?…]$/.test(w.word) || (next && next.start - w.end > 0.5) || !next) {
      pages.push(cur);
      cur = [];
    }
  });
  return pages;
};

export const Captions: React.FC<{ words: Word[]; cfg: Cfg; safe: Safe }> = ({ words, cfg, safe }) => {
  const frame = useCurrentFrame();
  const { fps, width, height } = useVideoConfig();
  const t = frame / fps;
  const u = Math.min(width, height) / 1080;
  const pages = React.useMemo(() => paginate(words, cfg.words_per_page ?? 3), [words, cfg.words_per_page]);
  const page = pages.find((p, k) => t >= p[0].start - 0.05 && t < (pages[k + 1]?.[0].start ?? p[p.length - 1].end + 0.4));
  if (!page) return null;
  const vertical = height > width;
  const bottom = vertical ? safe.bottom + 120 * u : 110 * u;
  return (
    <AbsoluteFill style={{ justifyContent: "flex-end", alignItems: "center", paddingBottom: bottom, paddingLeft: safe.left + 40 * u, paddingRight: safe.right + 40 * u }}>
      <div style={{ display: "flex", flexWrap: "wrap", justifyContent: "center", gap: "0 0.32em", fontFamily: FONT, fontWeight: 900, fontSize: (vertical ? 86 : 64) * u, textTransform: "uppercase", lineHeight: 1.1, textAlign: "center" }}>
        {page.map((w, k) => {
          const active = t >= w.start && t < w.end + 0.05;
          const said = t >= w.start;
          const pop = cfg.style === "pop" ? spring({ frame: frame - Math.round(w.start * fps), fps, config: { damping: 12, stiffness: 220 } }) : 1;
          if (cfg.style === "pop" && !said) return null;
          const color = cfg.style === "plain" ? "#fff" : active ? "#ffd23f" : "#fff";
          return (
            <span key={k} style={{ color, WebkitTextStroke: `${10 * u}px #000`, paintOrder: "stroke fill", transform: `scale(${(active && cfg.style === "karaoke" ? 1.06 : 1) * pop})`, transformOrigin: "center", display: "inline-block" }}>
              {w.word}
            </span>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
