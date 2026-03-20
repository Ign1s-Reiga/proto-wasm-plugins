# proto-wasm-plugins

WASM plugins for [proto](https://github.com/moonrepo/proto).

## Plugin List

| Project Name   | Project Path     | Plugin ID |
|----------------|------------------|-----------|
| wrangler_tool  | crates/wrangler  | wrangler  |
| vite_plus_tool | crates/vite-plus | vite-plus |

## Installation

Via CLI
```bash
proto plugin add <plugin-id> "github://Ign1s-Reiga/proto-wasm-plugins/<project-name>"
proto install <plugin-id>
```

Manual Configuration (.prototools)
```
[plugins.tools]
<plugin-id> = "github://Ign1s-Reiga/proto-wasm-plugins/<project-name>"
```
