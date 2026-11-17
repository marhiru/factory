# QA visual em cinco etapas

Use para vídeos animados com personagens, props ou cenas 3D compostas. A revisão técnica
de codec, áudio, freezes e frames pretos continua necessária, mas não aprova modelagem,
contato físico, significado do efeito ou composição. Um asset pode se mover e continuar errado.

## 1. Modelagem

Confira o asset isolado sem DOF, glow ou motion blur que escondam a silhueta. Em 3D,
observe frente e vistas laterais úteis ao enquadramento; em 2D, examine os estados usados.
Depois confira o mesmo modelo sob a luz e no tamanho em que será exibido.

- Proporção, volume e partes funcionais devem comunicar o objeto. Uma ampulheta tem
  dois bojos e um gargalo central, não uma única elipse com cones sobrepostos.
- Transparência deve permitir ver o interior sem revelar faces indevidas ou esconder
  areia/ponteiros. Tampas, colunas, placas e bases não podem cortar o conteúdo principal.
- O detalhe depende da escala: um planeta em close pede costas, relevo, biomas e nuvens
  legíveis. Declare quando o relevo é ilustrativo; não apresente ruído como dado medido.
- Não aprove pela densidade da malha. Registre o defeito visual e compare a silhueta corrigida.

## 2. Animação e contato

Revise um clipe em movimento normal e uma sequência densa na ação crítica, incluindo
entrada, extremo e recuperação. Um still central não prova atuação nem colisão.

- Defina contatos nomeados: pés/chão, quadril/assento, gato/colo, mão/prop. Para rigs
  simples, limites/âncoras estáticos são suficientes se impedirem penetração em toda a ação;
  não é obrigatório instalar um motor de física. Não chame ancoragem de simulação dinâmica.
- Sentar exige pose sentada e quadril no assento: elevar um sprite em pé não resolve.
  Confira membros e coadjuvantes, não só o rosto. Billboards não podem girar para dentro do móvel.
- Cards que interagem precisam de uma ordem de profundidade coerente ao terminar o movimento.
  Gato/colo, mão/prop ou personagem/móvel não devem terminar coplanares: isso pode fazer partes
  sumirem ou virar faixas entre frames. Confira também depois do contato; use separação física
  ou composição explícita, sem desligar o depth test para esconder o problema.
- Efeitos devem permanecer no referencial do objeto e preservar fase entre cortes.
  Use uma máscara geográfica e um estado contínuo para oceano/terra; duas cascas ou máscaras
  independentes podem produzir costuras, degraus e faixas que nada explicam.
- Valide o significado: oceanos migrando, linhas de campo magnético e nuvens são fenômenos
  diferentes. Não acrescente grade, setas ou partículas que confundam essa leitura.
- Ruído, piscada e deslocamento de câmera não compensam uma ação incoerente.

## 3. Posicionamento e coesão dos assets

Observe a cena com o elenco e a câmera finais, não apenas o asset isolado.

- Cada prop deve ter suporte ou uma convenção visual deliberada: poste, parede, mesa,
  mão ou inserto explicativo. Instrumentos não aparecem flutuando por falta de layout.
- Objetos relacionados compartilham suporte, escala e orientação. Mostradores e placas
  voltam a face para o público sem perder a lógica de fixação no cenário.
- Confira silhueta, oclusão e tangências: fios não cruzam dados, vegetação não esconde
  informação e um segundo prop não parece uma peça solta do primeiro.
- Um prop posto em foreground deve ser reconhecível, não um recorte sem função narrativa.
- Reprova sem exceção: asset mal posicionado (atravessando móvel, parede ou chão, flutuando,
  fora de escala/perspectiva), asset que quebra a harmonia do cenário (orientação, luz, paleta
  ou estilo destoando do conjunto) e asset que atrapalha a visibilidade da cena (cobrindo o
  assunto, o rosto, a ação principal, legenda ou safe zone). Reposicione dentro da lógica do
  cenário construído; não esconda com DOF, escala mínima ou corte de câmera.

## 3b. Identidade estável dos assets ao corrigir

Uma correção pedida (posição, pose, contato, escala) altera **só** o atributo pedido do
**mesmo** asset. O personagem que estava dentro do sofá continua sendo o mesmo personagem,
apenas reposicionado no assento.

- Corrija pelo layout: `x`/`y`/`scale`/`z`/âncora/pose do pack existente (mesmo arquivo,
  mesmo id em `assets/manifest.yaml`). Não regenere nem troque por outro asset "parecido".
- Se a correção exigir desenho novo (pose sentada inexistente), gere-o a partir da mesma
  referência fixa do elenco (style bible, seed/referência do Flow, mesmo pack) e registre-o
  como novo estado do mesmo personagem, não como personagem novo.
- Antes de aceitar, compare lado a lado antes × depois: rosto, cabelo, roupa, cores, proporção
  e acessórios idênticos. Qualquer divergência de identidade reprova a correção.
- Nada além do pedido muda na cena: demais atores, props, câmera e luz ficam como estavam,
  salvo dependência real (ver "Evidência e encerramento").

## 4. Integridade e legibilidade dos assets

Inspecione no enquadramento de entrega e em tamanho de celular, com o pós ativo.

- Ponteiros, agulhas, números, rótulos, expressões e partes necessárias devem existir e
  continuar legíveis. Um relógio com mostrador sem ponteiros reprova mesmo sem shader error.
- Não use z quase coplanar para detalhes finos sem checar z-fighting. Separe profundidades,
  dê espessura/contraste ou desenhe partes móveis no mesmo dial quando isso for mais confiável.
- Teste transparência, frente/verso, luz e redução de escala. Materiais unlit podem servir
  à face de instrumentos; não desligue o depth test para esconder oclusões reais.
- Confira as partes móveis em mais de uma posição, evitando aprovar ponteiros sobrepostos.

## 5. Câmera

Confira início, meio, fim e extremos da ação; para glides/pans, use o clipe inteiro.

- Escolha o assunto e a área que precisa permanecer visível. Reserve margem para cabeça,
  membros, props e reação antes de aplicar push-in, roll ou kick. Não imponha corpo inteiro
  a um close deliberado; registre a intenção quando o recorte for parte da narrativa.
- Um plano médio não vira close cortado no fim do push. Se necessário recue, altere FOV,
  reposicione o assunto ou reduza o movimento, conferindo novamente oclusões e perspectiva.
- Em 3D, teste a câmera contra paredes, grades e props ao longo do caminho. Uma câmera
  que evita um corrimão no meio do plano pode atravessá-lo no início ou no fim.
- Trechos contínuos com contexto semelhante (mesmo cenário/assunto, ideia que continua)
  não ganham corte seco nem cenário novo: a câmera faz zoom-out do enquadramento atual e
  zoom-in no novo ponto de interesse do mesmo cenário, num movimento único e suave.
  Corte só quando o contexto muda de fato (lugar, tempo ou assunto).
- Verifique safe zones do formato. Uma versão 16:9 ou uma folha de miniaturas não prova
  enquadramento vertical nem legibilidade sob a interface da plataforma.

## Evidência e encerramento

Registre por etapa em `out/qa-visual/qa.md` ou JSON equivalente: estado
`aprovado | reprovado | pendente | nao_aplicavel`, critérios conferidos, defeitos com
plano/frame, caminhos de evidência e revisão do código/render. Justifique `nao_aplicavel`.
Evidência de animação inclui vídeo; modelagem 3D inclui vistas úteis; câmera inclui extremos.

Corrija e reveja a etapa afetada e suas dependências: mudou modelo/pose → reveja contatos,
oclusão e câmera; mudou câmera → reveja contatos aparentes, leitura e margens. Não repita
checks independentes que continuam válidos. Uma aprovação fica pendente se o render mudou
depois da evidência e a área afetada não foi revista.

Antes da entrega, todas as etapas aplicáveis precisam estar aprovadas e o `vf qa` estrito
também precisa passar. Prévia sem voz pode servir como evidência visual, com áudio/exportação
pendentes e essa limitação explícita. Nunca troque FAIL de qualidade por uma exceção técnica,
nem use `--planned` para apagar defeitos visuais ou intervalos inteiros de ação.
