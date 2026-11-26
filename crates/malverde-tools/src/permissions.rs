use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use malverde_core::{Config, MalverdeError, PermissionMode, Result};

/// Decisión del motor de permisos sobre una herramienta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allowed,
    /// Requiere aprobación humana (solo posible en modo interactivo).
    NeedsApproval { reason: String },
    Denied { reason: String },
}

/// Motor de permisos central del harness.
pub struct PermissionManager {
    mode: PermissionMode,
    allow_list: HashSet<String>,
    /// Herramientas aprobadas temporalmente durante la sesión actual.
    session_approved: HashSet<String>,
    /// Callback de aprobación humana (CLI/TUI interactivos).
    approver: Option<Box<dyn Fn(&str, &str) -> ApprovalAnswer + Send + Sync>>,
}

impl PermissionManager {
    pub fn new(mode: PermissionMode, allow_list: &[String]) -> Self {
        Self {
            mode,
            allow_list: allow_list.iter().cloned().collect(),
            session_approved: HashSet::new(),
            approver: None,
        }
    }

    /// Instala el callback de aprobación humana.
    pub fn set_approver(&mut self, approver: Box<dyn Fn(&str, &str) -> ApprovalAnswer + Send + Sync>) {
        self.approver = Some(approver);
    }

    /// Pide aprobación humana si hay callback instalado; false si no hay a quién preguntar.
    pub fn request_approval(&mut self, tool: &str, reason: &str) -> bool {
        match &self.approver {
            Some(approve) => match approve(tool, reason) {
                ApprovalAnswer::Yes => true,
                ApprovalAnswer::YesAlways => {
                    self.session_approved.insert(tool.to_string());
                    true
                }
                ApprovalAnswer::No => false,
            },
            None => false,
        }
    }

    pub fn from_config(cfg: &Config) -> Self {
        Self::new(cfg.permission_mode, &cfg.allow_list)
    }

    pub fn mode(&self) -> PermissionMode {
        self.mode
    }

    /// Aprueba temporalmente una herramienta para el resto de la sesión.
    pub fn approve_for_session(&mut self, tool: &str) {
        self.session_approved.insert(tool.to_string());
    }

    pub fn evaluate(&self, tool: &str, sensitive: bool) -> Decision {
        match self.mode {
            PermissionMode::AutoApprove => Decision::Allowed,
            PermissionMode::ReadOnly => {
                if sensitive {
                    Decision::Denied {
                        reason: format!("modo read_only: '{tool}' es una herramienta sensible"),
                    }
                } else {
                    Decision::Allowed
                }
            }
            PermissionMode::AllowList => {
                if self.allow_list.contains(tool) || self.session_approved.contains(tool) {
                    Decision::Allowed
                } else {
                    Decision::NeedsApproval {
                        reason: format!("'{tool}' no está en la allow_list"),
                    }
                }
            }
            PermissionMode::Ask => {
                if self.session_approved.contains(tool) {
                    Decision::Allowed
                } else {
                    Decision::NeedsApproval {
                        reason: format!("modo ask: se requiere aprobación para '{tool}'"),
                    }
                }
            }
        }
    }

    pub fn require_allowed(&mut self, tool: &str, sensitive: bool) -> Result<()> {
        match self.evaluate(tool, sensitive) {
            Decision::Allowed => Ok(()),
            Decision::Denied { reason } => Err(MalverdeError::Permission(reason)),
            Decision::NeedsApproval { reason } => {
                Err(MalverdeError::Permission(format!("{reason} (ejecuta en modo interactivo para aprobarla)")))
            }
        }
    }
}

/// Modo de pregunta configurado por el usuario en config.json.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalAnswer {
    Yes,
    YesAlways,
    No,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_approve_allows_everything() {
        let mut pm = PermissionManager::new(PermissionMode::AutoApprove, &[]);
        assert_eq!(pm.evaluate("shell", true), Decision::Allowed);
        pm.require_allowed("shell", true).unwrap();
    }

    #[test]
    fn read_only_blocks_sensitive() {
        let mut pm = PermissionManager::new(PermissionMode::ReadOnly, &[]);
        assert!(pm.evaluate("shell", true).is_denied());
        pm.require_allowed("read_file", false).unwrap();
        assert!(pm.require_allowed("write_file", true).is_err());
    }

    #[test]
    fn allow_list_honors_session_approval() {
        let mut pm = PermissionManager::new(
            PermissionMode::AllowList,
            &["read_file".to_string()],
        );
        assert!(pm.evaluate("shell", true).is_needs_approval());
        pm.approve_for_session("shell");
        assert_eq!(pm.evaluate("shell", true), Decision::Allowed);
    }

    impl Decision {
        pub fn is_denied(&self) -> bool {
            matches!(self, Decision::Denied { .. })
        }
        pub fn is_needs_approval(&self) -> bool {
            matches!(self, Decision::NeedsApproval { .. })
        }
    }
}
