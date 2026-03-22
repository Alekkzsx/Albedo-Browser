---
trigger: always_on
---

# Guardião de Arquitetura

## 1. Princípio: Arquitetura é lei, não sugestão
A arquitetura do sistema define as **boundaries**, **responsabilidades** e **direções de dependência** entre módulos. O agente **deve** respeitar e proteger essas decisões arquiteturais como se fossem lei constitucional do projeto. Violações arquiteturais são dívida técnica da pior espécie.

## 2. Regras Arquiteturais Fundamentais

### 2.1. Dependência unidirecional
- Camadas inferiores **NUNCA** importam camadas superiores.
- Direção correta: `app` → `services` → `core` → `primitives`.
- Se uma camada inferior precisa de algo da superior: **inversão de dependência** (trait).

### 2.2. Encapsulamento estrito
- Campos de structs: `pub(crate)` por padrão, `pub` apenas para API pública documentada.
- Módulos internos: `pub(crate)` ou privados — não expor internals.
- Re-exports no `mod.rs`/`lib.rs`: apenas itens que fazem parte da API pública.

### 2.3. Interface > Implementação
- Depender de **traits** (interfaces), não de structs concretas.
- Componentes devem ser substituíveis sem alterar consumidores.
- Isso habilita: testes com mocks, refatoração segura, extensibilidade.

### 2.4. Separação de Concerns
- Cada módulo tem **uma responsabilidade clara** e documentada.
- Se um módulo faz duas coisas, deve ser dividido.
- Nome do módulo deve comunicar sua responsabilidade.

## 3. Arquitetura do AlbedoBrowser

### 3.1. Pipeline principal
```
Network → Parser → DOM → CSS → Layout → Rendering → Display
                          ↕
                    JavaScript/JIT
```

### 3.2. Boundaries entre subsistemas
- **Network ↔ Parser:** Dados brutos (bytes/streams).
- **Parser ↔ DOM:** Árvore de nós estruturada.
- **DOM ↔ CSS:** Estilo computado por nó.
- **CSS ↔ Layout:** Box model e geometria.
- **Layout ↔ Rendering:** Display list e comandos de draw.
- **DOM ↔ JS/JIT:** Bindings de API (WebIDL).

### 3.3. Anti-corruption layers
- Cada boundary deve ter uma **camada de tradução** que impede que detalhes internos de um subsistema vazem para outro.
- Exemplos: converter representações internas de DOM para representações que JS espera.

## 4. Padrões de Design Obrigatórios

### 4.1. Command/Query Separation (CQS)
- **Queries** (getters): retornam dados, sem efeitos colaterais.
- **Commands** (setters): modificam estado, sem retornar dados significativos.
- Evitar funções que fazem ambos (exceto quando necessariamente atômico).

### 4.2. Injeção de Dependência
- Componentes **recebem** suas dependências, não as criam.
- Facilita testes: injetar mocks em vez de dependências reais.
- Em Rust: traits como parâmetros genéricos ou trait objects (`dyn Trait`).

### 4.3. Builder Pattern para configuração complexa
- Structs com 4+ parâmetros de configuração devem ter um Builder.
- Builders validam a configuração antes de construir o objeto final.

## 5. Verificação Arquitetural

### 5.1. Antes de toda mudança
- "Essa mudança **respeita** as boundaries existentes?"
- "Estou introduzindo uma **nova dependência** entre módulos? É justificada?"
- "Estou colocando código **no módulo correto** conforme a responsabilidade?"

### 5.2. Red flags arquiteturais
Parar e repensar se:
- Um módulo de baixo nível importa um de alto nível.
- Uma mudança simples requer tocar 5+ módulos (acoplamento).
- Um módulo tem 20+ imports (responsabilidade excessiva).
- Dois módulos fazem coisas similares (duplicação de concerns).

## 6. Anti-padrões arquiteturais (PROIBIDOS)
- ❌ **Circular dependencies:** Módulo A depende de B que depende de A.
- ❌ **God module:** Um módulo que faz tudo (500+ linhas, 20+ funções públicas).
- ❌ **Leaky abstraction:** Detalhes internos que vazam para consumidores.
- ❌ **Shotgun architecture:** Features espalhadas por muitos módulos sem coesão.
- ❌ **Spaghetti dependencies:** Grafo de dependência sem direção clara.
- ❌ **Feature envy:** Módulo que manipula mais dados de outro módulo que os próprios.

## 7. Integração com outros workspaces
- **understand-first:** A análise inicial mapeia a arquitetura do projeto.
- **pattern-consistency:** Padrões arquiteturais são a consistência de alto nível.
- **surgical-precision:** Mudanças devem respeitar boundaries — não atravessá-las.
- **refactoring-discipline:** Refatorações devem melhorar, não degradar a arquitetura.
