# 🛡️ Central de Segurança — Albedo Browser & Engine

> **Diretório:** `docs/security/`  
> **Status:** Ativo & Vigente  
> **Propósito:** Centralizar a documentação de segurança ofensiva e defensiva do Albedo Browser, incluindo modelos de ameaças, inventário de blocos `unsafe` e diretrizes de auditoria.

---

## 📜 Histórico de Governança de Segurança

| Versão | Data | Contexto / Marco | Decisões & Escolhas de Segurança | Progresso & Mudanças |
| :---: | :---: | :--- | :--- | :--- |
| **1.0.0** | 2026-10-07 | Auditoria e Formalização v7.0 | • Criação do Registro Central de Unsafe (`unsafe-registry.md`) catalogando 64 ocorrências reais.<br>• Definição do Modelo Formal de Ameaças Multi-Processo (`threat-model.md`).<br>• Obrigação de Miri no CI para módulos com código inseguro. | • Centralização de segurança na pasta dedicada `docs/security/`. |

---

## 🧭 Documentos de Segurança

| Documento | Foco | O que Detalha |
| :--- | :--- | :--- |
| [**`unsafe-registry.md`**](./unsafe-registry.md) | **Registro de `unsafe`** | Inventário exaustivo de cada bloco `unsafe` existente no workspace, a invariante que protege a memória e o plano de teste sob Miri/Loom. |
| [**`threat-model.md`**](./threat-model.md) | **Modelo de Ameaças** | Análise STRIDE das fronteiras de segurança entre o Processo do Browser, os Processos de Renderização isolados por site e a camada de Rede. |

---

## 🔒 Princípios de Segurança Inegociáveis

1. **Princípio do Menor Privilégio:** Todo código de renderização de páginas web (HTML, CSS, JS) executa em processos sem privilégios de sistema operacional (sem acesso a arquivos locais, sem acesso a periféricos).
2. **Defesa em Profundidade:**
   - Camada 1: *Type Safety* e *Borrow Checker* do Rust em 99%+ da base de código.
   - Camada 2: Políticas web nativas rigorosas (SOP, CORS, CSP3, HSTS, CHIPS).
   - Camada 3: *Site Isolation* (isolamento físico por processo no SO para cada eTLD+1).
   - Camada 4: *Sandboxing* de sistema operacional (AppContainers no Windows, seccomp-bpf no Linux).
3. **Modo *Jitless* Permanente:** O motor JavaScript do Albedo adota o interpretador de bytecode sem compilação dinâmica em tempo de execução como padrão seguro, eliminando a principal fonte histórica de exploits zero-day da web.

