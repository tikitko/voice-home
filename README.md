# voice-home

A local voice assistant written in Rust. Uses Vosk for offline speech recognition, OpenAI for conversation, and Piper for neural text-to-speech — all configurable via a single TOML file.

## How it works

1. Listens for a **wake word** using Vosk (offline, no cloud)
2. Records your query until silence is detected
3. Sends the query to **OpenAI** (with tool-calling support)
4. Speaks the response aloud via **Piper TTS**
5. Pauses the microphone during playback to avoid self-hearing
6. Returns to listening

Say a **stop word** (e.g. "stop") to end the conversation early.

## Prerequisites

- Rust (edition 2024)
- `cmake` (for building espeak-ng, a Piper dependency)
- `OPENAI_API_KEY` environment variable
- PulseAudio (`parec`) for microphone input

### Models

**Vosk** (speech recognition):
```sh
wget https://alphacephei.com/vosk/models/vosk-model-small-ru-0.22.zip
unzip vosk-model-small-ru-0.22.zip
```

**Piper** (text-to-speech):
```sh
mkdir ru_RU-irina-medium && cd ru_RU-irina-medium
wget https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/irina/medium/ru_RU-irina-medium.onnx
wget https://huggingface.co/rhasspy/piper-voices/resolve/main/ru/ru_RU/irina/medium/ru_RU-irina-medium.onnx.json
```

## Build & Run

```sh
cargo build --release
OPENAI_API_KEY=sk-... ./target/release/voice-home
```

Optionally pass a custom config path:
```sh
./target/release/voice-home my-config.toml
```

## Configuration

All settings live in `config.toml`:

```toml
[vosk]
model_path = "./vosk-model-small-ru-0.22"

[assistant]
wake_word = "ирина"
stop_words = ["стоп", "спасибо", "хватит", "отмена"]
system_prompt = "You are a voice assistant. Reply briefly."

[openai]
model = "gpt-4o-mini"

[tts]
model_path = "./ru_RU-irina-medium/ru_RU-irina-medium.onnx.json"

[time_range]
start_hour = 0   # active from this hour
end_hour = 23    # active until this hour
```

### Tools

Tools let the assistant execute shell commands. Define them as `[[tool]]` blocks:

```toml
[[tool]]
name = "get_current_time"
description = "Get the current date and time"
command = "date '+%Y-%m-%d %H:%M:%S'"

[[tool]]
name = "wake_on_lan"
description = "Wake a computer via Wake-on-LAN"
command = "wakeonlan {{mac_address}}"
required_params = ["mac_address"]

[tool.params.mac_address]
type = "string"
description = "MAC address of the computer to wake"
```

The assistant decides when to call tools based on the conversation. Parameters use `{{name}}` placeholders substituted at runtime.

## License

MIT
