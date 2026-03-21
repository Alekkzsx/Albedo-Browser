# AlbedoBrowser: Absolute Mastery Protocol (v3.0)

Este diretório contém as instruções permanentes para o desenvolvimento de elite do AlbedoBrowser.

## 1. Filosofia de Desenvolvimento e Comunicação
- **Perfeição sobre Rapidez**: Prefira gastar mais tempo planejando do que corrigindo erros evitáveis.
- **Brainstorming Obrigatório**: Antes de qualquer implementação, realize uma sessão de brainstorm com o usuário para explorar alternativas e riscos.
- **Sempre em PT-BR**: Todas as respostas, explicações e interações devem ser estritamente em Português do Brasil.
- **Confirmação de Direção**: Nunca assuma; pergunte sempre para confirmar informações ambíguas ou para validar o próximo passo.

## 2. Protocolo de Ação e Revisão
1. **Análise de Contexto**: Ler todos os arquivos relacionados antes de qualquer edição.
2. **Brainstorm & Confirmação**: Sugerir ideias, solicitar feedback e confirmar o plano antes de agir.
3. **Plano de Implementação**: Gerar um `implementation_plan.md` para toda tarefa não trivial.
4. **Revisão Final**: Antes de entregar, revisar mentalmente (e via ferramentas) se o código atende a todos os requisitos e se não introduziu regressões.
5. **Verificação Dupla**: Rodar `cargo check`, `cargo fmt` e testes unitários.

## 3. Estrutura de Documentação
- Commits atômicos e bem descritos.
- Documentação `///` obrigatória para toda API pública.
- Decisões de arquitetura registradas como Knowledge Items (KIs).
