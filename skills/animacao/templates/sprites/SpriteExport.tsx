import React, { useLayoutEffect, useRef } from "react";
import { AbsoluteFill, Composition, continueRender, delayRender, useCurrentFrame } from "remotion";
import type { Pack } from "./types";

type Props = { pack: string; names?: string[]; meta?: unknown };

/** Composição "Sprites": cada frame é um item do pack pedido (inputProps.pack), desenhado num canvas transparente. */
export const SpritesRoot: React.FC<{ packs: Pack[] }> = ({ packs }) => {
  const Draw: React.FC<Props> = ({ pack }) => {
    const frame = useCurrentFrame();
    const p = packs.find((x) => x.name === pack)!;
    const ref = useRef<HTMLCanvasElement>(null);
    const [handle] = React.useState(() => delayRender(`sprite ${pack}#${frame}`));
    useLayoutEffect(() => {
      const cv = ref.current!;
      const g = cv.getContext("2d")!;
      g.setTransform(1, 0, 0, 1, 0, 0);
      g.clearRect(0, 0, cv.width, cv.height);
      g.setTransform(p.scale, 0, 0, p.scale, 0, 0);
      p.items[frame].draw(g);
      continueRender(handle);
    }, [frame, p, handle]);
    return (
      <AbsoluteFill style={{ background: "transparent" }}>
        <canvas ref={ref} width={Math.round(p.w * p.scale)} height={Math.round(p.h * p.scale)} />
      </AbsoluteFill>
    );
  };
  return (
    <Composition
      id="Sprites"
      component={Draw}
      durationInFrames={1}
      fps={30}
      width={100}
      height={100}
      defaultProps={{ pack: packs[0]?.name ?? "" } as Props}
      calculateMetadata={({ props }) => {
        const p = packs.find((x) => x.name === props.pack) ?? packs[0];
        const meta = { name: p.name, w: p.w, h: p.h, scale: p.scale, anchor: p.anchor, fps: p.fps, items: p.items.map(({ file, state, role }) => ({ file, state, role })) };
        return {
          width: Math.round(p.w * p.scale),
          height: Math.round(p.h * p.scale),
          durationInFrames: p.items.length,
          props: { pack: p.name, names: packs.map((x) => x.name), meta },
        };
      }}
    />
  );
};
