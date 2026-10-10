# ADR-0003: Pilha de Criptografia e TLS baseada em Rustls

- **Status:** Aceito
- **Data:** 2026-08-16
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

