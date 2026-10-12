# ADR-0003: Pilha de Criptografia e TLS baseada em Rustls

- **Status:** Aceito & Implementado
- **Data:** 2026-08-16 (Implementado na Fase 3)
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_net`

---

## 1. Contexto e Problema

Toda a conectividade web moderna passa por conexões criptografadas (HTTPS/TLS). Bibliotecas legadas em C (como OpenSSL ou LibreSSL) possuem histórico constante de vulnerabilidades de corrupção de memória e são difíceis de auditar e empacotar de forma reproduzível.

## 2. Decisão

Adotamos a crate **`rustls`** (com backend criptográfico moderno `aws-lc-rs` e raízes de certificados via `webpki-roots`) como Fundação exclusiva para transporte TLS 1.2 e TLS 1.3 no `ace_net`.

## 3. Consequências

- **Positivas:** 100% Memory-Safe, elimina vulnerabilidades estilo Heartbleed, simplifica a compilação cruzada e é compatível com os padrões modernos de ALPN (h2, h3).
- **Negativas / Riscos:** Não suporta versões inseguras e obsoletas de criptografia (como SSL 3.0 ou TLS 1.0), o que é um benefício arquitetural de segurança para o Albedo.

## 4. Notas de Implementação

> [!NOTE]
> **Status de Materialização:** Implementado e validado em produção no crate `ace_net`.

- **`ace_net` (`Albedo_Core_Engine/ace_net/Cargo.toml`):**
  - Dependências criptográficas:
    - `rustls = { version = "0.23", default-features = false, features = ["aws_lc_rs", "logging", "std", "tls12"] }`
    - `webpki-roots = "0.26"`
    - `tokio-rustls = "0.26"`
  - Camada de transporte: `src/transport/tls.rs` e `src/transport/connector.rs` encapsulam a inicialização e handshake TLS 1.2/1.3 sem dependências de C ou OpenSSL.
  - Negociação de protocolo (ALPN): Suporte e seleção nativa de protocolos (`h2`, `http/1.1`).
  - Verificação de cadeias: Raízes confiáveis validadas via `webpki-roots` em conformidade estrita com o ecossistema WebPKI.
- **Segurança e Conformidade:**
  - Zero blocos `unsafe` na camada de transporte de rede de `ace_net`. Elimina vetores de estouro de buffer típicos de bibliotecas criptográficas herdadas em C.

