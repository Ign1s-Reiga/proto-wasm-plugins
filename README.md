# proto-plugin-wrangler

[Wrangler](https://github.com/cloudflare/workers-sdk/tree/main/packages/wrangler) WASM plugin for [proto](https://github.com/moonrepo/proto).

## Installation

Via CLI
```bash
proto plugin add wrangler "github://Ign1s-Reiga/proto-plugin-wrangler"
proto install wrangler
```

Manual Configuration (.prototools)
```toml
[plugins.tools]
wrangler = "github://Ign1s-Reiga/proto-plugin-wrangler"
```
