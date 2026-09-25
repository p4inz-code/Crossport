# Glossary

| Term | Meaning |
| --- | --- |
| AppConfig | Immutable application configuration in the Rust backend |
| AppError | Central backend error type; serializes as `{ code, message }` |
| AppPaths | Application-owned directories (config, logs) resolved through Tauri's path resolver |
| AppResult | `Result<T, AppError>` used by every command |
| Capability | Tauri permission scope granted to a webview |
| Design token | A named CSS custom property (e.g. `--color-primary`) |
| Drive root | An absolute storage root the host exposes, e.g. `C:\` or `/mnt/usb` |
| EntryMetadata | Read-only description of one path (kind, size, modification time) |
| IPC | Inter-process communication between the webview and Rust |
| IpcError | Frontend error type carrying a stable code plus message |
| Path normalization | Lexically resolving `.`/`..` and separators without touching the filesystem |
| Platform abstraction | The Rust module that owns all OS-specific knowledge |
| Resolved theme | The effective light/dark value after applying `system` mode |
| Settings service | Frontend module that talks to the backend over IPC with a validated fallback |
| Structured error | An error with a machine-readable `code` plus human `message` |
