# Desenho e atuação cartoon 2D

Aplicável ao cutout sobre plates e aos cards num mundo 3D. Implementação reutilizável: `../templates/remotion-three/src/videos/cartoon/toon.ts` (Canvas2D sem dependência de three.js). Licença e origem: `../templates/remotion-three/ORIGEM.md`.

## Construção do elenco

- Espaço de desenho 600×1000; solas em y=975, âncora `[0.5, 0.975]`. Preserve espaço para chapéus, gestos e FX sem cortar o alpha.
- Cabeça de feijão/superelipse, coroa mais pontuda, queixo estreito, assimetria determinística por personagem. Razão altura/largura perto de 0,93–0,94; adulto com cerca de 3,5 cabeças, pernas com 1,4 cabeça. Essas proporções são ponto de partida para uma identidade própria.
- Membros simples com espessura próxima de 0,055 da largura da cabeça; mãos em pontos de tinta. Evite cabeças perfeitamente circulares, membros de espessura variável por acidente e blush/brilhos de olhos quando estiver seguindo este look.
- Contorno ameixa `#241726` (faixa `#201628`–`#2f1d28`), mais espesso no lado inferior/sombra. Não aplique o contorno do mundo sobre esse desenho novamente.
- `Pen` oferece polígonos, elipses, linhas e strokes; `Outfit`, `bodyFront`, `bodySide` e a biblioteca de faces separam roupa, pose e expressão. Há 14 expressões no exemplo; escolha as necessárias ao roteiro.
- Sombra pintada suave: pele clara, roupa com sombra mais fria. Evite preencher roupas inteiras com preto absoluto; use marinho, berinjela ou vinho escuros. A iluminação 3D complementa a pintura; não substitua toda a luz por emissão.

## Atuação e ciclos

- Anime em função do frame, com sementes fixas; render fora de ordem deve produzir o mesmo resultado. Movimento em cada frame quando a ação estiver acontecendo; sem quantizar passeio em poses longas por acidente.
- Troque expressão/gesto 0–3 frames antes da palavra. Piscada de quatro frames: meio fechado → fechado → fechado → meio fechado; intervalo variável de 0,5–2,3 s. Piscadas escondem trocas de expressão.
- Caminhada: passo de 9–12 frames a 30 fps como referência, bob de 3–7% da altura, lean de 1–7°. A passada do exemplo é 0,8 m; compatibilize velocidade de deslocamento, comprimento das pernas e frequência do ciclo para plantar o pé de apoio.
- Não deixe figurantes acenando ou pulando continuamente sem motivo. No perfil cartoon medido, prefira atuação pontual e câmera em movimento a um `breathe` automático em todos. O idle padrão do cutout continua disponível para outros estilos.
- Multidão usa variação de roupa, rosto, fase e reação com seed fixa, mantendo a formação legível. FX (notas, burst, estrelas, splash, fumaça, poeira) também são desenhos em cards/sprites.

## Exportação para o cutout da fábrica

Importe funções de `toon.ts` no registro de sprites, adaptando a chamada ao `draw(g)` do tipo `Pack`; use `templates/sprites/packs.ts` para expressões, piscadas e ciclos. Exporte PNGs com `bun sprites/export-sprites.mjs sprites/index.ts public/sprites`. Preserve a âncora dos pés no `sprite.json`. Remova dependências do mundo/câmera do desenho exportável; poses devem receber parâmetros explícitos.

Confira uma folha com todas as expressões, extremos do ciclo e props; depois examine uma sequência densa para detectar pés deslizando, recortes e trocas fora da palavra. Para cards no mundo 3D, continue em [2D em mundo 3D](2d-in-3d-world.md).
