#![allow(dead_code)]

mod config;
mod openai;
mod tools;
mod tts;

use chrono::*;
use voskrust::api::*;
use voskrust::sound::*;

use config::Config;
use openai::{Message, OpenAi};
use tools::ToolManager;
use tts::Tts;

const CONTINUATION_CHUNKS: u32 = 3; // ~300 ms grace period after final result for multi-sentence
const SILENCE_TO_IDLE_CHUNKS: u32 = 20; // ~1.0 s of silence after response → idle

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

#[derive(PartialEq)]
enum AppState {
    Idle,
    ListeningQuery,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    set_log_level(1);

    // ---- config ----
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".into());
    let config = Config::load(&config_path).unwrap_or_else(|e| {
        eprintln!("Ошибка конфигурации: {}", e);
        std::process::exit(1);
    });

    // ---- vosk model ----
    let model = Model::new(&config.vosk.model_path).unwrap();

    // ---- OpenAI client ----
    let ai = OpenAi::new(&config.openai.model);

    // ---- tools ----
    let tool_mgr = ToolManager::new(config.tool);

    // ---- TTS ----
    let tts = Tts::new(&config.tts.model_path);

    // ---- main-loop state ----
    let mut recognizer: Option<Recognizer> = None;
    let mut audioreader: Option<ParecStream> = None;

    let mut state = AppState::Idle;
    let mut accumulated_text = String::new();
    let mut silence_counter: u32 = 0;
    let mut history: Vec<Message> = openai::initial_history(&config.assistant.system_prompt);

    eprintln!("[Система]: Голосовой ассистент запущен.");
    eprintln!(
        "[Система]: Скажите «{}» для активации.",
        config.assistant.wake_word
    );

    loop {
        // ---- time-range gate ----
        let hour = Local::now().hour();
        if hour < config.time_range.start_hour || hour >= config.time_range.end_hour {
            println!("{}", hour);
            recognizer = None;
            audioreader = None;
            state = AppState::Idle;
            accumulated_text.clear();
            silence_counter = 0;
            history = openai::initial_history(&config.assistant.system_prompt);
            std::thread::sleep(std::time::Duration::from_secs(60));
            continue;
        }

        // ---- ensure recognizer & audio stream ----
        if recognizer.is_none() {
            recognizer = Some(Recognizer::new(&model, 16000f32));
        }
        if audioreader.is_none() {
            audioreader = Some(ParecStream::init().unwrap());
        }

        // ---- read 100 ms of audio ----
        let buf = {
            let ar = audioreader.as_mut().unwrap();
            ar.read_n_milliseconds(100.0).unwrap()
        };

        // ---- speech recognition ----
        let (text, is_final) = {
            let rec = recognizer.as_mut().unwrap();
            if rec.accept_waveform(&buf[..]) {
                (rec.final_result(), true)
            } else {
                (rec.partial_result(), false)
            }
        };

        // ---- state machine ----
        match state {
            // ====================== IDLE ======================
            AppState::Idle => {
                if let Some(pos) = text.find(&*config.assistant.wake_word) {
                    let remainder = if is_final {
                        text[pos + config.assistant.wake_word.len()..]
                            .trim()
                            .to_string()
                    } else {
                        String::new()
                    };

                    eprintln!("[Ассистент]: Слушаю...");
                    state = AppState::ListeningQuery;
                    accumulated_text = remainder;
                    silence_counter = 0;
                    history = openai::initial_history(&config.assistant.system_prompt);
                    recognizer = None;
                }
            }

            // ====================== LISTENING ======================
            AppState::ListeningQuery => {
                // -- stop word → immediately back to idle --
                if config
                    .assistant
                    .stop_words
                    .iter()
                    .any(|w| text.contains(w.as_str()))
                {
                    eprintln!("[Ассистент]: Хорошо, до встречи.");
                    state = AppState::Idle;
                    accumulated_text.clear();
                    silence_counter = 0;
                    history = openai::initial_history(&config.assistant.system_prompt);
                    recognizer = None;
                    continue;
                }

                // -- accumulate finalized text, track silence --
                if is_final && !text.is_empty() {
                    if !accumulated_text.is_empty() {
                        accumulated_text.push(' ');
                    }
                    accumulated_text.push_str(&text);
                    silence_counter = 0;
                } else if is_final && text.is_empty() && !accumulated_text.is_empty() {
                    // Empty final after we have text — Vosk detected end of utterance.
                    // Brief grace period then send to OpenAI.
                    silence_counter += CONTINUATION_CHUNKS;
                } else if text.is_empty() {
                    silence_counter += 1;
                } else {
                    // non-empty partial → user is still speaking
                    silence_counter = 0;
                }

                // -- have accumulated text & grace period elapsed → send to OpenAI --
                if !accumulated_text.is_empty() && silence_counter >= CONTINUATION_CHUNKS {
                    eprintln!("[Вы]: {}", accumulated_text);

                    let tools = tool_mgr.tools();
                    let response = ai.ask(
                        &accumulated_text,
                        &mut history,
                        &tools,
                        &mut |name, args| tool_mgr.call_tool(name, args),
                    );
                    eprintln!("[Ассистент]: {}", response);

                    // Stop mic → speak → mic auto-recreates next iteration
                    audioreader = None;
                    tts.speak(&response);
                    recognizer = None;

                    accumulated_text.clear();
                    silence_counter = 0;
                }

                // -- silence with no pending text → go idle --
                if accumulated_text.is_empty() && silence_counter >= SILENCE_TO_IDLE_CHUNKS {
                    eprintln!("[Ассистент]: (режим ожидания)");
                    state = AppState::Idle;
                    history = openai::initial_history(&config.assistant.system_prompt);
                    recognizer = None;
                }
            }
        }
    }
}
