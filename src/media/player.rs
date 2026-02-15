// ARQUIVO: src/media/player.rs

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use parking_lot::Mutex;
use std::io::Cursor;
use crossbeam_channel::{Sender, Receiver, unbounded};

// Tipos de Estado de Mídia (HTML5 Spec compliant)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MediaState {
    Empty,      // Nenhum arquivo carregado
    Loading,    // Baixando/Buffering
    Ready,      // Carregado (Metadados prontos)
    Playing,
    Paused,
    Ended,
    Error,
}

// Comandos que a UI/Engine envia para o Player
pub enum MediaCommand {
    Load(String),       // Carregar URL
    Play,
    Pause,
    Stop,
    Seek(f32),          // Ir para segundo X
    SetVolume(f32),     // 0.0 a 1.0
}

// Eventos que o Player envia para a UI
pub enum MediaEvent {
    TimeUpdate(f32),    // Segundo atual (para barra de progresso)
    StateChange(MediaState),
    VideoFrame(Vec<u8>, u32, u32), // Bytes RGBA, Width, Height
    Error(String),
}

/// O Controlador Principal. O Albedo cria uma instância disto para cada tag <video> ou <audio>.
pub struct MediaPlayer {
    command_tx: Sender<MediaCommand>,
    event_rx: Receiver<MediaEvent>,
    
    // Controle interno thread-safe
    is_playing: Arc<AtomicBool>,
}

impl MediaPlayer {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = unbounded();
        let (evt_tx, evt_rx) = unbounded();
        let is_playing = Arc::new(AtomicBool::new(false));
        
        let player_running = is_playing.clone();

        // 🧵 Thread de Áudio/Vídeo (Media Loop)
        // Isso roda separado para não travar o scroll do navegador
        std::thread::Builder::new()
            .name("Albedo Media Thread".into())
            .spawn(move || {
                run_media_thread(cmd_rx, evt_tx, player_running);
            })
            .expect("Falha ao criar thread de mídia");

        Self {
            command_tx: cmd_tx,
            event_rx: evt_rx,
            is_playing,
        }
    }

    // --- API PÚBLICA (Engine chama isso) ---

    pub fn load_url(&self, url: &str) {
        let _ = self.command_tx.send(MediaCommand::Load(url.to_string()));
    }

    pub fn play(&self) {
        let _ = self.command_tx.send(MediaCommand::Play);
    }

    pub fn pause(&self) {
        let _ = self.command_tx.send(MediaCommand::Pause);
    }

    pub fn set_volume(&self, volume: f32) {
        let _ = self.command_tx.send(MediaCommand::SetVolume(volume));
    }

    /// O loop da Engine deve chamar isso para ver se tem frame novo ou atualizar a barra
    pub fn poll_events(&self) -> Vec<MediaEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }
}

// --- CORE: O "Motor" que roda na Thread de Fundo ---

fn run_media_thread(
    cmd_rx: Receiver<MediaCommand>,
    evt_tx: Sender<MediaEvent>,
    is_playing_atomic: Arc<AtomicBool>
) {
    // 1. Inicializa Sistema de Som (Rodio)
    // Tenta pegar o driver de som padrão do sistema (Alsa/Wasapi/CoreAudio)
    let (_stream, stream_handle) = match rodio::OutputStream::try_default() {
        Ok(s) => s,
        Err(e) => {
            let _ = evt_tx.send(MediaEvent::Error(format!("Erro Audio Driver: {}", e)));
            return; // Sai se não tiver placa de som
        }
    };

    let sink = rodio::Sink::try_new(&stream_handle).unwrap();
    let mut current_state = MediaState::Empty;
    
    // Buffer para vídeo (no futuro aqui conecta o decoder ffmpeg)
    // let mut video_decoder = None;

    loop {
        // 1. Processar Comandos
        // Usamos try_recv para não bloquear o loop (queremos loop de renderização)
        if let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                MediaCommand::Load(url) => {
                    // Simulamos carregamento (Aqui entra o Fetch API depois)
                    let _ = evt_tx.send(MediaEvent::StateChange(MediaState::Loading));
                    println!("MEDIA: Carregando recurso {}", url);
                    
                    // Exemplo: Se for áudio, baixamos e decodificamos
                    // OBS: No Albedo real, use fetcher.rs para baixar bytes
                    
                    // Mock: Dizemos que está pronto
                    current_state = MediaState::Ready;
                    let _ = evt_tx.send(MediaEvent::StateChange(MediaState::Ready));
                }
                
                MediaCommand::Play => {
                    if current_state == MediaState::Ready || current_state == MediaState::Paused {
                        sink.play(); // Despausa o sink do Rodio
                        current_state = MediaState::Playing;
                        is_playing_atomic.store(true, Ordering::SeqCst);
                        let _ = evt_tx.send(MediaEvent::StateChange(MediaState::Playing));
                    }
                }
                
                MediaCommand::Pause => {
                    sink.pause();
                    current_state = MediaState::Paused;
                    is_playing_atomic.store(false, Ordering::SeqCst);
                    let _ = evt_tx.send(MediaEvent::StateChange(MediaState::Paused));
                }
                
                MediaCommand::Stop => {
                    sink.stop();
                    current_state = MediaState::Ready;
                    is_playing_atomic.store(false, Ordering::SeqCst);
                }

                MediaCommand::SetVolume(v) => {
                    sink.set_volume(v);
                }

                MediaCommand::Seek(time) => {
                    // Tenta fazer seek no sink se suportado
                    // sink.try_seek(Duration::from_secs_f32(time));
                    println!("MEDIA: Seek para {}s", time);
                }
            }
        }

        // 2. Loop de Playback (Sincronia AV)
        if current_state == MediaState::Playing {
            // Emite o tempo atual
            // No futuro: let pos = video_decoder.current_time();
            // Mock:
            let _ = evt_tx.send(MediaEvent::TimeUpdate(0.0)); // Fake tick

            // 3. Processamento de Vídeo
            // Aqui é onde geraríamos a textura
            /*
            if let Some(frame) = decoder.get_next_frame() {
                evt_tx.send(MediaEvent::VideoFrame(frame.bytes, frame.w, frame.h));
            }
            */
        }

        // Dorme um pouco para não comer 100% de CPU na thread
        // ~60 FPS wait
        std::thread::sleep(Duration::from_millis(16));
    }
}

// --- HELPERS (Ex: Gerar beep para teste) ---

pub fn create_test_sound(player: &MediaPlayer) {
    // Isso seria chamado para testar se o audio funciona
    // Usa uma senoidal pura para não depender de arquivos mp3
    println!("MEDIA: Tentativa de som de teste não implementada sem decoders externos por enquanto.");
    // Rodio suporta `source::SineWave::new(440)` se quisermos adicionar depois.
}