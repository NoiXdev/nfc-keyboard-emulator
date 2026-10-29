## [1.1.0](https://github.com/NoiXdev/nfc-keyboard-emulator/compare/v1.0.1...v1.1.0) (2026-10-29)


### Features

* **integration:** add configurable token transport ([7f93435](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/7f93435ac38e1ff833b175c2a5e16e964363b569))
* **integration:** add scan request slot and answer requests in the worker ([c01b0d8](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/c01b0d828791e75253c291ea80efc231792f12fe))
* **integration:** add server config file format and link decoding ([657a9e9](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/657a9e99cdfecd5d4d5be931e640e0e38d24823b))
* **integration:** add server integration settings to config ([2a9d9db](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/2a9d9dbca2596f517be811e4eb47d09518572590))
* **integration:** add websocket stream and server scan requests ([68e4fee](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/68e4fee02371757295cb7ddade7212ec3bf8b5f8))
* **integration:** apply auth and payload settings to the websocket ([1a843c5](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/1a843c5226096d34fa8c5ca878ad2b5def517142))
* **integration:** carry auth and payload settings through import and export ([f21d591](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/f21d59148e3d153a3f996225362142f7be84382d))
* **integration:** define server protocol messages and reconnect backoff ([e18596d](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/e18596d8212edf679e2e83724c3f8e8449aebb1f))
* **integration:** deliver scans by HTTP push ([e112227](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/e1122274ff923885cf773ea0412f527f32cb934c))
* **integration:** import and export server settings by file and link ([69f0782](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/69f0782110cd2a8184cafabc3f9c03e1b5779742))
* **integration:** render custom JSON payload templates ([3517177](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/35171773c4954a428e61c1b63374c38dfcb35ec7))
* **integration:** route scans to http and websocket targets ([1dd2011](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/1dd201168c707944b00a9659470ab0cf37f8ec7b))
* **integration:** send HTTP scans with the configured auth and payload ([b604f78](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/b604f786eca6149e5955e97836445dea8c0fd076))
* **integration:** store auth and payload settings per target ([a24c96a](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/a24c96a27671a9e523593fbc1b9041da8345e391))
* **scan:** add scan id, formatted uid and request id to scan records ([7412889](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/74128897d7cb891a8018ba7c2058f5a74ed8359c))
* **ui:** add advanced auth and payload settings to the server view ([8ef250f](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/8ef250ff50ca79e22de6e447c37fb0e03523a263))
* **ui:** add server settings view and import preview ([b05a0f1](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/b05a0f1e033e2c3cd072be6a8862ffe72db596b4))
* **ui:** add template renderer and error texts for advanced settings ([11712ad](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/11712ad9b9eb8bcdfa5d8370347a16c2a94784ed))
* **ui:** always show the server status in the footer ([c66c3c0](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/c66c3c04b07305222baca837cb0ffff2dd959d66))
* **ui:** show auth and payload changes in the import preview ([8fede4f](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/8fede4ffeb6b3123dad3a5eb7fc69e4f6ec1cc0a))
* **ui:** show server delivery in the scan log and websocket state in the status bar ([30e0d15](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/30e0d15f7f4dd7c9f034196681b897be2fd226b7))


### Bug Fixes

* **examples:** keep the example server alive on bad clients ([38c5bd2](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/38c5bd2ec20eb45573e10f904a5226c519f9e4ab))
* **examples:** only end requests owned by the closing app ([d3c5146](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/d3c5146f4218b4885418d43d5f7f1ac675edcd36))
* **integration:** also stream scans that answer a request ([aa973ff](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/aa973ff29847a6d7425045ed39fccfc937c5f277))
* **integration:** always export auth and payload settings ([5ee77d2](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/5ee77d2b2b9478062b1884140587d82e9352e21c))
* **integration:** detect dead websocket peers ([1959e35](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/1959e35ae8e94ef065c8d9159f8f503488608fdc))
* **integration:** do not follow redirects and use ring for TLS ([12197fa](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/12197fafdd469b659a988eed1a44e5a631f57026))
* **integration:** only reset reconnect backoff after a stable session ([3ce1962](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/3ce1962a6011309535ef85e53014b0473709cd50))
* **integration:** reject tokens with invalid characters ([9ddff92](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/9ddff925d8632e9b72f8eae794734af61fda7be7))
* **integration:** report invalid test auth and split import warnings ([d1e999e](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/d1e999e140646531dba002b3e18b57473ad929c3))
* **integration:** route each scan from one config snapshot ([15a2739](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/15a27399c4315da1c0b8472fd759bf812459d989))
* **integration:** send payload timestamps in UTC with milliseconds ([3aac75d](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/3aac75d40594bab820b76631035bd045329df878))
* **integration:** show errors from links that start the app ([a03167b](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/a03167bdb0649ae992f7050589519c0fc3247d50))
* **integration:** show host changes in the import preview ([7392b1f](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/7392b1f17d84eb6df8c9507c207c6e51ac081e06))
* **ui:** correct auth error texts, import warnings and footer state ([75686df](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/75686df4735d9442e83bb8fd7cdad531af34f463))
* **ui:** dedupe scan-request notices and ignore stale test results ([ebb9f4e](https://github.com/NoiXdev/nfc-keyboard-emulator/commit/ebb9f4e7a945754cc0283f449e1d1062da3f0d4b))

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

