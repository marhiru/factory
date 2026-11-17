# Origem dos templates cartoon

Código adaptado de https://github.com/mromidev/claude-animated-shorts, commit `e3b817b6afc6cd413466d43f11ba31209e034bbf` (MIT, copyright 2026 Mr omi; licença em `LICENSE`).

- `src/videos/cartoon/`: o exemplo técnico da ponte, com carregamento do kit Kenney opcional (`inputProps.kenney`). Os GLBs não estão incluídos. Para usá-los, obtenha o kit CC0 e registre origem/licença no manifest; coloque os arquivos em `public/bridge/kenney/`.
- Áudio procedural correspondente em `skills/assets/templates/audio-procedural/` (com licença própria) e ferramentas de stills/comparação em `skills/edicao/templates/cartoon/` (com licença própria).
- Adaptações locais: Bun, tsgo, composição `CartoonShort`, entrada/tsconfig/dependências, caminhos da voz e palavras da fábrica, Chrome configurável por `REMOTION_BROWSER` e temporários por `REMOTION_TMP`, filhos opcionais do EffectComposer filtrados para tipagem estrita, e plano de QA derivado das constantes TS (helper local).

O enredo, as constantes de tempo e as roupas são **exemplo**, não um novo vídeo pronto. Substitua `story.ts`, o layout em `build.ts`/`world.ts`, o elenco em `toon.ts`/`cast.ts` e a timeline de áudio para cada história original. Stills e métricas do canal estudado não foram incorporados.

O idioma inglês, ausência de legendas, ganhos de música e revisão por um modelo específico eram escolhas do projeto original: use as escolhas de `factory.yaml` e do usuário.
