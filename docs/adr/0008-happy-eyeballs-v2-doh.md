# ADR-0008: Happy Eyeballs v2 e DNS-over-HTTPS (DoH) Nativo

- **Status:** Aceito
- **Data:** 2026-08-16
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_net`, `ace_core`

---

## 1. Contexto e Problema

A resolução DNS tradicional via chamadas síncronas do sistema operacional (`getaddrinfo`) apresenta graves limitações para navegadores modernos:
1. **Falta de Privacidade e Criptografia:** Consultas em texto claro na porta 53 UDP estão sujeitas a espionagem, spoofing, sequestro de DNS por provedores (*ISP tampering*) e censura.
2. **Latência de Fallback IPv6/IPv4:** Quando um domínio retorna endereços IPv6 (AAAA) e IPv4 (A), tentativas sequenciais que falham no IPv6 introduzem atrasos perceptíveis (frequentemente de 3 a 20 segundos de timeout de conexão TCP), degradando severamente a experiência do usuário.

## 2. Decisão

Adotamos a implementação nativa em `ace_net::dns` de dois padrões normativos da IETF:

1. **DNS-over-HTTPS (DoH — RFC 8484):**
   - As consultas DNS são enviadas como requisições binárias seguras (`application/dns-message`) encapsuladas em HTTPS com TLS 1.3 via `rustls`.
   - Suporte a resolvers DoH públicos (Cloudflare, Quad9, Google) e endpoints locais customizados pelo usuário.
2. **Happy Eyeballs v2 (RFC 8305):**
   - Resolução paralela de registros A e AAAA.
   - Ordenação intercalada de endereços IP favorecendo IPv6 quando disponível.
   - Disparo de tentativas de conexão TCP com escalonamento temporal controlado (*Connection Attempt Delay*, padronizado em 250ms a 300ms): se o primeiro endereço não estabelecer o handshake TCP na janela inicial, uma tentativa concorrente é iniciada para a família alternativa.
   - A primeira conexão que completar com sucesso o handshake vence e assume o stream de transporte; as demais tentativas concorrentes são canceladas imediatamente.

## 3. Consequências

- **Positivas:**
  - Redução drástica da latência percebida em redes mistas ou com conectividade IPv6 defeituosa (*broken IPv6*).
  - Proteção criptográfica completa contra manipulação de respostas de DNS na camada de transporte.
  - Eliminação de chamadas de sistema bloqueantes na thread assíncrona do `tokio`.
- **Negativas / Riscos:**
  - Maior complexidade na gestão de timers assíncronos e cancelamento de sockets parciais.
  - Consumo ligeiramente superior de sockets temporários durante handshakes em redes com alta latência.

