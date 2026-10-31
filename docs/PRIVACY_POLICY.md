# Privacy Policy

Last updated: October 31, 2026

Wirecraft is a desktop application for designing dataflow programs and generating
code from them. It works offline, needs no account, and contains no telemetry,
analytics, advertising or crash reporting. The developer receives no data from it.

## What data Wirecraft collects

None. Wirecraft does not send your projects, settings or usage information to the
developer or to anyone else.

## What data Wirecraft stores on your device

- **Project files** (`.orch`) that you create, open or save, and the code you
  generate or export (source files, PDF, SVG, PNG, Mermaid) in folders you choose.
  Projects can contain personal information you enter, such as the people named
  as responsible or assignee on the project board and in revision history.
- **Settings and session state** in Wirecraft's state directory: `settings.json`,
  the editor session and unapplied drafts (`ui.json`) and a crash-recovery copy of
  the open project (`recovery.json`). On Linux this is `$XDG_STATE_HOME/wirecraft`
  or `~/.local/state/wirecraft`; elsewhere it is `.local/state/wirecraft` under
  your home directory (`HOME`), or under the temporary folder if `HOME` is not
  set. `--state-dir` chooses another directory.

Deleting these files removes this data; uninstalling Wirecraft may leave them in
place. Wirecraft never changes Threadworks' preferences; importing them is an
explicit command that excludes identity and AI credentials.

## Network access

- The application runs its own backend on your computer, bound to `127.0.0.1`
  only and protected by a random access token created for each run. It is not
  reachable from other computers. A short request log (time, method, status,
  duration) is kept in memory and discarded when the application closes.
- Wirecraft makes no requests to the internet. Network AI providers listed in
  Options are not implemented: AI assistance copies a prompt to your clipboard,
  and nothing is sent unless you paste it somewhere yourself.
- **Printing** submits a job only when you choose Print, to a printer configured
  on your system (CUPS on Linux and macOS).

## Tools and clients you connect

- **Compilers and language servers** (clang, clangd, rust-analyzer, Go, Node.js
  and others) are separate programs installed by you. Wirecraft runs them locally
  on the code it generates; their own privacy practices apply.
- **MCP clients** (for example an AI coding agent) can read and change the open
  project when you give them the local endpoint and token, or start
  `wirecraft mcp-stdio`. What such a client does with the data, including sending
  it to an AI service, is governed by that client's privacy policy.

## Children

Wirecraft is a software development tool and is not directed at children. It
collects no personal information from anyone.

## Changes to this policy

This policy may change as the application evolves. Material changes are recorded
in this file's history and in the release notes.

## Contact

For privacy questions, open an issue at
<https://github.com/alfu32/wirecraft/issues>.
