import React from "react";
import { AbsoluteFill, interpolate, OffthreadVideo, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import type { Clip } from "../lib/edl";
import type { Safe } from "../lib/formats";
import { KenBurns } from "./KenBurns";
import { TextCard } from "./TextCard";
import { mix, progress } from "../lib/ease";

export const ClipView: React.FC<{ clip: Clip; dur: number; safe: Safe }> = ({ clip, dur, safe }) => {
  const frame = useCurrentFrame();
  const { fps, width } = useVideoConfig();
  const tr = clip.transition_in;
  const tf = Math.max(1, Math.round((tr?.dur ?? 0.25) * fps));
  const opacity = tr?.kind === "fade" ? interpolate(frame, [0, tf], [0, 1], { extrapolateRight: "clamp" }) : 1;
  const slide = tr?.kind === "slide" ? interpolate(frame, [0, tf], [width, 0], { extrapolateRight: "clamp", easing: (x) => 1 - Math.pow(1 - x, 3) }) : 0;
  const flash = tr?.kind === "flash" ? interpolate(frame, [0, tf], [1, 0], { extrapolateRight: "clamp" }) : 0;

  let body: React.ReactNode = null;
  if (clip.type === "image") body = <KenBurns src={clip.src} layers={clip.layers} motion={clip.motion} fit={clip.fit} dur={dur} />;
  else if (clip.type === "video") {
    const p = mix(progress(frame, dur, clip.motion?.ease), clip.motion?.from, clip.motion?.to);
    body = (
      <AbsoluteFill style={{ overflow: "hidden", transform: `translate(${p.x * 100}%, ${p.y * 100}%) scale(${p.scale})` }}>
        <OffthreadVideo
          src={staticFile(clip.src)}
          startFrom={Math.round((clip.trim ?? 0) * fps)}
          volume={clip.volume ?? 0}
          playbackRate={clip.playbackRate ?? 1}
          style={{ width: "100%", height: "100%", objectFit: clip.fit ?? "cover" }}
        />
      </AbsoluteFill>
    );
  } else if (clip.type === "text") body = <TextCard text={clip.text} style={clip.style} color={clip.color} dur={dur} safe={safe} />;
  else if (clip.type === "color") body = <AbsoluteFill style={{ background: clip.color }} />;

  return (
    <AbsoluteFill style={{ opacity, transform: slide ? `translateX(${slide}px)` : undefined }}>
      {body}
      {flash > 0 && <AbsoluteFill style={{ background: "#fff", opacity: flash }} />}
    </AbsoluteFill>
  );
};
