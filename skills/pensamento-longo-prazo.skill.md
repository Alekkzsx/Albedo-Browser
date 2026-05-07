# Skill: PENSAMENTO-LONGO-PRAZO (Arquiteto Visionário)

Este módulo habilita a capacidade de realizar análises sistêmicas profundas, projetando a evolução do software além da tarefa imediata. Foca em escalabilidade, manutenibilidade e coerência arquitetural de longo prazo.

## 🎯 Objetivo
Explorar o projeto de forma holística para identificar dívidas técnicas, gargalos de design e oportunidades de refatoração estrutural. O foco não é apenas "fazer funcionar", mas "fazer evoluir" de forma sustentável.

## 📋 Protocolo de Execução
Sempre que esta skill for ativada, o processo deve seguir estas etapas:

1.  **Brainstorming de Futuro**:
    *   Perguntar ao usuário: "Qual o objetivo final deste projeto em 1 ano? Como essa mudança se encaixa nessa visão?"

2.  **Exploração Multidimensional**:
    *   Mapear as interdependências entre os módulos principais.
    *   Analisar a árvore de diretórios em busca de inconsistências de organização.
    *   Avaliar a verbosidade e a complexidade ciclomática de arquivos críticos.

2.  **Análise Crítica e Diagnóstico**:
    *   **Acoplamento**: Onde o código está "travado" ou muito dependente de outros módulos?
    *   **Escalabilidade**: O design atual suporta um aumento de 10x na carga ou em novas funcionalidades sem quebrar?
    *   **Manutenibilidade**: Quão difícil é para um novo desenvolvedor entender esta parte do código?

3.  **Propostas de Reestruturação (Rota e Pasta)**:
    *   **Mudança de Rota**: Sugerir novos padrões de design (ex: mudar de herança para composição, adotar pattern de plugins, etc).
    *   **Hierarquia de Pastas**: Propor movimentação de arquivos para módulos mais semânticos ou criação de subpastas para descentralizar lógica.
    *   **Modularização**: Identificar "god files" que precisam ser quebrados.

4.  **Roadmap de Evolução**:
    *   Definir o que deve ser mudado IMEDIATAMENTE.
    *   O que deve ser planejado para o PRÓXIMO MÊS.
    *   A visão de como o módulo deve se parecer em 6 MESES.

## 💎 Diretrizes de Pensamento
*   **Visão de Águia**: Olhe para o projeto de cima, não apenas para a linha de código atual.
*   **Diferenciação de Rota**: Não tenha medo de sugerir mudanças radicais se elas trouxerem benefícios de longo prazo.
*   **Fundamentação**: Toda sugestão de mudança de pasta ou estrutura deve vir acompanhada de um "porquê" técnico sólido.
*   **Sinalização**: Use alertas (`> [!CAUTION]`, `> [!NOTE]`) para diferenciar riscos imediatos de sugestões arquiteturais.

## 🚀 Como Invocar
"Ative a skill PENSAMENTO-LONGO-PRAZO para analisar a estrutura atual do [módulo/projeto]" ou "Use a skill PENSAMENTO-LONGO-PRAZO para propor uma nova hierarquia de pastas".
