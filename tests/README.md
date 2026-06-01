# 📁 tests/

Esta pasta contém a suíte completa de testes de integração, conformidade técnica e testes de regressão do **Albedo Browser**.

## 🎯 Objetivo & Função
Garantir que os componentes do Albedo (especialmente o parser HTML, o resolve de URLs, os seletores de CSS, o motor de Layout e o compilador JIT) funcionem exatamente como prescrevem as especificações W3C e WHATWG, prevenindo que refatorações ou otimizações introduzam bugs ou falhas de segurança (*regressions*).

## 📄 Arquivos e Suas Funções

| Pasta/Arquivo | Função / Propósito |
| :--- | :--- |
| [ace_html_conformance/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/ace_html_conformance) | Contém especificações em formato JSON (`required.json` e `non_required.json`) que mapeiam as tags e comportamentos que o parser do ACE-HTML deve cobrir. |
| [html5lib/](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/html5lib) | Pasta contendo suítes de teste oficiais do projeto `html5lib` para construção de árvores (tree construction) e tokenização. |
| [html5lib_tokenizer_harness.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/html5lib_tokenizer_harness.rs) | Harness de teste em Rust que lê os casos oficiais de tokenização do html5lib e valida o analisador léxico do ACE. |
| [html5lib_tree_harness.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/html5lib_tree_harness.rs) | Harness de teste que lê as árvores de DOM esperadas do html5lib e valida o Tree Builder do ACE. |
| [ace_html_equivalence.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/ace_html_equivalence.rs) | Valida a equivalência do parser ACE-HTML em relação a parsers maduros da comunidade Rust. |
| [ace_html_hard_cases.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/ace_html_hard_cases.rs) | Testes voltados a validar comportamentos complexos do parser (tags aninhadas incorretamente, Adoption Agency Algorithm, Foster Parenting). |
| [ace_html_proptests.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/ace_html_proptests.rs) | Testes baseados em propriedades (*property-based testing* via crate `proptest`) que enviam strings de HTML malformadas aleatórias para garantir que o parser nunca entre em pânico. |
| [ace_url_compliance_tests.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/ace_url_compliance_tests.rs) | Valida se o processador de URLs adere às especificações do WHATWG URL Standard (endereços IPv4/IPv6, normalizações de caminhos, etc.). |
| [form_validation_tests.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/form_validation_tests.rs) | Garante o funcionamento de validações de formulário nativas (restrições de required, min/max, pattern, etc.). |
| [table_colspan_tests.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/table_colspan_tests.rs) | Valida a corretude do layout bidimensional complexo de tabelas HTML com propriedades `colspan` e `rowspan`. |
| [osr_function_test.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/osr_function_test.rs) | Testes unitários para validar a compilação e execução correta do AlbedoJIT em funções de loop ativando OSR. |
| [osr_stress_test.rs](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/tests/osr_stress_test.rs) | Teste de estresse que força deopt e transições rápidas entre o interpretador Tier 0 e o JIT nativo milhares de vezes para garantir estabilidade da pilha. |

## 🛠️ O que DEVE e NÃO DEVE estar aqui (Regras de Design)

### O que DEVE estar aqui:
- Casos de testes de integração complexos que tocam em múltiplos componentes do navegador.
- Harnesses que carregam datasets de testes e arquivos JSON externos para validações oficiais de especificações.
- Testes baseados em propriedades ou fuzzing de integração.

### O que NÃO DEVE estar aqui:
- Testes unitários muito simples e acoplados a funções privadas (estes devem estar dentro dos próprios arquivos em módulos `#[cfg(test)] mod tests`).
- Código-fonte principal da aplicação.
- Benchmarks de desempenho puros (devem residir na pasta `benchmarks/`).
