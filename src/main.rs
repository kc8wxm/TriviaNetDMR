pub mod qrz;
pub mod state;
pub mod ui;

use crate::state::{App, InputMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
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
    
    if qrz_client.is_mocked() {
        app.api_status = String::from("Offline (Mock Mode)");
    } else {
        app.api_status = String::from("Online (QRZ Live)");
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
            
            if event::poll(timeout).unwrap_or(false) {
                if let Event::Key(key) = event::read().unwrap() {
                    if key.kind == event::KeyEventKind::Press {
                        let _ = keys_tx.send(AppEvent::Key(key)).await;
                    }
                }
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
            match app_evt {
                AppEvent::Key(key) => match app.input_mode {
                    // --- INPUT MODE: ADD CHECK-IN ---
                    InputMode::AddCheckin => match key.code {
                        KeyCode::Enter => {
                            let trimmed = app.input_buffer.trim().to_uppercase();
                            if !trimmed.is_empty() {
                                let id = app.add_participant(trimmed.clone());
                                
                                // Spawn background async lookup for QRZ details
                                let qrz = qrz_client.clone();
                                let tx = event_tx.clone();
                                tokio::spawn(async move {
                                    match qrz.lookup(&trimmed).await {
                                        Ok(data) => {
                                            let _ = tx.send(AppEvent::QrzResult {
                                                id,
                                                name: data.name,
                                                location: data.location,
                                                is_mock: data.is_mock,
                                            }).await;
                                        }
                                        Err(_) => {
                                            // Fallback to region-specific mock on error
                                            let mock = qrz::QrzClient::get_mock_data(&trimmed);
                                            let _ = tx.send(AppEvent::QrzResult {
                                                id,
                                                name: mock.name,
                                                location: mock.location,
                                                is_mock: true,
                                            }).await;
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
                        KeyCode::Char(c) => {
                            // Enforce reasonable callsign length (e.g. 10 chars)
                            if app.input_buffer.len() < 10 {
                                app.input_buffer.push(c);
                            }
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
                        }
                        KeyCode::Char('p') => {
                            app.prev_turn();
                        }
                        KeyCode::Char('y') => {
                            app.award_trivia_point();
                        }
                        KeyCode::Char('x') => {
                            app.remove_trivia_point();
                        }
                        KeyCode::Char('b') => {
                            app.award_bonus_point();
                        }
                        KeyCode::Char('v') => {
                            app.remove_bonus_point();
                        }
                        KeyCode::Char('r') => {
                            app.next_round();
                        }
                        _ => {}
                    },
                },
                AppEvent::Tick => {
                    // Can do periodic tasks here if needed
                }
                AppEvent::QrzResult { id, name, location, is_mock } => {
                    if let Some(p) = app.participants.get_mut(id) {
                        p.name = name;
                        p.location = location;
                        p.qrz_fetched = !is_mock;
                    }
                }
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

    // 6. Print Final Scoreboard
    if !app.participants.is_empty() {
        println!("\n=========================================================");
        println!("             🏆 TriviaNetDMR Final Scores 🏆");
        println!("=========================================================");
        println!("{:<4} {:<10} {:<25} {:<6}", "Pos", "Callsign", "Operator Name", "Score");
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
