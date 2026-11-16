# Style bible do tema (vídeo longo estático)

Todas as imagens do vídeo parecem do mesmo filme. Preencha antes do primeiro lote. O resultado vai para `assets/flow/estilo.json` (`sufixo`, `negativos`, `referencias`, `cenarios`).

## Identidade
- **Tema / época / lugar:**
- **Sensação em 3 palavras:** (ex.: "misterioso, úmido, solene")
- **Referências visuais** (3-6 imagens geradas no 1º lote de teste e aprovadas, em `assets/ref/estilo-NN.jpg`): elas entram como `reference_images` em TODOS os lotes.

## Look
| Item | Escolha | Exemplo |
|---|---|---|
| Linguagem | pintura realista, fotografia de época, gravura, 3D estilizado | "painterly realism, oil on canvas texture" |
| Paleta | 2 cores dominantes + 1 de acento | "deep teal shadows, warm amber highlights, crimson accents" |
| Luz | direção, dureza, hora | "low warm side light, volumetric haze" |
| Lente | focal, profundidade | "35mm, shallow depth of field" |
| Textura | grão, pincel, papel | "subtle film grain" |
| Composição | regra dos terços, **folga em volta do sujeito** (o zoom-in come ~10% das bordas) | "rule of thirds, subject not touching the frame edges" |

## Regras de capricho
- **Nada de texto na imagem.** Texto, se houver, entra em HTML (cards) e fica nítido.
- Mãos e rostos só quando necessários. Prefira silhuetas, costas, objetos e detalhes.
- Cada cenário tem pelo menos 3 enquadramentos (aberto, médio, detalhe) para o corte respirar sem repetir.
- Uma graduação única para o vídeo inteiro (`overlays > grade` no edl.json) amarra imagens de lotes diferentes.
- Resolução: o zoom precisa de folga. Abaixo de 2150 px de largura a imagem é marcada `UP` pelo `vf sheet` e passa por upscale antes de entrar.

## Sufixo de estilo (cole em `estilo.json > sufixo`)
```
<linguagem>, <luz>, <paleta>, <lente>, <textura>, <composição com folga para zoom>, no text.
```
## Negativos
```
text, letters, captions, watermark, logo, signature, extra fingers, deformed hands, distorted faces, anachronistic objects, frame borders
```
