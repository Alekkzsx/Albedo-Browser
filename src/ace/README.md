# 📁 src/ace/

Esta pasta é a raiz do **ACE (Albedo Core Engine)**, o motor de renderização e execução nativo do **Albedo Browser**.

## 🎯 Objetivo & Função
O motor ACE processa arquivos de entrada (HTML, CSS e scripts JavaScript), monta e atualiza a árvore DOM, realiza o matching de estilos CSS, resolve posicionamentos através da engine de Layout, e prepara a Display List de primitivas visuais para serem enviadas ao rasterizador (`src/renderer/`). Ele atua como o motor central que encapsula a inteligência de renderização da Web e o ambiente de script.

## 📄 Estrutura de Submódulos e Arquivos

| Subdiretório / Arquivo | Função / Propósito |
| :--- | :--- |
| [engine/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/engine) | Orquestrador principal. Combina a árvore de nós DOM, o cascade de CSS (`style/`), o cálculo geométrico (`layout/`), representações de canvas/composição GPU (`graphics/`) e a lógica de animações. |
| [html/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/html) | Módulo de parsing ACE-HTML (tokenizer complacente com WHATWG, tree builder, decodificadores de encoding, parser de links de preload e serializadores). |
| [runtime/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/runtime) | Camada de runtime Javascript que integra com QuickJS (`rquickjs`) e fornece as amarras de tipos para DOM e Web APIs (fetch, websockets, localstorage, subtle crypto). |
| [url.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/url.rs) | Implementação utilitária simplificada da especificação de URL. |
| [json.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/json.rs) | Lógica simples de serialização/desserialização de JSON nativo da engine. |
| [crypto.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/crypto.rs) | Utilitários criptográficos de hashing (SHA1, SHA256) e assinaturas para APIs Web. |
| [util/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/ace/util) | Pasta com utilitários gerais como monitor do sistema (`sysinfo.rs`), geradores pseudoaleatórios (`random.rs`), codificadores Base64 e UUID. |

## 🛠️ Regras de Design (O que DEVE e NÃO DEVE estar aqui)

### O que DEVE estar aqui:
- Lógica de conformidade de especificações web (DOM Living Standard, CSS selectors, HTML5 Parsing, ECMAScript).
- Definições de estruturas de nós da árvore e representação em memória do documento.
- Algoritmos de Cascade, herança de propriedades e layouts geométricos.

### O que NÃO DEVE estar aqui:
- Qualquer lógica de desenho físico de pixels ou rasterização na tela (deve estar em `src/renderer/`).
- Gerenciamento do Browser Chrome (janelas do SO, barra de menus, tabs, botão de favoritos, etc. - deve estar em `src/browser/` ou `src/ui/`).
- Algoritmos complexos de JIT e manipulação nativa de Cranelift (deve estar na crate `albedo-jit/`).
