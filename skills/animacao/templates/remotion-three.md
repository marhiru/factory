# Remotion + three.js: cartoon 2D em mundo 3D

O motor agora está incluído na fábrica: [template local](remotion-three/) e [guia completo](../references/2d-in-3d-world.md). Não é necessário instalar `cartoon-3d-short` separadamente.

- `scripts/novo.sh <pasta> --motor remotion-three --formato yt-short --install` cria o projeto com Bun, tsgo, composição `CartoonShort` e helpers de stills/comparação.
- Fonte dos desenhos, materiais, mundo, timeline e pós em `src/videos/cartoon/`. O template da ponte é demonstração técnica; refaça história, cenário e elenco.
- `beats.json` alinhado + `data/words.json` alimentam `SHOTS`, `CAMS`, tempo físico e reações; uma única régua de voz.
- Áudio procedural opcional: `skills/assets/templates/audio-procedural/`; QA visual e operação: `skills/edicao/references/cartoon-qa.md`.
- Clipes 3D renderizados reais entram como `video` no EDL; `edl-check` + `vf export` + `vf qa` continuam obrigatórios. Não use `short_qa.py` como substituto do QA estrito.
- Kit Kenney opcional não incluído: ver `remotion-three/ORIGEM.md`, que também registra o commit de origem e licença MIT.
