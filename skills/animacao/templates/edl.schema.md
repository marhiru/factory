# data/edl.json — especificação

Tipos de clipe: **`scene`** (animação cutout: o padrão de um vídeo animado), `image` (still/Ken Burns), `video`, `text`, `color`.

```jsonc
{
  "fps": 30,
  "duration": 62.4,                 // segundos; vira durationInFrames
  "background": "#000000",
  "font": { "family": "Marca", "src": "fonts/marca.ttf" },   // opcional; o MESMO arquivo nos dois renderers
  "audio": [                        // Remotion toca para preview; `vf mix` gera o mix final
    { "src": "voice/voice.wav", "role": "voice" },
    { "src": "audio/music.wav", "role": "music", "gain_db": -2, "loop": true, "fade_out": 1.5 },
    { "src": "audio/sfx/pop.wav", "role": "sfx", "start": 7.3, "gain_db": -8 }
  ],
  "clips": [
    // image: Ken Burns; `layers` ativa parallax 2.5D
    { "id": "b1-1", "type": "image", "src": "img/b01-porto.jpg", "start": 0, "end": 2.6,
      "fit": "cover",
      "motion": { "from": { "scale": 1, "x": 0, "y": 0 }, "to": { "scale": 1.1, "x": -0.03 }, "ease": "inOut" } },
    { "id": "b2-1", "type": "image", "start": 2.6, "end": 5.4,
      "layers": [ { "src": "img/b02-ceu.png", "depth": 0 }, { "src": "img/b02-predio.png", "depth": 0.5 }, { "src": "img/b02-pessoa.png", "depth": 1 } ],
      "motion": { "from": { "scale": 1.0 }, "to": { "scale": 1.08, "x": 0.02 } } },
    // video: clipe IA/stock; trim = início dentro do arquivo (s)
    { "id": "b3-1", "type": "video", "src": "clips/b03-1.mp4", "start": 5.4, "end": 8.1, "trim": 0.5, "playbackRate": 1, "volume": 0,
      "transition_in": { "kind": "fade", "dur": 0.3 } },
    // text: title | lower | label | number (number anima o contador)
    { "id": "b3-txt", "type": "text", "style": "number", "text": "1889", "start": 5.5, "end": 7.5, "layer": 2 },
    { "id": "card", "type": "color", "color": "#101014", "start": 8.1, "end": 9 }
  ],
  "captions": { "enabled": true, "words_per_page": 3, "style": "karaoke" }   // karaoke | pop | plain
}
```

- `layer`: ordem de empilhamento (padrão 0). Texto em 2, legendas sempre por cima.
- `transition_in`: `cut` (padrão), `fade`, `flash` ou `slide`, com `dur` em segundos. No `fade`, o clipe anterior da mesma camada é estendido automaticamente para cobrir a entrada.
- `motion.x/y`: fração da largura/altura. `scale`: 1 = enquadrado. `ease`: `linear | in | out | inOut`.
- Caminhos `src` são relativos a `public/`.
- `font`: sem ela, cada renderer usa a fonte do sistema (Helvetica Neue no Chrome, SF no MyGo) e as legendas mudam de largura e quebra. **Em produção, defina sempre a fonte da marca.**
- Renderers que implementam este contrato: `remotion` (template `remotion-2d/`) e `mygo` (`bin/vf-mygo`). Um renderer novo precisa implementar só isto.

## Clipe `scene` (animação cutout 2D)
```jsonc
{ "id": "s02", "type": "scene", "start": 2.73, "end": 5.73,
  "transition_in": { "kind": "flash", "dur": 0.2 },
  "scene": {
    "bg": { "src": "img/plates/s02.jpg" },                       // plate sem personagens (ou "color")
    "camera": { "from": { "scale": 1.04 }, "to": { "scale": 1.0 }, "ease": "linear" },  // move o plate e os atores
    "actors": [
      { "id": "heroi", "sprite": "sprites/heroi",               // pasta com sprite.json
        "x": 0.5, "y": 0.86, "h": 0.52,                           // pés em (x, y) do quadro; h = altura do canvas do sprite / altura do quadro
        "state": "sick", "idle": "breathe", "shadow": true, "depth": 0.15,
        "enter": { "kind": "rise", "at": 0.05, "dur": 0.5 },      // pop | slide-left | slide-right | drop | rise | fade
        "exit":  { "kind": "pop", "at_word": 32 },
        "keys": [                                                  // t relativo ao clipe, OU at_word (2 frames antes da palavra)
          { "at_word": 14, "state": "dazed" },
          { "at_word": 17, "state": "shock" },
          { "t": 1.2, "x": 0.5 }, { "t": 1.9, "x": 0.7, "ease": "inOut" }   // x, y, scale, rot, opacity interpolam; state, flip trocam
        ],
        "clip_y": 0.69                                             // opcional: recorta abaixo dessa linha (atrás de mesa/muro)
      }
    ]
  } }
```
- **Interpolação:** cada campo (`x`, `y`, `scale`, `rot`, `opacity`) segura o valor base até a primeira key que o define. Entre duas keys, interpola com o `ease` da key de destino (padrão `inOut`). `state` e `flip` trocam na key.
- **`idle`:** `breathe` (padrão: respira escalando a partir dos pés), `bob`, `sway`, `float`, `jitter` (nervoso) ou `none`. A fase muda por ator.
- **Piscada automática:** usa o frame `blink` do estado, a cada 0,5-2,3 s por 4 frames, e também em toda troca de estado (esconde o corte). `"blink": false` desliga.
- **Câmera:** cada camada recebe a câmera com parallax `1 + depth × 0.5` (o plate tem depth 0).
- **Junção sem corte (trechos contínuos no mesmo cenário):** `"camera": {"from": {...}, "via": {"scale": 1.0}, "to": {...}, "via_at": 0.5}`
  faz from → via (zoom-out) → to (zoom-in), cada perna com o `ease` (padrão `inOut`). Helper: `junta()` no `vfscene.py`.
  Implementado no mygo (`camAt`) e no Remotion (`Scene.tsx`) com a mesma matemática. O `edl-check` reprova corte seco
  entre clipes vizinhos com o mesmo plate (use `corte_motivo` para uma exceção justificada).
- **Estado em ciclo:** com vários frames (marcha, aceno), avança a `fps` do estado em loop.

## `sprite.json` (um por pack, em `public/sprites/<nome>/`)
```json
{ "w": 1200, "h": 2000, "anchor": [0.5, 0.975],
  "states": {
    "sick":  { "frames": ["sick.png"], "blink": "sick_blink.png" },
    "march": { "frames": ["march_00.png", "march_01.png", "march_02.png", "march_03.png", "march_04.png", "march_05.png", "march_06.png", "march_07.png"], "fps": 8 }
  } }
```
- `w` e `h` são os pixels dos PNGs. `anchor` é o ponto que vai em (`x`, `y`): pés por padrão, centro em efeitos.
- O exportador `templates/sprites/export-sprites.mjs` gera tudo isso a partir de desenhos Canvas2D.
