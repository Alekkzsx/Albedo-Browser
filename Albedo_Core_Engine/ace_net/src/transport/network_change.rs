//! # Sensoriamento de Mudança de Rede e Resiliência de Sockets (`NetworkChangeNotifier`)
//!
//! Monitora alterações no estado de conectividade do sistema operacional (troca de Wi-Fi,
//! Ethernet, VPN, perda de rota, renovação de DHCP) e orquestra a limpeza imediata de sockets
//! ociosos ("black-hole") no pool de transporte, invalidação de caches de DNS e recalibração do NQE.

use parking_lot::RwLock;
use smol_str::SmolStr;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Tipo de conexão de rede detectado segundo os padrões Chromium e W3C.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NetworkConnectionType {
    #[default]
    Unknown,
    Ethernet,
    Wifi,
    Cellular,
    Vpn,
    Bluetooth,
    None,
}

impl NetworkConnectionType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Ethernet => "ethernet",
            Self::Wifi => "wifi",
            Self::Cellular => "cellular",
            Self::Vpn => "vpn",
            Self::Bluetooth => "bluetooth",
            Self::None => "none",
        }
    }

    pub const fn is_connected(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Evento emitido quando o sistema operacional relata uma transição de conectividade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkChangeEvent {
    pub connection_type: NetworkConnectionType,
    pub ip_changed: bool,
    pub dns_changed: bool,
    pub interface_name: Option<SmolStr>,
}

/// Notificador central de conectividade e orquestrador de resiliência.
#[derive(Debug, Clone)]
pub struct NetworkChangeNotifier {
    current_type: Arc<RwLock<NetworkConnectionType>>,
    is_online: Arc<RwLock<bool>>,
    event_tx: broadcast::Sender<NetworkChangeEvent>,
}

impl Default for NetworkChangeNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkChangeNotifier {
    /// Cria uma nova instância de `NetworkChangeNotifier`.
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(64);
        Self {
            current_type: Arc::new(RwLock::new(NetworkConnectionType::Unknown)),
            is_online: Arc::new(RwLock::new(true)),
            event_tx,
        }
    }

    /// Retorna o tipo de conexão atual.
    pub fn connection_type(&self) -> NetworkConnectionType {
        *self.current_type.read()
    }

    /// Indica se o dispositivo possui conectividade de rede ativa (conforme `navigator.onLine`).
    pub fn is_online(&self) -> bool {
        *self.is_online.read()
    }

    /// Cria um receptor assíncrono para escutar eventos de mudança de rede.
    pub fn subscribe(&self) -> broadcast::Receiver<NetworkChangeEvent> {
        self.event_tx.subscribe()
    }

    /// Notifica uma mudança de conectividade no sistema operacional,
    /// atualizando o estado interno e disparando eventos para todos os observadores inscritos.
    pub fn notify_network_change(&self, event: NetworkChangeEvent) {
        let is_connected = event.connection_type.is_connected();
        *self.current_type.write() = event.connection_type;
        *self.is_online.write() = is_connected;

        let _ = self.event_tx.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_change_notifier_events() {
        let notifier = NetworkChangeNotifier::new();
        assert!(notifier.is_online());
        assert_eq!(notifier.connection_type(), NetworkConnectionType::Unknown);

        let mut rx = notifier.subscribe();

        // Simula queda de conexão (offline)
        notifier.notify_network_change(NetworkChangeEvent {
            connection_type: NetworkConnectionType::None,
            ip_changed: true,
            dns_changed: false,
            interface_name: Some("wlan0".into()),
        });

        assert!(!notifier.is_online());
        assert_eq!(notifier.connection_type(), NetworkConnectionType::None);

        let event = rx.recv().await.expect("Deve receber evento");
        assert_eq!(event.connection_type, NetworkConnectionType::None);
        assert!(event.ip_changed);

        // Simula conexão com cabo Ethernet
        notifier.notify_network_change(NetworkChangeEvent {
            connection_type: NetworkConnectionType::Ethernet,
            ip_changed: true,
            dns_changed: true,
            interface_name: Some("eth0".into()),
        });

        assert!(notifier.is_online());
        assert_eq!(notifier.connection_type(), NetworkConnectionType::Ethernet);
    }
}

