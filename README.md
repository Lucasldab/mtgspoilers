# MTG Spoiler TUI

Terminal UI for tracking Magic: The Gathering card spoilers with multi-source verification.

## Features

- 🟡🔵🟢⚪ 4-tier confidence system (Rumor → Official)
- Automatic deduplication via image hashing
- Reddit r/magicTCG integration with authenticity scoring
- SQLite storage with search and filtering
- Vim-style keybindings

## Installation

```bash
cargo build --release
./target/release/mtg-spoiler-tui
