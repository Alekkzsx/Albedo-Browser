# 🗺️ Plano Mestre de Engenharia — Albedo Browser & Engine (Edição Purista)

> **O que é este documento:** o roteiro definitivo de construção do Albedo Browser e do Albedo Engine.
> **A Filosofia:** O Albedo é uma prova de força bruta de engenharia em Rust. Diferente de projetos que orquestram bibliotecas prontas, **o Albedo proíbe o uso de dependências externas**. É uma obra-prima de software criada 100% do zero. 

---

## 0. Constituição do Projeto — A Única Regra de Ouro

**Regra 1 — Zero Dependências (O Manifesto NIH).**
O arquivo `Cargo.toml` não pode conter nenhuma biblioteca de terceiros na seção `[dependencies]`. 
- Banidos: `tokio`, `serde`, `reqwest`, `rustls`, `html5ever`, `cssparser`, `wgpu`, `skia`, `image`, `rusty_v8`, `Boa`, `winit`, etc.
- **Exceção Única:** Bibliotecas que expõem chamadas de sistema brutas do Sistema Operacional (ex: `libc` no Linux, `windows-sys` no Windows), pois o Rust não possui magia para acessar hardware sem pedir permissão ao Kernel.
- Tudo — do parsing de strings HTTP até o preenchimento de pixels de fontes na tela — deve ser escrito em código próprio.

---

## 1. Estrutura do Workspace — Arquitetura Completa

O Albedo continua precisando de **12 crates**, mas agora elas carregam o peso de implementar suas próprias primitivas tecnológicas.

| Crate | Responsabilidade (100% In-House) |
|---|---|
| `albedo_core` | Tipos fundamentais, alocadores customizados, matrizes matemáticas, Event Loop próprio via threads. |
| `albedo_ipc` | Serialização binária própria (sem serde) e comunicação multi-processo nativa (pipes/sockets do SO). |
| `albedo_net` | Wrappers de TCP puro. Parser manual de HTTP/1.1 (texto). Resolução de DNS via sockets brutos. |
| `albedo_security` | Same-Origin Policy, CORS e parser de CSP escritos do zero. |
| `albedo_dom` | **Tokenizer e Tree Builder HTML5 customizados**. Implementação completa de Arena de Nós. |
| `albedo_style` | **Lexer e Parser CSS customizados**. Algoritmo de Cascata. |
| `albedo_layout` | Motor de Geometria: Block, Inline e Flexbox box-models (matemática pura). |
| `albedo_media` | **Decodificadores binários próprios** para ler cabeçalhos e chunks de arquivos `.png` e `.bmp`. |
| `albedo_render` | **Software Rasterizer (CPU)**. Parser próprio de fontes `.ttf`. Motor de rasterização de curvas de Bézier na mão. |
| `albedo_js` | **Motor JavaScript Próprio:** Lexer, AST Parser e Máquina Virtual (VM) de Bytecode escritos do zero. |
| `albedo_storage` | Parser de banco de dados chave-valor próprio salvo em binário/texto para cookies e cache. |
| `albedo_browser` | Chamadas Win32/X11 puras para criar a janela do sistema operacional e pintar o Frame Buffer. |

---

## 2. Fases Detalhadas da Engenharia Extrema

### Fase 1 & 2 — Core Engine e Loop de Eventos Base
**🎯 Objetivo:** Esqueleto do projeto e paralelismo sem bibliotecas async.
- Sem `tokio`. Criar um Thread Pool customizado usando as threads padrões do `std`.
- Sem `serde`. Criar funções de serialização em bytes raw para o IPC.
- Event Loop de filas (Macrotask, Microtask) controlado por Mutexes padrão e Condition Variables.

### Fase 3 — Motor de Rede (TCP Puro)
**🎯 Objetivo:** Falar HTTP diretamente nos sockets.
- Mapear chamadas `std::net::TcpStream`.
- Escrever uma máquina de estados (State Machine) para ler e interpretar o texto bruto do protocolo HTTP/1.1 (ler até `\r\n\r\n`, interpretar headers).
- Criar o próprio gerador de requests HTTP do zero.
- *Nota: TLS (HTTPS) e Criptografia estão explicitamente adiados para o final do projeto devido à complexidade matemática e risco de segurança. O MVP funcionará apenas via HTTP em portas 80/locais.*

### Fase 4 — Parsing Completo (HTML e CSS)
**🎯 Objetivo:** Bytes para Árvore (Sem `html5ever`).
- A tarefa mais complexa das fases iniciais. Ler a especificação do WHATWG e implementar o "Insertion Mode" state machine caractere por caractere.
- Parser de CSS que lê strings brutas e constrói a árvore de propriedades (CSSOM).
- Lidar com falhas clássicas de sites antigos de forma manual.

### Fase 5 e 6 — Layout e Geometria
**🎯 Objetivo:** Calcular X e Y de tudo (Misteriosa arte do Flexbox).
- Já era previsto ser in-house, mas agora a matemática vetorial não usará crates externas (nada de `glam` ou `euclid`). 
- Structs de Matrix2D e Point próprias.
- Motor de layout multi-thread manual.

### Fase 7 — Renderização em Software e Rasterização
**🎯 Objetivo:** Preencher os pixels da tela (Sem GPU, sem Skia).
- Como não usaremos API de GPU (`wgpu`/Vulkan) para manter a portabilidade máxima sem dependências, o Albedo usará um **Software Renderer**.
- Teremos um array gigantesco de pixels na CPU (Frame Buffer, ex: `Vec<u32>` com ARGB).
- Algoritmos matemáticos próprios para desenhar linhas (Bresenham) e retângulos.
- **Tipografia:** Escrever um parser para ler os binários da tabela gráfica de um arquivo fonte `.ttf` (TrueType), entender as curvas de Bézier e rasterizar matematicamente no frame buffer.

### Fase 8 — A Interface do Sistema (A Janela)
**🎯 Objetivo:** Mostrar a tela sem bibliotecas de abstração.
- Sem usar crates como `winit`.
- No Windows: Usar a API do Win32 diretamente via `windows-sys` para chamar `CreateWindowExW`, capturar os ponteiros da tela via `BitBlt` e injetar nosso Array de Pixels lá dentro.
- Mapear inputs do mouse e teclado lendo as interrupções/eventos nativos do SO.

### Fase 9 — O Motor JavaScript do Albedo
**🎯 Objetivo:** Rodar JS.
- Esqueça o V8 ou o Boa. 
- Criar um Lexer (quebra o texto JS em tokens).
- Criar um Parser (quebra os tokens em uma Abstract Syntax Tree - AST).
- Criar uma Máquina Virtual Baseada em Pilha (Stack VM) em Rust que lê os nós da árvore e executa as operações de memória de forma interpretada.
- Adicionar um Garbage Collector simples de "Mark-and-Sweep" varrendo os ponteiros do Rust.

### Fase 10 — Multiprocesso Nativo (Isolamento)
**🎯 Objetivo:** Uma aba crasha, o navegador não morre.
- Usar chamadas diretas de criação de processo (`CreateProcessW` no Windows ou `fork`/`exec` em UNIX).
- Serializar as mensagens de pintura do Renderer Process para o Browser Process através de *Named Pipes* criados manualmente.

---

## 3. O Custo da Glória (Realidade do Prazo)
Ao banir dependências e focar na pureza total do "feito a mão", o roadmap do Albedo se transforma. 
- O MVP funcional (carregar uma página HTML simples offline e renderizar o texto) que antes levaria alguns meses, agora é o objetivo principal de 1 a 2 anos de trabalho intensivo.
- A recompensa: Um profundo domínio sobre cada byte e cada transação eletrônica que acontece no navegador. Conhecimento de kernel, redes, matemática gráfica e compiladores que pouquíssimos engenheiros no mundo possuem.

---
*Este é o manifesto do Albedo Purista. Não importamos conhecimento, nós o forjamos.*