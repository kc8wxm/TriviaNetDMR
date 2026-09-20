use crate::state::{App, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Row, Table, Wrap},
    Frame,
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

    // Main Layout (Header, Body, Footer)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Footer / Controls
        ])
        .split(size);

    // ==========================================
    // 1. HEADER
    // ==========================================
    let header_chunk = chunks[0];
    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Title
            Constraint::Percentage(30), // Round Status
            Constraint::Percentage(30), // API / Connection Status
        ])
        .split(header_chunk);

    let title_p = Paragraph::new(Line::from(vec![
        Span::styled(" 📻 TriviaNetDMR ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Net Control Companion", Style::default().fg(Color::Gray)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));

    let round_status_p = Paragraph::new(Line::from(vec![
        Span::styled(" Round: ", Style::default().fg(Color::Gray)),
        Span::styled(format!("{}", app.round_number), Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));

    let api_status_style = if app.api_status.contains("Offline") {
        Style::default().fg(Color::LightYellow)
    } else {
        Style::default().fg(Color::LightGreen)
    };
    let api_p = Paragraph::new(Line::from(vec![
        Span::styled(" QRZ API: ", Style::default().fg(Color::Gray)),
        Span::styled(&app.api_status, api_status_style),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));

    f.render_widget(title_p, header_layout[0]);
    f.render_widget(round_status_p, header_layout[1]);
    f.render_widget(api_p, header_layout[2]);


    // ==========================================
    // 2. MAIN BODY
    // ==========================================
    let body_chunk = chunks[1];
    let body_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Participant list table
            Constraint::Percentage(40), // Active info & statistics
        ])
        .split(body_chunk);

    // -- Left side: Participant Table --
    let table_rect = body_layout[0];
    let header_cells = ["Act", "Callsign", "Name", "Check-in", "Trivia", "Bonus", "Score"]
        .iter()
        .map(|h| Span::styled(*h, Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)));
    let header_row = Row::new(header_cells).height(1).bottom_margin(1);

    let mut rows = Vec::new();
    for (i, &p_id) in app.queue.iter().enumerate() {
        if let Some(p) = app.participants.get(p_id) {
            let is_active = i == app.current_queue_index;
            let active_marker = if is_active { " 👉 " } else { "    " };
            
            // Text color depends on whether participant is currently active
            let cell_style = if is_active {
                Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)
            } else if i < app.current_queue_index {
                Style::default().fg(Color::DarkGray) // Already took their turn
            } else {
                Style::default().fg(Color::White)
            };

            let row_cells = vec![
                Span::styled(active_marker, Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:<8}", p.callsign), cell_style),
                Span::styled(format!("{:<15}", p.name.as_deref().unwrap_or("Fetching...")), cell_style),
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
        ]
    )
    .header(header_row)
    .block(list_block);

    f.render_widget(table, table_rect);


    // -- Right side: Details & Status --
    let right_rect = body_layout[1];
    let details_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(11), // Selected Operator Card
            Constraint::Min(4),    // Net Stats / Round Status block
        ])
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
            Span::styled(" ⌛ Fetching QRZ data...", Style::default().fg(Color::LightYellow))
        };

        let op_lines = vec![
            Line::from(vec![
                Span::styled(" Callsign:  ", Style::default().fg(Color::Gray)),
                Span::styled(&active_op.callsign, Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
                Span::raw("    "),
                qrz_status_span,
            ]),
            Line::from(vec![
                Span::styled(" Name:      ", Style::default().fg(Color::Gray)),
                Span::styled(active_op.name.as_deref().unwrap_or("Unknown Operator"), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled(" Location:  ", Style::default().fg(Color::Gray)),
                Span::styled(active_op.location.as_deref().unwrap_or("No details available"), Style::default().fg(Color::White)),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled(" Current Scorecard: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED)),
            ]),
            Line::from(vec![
                Span::raw("  • Check-in Score:   "),
                Span::styled(format!("{} pt", active_op.points_checkin), Style::default().fg(Color::LightGreen)),
            ]),
            Line::from(vec![
                Span::raw("  • Trivia Score:     "),
                Span::styled(format!("{} pt", active_op.points_trivia), Style::default().fg(Color::LightGreen)),
                Span::styled("  [y] correct  | [x] deduct", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(vec![
                Span::raw("  • Bonus Score:      "),
                Span::styled(format!("{} pt", active_op.points_bonus), Style::default().fg(Color::LightGreen)),
                Span::styled("  [b] bonus    | [v] deduct", Style::default().fg(Color::DarkGray)),
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
            Line::from(Span::styled("  Queue is currently empty.", Style::default().fg(Color::LightYellow))),
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
    let current_turn_num = if app.queue.is_empty() { 0 } else { std::cmp::min(app.current_queue_index + 1, app.queue.len()) };
    let total_queue_len = app.queue.len();

    let mut summary_lines = vec![
        Line::from(vec![
            Span::raw(" Total Unique Check-ins: "),
            Span::styled(format!("{}", total_checkins), Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw(" Current Turn Progress:  "),
            Span::styled(format!("{}/{}", current_turn_num, total_queue_len), Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
        ]),
    ];

    if !app.queue.is_empty() && app.current_queue_index >= app.queue.len() {
        summary_lines.push(Line::raw(""));
        summary_lines.push(Line::from(Span::styled(" 🏆 ROUND COMPLETED! 🏆", Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD))));
        summary_lines.push(Line::from(Span::styled(" Press [r] to begin the next round (order rotates!).", Style::default().fg(Color::LightYellow))));
    } else if app.queue.is_empty() {
        summary_lines.push(Line::raw(""));
        summary_lines.push(Line::from(Span::styled(" 📡 Ready for trivia net operations. ", Style::default().fg(Color::Gray))));
    }

    let summary_p = Paragraph::new(summary_lines)
        .block(summary_block);
    f.render_widget(summary_p, summary_rect);


    // ==========================================
    // 3. FOOTER
    // ==========================================
    let footer_chunk = chunks[2];
    let controls_p = Paragraph::new(Line::from(vec![
        Span::styled(" [c]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)), Span::raw(" Check-in "),
        Span::styled(" [n/Enter]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)), Span::raw(" Next Turn "),
        Span::styled(" [p]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)), Span::raw(" Prev Turn "),
        Span::styled(" [y]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)), Span::raw(" Correct (+1pt) "),
        Span::styled(" [b]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)), Span::raw(" Bonus (+1pt) "),
        Span::styled(" [r]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)), Span::raw(" Rotate Round "),
        Span::styled(" [q]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)), Span::raw(" Quit "),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));

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
                Span::styled(&app.input_buffer, Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
                Span::styled("_", Style::default().fg(Color::LightGreen).add_modifier(Modifier::SLOW_BLINK)), // Blinking cursor block
            ]),
            Line::raw(""),
            Line::from(Span::styled(" Press [Enter] to submit, [Esc] to cancel.", Style::default().fg(Color::DarkGray))),
        ];

        let input_p = Paragraph::new(input_text).block(input_block);
        f.render_widget(input_p, popup_area);
    }
}
