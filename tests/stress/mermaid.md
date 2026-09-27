# Mermaid

## Flowchart top-down

```mermaid
graph TD
    A[Start] --> B{Is it working?}
    B -->|Yes| C[Ship it]
    B -->|No| D[Debug]
    D --> B
```

## Flowchart with subgraphs and edge styles

```mermaid
flowchart LR
    subgraph Frontend
        UI[Web UI] --> API
    end
    subgraph Backend
        API --> DB[(Database)]
        API -.-> Cache((Cache))
        API ==> Queue>Queue]
    end
```

## Sequence with notes and loops

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server
    C->>S: GET /items
    Note over S: validate token
    loop every page
        S-->>C: 200 page
    end
    C-xS: cancel
```

## State

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Running: start
    Running --> Idle: stop
    Running --> Failed: error
    Failed --> [*]
```

## Class (ASCII names)

```mermaid
classDiagram
    Animal <|-- Dog
    Animal <|-- Cat
    class Animal {
        +String name
        +speak()
    }
```

## Class with a non-ASCII name (crashes mermaid-text; must show the source)

```mermaid
classDiagram
    class Café
    Café <|-- Bistro
```

## Pie

```mermaid
pie title Pets adopted
    "Dogs" : 386
    "Cats" : 85
    "Rats" : 15
```

## Gantt

```mermaid
gantt
    title Release plan
    dateFormat YYYY-MM-DD
    section Build
    Math     :a1, 2026-01-01, 7d
    Diagrams :after a1, 10d
    section Ship
    Release  :2026-01-20, 2d
```

## Entity relationship

```mermaid
erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE_ITEM : contains
```

## Journey, mindmap, timeline, git graph

```mermaid
journey
    title Reading docs
    section Open
      Find file: 5: Me
      Read it: 3: Me
```

```mermaid
mindmap
  root((marustdown))
    Pager
    Math
    Diagrams
```

```mermaid
timeline
    title History
    2024 : idea
    2025 : prototype
    2026 : release
```

```mermaid
gitGraph
    commit
    branch feature
    commit
    checkout main
    merge feature
```

## Broken and odd input (all must show the source, no crash)

```mermaid
graph TD
    A[unclosed --> B{{{
```

```mermaid
notADiagramType
    whatever
```

```mermaid
```

```mermaid
%% only a comment
```

```MERMAID
graph LR
    upper --> case
```

## Big flowchart

```mermaid
graph TD
    N0[Node 0] --> N1[Node 1]
    N1[Node 1] --> N2[Node 2]
    N2[Node 2] --> N3[Node 3]
    N3[Node 3] --> N4[Node 4]
    N4[Node 4] --> N5[Node 5]
    N5[Node 5] --> N6[Node 6]
    N6[Node 6] --> N7[Node 7]
    N7[Node 7] --> N8[Node 8]
    N8[Node 8] --> N9[Node 9]
    N9[Node 9] --> N10[Node 10]
    N10[Node 10] --> N11[Node 11]
    N11[Node 11] --> N12[Node 12]
    N12[Node 12] --> N13[Node 13]
    N13[Node 13] --> N14[Node 14]
    N14[Node 14] --> N15[Node 15]
    N15[Node 15] --> N16[Node 16]
    N16[Node 16] --> N17[Node 17]
    N17[Node 17] --> N18[Node 18]
    N18[Node 18] --> N19[Node 19]
    N19[Node 19] --> N20[Node 20]
    N20[Node 20] --> N21[Node 21]
    N21[Node 21] --> N22[Node 22]
    N22[Node 22] --> N23[Node 23]
    N23[Node 23] --> N24[Node 24]
    N24[Node 24] --> N25[Node 25]
    N25[Node 25] --> N26[Node 26]
    N26[Node 26] --> N27[Node 27]
    N27[Node 27] --> N28[Node 28]
    N28[Node 28] --> N29[Node 29]
    N29[Node 29] --> N30[Node 30]
    N30[Node 30] --> N31[Node 31]
    N31[Node 31] --> N32[Node 32]
    N32[Node 32] --> N33[Node 33]
    N33[Node 33] --> N34[Node 34]
    N34[Node 34] --> N35[Node 35]
    N35[Node 35] --> N36[Node 36]
    N36[Node 36] --> N37[Node 37]
    N37[Node 37] --> N38[Node 38]
    N38[Node 38] --> N39[Node 39]
    N39[Node 39] --> N40[Node 40]
    N0 --> N0
    N3 --> N21
    N6 --> N2
    N9 --> N23
    N12 --> N4
    N15 --> N25
    N18 --> N6
    N21 --> N27
    N24 --> N8
    N27 --> N29
    N30 --> N10
    N33 --> N31
    N36 --> N12
    N39 --> N33
```
