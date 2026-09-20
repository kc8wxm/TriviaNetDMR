# TriviaNetDMR - Net Control Companion

TriviaNetDMR is a modern, highly interactive, and visually polished Terminal User Interface (TUI) application designed for Amateur Radio Net Control Stations (NCS) running trivia nets. It streamlines the check-in process, automatically pulls operator details from the QRZ.com XML Callbook database, tracks scores across multiple categories, and automatically handles fair participant rotation between trivia rounds.

## 🚀 Key Features
- **Modern Ratatui TUI:** Clean, responsive terminal dashboard layout with dedicated panes for the participant queue, active operator profile card, net statistics, and control shortcuts.
- **Asynchronous Background Lookups:** Powered by `tokio`. Checking in a callsign immediately appends them to the queue and triggers a non-blocking background lookup, ensuring the user interface remains completely smooth and responsive without freezing.
- **QRZ.com XML API Integration:** Automatically logs in, caches the session key, and fetches operator names and locations (city/state/country) on the fly.
- **Region-Aware Mock Fallback:** When run without QRZ credentials, the app enters a highly realistic offline mock mode. It dynamically parses callsign prefixes (both US regions 1-0 and international prefixes like `VE`, `G`, `DL`, `JA`, `F`) to generate appropriate names and locations.
- **Trivia Question & Answer Prompter:** Automatically parses and displays trivia decks from Markdown files (such as `Topic/Questions-1.md`). Shows current question prompt, canonical answer, and Net Control facts with answer reveal toggling (`a`) and question navigation (`[` / `]`), synced to rounds.
- **Trivia Queue Rotation:** Automatically rotates the active list of participants after each round, ensuring a fair starting position for all check-ins, while placing late check-ins at the end of the rotation.
- **Granular Scoring:** Tracks automatic check-in points (1pt), trivia answer points (1pt per correct answer), and custom bonus points (1pt for best/interesting answers) per participant.
- **Callsign Management:** Easily edit mistyped callsigns (`e`) with automatic QRZ re-lookup, or delete operators (`d`) from the active queue and roster with safe confirmation.
- **Interactive Final Scoreboard & Leaderboard:** View full contest standings at any time (`f`) in an interactive leaderboard popup with podium ranks (🥇, 🥈, 🥉), full score breakdowns, and smooth scrolling for nets of any size.
- **Contest Export (CSV & JSON):** Save contest results and final scoreboards at any time (`s`) to CSV spreadsheets or structured JSON data files, complete with ranks and participant statistics.
- **In-App Topic & Deck Switcher:** Browse and switch question decks on the fly (`t`) directly within the TUI from `Topic/*.md` without restarting the application or losing state.
- **Hot-Reload Decks:** Edit questions in an external editor during net prep and instantly hot-reload (`u`) from disk while preserving the participant queue, active round, and scores.
- **CLI & In-App Deck Validator:** Built-in validator (`--check` / `-c`) that analyzes Markdown question decks for format errors, missing answers, empty prompts, and duplicate or non-sequential numbers.

---

## 🛠 Project Architecture
The codebase is structured into five highly focused modules:
1. `src/state.rs`: Holds the pure domain models (`Participant`, `App`, `InputMode`, `ExportFormat`), scoring mutations, queue rotation, participant edit/delete logic, export generators, trivia question navigation, deck discovery, topic picker switching, and hot-reloading. Contains unit tests for rotation offsets, scoring, editing, deletion, exports, and deck management.
2. `src/trivia.rs`: Loads and parses Markdown question decks into structured topics, questions, answers, and Net Control facts, with robust Markdown syntax cleaning, deck validation diagnostics, and directory file scanners.
3. `src/qrz.rs`: Features the asynchronous `QrzClient`. Manages session-cached authentication, XML response parsing using `roxmltree`, and the region-aware mock generator.
4. `src/ui.rs`: Handles the layout rendering using `ratatui`. Draws the multi-column header (with round & question indicators), trivia question/answer banner, participant table, operator detail card, net summary stats (with status/error feedback), and modals for check-in, edit, delete, export, final scores, and topic deck picker.
5. `src/main.rs`: Coordinates startup, CLI argument handling (e.g. `--check` validator mode or specifying question decks), crossterm raw-mode initialization, event routing, background QRZ lookups, and graceful shutdown.

---

## ⚙ Setup & Configuration

### Prerequisites
Make sure you have Rust and Cargo installed:
```bash
cargo --version
```

### QRZ API Integration
To enable live lookups against the real QRZ.com database, export your credentials in your terminal session:
```bash
export QRZ_USERNAME="your_callsign"
export QRZ_PASSWORD="your_qrz_password"
```
*If these variables are omitted, the application automatically runs in **Offline Mock Mode**, seamlessly providing region-appropriate operator profiles for testing and offline operations.*

---

## 🎮 How to Run

### Run the Application
To compile and launch the interactive TUI dashboard:
```bash
cargo run --release
```

### Run Unit Tests
To execute the comprehensive test suite (testing scoring, queue rotation, XML parsing, callsign editing/deletion, and exports):
```bash
cargo test
```

---

## ⌨ Keyboard Controls & Shortcuts

The application features intuitive single-key controls:

| Key | Action | Description |
| :--- | :--- | :--- |
| `c` | **Check-in Operator** | Opens a modal popup to enter a callsign. Press `Enter` to submit, `Esc` to cancel. |
| `e` | **Edit Callsign** | Opens a modal to edit the active operator's callsign and re-query QRZ. Press `Enter` to save, `Esc` to cancel. |
| `d` | **Delete Operator** | Prompts confirmation to remove the active operator from the net and queue. Press `y` to confirm, `n`/`Esc` to cancel. |
| `s` | **Export Contest** | Opens export modal to save contest results to CSV or JSON. Press `Tab` to switch format, `Enter` to export, `Esc` to cancel. |
| `n`, `Enter`, or `↓` | **Next Turn** | Advances the active turn to the next operator in the queue. |
| `p` or `↑` | **Prev Turn** | Moves the active turn back to the previous operator (for corrections). |
| `y` | **Award Correct Answer** | Adds `1 pt` to the active operator's trivia score. |
| `x` | **Deduct Correct Answer** | Deduct `1 pt` from the active operator's trivia score (for corrections). |
| `b` | **Award Bonus Point**| Adds `1 pt` to the active operator's bonus score. |
| `v` | **Deduct Bonus Point** | Deduct `1 pt` from the active operator's bonus score (for corrections). |
| `r` | **Rotate Round** | Finishes the current round, increments round number, rotates queue, and advances question. |
| `f` | **Show Final Scores** | Opens interactive final scoreboard and contest standings modal. Press `Esc`/`Enter` to close, `s` to export. |
| `t` | **Browse & Switch Decks** | Opens the topic picker modal to choose and switch trivia decks (`Topic/*.md`). |
| `u` | **Hot-Reload Deck** | Reloads current active trivia markdown deck from disk without losing scores. |
| `a` | **Toggle Answer Visibility** | Toggles hiding/revealing the answer and Net Control fact. |
| `[` or `←` | **Previous Question** | Manually moves back to the previous trivia question. |
| `]` or `→` | **Next Question** | Manually advances to the next trivia question. |
| `Backspace` or `k` | **Clear Session Log** | Prompts confirmation to clear session and reset all scores. |
| `?` or `h` | **Help Menu** | Opens popup reference modal displaying all keyboard shortcuts and commands. |
| `q` or `Esc` | **Quit** | Restores the terminal to its original state and exits the application. |
