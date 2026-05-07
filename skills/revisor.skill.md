# Skill: REVISOR (Garantia de Qualidade e Segurança)

Este módulo habilita a capacidade de revisão crítica de código, focando em segurança, performance e aderência a boas práticas (Clean Code).

## 🎯 Objetivo
Analisar alterações propostas ou códigos existentes para identificar vulnerabilidades, gargalos de performance e inconsistências arquiteturais antes da integração final.

## 🔍 Checklist de Revisão
Sempre que esta skill for ativada, a análise deve cobrir:

1.  **Segurança**:
    *   Validação de inputs e proteção contra injeção.
    *   Gerenciamento seguro de memória (especialmente em Rust/C++, se aplicável).
    *   Exposição acidental de chaves ou segredos.

2.  **Performance**:
    *   Complexidade de algoritmos (Big O).
    *   Uso excessivo de recursos (CPU/RAM).
    *   Operações de I/O desnecessárias.

3.  **Manutenibilidade**:
    *   Clareza de nomes de variáveis e funções.
    *   Princípio de Responsabilidade Única (SRP).
    *   Documentação de trechos complexos.

4.  **Aderência ao Projeto**:
    *   O código segue o padrão já estabelecido no repositório?

## 💎 Formato da Resposta
A revisão deve ser entregue em formato de tabela ou lista de tópicos:
*   **Problema**: Descrição sucinta.
*   **Gravidade**: [Baixa/Média/Alta/Crítica].
*   **Sugestão de Correção**: Snippet de código ou explicação técnica.

## 🚀 Como Invocar
"Use a skill REVISOR para analisar este arquivo" ou "Revise estas mudanças com a skill REVISOR".
