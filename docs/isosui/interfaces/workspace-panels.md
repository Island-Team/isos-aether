# U03 workspace panel contract

One persistent EOS workbench owns the live control, monitor and Vision projections across Graph/EOS navigation. One Logs consumer owns the P07 cursor. Dock/collapse keeps panel content mounted; close hides Control while keeping its current-run command record in memory and unmounts Logs so reopen must recheck IsOS status and current scope. A hidden or collapsed Monitor, Vision or Logs panel does not poll or stream; the control panel pauses automatic result reads while hidden. All data remains fenced by owner run and active revision.

**Vision provenance implemented:** The [Vision provenance display contract](#vision-provenance) adds a run/revision/stream/generation fence and same-byte digest verification to the Vision panel. Hiding or stopping the panel clears its current evidence and prevents late frames from repainting it. The Q03 final-17 and Q04 final-5 replays observed rendered frames on their separately pinned inputs; final-5 used a simulated hosted Origin on loopback.

## Vision provenance

The decoder now limits metadata to 4096 bytes and validates the selected owner fields and JPEG digest before display. The panel shows selected source, gate, run, stream, generation, digest, subscription cap, delivered FPS, and locally observed skipped generations. It does not claim EOS router-wide cap or gap totals. Focused tests cover positive and negative frame decisions plus a mounted Stop/late-digest boundary. A selected-pin browser replay remains Q01 qualification work. The UI has no image upload, prompt, or actuator path from this display.

**Historical design-round verification, 2026-11-02:** `npm ci`, `npm test` (179 passed), and `npm run build` (local and hosted) passed. Changed-page local links passed a read-only check; `git diff --check` passed. Complete output is in ignored `data/test/20261102-q01-camera-design/`. These checks validated the UI baseline at that design round; later implementation evidence follows below.

### Public browser endpoint

**Contract repair verification, 2026-11-02:** `python3 tests/fixtures/q01_vision_display/check_cases.py` validated 12 frozen cases. `npm test` (179 passed), `npm run build` (local and hosted), and `git diff --check` passed. Changed-page links passed the read-only check. Full command output is in ignored `data/test/20261102-q01-contract-repair/`; no live browser frame replay is claimed.

**Historical implementation verification:** `npm test`, `npm run build` for local and hosted, fixture structure check, and `git diff --check` are recorded under ignored `data/test/20261103-q01-vision-ui/`. That slice's mounted test used a stub EOS socket; this record does not claim a cross-owner Q01 replay.

**Out-of-order digest repair, 2026-11-03:** The panel rechecks the live generation after SHA-256 resolves, before replacing the image. A mounted test resolves generation 2 before generation 1 and verifies that generation 2 remains displayed. `npm test` passed 186 tests across 41 files; `npm run build` passed TypeScript, local and hosted bundles, and manifest verification; the 12 design fixture checks and `git diff --check` passed. Complete output is in ignored `data/test/20261103-q01-vision-ui-race-repair/`.
