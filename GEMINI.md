# TriviaNetDMR - Net Control Companion

TriviaNetDMR is a modern, highly interactive, and visually polished Terminal User Interface (TUI) application designed for Amateur Radio Net Control Stations (NCS) running trivia nets. It streamlines the check-in process, automatically pulls operator details from the QRZ.com XML Callbook database, tracks scores across multiple categories, and automatically handles fair participant rotation between trivia rounds.

## 🚀 Key Features
- **Modern Ratatui TUI:** Clean, responsive terminal dashboard layout with dedicated panes for the participant queue, active operator profile card, net statistics, and control shortcuts.
- **Asynchronous Background Lookups:** Powered by `tokio`. Checking in a callsign immediately appends them to the queue and triggers a non-blocking background lookup, ensuring the user interface remains completely smooth and responsive without freezing.
- **QRZ.com XML API Integration:** Automatically logs in, caches the session key, and fetches operator names and locations (city/state/country) on the fly.
- **Region-Aware Mock Fallback:** When run without QRZ credentials, the app enters a highly realistic offline mock mode. It dynamically parses callsign prefixes (both US regions 1-0 and international prefixes like `VE`, `G`, `DL`, `JA`, `F`) to generate appropriate names and locations.
- **Trivia Queue Rotation:** Automatically rotates the active list of participants after each round, ensuring a fair starting position for all check-ins, while placing late check-ins at the end of the rotation.
- **Granular Scoring:** Tracks automatic check-in points (1pt), trivia answer points (1pt per correct answer), and custom bonus points (1pt for best/interesting answers) per participant.

---

## 🛠 Project Architecture
The codebase is structured into four highly focused modules:
1. `src/state.rs`: Holds the pure domain models (`Participant`, `App`, `InputMode`), scoring mutations, and the queue rotation logic. It contains comprehensive unit tests verifying rotation offsets and late arrivals.
2. `src/qrz.rs`: Features the asynchronous `QrzClient`. It manages session-cached authentication, XML response parsing using `roxmltree`, and the smart, region-aware mock generator.
3. `src/ui.rs`: Handles the layout rendering using `ratatui`. It draws a multi-column header (with a live round indicator and API status), a beautiful data table for participants, an operator detail card, net summary stats, and a modal popup dialog for check-ins.
4. `src/main.rs`: Coordinates the startup, crossterm raw-mode initialization, the multi-producer single-consumer (`mpsc`) event router, and handles the graceful shutdown/terminal restoration.

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
To execute the comprehensive test suite (testing scoring, queue rotation, and XML parsing):
```bash
cargo test
```

---

## ⌨ Keyboard Controls & Shortcuts

The application features intuitive single-key controls:

| Key | Action | Description |
| :--- | :--- | :--- |
| `c` | **Check-in Operator** | Opens a modal popup to enter a callsign. Press `Enter` to submit, `Esc` to cancel. |
| `n` or `Enter` | **Next Turn** | Advances the active turn to the next operator in the queue. |
| `p` | **Prev Turn** | Moves the active turn back to the previous operator (for corrections). |
| `y` | **Award Correct Answer** | Adds `1 pt` to the active operator's trivia score. |
| `x` | **Deduct Correct Answer** | Deduct `1 pt` from the active operator's trivia score (for corrections). |
| `b` | **Award Bonus Point**| Adds `1 pt` to the active operator's bonus score. |
| `v` | **Deduct Bonus Point** | Deduct `1 pt` from the active operator's bonus score (for corrections). |
| `r` | **Rotate Round** | Finishes the current round, increments round number, and rotates the queue left by 1 (the current starter goes to the end). |
| `q` or `Esc` | **Quit** | Restores the terminal to its original state and exits the application. |
