import React from "react";
import { Composition } from "remotion";
import { BridgeShort, BRIDGE_FRAMES } from "./videos/cartoon/Bridge";
import { FPS } from "./videos/cartoon/story";
// Demonstração técnica herdada da ponte. Substitua história, elenco e layout antes de produzir outro vídeo.
export const Root: React.FC = () => <Composition id="CartoonShort" component={BridgeShort} durationInFrames={BRIDGE_FRAMES} fps={FPS} width={1080} height={1920} />;
