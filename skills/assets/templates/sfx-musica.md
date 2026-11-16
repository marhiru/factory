# SFX e trilha

Toda faixa entra em `data/edl.json > audio` com `role` (`voice` | `music` | `sfx`). `vf mix` faz a mixagem:
```
$VF mix data/edl.json -o public/audio/mix.wav --duck 10
```
Padrão de mix da fábrica: voz inalterada (0 dB), música ~10 LU abaixo da voz com ducking, SFX -6 dB. O master final (-14 LUFS, -1 dBTP) fica a cargo do `vf export`.

## Fontes, em ordem de preferência
1. **Procedural local** (custo zero, sem licença, sincronia perfeita): sintetize em Python/numpy (padrão do `audio/sound.py` do projeto republica: tabela de eventos → síntese → stems). Bom para whoosh, pop, tick, impacto grave, riser, sino e drones.
2. **ElevenLabs Sound Effects** (`POST /v1/sound-generation` com `text` e `duration_seconds`): para efeitos realistas específicos ("multidão em praça de pedra, 1889"). Gere 2-3 variações e escolha.
3. **CC0** (Freesound com filtro CC0, Pixabay Sound, Kenney Audio). Registre URL e licença no manifest.
4. **Música**: trilha original procedural ou biblioteca royalty-free com licença guardada (YouTube Audio Library para YouTube). Nunca música comercial.

## Onde pôr SFX
- Toda reação visual cai **0-3 frames antes** da palavra (use `start` dos beats e de `data/words.json`).
- Corte seco forte → impacto curto. Texto que aparece → pop ou tick. Mudança de lugar → whoosh de 0,3-0,5 s que termina no corte.
- No máximo 1 SFX a cada ~1,5 s nos shorts: SFX demais vira ruído.
- Música: entra no frame 0, muda de intensidade nas viradas (`tipo: virada`/`reganho`), termina com fade de 1-2 s na última palavra.

## Exemplo de `audio` no edl.json
```json
"audio": [
  {"src": "voice/voice.wav", "role": "voice"},
  {"src": "audio/music.wav", "role": "music", "gain_db": -2, "loop": true, "fade_out": 1.5},
  {"src": "audio/sfx/whoosh-01.wav", "role": "sfx", "start": 6.62, "gain_db": -6},
  {"src": "audio/sfx/pop-01.wav", "role": "sfx", "start": 7.31, "gain_db": -8}
]
```
