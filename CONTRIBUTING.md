# Contributing

Thanks for your interest in nfc-keyboard-emulator! A few conventions keep the
project tidy.

## Development setup

Prerequisites: [Rust](https://rustup.rs) (stable) and [Node.js](https://nodejs.org) 20+.

```bash
npm ci
npm run tauri dev
```

## Checks (run before opening a PR)

```bash
npm test                                   # frontend unit tests (vitest)
cd src-tauri
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

CI runs the same on Linux, macOS and Windows, plus a full `tauri build` on macOS
and Windows.

## Commit messages

This repository uses [Conventional Commits](https://www.conventionalcommits.org)
(`feat:`, `fix:`, `docs:`, `ci:`, `chore:`, …). The version number and
`CHANGELOG.md` are derived from them automatically on release.

## Manual pre-release checklist (with real hardware)

Run on macOS **and** Windows with an ACS ACR122U (or a compatible 13.56 MHz PC/SC
reader). A hardware UID-read smoke test is available behind a flag:

```bash
NFC_HW_TEST=1 cargo test --manifest-path src-tauri/Cargo.toml reader_pcsc::hw_tests -- --ignored --nocapture
```

- [ ] Reader appears in the dropdown after plugging it in.
- [ ] Selecting it and arming typing types a card UID into a focused text field.
- [ ] The trailing key (Enter / Tab / none) behaves as configured.
- [ ] Disarming stops typing (the log still records scans).
- [ ] Unplugging/replugging the reader toggles the status indicator.
- [ ] Both 4-byte and 7-byte UIDs read correctly.
- [ ] An unknown/incompatible card logs a read error without crashing.
- [ ] CSV export writes a valid file.
- [ ] Closing the window hides it to the tray; Quit exits.
- [ ] (macOS) The Accessibility banner and "open settings" button work.
