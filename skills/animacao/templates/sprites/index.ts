import React from "react";
import { registerRoot } from "remotion";
import { SpritesRoot } from "./SpriteExport";
import { PACKS } from "./registry";

registerRoot(() => React.createElement(SpritesRoot, { packs: PACKS }));
