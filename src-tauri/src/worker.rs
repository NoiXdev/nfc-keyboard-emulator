use crate::config::Config;
use crate::pipeline::{handle_scan, read_error_record};
use crate::reader::{ReaderBackend, ReaderEvent, ReaderStatus, WorkerCommand};
use crate::scan::ScanRecord;
use crate::typer::Typer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
        for ev in deps.backend.poll(Duration::from_millis(400)) {
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
    }

    impl WorkerSink for TestSink {
        fn scan(&self, record: &ScanRecord) {
            self.scans.lock().unwrap().push(record.clone());
        }
        fn readers_changed(&self, readers: &[String]) {
            self.readers.lock().unwrap().push(readers.to_vec());
        }
        fn status(&self, _status: &ReaderStatus) {}
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
}
