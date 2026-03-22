---
trigger: always_on
---

# Segurança em Primeiro Lugar

## 1. Princípio: Um browser é uma superfície de ataque
O AlbedoBrowser executa código arbitrário da internet, renderiza content não-confiável e acessa a rede. Segurança não é um recurso — é um **requisito fundamental**. Todo código deve ser escrito com a mentalidade de que input externo é potencialmente malicioso.

## 2. Modelo de Ameaças para Browser

### 2.1. Superfícies de ataque
- **HTML/CSS/JS malicioso:** Páginas web adversariais.
- **Rede:** MITM, DNS spoofing, certificados inválidos.
- **Plugins e extensões:** Código de terceiros com acesso ao sistema.
- **URLs:** Scheme injection, path traversal, open redirect.
- **Recursos locais:** Acesso ao filesystem, cookies, credenciais.

### 2.2. Atores de ameaça
- Páginas web maliciosas tentando explorar o browser.
- Scripts injetados (XSS) tentando roubar dados.
- Ataques de rede (interceptação, manipulação de tráfego).

## 3. Princípios de Segurança

### 3.1. Princípio do menor privilégio
- Cada componente/processo deve ter acesso ao **mínimo necessário**.
- Processos de rendering não devem acessar filesystem.
- Scripts JS não devem acessar dados de outras origins.

### 3.2. Defense in depth
- Múltiplas camadas de defesa — nunca confiar em uma única barreira.
- Input validation + sanitization + output encoding + sandboxing.
- Se uma camada falhar, a próxima deve proteger.

### 3.3. Fail secure
- Em caso de dúvida, **negar** acesso (default deny).
- Erros de segurança devem falhar de forma segura, não aberta.
- Nunca expor informações sensíveis em mensagens de erro.

## 4. Validação de Input

### 4.1. Toda entrada é suspeita
- **HTML:** Sanitizar antes de processar (tags, atributos, entities).
- **URLs:** Validar scheme, host, path. Bloquear schemes perigosos (`javascript:`, `data:`).
- **Headers HTTP:** Verificar tamanhos, encoding, valores inesperados.
- **Cookies:** Validar flags (Secure, HttpOnly, SameSite).

### 4.2. Limites de tamanho
- **TODA** entrada deve ter limite máximo de tamanho.
- URLs: limitar comprimento (ex.: 2048 caracteres).
- Headers: limitar tamanho total e individual.
- Response body: limitar para prevenir exhaustão de memória.


## 5. Web Security Policies

### 5.1. Same-Origin Policy (SOP)
- Isolar dados entre diferentes origins (scheme + host + port).
- Scripts de uma origin NÃO podem acessar dados de outra origin.
- Implementar CORS (Cross-Origin Resource Sharing) corretamente.

### 5.2. Content Security Policy (CSP)
- Respeitar headers CSP de páginas carregadas.
- Bloquear recursos que violam a política declarada.
- Reportar violações quando configurado.

### 5.3. Outras políticas
- **HTTPS:** Preferir conexões seguras. Indicar visualmente HTTP inseguro.
- **HSTS:** Respeitar Strict-Transport-Security headers.
- **X-Frame-Options / frame-ancestors:** Prevenir clickjacking.
- **Referrer-Policy:** Controlar informação no header Referer.

## 6. Proteções Específicas

### 6.1. Cross-Site Scripting (XSS)
- Sanitizar todo output inserido no DOM.
- Encoding contextual (HTML, JS, URL, CSS).
- Nunca inserir input do usuário diretamente em innerHTML.

### 6.2. Injeção
- Parametrizar queries (se aplicável).
- Escapar dados antes de inserir em contextos interpretativos.
- Validar encoding de caracteres (UTF-8 estrita).

### 6.3. Path Traversal
- Normalizar paths antes de usar.
- Bloquear `..` e sequences de escape em paths de arquivo.
- Usar allowlists para diretórios acessíveis.

## 7. Auditoria e Monitoramento

### 7.1. Cargo audit
- Executar `cargo audit` periodicamente para detectar vulnerabilidades em dependências.
- Atualizar dependências com CVEs conhecidos imediatamente.
- Preferir crates com histórico de segurança comprovado.

### 7.2. Secrets
- **NUNCA** hardcodar secrets, tokens ou credenciais no código.
- **NUNCA** logar informações sensíveis (cookies, tokens, senhas).
- Usar variáveis de ambiente ou secure storage para secrets.

## 8. Anti-padrões de segurança (PROIBIDOS)
- ❌ **Security through obscurity:** Depender de atacante não conhecer o código.
- ❌ **Trust by default:** Confiar em input externo sem validar.
- ❌ **Hardcoded secrets:** Tokens, senhas ou chaves no código-fonte.
- ❌ **Error info leak:** Expor stack traces, paths ou versões em erros.
- ❌ **Disabled security features:** Desabilitar TLS verification "para facilitar".
- ❌ **Eval de strings:** Executar strings como código sem sanitização.

## 9. Integração com outros workspaces
- **defensive-programming:** Validação de input é a base da segurança.
- **error-handling:** Erros não devem vazar informações sensíveis.
- **failure-analysis:** Ameaças de segurança são modos de falha.
- **review-full:** A revisão deve incluir aspecto de segurança.
