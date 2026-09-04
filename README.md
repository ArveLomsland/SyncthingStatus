# SyncthingStatus

Liten Rust-app som viser status for en lokal Syncthing-instans som ikon i systemkurven.
Fungerer på **Windows** og **Linux** (Ubuntu m.fl.).

## Funksjoner

- Fargekodet ikon med status:
  - 🟢 grønn hake – alt er oppdatert
  - 🔵 blå roterende piler – synkroniserer / skanner (animert)
  - 🟡 gul pause – alle mapper pauset
  - 🔴 rødt utropstegn – feil i mappe eller systemfeil
  - ⚪ grått kryss – Syncthing svarer ikke
- Tooltip (Windows) / tittel (Linux) med versjon, tilkoblede enheter, gjenstående data og ned-/opplastingshastighet
- Meny med status per mappe (fullførelse i %, gjenstående data, pauset/skanner/feil)
- Menyvalg: åpne Web GUI, skann alle mapper, oppdater status nå, avslutt
- Venstreklikk på ikonet åpner Syncthing Web GUI (Windows)
- Finner adresse og API-nøkkel automatisk fra Syncthing sin `config.xml`
- Setter selv ikonet som «alltid synlig» på oppgavelinjen i Windows 11

## Installering

### Windows

Last ned `syncthing-status-<versjon>-setup.exe` og kjør den. Installeres per bruker
(ingen administratorrettigheter), med valgfri autostart ved innlogging.

Stille installasjon:

```powershell
.\syncthing-status-0.1.0-setup.exe /SILENT /TASKS=startup
```

Avinstalleres fra Innstillinger → Apper, eller `unins000.exe /SILENT` i installasjonsmappen
(`%LOCALAPPDATA%\Programs\SyncthingStatus`).

### Ubuntu / Debian

```bash
sudo apt install ./syncthing-status_0.1.0_amd64.deb
```

Pakken legger inn `/usr/bin/syncthing-status`, menyoppføring og autostart
(`/etc/xdg/autostart/syncthing-status.desktop`).

> **GNOME-brukere:** GNOME (standard i Ubuntu) har ikke systemkurv innebygd.
> Installer utvidelsen **AppIndicator and KStatusNotifierItem Support**:
> ```bash
> sudo apt install gnome-shell-extension-appindicator
> ```
> Logg ut/inn, og aktiver den i Extensions-appen. KDE, XFCE, Cinnamon og MATE
> virker uten tillegg.

## Bygging

### Windows

```powershell
cargo build --release
# eller inkludert installer:
powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1
```

Installeren krever Inno Setup: `winget install JRSoftware.InnoSetup`

Denne maskinen mangler MSVC C++-verktøy, så prosjektet er satt opp med GNU-toolchain
(`rustup override set stable-x86_64-pc-windows-gnu`) og MinGW fra WinLibs. Byggeskriptet
legger MinGW i `PATH` automatisk. Installeres MSVC «C++ build tools» senere, kan overriden
fjernes med `rustup override unset`.

### Ubuntu / Debian

```bash
sudo apt install build-essential pkg-config libgtk-3-dev libayatana-appindicator3-dev
cargo build --release
# eller .deb-pakke:
./packaging/linux/build-deb.sh
```

GitHub Actions (`.github/workflows/release.yml`) bygger både installer og .deb,
og publiserer dem som release-filer når du pusher en `v*`-tag.

## Konfigurasjon

Alt er valgfritt – appen leser normalt Syncthing sin egen `config.xml`
(`%LOCALAPPDATA%\Syncthing\config.xml`, evt. `%APPDATA%\Syncthing\config.xml`,
`~/.local/state/syncthing/config.xml`, `~/.config/syncthing/config.xml`,
`~/Library/Application Support/Syncthing/config.xml`).

| Miljøvariabel         | Beskrivelse                         | Standard         |
| --------------------- | ----------------------------------- | ---------------- |
| `SYNCTHING_URL`       | Adresse til GUI/API                 | fra `config.xml` |
| `SYNCTHING_APIKEY`    | API-nøkkel                          | fra `config.xml` |
| `SYNCTHING_POLL_SECS` | Sekunder mellom statusoppdateringer | `3`              |
| `SYNCTHING_HOME`      | Katalog med `config.xml`            | OS-standard      |

Selvsignerte HTTPS-sertifikater (Syncthing med `tls="true"`) godtas.

## Kommandolinjeflagg

```bash
syncthing-status --status        # hent status én gang og skriv til konsollet
syncthing-status --preview       # vis ikonene som ASCII-kunst
syncthing-status --no-promote    # ikke fest ikonet på oppgavelinjen (Windows 11)
```

## API-endepunkter som brukes

`/rest/system/version`, `/rest/config/folders`, `/rest/config/devices`,
`/rest/system/status`, `/rest/db/status`, `/rest/system/connections`,
`/rest/system/error`, `/rest/db/scan` (POST).

## Prosjektstruktur

| Fil                                | Innhold                                          |
| ---------------------------------- | ------------------------------------------------ |
| `src/main.rs`                      | Tray-ikon, meny, event loop, bakgrunnstråd       |
| `src/syncthing.rs`                 | REST-klient og statusmodell                      |
| `src/config.rs`                    | Oppdaging av adresse/API-nøkkel                  |
| `src/icon.rs`                      | Ikoner generert i kode (ingen bildefiler)        |
| `src/platform.rs`                  | Win32- og GTK-meldingsløkke, feildialog          |
| `src/win_promote.rs`               | Fester ikonet på oppgavelinjen (Windows 11)      |
| `packaging/windows/*.iss`, `*.ps1` | Inno Setup-installer                             |
| `packaging/linux/*`                | .desktop-filer, ikon og `.deb`-bygging           |

## Status for plattformene

| Plattform | Status |
| --------- | ------ |
| Windows 10/11 | Bygget og testet (Syncthing v2.1.3) |
| Ubuntu/Debian | Kode og pakking klar, men **ikke testet** – mangler Linux-maskin |
| macOS | Kompilerer sannsynligvis, men ingen event loop er implementert |
