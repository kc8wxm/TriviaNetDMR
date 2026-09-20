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
        id: usize,
        name: Option<String>,
        location: Option<String>,
        is_mock: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let args: Vec<String> = std::env::args().collect();
    let topic_path = if args.len() > 1 {
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
                                let id = app.add_participant(trimmed.clone());
                                state_changed = true;

                                // Spawn background async lookup for QRZ details
                                let qrz = qrz_client.clone();
                                let tx = event_tx.clone();
                                tokio::spawn(async move {
                                    match qrz.lookup(&trimmed).await {
                                        Ok(data) => {
                                            let _ = tx
                                                .send(AppEvent::QrzResult {
                                                    id,
                                                    name: data.name,
                                                    location: data.location,
                                                    is_mock: data.is_mock,
                                                })
                                                .await;
                                        }
                                        Err(_) => {
                                            // Fallback to region-specific mock on error
                                            let mock = qrz::QrzClient::get_mock_data(&trimmed);
                                            let _ = tx
                                                .send(AppEvent::QrzResult {
                                                    id,
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
                        KeyCode::Char('n') | KeyCode::Enter => {
                            app.next_turn();
                            state_changed = true;
                        }
                        KeyCode::Char('p') => {
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
                        KeyCode::Char('[') => {
                            app.prev_question();
                            state_changed = true;
                        }
                        KeyCode::Char(']') => {
                            app.next_question();
                            state_changed = true;
                        }
                        KeyCode::Backspace | KeyCode::Char('k') => {
                            app.input_mode = InputMode::ClearConfirm;
                        }
                        _ => {}
                    },

                    // --- INPUT MODE: CLEAR LOG CONFIRMATION ---
                    InputMode::ClearConfirm => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Err(e) = app.clear_session("trivia_log.json") {
                                app.error_message = Some(format!("Failed to clear session: {}", e));
                            }
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },
                },
                AppEvent::Tick => {
                    // Can do periodic tasks here if needed
                }
                AppEvent::QrzResult {
                    id,
                    name,
                    location,
                    is_mock,
                } => {
                    if let Some(p) = app.participants.get_mut(id) {
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

        let mut final_scores = app.participants.clone();
        final_scores.sort_by(|a, b| {
            b.total_score()
                .cmp(&a.total_score())
                .then_with(|| a.callsign.cmp(&b.callsign))
        });

        for (idx, p) in final_scores.iter().enumerate() {
            let name = p.name.as_deref().unwrap_or("Unknown Operator");
            let truncated_name = if name.len() > 24 { &name[..24] } else { name };
            println!(
                "{:<4} {:<10} {:<25} {:^5}",
                idx + 1,
                p.callsign,
                truncated_name,
                p.total_score()
            );
        }
        println!("=========================================================\n");
    }

    Ok(())
}
