# Medir o look cartoon: 2D em 3D

Derivado do estudo em [claude-animated-shorts](https://github.com/mromidev/claude-animated-shorts). Origem fixada em `skills/animacao/templates/remotion-three/ORIGEM.md`. Os números abaixo são alvos iniciais de um estilo estudado, não medições de todo vídeo cartoon.

## Protocolo de análise

Além de cortes/WPM, analise movimento, profundidade, texturas, personagens, luz, contorno e grade. Salve números, método, resolução/FPS, incerteza e frames usados em `ref/<slug>/reports/<tema>.md`. O relatório alimenta a style bible e os critérios por plano. Mascare legendas gravadas antes de medir cor/ink ou estimar movimento.

Uma folha por plano não mede câmera nem atuação. Extraia trechos densos e acompanhe fundo e personagem separadamente. Corrija o movimento da câmera antes de avaliar bob, passada, piscada e pé deslizando. Um pico de histograma pode ser flash, não corte. Movimento subpixel não pode ser tratado como ausência de movimento: o estudo inicial dizia “ease-out bursts/80% holds”; a análise posterior com ajuste subpixel corrigiu isso para glides simétricos e câmera quase sempre ativa. Prefira a medição corrigida.

| Dimensão | Ponto de partida medido / o que comparar |
|---|---|
| Formato/fala | vertical 30 fps, mediana ~60 s, 3,5–4,1 palavras/s; voz já no início |
| Movimento | glide ~23 frames, ~10 de ataque e 10 de saída; pico típico 14–16 px/frame a 720p, teto de trabalho ~25 |
| Holds/impactos | push ~0,14%/frame na mediana quieta; kick alterna sinal e acaba em 6–8 frames; sem float de câmera de mão |
| Desenho | cabeça não circular, adulto ~3,5 cabeças, contorno ameixa próprio, membros simples |
| Atuação | a cada frame durante ação; bob 3–7%, lean 1–7°, passo 9–12 frames, pé apoiado estável |
| Piscada | quatro frames, intervalo 0,5–2,3 s, cobre expressão |
| Ink do mundo | ~3 px a 1080 de largura, silhueta/depth/normal; não horizonte nem contorno duplo do card |
| Cor dia | luma mediana ~0,57, p5 sem ink ~0,23, chroma C* p90 iluminado ~55; manter pretos coloridos fora da tinta |
| Golden | horizonte pêssego, sombra violeta-azul, contraluz; não monocromia laranja |
| Profundidade | DOF em CU/MS/inserts em ~1/3 do tempo, 10–16 px a 1080, abertos nítidos, fog cor do horizonte |
| Textura | juntas, volumes e pinceladas visíveis em escala útil, sem ruído cobrindo o material |

Fórmula narrativa e som: gancho paradoxal, contexto breve, escalada, mistério, mecanismo com analogia e payoff irônico; música e SFX sob a voz. Legenda observada na referência não implica legenda obrigatória no projeto.

Compare cortes equivalentes (dia/golden, CU/aberto) e normalize resolução/recorte antes de concluir. Separe diferenças de albedo, luz e pós: uma curva global não conserta sombra sem cor ou card emissivo. Para convergir, siga `skills/edicao/references/cartoon-qa.md`. Frames e vídeos de referência são material de estudo local, nunca entrega nem ingrediente do vídeo.
