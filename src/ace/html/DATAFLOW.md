# ACE HTML Parser - Data Flow Diagrams

This document formalizes the data flow for ACE-HTML parsing paths, memory behavior, token movement, and error propagation.

## 1. Single-Threaded Parsing Flow

```mermaid
flowchart TD
    A[Input HTML bytes/string] --> B[Encoding detection/normalization]
    B --> C[Lexer state machine]
    C --> D[Tokenizer]
    D --> E[Tree Builder insertion modes]
    E --> F[DOM tree output]
    E --> G[Parse errors collection]
    D --> G
    C --> G
```

## 2. Multi-Threaded Speculative Flow

```mermaid
flowchart LR
    A[Input HTML] --> B[Speculative tokenizer thread]
    A --> C[Preload scanner thread pool]
    B --> D[Bounded token channel]
    D --> E[Tree Builder consumer]
    C --> F[Preload request queue]
    E --> G[DOM output]
    E --> H[Parse errors]
```

## 3. Memory Allocation and Reuse Flow

```mermaid
flowchart TD
    A[Parser start] --> B[NodeArena initialize chunk]
    B --> C[Allocate nodes in chunks]
    C --> D[StringInterner lookup/intern]
    D --> E[Build DOM + metadata]
    E --> F[Emit stats]
    F --> G[Clear/reuse arena for next parse]
```

## 4. Token Flow Patterns

```mermaid
flowchart TD
    A[Character stream] --> B{Lexer state}
    B -->|Tag path| C[Start/End tag token]
    B -->|Text path| D[Character token]
    B -->|Special path| E[Comment/Doctype token]
    C --> F[Tokenizer output stream]
    D --> F
    E --> F
    F --> G[Tree Builder mode dispatch]
```

## 5. Error Propagation Paths

```mermaid
flowchart TD
    A[Lexer recoverable error] --> D[Unified parse error list]
    B[Tokenizer recoverable error] --> D
    C[Tree builder recoverable error] --> D
    D --> E[Result.parse_errors]
    D --> F[Conformance/bench reports]
```

## Notes

- Errors are collected and reported without aborting parsing, following WHATWG recovery behavior.
- In speculative mode, bounded channels enforce backpressure and avoid unbounded memory growth.
- Arena/interner stats are consumed by integrated reports and validation tooling.

