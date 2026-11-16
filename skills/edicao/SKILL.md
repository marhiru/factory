---
name: edicao
description: "Templates de edição, export e QA da fábrica de vídeo: mix de áudio com ducking (vf mix), render, export por formato com loudnorm e enquadramento (vf export: yt-long, yt-short, tiktok), QA frame a frame (vf qa: pretos, congelados, pops, loudness, specs), recorte de vídeo longo em shorts e pacote de publicação (título, descrição, capítulos, thumbnail). Use na etapa final de um projeto video-factory ou para exportar/checar/recortar um vídeo."
---

# Edição, export e QA (etapa 4 da fábrica)

`VF=${CLAUDE_PLUGIN_ROOT}/bin/vf`

## Pipeline por formato
Antes de tudo: `node ${CLAUDE_PLUGIN_ROOT}/skills/animacao/scripts/edl-check.mjs data/edl.json` precisa sair com código 0. Se reprovar, volte para a skill `animacao`: não se renderiza nem se entrega vídeo sem animação.

Para cada formato em `factory.yaml > formatos`:
1. **Mix**: `$VF mix data/edl.json -o public/audio/mix.wav` (voz 0 dB, música com ducking de ~10 dB, SFX conforme `gain_db`). Para motores com som próprio (cartoon-3d-short), use o mix deles.
2. **Render** (pelo `renderer` do `factory.yaml`):
   ```
   # mygo (padrão para remotion-2d / kenburns / ia-video): GPU nativa, ~5x mais rápido
   ${CLAUDE_PLUGIN_ROOT}/bin/vf-mygo -project . -formato <formato> -o out/video_<formato>.mp4
   # remotion
   npx remotion render src/index.ts <formato> out/video_<formato>.mp4 --codec h264 --crf 16 --muted
   ```
   Shorts em 2K: `-scale 1.3333` no mygo ou `--scale 1.3333` no Remotion (1440x2560). O formato aceita e o YouTube comprime melhor a partir de 2K.
3. **Export** (mux, loudnorm em duas passadas para -14 LUFS / -1 dBTP, AAC 48 kHz, faststart):
   ```
   $VF export out/video_<formato>.mp4 -f <formato> -o out/final_<formato>.mp4 --audio public/audio/mix.wav
   ```
   Se o vídeo já está no tamanho certo em h264 yuv420p, o vídeo é copiado sem recodificar. `--fit pad|crop|blur` controla o reenquadramento quando o aspecto difere. `--fast` usa o encoder de hardware (bom para prévias).
4. **QA**:
   ```
   $VF qa out/final_<formato>.mp4 -f <formato> -o out/qa_<formato> --strict
   ```
   Leia o `qa.md` e abra 2-3 sheets (`out/qa_<formato>/sheets/qa_*.jpg`) nos trechos de gancho, virada e final. Crossfades e cortes detectados já são ignorados; para transições planejadas extras, passe `--plan plan.json` (`{"flash":[a,b],"xfade":[[a,b]],"planned":[[a,b]]}`).
5. **Corrija e repita** até não sobrar `FAIL`. Um `WARN` aceito precisa ser justificado em `factory.yaml > notas`.
6. Registre em `entregas` e marque `edicao`, `qa` e `entrega` como `feito`.

## Como ler o QA
| Item | FAIL quando | Correção típica |
|---|---|---|
| duração | fora da faixa do formato | cortar beats; Shorts > 180 s não vira Short |
| resolução | aspecto errado | composição errada no render, ou `--fit` no export |
| áudio | sem trilha | o Remotion grava mudo: faça o export com `--audio` |
| frames pretos | > 2 frames seguidos fora de transição | clipe com `end` antes do próximo `start` (buraco no EDL) |
| congelados (WARN) | hold ≥ 12 f sem movimento | adicionar `motion` ao still |
| pops de 1 frame | frame isolado diferente dos vizinhos | clipe de 1 frame no EDL, ou erro de arredondamento entre `end` e `start` |
| loudness (WARN) | fora de -14 ±1 LUFS | rodar o export sem `--no-loudnorm` |

## Outros templates
- `templates/recorte.md`: vídeo longo → shorts (seleção por transcrição, reenquadramento, legendas).
- `templates/publicacao.md`: título, descrição com capítulos e atribuições, tags, thumbnail e checklist por plataforma.
