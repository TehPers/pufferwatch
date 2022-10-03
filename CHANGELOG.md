# Changelog

Changelog for [pufferwatch](https://github.com/TehPers/pufferwatch). This changelog follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased

### Added

- Added a changelog. (TODO)

### Changed

- Improved the performace of the `--follow` flag by reducing the number of times the log files is read. ([#26])

### Fixed

- Fixed the `--output-log` log file not being truncated. ([#26])

[#26]: https://github.com/TehPers/pufferwatch/pull/26

## 0.2.0 (2022-09-30)

### Added

- Improved documentation from `--help`.
- Added documentation on how to use pufferwatch to replace SMAPI's terminal in Steam.
- `pufferwatch run` (previously `pufferwatch --execute`) now searches for SMAPI if not provided a path.

### Changed

- Restructured the command interface. Rather than using mutually exclusive `--remote`/`--execute`/`--stdin`/etc. flags, Pufferwatch now uses subcommands. For example, `pufferwatch --remote <url>` has been changed to `pufferwatch remote <url>`. This reduces confusion on when certain flags are allowed.
- Updated to clap v4, meaning the help output is no longer colored and instead uses the new style.

## 0.1.2 (2022-05-16)

### Added

- View remote SMAPI logs using `--remote`
- Start SMAPI from pufferwatch using `--execute` and send it commands

### Changed

- Formatted log and raw log are now tabs that can be switched between. This makes the widgets larger.
- Linux target now uses GNU instead of MUSL

### Fixed

- Incorrect control icon on Mac
- Divide-by-zero panic when making the terminal too small

## 0.1.1 (2021-11-08)

### Added

- Auto-scroll - when either the `log` or `raw` views are at the bottom of a log, if more content is added to that log, then the view will automatically scroll down.
- Read from `stdin` - pipe files or even SMAPI directly into `pufferwatch`! Usage: `pufferwatch --stdin`

## 0.1.0 (2021-11-08)

Initial pre-release version.
