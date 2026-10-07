# 🏷️ Catálogo Normativo de Elementos HTML5 no MVP (v1.0)

> **Documento Normativo de Escopo HTML**  
> **Subsistema Alvo:** `Albedo_Core_Engine/ace_dom`  
> **Status:** Vigente (Alinhado com o [PLANO.md v7.0](../../PLANO.md) — §1.4 e Fase 5)  
> **Padrões Normativos:** WHATWG HTML Living Standard (§4 The elements of HTML).

---

## 📜 Histórico de Revisões e Decisões

| Versão | Data | Contexto / Marco | Decisões & Escolhas Arquiteturais | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **1.0.0** | 2026-10-07 | Catalogação Normativa v7.0 | • Definição do conjunto taxativo de tags e atributos parseados na árvore DOM na versão 1.0 desktop.<br>• Suporte nativo a Declarative Shadow DOM (`<template shadowrootmode>`). | • Documento criado para fechar a tríade de escopo (HTML + CSS + Web APIs). |

---

## 1. Diretriz do Parser HTML5

Todo elemento listado abaixo é reconhecido pelo **Tokenizer HTML5** e estruturado pelo **Tree Builder** do `ace_dom` na `Arena<NodeData>`. Elementos desconhecidos ou personalizados (*Custom Elements*) são instanciados como `HTMLUnknownElement` ou genericamente como nós de elemento sem quebrar a árvore.

---

## 2. Elementos Suportados por Categoria

### 2.1 Raiz e Metadados do Documento
- `<html>`: Elemento raiz do documento.
- `<head>`: Contêiner de metadados.
- `<title>`: Título da página exposto na aba do navegador.
- `<base>`: URL base para resolução de links relativos.
- `<link>`: Folhas de estilo externas (`rel="stylesheet"`), preloads (`rel="preload"`) e favicons.
- `<meta>`: Codificação (`charset="utf-8"`), viewport e metadados HTTP-equiv.
- `<style>`: Folhas de estilo CSS inline vinculadas ao CSSOM.

### 2.2 Estrutura e Seções do Documento
- `<body>`: Corpo do documento principal.
- `<article>`, `<section>`, `<nav>`, `<aside>`: Seções semânticas.
- `<header>`, `<footer>`, `<main>`: Cabeçalhos, rodapés e área de conteúdo principal.
- `<h1>`, `<h2>`, `<h3>`, `<h4>`, `<h5>`, `<h6>`: Títulos de nível 1 a 6.
- `<address>`: Informações de contato.

### 2.3 Conteúdo em Bloco e Agrupamentos
- `<p>`: Parágrafos de texto com separação BFC.
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
- `<picture>`, `<source>`: Imagens responsivas e direção de arte.
- `<svg>`: Elementos gráficos vetoriais inline básicos parseados no namespace SVG.

### 2.6 Tabelas
- `<table>`: Tabela de dados.
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
- `<select>`, `<optgroup>`, `<option>`: Menus suspensos de seleção única.
- `<textarea>`: Caixa de texto multilinha editável.
- `<fieldset>`, `<legend>`: Agrupamento de campos de formulário.

### 2.8 Componentes e Interatividade
- `<script>`: Scripts executáveis (`src`, `type="module"`, `async`, `defer`).
- `<noscript>`: Conteúdo de fallback para scripts desativados.
- `<template>`: Fragmentos de modelo inativos, com suporte a **Declarative Shadow DOM** (`shadowrootmode="open|closed"`).
- `<slot>`: Pontos de inserção para componentes Shadow DOM.
- `<details>`, `<summary>`: Caixas de divulgação expansíveis nativas.
- `<dialog>`: Caixas de diálogo modais e não modais (`show()`, `showModal()`, `close()`).

---

## 3. Atributos Globais Suportados em Todos os Elementos

- `id`: Identificador único no documento.
- `class`: Lista de classes CSS separadas por espaço.
- `style`: Declarações CSS inline com resolução no CSSOM.
- `title`: Texto de dica de ferramenta (*tooltip*).
- `lang`, `dir`: Idioma e direção de texto (`ltr`, `rtl`, `auto`).
- `hidden`: Ocultação nativa do elemento (`display: none`).
- `tabindex`: Ordem de foco de teclado e acessibilidade.
- `data-*`: Atributos de dados customizados acessíveis via `dataset`.
- `role`, `aria-*`: Atributos de semântica de acessibilidade mapeados para a Accessibility Tree.

