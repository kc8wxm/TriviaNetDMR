pub mod qrz;
pub mod state;
pub mod trivia;
pub mod ui;

use crate::state::{App, InputMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io;

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    QrzResult {
        callsign: String,
        name: Option<String>,
        location: Option<String>,
        is_mock: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 0. CLI Validator Mode (--check / --validate / -c)
    let args: Vec<String> = std::env::args().collect();
    if args
        .iter()
        .any(|a| a == "--check" || a == "--validate" || a == "-c")
    {
        let target = args.iter().skip(1).find(|a| !a.starts_with('-')).cloned();
        let exit_code = run_validator(target.as_deref());
        std::process::exit(exit_code);
    }

    // 1. Terminal Initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // 2. Application and QRZ Client initialization
    let mut app = App::new();
    let qrz_client = std::sync::Arc::new(qrz::QrzClient::new());

    // Attempt to load previous session from log file
    let log_file = "trivia_log.json";
    let mut loaded_from_file = false;
    if std::path::Path::new(log_file).exists() {
        if let Err(e) = app.load_from_file(log_file) {
            app.error_message = Some(format!("Failed to load session: {}", e));
        } else {
            loaded_from_file = true;
        }
    }

    // Load trivia questions (CLI arg -> Topic/Questions.md -> Topic/Questions-1.md -> Questions.md -> Questions-1.md)
    let topic_path = if args.len() > 1 && !args[1].starts_with('-') {
        Some(args[1].clone())
    } else if std::path::Path::new("Topic/Questions.md").exists() {
        Some("Topic/Questions.md".to_string())
    } else if std::path::Path::new("Topic/Questions-1.md").exists() {
        Some("Topic/Questions-1.md".to_string())
    } else if std::path::Path::new("Questions.md").exists() {
        Some("Questions.md".to_string())
    } else if std::path::Path::new("Questions-1.md").exists() {
        Some("Questions-1.md".to_string())
    } else {
        None
    };

    if let Some(path) = topic_path {
        match trivia::TriviaTopic::load_from_file(&path) {
            Ok(topic) => {
                // If round > 1 and current_question_index is 0 (e.g. from restored legacy session), sync it
                if app.round_number > 1
                    && app.current_question_index == 0
                    && !topic.questions.is_empty()
                {
                    app.current_question_index =
                        (app.round_number - 1).min(topic.questions.len() - 1);
                }
                app.trivia_topic = Some(topic);
                app.current_topic_path = Some(path);
            }
            Err(e) => {
                app.error_message = Some(format!("Could not load {}: {}", path, e));
            }
        }
    }

    if qrz_client.is_mocked() {
        if loaded_from_file {
            app.api_status = String::from("Offline (Mock) [Restored]");
        } else {
            app.api_status = String::from("Offline (Mock Mode)");
        }
    } else {
        if loaded_from_file {
            app.api_status = String::from("Online (Live) [Restored]");
        } else {
            app.api_status = String::from("Online (QRZ Live)");
        }
    }

    // 3. Event Channels setup
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(100);

    // Spawn crossterm event listening task
    let keys_tx = event_tx.clone();
    tokio::spawn(async move {
        let tick_rate = std::time::Duration::from_millis(200);
        let mut last_tick = std::time::Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| std::time::Duration::from_secs(0));

            if event::poll(timeout).unwrap_or(false)
                && let Event::Key(key) = event::read().unwrap()
                && key.kind == event::KeyEventKind::Press
            {
                let _ = keys_tx.send(AppEvent::Key(key)).await;
            }

            if last_tick.elapsed() >= tick_rate {
                let _ = keys_tx.send(AppEvent::Tick).await;
                last_tick = std::time::Instant::now();
            }
        }
    });

    // 4. Main TUI Event Loop
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if let Some(app_evt) = event_rx.recv().await {
            let mut state_changed = false;
            match app_evt {
                AppEvent::Key(key) => match app.input_mode {
                    // --- INPUT MODE: ADD CHECK-IN ---
                    InputMode::AddCheckin => match key.code {
                        KeyCode::Enter => {
                            let trimmed = app.input_buffer.trim().to_uppercase();
                            if !trimmed.is_empty() {
                                let _ = app.add_participant(trimmed.clone());
                                state_changed = true;

                                // Spawn background async lookup for QRZ details
                                let qrz = qrz_client.clone();
                                let tx = event_tx.clone();
                                let lookup_call = trimmed.clone();
                                tokio::spawn(async move {
                                    match qrz.lookup(&lookup_call).await {
                                        Ok(data) => {
                                            let _ = tx
                                                .send(AppEvent::QrzResult {
                                                    callsign: lookup_call.clone(),
                                                    name: data.name,
                                                    location: data.location,
                                                    is_mock: data.is_mock,
                                                })
                                                .await;
                                        }
                                        Err(_) => {
                                            // Fallback to region-specific mock on error
                                            let mock = qrz::QrzClient::get_mock_data(&lookup_call);
                                            let _ = tx
                                                .send(AppEvent::QrzResult {
                                                    callsign: lookup_call.clone(),
                                                    name: mock.name,
                                                    location: mock.location,
                                                    is_mock: true,
                                                })
                                                .await;
                                        }
                                    }
                                });
                            }
                            app.input_buffer.clear();
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.input_buffer.clear();
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        KeyCode::Char(c)
                            // Enforce reasonable callsign length (e.g. 10 chars)
                            if app.input_buffer.len() < 10 => {
                                app.input_buffer.push(c);
                            }
                        _ => {}
                    },

                    // --- NORMAL OPERATION MODE ---
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            break; // Quit
                        }
                        KeyCode::Char('c') => {
                            app.input_mode = InputMode::AddCheckin;
                            app.input_buffer.clear();
                        }
                        KeyCode::Char('e') => {
                            if let Some(active) = app.active_participant() {
                                app.input_buffer = active.callsign.clone();
                                app.input_mode = InputMode::EditCallsign;
                            } else {
                                app.error_message = Some("No active operator to edit".to_string());
                            }
                        }
                        KeyCode::Char('d') => {
                            if app.active_participant().is_some() {
                                app.input_mode = InputMode::DeleteConfirm;
                            } else {
                                app.error_message = Some("No active operator to delete".to_string());
                            }
                        }
                        KeyCode::Char('s') => {
                            app.input_mode = InputMode::ExportDialog {
                                format: state::ExportFormat::Csv,
                            };
                            app.input_buffer = "contest_results.csv".to_string();
                        }
                        KeyCode::Char('f') | KeyCode::Char('F') => {
                            app.scoreboard_scroll = 0;
                            app.input_mode = InputMode::FinalScores;
                        }
                        KeyCode::Char('n') | KeyCode::Enter | KeyCode::Down => {
                            app.next_turn();
                            state_changed = true;
                        }
                        KeyCode::Char('p') | KeyCode::Up => {
                            app.prev_turn();
                            state_changed = true;
                        }
                        KeyCode::Char('y') => {
                            app.award_trivia_point();
                            state_changed = true;
                        }
                        KeyCode::Char('x') => {
                            app.remove_trivia_point();
                            state_changed = true;
                        }
                        KeyCode::Char('b') => {
                            app.award_bonus_point();
                            state_changed = true;
                        }
                        KeyCode::Char('v') => {
                            app.remove_bonus_point();
                            state_changed = true;
                        }
                        KeyCode::Char('r') => {
                            app.next_round();
                            state_changed = true;
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            app.toggle_show_answer();
                            state_changed = true;
                        }
                        KeyCode::Char('[') | KeyCode::Left => {
                            app.prev_question();
                            state_changed = true;
                        }
                        KeyCode::Char(']') | KeyCode::Right => {
                            app.next_question();
                            state_changed = true;
                        }
                        KeyCode::Char('t') | KeyCode::Char('T') => {
                            app.open_topic_picker();
                        }
                        KeyCode::Char('u') | KeyCode::Char('U') => {
                            match app.reload_current_topic() {
                                Ok(msg) => {
                                    app.status_message = Some(msg);
                                    app.error_message = None;
                                    state_changed = true;
                                }
                                Err(err) => {
                                    app.error_message = Some(err);
                                }
                            }
                        }
                        KeyCode::Backspace | KeyCode::Char('k') => {
                            app.input_mode = InputMode::ClearConfirm;
                        }
                        _ => {}
                    },

                    // --- INPUT MODE: EDIT CALLSIGN ---
                    InputMode::EditCallsign => match key.code {
                        KeyCode::Enter => {
                            let trimmed = app.input_buffer.trim().to_uppercase();
                            if let Some(active) = app.active_participant() {
                                let active_id = active.id;
                                match app.edit_participant_callsign(active_id, trimmed.clone()) {
                                    Ok(new_call) => {
                                        app.status_message =
                                            Some(format!("Updated callsign to {}", new_call));
                                        app.error_message = None;
                                        state_changed = true;

                                        // Lookup QRZ info for the new callsign
                                        let qrz = qrz_client.clone();
                                        let tx = event_tx.clone();
                                        let lookup_call = new_call.clone();
                                        tokio::spawn(async move {
                                            match qrz.lookup(&lookup_call).await {
                                                Ok(data) => {
                                                    let _ = tx
                                                        .send(AppEvent::QrzResult {
                                                            callsign: lookup_call.clone(),
                                                            name: data.name,
                                                            location: data.location,
                                                            is_mock: data.is_mock,
                                                        })
                                                        .await;
                                                }
                                                Err(_) => {
                                                    let mock =
                                                        qrz::QrzClient::get_mock_data(&lookup_call);
                                                    let _ = tx
                                                        .send(AppEvent::QrzResult {
                                                            callsign: lookup_call.clone(),
                                                            name: mock.name,
                                                            location: mock.location,
                                                            is_mock: true,
                                                        })
                                                        .await;
                                                }
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        app.error_message = Some(e);
                                    }
                                }
                            }
                            app.input_buffer.clear();
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.input_buffer.clear();
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        KeyCode::Char(c) if app.input_buffer.len() < 10 => {
                            app.input_buffer.push(c.to_ascii_uppercase());
                        }
                        _ => {}
                    },

                    // --- INPUT MODE: DELETE CONFIRMATION ---
                    InputMode::DeleteConfirm => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Some(active) = app.active_participant() {
                                let active_id = active.id;
                                match app.delete_participant(active_id) {
                                    Ok(call) => {
                                        app.status_message =
                                            Some(format!("Deleted operator {} from net", call));
                                        app.error_message = None;
                                        state_changed = true;
                                    }
                                    Err(e) => {
                                        app.error_message = Some(e);
                                    }
                                }
                            }
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },

                    // --- INPUT MODE: EXPORT CONTEST ---
                    InputMode::ExportDialog { format } => match key.code {
                        KeyCode::Tab => {
                            let next_format = match format {
                                state::ExportFormat::Csv => state::ExportFormat::Json,
                                state::ExportFormat::Json => state::ExportFormat::Csv,
                            };
                            if app.input_buffer == "contest_results.csv" {
                                app.input_buffer = "contest_results.json".to_string();
                            } else if app.input_buffer == "contest_results.json" {
                                app.input_buffer = "contest_results.csv".to_string();
                            } else if app.input_buffer.ends_with(".csv")
                                && next_format == state::ExportFormat::Json
                            {
                                app.input_buffer = format!(
                                    "{}.json",
                                    &app.input_buffer[..app.input_buffer.len() - 4]
                                );
                            } else if app.input_buffer.ends_with(".json")
                                && next_format == state::ExportFormat::Csv
                            {
                                app.input_buffer = format!(
                                    "{}.csv",
                                    &app.input_buffer[..app.input_buffer.len() - 5]
                                );
                            }
                            app.input_mode = InputMode::ExportDialog {
                                format: next_format,
                            };
                        }
                        KeyCode::Enter => {
                            let filename = app.input_buffer.trim().to_string();
                            if filename.is_empty() {
                                app.error_message =
                                    Some("Export filename cannot be empty.".to_string());
                            } else {
                                let res = match format {
                                    state::ExportFormat::Csv => app.export_to_csv(&filename),
                                    state::ExportFormat::Json => app.export_to_json(&filename),
                                };
                                match res {
                                    Ok(()) => {
                                        app.status_message =
                                            Some(format!("Exported contest to {}", filename));
                                        app.error_message = None;
                                    }
                                    Err(e) => {
                                        app.error_message =
                                            Some(format!("Export failed: {}", e));
                                    }
                                }
                            }
                            app.input_buffer.clear();
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Esc => {
                            app.input_buffer.clear();
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Backspace => {
                            app.input_buffer.pop();
                        }
                        KeyCode::Char(c) if app.input_buffer.len() < 64 => {
                            app.input_buffer.push(c);
                        }
                        _ => {}
                    },

                    // --- INPUT MODE: CLEAR LOG CONFIRMATION ---
                    InputMode::ClearConfirm => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Err(e) = app.clear_session("trivia_log.json") {
                                app.error_message = Some(format!("Failed to clear session: {}", e));
                            } else {
                                app.status_message =
                                    Some("Cleared session log and reset scores".to_string());
                            }
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },

                    // --- INPUT MODE: FINAL SCORES / LEADERBOARD ---
                    InputMode::FinalScores => match key.code {
                        KeyCode::Esc
                        | KeyCode::Enter
                        | KeyCode::Char('f')
                        | KeyCode::Char('F')
                        | KeyCode::Char('q') => {
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            app.input_mode = InputMode::ExportDialog {
                                format: state::ExportFormat::Csv,
                            };
                            app.input_buffer = "contest_results.csv".to_string();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.scoreboard_scroll > 0 {
                                app.scoreboard_scroll -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let total = app.participants.len();
                            if app.scoreboard_scroll + 1 < total {
                                app.scoreboard_scroll += 1;
                            }
                        }
                        KeyCode::PageUp => {
                            app.scoreboard_scroll = app.scoreboard_scroll.saturating_sub(5);
                        }
                        KeyCode::PageDown => {
                            let total = app.participants.len();
                            app.scoreboard_scroll =
                                (app.scoreboard_scroll + 5).min(total.saturating_sub(1));
                        }
                        _ => {}
                    },

                    // --- INPUT MODE: TOPIC PICKER ---
                    InputMode::TopicPicker => match key.code {
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('p') => {
                            if app.topic_picker_index > 0 {
                                app.topic_picker_index -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('n') => {
                            if !app.available_decks.is_empty()
                                && app.topic_picker_index + 1 < app.available_decks.len()
                            {
                                app.topic_picker_index += 1;
                            }
                        }
                        KeyCode::Enter => {
                            match app.select_picked_topic() {
                                Ok(msg) => {
                                    app.status_message = Some(msg);
                                    app.error_message = None;
                                    state_changed = true;
                                }
                                Err(err) => {
                                    app.error_message = Some(err);
                                }
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },
                },
                AppEvent::Tick => {
                    // Can do periodic tasks here if needed
                }
                AppEvent::QrzResult {
                    callsign,
                    name,
                    location,
                    is_mock,
                } => {
                    if let Some(p) = app.participants.iter_mut().find(|p| p.callsign == callsign) {
                        p.name = name;
                        p.location = location;
                        p.qrz_fetched = !is_mock;
                        state_changed = true;
                    }
                }
            }

            if state_changed {
                let _ = app.save_to_file("trivia_log.json");
            }
        }
    }

    // 5. Restore Terminal State
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Save final state before exiting
    let _ = app.save_to_file("trivia_log.json");

    // 6. Print Final Scoreboard
    if !app.participants.is_empty() {
        println!("\n=========================================================");
        println!("             🏆 TriviaNetDMR Final Scores 🏆");
        println!("=========================================================");
        println!(
            "{:<4} {:<10} {:<25} {:<6}",
            "Pos", "Callsign", "Operator Name", "Score"
        );
        println!("---------------------------------------------------------");

        let final_scores = app.get_scoreboard();

        for (rank, p) in final_scores {
            let name = p.name.as_deref().unwrap_or("Unknown Operator");
            let truncated_name = if name.len() > 24 { &name[..24] } else { name };
            println!(
                "{:<4} {:<10} {:<25} {:^5}",
                rank,
                p.callsign,
                truncated_name,
                p.total_score()
            );
        }
        println!("=========================================================\n");
    }

    Ok(())
}

/// Standalone CLI deck validator mode (--check / --validate / -c)
fn run_validator(target: Option<&str>) -> i32 {
    println!("🔍 TriviaNetDMR Deck Validator\n");

    let validations = match target {
        Some(path) => {
            let p = std::path::Path::new(path);
            if p.is_dir() {
                println!("Scanning directory: {}", path);
                trivia::scan_and_validate_directory(p)
            } else if p.is_file() {
                println!("Validating single file: {}", path);
                vec![trivia::validate_deck(p)]
            } else {
                eprintln!("Error: Target path does not exist: {}", path);
                return 1;
            }
        }
        None => {
            println!("Scanning default locations (Topic/ directory and current path)...");
            trivia::find_all_deck_files()
        }
    };

    if validations.is_empty() {
        println!("⚠️  No Markdown trivia deck files (*.md) found to validate.");
        return 0;
    }

    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut valid_decks = 0;

    for val in &validations {
        println!("--------------------------------------------------");
        println!("📄 File:      {}", val.file_path);
        println!("   Topic:     \"{}\"", val.title);
        println!("   Questions: {}", val.question_count);

        if val.is_valid() && val.issues.is_empty() {
            println!("   ✔ Status:    PERFECT (No issues detected)");
            valid_decks += 1;
        } else if val.is_valid() {
            println!(
                "   ✔ Status:    VALID (with {} warning{})",
                val.warning_count(),
                if val.warning_count() == 1 { "" } else { "s" }
            );
            valid_decks += 1;
        } else {
            println!(
                "   ✘ Status:    INVALID ({} error{}, {} warning{})",
                val.error_count(),
                if val.error_count() == 1 { "" } else { "s" },
                val.warning_count(),
                if val.warning_count() == 1 { "" } else { "s" }
            );
        }

        for issue in &val.issues {
            match issue.level {
                trivia::IssueLevel::Error => {
                    total_errors += 1;
                    if let Some(q) = issue.question_number {
                        println!("     ✘ ERROR   [Q{}]: {}", q, issue.message);
                    } else {
                        println!("     ✘ ERROR:  {}", issue.message);
                    }
                }
                trivia::IssueLevel::Warning => {
                    total_warnings += 1;
                    if let Some(q) = issue.question_number {
                        println!("     ⚠ WARNING [Q{}]: {}", q, issue.message);
                    } else {
                        println!("     ⚠ WARNING: {}", issue.message);
                    }
                }
            }
        }
        println!();
    }

    println!("==================================================");
    println!(
        "Summary: {} deck(s) scanned | {} valid | {} error(s) | {} warning(s)",
        validations.len(),
        valid_decks,
        total_errors,
        total_warnings
    );

    if total_errors > 0 { 1 } else { 0 }
}

