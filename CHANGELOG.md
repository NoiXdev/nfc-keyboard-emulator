# Changelog

## 1.0.0

First public release.

### Features

- Read NFC card UIDs from any PC/SC reader (verified with the ACS ACR122U) and type
  them into the focused input field — a software keyboard wedge.
- Configurable output format: upper/lower case, separator, byte order, prefix, and
  trailing key (Enter / Tab / none), with a live preview.
- Live scan log with timestamp, reader, UID and status; CSV export.
- Scanner selection with hot-plug detection and an arm/disarm typing safety toggle.
- System tray with close-to-tray behaviour and optional launch on login.
- macOS (signed + notarized) and Windows builds.
