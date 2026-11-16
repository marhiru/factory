# Remotion + three.js (cartoon 2D em mundo 3D)

Este motor tem skill própria: **`cartoon-3d-short`**. Ela traz o pipeline completo (story.ts, world, cast, ink, lente, grade), os valores de look medidos e as armadilhas. Templates: https://github.com/mromidev/claude-animated-shorts (`templates/bridge/`).

Integração com a fábrica:
- O `roteiro/beats.json` com tempos (`vf align`) é a fonte para `SHOTS`, `CAMS` e reações em `story.ts`: cada beat vira 1-3 planos, e as reações caem 0-3 frames antes de `w0`.
- O `data/words.json` substitui o `words.json`/`gen_words.py` da skill.
- Som: `vf mix` com as faixas geradas, ou o `audio/sound.py` da skill.
- Export e QA: `vf export` + `vf qa` (que substitui o `short_qa.py`, mais rápido e com detecção automática de crossfades).
