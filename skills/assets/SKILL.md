---
name: assets
description: "Templates para fabricar os assets de um vídeo da fábrica: style bible, inventário com licença (manifest.yaml), imagens geradas no Google Flow (Nano Banana) via MCP em lotes de 24 com cota diária global e curadoria por folha de contato, narração local com Chatterbox (vf voz, pt-BR, sem custo) ou ElevenLabs, com timestamps por palavra (vf transcribe), SFX e trilha (ElevenLabs, procedural local ou CC0) e imagens/clipes de bancos CC0/stock. Use na etapa de assets/voz de um projeto video-factory ou quando precisar gerar voz, efeitos, música ou buscar imagens para um vídeo."
---

# Assets e voz (etapa 2 da fábrica)

Ferramental JS/TS: siga [Bun e tsgo/tsrs](../factory/references/ferramental.md) ao executar ou alterar scripts e templates.

Entrada: `roteiro/beats.json` aprovado e `factory.yaml`. Saída: todo arquivo em `public/`, registrado em `assets/manifest.yaml`, mais `data/words.json`.

`VF=${CLAUDE_PLUGIN_ROOT}/bin/vf`

## Assets cartoon
Para elenco, texturas, voz alinhada e trilha sintetizada, leia [assets e som cartoon](references/cartoon-assets-som.md). Os templates MIT em `templates/audio-procedural/` trazem DSP, instrumentos, partitura, SFX, timeline e mix; adapte a história antes de executar.

## Ordem
1. **Style bible** (`assets/style-bible.md`, template em `templates/style-bible.md`): paleta, tipografia, tratamento de imagem, personagens e o que nunca fazer. Escreva uma vez por canal e reaproveite entre vídeos.
2. **Lista de assets a partir dos beats**: para cada beat, o que o `visual` exige. Registre cada item no `manifest.yaml` com `status: pendente` **antes** de buscar ou gerar.
3. **Voz**: é a régua de tempo de todo o resto, então vem antes de animar. O provedor vem de `factory.yaml > voz.provedor`:
   - `chatterbox` (local, sem custo, pt-BR, voz do canal clonada de um WAV de referência): `$VF voz voice/texto.txt -o public/voice/voice.wav` → [templates/voz-chatterbox.md](templates/voz-chatterbox.md);
   - `elevenlabs` (API): [templates/voz-elevenlabs.md](templates/voz-elevenlabs.md).
   Clonagem só da própria voz ou com autorização por escrito do dono da voz.
4. **Timestamps**: `$VF transcribe public/voice/voice.wav -o data --lang pt` gera `data/words.json`, `transcript.srt` e `transcript.txt`. Para nomes próprios difíceis, passe `--prompt "<nomes próprios>, Ouro Preto, Ilha Fiscal"`. Para precisão máxima, use `--dtw`. Se você já tem o JSON do Speech-to-Text do ElevenLabs (Scribe), use `$VF transcribe alinhamento.json -o data --elevenlabs`.
5. **Amarrar o roteiro**: `$VF align roteiro/beats.json data/words.json`.
6. **SFX e música** (`templates/sfx-musica.md`), já sabendo os tempos de cada beat.
7. **Imagens e clipes** (`templates/fontes-cc0.md`): CC0 primeiro, stock depois, procedural quando não houver nada bom.
8. Marque `etapas.assets` e `etapas.voz` como `feito`.

## Imagens do Google Flow (motor `estatico`, e qualquer projeto que precise de ilustração)
`FL=${CLAUDE_PLUGIN_ROOT}/skills/assets/scripts/flow-lotes.mjs` · instalação/login do MCP: `templates/mcp-google-flow.md`
1. **Style bible do tema** (`templates/style-bible-tema.md`) → `assets/flow/estilo.json` (`sufixo`, `negativos`, `cenarios`, `referencias`). Prompts: `templates/prompt-nano-banana.md`.
2. **Lote de estilo:** 1 prompt do cenário principal, sem referências. Escolha 3-6 imagens que definem o look → `assets/ref/estilo-NN.jpg` → `estilo.json > referencias`.
3. `bun $FL plan` → `assets/flow/lotes.json`. **1 lote = 1 prompt = 1 cenário = 24 imagens**, com até 8 planos (divididos por igual) e o resto como reserva.
4. Loop: `bun $FL proximo` → chamar `flow_generate_image` do MCP com os `args` exatos (`count` é sempre o máximo) → `bun $FL registrar <lote> <output_dir>` → `$VF sheet assets/flow/<lote>`. Repita até `fim`. Com `cota_esgotada` (código 3), marque `etapas.assets: aguardando_cota` e pare.
5. **Curadoria** (olhe as folhas, não pule): para cada plano, a imagem que melhor mostra o `visual` do beat. Rejeite texto, mãos/rostos deformados, anacronismo e estilo fora da bíblia. Imagem nenhuma serve para dois planos a menos de 2 min de distância. Grave `assets/flow/curadoria.json` (`{"b3-2": "assets/flow/<lote>/<arquivo>", "_rejeitadas": {...}}`). Se faltar plano bom, ele vai para um lote novo com o prompt ajustado.
6. `bun ${CLAUDE_PLUGIN_ROOT}/skills/assets/scripts/curadoria.mjs aplicar` → `public/img/planos/<plano>.jpg` (upscale automático abaixo de 2150 px) + manifest. `curadoria.mjs faltando` precisa sair 0.
7. Marque `lotes_flow` e `curadoria` como `feito`.

## Regras
- **Nada entra no vídeo sem linha no manifest** com `origem`, `licenca`, `url` (se baixado) e `autor` (se a licença exigir atribuição). Atribuições obrigatórias vão para a descrição do vídeo.
- Nunca use material de outro canal, logos ou marcas, nem rostos reais de pessoas privadas. Figuras históricas só com imagem em domínio público.
- Nomes de arquivo: `public/img/b03-porto.jpg` (beat + descrição), `public/clips/b07-desfile.mp4`, `public/audio/sfx/whoosh-01.wav`.
- Imagens: pelo menos o tamanho do formato (1920 de largura no longo, 1080x1920 no vertical) e mais 20% de margem se tiverem Ken Burns. Converta para JPG q90 ou WebP; PNG só com alpha (camadas de parallax).
- Clipes: corte só o trecho usado (`ffmpeg -ss A -to B -c:v libx264 -crf 18 -an`). `public/` vai inteiro para cada render do Remotion.
- Áudio: WAV 48 kHz. A voz nunca é processada além de um corte de silêncio no começo e no fim.

## Qualidade dos modelos e assets visuais

Ao produzir elenco/props, aplique as etapas 1 e 4 do [QA visual em cinco etapas](../edicao/references/qa-cinco-etapas.md).
Confira silhueta/volume isolados e detalhes funcionais no tamanho de exibição: ponteiros,
rótulos, transparência e frente/verso. Assets em 3D precisam de vistas úteis, não apenas
um hero frame com DOF. Registre a evidência; a integração de posicionamento é revista na animação.

## FX, luz e bibliotecas de referência

- `ref/fx/pixelfx` (MIT) e `ref/fx/pixel-splats` (Apache-2.0) chegam com o `vf init`. Todo FX importado ou
  gerado (`fx-pack.mjs`, `FxSheet`) entra em `assets/manifest.yaml` com fonte e licença (o pack traz `ORIGEM.json`).
- A style bible registra o rig de luz de cada cenário (direção/cor da chave, preenchimento, práticos) e a paleta
  dos FX; o iluminador e o artista de FX leem dali ([luz-fx-shaders.md](../animacao/references/luz-fx-shaders.md)).
- Sons prontos: `audio/procedural/` (cópia de `templates/audio-procedural/`, MIT) no projeto.

## Posicionamento e identidade estável

- Todo asset é posto com coesão no cenário construído: apoiado no suporte certo, em escala e
  perspectiva do plate, sem quebrar a harmonia (luz, paleta, estilo) e sem tapar o assunto,
  a ação, a legenda ou a safe zone. Mal posicionado reprova ([etapa 3](../edicao/references/qa-cinco-etapas.md)).
- Pedido de correção (ex.: "o personagem está dentro do sofá") muda só a posição/pose do
  **mesmo** asset, mesmo pack e mesmo id no manifest. Nunca substitua por um asset parecido
  ou regenerado do zero; se faltar uma pose, gere-a da mesma referência fixa e confira antes ×
  depois lado a lado ([etapa 3b](../edicao/references/qa-cinco-etapas.md)).
