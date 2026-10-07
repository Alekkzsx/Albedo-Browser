# 🎯 Central de Escopo Normativo do MVP (v1.0)

> **Diretório:** `docs/scope/`  
> **Status:** Ativo & Vigente  
> **Propósito:** Definir os limites normativos estritos de escopo da versão 1.0 do Albedo Browser para desktop (Windows, Linux, macOS).

---

## 🧭 Visão Geral do Escopo

Para que o Albedo atinja estabilidade e excelência industrial sem sofrer de expansão descontrolada de escopo (*scope creep*), todo recurso admitido no motor deve estar formalmente catalogado nos documentos desta pasta.

A governança do MVP divide o conteúdo web em **três especificações normativas complementares**:

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
| [**`mvp_scope.md`**](./mvp_scope.md) | **Fronteira Geral do MVP** | Define a entrega da v1.0 para desktop, conectividade HTTPS (H1/H2/H3), cache RFC 9111, isolamento de processos por site e a política de motor JS interpretado (*jitless*). |
| [**`html_elements.md`**](./html_elements.md) | **Elementos HTML5 (WHATWG §4)** | Lista taxativa das tags HTML e atributos essenciais que o Tokenizer e o Tree Builder constroem na árvore DOM sem descarte. |
| [**`css_properties.md`**](./css_properties.md) | **Propriedades CSS (W3C)** | Subconjunto estrutural de CSS: Box Model, Flexbox, Grid básico com `minmax()`, Cores Oklab/Oklch e camadas de cascata `@layer`. |
| [**`web_api.md`**](./web_api.md) | **Web APIs para SPAs (W3C/WHATWG)** | Conjunto mínimo de APIs de DOM, EventTarget, Timers, Fetch, Web Workers e History API para execução fluida de SPAs. |

---

## ⚖️ Critério de Admissão de Novos Recursos

Qualquer funcionalidade web não listada nestes catálogos:
1. **No HTML/CSS:** É tratada de forma silenciosa e resiliente pelo parser, sem causar pânico (*panic*) ou corromper a renderização do restante da página.
2. **No JavaScript:** Emite um aviso no console e lança a exceção padrão `NotSupportedError`.
3. **Inclusão no Escopo:** A inclusão de novas propriedades ou APIs no MVP exige a abertura formal de um **ADR** em `docs/adr/`.

