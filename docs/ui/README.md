# op-web UI

Embedded React SPA for op-dbus management.

## Development

```bash
cd crates/op-web/ui
npm install
npm run dev    # Starts Vite dev server with proxy to :8080
```

Dev server proxies `/api/*` and `/ws` to `localhost:8080`.

## Production Build

```bash
# Build UI (outputs to ui/dist/)
cd crates/op-web/ui
npm run build

# Build Rust binary with embedded UI
cd ../..  # back to workspace root
cargo build -p op-web --release
```

The UI is embedded via `rust-embed` - single binary deployment.

## Architecture

```
ui/
├── src/
│   ├── api/          # REST, WebSocket, gRPC clients
│   ├── components/   # Reusable UI components
│   │   ├── layout/   # AppShell, Sidebar, Header
│   │   ├── data/     # DataTable, VirtualList, PayloadViewer
│   │   ├── form/     # SearchInput, ConfirmModal
│   │   ├── security/ # RBACGate, QuotaMeter
│   │   ├── viz/      # MetricChart
│   │   └── chat/     # ChatPanel, ChatMessage, ChatInput
│   ├── pages/        # Route pages
│   ├── stores/       # Zustand state (auth, quota, ui)
│   └── wasm/         # Optional WASM decoder
├── wasm/decoder/     # Rust WASM project (optional)
└── dist/             # Build output (embedded by Rust)
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health` | GET | Health check |
| `/api/chat` | POST | Send chat message |
| `/api/tools` | GET | List tools |
| `/api/tools/:name` | GET | Get tool details |
| `/api/tools/:name/execute` | POST | Execute tool |
| `/api/llm/status` | GET | LLM provider status |
| `/api/llm/models` | GET | Available models |
| `/ws` | WS | Real-time updates |

## WASM Decoder (Optional)

For faster JSON parsing, build the WASM decoder:

```bash
cd ui/wasm/decoder
wasm-pack build --target web --out-dir ../../src/wasm/pkg
```

Falls back to JS `JSON.parse` if WASM not available.

## Troubleshooting

**UI not loading**: Ensure `ui/dist/` exists. Run `npm run build` in `ui/`.

**API errors**: Check backend is running on port 8080.

**Hot reload not working**: Vite dev server must be running (`npm run dev`).
