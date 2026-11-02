# Diagrams

Always use Mermaid. No PlantUML or other tools. Embed diagrams inline in any `.md` file where they help. Do not commit rendered files (SVG, PNG).

## Sequence diagrams

Every sequence diagram uses this frontmatter:

```yaml
---
title: <title>
config:
  theme: neutral
  sequence:
    useMaxWidth: true
    wrap: false
    mirrorActors: false
    showSequenceNumbers: true
---
```

To highlight a message or range, wrap it in a `rect rgb(...)` block (nesting is allowed):

```
rect rgb(220, 245, 220)
  A->>B: highlighted message
end
```

When the user says `mark number N as <colour>`, wrap the message with that rendered sequence number in a `rect` using the light shade below. For a range, wrap all of them in one `rect`.

| Colour | rgb |
|---|---|
| green | `rgb(220, 245, 220)` |
| red | `rgb(255, 230, 230)` |
| yellow | `rgb(255, 250, 205)` |
| blue | `rgb(220, 235, 255)` |
| orange | `rgb(255, 235, 205)` |
| grey | `rgb(235, 235, 235)` |

Mermaid draws `X->>X: ...` as a small curved loop that cannot become a box. For a rectangular internal action, use `note over X: ...` instead. The trade-off is that sequence numbering does not count notes.
