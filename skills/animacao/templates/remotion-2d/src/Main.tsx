import React from "react";
import { AbsoluteFill, Audio, Sequence, staticFile } from "remotion";
import { Captions } from "./components/Captions";
import { ClipView } from "./components/ClipView";
import { EDL, sec, WORDS } from "./lib/edl";
import type { Clip } from "./lib/edl";
import { FORMATS, type FormatId } from "./lib/formats";

/** Estende o clipe de baixo para cobrir a entrada em fade do clipe seguinte na mesma camada. */
const withOverlap = (clips: Clip[]): (Clip & { _end: number })[] =>
  clips.map((c) => {
    const next = clips.find((n) => (n.layer ?? 0) === (c.layer ?? 0) && Math.abs(n.start - c.end) < 1e-3 && n.transition_in?.kind === "fade");
    return { ...c, _end: c.end + (next ? next.transition_in?.dur ?? 0.25 : 0) };
  });

export const Main: React.FC<{ format: FormatId }> = ({ format }) => {
  const safe = FORMATS[format].safe;
  const clips = withOverlap([...EDL.clips].sort((a, b) => (a.layer ?? 0) - (b.layer ?? 0) || a.start - b.start));
  return (
    <AbsoluteFill style={{ background: EDL.background ?? "#000" }}>
      {clips.map((c, k) => {
        const from = sec(c.start);
        const dur = Math.max(1, sec(c._end) - from);
        return (
          <Sequence key={c.id ?? k} from={from} durationInFrames={dur} name={c.id ?? `${c.type}-${k}`}>
            <ClipView clip={c} dur={dur} safe={safe} />
          </Sequence>
        );
      })}
      {EDL.captions?.enabled && WORDS.length > 0 && <Captions words={WORDS} cfg={EDL.captions} safe={safe} />}
      {EDL.audio.map((a, k) => (
        <Sequence key={`a${k}`} from={sec(a.start ?? 0)} name={`audio:${a.src}`}>
          <Audio src={staticFile(a.src)} volume={Math.pow(10, (a.gain_db ?? 0) / 20)} startFrom={sec(a.trim ?? 0)} loop={a.loop} />
        </Sequence>
      ))}
    </AbsoluteFill>
  );
};
