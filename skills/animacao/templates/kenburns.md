# Ken Burns e parallax — guia de movimento

| Intenção | motion |
|---|---|
| Revelar detalhe / tensão | push-in `1.00 → 1.10`, `linear` |
| Percorrer paisagem/documento | pan `x: 0.03 → -0.03` com `scale 1.08` fixo, `linear` |
| Conclusão / respiro | pull-out `1.10 → 1.00`, `out` |
| Impacto (palavra forte) | push rápido `1.0 → 1.15` em 0,5 s e segura com push leve (dois clipes da mesma imagem) |
| Foto vertical em formato 16:9 | `fit: contain` em cima de uma cópia `cover` desfocada (dois clipes empilhados) |

- **Velocidade mínima**: ~0,1% de zoom ou ~0,6 px de pan por frame (a 1080). Use `linear` em stills: com `inOut`, o começo e o fim do plano quase param e o `vf qa` acusa congelamento. Guarde `inOut` para clipes curtos (< 1,5 s) e movimentos de câmera de vídeo.
- Pan com amplitude `x: ±a` precisa de `scale ≥ 1 + 2a`, senão aparece a borda da imagem.
- Alterne a direção entre planos seguidos (o `beats2edl` faz isso).
- **Parallax 2.5D**: recorte 2-4 camadas em PNG (céu, meio e frente) e use `layers` com `depth` 0, 0.5 e 1. As camadas da frente se movem e crescem mais. Preencha o buraco atrás do recorte na camada de fundo.
- Vídeo longo: 70-80% dos planos podem ser stills com Ken Burns. Intercale clipes de vídeo nos momentos de ação e um card de dado a cada 1-2 min.
