# 🎯 Central de Escopo Normativo do MVP (v1.0)

> **Diretório:** `docs/scope/`  
> **Status:** Ativo & Vigente (Alinhado com o [PLANO.md v7.1](../../PLANO.md) — §1.4 e Fases 1–6 Concluídas)  
> **Última Atualização:** 2026-10-12  
> **Propósito:** Definir os limites normativos estritos de escopo da versão 1.0 do Albedo Browser para desktop (Windows, Linux, macOS).

---

## 📜 Histórico de Governança do Escopo

| Versão | Data | Contexto / Marco | Decisões & Escolhas Técnicas | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **2.0.0** | 2026-10-12 | Sincronização Ground Truth & Pós-Fase 6 | • Alinhamento do escopo com a conclusão das Fases 1 a 6 (**1.024 testes verdes no workspace**).<br>• Formalização do contrato de caixas (`BoxGenerationKind`) para a **Fase 7 (`ace_layout`)**.<br>• Ratificação do modo *jitless* permanente para o motor JS e da infraestrutura de GC Tracing em `ace_dom`. | • Catálogos de HTML, CSS e Web APIs perfeitamente ancorados no código real.<br>• Resolução de DV-01, DV-05, DV-09 e DV-11 refletida. |
| **1.0.0** | 2026-10-07 | Catalogação Inicial Normativa | • Tríade de escopo (HTML, CSS e Web APIs) para SPAs e documentos web reais. | • Publicação dos catálogos normativos. |

---

## 🧭 Visão Geral do Escopo

Para que o Albedo atinja estabilidade e excelência industrial sem sofrer de expansão descontrolada de escopo (*scope creep*), todo recurso admitido no motor deve estar formalmente catalogado nos documentos desta pasta.

A governança do MVP divide o conteúdo web em **quatro especificações normativas complementares**:

```
docs/scope/
├── README.md               # Este índice geral de escopo
├── mvp_scope.md            # Fronteira geral do MVP: o que entra na v1.0 vs pós-MVP
├── html_elements.md        # Catálogo taxativo dos elementos HTML5 e atributos suportados
├── css_properties.md       # Catálogo taxativo das propriedades CSS suportadas
└── web_api.md              # Catálogo normativo de APIs JavaScript e DOM expostas
```

---

## 📚 Catálogos de Especificação Normativa

| Documento | Foco Normativo | O que Garante no MVP |
| :--- | :--- | :--- |
| [**`mvp_scope.md`**](./mvp_scope.md) | **Fronteira Geral do MVP** | Define a entrega da v1.0 para desktop, orquestração e2e (`BrowserPipeline`), conectividade HTTPS (H1/H2/H3 via `ace_net`), cache RFC 9111, barramento lock-free `ace_ipc` (>2.22M msg/s), isolamento de processos por site e o motor JS interpretado (*jitless* permanente). |
| [**`html_elements.md`**](./html_elements.md) | **Elementos HTML5 (WHATWG §4, §12)** | Lista taxativa das tags HTML e atributos essenciais que o Tokenizer e o Tree Builder constroem na árvore DOM em Arena sem descarte (Adoption Agency Algorithm de 16 passos, DSD, `StreamingHtmlParser`, 252 testes verdes no `ace_dom`). |
| [**`css_properties.md`**](./css_properties.md) | **Propriedades CSS (W3C)** | Subconjunto estrutural de CSS: Box Model, Flexbox, Grid básico com `minmax()`, Cores Oklab/Oklch, funções matemáticas `calc()`/`min()`/`max()`/`clamp()`, camadas `@layer`, 8 origens de cascata e contrato `BoxGenerationKind` para `ace_layout` (25 testes verdes no `ace_style`). |
| [**`web_api.md`**](./web_api.md) | **Web APIs para SPAs (W3C/WHATWG)** | Conjunto mínimo de APIs de DOM Core (bindings WebIDL funcionais, `DOMDataStore` GC Tracing, Ancestor Bloom Filter), EventTarget, Timers, Fetch, Web Workers e History API para execução fluida de SPAs. |

> [!NOTE]
> **Status de Engenharia do Motor:**  
> O Albedo Core Engine completou com êxito as Fases 1, 2, 3, 4a, 5 e 6 com **1.024 testes verdes (181 suítes)** e 0 warnings no Clippy.  
> As especificações desta pasta servem de contrato mandatório para a **Fase 7 (`ace_layout`)**, que implementará os algoritmos de posicionamento e dimensionamento geométrico (BFC, IFC, Flexbox, Grid) consumindo a `RenderTree` gerada pelo `ace_style`.

---

## ⚖️ Critério de Admissão de Novos Recursos

Qualquer funcionalidade web não listada nestes catálogos:
1. **No HTML/CSS:** É tratada de forma silenciosa e resiliente pelo parser, sem causar pânico (*panic*) ou corromper a renderização do restante da página.
2. **No JavaScript:** Emite um aviso no console e lança a exceção padrão `NotSupportedError`.
3. **Inclusão no Escopo:** A inclusão de novas propriedades ou APIs no MVP exige a abertura formal de um **ADR** em `docs/adr/`.


