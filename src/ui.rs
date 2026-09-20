use crate::state::{App, InputMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Row, Table, Wrap},
};

/// Helper function to create a centered Rect for popups
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    // Main Layout (Header, Trivia Q&A if present, Body, Footer)
    let (header_chunk, trivia_chunk, body_chunk, footer_chunk) = if app.trivia_topic.is_some() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(7), // Trivia Q&A Card
                Constraint::Min(8),    // Main content
                Constraint::Length(3), // Footer / Controls
            ])
            .split(size);
        (chunks[0], Some(chunks[1]), chunks[2], chunks[3])
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Main content
                Constraint::Length(3), // Footer / Controls
            ])
            .split(size);
        (chunks[0], None, chunks[1], chunks[2])
    };

    // ==========================================
    // 1. HEADER
    // ==========================================
    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Title
            Constraint::Percentage(30), // Round Status
            Constraint::Percentage(30), // API / Connection Status
        ])
        .split(header_chunk);

    let title_p = Paragraph::new(Line::from(vec![
        Span::styled(
            " 📻 TriviaNetDMR ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Net Control Companion", Style::default().fg(Color::Gray)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    let mut round_spans = vec![
        Span::styled(" Round: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", app.round_number),
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if let Some(topic) = &app.trivia_topic {
        let total_q = topic.questions.len();
        let q_display = if total_q > 0 {
            format!("{}/{}", (app.current_question_index + 1).min(total_q), total_q)
        } else {
            "0/0".to_string()
        };
        round_spans.push(Span::styled("  |  Q: ", Style::default().fg(Color::Gray)));
        round_spans.push(Span::styled(
            q_display,
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let round_status_p = Paragraph::new(Line::from(round_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    let api_status_style = if app.api_status.contains("Offline") {
        Style::default().fg(Color::LightYellow)
    } else {
        Style::default().fg(Color::LightGreen)
    };
    let api_p = Paragraph::new(Line::from(vec![
        Span::styled(" QRZ API: ", Style::default().fg(Color::Gray)),
        Span::styled(&app.api_status, api_status_style),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(title_p, header_layout[0]);
    f.render_widget(round_status_p, header_layout[1]);
    f.render_widget(api_p, header_layout[2]);

    // ==========================================
    // 2. TRIVIA QUESTION & ANSWER CARD (IF LOADED)
    // ==========================================
    if let Some(q_area) = trivia_chunk {
        let (title, lines) = if let Some(q) = app.current_question() {
            let total_q = app
                .trivia_topic
                .as_ref()
                .map(|t| t.questions.len())
                .unwrap_or(0);
            let topic_name = app
                .trivia_topic
                .as_ref()
                .map(|t| t.title.as_str())
                .unwrap_or("Trivia Net");
            let card_title = format!(" 🎯 Question {}/{} — {} ", q.number, total_q, topic_name);

            let mut card_lines = vec![Line::from(vec![
                Span::styled(
                    " Q: ",
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &q.question,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ])];

            if app.show_answer {
                card_lines.push(Line::from(vec![
                    Span::styled(
                        " A: ",
                        Style::default()
                            .fg(Color::LightGreen)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        &q.answer,
                        Style::default()
                            .fg(Color::LightGreen)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));

                if let Some(fact) = &q.fact {
                    card_lines.push(Line::from(vec![
                        Span::styled(
                            " 💡 Fact: ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(fact, Style::default().fg(Color::Gray)),
                    ]));
                }
            } else {
                card_lines.push(Line::from(vec![
                    Span::styled(
                        " A: ",
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "[Answer Hidden — Press 'a' to reveal]",
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            }

            (card_title, card_lines)
        } else {
            let total_q = app
                .trivia_topic
                .as_ref()
                .map(|t| t.questions.len())
                .unwrap_or(0);
            let card_title = " 🎯 Trivia Net ".to_string();
            let card_lines = vec![Line::from(Span::styled(
                format!(
                    " All {} questions completed! Press [r] to rotate round or [ [ ] to review.",
                    total_q
                ),
                Style::default().fg(Color::LightYellow),
            ))];
            (card_title, card_lines)
        };

        let trivia_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightMagenta));

        let trivia_p = Paragraph::new(lines)
            .block(trivia_block)
            .wrap(Wrap { trim: true });

        f.render_widget(trivia_p, q_area);
    }

    // ==========================================
    // 3. MAIN BODY
    // ==========================================
    let body_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Participant list table
            Constraint::Percentage(40), // Active info & statistics
        ])
        .split(body_chunk);

    // -- Left side: Participant Table --
    let table_rect = body_layout[0];
    let header_cells = [
        "Act", "Callsign", "Name", "Check-in", "Trivia", "Bonus", "Score",
    ]
    .iter()
    .map(|h| {
        Span::styled(
            *h,
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header_row = Row::new(header_cells).height(1).bottom_margin(1);

    let mut rows = Vec::new();
    for (i, &p_id) in app.queue.iter().enumerate() {
        if let Some(p) = app.participants.get(p_id) {
            let is_active = i == app.current_queue_index;
            let active_marker = if is_active { " 👉 " } else { "    " };

            // Text color depends on whether participant is currently active
            let cell_style = if is_active {
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD)
            } else if i < app.current_queue_index {
                Style::default().fg(Color::DarkGray) // Already took their turn
            } else {
                Style::default().fg(Color::White)
            };

            let row_cells = vec![
                Span::styled(
                    active_marker,
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{:<8}", p.callsign), cell_style),
                Span::styled(
                    format!("{:<15}", p.name.as_deref().unwrap_or("Fetching...")),
                    cell_style,
                ),
                Span::styled(format!(" {:^8}", p.points_checkin), cell_style),
                Span::styled(format!(" {:^6}", p.points_trivia), cell_style),
                Span::styled(format!(" {:^5}", p.points_bonus), cell_style),
                Span::styled(format!(" {:^5}", p.total_score()), cell_style),
            ];

            let mut row = Row::new(row_cells).height(1);
            if is_active {
                row = row.style(Style::default().bg(Color::Rgb(20, 40, 20))); // Subtle green highlighting
            }
            rows.push(row);
        }
    }

    let list_block = Block::default()
        .title(" 👥 Participant Queue (Rotating Order) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),  // Act
            Constraint::Length(10), // Callsign
            Constraint::Min(15),    // Name
            Constraint::Length(10), // Check-in Pt
            Constraint::Length(8),  // Trivia Pt
            Constraint::Length(8),  // Bonus Pt
            Constraint::Length(8),  // Total Score
        ],
    )
    .header(header_row)
    .block(list_block);

    f.render_widget(table, table_rect);

    // -- Right side: Details & Status --
    let right_rect = body_layout[1];
    let details_constraints = if right_rect.height >= 16 {
        [Constraint::Length(11), Constraint::Min(4)]
    } else {
        [Constraint::Percentage(60), Constraint::Percentage(40)]
    };
    let details_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(details_constraints)
        .split(right_rect);

    // Operator Detail Card
    let op_card_rect = details_layout[0];
    let op_block = Block::default()
        .title(" 📇 Operator QRZ Profile ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    if let Some(active_op) = app.active_participant() {
        let qrz_status_span = if active_op.qrz_fetched {
            Span::styled(" ✔ QRZ Verified", Style::default().fg(Color::LightGreen))
        } else {
            Span::styled(
                " ⌛ Fetching QRZ data...",
                Style::default().fg(Color::LightYellow),
            )
        };

        let op_lines = vec![
            Line::from(vec![
                Span::styled(" Callsign:  ", Style::default().fg(Color::Gray)),
                Span::styled(
                    &active_op.callsign,
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("    "),
                qrz_status_span,
            ]),
            Line::from(vec![
                Span::styled(" Name:      ", Style::default().fg(Color::Gray)),
                Span::styled(
                    active_op.name.as_deref().unwrap_or("Unknown Operator"),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(" Location:  ", Style::default().fg(Color::Gray)),
                Span::styled(
                    active_op
                        .location
                        .as_deref()
                        .unwrap_or("No details available"),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::raw(""),
            Line::from(vec![Span::styled(
                " Current Scorecard: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::UNDERLINED),
            )]),
            Line::from(vec![
                Span::raw("  • Check-in Score:   "),
                Span::styled(
                    format!("{} pt", active_op.points_checkin),
                    Style::default().fg(Color::LightGreen),
                ),
            ]),
            Line::from(vec![
                Span::raw("  • Trivia Score:     "),
                Span::styled(
                    format!("{} pt", active_op.points_trivia),
                    Style::default().fg(Color::LightGreen),
                ),
                Span::styled(
                    "  [y] correct  | [x] deduct",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(vec![
                Span::raw("  • Bonus Score:      "),
                Span::styled(
                    format!("{} pt", active_op.points_bonus),
                    Style::default().fg(Color::LightGreen),
                ),
                Span::styled(
                    "  [b] bonus    | [v] deduct",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ];

        let op_card = Paragraph::new(op_lines)
            .block(op_block)
            .wrap(Wrap { trim: true });

        f.render_widget(op_card, op_card_rect);
    } else {
        // No participants in the queue
        let op_card = Paragraph::new(vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  Queue is currently empty.",
                Style::default().fg(Color::LightYellow),
            )),
            Line::raw(""),
            Line::raw("  Press [c] to check-in the first operator."),
        ])
        .block(op_block);

        f.render_widget(op_card, op_card_rect);
    }

    // Net Summary / Round status
    let summary_rect = details_layout[1];
    let summary_block = Block::default()
        .title(" 📊 Net Summary ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let total_checkins = app.participants.len();
    let current_turn_num = if app.queue.is_empty() {
        0
    } else {
        std::cmp::min(app.current_queue_index + 1, app.queue.len())
    };
    let total_queue_len = app.queue.len();

    let mut summary_lines = vec![
        Line::from(vec![
            Span::raw(" Total Unique Check-ins: "),
            Span::styled(
                format!("{}", total_checkins),
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw(" Current Turn Progress:  "),
            Span::styled(
                format!("{}/{}", current_turn_num, total_queue_len),
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    if let Some(msg) = &app.status_message {
        summary_lines.push(Line::raw(""));
        summary_lines.push(Line::from(Span::styled(
            format!(" ✔ {}", msg),
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )));
    } else if let Some(err) = &app.error_message {
        summary_lines.push(Line::raw(""));
        summary_lines.push(Line::from(Span::styled(
            format!(" ⚠️ {}", err),
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        )));
    } else if !app.queue.is_empty() && app.current_queue_index >= app.queue.len() {
        summary_lines.push(Line::raw(""));
        summary_lines.push(Line::from(Span::styled(
            " 🏆 ROUND COMPLETED! 🏆",
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )));
        summary_lines.push(Line::from(Span::styled(
            " Press [r] for next round or [f] to view final scores.",
            Style::default().fg(Color::LightYellow),
        )));
    } else if app.queue.is_empty() {
        summary_lines.push(Line::raw(""));
        summary_lines.push(Line::from(Span::styled(
            " 📡 Ready for trivia net operations. ",
            Style::default().fg(Color::Gray),
        )));
    }

    let summary_p = Paragraph::new(summary_lines).block(summary_block);
    f.render_widget(summary_p, summary_rect);

    // ==========================================
    // 4. FOOTER
    // ==========================================
    let mut control_spans = vec![
        Span::styled(
            " [c]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Add "),
        Span::styled(
            " [e]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Edit "),
        Span::styled(
            " [d]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Del "),
        Span::styled(
            " [n/p/↕]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Turn "),
        Span::styled(
            " [y/b]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Point "),
        Span::styled(
            " [r]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Rotate "),
        Span::styled(
            " [f]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Scores "),
        Span::styled(
            " [s]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Export "),
    ];

    if app.trivia_topic.is_some() {
        control_spans.extend(vec![
            Span::styled(
                " [a]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Ans "),
            Span::styled(
                " [ [ / ] ]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Q Nav "),
        ]);
    }

    control_spans.extend(vec![
        Span::styled(
            " [Backspace]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Clear "),
        Span::styled(
            " [q]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Quit "),
    ]);

    let controls_p = Paragraph::new(Line::from(control_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(controls_p, footer_chunk);

    // ==========================================
    // 4. POPUP INPUT MODAL
    // ==========================================
    if let InputMode::AddCheckin = app.input_mode {
        let popup_area = centered_rect(50, 20, size);

        // Clear background of the popup area
        f.render_widget(Clear, popup_area);

        let input_block = Block::default()
            .title(" 📥 Operator Check-in ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightCyan));

        let input_text = vec![
            Line::raw(" Enter Ham Radio Callsign:"),
            Line::raw(""),
            Line::from(vec![
                Span::raw(" > "),
                Span::styled(
                    &app.input_buffer,
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "_",
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::SLOW_BLINK),
                ), // Blinking cursor block
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                " Press [Enter] to submit, [Esc] to cancel.",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let input_p = Paragraph::new(input_text).block(input_block);
        f.render_widget(input_p, popup_area);
    }

    // ==========================================
    // 5. POPUP CLEAR CONFIRM MODAL
    // ==========================================
    if let InputMode::ClearConfirm = app.input_mode {
        let popup_area = centered_rect(50, 20, size);

        // Clear background of the popup area
        f.render_widget(Clear, popup_area);

        let confirm_block = Block::default()
            .title(" ⚠️ Clear Session Log ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightRed));

        let confirm_text = vec![
            Line::raw(" Are you sure you want to delete the log"),
            Line::raw(" and reset all contestants and scores?"),
            Line::raw(""),
            Line::from(vec![
                Span::styled(" Press ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "[y]",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to confirm reset, or ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "[n or Esc]",
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to cancel.", Style::default().fg(Color::Gray)),
            ]),
        ];

        let confirm_p = Paragraph::new(confirm_text).block(confirm_block);
        f.render_widget(confirm_p, popup_area);
    }

    // ==========================================
    // 6. POPUP EDIT CALLSIGN MODAL
    // ==========================================
    if let InputMode::EditCallsign = app.input_mode {
        let popup_area = centered_rect(50, 20, size);
        f.render_widget(Clear, popup_area);

        let active_call = app
            .active_participant()
            .map(|p| p.callsign.as_str())
            .unwrap_or("");

        let edit_block = Block::default()
            .title(format!(" ✏️ Edit Operator Callsign ({}) ", active_call))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightYellow));

        let edit_text = vec![
            Line::raw(" Modify operator callsign:"),
            Line::raw(""),
            Line::from(vec![
                Span::raw(" > "),
                Span::styled(
                    &app.input_buffer,
                    Style::default()
                        .fg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "_",
                    Style::default()
                        .fg(Color::LightYellow)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                " Press [Enter] to save & update QRZ, [Esc] to cancel.",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let edit_p = Paragraph::new(edit_text).block(edit_block);
        f.render_widget(edit_p, popup_area);
    }

    // ==========================================
    // 7. POPUP DELETE OPERATOR MODAL
    // ==========================================
    if let InputMode::DeleteConfirm = app.input_mode {
        let popup_area = centered_rect(52, 22, size);
        f.render_widget(Clear, popup_area);

        let target_call = app
            .active_participant()
            .map(|p| p.callsign.as_str())
            .unwrap_or("operator");

        let delete_block = Block::default()
            .title(" 🗑️ Delete Operator ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightRed));

        let delete_text = vec![
            Line::from(vec![
                Span::raw(" Are you sure you want to remove "),
                Span::styled(
                    target_call,
                    Style::default()
                        .fg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" from the net?"),
            ]),
            Line::raw(" This will remove them from the active queue and scoreboard."),
            Line::raw(""),
            Line::from(vec![
                Span::styled(" Press ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "[y]",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to confirm deletion, or ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "[n or Esc]",
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to cancel.", Style::default().fg(Color::Gray)),
            ]),
        ];

        let delete_p = Paragraph::new(delete_text).block(delete_block);
        f.render_widget(delete_p, popup_area);
    }

    // ==========================================
    // 8. POPUP EXPORT CONTEST MODAL
    // ==========================================
    if let InputMode::ExportDialog { format } = app.input_mode {
        let popup_area = centered_rect(56, 24, size);
        f.render_widget(Clear, popup_area);

        let export_block = Block::default()
            .title(" 💾 Export Contest Results ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightGreen));

        let (csv_span, json_span) = match format {
            crate::state::ExportFormat::Csv => (
                Span::styled(
                    "[● CSV Format]",
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ○ JSON Format", Style::default().fg(Color::DarkGray)),
            ),
            crate::state::ExportFormat::Json => (
                Span::styled("  ○ CSV Format", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "[● JSON Format]",
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                ),
            ),
        };

        let export_text = vec![
            Line::from(vec![
                Span::raw(" Format: "),
                csv_span,
                Span::raw("    "),
                json_span,
                Span::styled("   (Press [Tab] to switch)", Style::default().fg(Color::Gray)),
            ]),
            Line::raw(""),
            Line::raw(" Destination Filename:"),
            Line::from(vec![
                Span::raw(" > "),
                Span::styled(
                    &app.input_buffer,
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "_",
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                " Press [Enter] to export, [Tab] to toggle format, [Esc] to cancel.",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let export_p = Paragraph::new(export_text).block(export_block);
        f.render_widget(export_p, popup_area);
    }

    // ==========================================
    // 9. POPUP FINAL SCORES / LEADERBOARD MODAL
    // ==========================================
    if let InputMode::FinalScores = app.input_mode {
        let popup_area = centered_rect(80, 75, size);
        f.render_widget(Clear, popup_area);

        let main_block = Block::default()
            .title(" 🏆 Contest Final Scores & Leaderboard 🏆 ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner_area = main_block.inner(popup_area);
        f.render_widget(main_block, popup_area);

        let popup_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Contest info banner
                Constraint::Min(4),    // Table or empty text
                Constraint::Length(1), // Footer shortcuts
            ])
            .split(inner_area);

        // Header info banner
        let topic_name = app
            .trivia_topic
            .as_ref()
            .map(|t| t.title.as_str())
            .unwrap_or("Amateur Radio Trivia Net");

        let header_line = Line::from(vec![
            Span::styled(" Topic: ", Style::default().fg(Color::Gray)),
            Span::styled(
                topic_name,
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Round: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", app.round_number),
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Check-ins: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", app.participants.len()),
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        f.render_widget(Paragraph::new(vec![header_line, Line::raw("")]), popup_chunks[0]);

        // Scoreboard table
        let scoreboard = app.get_scoreboard();
        if scoreboard.is_empty() {
            let empty_p = Paragraph::new(vec![
                Line::raw(""),
                Line::from(Span::styled(
                    "  No participants checked in yet. Press [c] to check-in operators.",
                    Style::default().fg(Color::LightYellow),
                )),
            ]);
            f.render_widget(empty_p, popup_chunks[1]);
        } else {
            let visible_capacity = popup_chunks[1].height.saturating_sub(2) as usize; // header + spacing
            let scroll = app.scoreboard_scroll.min(scoreboard.len().saturating_sub(1));
            let visible_slice = &scoreboard[scroll..std::cmp::min(scroll + visible_capacity, scoreboard.len())];

            let header_cells = [
                "Rank", "Callsign", "Operator Name", "Location", "Check-in", "Trivia", "Bonus", "Score",
            ]
            .iter()
            .map(|h| {
                Span::styled(
                    *h,
                    Style::default()
                        .fg(Color::LightBlue)
                        .add_modifier(Modifier::BOLD),
                )
            });
            let header_row = Row::new(header_cells).height(1).bottom_margin(1);

            let mut rows = Vec::new();
            for &(rank, ref p) in visible_slice {
                let (rank_str, rank_style) = match rank {
                    1 => (
                        " 🥇 1 ".to_string(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    2 => (
                        " 🥈 2 ".to_string(),
                        Style::default()
                            .fg(Color::LightCyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    3 => (
                        " 🥉 3 ".to_string(),
                        Style::default()
                            .fg(Color::LightYellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    _ => (
                        format!("  {:^3} ", rank),
                        Style::default().fg(Color::Gray),
                    ),
                };

                let score_style = Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD);

                let row_cells = vec![
                    Span::styled(rank_str, rank_style),
                    Span::styled(
                        format!("{:<9}", p.callsign),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<20}", p.name.as_deref().unwrap_or("Unknown Operator")),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!("{:<20}", p.location.as_deref().unwrap_or("-")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!(" {:^8} ", p.points_checkin),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!(" {:^6} ", p.points_trivia),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!(" {:^5} ", p.points_bonus),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(
                        format!(" {:^6} ", p.total_score()),
                        score_style,
                    ),
                ];

                let mut row = Row::new(row_cells).height(1);
                if rank == 1 {
                    row = row.style(Style::default().bg(Color::Rgb(35, 35, 15)));
                }
                rows.push(row);
            }

            let score_table = Table::new(
                rows,
                [
                    Constraint::Length(7),  // Rank
                    Constraint::Length(10), // Callsign
                    Constraint::Min(20),    // Name
                    Constraint::Min(20),    // Location
                    Constraint::Length(10), // Check-in Pt
                    Constraint::Length(8),  // Trivia Pt
                    Constraint::Length(8),  // Bonus Pt
                    Constraint::Length(8),  // Total Score
                ],
            )
            .header(header_row);

            f.render_widget(score_table, popup_chunks[1]);
        }

        // Footer hint
        let mut hint_spans = vec![
            Span::styled(
                " [Esc/Enter/f]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Close  "),
            Span::styled(
                " [↑/↓/PgUp/PgDn]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Scroll  "),
            Span::styled(
                " [s]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Export  "),
        ];
        if !scoreboard.is_empty() {
            let visible_capacity = popup_chunks[1].height.saturating_sub(2) as usize;
            if scoreboard.len() > visible_capacity {
                let scroll = app.scoreboard_scroll.min(scoreboard.len().saturating_sub(1));
                let end = (scroll + visible_capacity).min(scoreboard.len());
                hint_spans.push(Span::styled(
                    format!("(Showing {}-{} of {})", scroll + 1, end, scoreboard.len()),
                    Style::default().fg(Color::LightCyan),
                ));
            }
        }

        let hint_p = Paragraph::new(Line::from(hint_spans));
        f.render_widget(hint_p, popup_chunks[2]);
    }
}
