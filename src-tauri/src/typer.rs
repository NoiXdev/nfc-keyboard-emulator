use crate::config::Suffix;
use crate::formatter::FormattedScan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    // Nur auf macOS konstruiert (fehlendes Bedienungshilfen-Recht).
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    NoAccessibility,
    Other(String),
}

pub trait Typer: Send {
    fn type_scan(&mut self, formatted: &FormattedScan) -> Result<(), TypeError>;
}

/// Fallback wenn enigo nicht initialisiert werden konnte.
pub struct NullTyper;

impl Typer for NullTyper {
    fn type_scan(&mut self, _formatted: &FormattedScan) -> Result<(), TypeError> {
        Err(TypeError::Other("keyboard emulation unavailable".into()))
    }
}

pub struct EnigoTyper {
    enigo: enigo::Enigo,
}

// SAFETY: EnigoTyper is used exclusively on the worker thread; it is never
// shared across threads simultaneously. The inner CGEventSource is not Sync
// but we only move/use EnigoTyper from one thread at a time.
#[cfg(target_os = "macos")]
unsafe impl Send for EnigoTyper {}

impl EnigoTyper {
    pub fn new() -> Result<Self, TypeError> {
        enigo::Enigo::new(&enigo::Settings::default())
            .map(|enigo| Self { enigo })
            .map_err(|e| TypeError::Other(e.to_string()))
    }
}

impl Typer for EnigoTyper {
    fn type_scan(&mut self, formatted: &FormattedScan) -> Result<(), TypeError> {
        use enigo::{Direction, Key, Keyboard};

        #[cfg(target_os = "macos")]
        if !crate::accessibility::is_trusted() {
            return Err(TypeError::NoAccessibility);
        }

        self.enigo
            .text(&formatted.body)
            .map_err(|e| TypeError::Other(e.to_string()))?;
        match formatted.suffix {
            Suffix::Enter => self
                .enigo
                .key(Key::Return, Direction::Click)
                .map_err(|e| TypeError::Other(e.to_string()))?,
            Suffix::Tab => self
                .enigo
                .key(Key::Tab, Direction::Click)
                .map_err(|e| TypeError::Other(e.to_string()))?,
            Suffix::None => {}
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    pub struct MockTyper {
        pub calls: Arc<Mutex<Vec<FormattedScan>>>,
        pub result: Arc<Mutex<Result<(), TypeError>>>,
    }

    impl MockTyper {
        pub fn ok() -> Self {
            Self {
                calls: Arc::new(Mutex::new(vec![])),
                result: Arc::new(Mutex::new(Ok(()))),
            }
        }
        pub fn failing(err: TypeError) -> Self {
            Self {
                calls: Arc::new(Mutex::new(vec![])),
                result: Arc::new(Mutex::new(Err(err))),
            }
        }
        pub fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }

    impl Typer for MockTyper {
        fn type_scan(&mut self, formatted: &FormattedScan) -> Result<(), TypeError> {
            self.calls.lock().unwrap().push(formatted.clone());
            self.result.lock().unwrap().clone()
        }
    }

    #[test]
    fn mock_records_calls() {
        let mut t = MockTyper::ok();
        t.type_scan(&FormattedScan {
            body: "AB".into(),
            suffix: Suffix::Enter,
        })
        .unwrap();
        assert_eq!(t.call_count(), 1);
    }
}
