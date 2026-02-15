// ARQUIVO: src/net/websocket.rs

use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc;
use tokio::task;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

/// Eventos que o Navegador recebe do WebSocket
#[derive(Debug, Clone)]
pub enum WsEvent {
    Connected,
    Message(String),
    Binary(Vec<u8>),
    Disconnected,
    Error(String),
}

/// Comandos que o Navegador envia para o WebSocket
#[derive(Debug)]
pub enum WsCommand {
    SendText(String),
    SendBinary(Vec<u8>),
    Close,
}

/// O "Controle Remoto" do WebSocket.
/// A Engine segura isso. É super leve (só tem canais de comunicação).
pub struct WebSocketClient {
    command_sender: mpsc::Sender<WsCommand>,
    // O ID ajuda a saber qual aba é dona desse socket se precisarmos no futuro
    pub id: String, 
}

impl WebSocketClient {
    /// Conecta em um URL (ex: wss://echo.websocket.org) e retorna o Cliente + O Canal de Escuta
    pub fn connect(url_str: &str, id: String) -> (Option<Self>, mpsc::Receiver<WsEvent>) {
        // Valida URL
        let url = match Url::parse(url_str) {
            Ok(u) => u,
            Err(e) => return (None, create_error_receiver(format!("URL Inválida: {}", e))),
        };

        // Canais de Comunicação (Rust comunica com a Thread de Rede por aqui)
        // Buffer de 32 mensagens para não estourar RAM se o servidor for muito rápido
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<WsCommand>(32);
        let (event_tx, event_rx) = mpsc::channel::<WsEvent>(128);

        let event_tx_clone = event_tx.clone();

        // SPIDER TASK: Cria uma thread leve (Green Thread) isolada para cuidar da rede
        tokio::spawn(async move {
            match connect_async(url).await {
                Ok((ws_stream, _response)) => {
                    let _ = event_tx_clone.send(WsEvent::Connected).await;

                    // Divide o socket em "Ler" e "Escrever"
                    let (mut write_stream, mut read_stream) = ws_stream.split();

                    // Loop Principal da Conexão (Select)
                    loop {
                        tokio::select! {
                            // 1. Ocorreu algo na leitura (Server mandou msg)?
                            msg = read_stream.next() => {
                                match msg {
                                    Some(Ok(message)) => match message {
                                        Message::Text(text) => { 
                                            let _ = event_tx_clone.send(WsEvent::Message(text)).await; 
                                        },
                                        Message::Binary(bin) => { 
                                            let _ = event_tx_clone.send(WsEvent::Binary(bin)).await; 
                                        },
                                        Message::Close(_) => {
                                            let _ = event_tx_clone.send(WsEvent::Disconnected).await;
                                            break;
                                        },
                                        _ => {} // Ignora Pings/Pongs para economizar CPU
                                    },
                                    Some(Err(e)) => {
                                        let _ = event_tx_clone.send(WsEvent::Error(e.to_string())).await;
                                        break;
                                    },
                                    None => break, // Stream fechou
                                }
                            }

                            // 2. O Navegador mandou um comando?
                            cmd = cmd_rx.recv() => {
                                match cmd {
                                    Some(WsCommand::SendText(txt)) => {
                                        if let Err(_) = write_stream.send(Message::Text(txt)).await { break; }
                                    },
                                    Some(WsCommand::SendBinary(bin)) => {
                                        if let Err(_) = write_stream.send(Message::Binary(bin)).await { break; }
                                    },
                                    Some(WsCommand::Close) | None => {
                                        let _ = write_stream.close().await;
                                        break; 
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = event_tx_clone.send(WsEvent::Error(format!("Falha na conexão: {}", e))).await;
                }
            };
            // Cleanup final
            let _ = event_tx_clone.send(WsEvent::Disconnected).await;
        });

        (Some(Self { command_sender: cmd_tx, id }), event_rx)
    }

    // Métodos públicos fáceis para a Engine usar
    pub fn send_text(&self, text: &str) {
        let sender = self.command_sender.clone();
        let t = text.to_string();
        tokio::spawn(async move {
            let _ = sender.send(WsCommand::SendText(t)).await;
        });
    }

    pub fn close(&self) {
        let sender = self.command_sender.clone();
        tokio::spawn(async move {
            let _ = sender.send(WsCommand::Close).await;
        });
    }
}

// Helper para retornar erro imediato sem thread
fn create_error_receiver(msg: String) -> mpsc::Receiver<WsEvent> {
    let (tx, rx) = mpsc::channel(1);
    let _ = tx.blocking_send(WsEvent::Error(msg));
    rx
}