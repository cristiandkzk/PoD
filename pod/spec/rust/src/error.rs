//! Errores del formato — SPEC.md §7.

use std::fmt;

/// El `code` es parte del contrato entre implementaciones (SPEC §7). El `path` es
/// diagnóstico: ayuda a ubicar el problema pero no es normativo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecError {
    pub code: &'static str,
    pub path: String,
}

impl SpecError {
    pub fn new(code: &'static str, path: impl Into<String>) -> Self {
        SpecError {
            code,
            path: path.into(),
        }
    }
}

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\t{}", self.code, self.path)
    }
}

impl std::error::Error for SpecError {}
