# 🏷️ Catálogo Normativo de Elementos HTML5 no MVP (v1.0)

> **Documento Normativo de Escopo HTML**  
> **Subsistema Alvo:** `Albedo_Core_Engine/ace_dom`  
> **Status:** Vigente (Alinhado com o [PLANO.md v7.1](../../PLANO.md) — §1.4 e Fase 5 Concluída)  
> **Última Atualização:** 2026-10-12  
> **Padrões Normativos:** WHATWG HTML Living Standard (§4 The elements of HTML, §12 Parsing HTML documents).

---

## 📜 Histórico de Revisões e Decisões

| Versão | Data | Contexto / Marco | Decisões & Escolhas Arquiteturais | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **2.0.0** | 2026-10-12 | Alinhamento com Tree Builder & WPT | • Validação normativa dos modos de inserção do Tree Builder conforme o W3C/WHATWG §12.<br>• Algoritmo da Agência de Adoção (AAA) de 16 passos validado com fixtures canônicas `tests1.dat` (7/7 cenários aprovados).<br>• Integração com o `StreamingHtmlParser` para construção incremental da árvore sem bloqueios. | • Escopo 100% alinhado aos 252 testes do `ace_dom`.<br>• Dívida DV-11 100% quitada. |
| **1.0.0** | 2026-10-07 | Catalogação Normativa v7.0 | • Definição do conjunto taxativo de tags e atributos parseados na árvore DOM na versão 1.0 desktop.<br>• Suporte nativo a Declarative Shadow DOM (`<template shadowrootmode>`). | • Documento criado para fechar a tríade de escopo (HTML + CSS + Web APIs). |

---

## 1. Diretriz do Parser HTML5 e Tree Builder

Todo elemento listado abaixo é reconhecido pelo **Tokenizer HTML5** acelerado por instruções SIMD (`memchr`) e estruturado pelo **Tree Builder** do `ace_dom` na `Arena<NodeData>`.

> [!NOTE]
> **Modos de Inserção e Conformidade W3C:**  
> O Tree Builder implementa a máquina de estados completa do WHATWG HTML §12 (`InHead`, `InBody`, `Text`, `InTable`, `InSelect`, `InTemplate`, etc.) e o algoritmo AAA de 16 passos para recuperação de tags de formatação não-balanceadas, com 100% de conformidade demonstrada na suíte oficial `html5lib-tests` (`tests1.dat`).

Elementos desconhecidos ou personalizados (*Custom Elements*) são instanciados como `HTMLUnknownElement` ou genericamente como nós de elemento válidos, sem quebrar ou desestabilizar a árvore DOM.

---

## 2. Elementos Suportados por Categoria

### 2.1 Raiz e Metadados do Documento
- `<html>`: Elemento raiz do documento.
- `<head>`: Contêiner de metadados.
- `<title>`: Título da página exposto na aba do navegador.
- `<base>`: URL base para resolução de links relativos.
- `<link>`: Folhas de estilo externas (`rel="stylesheet"`), preloads (`rel="preload"`) e favicons carregados via `ace_net`.
- `<meta>`: Codificação (`charset="utf-8"`), viewport e metadados HTTP-equiv.
- `<style>`: Folhas de estilo CSS inline vinculadas diretamente ao `StyleEngine` de `ace_style`.

### 2.2 Estrutura e Seções do Documento
- `<body>`: Corpo do documento principal.
- `<article>`, `<section>`, `<nav>`, `<aside>`: Seções semânticas.
- `<header>`, `<footer>`, `<main>`: Cabeçalhos, rodapés e área de conteúdo principal.
- `<h1>`, `<h2>`, `<h3>`, `<h4>`, `<h5>`, `<h6>`: Títulos de nível 1 a 6.
- `<address>`: Informações de contato.

### 2.3 Conteúdo em Bloco e Agrupamentos
- `<p>`: Parágrafos de texto com separação e colapso de margens no BFC.
- `<hr>`: Linha horizontal divisória.
- `<pre>`: Texto pré-formatado (preservação estrita de quebras de linha e espaços).
- `<blockquote>`: Citações em bloco.
- `<ol>`, `<ul>`, `<li>`: Listas ordenadas e não ordenadas com marcadores padrão.
- `<dl>`, `<dt>`, `<dd>`: Listas de descrição e glossários.
- `<figure>`, `<figcaption>`: Figuras com legenda.
- `<div>`: Contêiner genérico de fluxo em bloco.

### 2.4 Semântica Textual Inline
- `<a>`: Links de navegação com atributos `href`, `target`, `rel`, `download`.
- `<em>`, `<strong>`: Ênfase e relevância forte.
- `<small>`, `<s>`, `<cite>`, `<q>`: Pequeno, texto riscado e citações inline.
- `<code>`, `<var>`, `<samp>`, `<kbd>`: Fragmentos de código de computador e teclado.
- `<sub>`, `<sup>`: Texto subscrito e sobrescrito.
- `<i>`, `<b>`, `<u>`, `<mark>`: Estilos tipográficos tradicionais e destaque.
- `<span>`: Contêiner genérico inline.
- `<br>`, `<wbr>`: Quebras de linha manuais e oportunidades de quebra de palavra.

### 2.5 Mídia e Conteúdo Embutido
- `<img>`: Imagens bitmap e vetoriais (`src`, `alt`, `width`, `height`, `loading="lazy"`).
- `<picture>`, `<source>`: Imagens responsivas e seleção de fontes de mídia.
- `<svg>`: Elementos gráficos vetoriais inline básicos parseados no namespace SVG.

### 2.6 Tabelas
- `<table>`: Tabela de dados (suporte aos modos `InTable`, `InTableText`, `InTableBody`, `InRow`, `InCell`).
- `<caption>`: Título da tabela.
- `<colgroup>`, `<col>`: Agrupamento e definição de colunas.
- `<thead>`, `<tbody>`, `<tfoot>`: Cabeçalho, corpo e rodapé da tabela.
- `<tr>`: Linha da tabela.
- `<th>`, `<td>`: Células de cabeçalho e dados.

### 2.7 Formulários e Controles de Entrada
- `<form>`: Formulário de submissão (`action`, `method`, `enctype`).
- `<label>`: Rótulo de controle com associação via atributo `for`.
- `<input>`: Tipos essenciais suportados no MVP:
  - `type="text"`, `type="password"`, `type="email"`, `type="search"`, `type="url"`
  - `type="checkbox"`, `type="radio"`
  - `type="button"`, `type="submit"`, `type="reset"`, `type="hidden"`
- `<button>`: Botão interativo (`type="button"`, `type="submit"`).
- `<select>`, `<optgroup>`, `<option>`: Menus suspensos de seleção única com modo `InSelect`.
- `<textarea>`: Caixa de texto multilinha editável.
- `<fieldset>`, `<legend>`: Agrupamento de campos de formulário.

### 2.8 Componentes e Interatividade
- `<script>`: Scripts executáveis (`src`, `type="module"`, `async`, `defer`).
- `<noscript>`: Conteúdo de fallback para scripts desativados.
- `<template>`: Fragmentos de modelo inativos, com suporte nativo a **Declarative Shadow DOM** (`shadowrootmode="open|closed"`).
- `<slot>`: Pontos de inserção para componentes Shadow DOM.
- `<details>`, `<summary>`: Caixas de divulgação expansíveis nativas.
- `<dialog>`: Caixas de diálogo modais e não modais (`show()`, `showModal()`, `close()`).

---

## 3. Atributos Globais Suportados em Todos os Elementos

- `id`: Identificador único no documento com busca $O(1)$ via índice de arena.
- `class`: Lista de classes CSS separadas por espaço (aceleração por Bloom Filter).
- `style`: Declarações CSS inline com resolução dinâmica no `StyleEngine`.
- `title`: Texto de dica de ferramenta (*tooltip*).
- `lang`, `dir`: Idioma e direção de texto (`ltr`, `rtl`, `auto`).
- `hidden`: Ocultação nativa do elemento (`display: none`).
- `tabindex`: Ordem de foco de teclado e acessibilidade.
- `data-*`: Atributos de dados customizados acessíveis via `dataset`.
- `role`, `aria-*`: Atributos de semântica de acessibilidade mapeados para a Accessibility Tree.


