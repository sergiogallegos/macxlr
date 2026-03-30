# MacXLR Handoff

## Current State

- Fork rebranded to `MacXLR`
- GitHub repo renamed and pushed to `sergiogallegos/macxlr`
- macOS app identity updated
- macOS local app bundle script added
- CoreAudio GoXLR detection improved
- macOS aggregate device creation now working

## Recent Commits

- `aebb350c` Rebrand fork to MacXLR and improve macOS support
- `65675a11` Polish MacXLR project metadata and docs

## Rebrand Changes

- app name changed to `MacXLR`
- macOS bundle identifiers changed to `com.sergiogallegos.macxlr`
- macOS support/config path changed to:
  - `~/Library/Application Support/com.sergiogallegos.MacXLR`
- macOS package project renamed to:
  - `ci/macos/MacXLR.pkgproj`
- README updated for fork identity, support links, and releases

## Runtime / Audio Fixes Done

- removed several panic paths and `unwrap()`-style failure points
- fixed daemon/UI state sync issue in primary worker
- improved sampler playback and recording error propagation
- added retry behavior for stale CoreAudio playback devices
- added recorder rebuild/retry behavior for recording
- improved macOS CoreAudio device discovery with fallback enumeration
- reduced noisy repeated CoreAudio detection logging
- aggregate device creation now logs created devices

## Verified Working

- daemon starts correctly on macOS
- web UI loads at `http://localhost:14564/`
- GoXLR USB control works
- physical faders/buttons update in UI and daemon logs
- GoXLR CoreAudio device is detected
- aggregate devices are created:
  - `System`
  - `Game`
  - `Chat`
  - `Music`
  - `Sample`
  - `Stream Mix`
  - `Chat Mic`
  - `Sampler`
- QuickTime can see and use the input path
- mic capture appears to work
- headphones appear to work

## Still To Investigate

- QuickTime playback through GoXLR headphones seems quieter than expected
- playback through Mac speakers sounds louder than through GoXLR headphones
- Apple Music playback sounded mostly OK at full volume

## Likely Next Debug Target

Focus on output routing / monitoring calibration, not mic capture.

Most likely areas:

- QuickTime output device mapping on macOS
- GoXLR `System` / `Music` / `Headphones` routing interaction
- per-app playback path differences between QuickTime and other apps
- monitor/output gain behavior for GoXLR aggregate outputs

## Suggested Next Session Tests

1. Use the same recorded file in QuickTime and VLC.
2. Select the same macOS output device for both tests.
3. Check which GoXLR fader affects each app.
4. Compare playback on:
   - GoXLR headphones
   - Mac speakers
5. Verify `Headphones` level and routing in the MacXLR UI.
6. If needed, add more targeted logging around playback/output mapping.

## Useful Commands

Run daemon with logs:

```bash
pkill -x goxlr-daemon
RUST_LOG=debug ./target/release/goxlr-daemon --start-ui
```

Build local app bundle:

```bash
./scripts/build-local-macos-app.sh
open "dist/MacXLR.app"
```

## Notes

- Local folder name does not need to be renamed for the fork to work.
- Repo remote now points to `git@github.com:sergiogallegos/macxlr.git`.
- `cargo check` was passing at the end of the last polish pass.
