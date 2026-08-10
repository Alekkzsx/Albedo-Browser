# Escopo do MVP (v1.0) - O Abismo Restrito

Para evitar o colapso do projeto e garantir que as Regras de Ouro (Zero-Dependency) sejam exequíveis, a versão 1.0 focará na sobrevivência e infraestrutura base.

## O que ENTRA no MVP:
- **Rede:** Conexão TCP/HTTP/1.1 e resolução DNS própria em sockets assíncronos OS.
- **Parsing:** HTML5 Tokenizer básico e CSS3 Lexer (seletores simples).
- **Estilo/Layout:** Box model completo, Flexbox e BFC/IFC básico. Suporte a Grid com `minmax()`.
- **Motor JS:** Interpretador de Bytecode (sem JIT), Mark-and-Sweep GC e subset do ES6. Microtasks rigorosas.
- **Renderização (Software):** CPU rasterizer para retângulos coloridos e texto LTR (TrueType básico sem suporte CJK).
- **Integração OS:** Criação de janela e input (`ace_browser`) via bindings puros (`windows-sys`/`libc`).

## O que FICA PARA O PÓS-MVP:
- Compilação JIT x86_64/ARM64 (o interpretador será a única forma de rodar JS na v1).
- HarfBuzz FFI (Texto Complexo RTL, Árabe, Chinês, Japonês e Emojis ZWJ).
- Aceleração por Hardware GPU Compositing.
- WebAssembly, Service Workers complexos, WebRTC.
- HTTP/2, HTTP/3 (QUIC) e criptografia ASN.1 complexa (se houver fallback para HTTPS seguro, será via FFI).
- Decodificador de vídeo (AV1/H.264) e JPEG complexo.
