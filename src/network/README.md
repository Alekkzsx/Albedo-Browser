# 📁 ACE-Net (`src/network/`)

> [!NOTE]
> O **ACE-Net** (*Albedo Communication Engine - Network*) é a pilha (stack) de rede nativa de alta performance do Albedo Browser, responsável pelo carregamento assíncrono de recursos, segurança de comunicação e protocolos da web moderna.

Este diretório contém a engine de rede do navegador, focada em eficiência energética, latência ultra-baixa e segurança robusta de acordo com os padrões W3C e IETF.

---

## 🎯 Objetivo & Função

Na arquitetura do **Albedo Browser**, o **ACE-Net** atua como a ponte entre o motor de renderização (Layout/DOM) e a World Wide Web. Suas principais responsabilidades incluem:

1. **Carregamento Assíncrono de Recursos**: Resolução em paralelo de folhas de estilo (CSS), scripts (JS), marcação (HTML) e mídias sem bloquear a thread principal da interface.
2. **Protocolos de Última Geração**: Suporte nativo a **HTTP/3 sobre conexões QUIC**, com fallback automático para HTTP/2 e HTTP/1.1 sob demanda.
3. **Persistência & Otimização local**: Banco de dados SQLite mapeando o cache em disco para evitar requisições redundantes, acompanhado de um armazenamento local em memória para Blobs.
4. **Aplicação de Políticas de Segurança**: Validação ativa de regras CORS, Same-Origin Policy (SOP), Content Security Policy (CSP) e sandboxing de cookies.
5. **Pré-resolução Preditiva**: Mecanismo de DNS Prefetching para aquecer o cache do sistema operacional e economizar milissegundos críticos durante a navegação.

---

## 📄 Arquivos e Suas Funções

Abaixo estão descritos todos os arquivos que compõem o módulo `src/network/`:

| Arquivo | Componente / Estrutura | Função Principal |
| :--- | :--- | :--- |
| [**mod.rs**](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/network/mod.rs) | Módulo Raiz | Declara e expõe de forma pública todos os submódulos da stack de rede (`blob`, `cache`, `client`, `dns`, `http3`, `protocols`, `resources`, `security`). |
| [**blob.rs**](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/network/blob.rs) | `BlobStore` / `Blob` | Armazena objetos binários (Blobs) em memória, provendo URLs temporárias no padrão `blob:<uuid>` e controlando a sua liberação (revogação). |
| [**cache.rs**](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/network/cache.rs) | `DiskCache` | Gerencia o cache local persistente em disco. Mapeia metadados e controle de expiração em um banco **SQLite** (`cache.db`) e gerencia a compactação das respostas (Gzip, Brotli, Zstd) com política de despejo LRU. |
| [**client.rs**](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/network/client.rs) | `FetchClient` | Implementa a API de requisições equivalente ao `fetch()` dos navegadores. Controla pools de conexões e gerencia respostas opacas (*opaque response*) para requisições cross-origin sem CORS. |
| [**dns.rs**](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/network/dns.rs) | `DnsPrefetcher` | Resolve nomes de domínio em background, populando o cache de DNS do sistema operacional de forma antecipada para acelerar conexões futuras. |
| [**http3.rs**](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/network/http3.rs) | `Http3Client` | Cliente **HTTP/3 baseado em QUIC** real utilizando as bibliotecas `quinn` e `h3`. Suporta conexões 0-RTT, pooling de socket UDP único e recuperação rápida de perda de pacotes. |
| [**resources.rs**](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/network/resources.rs) | `ResourceManager` | Gerenciador assíncrono de recursos que coordena as requisições. Suporta protocolos especiais (`data:`, `blob:`, `file:`), interceptação por **Service Workers** e descarrega a decodificação de imagens para threads de background. |
| [**security.rs**](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/network/security.rs) | `Origin`, `AccessControl`, `CookieJar`, `CSP` | Centraliza a lógica de proteção. Valida regras Same-Origin, requisições CORS com credenciais, interpretação de cabeçalhos CSP (`script-src`, `connect-src`) e persistência de cookies seguros. |
| [**protocols/mod.rs**](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/network/protocols/mod.rs) | Submódulo de Protocolos | Ponto de entrada para canais de comunicação bidirecionais contínuos sobre a rede. |
| [**protocols/websocket.rs**](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/src/network/protocols/websocket.rs) | `WebSocketClient` | Cliente WebSocket assíncrono baseado em `tokio-tungstenite`. Opera isolado em uma tarefa assíncrona comunicando-se com a engine por canais de mensagens (`mpsc`). |

---

## 🛠️ O que DEVE e NÃO DEVE estar aqui (Regras de Design)

Para manter a separação de conceitos limpa e preservar a modularidade da arquitetura, todos os desenvolvedores devem aderir às seguintes diretrizes:

### ✅ O que DEVE estar aqui
- **Protocolos de Transporte**: Qualquer protocolo ou canal de comunicação da web (HTTP/3, HTTP/2, WebSockets, WebTransport, etc.).
- **Políticas Globais de Segurança de Rede**: Mecanismos de validação de cabeçalhos HTTP de segurança como CORS, CSP, Same-Origin e Cookie sandboxing.
- **Armazenamento de Rede**: Cache HTTP persistente, cache de DNS e gerenciamento de arquivos em memória como o `BlobStore`.
- **Estratégias de Redundância e Fallback**: Código que gerencie a transição transparente de conexões lentas ou com falhas de HTTP/3 para HTTP/2 e HTTP/1.1.
- **Decodificação Assíncrona de Mídia**: Pré-processamento e decodificação leve de ativos (como carregar bytes de imagens em pixels brutos no threadpool) para acelerar a renderização da UI.

### ❌ O que NÃO DEVE estar aqui
- **Lógica de Interface de Usuário (UI)**: Código do shell do navegador, renderização gráfica direta, gerenciamento de abas físicas ou do canvas de desenho.
- **Manipulação direta do DOM**: O módulo de rede fornece apenas os bytes crus ou imagens decodificadas; ele não sabe o que é uma árvore DOM ou regras de folha de estilo CSS.
- **Ambiente de Execução JavaScript (JS Engine)**: Nenhuma interação direta com o interpretador de JavaScript (V8, QuickJS) deve ser programada aqui. A engine de rede é agnóstica a quem consome seus recursos.
- **Configurações Gerais do Shell**: Histórico de navegação, favoritos, gerenciamento de janelas e configurações locais do usuário não devem vazar para o ACE-Net.

---

> [!TIP]
> **Dica de Performance**: Ao carregar múltiplos assets (como imagens ou CSS) em um documento, prefira sempre o uso de `ResourceManager::fetch`, pois ele tira vantagem do multiplexing e pooling de conexões internas do `Http3Client` e `FetchClient`.
