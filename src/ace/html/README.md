# 📁 src/ace/html/

O diretório `src/ace/html/` abriga o **ACE-HTML parser**, o componente responsável pela análise léxica (tokenização), análise sintática (tree building), decodificação de codificações de caracteres e serialização de documentos HTML no navegador **Albedo**. 

Para garantir alto desempenho e total conformidade com as especificações da **WHATWG HTML5**, o parser combina o poder da biblioteca `html5gum` (para tokenização rápida e otimizada por SIMD/Arena) com a robustez da `html5ever` (da engine Servo, para a construção da árvore DOM em conformidade com as regras complexas de *tree building* da especificação).

---

## 🎯 Objetivo & Função

Na arquitetura do sistema, o **ACE-HTML** atua como a primeira etapa do pipeline de renderização do documento. Ele traduz fluxos brutos de bytes (recebidos da rede ou de arquivos locais) em uma estrutura de dados de árvore manipulável (`HtmlDocument`).

### Principais Atribuições:
1. **Detecção e Decodificação de Codificação**: Identifica a codificação correta do documento (UTF-8, Windows-1252, etc.) por meio de BOM (Byte Order Mark), metadados do cabeçalho HTTP ou tags `<meta charset>`.
2. **Tokenização**: Analisa o texto HTML e gera um fluxo de tokens (tags, texto, comentários, doctypes).
3. **Construção de Árvore (Tree Building)**:
   - **Caminho Padrão (Full Path)**: Executa a especificação completa de HTML5 via `html5ever` e um *sink* customizado (`AceTreeSink`) que constrói a árvore e lida com quirks como Foster Parenting e elementos especiais (ex: `<selectedcontent>`).
   - **Caminho Rápido (Fast Path)**: Evita a sobrecarga do construtor de árvore completo para documentos simples de grande porte (bypass do tree-builder completo quando não há scripts, comentários complexos ou entidades especiais), proporcionando alto desempenho em páginas grandes de dados puros.
4. **Varredura de Recursos Especulativa (Preloads)**: Identifica de forma antecipada recursos críticos a serem carregados (folhas de estilo, scripts, fontes) antes de concluir o parsing completo do documento, otimizando o carregamento da página.
5. **Parsing Incremental (Streaming)**: Suporta a alimentação incremental de bytes (`feed_bytes`), ideal para renderizar a página enquanto ela ainda está sendo baixada da rede.

---

## 📄 Arquivos e Suas Funções

Abaixo estão os arquivos contidos neste diretório e suas respectivas responsabilidades:

| Arquivo | Função / Descrição |
| :--- | :--- |
| [`mod.rs`](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html/mod.rs) | Ponto de entrada do módulo. Define as APIs públicas de parsing (`parse_document`, `parse_fragment`), coordena o uso do *fast-path* ou do parser `html5ever`, e converte tokens do `html5gum` para o formato interno do ACE. |
| [`types.rs`](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html/types.rs) | Contém as definições de tipos fundamentais da árvore sintática (DOM) do ACE (`HtmlDocument`, `HtmlNode`, `HtmlElement`), estruturas de tokens (`HtmlToken`), tipos de erros (`ParseError`) e configurações do parser. |
| [`html5ever_parser.rs`](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html/html5ever_parser.rs) | Módulo interno que encapsula o parser `html5ever`. Configura e executa a análise completa de documentos e fragmentos HTML em conformidade estrita com o padrão WHATWG. |
| [`sink.rs`](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html/sink.rs) | Implementa a trait `TreeSink` da biblioteca `html5ever`. É responsável por traduzir os eventos de criação e movimentação de nós do parser na árvore interna de nós do ACE (`AceSinkNode`), aplicando também hacks de compatibilidade (como injeção de quebras de linha e espelhamento de `<selectedcontent>`). |
| [`tokenizer_v2.rs`](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html/tokenizer_v2.rs) | O `AceTokenizer` é um wrapper de alta performance baseado em `html5gum`. Ele otimiza a conversão de bytes e strings HTML em tokens WHATWG, integrando-se a um alocador de arena (`AceAllocator`) para eliminar alocações desnecessárias. |
| [`tree_builder.rs`](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html/tree_builder.rs) | Implementação manual e simplificada de um construtor de árvore (`HtmlTreeBuilder`) baseado em máquinas de estado WHATWG (InsertionModes). É usado principalmente em contextos alternativos ou leves. |
| [`streaming.rs`](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html/streaming.rs) | Implementa o `StreamingHtmlParser`, que permite o parsing incremental de documentos à medida que blocos de bytes são recebidos, gerenciando latências (p50/p99) e snapshots de estado para restauração rápida. |
| [`preloads.rs`](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html/preloads.rs) | Módulo focado na varredura especulativa de elementos `<link>` e `<script>` para pre-loading. Apresenta uma varredura ultra-rápida (`collect_preloads_fast`) usando SIMD/`memchr` para achar tags de preload antes da construção total do DOM. |
| [`fast_parse.rs`](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html/fast_parse.rs) | Implementa o parser de caminho rápido (`try_fast_parse_document`). Se o HTML for grande e atender a critérios restritos (ex: sem comentários complexos, scripts, estilos ou entidades especiais), ele usa uma estratégia de parsing baseada em pilha direta, reduzindo drasticamente o consumo de CPU. |
| [`encoding.rs`](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html/encoding.rs) | Lógica para decodificação de bytes em strings UTF-8. Implementa heurísticas de detecção de BOM e análise do cabeçalho `<meta charset>` para dar suporte a UTF-8, Windows-1252, UTF-16LE e UTF-16BE. |
| [`serializer.rs`](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html/serializer.rs) | Utilitários para converter a árvore DOM customizada do ACE de volta em strings de texto HTML (ideal para depuração ou testes de conformidade), com suporte a indentação bonita e escape de caracteres especiais. |
| `entities.json` | Arquivo contendo o mapeamento padrão de entidades de caracteres nomeadas do HTML5 (ex: `&amp;`, `&lt;`) para seus respectivos caracteres Unicode. |

---

## 🛠️ O que DEVE e NÃO DEVE estar aqui (Regras de Design)

Para manter a separação de conceitos limpa e preservar a performance do parser, siga rigorosamente as seguintes diretrizes de design:

### 🟢 O que DEVE estar aqui:
* **Conformidade WHATWG**: Qualquer código de especificação de tokenização ou de tree building para HTML5.
* **Otimizações de Baixo Nível**: Técnicas para evitar alocações de memória (como o uso de alocadores de Arena ou estruturas pequenas como `SmallVec` / `SmolStr`).
* **Suporte a Novas Tags Estruturais**: Mapeamentos de elementos HTML legítimos ou propostas de web standards (ex: o mirroring do componente `<selectedcontent>` do Open UI no `sink.rs`).
* **Tratamento de Codificação (Encodings)**: Regras de fallback de codificações e sniffers de tags `<meta>`.

### 🔴 O que NÃO DEVE estar aqui:
* **Lógica de Interface do Usuário (UI)**: Este diretório lida estritamente com dados e sintaxe. Nenhuma biblioteca gráfica, janelas ou chamadas à interface devem estar aqui.
* **Lógica de Rede (Networking)**: Embora o módulo identifique links para pré-carregamento (`PreloadRequest`), ele **não** deve realizar requisições de rede. A busca real dos recursos deve ser delegada ao gerenciador de rede do navegador.
* **Mecanismo de Execução de JavaScript**: Nenhuma lógica de execução de código JS, manipulação de contexto V8/SpiderMonkey ou APIs do DOM voltadas a scripts (bindings) deve constar aqui.
* **Cálculos de Layout ou Estilo**: A resolução de seletores CSS e o cálculo geométrico de posições físicas (Render/Layout tree) pertencem a módulos subsequentes no pipeline do navegador.
