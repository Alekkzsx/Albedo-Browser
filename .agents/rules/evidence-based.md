---
trigger: always_on
---

# Decisões Baseadas em Evidências

## 1. Princípio: Nunca assumir — sempre verificar
O agente **deve** fundamentar toda decisão, sugestão e implementação em **evidências concretas**: código lido, testes executados, métricas coletadas, specs consultadas. Suposições são a fonte número um de bugs. Evidências são o antídoto.

## 2. Hierarquia de Evidências

A menor evidência concreta vale mais que a maior intuição. Ordem de prioridade:
1. **Código-fonte:** O que está realmente escrito no projeto (leitura direta).
2. **Testes exitentes:** O que os testes verificam (execução real).
3. **Compilação:** O que o compilador aceita/rejeita (`cargo check`).
4. **Documentação oficial:** Specs, docs de libs, RFCs.
5. **Implementações de referência:** Servo, Chromium, Firefox, V8.
6. **Experiência/intuição:** Último recurso, e sempre verificada.

## 3. Comportamentos Obrigatórios

### 3.1. Ler antes de opinar
- **NUNCA** sugerir mudanças em código que não foi lido.
- **NUNCA** afirmar que algo funciona sem ter verificado (compilação ou teste).
- **SEMPRE** ler o módulo completo antes de alterar uma função nele.
- **SEMPRE** ler os testes existentes antes de propor novos.

### 3.2. Testar hipóteses
- Toda afirmação sobre comportamento do código deve ser verificável.
- Se afirmar "isso vai funcionar", executar `cargo check` ou `cargo test` para provar.
- Se afirmar "isso não afeta X", executar os testes de X para provar.

### 3.3. Reproduzir antes de corrigir
Para qualquer bug:
1. **Reproduzir** o bug com um teste ou execução.
2. **Confirmar** que o teste falha sem a correção.
3. **Implementar** a correção.
4. **Verificar** que o teste passa com a correção.

### 3.4. Citar fontes
Em planos de implementação e decisões de design:
- Incluir links para specs WHATWG/W3C quando implementando WebAPIs.
- Referenciar commits, issues ou KIs relevantes.
- Mencionar exemplos de código existente quando justificando um padrão.

### 3.5. Dados sobre intuição
Para decisões de performance:
- **NUNCA** otimizar baseado em "acho que isso é lento".
- **SEMPRE** usar benchmarks, profiling ou métricas antes de otimizar.
- **SEMPRE** medir o impacto da otimização depois de implementar.

## 4. Processo de Verificação de Evidências

### 4.1. Antes de implementar
- [ ] Li o código-fonte que será afetado?
- [ ] Li os testes existentes dessa área?
- [ ] Consultei a documentação/spec relevante?
- [ ] Verifiquei exemplos existentes no projeto?

### 4.2. Depois de implementar
- [ ] O código compila sem warnings? (`cargo check`)
- [ ] Todos os testes existentes passam? (`cargo test`)
- [ ] O linter está satisfeito? (`cargo clippy`)
- [ ] A formatação está correta? (`cargo fmt --check`)

### 4.3. Ao afirmar algo
- [ ] Posso apontar o código/teste que comprova minha afirmação?
- [ ] Se não, posso verificar antes de afirmar?

## 5. Anti-padrões de evidência (PROIBIDOS)
- ❌ **Afirmação sem verificação:** "Isso certamente funciona" sem testar.
- ❌ **Memória sobre leitura:** "Acho que esse módulo faz X" sem reler.
- ❌ **Otimização por intuição:** "Esse loop é lento" sem profiling.
- ❌ **Copy-paste sem entendimento:** Copiar código sem verificar que se aplica.
- ❌ **Confiança cega em docs:** Docs podem estar desatualizadas — verificar no código.

## 6. Integração com outros workspaces
- **understand-first:** A análise inicial é um exercício de coleta de evidências.
- **deep-reasoning:** Raciocínio profundo deve ser baseado em fatos, não suposições.
- **review-full:** A revisão valida que as evidências suportam a implementação.
- **smart-automation:** As ferramentas automatizadas são fontes objetivas de evidência.
