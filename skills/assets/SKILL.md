---
name: assets
description: "Templates para fabricar os assets de um vídeo da fábrica: style bible, inventário com licença (manifest.yaml), narração ElevenLabs com timestamps por palavra (vf transcribe), SFX e trilha (ElevenLabs, procedural local ou CC0) e imagens/clipes de bancos CC0/stock. Use na etapa de assets/voz de um projeto video-factory ou quando precisar gerar voz, efeitos, música ou buscar imagens para um vídeo."
---

# Assets e voz (etapa 2 da fábrica)

Entrada: `roteiro/beats.json` aprovado e `factory.yaml`. Saída: todo arquivo em `public/`, registrado em `assets/manifest.yaml`, mais `data/words.json`.

`VF=${CLAUDE_PLUGIN_ROOT}/bin/vf`

## Ordem
1. **Style bible** (`assets/style-bible.md`, template em `templates/style-bible.md`): paleta, tipografia, tratamento de imagem, personagens e o que nunca fazer. Escreva uma vez por canal e reaproveite entre vídeos.
2. **Lista de assets a partir dos beats**: para cada beat, o que o `visual` exige. Registre cada item no `manifest.yaml` com `status: pendente` **antes** de buscar ou gerar.
3. **Voz** (`templates/voz-elevenlabs.md`): é a régua de tempo de todo o resto, então vem antes de animar.
4. **Timestamps**: `$VF transcribe public/voice/voice.wav -o data --lang pt` gera `data/words.json`, `transcript.srt` e `transcript.txt`. Para nomes próprios difíceis, passe `--prompt "<nomes próprios>, Ouro Preto, Ilha Fiscal"`. Para precisão máxima, use `--dtw`. Se você já tem o JSON do Speech-to-Text do ElevenLabs (Scribe), use `$VF transcribe alinhamento.json -o data --elevenlabs`.
5. **Amarrar o roteiro**: `$VF align roteiro/beats.json data/words.json`.
6. **SFX e música** (`templates/sfx-musica.md`), já sabendo os tempos de cada beat.
7. **Imagens e clipes** (`templates/fontes-cc0.md`): CC0 primeiro, stock depois, procedural quando não houver nada bom.
8. Marque `etapas.assets` e `etapas.voz` como `feito`.

## Regras
- **Nada entra no vídeo sem linha no manifest** com `origem`, `licenca`, `url` (se baixado) e `autor` (se a licença exigir atribuição). Atribuições obrigatórias vão para a descrição do vídeo.
- Nunca use material de outro canal, logos ou marcas, nem rostos reais de pessoas privadas. Figuras históricas só com imagem em domínio público.
- Nomes de arquivo: `public/img/b03-porto.jpg` (beat + descrição), `public/clips/b07-desfile.mp4`, `public/audio/sfx/whoosh-01.wav`.
- Imagens: pelo menos o tamanho do formato (1920 de largura no longo, 1080x1920 no vertical) e mais 20% de margem se tiverem Ken Burns. Converta para JPG q90 ou WebP; PNG só com alpha (camadas de parallax).
- Clipes: corte só o trecho usado (`ffmpeg -ss A -to B -c:v libx264 -crf 18 -an`). `public/` vai inteiro para cada render do Remotion.
- Áudio: WAV 48 kHz. A voz nunca é processada além de um corte de silêncio no começo e no fim.
