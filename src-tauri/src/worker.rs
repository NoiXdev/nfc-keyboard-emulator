use crate::config::Config;
use crate::pipeline::{handle_scan, read_error_record};
use crate::reader::{ReaderBackend, ReaderEvent, ReaderStatus, WorkerCommand};
use crate::scan::ScanRecord;
use crate::typer::Typer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A backend is expected to block for up to its poll timeout. A broken one can
/// return instantly instead and turn this loop into a spin - which is what a dead
/// PC/SC context did on Windows when an RDP session restarted the smart card
/// service (issue #1): ~5M error events per second, a burnt core, and a flooded
/// UI. The floor below makes that impossible for any backend, while card reads
/// still pass through without added latency.
const POLL_TIMEOUT: Duration = Duration::from_millis(400);
const MIN_CYCLE: Duration = Duration::from_millis(50);

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub trait WorkerSink: Send {
    fn scan(&self, record: &ScanRecord);
    fn readers_changed(&self, readers: &[String]);
    fn status(&self, status: &ReaderStatus);
}

pub struct WorkerDeps {
    pub backend: Box<dyn ReaderBackend>,
    pub sink: Box<dyn WorkerSink>,
    pub typer: Box<dyn Typer>,
    pub config: Arc<Mutex<Config>>,
    pub typing_enabled: Arc<AtomicBool>,
    pub ctrl_rx: Receiver<WorkerCommand>,
}

pub fn run_worker(mut deps: WorkerDeps) {
    let initial = deps.backend.list_readers();
    deps.sink.readers_changed(&initial);
    let selected = deps.config.lock().unwrap().selected_reader.clone();
    deps.backend.set_selected(selected);

    loop {
        let cycle_start = Instant::now();
        let events = deps.backend.poll(POLL_TIMEOUT);
        let had_card_event = events
            .iter()
            .any(|e| matches!(e, ReaderEvent::Scan { .. } | ReaderEvent::ReadError { .. }));

        for ev in events {
            match ev {
                ReaderEvent::Scan { reader, uid } => {
                    let cfg = deps.config.lock().unwrap().clone();
                    let enabled = deps.typing_enabled.load(Ordering::Relaxed);
                    let rec =
                        handle_scan(&uid, &reader, now_iso(), &cfg, enabled, deps.typer.as_mut());
                    deps.sink.scan(&rec);
                }
                ReaderEvent::ReadError { reader } => {
                    deps.sink.scan(&read_error_record(&reader, now_iso()));
                }
                ReaderEvent::ReadersChanged(readers) => deps.sink.readers_changed(&readers),
                ReaderEvent::Status(status) => deps.sink.status(&status),
            }
        }

        while let Ok(cmd) = deps.ctrl_rx.try_recv() {
            match cmd {
                WorkerCommand::SelectReader(name) => deps.backend.set_selected(name),
                WorkerCommand::Rescan => {
                    let readers = deps.backend.list_readers();
                    deps.sink.readers_changed(&readers);
                }
                WorkerCommand::Shutdown => return,
            }
        }

        if !had_card_event {
            if let Some(rest) = MIN_CYCLE.checked_sub(cycle_start.elapsed()) {
                std::thread::sleep(rest);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::mock::MockReader;
    use crate::scan::ScanStatus;
    use crate::typer::mock::MockTyper;
    use std::sync::mpsc;

    #[derive(Clone, Default)]
    struct TestSink {
        scans: Arc<Mutex<Vec<ScanRecord>>>,
        readers: Arc<Mutex<Vec<Vec<String>>>>,
        statuses: Arc<Mutex<Vec<ReaderStatus>>>,
    }

    impl WorkerSink for TestSink {
        fn scan(&self, record: &ScanRecord) {
            self.scans.lock().unwrap().push(record.clone());
        }
        fn readers_changed(&self, readers: &[String]) {
            self.readers.lock().unwrap().push(readers.to_vec());
        }
        fn status(&self, status: &ReaderStatus) {
            self.statuses.lock().unwrap().push(status.clone());
        }
    }

    /// Stands in for a PC/SC context that has died: `get_status_change` then fails
    /// immediately instead of waiting out the timeout.
    struct InstantErrors;

    impl ReaderBackend for InstantErrors {
        fn list_readers(&mut self) -> Vec<String> {
            Vec::new()
        }
        fn set_selected(&mut self, _name: Option<String>) {}
        fn poll(&mut self, _timeout: Duration) -> Vec<ReaderEvent> {
            vec![ReaderEvent::Status(ReaderStatus::Error {
                message: "SCARD_E_SERVICE_STOPPED".into(),
            })]
        }
    }

    #[test]
    fn worker_processes_scan_then_shuts_down() {
        let mut backend = MockReader::new(vec!["ACR122U".into()]);
        backend.queue(vec![ReaderEvent::Scan {
            reader: "ACR122U".into(),
            uid: vec![0x04, 0xA1, 0xB2, 0xC3],
        }]);

        let sink = TestSink::default();
        let collected = sink.scans.clone();
        let (tx, ctrl_rx) = mpsc::channel();
        tx.send(WorkerCommand::Shutdown).unwrap();

        let typing_enabled = Arc::new(AtomicBool::new(true));
        run_worker(WorkerDeps {
            backend: Box::new(backend),
            sink: Box::new(sink),
            typer: Box::new(MockTyper::ok()),
            config: Arc::new(Mutex::new(Config::default())),
            typing_enabled,
            ctrl_rx,
        });

        let scans = collected.lock().unwrap();
        assert_eq!(scans.len(), 1);
        assert_eq!(scans[0].uid_hex, "04A1B2C3");
        assert_eq!(scans[0].status, ScanStatus::Ok);
    }

    #[test]
    fn a_failing_backend_does_not_spin_the_worker() {
        let sink = TestSink::default();
        let statuses = sink.statuses.clone();
        let (tx, ctrl_rx) = mpsc::channel();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            let _ = tx.send(WorkerCommand::Shutdown);
        });

        let start = Instant::now();
        run_worker(WorkerDeps {
            backend: Box::new(InstantErrors),
            sink: Box::new(sink),
            typer: Box::new(MockTyper::ok()),
            config: Arc::new(Mutex::new(Config::default())),
            typing_enabled: Arc::new(AtomicBool::new(false)),
            ctrl_rx,
        });

        // Without the cycle floor this ran at ~5,000,000 cycles per second.
        let per_second = statuses.lock().unwrap().len() as f64 / start.elapsed().as_secs_f64();
        assert!(
            per_second < 100.0,
            "worker spun at {per_second:.0} cycles/s - the cycle floor is gone"
        );
    }
}
