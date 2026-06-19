use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReaderStatus {
    Connected,
    Disconnected,
    Error { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReaderEvent {
    Scan { reader: String, uid: Vec<u8> },
    ReadError { reader: String },
    ReadersChanged(Vec<String>),
    Status(ReaderStatus),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerCommand {
    SelectReader(Option<String>),
    Rescan,
    // Reserved for tests; not constructed in production code.
    #[allow(dead_code)]
    Shutdown,
}

pub trait ReaderBackend: Send {
    fn list_readers(&mut self) -> Vec<String>;
    fn set_selected(&mut self, name: Option<String>);
    fn poll(&mut self, timeout: Duration) -> Vec<ReaderEvent>;
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::VecDeque;

    pub struct MockReader {
        pub readers: Vec<String>,
        pub selected: Option<String>,
        pub queued: VecDeque<Vec<ReaderEvent>>,
    }

    impl MockReader {
        pub fn new(readers: Vec<String>) -> Self {
            Self {
                readers,
                selected: None,
                queued: VecDeque::new(),
            }
        }
        pub fn queue(&mut self, batch: Vec<ReaderEvent>) {
            self.queued.push_back(batch);
        }
    }

    impl ReaderBackend for MockReader {
        fn list_readers(&mut self) -> Vec<String> {
            self.readers.clone()
        }
        fn set_selected(&mut self, name: Option<String>) {
            self.selected = name;
        }
        fn poll(&mut self, _timeout: Duration) -> Vec<ReaderEvent> {
            self.queued.pop_front().unwrap_or_default()
        }
    }

    #[test]
    fn mock_pops_queued_batches_then_empties() {
        let mut m = MockReader::new(vec!["A".into()]);
        m.queue(vec![ReaderEvent::Scan {
            reader: "A".into(),
            uid: vec![1, 2],
        }]);
        assert_eq!(m.poll(Duration::from_millis(1)).len(), 1);
        assert_eq!(m.poll(Duration::from_millis(1)).len(), 0);
    }
}
