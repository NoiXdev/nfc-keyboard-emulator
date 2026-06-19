use crate::reader::{ReaderBackend, ReaderEvent, ReaderStatus};
use pcsc::{Context, Protocols, ReaderState, Scope, ShareMode, State, PNP_NOTIFICATION};
use std::ffi::CString;
use std::time::Duration;

pub struct PcscReader {
    ctx: Context,
    selected: Option<CString>,
    pnp_state: State,
    selected_state: State,
    card_present: bool,
    connected_emitted: bool,
}

impl PcscReader {
    pub fn new() -> Result<Self, pcsc::Error> {
        Ok(Self {
            ctx: Context::establish(Scope::User)?,
            selected: None,
            pnp_state: State::UNAWARE,
            selected_state: State::UNAWARE,
            card_present: false,
            connected_emitted: false,
        })
    }

    fn read_uid(&self, reader: &CString) -> Option<Vec<u8>> {
        let card = self
            .ctx
            .connect(reader, ShareMode::Shared, Protocols::ANY)
            .ok()?;
        let apdu = [0xFF, 0xCA, 0x00, 0x00, 0x00];
        let mut buf = [0u8; 264];
        let resp = card.transmit(&apdu, &mut buf).ok()?;
        if resp.len() >= 2 && resp[resp.len() - 2] == 0x90 && resp[resp.len() - 1] == 0x00 {
            Some(resp[..resp.len() - 2].to_vec())
        } else {
            None
        }
    }
}

impl ReaderBackend for PcscReader {
    fn list_readers(&mut self) -> Vec<String> {
        let mut buf = [0u8; 2048];
        let readers = match self.ctx.list_readers(&mut buf) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        readers.map(|c| c.to_string_lossy().into_owned()).collect()
    }

    fn set_selected(&mut self, name: Option<String>) {
        self.selected = name.and_then(|n| CString::new(n).ok());
        self.selected_state = State::UNAWARE;
        self.card_present = false;
        self.connected_emitted = false;
    }

    fn poll(&mut self, timeout: Duration) -> Vec<ReaderEvent> {
        let mut events = Vec::new();

        let Some(selected) = self.selected.clone() else {
            // Ohne gewählten Leser nur auf Reader-Plug/Unplug lauschen.
            let mut states = vec![ReaderState::new(PNP_NOTIFICATION(), self.pnp_state)];
            match self.ctx.get_status_change(timeout, &mut states) {
                Ok(()) => {
                    let event_state = states[0].event_state();
                    self.pnp_state = event_state;
                    if event_state.contains(State::CHANGED) {
                        let names = self.list_readers();
                        events.push(ReaderEvent::ReadersChanged(names));
                    }
                }
                Err(pcsc::Error::Timeout) => {}
                Err(e) => {
                    events.push(ReaderEvent::Status(ReaderStatus::Error {
                        message: e.to_string(),
                    }));
                }
            }
            return events;
        };

        let mut states = vec![
            ReaderState::new(PNP_NOTIFICATION(), self.pnp_state),
            ReaderState::new(selected.as_c_str(), self.selected_state),
        ];

        match self.ctx.get_status_change(timeout, &mut states) {
            Ok(()) => {}
            Err(pcsc::Error::Timeout) => return events,
            Err(e) => {
                events.push(ReaderEvent::Status(ReaderStatus::Error {
                    message: e.to_string(),
                }));
                return events;
            }
        }

        // Zustände lesen bevor states dropped wird (Copy)
        let new_pnp_state = states[0].event_state();
        let new_selected_state = states[1].event_state();

        if new_pnp_state.contains(State::CHANGED) {
            let names = self.list_readers();
            events.push(ReaderEvent::ReadersChanged(names));
        }

        {
            let es = new_selected_state;
            let reader_name = selected.to_string_lossy().into_owned();

            if es.contains(State::UNAVAILABLE) || es.contains(State::UNKNOWN) {
                if self.connected_emitted {
                    self.connected_emitted = false;
                    events.push(ReaderEvent::Status(ReaderStatus::Disconnected));
                }
                self.card_present = false;
            } else {
                if !self.connected_emitted {
                    self.connected_emitted = true;
                    events.push(ReaderEvent::Status(ReaderStatus::Connected));
                }

                let present = es.contains(State::PRESENT);
                if present && !self.card_present {
                    self.card_present = true;
                    match self.read_uid(&selected) {
                        Some(uid) => events.push(ReaderEvent::Scan {
                            reader: reader_name,
                            uid,
                        }),
                        None => events.push(ReaderEvent::ReadError {
                            reader: reader_name,
                        }),
                    }
                } else if !present {
                    self.card_present = false;
                }
            }
        }

        // Zustände für nächsten Zyklus persistieren
        self.pnp_state = new_pnp_state;
        self.selected_state = new_selected_state;

        events
    }
}

#[cfg(test)]
mod hw_tests {
    use super::*;

    // Echter Hardware-Test. Läuft NICHT in CI.
    // Ausführen mit angeschlossenem Leser + aufgelegter Karte:
    //   NFC_HW_TEST=1 cargo test reader_pcsc::hw_tests -- --ignored --nocapture
    #[test]
    #[ignore]
    fn hw_reads_a_uid_from_first_reader() {
        if std::env::var("NFC_HW_TEST").is_err() {
            return;
        }
        let mut r = PcscReader::new().expect("establish PC/SC context");
        let readers = r.list_readers();
        assert!(!readers.is_empty(), "kein PC/SC-Leser gefunden");
        println!("Leser: {readers:?}");
        r.set_selected(Some(readers[0].clone()));

        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        let mut got_uid: Option<Vec<u8>> = None;
        while std::time::Instant::now() < deadline && got_uid.is_none() {
            for ev in r.poll(Duration::from_millis(400)) {
                if let ReaderEvent::Scan { uid, .. } = ev {
                    got_uid = Some(uid);
                }
            }
        }
        let uid = got_uid.expect("keine Karte innerhalb 15s gelesen");
        println!(
            "UID: {}",
            uid.iter().map(|b| format!("{b:02X}")).collect::<String>()
        );
        assert!((4..=10).contains(&uid.len()));
    }
}
