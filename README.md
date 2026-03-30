Based on [GoXLR Utility](https://github.com/GoXLR-on-Linux/goxlr-utility)

## MacXLR

MacXLR is a macOS-focused fork of GoXLR Utility for configuring and controlling a TC-Helicon
GoXLR or GoXLR Mini. It is based on the original
[GoXLR Utility](https://github.com/GoXLR-on-Linux/goxlr-utility), but this fork is intended to
evolve independently with macOS-specific fixes, packaging, and quality-of-life improvements.

## Features

* Full control over the GoXLR and GoXLR Mini (Similar to the official App)
* Compatibility with profiles created by the official application
* An accessible UI designed to work well with Assistive Technologies
* Remote Access. Control your GoXLR from another computer on your network
* A Sample 'Pre-Buffer'. Record audio from before you press the button
* Exit Actions, including saving profiles and loading other profiles / lighting
* Multiple Device Support. Run more than one GoXLR on one PC
* A CLI and API for basic or advanced scripting and automation
* Streamdeck Integration (
  through [The StreamDeck Repository](https://github.com/FrostyCoolSlug/goxlr-utility-streamdeck))

## Downloads

MacXLR is currently being developed as an independent fork for macOS. If you publish releases for
this fork, point users at your own GitHub Releases page rather than the upstream GoXLR Utility
releases.

For local development and daily use on macOS, the simplest path is the included local app bundle
script:

```bash
./scripts/build-local-macos-app.sh
open "dist/MacXLR.app"
```

Attribution note:

MacXLR remains based on the original GoXLR Utility project, but the packaging, fixes, and fork
direction in this repository are intentionally separate from upstream.

## Integrations

* [twitchat](https://twitchat.fr/) - Activate and change GoXLR settings based on twitch bits / donations (Thanks Durss!)
* [MacroGraph](https://www.macrograph.app/) - A visual programmer for Streamers. (Thanks JDUDE!)
* [OBS Fader Sync](https://github.com/parzival-space/obs-goxlr-fader-sync-plugin) - An OBS plugin to sync pre-mix
  volumes to fader volumes (Thanks parzival!)
* [Home Assistant](https://github.com/timmo001/homeassistant-integration-goxlr-utility) - A plugin that lets you tie the
  GoXLR into your home automation (Thanks timmmo!)

## Getting Started

Once installed, you can launch the Utility using the `MacXLR` item in your Applications Menu, this will launch
the utility and configuration UI. The UI will then be accessible via the system tray icon, or (if you don't have a tray)
by re-running the `MacXLR` menu item.

If you're running on Linux, a first configuration step should be to enable `Autostart on Login`
via System -> Settings. Windows users will get the choice during installation. If you change your
mind, you can change the setting.

If you want to import your profiles from the official app, simply click on the folder icon in the top right of the
relevant profiles pane (either Main or Mic) which will open the directory in your file browser. Copy the profile across
from the Official App's directory (normally `Documents/GoXLR`) and they'll appear in the util ready to load, simply
double click them.

If you're setting up from scratch, the best place to start is configuring your microphone. Head over to the `Mic` tab
and hit `Mic Setup` to configure your microphone type and gain. It may be easier to configure if you first set your
Gate Amount to 0, then reconfigure it once your mic is working. Once done, go explore the UI!

## The UI

The Utility's UI is web based and served directly from the utility to your web browser of choice (if configured, it
can also be served to a web browser on another computer). The Utility also provides an 'Application' which wraps the
web UI into a dedicated app. If you're using the Utility on Windows this option is presented to you during install.
The UI design was modelled around the official application in an attempt to provide a familiar interface for those
moving from Windows to other platforms, rather than forcing people to learn a new configuration paradigm.

![image](https://github.com/GoXLR-on-Linux/goxlr-utility/assets/574943/8f14bd2c-e67a-42e5-bd9f-b3cb367e171d)

If you're running on Linux, the 'Application' isn't provided as part of the base utility installation. If you'd
prefer to use it, check out the [GoXLR UI Repository](https://github.com/frostyCoolSlug/goxlr-utility-ui/), which
provides various builds for distributions. Once installed, you should be able to go to System -> Utility Settings
and change the UI Handler there.

## Building

Build instructions for the original project can still be useful as reference:
[GoXLR Utility compilation guide](https://github.com/GoXLR-on-Linux/goxlr-utility/wiki/Compilation-Guide).
MacXLR may diverge from upstream packaging and macOS behavior over time.

### Local macOS App Bundle

If you're working on the utility locally on macOS and want a simple `.app` bundle for daily use,
this repo now includes a local packaging script:

```bash
./scripts/build-local-macos-app.sh
open "dist/MacXLR.app"
```

This produces a local app bundle containing the Rust binaries from `target/release/`. It does not
build the separate Tauri desktop UI project or a signed `.pkg`, but it gives you a native-feeling
launcher for local testing and personal use on macOS.

## Disclaimer

This project is also not supported by, or affiliated in any way with, TC-Helicon. For the official GoXLR software,
please refer to their website.

In addition, this project accepts no responsibility or liability for use of this software, or any
problems which may occur from its use. Please read the [LICENSE](LICENSE) for more information.
