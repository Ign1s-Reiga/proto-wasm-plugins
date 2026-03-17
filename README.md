# proto-wasm-plugins

WASM plugins for [proto](https://github.com/moonrepo/proto).

## Plugin List

| Project Name  | Project Path         | Plugin ID |
|---------------|----------------------|-----------|
| wrangler_tool | crates/wrangler_tool | wrangler  |

## Installation

Via CLI
```bash
# wrangler
proto plugin add <plugin-id> "github://Ign1s-Reiga/proto-wasm-plugins/<project-name>"
proto install <plugin-id>
```

Manual Configuration (.prototools)
```
[plugins.tools]
<plugin-id> = "github://Ign1s-Reiga/proto-wasm-plugins/<project-name>"
```
