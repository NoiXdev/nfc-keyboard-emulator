## [1.0.1](https://github.com/NoiXdev/nfc-keyboard-emulator/compare/v1.0.0...v1.0.1) (2026-09-16)


### Bug Fixes

* run a single instance only (single-instance guard + macOS dock reopen) ([5a839fe](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/5a839fedd93fb82519a13f11f266bb6dbaabb2d6))
* stop the reader worker spinning when the PC/SC context dies ([fc8e1a8](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/fc8e1a88f2a755cb6d88b17579083d38f9ca3131))
* use a real template image for the macOS tray icon ([7dc409f](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/7dc409f91839a85fdff677f05684f8b7470893bf))

## [1.0.0](https://github.com/NoiXdev/nfc-keyboard-emulator/compare/9a196b3c52b4944bbcd7fe6128435e9410fd8a83...v1.0.0) (2026-06-20)


### Features

* About view with logo, noix.dev link, app license and third-party OSS licenses ([a9d24ea](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/a9d24ea692bd83f6bb375af2bd8e32317947ee81))
* add app logo and tray glyph (card + waves, teal) ([f59f1c2](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/f59f1c233fc90ff35c46818ca5dc3d0139179e09))
* add app state and store-backed config persistence ([3943091](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/3943091578a7c73d3baa2d770d63a4aa410b22e0))
* add config types with serde defaults and camelCase ([9a196b3](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/9a196b3c52b4944bbcd7fe6128435e9410fd8a83))
* add CSV export for scan log ([9ae8467](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/9ae8467cc07e8b69cae230f8225b1195ea0deabb))
* add macOS accessibility check and settings prompt ([1e5457b](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/1e5457bb695d7ec8fa51e324bc050aff2eb65e3c))
* add PC/SC reader implementation with UID read via APDU ([e3271d7](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/e3271d7ab4d5da573768325ee198d011d9b7ec11))
* add ReaderBackend trait, events, and mock reader ([218982d](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/218982df6b7d5112f5bb6f64ea3420c795b3cf07))
* add scan handling pipeline with typing decision logic ([018ccfe](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/018ccfe7fc7b4a5e7c3613142789643bdb1d3e56))
* add scan log ring buffer ([d041236](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/d041236eaf7ee769c060d232a6be710654544901))
* add scan record and status types ([b43b4d9](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/b43b4d9b6741a860ef456b67058bf4003898925f))
* add tauri commands for config, readers, format, csv, a11y ([c732b63](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/c732b633cac939b5b0bafa59c363108eea061640))
* add typed tauri command and event bindings ([8db9e63](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/8db9e634c24c1cf0a775765dd7f9cc4f05486023))
* add Typer trait with enigo, null, and mock impls ([b8eec86](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/b8eec8604010a5a53705af171f97222425e3d875))
* add UID formatter (case, separator, byte order, prefix) ([d3d61a3](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/d3d61a3c710cdf0023b7190d930f1cfa4dd9d0c3))
* add worker loop with sink abstraction ([3d7bd28](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/3d7bd2814c4cfc16161ea5ad0693e86e92a07838))
* branded sidebar navigation (Scanner/Log, Optionen/Bedienungshilfen) with in-app logo ([ca6067e](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/ca6067e14d9a1ee8141f55852657041946a2049d))
* build UI components and wire frontend to backend ([6742abe](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/6742abe93d6604dd30ee0e866ad3e341f1158291))
* i18n (de/en) with language switch, custom JS select + toggle, translated status labels ([641bb6a](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/641bb6a1ea948431887c6e37942b1b55eceacc84))
* native scan notifications (off / errors only / all) with status and UID ([79fe80b](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/79fe80bb1ef69c707138eeb17c11a9f8b5ff0862))
* rename app to "NFC Keyboard Emulator" and add startup update check (About) ([9a1d899](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/9a1d899002464eefa5a868b63f28a9841abae31f))
* wire builder, plugins, pcsc worker thread, and tray ([47dd5a8](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/47dd5a8f8310f1f63e29a737d650464eda92fc27))


### Bug Fixes

* correct tauri notarization env (APPLE_API_KEY=key id, APPLE_API_KEY_PATH=p8 file) ([54b0836](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/54b083689878302202a5944b2a699f9891d1bc1d))
* install x86_64 apple target and disable fail-fast in release workflow ([6f2d4c6](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/6f2d4c6488f5c5ff69cf202626f1cdedda7ee1ba))
* persist PC/SC reader state across polls to avoid busy-loop and event spam ([f7efb25](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/f7efb25e11d6fbb2b857c1d9fbac332896e808ed))
* resolve linux-only clippy lints (unused_mut tray, dead_code NoAccessibility) ([703d36a](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/703d36a7beb69f9ed03937b202c97da807aeada3))

