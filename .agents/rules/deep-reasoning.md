---
trigger: always_on
---

# Raciocínio Profundo e Pensamento em Camadas

## 1. Princípio: Pensar ANTES de agir
O agente **deve** raciocinar profundamente sobre cada problema antes de propor ou implementar qualquer solução. Código sem raciocínio prévio é código com bugs futuros. A qualidade do pensamento determina a qualidade do código.

## 2. Raciocínio em Cadeia (Chain of Thought)
Para qualquer problema não-trivial, o agente deve:

### 2.1. Decomposição lógica
- Quebrar o problema em passos lógicos explícitos e sequenciais.
- Cada passo deve ser verificável independentemente.
- Documentar o raciocínio: "Preciso de X porque Y, que depende de Z."

### 2.2. Múltiplas perspectivas
- Antes de escolher uma abordagem, considerar **no mínimo 2-3 alternativas**.
- Para cada alternativa, listar:
  - **Prós:** Vantagens técnicas e práticas.
  - **Contras:** Desvantagens, riscos, complexidade.
  - **Trade-offs:** O que se ganha e o que se perde.
- Justificar a escolha com base em evidências, não intuição.

### 2.3. Pensamento em camadas de abstração
O agente deve entender o problema em **múltiplos níveis**:
- **Nível conceitual:** O que estamos resolvendo? Qual o domínio?
- **Nível arquitetural:** Como se encaixa no sistema? Quais módulos são afetados?
- **Nível de implementação:** Qual a melhor forma de codificar?
- **Nível de detalhe:** Edge cases? Tipos? Lifetimes? Erros?

## 3. Técnicas de Raciocínio Obrigatórias

### 3.1. "Rubber Duck" Interno
Antes de implementar, o agente deve ser capaz de **explicar a solução em linguagem natural** de forma clara. Se não consegue explicar, não entendeu o suficiente.

### 3.2. Perguntas de Sócrates
Aplicar o método socrático ao próprio raciocínio:
- "**Por quê** estou fazendo dessa forma?"
- "**O que acontece se** essa suposição estiver errada?"
- "**Existe uma forma mais simples** de alcançar o mesmo resultado?"
- "**Quem mais** será afetado por essa mudança?"

### 3.3. Análise de Consequências de Segunda e Terceira Ordem
Não analisar apenas o efeito imediato de uma mudança:
- **1ª ordem:** O que muda diretamente? (o óbvio)
- **2ª ordem:** O que muda como consequência da mudança? (o indireto)
- **3ª ordem:** Que efeitos emergem da combinação das mudanças? (o não-óbvio)

### 3.4. Inversão Mental
Pensar no problema ao contrário:
- "O que faria essa solução **falhar**?"
- "Como um atacante **abusaria** dessa interface?"
- "Qual é o **pior cenário possível**?"

## 4. Quando aplicar raciocínio profundo

| Situação | Nível de Raciocínio |
|----------|---------------------|
| Mudança trivial (typo, formatação) | Mínimo — verificar apenas o impacto |
| Bug fix simples | Moderado — entender causa raiz |
| Nova feature | Profundo — múltiplas perspectivas obrigatórias |
| Mudança arquitetural | Máximo — análise completa em todos os níveis |
| Código concorrente/unsafe | Máximo — cada linha deve ser justificada |

## 5. Anti-padrões de raciocínio (PROIBIDOS)
- ❌ **Implementar antes de entender:** "Vou codificar e ver o que acontece."
- ❌ **Solução por andar de bêbado:** Tentativa e erro sem hipótese.
- ❌ **Viés de confirmação:** Buscar apenas evidências que suportam a primeira ideia.
- ❌ **Complexidade desnecessária:** Adicionar abstrações sem justificativa sólida.
- ❌ **Cargo cult:** Copiar padrões sem entender por quê existem.

## 6. Integração com outros workspaces
- **understand-task:** O raciocínio profundo é aplicado durante o entendimento da tarefa.
- **plan-mode:** O plano de implementação é o produto do raciocínio profundo.
- **failure-analysis:** Análise de falhas é uma forma especializada de raciocínio profundo.
- **evidence-based:** Raciocínio deve ser fundamentado em evidências.
