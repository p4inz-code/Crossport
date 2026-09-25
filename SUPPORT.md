# Support

## Getting help

- **Documentation** — start with the [README](README.md) and the
  `docs/` directory.
- **Bugs and feature requests** — open an issue using the provided templates
  in `templates/`.

## Troubleshooting

- **The app won't start** — check the log file written by the backend in the
  platform app-log directory (see `docs/development/SETUP.md` for locations).
- **Dev server issues** — ensure Node >= 22 and pnpm 10 are installed, then
  run `./scripts/bootstrap.sh` to reinstall dependencies.
- **Rust issues** — run `cargo check` in `apps/desktop/src-tauri` and include
  its output in any report.
