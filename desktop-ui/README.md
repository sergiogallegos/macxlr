# MacXLR Desktop UI

This is a separate native desktop shell for macOS built with Tauri 2, React, TypeScript, and Vite.

It does not replace the existing daemon yet. Instead, it starts `goxlr-daemon` and hosts the
existing configuration UI inside a native app window so you do not need to use a browser.

The app is designed to open directly into the GoXLR interface with minimal extra chrome. On
launch, it automatically starts the bundled daemon and waits for the mixer UI to become available.

## Why this stack

- `Tauri 2`: best fit for this repo because the backend is already Rust and the macOS app can stay
  small while integrating cleanly with the daemon.
- `React + TypeScript`: fast to iterate on and well suited for building a richer native shell over
  time.
- `Vite`: simple and fast local dev flow.

## Run in development

From the repository root:

```bash
cargo build -p goxlr-daemon
cd desktop-ui
npm install
npm run dev
```

## Build the desktop app

From the repository root:

```bash
./scripts/build-native-macos-desktop-app.sh
```

This produces a self-contained app bundle at:

```bash
dist/MacXLR Desktop.app
```

After the build finishes, drag `dist/MacXLR Desktop.app` into `/Applications` and launch it like a
normal macOS app. The bundled app includes `goxlr-daemon`, so it is ready to run immediately after
the build.

## Development run

If you want the desktop shell in dev mode:

```bash
./scripts/run-native-macos-desktop.sh
```
