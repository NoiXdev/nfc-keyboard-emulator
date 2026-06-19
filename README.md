# NFC-Keyboard-Emulator

Internes Desktop-Tool (macOS + Windows): NFC-Karte auf einen PC/SC-Leser legen → UID lesen → automatisch in das fokussierte Eingabefeld tippen (Keyboard-Wedge-Prinzip).

**Tech-Stack:** Tauri v2, Rust (pcsc, enigo), Vue 3 + TypeScript.

---

## Installation

### macOS

**Signiertes DMG (empfohlen):** Das `.dmg` aus den [GitHub Releases](../../releases) herunterladen, mounten und die App in den Programme-Ordner ziehen. Die App ist notarisiert — Gatekeeper öffnet sie direkt.

**Unsigniertes Build (Entwickler-Direktbuild):**
```bash
npm ci && npm run tauri build
# .dmg liegt in src-tauri/target/release/bundle/dmg/
```
Beim ersten Start ggf. Rechtsklick → „Öffnen" (Gatekeeper-Bypass einmalig bestätigen).

### Windows

Das `.msi`- oder `.exe`-Installationsprogramm aus den [GitHub Releases](../../releases) herunterladen. Solange das Binary **unsigniert** ist, zeigt Windows SmartScreen eine Warnung — „Weitere Informationen" → „Trotzdem ausführen" um fortzufahren.

---

## macOS-Bedienungshilfen einrichten

Damit die App in andere Anwendungen tippen kann, benötigt sie Zugriff auf die **Bedienungshilfen** (Accessibility API).

1. **Systemeinstellungen** öffnen → **Datenschutz & Sicherheit** → **Bedienungshilfen**
2. Schloss unten links entsperren
3. **NFC-Keyboard-Emulator** in der Liste aktivieren (Häkchen setzen)

Falls der Zugriff fehlt, zeigt die App ein Warn-Banner mit einem Button „Einstellungen öffnen", der direkt dorthin navigiert.

---

## Bedienung

1. **NFC-Leser wählen:** Im Dropdown oben den angeschlossenen Scanner auswählen. „Aktualisieren" scannt neu angesteckte Geräte.
2. **Tippen aktivieren:** Großen Toggle-Button drücken → „Tippen AKTIV". Karte auflegen → UID wird automatisch getippt.
3. **Format anpassen:**
   - *Groß/Klein:* GROSS (Standard) oder klein
   - *Trennzeichen:* keins (Standard), `:`, `-` oder Leerzeichen
   - *Byte-Reihenfolge:* normal oder umgekehrt
   - *Präfix:* beliebiger Text vor der UID
   - *Abschluss:* Enter (Standard), Tab oder keins
   - Die Vorschau aktualisiert sich live anhand einer Beispiel-UID (`04A1B2C3`).
4. **Scan-Log:** Alle gescannten UIDs erscheinen in der Tabelle (neueste zuerst). „CSV exportieren" speichert das Log als Datei.
5. **Tray:** Fenster schließen legt die App in den System-Tray. „Beenden" im Tray-Menü beendet sie vollständig.

---

## Manuelle Real-Checkliste (vor Release)

Die folgende Checkliste ist vor jedem Release mit echter Hardware (ACR122U oder kompatibler PC/SC-Leser) auf macOS **und** Windows durchzuführen:

- [ ] Leser einstecken → erscheint im Dropdown
- [ ] Leser wählen → StatusBar zeigt „verbunden"
- [ ] 4-Byte-NFC-Karte auflegen → UID erscheint im Log **und** wird in TextEdit/Notepad getippt (mit Enter)
- [ ] 7-Byte-NFC-Karte auflegen → UID korrekt im log (14 Hex-Zeichen), korrekt getippt
- [ ] Toggle auf „Tippen aus" → nächste Karte erscheint im Log, aber es wird **nicht** getippt
- [ ] Leser abziehen → StatusBar „getrennt"; wieder anstecken → „verbunden"
- [ ] Unbekannte/fehlerhafte Karte → Eintrag mit Status `read_error` im Log
- [ ] macOS ohne Accessibility-Berechtigung → Banner erscheint; Button öffnet Datenschutz-Einstellungen
- [ ] CSV-Export → Datei-Speichern-Dialog öffnet, gespeicherte Datei enthält Kopfzeile und alle Log-Einträge
- [ ] Tray: Fenster schließen → App läuft weiter im Tray; „Anzeigen" bringt Fenster zurück; „Beenden" beendet App vollständig
- [ ] Autostart: aktivieren → App startet nach Neustart automatisch

---

## Hardware-Test (opt-in, mit angeschlossenem Leser)

Mit angeschlossenem Leser und aufliegender Karte:

```bash
NFC_HW_TEST=1 cargo test --manifest-path src-tauri/Cargo.toml reader_pcsc::hw_tests -- --ignored --nocapture
```

Der Test wartet bis zu 15 Sekunden auf eine Karte und gibt Leser-Name und UID aus.

---

## Entwicklung

```bash
npm ci
npm run tauri dev       # Dev-Modus mit Hot-Reload
cargo test --manifest-path src-tauri/Cargo.toml   # Rust-Unit-Tests (23 passed)
npm test                # Frontend-Tests (vitest)
npx vue-tsc --noEmit   # TypeScript-Check
```

---

## Lizenz

MIT — siehe [LICENSE](LICENSE)
