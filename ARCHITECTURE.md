# Albedo Browser Architecture

## Visão Geral
O Albedo Browser é um navegador web moderno escrito em Rust, focado em performance, segurança e baixo consumo de recursos. Diferente de navegadores baseados em Chromium ou Gecko, o Albedo implementa seu próprio motor de renderização chamado **ACE (Albedo Core Engine)**.

## Estrutura do Projeto

### 1. `src/ace` - Albedo Core Engine
O coração do navegador. Responsável por parsear, interpretar e renderizar conteúdo web.
- **HTML Parser**: Implementação própria do HTML5 (com suporte a streaming e fast-path).
- **DOM Engine**: Construção e manipulação da árvore de documentos.
- **CSS Engine**: Parse, cascata, especificidade e resolução de estilos.
- **Layout Engine**: Cálculo de geometria (flexbox, grid, block layout) usando Taffy.
- **JS Runtime**: Integração com QuickJS para execução de JavaScript.
- **Bindings**: Ponte entre APIs Web (Window, Document, Element) e o motor interno.

### 2. `src/browser` - Lógica do Navegador
Gerencia a experiência do usuário, múltiplas abas, histórico e navegação.
- State management das abas.
- Comunicação entre UI e Engine.
- Gerenciamento de sessão e cookies.

### 3. `src/network` - Camada de Rede
Implementação assíncrona de protocolos de rede.
- HTTP/1.1, HTTP/2, HTTP/3 (QUIC).
- Cache de recursos.
- Segurança (TLS, validação de certificados).

### 4. `src/renderer` - Pipeline Gráfico
Responsável pela pintura final na tela.
- Rasterização com `tiny-skia`.
- Composição GPU com `wgpu`.
- Gerenciamento de camadas e texturas.

### 5. `src/ui` - Interface do Usuário
Interface nativa construída com Slint.
- Barra de endereços, abas, menu.
- Integração com o canvas de renderização.

### 6. `src/utils` & `albedo-jit`
- Utilitários gerais (crypto, allocators, strings).
- JIT Compiler para otimização de JavaScript (experimental).

## Fluxo de Renderização

1. **Network**: Baixa HTML/CSS/JS.
2. **HTML Parser**: Gera tokens e constrói a DOM Tree.
3. **CSS Parser**: Gera regras de estilo.
4. **Style Resolution**: Aplica CSS à DOM (Cascade).
5. **Layout**: Calcula posições e tamanhos (Reflow).
6. **Display List**: Gera comandos de pintura.
7. **Renderer**: Rasteriza e compõe na tela (Paint).
8. **JS Runtime**: Executa scripts, pode disparar refluxos/repinturas.

## Metas de Design
- **Zero Copy**: Minimizar cópias de dados entre threads.
- **Streaming**: Parse e renderização incremental.
- **Segurança**: Sandboxing rigoroso de processos e scripts.
- **Memória**: Alvo < 5MB por aba em idle.
