---
description: Implementação de Funcionalidades com Maestria Absoluta
---
Abaixo estão os passos obrigatórios para implementar qualquer nova funcionalidade no AlbedoBrowser seguindo o Protocolo de Maestria v3.0.

1. **Fase de Análise Profunda**
   - Ler o arquivo principal (`mod.rs` do componente).
   - Identificar dependências e possíveis efeitos colaterais.
   - Se for WebAPI, ler o padrão em [https://html.spec.whatwg.org/](https://html.spec.whatwg.org/).

2. **Criação do Plano de Implementação**
   - Criar `implementation_plan.md` no diretório de artifacts.
   - Listar todos os arquivos a serem modificados.
   - Definir a estratégia de testes ANTES de codar.

3. **Início da Execução Rigorosa**
   - Atualizar `task.md`.
   - Implementar o código seguindo as regras de Rust (Zero Unsafe, Zero Clone, Zero blocking).

4. **Ciclo de Verificação de Integridade**
   // turbo
   - Rodar `cargo check` e garantir ZERO avisos.
   // turbo
   - Rodar `cargo fmt`.
   - Rodar testes unitários relacionados.

5. **Finalização e Cadastro de Conhecimento**
   - Criar um Knowledge Item (KI) se houver decisão de arquitetura complexa.
   - Notificar o usuário com o walkthrough completo.
