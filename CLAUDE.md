# Notes for Claude

## Release flow (must keep working)

- All app work goes to `main`. Every push to `main` runs
  `.github/workflows/release.yml` on `windows-latest`, which tests, builds
  `cargo build --release`, packages `Brick-Builder/` (brick-builder.exe +
  `packaging/README.txt` + `assets/` if it exists) into
  `Brick-Builder-windows.zip`, and publishes it as release `v0.1.<run number>`,
  marked as latest.
- The README links to
  https://github.com/ShelaShelby/brick-builder/releases/latest/download/Brick-Builder-windows.zip
  — keep the zip name and the release flow so that link never breaks.
- The user (Alex) is not a programmer and does not use a terminal: every update
  must be downloadable from that link. After pushing, check that the Actions run
  succeeded and the new release has the zip; fix the workflow or build if not.
- The exe must run by double-click with nothing installed: keep the static CRT
  setting in `.cargo/config.toml` and `windows_subsystem = "windows"` in
  `src/main.rs`. If the app ever needs runtime files, put them in `assets/`
  (they are copied into the zip automatically).
- Keep `cargo test --release` green; the workflow runs the tests before releasing.
- When controls change, update both `README.md` and `packaging/README.txt`.
