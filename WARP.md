# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Commands
- Prereqs: Node.js ≥ 18, Rust toolchain for Tauri 2, .NET 8 SDK (for the UIA sidecar).
- Install JS deps: `npm install`.
- Run desktop dev (frontend + Rust): `npm run tauri dev` (Vite on 1420/1421; honors `TAURI_DEV_HOST`).
- Run frontend only: `npm run dev`.
- Typecheck & build web assets: `npm run build` (runs `vue-tsc --noEmit` then Vite, outputs `dist/`).
- Preview built web bundle: `npm run preview`.
- Bundle desktop app: `npm run tauri build` (artifacts under `src-tauri/target/*/release/bundle`).
- C# UIA agent build: `dotnet build Autothink.UiaAgent/Autothink.UiaAgent.csproj -c Release`.
- C# agent local run (JSON-RPC over stdio; prints READY then awaits messages): `dotnet run --project Autothink.UiaAgent/Autothink.UiaAgent.csproj`.
- Tests: none defined for JS/Rust or the C# sidecar yet.

## Architecture
- Frontend (Vite + Vue 3 + TS, `src/`):
  - `main.ts` mounts `App.vue`.
  - `App.vue` is the starter page calling the Tauri command `greet` via `@tauri-apps/api/core.invoke`, currently a demo UI.
- Tauri core (Rust, `src-tauri/`):
  - `lib.rs` exposes `#[tauri::command] greet`, builds the app with `tauri_plugin_opener`, and runs via `tauri_app_lib::run()` from `main.rs`.
  - `tauri.conf.json` sets `beforeDevCommand: npm run dev`, `devUrl: http://localhost:1420`, strictPort 1420/1421 HMR, and bundles for all targets with an 800×600 window.
- UIA sidecar agent (C#, `Autothink.UiaAgent/`):
  - Console app targeting `net8.0-windows`; references `StreamJsonRpc` and `FlaUI.UIA3`.
  - `Program.cs` spins a dedicated STA thread and runs `AgentHost.Run`, surfacing fatal exceptions to stderr; exit code stays non-zero unless Run completes.
  - `AgentHost` writes `READY` on startup, uses stdin/stdout with `HeaderDelimitedMessageHandler` for JSON-RPC frames, dispatches via a custom `SingleThreadSynchronizationContext` to keep all UIA work on one STA thread, and currently exposes a `Ping -> "pong"` RPC placeholder. Stdout is reserved for protocol frames; diagnostics go to stderr.
- Current integration: the Vue UI only talks to the Rust `greet` command; the UIA sidecar is not yet wired into the Tauri app.

## Notes
- HMR host can be overridden with `TAURI_DEV_HOST` (see `vite.config.ts`); otherwise Vite binds locally with `strictPort: true` on 1420.
- No frontend lint config; `npm run build` is the main type/compile check path.
