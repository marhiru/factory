# Glossário de FX, câmera e montagem (com a tradução para a fábrica)

Vocabulário comum para pedir, escrever e checar efeitos. Use os termos em inglês nos `visual` e `camera` do `beats.json` e nos prompts: são os nomes da indústria. A coluna "Na fábrica" diz como fazer com o que já existe (cutout `scene` no mygo/Remotion, `card`/overlays no HyperFrames, ou o mundo 3D).

## 1. Princípios de animação (Disney/Pixar)
| Termo | O que é | Na fábrica |
|---|---|---|
| **Anticipation** | pequeno movimento contrário antes da ação (agacha antes de pular) | key de `scale` 1 → 0,95 em 0,1 s antes da key principal |
| **Squash & stretch** | achatar no impacto, esticar na velocidade | keys `scale` no contato (sprite com `scaleX` ≠ `scaleY` exige estado próprio) |
| **Hang time / hold no ar** | o objeto para no topo do arco antes de cair | keys `rot`/`y` sobem em 0,15 s (`out`), seguram 0,5-0,8 s, caem em 0,2 s (`in`) |
| **Pose-to-pose / snap to pose** | o personagem "estala" na pose nova e congela | troca de `state` na palavra + `idle: breathe` (nada de interpolar o corpo) |
| **Moving hold** | pose congelada que ainda respira | `idle: breathe`/`bob`; a câmera continua andando |
| **Overlapping action / follow-through** | partes que continuam se mexendo depois do corpo parar (chapéu, cabelo) | prop separado com keys atrasadas 0,06-0,12 s |
| **Secondary action** | ação menor que reforça a principal (sacode a cabeça enquanto anda) | segundo ator ou `idle: jitter` em um elemento |
| **Ease in / ease out (slow in/out)** | acelera e desacelera | `ease: in | out | inOut` nas keys |
| **Staging** | uma ideia por plano, leitura imediata | 1 ação principal por cena; o resto em `idle` |
| **Silhouette** | a pose se entende só pela silhueta | conferir no still com o personagem preenchido de preto |
| **Smear frame** | 1-2 frames esticados que fazem o movimento rápido "borrar" | estado de 1 frame `smear` entre duas poses |
| **Impact frame** | 1-3 frames de alto contraste (branco/preto, invertido) no golpe | `transition_in: flash` de 0,06-0,1 s ou estado `impact` |
| **Reaction shot / take** | personagem reage (o "double take" olha duas vezes) | keys de `state` neutro → choque com 0,25 s entre eles |

## 2. Câmera
| Termo | O que é | Na fábrica |
|---|---|---|
| **Push-in / dolly-in** | câmera se aproxima | `camera` `scale` 1,0 → 1,1 |
| **Pull-back reveal** | câmera se afasta e revela o contexto (a piada aparece no fim) | `camera` `scale` 1,4 → 1,0 com `ease: out` |
| **Crash zoom / snap zoom** | zoom muito rápido para um detalhe (comédia) | clipe de 0,2 s, `scale` 1 → 2,5, `ease: in` |
| **Dolly zoom (vertigo)** | fundo estica enquanto o sujeito fica do mesmo tamanho | parallax: plate `scale` cai, ator `scale` sobe (camadas `depth`) |
| **Truck / track / travelling** | câmera anda de lado acompanhando | `camera` `x` -0,05 → 0,05 com `scale ≥ 1 + 2|x|` |
| **Crane / jib / boom** | sobe ou desce | `camera` `y` |
| **Orbit / arc** | gira em volta do sujeito | só no mundo 3D (Blender/three.js) |
| **Whip pan / swish pan** | giro rápido com motion blur que vira transição | `transition_in: slide` 0,15 s + whoosh |
| **Camera shake / trauma** | tremor no impacto, decai rápido | `idle: jitter` no plate por 0,3 s (falta key de shake de câmera: pedir no renderer) |
| **Rack focus** | foco muda do primeiro plano para o fundo | só no 3D (DOF) ou com blur no plate por key |
| **POV** | o que o personagem vê | plate em primeira pessoa (painel, volante) |
| **OTS (over the shoulder)** | por cima do ombro | ator de costas recortado com `clip_y` |
| **Establishing shot** | plano aberto que situa o lugar | primeiro plano de cada cenário |
| **Insert / cutaway** | detalhe (a placa, o contador) | cena curta só com o prop |
| **Dutch angle** | câmera inclinada (tensão) | `camera` `rot` 4-8° |
| **Low angle / high angle** | de baixo (poder) / de cima (fraqueza) | escolha do plate/enquadramento |

## 3. Montagem e transições
| Termo | O que é | Na fábrica |
|---|---|---|
| **Cut on action** | corta no meio do movimento | corte na key principal da cena |
| **Match cut** | forma/movimento igual liga dois planos | mesmo `x,y` e tamanho do objeto nos dois clipes |
| **Zoom-through / portal transition** | entra num objeto (olho, janela, buraco) e sai na cena seguinte | clipe de 0,3 s com `scale` 1,2 → 6 no ponto + `fade` 0,1 s no próximo |
| **Smash cut** | corte brusco de calma para caos (ou o contrário) | corte seco + SFX alto no frame |
| **Dissolve / cross-fade** | fusão (a costura padrão no estilo estudado: 6-10 frames, com a câmera andando no mesmo sentido) | `transition_in: fade` 0,2-0,3 s |
| **Dip to black/white** | fusão passando por preto/branco | `dip` / `flash` |
| **J-cut / L-cut** | o som do próximo plano começa antes / o som do anterior continua depois | `sfx` com `start` 0,2-0,3 s antes do corte |
| **Jump cut** | corte no mesmo enquadramento (pula o tempo) | dois clipes do mesmo plate com atores em posições diferentes |
| **Hard cut** | corte seco | sem `transition_in` |
| **Pattern interrupt** | quebra de padrão visual para segurar a atenção | card, mudança de clima, crash zoom |
| **Montage** | sequência rápida que comprime tempo | 4-6 clipes de 0,5-0,8 s |

## 4. Look, luz e render
| Termo | O que é | Na fábrica |
|---|---|---|
| **Billboard / card** | plano 2D sempre virado para a câmera dentro do 3D | personagens cutout no mundo 3D (`mundo-3d-blender.md`) |
| **2.5D** | camadas 2D em profundidade com parallax | `layers` com `depth` |
| **Cel shading / toon shading** | sombra em degraus, sem gradiente | personagens; o mundo fica PBR no estilo estudado |
| **Outline / Freestyle / inverted hull** | contorno preto | traço do sprite; no Blender, Freestyle ou casco invertido |
| **PBR** | material fisicamente correto (rugosidade, metal) | mundo 3D |
| **HDRI** | imagem 360° que ilumina a cena | Poly Haven (CC0) |
| **Key / fill / rim light** | luz principal / preenchimento / contraluz que separa do fundo | rim light nos personagens = legibilidade no celular |
| **Golden hour** | sol baixo, quente, sombras longas | a luz padrão do estilo estudado |
| **Day-for-night / mood shift** | troca de clima para mudar o tom | noite + chuva na parte da explicação |
| **Aerial perspective / atmospheric haze** | fundo mais claro e menos saturado | neblina/volumétrico; no 2D, overlay de gradiente |
| **Volumetrics / god rays** | luz visível no ar | volumétrico (3D) ou overlay `leak` |
| **Bloom / glow** | luzes fortes "vazam" | 3D ou overlay |
| **AO (ambient occlusion)** | escurecer cantos e contatos | sombra de contato dos atores (`shadow: true`) |
| **DOF (depth of field)** | fundo desfocado | 3D, ou plate com blur |
| **Motion blur** | borrão de movimento | 3D/mygo; nos impactos é o que vende a velocidade |
| **Color script** | paleta planejada por trecho do vídeo | uma linha por beat no `style-bible` |
| **Grade / LUT** | correção de cor final | overlay `grade` |
| **Chromatic aberration** | franja de cor nas bordas | só em impacto, 1-3 frames |

## 5. FX (efeitos visuais)
| Termo | O que é | Na fábrica |
|---|---|---|
| **Particle emitter** | gerador de partículas (poeira, faíscas, folhas, chuva) | sprite `fx` com ciclo de 8 frames; no 3D, sistema de partículas |
| **Debris** | pedaços voando | vários `fx` pequenos com keys de arco |
| **Cell fracture / Voronoi fracture** | objeto quebra em pedaços | 3D (Blender Cell Fracture) |
| **Shockwave / ring** | anel que expande no impacto | `fx` anel escalando + flash + shake |
| **Dust puff / impact dust** | nuvem de poeira no contato | `fx` `burst` marrom |
| **Speed lines / anime lines** | linhas de velocidade | overlay de 1 ciclo |
| **Glint / sparkle** | brilho pontual que "acende" o mundo | `fx` `sparkle` em props importantes |
| **Heat haze / distortion** | ar tremendo | só no 3D/shader |
| **Lens flare** | reflexo da lente | 3D ou overlay leve |
| **Rain / snow overlay** | camada de chuva | `fx` em ciclo cobrindo o quadro |
| **Diegetic text** | texto que existe no mundo (placa, letreiro, contador) | prop com o texto pintado; preferir a cartela |

## 6. Som (o FX que não se vê)
| Termo | O que é | Uso |
|---|---|---|
| **Whoosh / swish** | passagem de ar | whip pan, objetos rápidos, transições |
| **Riser** | som que sobe antes de uma revelação | 1-2 s antes da virada |
| **Impact / hit / boom** | pancada | no frame do contato |
| **Sub drop** | grave que despenca | revelação, clímax |
| **Stinger** | acorde curto de pontuação | punchline |
| **Foley** | sons do dia a dia (passos, porta, motor) | sob toda ação na tela |
| **Room tone / ambience** | o "ar" do lugar | sob a cena toda (evita ar morto) |
| **Ducking / sidechain** | música abaixa quando a voz entra | `vf mix` |
| **Mickey-mousing** | som/música que imita cada movimento | comédia; usar com moderação |
