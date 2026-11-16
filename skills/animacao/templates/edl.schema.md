# data/edl.json — especificação

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
