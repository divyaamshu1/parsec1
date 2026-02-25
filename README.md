# Parsec (local workspace)

This workspace contains the Parsec IDE Rust monorepo (core, gui, cli, extensions, ai, etc.).

This README was added automatically when preparing to publish the repository.

How to run the GUI (quick):

1. Build the GUI:

```powershell
cargo build -p parsec-gui --release
```

2. Run the binary:

```powershell
.\target\release\parsec.exe
```

Notes:
- The GUI serves a browser UI at a printed URL and opens your default browser.
- The repo includes many crates; see top-level `Cargo.toml` for workspace details.
# Parsec IDE

Lightweight browser-hosted frontend for the Parsec IDE workspace.

How to run locally

```powershell
cargo build -p parsec-gui --release
.\target\release\parsec.exe
```

Frontend is served from `gui/dist` and the server exposes REST and WebSocket endpoints for editor, terminal, explorer and AI stubs.

To publish to GitHub, see CONTRIBUTING section below.

Contributing

1. Create a new GitHub repository.
2. Add it as `origin` and push:

```powershell
git remote add origin https://github.com/<your-user>/<repo>.git
git push -u origin main
```
