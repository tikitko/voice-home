# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```sh
cargo build --release
OPENAI_API_KEY=sk-... ./target/release/voice-home          # default config.toml
OPENAI_API_KEY=sk-... ./target/release/voice-home my.toml  # custom config
```

`cmake` is required at build time (espeak-ng, a piper-rs dependency). PulseAudio (`parec`) must be available at runtime for microphone input.

**Pinned deps:** `ort` and `ort-sys` are pinned to `=2.0.0-rc.9` for compatibility with `piper-rs 0.1`. Do not upgrade without verifying piper-rs compiles.

## Architecture

Single-threaded blocking main loop processing 100ms audio chunks through a two-state machine (`Idle` / `ListeningQuery`).

**Data flow:** Microphone (voskrust/ParecStream) → Vosk speech recognition → wake word triggers `ListeningQuery` → silence detected → OpenAI chat completion (with tool calls) → Piper TTS speaks response → mic paused during playback (dropped, auto-recreates next iteration).

### Modules

- **main.rs** — State machine, audio loop, orchestrates all modules
- **config.rs** — TOML config structs with `#[serde(default)]` for all optional sections
- **openai.rs** — Wraps async-openai in a blocking interface (`tokio::runtime::block_on`). Handles multi-turn tool-call loops: dispatches tool calls via callback, feeds results back, re-prompts until a text response is returned
- **tools.rs** — Executes shell commands via `sh -c` with `{{param}}` placeholder substitution. Tool definitions come from `[[tool]]` blocks in config
- **tts.rs** — Loads Piper ONNX model once at startup, synthesizes with `synthesize_parallel`, plays via rodio `SamplesBuffer` at 22050 Hz mono

### Key constants (main.rs)

- `CONTINUATION_CHUNKS` (3) — ~300ms grace period for multi-sentence input
- `SILENCE_TO_IDLE_CHUNKS` (20) — ~2s silence before returning to idle

## Conventions

- All user-facing strings and error messages are in Russian
- Config sections use `#[serde(default)]` with explicit default functions — only `[assistant]` (with `wake_word`) is required
- Audio stream lifecycle: set `audioreader = None` to stop mic, it auto-recreates on next loop iteration; set `recognizer = None` to flush stale audio buffers

## External models (not in git)

- **Vosk:** `vosk-model-small-ru-0.22/` — offline Russian speech recognition
- **Piper:** `ru_RU-irina-medium/` — Russian female TTS voice (`.onnx` + `.onnx.json`)
