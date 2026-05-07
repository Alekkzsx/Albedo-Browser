# Skill: EXPLORAÇÃO (Arquiteto Investigador)

Este módulo habilita a capacidade de realizar uma auditoria completa e profunda em um código-fonte desconhecido, mapeando sua estrutura, tecnologias e fluxos de dados.

## 🎯 Objetivo
Proporcionar uma visão 360º do projeto, permitindo que qualquer desenvolvedor (ou a própria IA) entenda rapidamente como o sistema funciona, onde estão as regras de negócio e como os módulos se comunicam.

## 🗺️ Protocolo de Exploração
Sempre que esta skill for ativada, a investigação deve seguir estas etapas:

1.  **Brainstorming de Foco**:
    *   Perguntar ao usuário se há algum módulo ou tecnologia específica que ele deseja priorizar na investigação.

2.  **Varredura Estrutural**:
    *   Listar diretórios raiz e identificar a finalidade de cada um.
    *   Mapear a "espinha dorsal" do projeto (pastas de código, testes, scripts e configs).

2.  **Identificação da Stack (DNA)**:
    *   Analisar arquivos de manifesto (`Cargo.toml`, `package.json`, `go.mod`, etc.).
    *   Listar as dependências principais e o que elas indicam sobre o projeto (ex: uso de JIT, motores gráficos, frameworks web).

3.  **Localização de Pontos de Entrada**:
    *   Encontrar o `main()`, `index` ou handlers iniciais.
    *   Rastrear o fluxo inicial desde o início da execução até a primeira interação do usuário.

4.  **Mapa de Dependências Internas**:
    *   Identificar como os módulos conversam (ex: "O módulo A importa o módulo B para gerenciar memória").
    *   Detectar padrões de design (Singleton, Factory, Observer, etc.).

5.  **Destaques de Complexidade**:
    *   Apontar arquivos que parecem ser o "coração" do projeto e que exigem mais cuidado ao serem modificados.

## 📊 Formato do Relatório de Exploração
O resultado deve ser entregue com:
*   **📂 Árvore de Diretórios Anotada**: Breve explicação ao lado de cada pasta importante.
*   **🛠️ Ficha Técnica**: Tabela com Linguagem, Frameworks, Build System e Target OS.
*   **💡 Insights do Investigador**: Observações sobre a qualidade do código ou pontos de melhoria óbvios encontrados durante a exploração.

## 🚀 Como Invocar
"Ative a skill EXPLORAÇÃO para este repositório" ou "Explore o projeto minuciosamente usando a skill EXPLORAÇÃO".
