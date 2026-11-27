/// Mensaje entrante de una plataforma.
#[derive(Debug, Clone)]
pub struct InboundMessage {
    /// Identificador del chat/hilo en la plataforma.
    pub chat_id: String,
    pub text: String,
    /// Usuario remitente, si la plataforma lo informa.
    pub from: Option<String>,
}

/// Abstracción de plataforma de mensajería (CLI, webhook, bot de chat…).
///
/// Una implementación nueva solo necesita: recibir `InboundMessage` y entregar
/// la respuesta al usuario. El gateway central decide cómo responder con el agente.
pub trait Platform: Send + Sync {
    fn name(&self) -> &str;
    /// Id de sesión estable para un chat concreto (para continuidad cross-plataforma).
    fn session_for(&self, chat_id: &str) -> String {
        format!("{}-{chat_id}", self.name())
    }
}

/// Plataforma de terminal: recibe por stdin, responde por stdout.
pub struct CliPlatform;

impl Platform for CliPlatform {
    fn name(&self) -> &str {
        "cli"
    }
}
