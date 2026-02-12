use std::path::Path;

use piper_rs::synth::PiperSpeechSynthesizer;
use rodio::{OutputStream, Sink, buffer::SamplesBuffer};

pub struct Tts {
    synth: PiperSpeechSynthesizer,
}

impl Tts {
    pub fn new(config_path: &str) -> Self {
        let model = piper_rs::from_config_path(Path::new(config_path))
            .expect("Ошибка загрузки модели TTS");
        let synth = PiperSpeechSynthesizer::new(model)
            .expect("Ошибка инициализации TTS");
        Self { synth }
    }

    pub fn speak(&self, text: &str) {
        let audio_stream = self.synth.synthesize_parallel(text.to_string(), None)
            .expect("Ошибка синтеза речи");
        let samples: Vec<f32> = audio_stream
            .into_iter()
            .flat_map(|result| result.expect("Ошибка синтеза фрагмента").into_vec())
            .collect();
        if samples.is_empty() {
            return;
        }
        let buf = SamplesBuffer::new(1, 22050, samples);
        let (_stream, stream_handle) = OutputStream::try_default()
            .expect("Ошибка открытия аудио выхода");
        let sink = Sink::try_new(&stream_handle)
            .expect("Ошибка создания Sink");
        sink.append(buf);
        sink.sleep_until_end();
    }
}
