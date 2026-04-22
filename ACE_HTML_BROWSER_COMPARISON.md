# ACE-HTML Real Browser Comparison

## Escopo

Este relatório usa apenas medições reais feitas neste ambiente.

Sem inferência:
- `ace-html` foi medido localmente
- Chrome foi medido localmente
- Firefox foi medido localmente
- Brave **não foi medido**, porque não está instalado neste ambiente

Arquivos de evidência:
- [ace_html_benchmark_results.json](c:/Users/24802449/Documents/Github/Albedo-Browser/ace_html_benchmark_results.json)
- [browser_html_benchmark_results.json](c:/Users/24802449/Documents/Github/Albedo-Browser/browser_html_benchmark_results.json)

## Ambiente medido

- Chrome: `C:\Program Files\Google\Chrome\Application\chrome.exe`
- Firefox: `C:\Program Files\Mozilla Firefox\firefox.exe`
- Brave: não disponível

## Comandos usados

### ACE-HTML

```powershell
cargo run --bin ace_html_benchmark
cargo test --test ace_html_equivalence -- --nocapture
cargo test --test ace_html_final_validation -- --nocapture
cargo test --test html5lib_tree_harness ace_html_tree_construction_smoke -- --nocapture
cargo test --test html5lib_tokenizer_harness ace_html_required_conformance_passes_100 -- --nocapture
```

### Browsers reais

```powershell
python .codex-bench\browser_html_benchmark.py
```

Também foi executado o corpus completo `html5lib tree-construction` em Chrome e Firefox via Selenium.

---

## Comparação 1: Required Tree Cases

Dataset:
- `14` casos de árvore/fragmento do manifesto `tests/ace_html_conformance/required.json`

| Engine | Passou | Total | Taxa |
|---|---:|---:|---:|
| ACE-HTML | 4 | 14 | `28.57%` |
| Chrome | 13 | 14 | `92.86%` |
| Firefox | 13 | 14 | `92.86%` |
| Brave | N/D | N/D | N/D |

### Diferença observada

O `ace-html` falhou exatamente onde um parser real costuma ser mais sensível:
- adoption agency
- foster parenting
- namespace SVG
- fragment parsing em contexto de `table`
- frameset/head insertion

Chrome e Firefox erraram apenas `doc_template_element` neste serializer específico usado aqui.

### Veredito

No conjunto obrigatório de árvore, `ace-html` **não está comparável** a Chrome ou Firefox.

---

## Comparação 2: html5lib Tree Construction Sample

Dataset:
- primeiros `100` casos do corpus `tests/html5lib/tree-construction`

| Engine | Passou | Total | Taxa |
|---|---:|---:|---:|
| ACE-HTML | 0 | 100 | `0.00%` |
| Chrome | 99 | 100 | `99.00%` |
| Firefox | 100 | 100 | `100.00%` |
| Brave | N/D | N/D | N/D |

### Diferença observada

Neste recorte inicial do corpus, o `ace-html` falhou em todos os casos testados.

Os primeiros erros são fortemente concentrados em:
- adoption agency algorithm
- reconstrução e reparo de árvore
- reparenting implícito

### Veredito

No sample real do `html5lib`, o `ace-html` está **muito longe** de Chrome e Firefox.

---

## Comparação 3: html5lib Tree Construction Full Corpus

Dataset:
- `1498` casos do corpus `html5lib tree-construction`

### ACE-HTML

Resultado do harness Rust:

| Engine | Passou | Total | Taxa |
|---|---:|---:|---:|
| ACE-HTML | 20 | 1498 | `1.34%` |

### Browsers reais

Resultado do runner em browser real:

| Engine | Passou | Total | Taxa |
|---|---:|---:|---:|
| Chrome | 1328 | 1498 | `88.65%` |
| Firefox | 1307 | 1498 | `87.25%` |
| Brave | N/D | N/D | N/D |

### Leitura dos números

Gap absoluto para Chrome:
- `1328 - 20 = 1308` casos de diferença

Gap absoluto para Firefox:
- `1307 - 20 = 1287` casos de diferença

Gap percentual:
- ACE-HTML: `1.34%`
- Chrome: `88.65%`
- Firefox: `87.25%`

### Veredito

No corpus completo de tree construction, o `ace-html` **não é comparável** a Chrome e Firefox.

---

## Comparação 4: Throughput de Parsing

Dataset:
- documento HTML de `1,200,010` bytes
- benchmark real de parse repetido

| Engine | Tempo médio por parse | Throughput |
|---|---:|---:|
| ACE-HTML | `329.53 ms` | `3.64 MB/s` |
| Chrome | `14.11 ms` | `85.05 MB/s` |
| Firefox | `12.60 ms` | `95.24 MB/s` |
| Brave | N/D | N/D |

### Relação relativa

Chrome vs ACE-HTML:
- `85.05 / 3.64 = 23.37x` mais throughput

Firefox vs ACE-HTML:
- `95.24 / 3.64 = 26.16x` mais throughput

### Veredito

Em throughput real de parsing, o `ace-html` **não está comparável** a Chrome e Firefox.

---

## Observação Sobre Equivalence / Robustez

O `ace-html` continua indo bem em testes internos:

- `ace_html_equivalence`: `6/6`
- fuzz smoke: passou
- streaming `p99`: `100.2 µs`

Esses números mostram que o módulo está consistente e estável internamente.

Mas isso não compensa o gap de conformidade e performance frente a engines reais.

---

## Tabela Final

| Comparação | ACE-HTML | Chrome | Firefox | Brave |
|---|---:|---:|---:|---:|
| Required tree cases | `4/14` (`28.57%`) | `13/14` (`92.86%`) | `13/14` (`92.86%`) | N/D |
| html5lib sample | `0/100` (`0.00%`) | `99/100` (`99.00%`) | `100/100` (`100.00%`) | N/D |
| html5lib full corpus | `20/1498` (`1.34%`) | `1328/1498` (`88.65%`) | `1307/1498` (`87.25%`) | N/D |
| Throughput | `3.64 MB/s` | `85.05 MB/s` | `95.24 MB/s` | N/D |

## Conclusão

Com base em testes reais, números reais e corpus real:

- `ace-html` **não está comparável** a Chrome
- `ace-html` **não está comparável** a Firefox
- sobre Brave, **não há dado real coletado neste ambiente**

O diagnóstico objetivo hoje é:

1. Em conformidade de árvore, o gap é muito grande.
2. Em recuperação de markup inválido, o gap é muito grande.
3. Em throughput de parsing, o gap é grande.
4. Em consistência interna, o módulo está bem, mas isso ainda não o coloca no nível de browser real.

## Próximo passo técnico mais valioso

Se o objetivo é aproximar o `ace-html` de browsers reais, a prioridade prática é:

1. adoption agency algorithm
2. foster parenting
3. inserção implícita de `html/head/body`
4. parsing contextual de fragmentos (`table`, `select`, etc.)
5. foreign content / SVG / MathML
6. template handling
