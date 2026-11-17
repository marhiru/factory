# Assets, voz e som para cartoon 2D/3D

Leia na produção de elenco, texturas ou áudio procedural. Desenhos e material visual: `skills/animacao/references/cartoon-2d.md` e `2d-in-3d-world.md`.

## Kit visual

Registre na style bible proporções, roupa/paleta, tinta do elenco e do mundo, luz dia/golden, sombra de contato, fog, DOF e exemplos de enquadramento. Monte folhas de expressão/ciclo, props de piada, FX e cenários sem elenco. Estilo de referência informa qualidade e gramática; personagens e identidade pertencem ao canal.

Texturas Canvas2D HD pintam juntas como tons escuros, sem linhas pretas em toda superfície. Albedo e iluminação têm responsabilidades separadas: não tente corrigir sombra marrom com grade azul global. Kits Kenney precisam de recoloração e registro CC0; os modelos não acompanham o template importado. Não copie os frames privados do estudo para assets do vídeo.

## Voz como régua

Tom estável e andamento ~3,5–4,1 palavras/s são opções do formato, não obrigação de voz/idioma. Use a configuração do canal. Transcreva/alinha antes de animar e confira manualmente palavras de disparo, nomes próprios e pausas. Corrija tempos errados em `data/words.json` sem mudar arbitrariamente o áudio. Todas as reações usam essa mesma régua.

## Áudio procedural reutilizável

Fontes MIT em `../templates/audio-procedural/`:

| Arquivo | Responsabilidade |
|---|---|
| `timeline.py` | frames, eventos, fases e tempo físico |
| `dsp.py` | WAV, envelopes, filtros, loudness e true peak |
| `sfx_gen.py`, `sfx.py` | síntese de efeitos e tabela de cues |
| `instruments.py`, `music.py` | instrumentos e partitura original por seção |
| `mix.py` | stems, automação, ducking, limiter e relatório |

Copie os `.py` e `LICENSE` para `audio/` do projeto se usar esse motor; requer Python, numpy, scipy e ffmpeg/ffprobe. O exemplo continua com tempos da ponte: refaça `timeline.py`, eventos e partitura para o vídeo novo antes de gerar som. Os caminhos adaptados leem `public/voice/voice.wav` e `data/words.json`; escrevem stems, cache e `cues.md` em `audio/`.

Trave os tempos da imagem antes de gerar a trilha. Mudança de câmera, impacto, passo ou duração exige regenerar os cues afetados e invalidar o cache correspondente em `audio/_cache/`; para física periódica, derive passos/balanço do mesmo tempo `tau` usado pela imagem. Não mantenha uma segunda timeline divergente.

Produza WAV 48 kHz, duração exata e stems separados. Voz permanece sem EQ/compressão/pitch; ajuste a cama e SFX ao redor. Os ganhos “música +3/SFX -6” e master -15 do exemplo são históricos: o padrão da fábrica continua música ~10 LU abaixo da voz, SFX conforme beat e export em -14 LUFS/-1 dBTP. Use `vf mix` com stems ou o mix procedural pronto, nunca some o mesmo áudio duas vezes. Copie o mix escolhido para `public/audio/mix.wav`; `vf export --audio` e QA validam o arquivo final.

Registre no manifest tanto as fontes sintetizadas quanto bibliotecas/modelos externos e atribuições aplicáveis. Não marque assets completos só porque o código do sintetizador foi copiado: os arquivos usados no vídeo precisam existir e ter sido ouvidos.
