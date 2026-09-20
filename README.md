# 📻 TriviaNetDMR — Net Control Companion

[![Release](https://img.shields.io/github/v/release/kc8wxm/TriviaNetDMR?color=brightgreen&label=Latest%20Release)](https://github.com/kc8wxm/TriviaNetDMR/releases/latest)
[![Build Status](https://img.shields.io/github/actions/workflow/status/kc8wxm/TriviaNetDMR/release.yml?branch=master&label=Builds)](https://github.com/kc8wxm/TriviaNetDMR/actions)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blue)](https://github.com/kc8wxm/TriviaNetDMR/releases)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-orange)](#)

**TriviaNetDMR** is a modern, high-performance, and visually polished Terminal User Interface (TUI) application designed for Amateur Radio Net Control Stations (NCS) running trivia nets over DMR or analog repeaters.

It streamlines the check-in process, automatically pulls operator details from the **QRZ.com XML Callbook** database, tracks scores across multiple categories, prompts trivia questions and canonical answers with markdown decks, and automatically rotates the participant queue between rounds for fair contest play.

---

## 📥 Download Pre-Compiled Releases

Pre-compiled, standalone binaries are automatically built for **Windows**, **Linux**, and **macOS**.  
**No Rust, Python, or external dependencies are required to run.**

👉 **[View All Releases on GitHub](https://github.com/kc8wxm/TriviaNetDMR/releases/latest)**

| Operating System | Download Archive | Quick Start Instructions |
| :--- | :--- | :--- |
| **Windows** (64-bit) | [**`TriviaNetDMR-windows-x86_64.zip`**](https://github.com/kc8wxm/TriviaNetDMR/releases/latest/download/TriviaNetDMR-windows-x86_64.zip) | 1. Download and extract the `.zip`<br>2. Double-click **`run.bat`** (or **`TriviaNetDMR.exe`**) |
| **Linux** (x86_64) | [**`TriviaNetDMR-linux-x86_64.tar.gz`**](https://github.com/kc8wxm/TriviaNetDMR/releases/latest/download/TriviaNetDMR-linux-x86_64.tar.gz) | 1. `tar -xvf TriviaNetDMR-linux-x86_64.tar.gz`<br>2. `cd TriviaNetDMR* && ./TriviaNetDMR` |
| **macOS** (Apple Silicon M1-M4) | [**`TriviaNetDMR-macos-aarch64.tar.gz`**](https://github.com/kc8wxm/TriviaNetDMR/releases/latest/download/TriviaNetDMR-macos-aarch64.tar.gz) | 1. `tar -xvf TriviaNetDMR-macos-aarch64.tar.gz`<br>2. `cd TriviaNetDMR* && ./TriviaNetDMR` |
| **macOS** (Intel x86_64) | [**`TriviaNetDMR-macos-x86_64.tar.gz`**](https://github.com/kc8wxm/TriviaNetDMR/releases/latest/download/TriviaNetDMR-macos-x86_64.tar.gz) | 1. `tar -xvf TriviaNetDMR-macos-x86_64.tar.gz`<br>2. `cd TriviaNetDMR* && ./TriviaNetDMR` |

---

## 🚀 Key Features

- **Modern Ratatui Dashboard:** Responsive full-screen console layout with dedicated panes for queue rotation, active operator profile card, live scorecard, and net summary statistics.
- **Asynchronous QRZ.com XML Lookups:** Non-blocking background lookups powered by `tokio`. Entering a callsign immediately appends the operator to the queue while name and city/state are retrieved seamlessly.
- **Offline Region-Aware Mock Mode:** When run without QRZ credentials, the application automatically enters offline mode, dynamically generating realistic operator profiles based on US call districts (1–0) and international prefixes (`VE`, `G`, `DL`, `JA`, `F`, etc.).
- **Markdown Question Decks:** Automatically parses trivia decks (such as `Topic/Questions.md`) into questions, answers, and Net Control facts, with answer reveal toggling (`a`) and question navigation (`[` / `]`).
- **In-App Deck Picker (`t`):** Browse, validate, and switch trivia question decks on the fly directly within the TUI without restarting the application.
- **Hot-Reload Decks (`u`):** Edit questions in your favorite text editor during preparation and instantly hot-reload the deck from disk without losing participant queues or scores.
- **CLI & In-App Deck Validator (`--check`):** Fast pre-flight diagnostic tool to scan and validate Markdown question decks for formatting errors, missing answers, or numbering gaps.
- **Rotating Queue:** Automatically rotates queue order at the start of each round (`r`), giving every participant a fair opportunity to answer first.
- **Granular Scoring:** Tracks automatic check-in points (1pt), trivia answer points (1pt per correct answer), and bonus points (1pt for best/interesting answer).
- **Callsign Management:** Edit mistyped callsigns on the fly (`e`) with automatic QRZ re-lookup, or delete operators (`d`) from the net and queue with safe confirmation.
- **Interactive Leaderboard & Final Scores:** Press `f` at any time to open an interactive scoreboard popup featuring podium medal ranks (🥇, 🥈, 🥉), complete score breakdowns, and smooth scrolling (`↑` / `↓`).
- **Contest Export (CSV & JSON):** Export results at any time (`s`) to standard CSV spreadsheets (RFC 4180 compliant) or structured JSON data files.

---

## ⌨ Keyboard Controls & Shortcuts

The application features intuitive, single-key shortcuts:

| Key | Action | Description |
| :--- | :--- | :--- |
| `c` | **Check-in Operator** | Opens a modal to enter a callsign. Press `Enter` to submit, `Esc` to cancel. |
| `e` | **Edit Callsign** | Opens a modal to edit the active operator's callsign and re-query QRZ. |
| `d` | **Delete Operator** | Prompts confirmation to remove the active operator from the net and queue. |
| `s` | **Export Contest** | Opens modal to export contest standings to CSV or JSON. Press `Tab` to switch format. |
| `f` | **Show Final Scores** | Opens the interactive final scoreboard and contest standings modal. |
| `t` | **Browse & Switch Decks** | Opens the topic picker modal to choose and switch trivia decks (`Topic/*.md`). |
| `u` | **Hot-Reload Deck** | Reloads current active trivia markdown deck from disk without losing scores. |
| `n`, `Enter`, or `↓` | **Next Turn** | Advances the active turn to the next operator in the queue. |
| `p` or `↑` | **Prev Turn** | Moves the active turn back to the previous operator (for score corrections). |
| `y` | **Award Correct Answer** | Adds `1 pt` to the active operator's trivia score. |
| `x` | **Deduct Correct Answer** | Deducts `1 pt` from the active operator's trivia score. |
| `b` | **Award Bonus Point** | Adds `1 pt` to the active operator's bonus score. |
| `v` | **Deduct Bonus Point** | Deducts `1 pt` from the active operator's bonus score. |
| `r` | **Rotate Round** | Completes current round, increments round number, rotates queue, and advances question. |
| `a` | **Toggle Answer** | Toggles hiding/revealing the canonical answer and Net Control fact. |
| `[` or `←` | **Previous Question** | Manually moves back to the previous trivia question. |
| `]` or `→` | **Next Question** | Manually advances to the next trivia question. |
| `Backspace` or `k` | **Clear Session Log** | Prompts confirmation to clear session and reset all scores. |
| `?` or `h` | **Help Menu** | Opens popup reference modal displaying all keyboard shortcuts and commands. |
| `q` or `Esc` | **Quit** | Restores the terminal to normal state and exits (printing final scores). |

---

## ⚙ QRZ.com API Configuration

### Live Lookups
To enable live lookups against the real QRZ.com database:

#### Windows
Edit `run.bat` and uncomment the credential lines:
```bat
set QRZ_USERNAME=your_callsign
set QRZ_PASSWORD=your_qrz_password
```

#### Linux / macOS
Export credentials in your shell before launching:
```bash
export QRZ_USERNAME="your_callsign"
export QRZ_PASSWORD="your_qrz_password"
./TriviaNetDMR
```

*If these credentials are not provided, TriviaNetDMR automatically operates in **Offline Mock Mode**, seamlessly providing region-appropriate operator profiles.*

---

## 🛠 Building From Source

If you have [Rust and Cargo](https://rustup.rs/) installed, you can build and run directly from source:

```bash
# Clone the repository
git clone https://github.com/kc8wxm/TriviaNetDMR.git
cd TriviaNetDMR

# Run the test suite
cargo test

# Compile and launch the release build
cargo run --release
```

---

## 🔍 Deck Validator & Custom Questions

You can create and organize your own custom trivia decks simply by dropping Markdown files into the `Topic/` folder. Format questions using standard `Question:`, `Answer:`, and optional `Net Control Fact:` lines:

```markdown
### **10 Trivia Questions & Answers: Astronomy**

1. **Question:** What is the closest planet to the Sun?
   * **Answer:** Mercury.
   * **Net Control Fact:** Mercury has no atmosphere and extreme temperature swings.
```

To pre-flight check your trivia decks for syntax errors, missing answers, or numbering issues without launching the full TUI:

```bash
# Validate all decks in Topic/ directory
./TriviaNetDMR --check

# Validate a specific deck file
./TriviaNetDMR --check Topic/Questions-2.md
```

---

## 📄 License
This project is open-source under the MIT / Apache-2.0 license.
