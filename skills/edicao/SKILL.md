---
name: edicao
description: "Templates de edição, export e QA da fábrica de vídeo: mix de áudio com ducking (vf mix), render, export por formato com loudnorm e enquadramento (vf export: yt-long, yt-short, tiktok), QA frame a frame (vf qa: pretos, congelados, pops, loudness, specs), recorte de vídeo longo em shorts e pacote de publicação (título, descrição, capítulos, thumbnail). Use na etapa final de um projeto video-factory ou para exportar/checar/recortar um vídeo."
---

# Edição, export e QA (etapa 4 da fábrica)

Ferramental JS/TS: siga [Bun e tsgo/tsrs](../factory/references/ferramental.md) ao executar ou alterar scripts e templates.

`VF=${CLAUDE_PLUGIN_ROOT}/bin/vf`

## Revisão cartoon 2D/3D
Leia [cartoon-qa.md](references/cartoon-qa.md) para comparação de look, integração de sandboxes, legibilidade, temporários, escala do pós e QA de clipes 3D. Helpers MIT em `templates/cartoon/`: stills Remotion com Bun e folha lado a lado em Python.

## Pipeline por formato
Antes de tudo: `bun ${CLAUDE_PLUGIN_ROOT}/skills/animacao/scripts/edl-check.mjs data/edl.json` precisa sair com código 0 (o perfil, animado ou `estatico`, vem do próprio edl.json). Se reprovar, volte para a skill `animacao`: não se renderiza nem se entrega vídeo sem animação.

Para cada formato em `factory.yaml > formatos`:
1. **Mix**: `$VF mix data/edl.json -o public/audio/mix.wav` (voz 0 dB, música com ducking de ~10 dB, SFX conforme `gain_db`). Para `remotion-three` com áudio procedural, use o mix pronto ou os stems conforme `skills/assets/references/cartoon-assets-som.md`, sem duplicar faixas.
2. **Render** (pelo `renderer` do `factory.yaml`):
   ```
   # mygo (padrão para remotion-2d / kenburns / ia-video): GPU nativa, ~5x mais rápido
   ${CLAUDE_PLUGIN_ROOT}/bin/vf-mygo -project . -formato <formato> -o out/video_<formato>.mp4
   # hyperframes (padrão do vídeo longo): compila o edl.json em HTML/GSAP, lint e render (captura segmentada > 2 min)
   ${CLAUDE_PLUGIN_ROOT}/skills/edicao/scripts/render-hf.sh <formato> out/video_<formato>.mp4
   # remotion
   bunx --bun remotion render src/index.ts <formato> out/video_<formato>.mp4 --codec h264 --crf 16 --muted
   ```
   Shorts em 2K: `-scale 1.3333` no mygo ou `--scale 1.3333` no Remotion (1440x2560). O formato aceita e o YouTube comprime melhor a partir de 2K.
3. **Export** (mux, loudnorm em duas passadas para -14 LUFS / -1 dBTP, AAC 48 kHz, faststart):
   ```
   $VF export out/video_<formato>.mp4 -f <formato> -o out/final_<formato>.mp4 --audio public/audio/mix.wav
   ```
   Se o vídeo já está no tamanho certo em h264 yuv420p, o vídeo é copiado sem recodificar. `--fit pad|crop|blur` controla o reenquadramento quando o aspecto difere. `--fast` usa o encoder de hardware (bom para prévias).
4. **QA (estrito por padrão)**:
   ```
   $VF qa out/final_<formato>.mp4 -f <formato> -o out/qa_<formato> --voice public/voice/voice.wav
   ```
   Rode dentro da pasta do projeto: ele lê `factory.yaml > qa_excecoes`. No modo estrito, **todo WARN vira FAIL** e o comando sai com código 1. A entrega só acontece com código 0. `--lenient` existe só para rascunho e nunca vale para entrega.
   Leia o `qa.md` e abra 2-3 sheets (`out/qa_<formato>/sheets/qa_*.jpg`) nos trechos de gancho, virada e final. Crossfades e cortes detectados já são ignorados; para transições planejadas extras, passe `--plan plan.json` (`{"flash":[a,b],"xfade":[[a,b]],"planned":[[a,b]]}`).
5. **Corrija e repita** até sair com código 0. Exceção só com um motivo concreto, que o usuário pediu ou que é inerente ao formato, registrada em `factory.yaml`:
   ```yaml
   qa_excecoes:
     - { check: "planos longos", motivo: "plano-sequência de 7 s da revelação, pedido pelo usuário" }
   ```
   O nome em `check` é o da tabela do QA. Exceção sem motivo é ignorada. "Ficou bom assim" não é motivo.
6. Registre em `entregas` e marque `edicao`, `qa` e `entrega` como `feito`.
7. **Subir na plataforma** (só se `factory.yaml > plataformas` tiver a plataforma, `vf auth status` mostrar autorizado e o usuário der o OK para subir): `$VF publish out/final_<formato>.mp4 platform=yt meta=out/publicacao_<formato>.json` (rascunho = YouTube privado) ou `platform=tiktok` (caixa de entrada do app). Vale para longo e Short. **Postar** (`mode=publico|nao-listado|privado`) ou **agendar** (`at="AAAA-MM-DD HH:MM"`; YouTube publishAt nativo, TikTok pela `vf agenda`) só quando o usuário pedir explicitamente, para aquele vídeo, conta e horário; registre em `publicacao.modo`/`agendado`. A conta vem de `factory.yaml > contas` (ou `account=`); confira com `vf auth status` antes de subir, para não publicar no canal errado. Credenciais, modos, agenda e limites: [plataformas.md](references/plataformas.md). Anote em `publicacao.enviados`. Análise posterior e correção de metadata: skill `metricas`.

## Legendas (shorts verticais)
Medido em 3 shorts de 19-35M de views: **uma palavra por vez**, branca, bold, contorno preto, presa à fala, na
faixa de **0,69-0,72 da altura** (dentro da safe zone de `yt-short`/`tiktok`). No `edl.json`:
`"captions": {"enabled": true, "words_per_page": 1, "style": "pop"}`. A faixa de 0,66-0,76 é proibida para rosto e
ação principal (QA visual etapa 3). Vídeo longo: 2-3 palavras (`karaoke`) no terço inferior.

## Como ler o QA
| Item | FAIL quando | Correção típica |
|---|---|---|
Limites por formato no bloco `qa:` do `formats.yaml`.

| Item | FAIL quando | Correção típica |
|---|---|---|
| duração | fora da faixa da plataforma | cortar beats; Shorts > 180 s não vira Short |
| duração × alvo | fora de `qa.duration_strict` (TikTok 21-60 s, Shorts 30-75 s, longo 8-20 min) | roteiro do tamanho certo: o TikTok não é o Short de 110 s reaproveitado |
| bitrate | acima de `qa.max_bitrate_mbps` | `vf export` já reencoda sozinho acima do teto; `--reencode` força |
| resolução | aspecto errado | composição errada no render, ou `--fit` no export |
| áudio | sem trilha | o render sai mudo: faça o export com `--audio` |
| loudness / true peak | fora de -14 ±0,5 LUFS ou acima de -1 dBTP | rodar o export sem `--no-loudnorm` |
| clipping | amostra ≥ -0,1 dBFS | baixar SFX/música no EDL e refazer o mix |
| abertura do áudio | som começa depois de `qa.audio_start_s` | voz ou SFX no frame 0, sem respiro antes do gancho |
| ar morto | silêncio total > `qa.max_silence_s` no meio | música ou ambiente cobrindo pausas, ou cortar a pausa |
| buracos na voz (`--voice`) | pausa na narração > `qa.max_voice_gap_s` | cortar a pausa na voz (o ElevenLabs às vezes respira demais) |
| frames pretos | > 2 frames seguidos fora de transição | clipe com `end` antes do próximo `start` (buraco no EDL) |
| congelados | hold ≥ `qa.freeze_frames` sem mudar nada em 0,5 s | câmera/ação na cena, `motion` no still |
| planos sem vida | movimento acumulado médio < `qa.min_life` | mais câmera, mais ação; no motor estático, zoom mais rápido |
| planos longos | plano > `qa.max_shot_s` | dividir o plano em dois enquadramentos |
| gancho visual | nem corte nem movimento forte nos primeiros `qa.hook_s` | primeiro plano curto ou com ação imediata |
| imagem repetida (longo) | a mesma imagem volta antes de `qa.repeat_window_s` | imagem nova, ou outro recorte da mesma |
| barras pretas | faixa preta uniforme na borda | render no tamanho do formato, ou `--fit crop/blur` |
| pops de 1 frame | frame isolado diferente dos vizinhos | clipe de 1 frame no EDL, ou erro de arredondamento entre `end` e `start` |

## Outros templates
- `templates/recorte.md`: vídeo longo → shorts (seleção por transcrição, reenquadramento, legendas).
- `templates/publicacao.md`: título, descrição com capítulos e atribuições, tags, thumbnail e checklist por plataforma. Gere também `out/publicacao_<formato>.json` (`{title, description, tags}`) para o `vf publish meta=`.

## Cinco etapas de revisão visual

Para animação com elenco/props ou composição 3D, execute o [QA visual](references/qa-cinco-etapas.md)
antes da aprovação de entrega: modelagem, animação/contato, posicionamento (e identidade estável após
correções, 3b), integridade dos assets, câmera e luz/FX (6). Gere evidência de modelos e trechos em movimento; confira no celular.
Consolide os estados por etapa junto ao QA técnico. Aprovação técnica e visual são distintas;
uma não substitui a outra. Reabra apenas a etapa afetada e suas dependências após corrigir.
