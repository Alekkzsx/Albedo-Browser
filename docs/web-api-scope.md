# Escopo das APIs JavaScript no MVP

O motor `ace_js` e os bindings DOM do ACE focarão apenas no que é necessário para não quebrar SPAs simples na v1.0.

### Document & Element (DOM Core)
- `document.getElementById`, `document.querySelector`, `document.querySelectorAll`
- `document.createElement`, `document.createTextNode`
- `Node.appendChild`, `Node.removeChild`, `Node.replaceChild`, `Node.insertBefore`
- `Element.innerHTML`, `Element.textContent`, `Element.setAttribute`, `Element.getAttribute`
- Acesso à API de estilos em tempo de execução via `HTMLElement.style`.

### Eventos e Loop
- `EventTarget.addEventListener`, `EventTarget.removeEventListener`, `EventTarget.dispatchEvent`
- Principais eventos: `click`, `keydown`, `keyup`, `mousemove`, `input`.

### Temporizadores
- `setTimeout`, `clearTimeout`
- `setInterval`, `clearInterval`
- `requestAnimationFrame`

### Assincronismo e Rede
- Implementação estrita de `Promise` (microtask queue isolation).
- Suporte a chamadas `async` e `await` nativamente no Bytecode.
- API `fetch()` basilar (requisições GET/POST textuais ou JSON) acoplada ao sistema `ace_net`.

### Objetos Globais
- `console` (log, error, warn, trace).
- ECMAScript genérico (ES6 classes, objetos, arrays map/filter/reduce, Symbol, BigInt, WeakMap, WeakSet).

> [!WARNING]
> APIs pesadas como WebGL, Canvas2D, WebAudio e WebRTC retornarão erros explícitos no console no MVP (`NotSupportedError`).
