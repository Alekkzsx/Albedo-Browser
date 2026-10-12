# ADR-0014: Adoção do AccessKit como Fundação de Acessibilidade

- **Status:** Aceito
- **Data:** 2026-08-16
- **Decisores:** Engenharia Central do Albedo
- **Subsistemas Afetados:** `ace_ui`, `ace_dom`

---

## 1. Contexto e Problema

Acessibilidade (a11y) é um requisito essencial para qualquer navegador web moderno e inclusivo. Leitores de tela e tecnologias assistivas dependem de APIs proprietárias e complexas em cada sistema operacional:
- **Windows:** UI Automation (UIA) e MSAA/IAccessible2.
- **macOS:** NSAccessibility.
- **Linux:** AT-SPI2 / D-Bus.

Implementar adapters C/C++ ou FFI artesanais para cada uma dessas três plataformas sobrecarregaria desnecessariamente o projeto com código inseguro, dependências do sistema e manutenção contínua de contratos de acessibilidade da plataforma.

## 2. Decisão

Adotamos a biblioteca Rust de código aberto [`AccessKit`](https://github.com/AccessKit/accesskit) como fundação de ponte multiplataforma para a árvore de acessibilidade do navegador:

1. **Separação de Papéis:**
   - O Albedo constrói e mantém sua própria **Accessibility Tree** (derivada da árvore DOM do `ace_dom` e dos nós de layout com papéis semânticos ARIA e atributos HTML5).
   - O `AccessKit` atua como a camada de adaptação que traduz mutações na árvore de nós para chamadas nativas de UI Automation (Windows), NSAccessibility (macOS) e AT-SPI (Linux).
2. **Atualizações Incrementais:** O motor envia apenas as árvores de nós modificadas (*tree updates*) para o AccessKit, minimizando impacto na performance de renderização.

## 3. Consequências

- **Positivas:**
  - Compatibilidade imediata com leitores de tela modernos (NVDA, JAWS, Narrator, VoiceOver, Orca).
  - Eliminação de centenas de linhas de código inseguro de FFI nas chamadas diretas de Win32/COM, Cocoa e D-Bus.
  - Conformidade estrita com o Paradigma Pragmático (Regra 1): a alma da árvore semântica é nossa, a infraestrutura de ponte é alavancada de uma crate madura da comunidade Rust.
- **Negativas / Riscos:**
  - Dependência do ciclo de lançamentos e evolução do projeto AccessKit para novas extensões de acessibilidade da plataforma.

