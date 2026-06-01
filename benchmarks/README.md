# 📁 benchmarks/

Esta pasta contém suítes de testes de estresse e performance em HTML para validar o comportamento da engine ACE e do interpretador/JIT JavaScript.

## 🎯 Objetivo & Função
Os arquivos nesta pasta servem para realizar testes práticos de renderização e execução de scripts na engine ACE. Eles simulam cenários complexos (loops intensos em JS, tabelas gigantes com colspan, validações de formulário, manipulação intensa do DOM) e ajudam a medir a performance do pipeline gráfico, tempos de layout do Taffy e a velocidade de execução do AlbedoJIT comparado ao QuickJS interpretado.

## 📄 Arquivos e Suas Funções

| Arquivo | Função / Propósito |
| :--- | :--- |
| [bench_runner.html](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/benchmarks/bench_runner.html) | Página HTML que executa benchmarks de performance de micro-operações (laços, operações matemáticas, criação de objetos) medindo tempo de execução em milissegundos. |
| [precisely_bench.html](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/benchmarks/precisely_bench.html) | Executa suítes de benchmarks mais robustas e precisas comparando funções com recursão, manipulação matemática (fibonacci, factoriais) e acesso de propriedades no JIT. |
| [stress_test.html](file:///c:/Users/24802449/Documents/Github/Albedo-Browser/benchmarks/stress_test.html) | Documento HTML gigante contendo milhares de nós DOM de testes estruturados, tabelas complexas, flexbox alinhados e formulários para estressar e verificar vazamentos de memória (memory leaks) e fidelidade de layout do motor ACE. |

## 🛠️ O que DEVE e NÃO DEVE estar aqui (Regras de Design)

### O que DEVE estar aqui:
- Arquivos de benchmark em HTML5, CSS e JS puros voltados a testar o navegador de forma pontual ou em estresse.
- Scripts JS que medem e exibem tempos de processamento diretamente no console ou na tela.

### O que NÃO DEVE estar aqui:
- Lógica de compilação de código nativo ou crates adicionais (devem estar no workspace Rust).
- Códigos de testes unitários que devem rodar na suíte nativa de testes (`tests/`).
