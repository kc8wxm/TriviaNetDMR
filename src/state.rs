#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Participant {
    pub id: usize,
    pub callsign: String,
    pub name: Option<String>,
    pub location: Option<String>,
    pub points_checkin: u32,
    pub points_trivia: u32,
    pub points_bonus: u32,
    pub qrz_fetched: bool,
    pub notes: String,
}

impl Participant {
    pub fn new(id: usize, callsign: String) -> Self {
        Self {
            id,
            callsign: callsign.to_uppercase(),
            name: None,
            location: None,
            points_checkin: 1, // 1 point automatically for checking in
            points_trivia: 0,
            points_bonus: 0,
            qrz_fetched: false,
            notes: String::new(),
        }
    }

    pub fn total_score(&self) -> u32 {
        self.points_checkin + self.points_trivia + self.points_bonus
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    AddCheckin,
    ClearConfirm,
}

#[derive(Clone, Debug)]
pub struct App {
    pub participants: Vec<Participant>,
    /// Queue of participant IDs for the current round
    pub queue: Vec<usize>,
    /// Index in `queue` of the active participant
    pub current_queue_index: usize,
    pub round_number: usize,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub error_message: Option<String>,
    pub api_status: String,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            participants: Vec::new(),
            queue: Vec::new(),
            current_queue_index: 0,
            round_number: 1,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            error_message: None,
            api_status: String::from("Offline (Mock Mode)"),
        }
    }

    /// Adds a new participant to the system.
    /// Returns the ID of the newly added participant.
    pub fn add_participant(&mut self, callsign: String) -> usize {
        let call_upper = callsign.trim().to_uppercase();

        // Prevent duplicate check-ins in the same session, but we can append to the queue again if needed.
        // Actually, let's see if the participant already exists.
        if let Some(existing) = self.participants.iter().find(|p| p.callsign == call_upper) {
            let id = existing.id;
            // If they are not already in the active queue, add them
            if !self.queue.contains(&id) {
                self.queue.push(id);
            }
            return id;
        }

        let id = self.participants.len();
        let participant = Participant::new(id, call_upper);
        self.participants.push(participant);
        self.queue.push(id);
        id
    }

    /// Returns the currently active participant, if any
    pub fn active_participant(&self) -> Option<&Participant> {
        if self.queue.is_empty() {
            None
        } else {
            let active_id = self.queue[self.current_queue_index % self.queue.len()];
            self.participants.get(active_id)
        }
    }

    /// Returns a mutable reference to the active participant, if any
    pub fn active_participant_mut(&mut self) -> Option<&mut Participant> {
        if self.queue.is_empty() {
            None
        } else {
            let active_id = self.queue[self.current_queue_index % self.queue.len()];
            self.participants.get_mut(active_id)
        }
    }

    /// Advances turn to the next participant in the current round
    pub fn next_turn(&mut self) {
        if self.queue.is_empty() {
            return;
        }
        self.current_queue_index += 1;
        if self.current_queue_index >= self.queue.len() {
            // End of round reached. The UI will show a prompt or allow starting a new round.
            // We cap it so it doesn't overflow out of bounds or loop unexpectedly without an explicit next round action.
            self.current_queue_index = self.queue.len();
        }
    }

    /// Moves turn to the previous participant in the current round
    pub fn prev_turn(&mut self) {
        if self.current_queue_index > 0 {
            self.current_queue_index -= 1;
        }
    }

    /// Advances the round, rotating the queue left by 1.
    pub fn next_round(&mut self) {
        if self.queue.is_empty() {
            return;
        }

        // Rotate queue left by 1.
        // E.g. [A, B, C, D] -> [B, C, D, A]
        self.queue.rotate_left(1);
        self.round_number += 1;
        self.current_queue_index = 0;
    }

    /// Awards a trivia point to the active participant
    pub fn award_trivia_point(&mut self) {
        if let Some(p) = self.active_participant_mut() {
            p.points_trivia += 1;
        }
    }

    /// Awards a bonus/best answer point to the active participant
    pub fn award_bonus_point(&mut self) {
        if let Some(p) = self.active_participant_mut() {
            p.points_bonus += 1;
        }
    }

    /// Decrements trivia point from active participant (useful for correction)
    pub fn remove_trivia_point(&mut self) {
        if let Some(p) = self.active_participant_mut()
            && p.points_trivia > 0
        {
            p.points_trivia -= 1;
        }
    }

    /// Decrements bonus point from active participant (useful for correction)
    pub fn remove_bonus_point(&mut self) {
        if let Some(p) = self.active_participant_mut()
            && p.points_bonus > 0
        {
            p.points_bonus -= 1;
        }
    }

    /// Saves the current state of participants, queue, active turn, and round to a JSON file.
    pub fn save_to_file(&self, filepath: &str) -> Result<(), std::io::Error> {
        let dump = AppStateDump {
            participants: self.participants.clone(),
            queue: self.queue.clone(),
            current_queue_index: self.current_queue_index,
            round_number: self.round_number,
        };
        let serialized = serde_json::to_string_pretty(&dump).map_err(std::io::Error::other)?;
        std::fs::write(filepath, serialized)?;
        Ok(())
    }

    /// Loads the state from a JSON file, restoring participants, queue, active turn, and round.
    pub fn load_from_file(&mut self, filepath: &str) -> Result<(), std::io::Error> {
        let content = std::fs::read_to_string(filepath)?;
        let dump: AppStateDump = serde_json::from_str(&content).map_err(std::io::Error::other)?;

        self.participants = dump.participants;
        self.queue = dump.queue;
        self.current_queue_index = dump.current_queue_index;
        self.round_number = dump.round_number;
        Ok(())
    }

    /// Resets the application state to a clean slate and deletes the saved log file from disk.
    pub fn clear_session(&mut self, filepath: &str) -> Result<(), std::io::Error> {
        self.participants.clear();
        self.queue.clear();
        self.current_queue_index = 0;
        self.round_number = 1;
        self.error_message = None;

        if std::path::Path::new(filepath).exists() {
            std::fs::remove_file(filepath)?;
        }
        Ok(())
    }
}

/// Helper struct for serializing and deserializing the state of the application.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AppStateDump {
    pub participants: Vec<Participant>,
    pub queue: Vec<usize>,
    pub current_queue_index: usize,
    pub round_number: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_initialization() {
        let app = App::new();
        assert_eq!(app.round_number, 1);
        assert_eq!(app.participants.len(), 0);
        assert_eq!(app.queue.len(), 0);
    }

    #[test]
    fn test_add_participant() {
        let mut app = App::new();
        let id_a = app.add_participant("W1AW".to_string());
        let id_b = app.add_participant("K1ABC".to_string());

        assert_eq!(app.participants.len(), 2);
        assert_eq!(app.queue, vec![id_a, id_b]);
        assert_eq!(app.participants[id_a].callsign, "W1AW");
        assert_eq!(app.participants[id_a].points_checkin, 1);
    }

    #[test]
    fn test_scoring() {
        let mut app = App::new();
        app.add_participant("W1AW".to_string());

        assert_eq!(app.active_participant().unwrap().total_score(), 1);

        app.award_trivia_point();
        assert_eq!(app.active_participant().unwrap().total_score(), 2);
        assert_eq!(app.active_participant().unwrap().points_trivia, 1);

        app.award_bonus_point();
        assert_eq!(app.active_participant().unwrap().total_score(), 3);
        assert_eq!(app.active_participant().unwrap().points_bonus, 1);
    }

    #[test]
    fn test_queue_rotation_and_rounds() {
        let mut app = App::new();
        let id_a = app.add_participant("W1AW".to_string()); // 0
        let id_b = app.add_participant("K1ABC".to_string()); // 1
        let id_c = app.add_participant("N1AAA".to_string()); // 2

        // Round 1 order: W1AW, K1ABC, N1AAA
        assert_eq!(app.current_queue_index, 0);
        assert_eq!(app.active_participant().unwrap().id, id_a);

        app.next_turn();
        assert_eq!(app.current_queue_index, 1);
        assert_eq!(app.active_participant().unwrap().id, id_b);

        app.next_turn();
        assert_eq!(app.current_queue_index, 2);
        assert_eq!(app.active_participant().unwrap().id, id_c);

        // Advance to next round: should rotate left by 1
        app.next_round();
        assert_eq!(app.round_number, 2);
        assert_eq!(app.current_queue_index, 0);
        assert_eq!(app.queue, vec![id_b, id_c, id_a]); // [1, 2, 0]
        assert_eq!(app.active_participant().unwrap().id, id_b);
    }

    #[test]
    fn test_late_checkins() {
        let mut app = App::new();
        let id_a = app.add_participant("W1AW".to_string());
        let id_b = app.add_participant("K1ABC".to_string());

        // Round 1 start
        assert_eq!(app.queue, vec![id_a, id_b]);

        // Late check-in
        let id_c = app.add_participant("N1AAA".to_string());
        assert_eq!(app.queue, vec![id_a, id_b, id_c]);
    }

    #[test]
    fn test_save_and_load() {
        let temp_file = "test_temp_session.json";

        let mut app = App::new();
        let id_a = app.add_participant("W1AW".to_string());
        let id_b = app.add_participant("K1ABC".to_string());

        // Award points and modify state
        app.award_trivia_point();
        app.award_bonus_point();
        app.next_turn();
        app.next_round();

        // Save to file
        app.save_to_file(temp_file)
            .expect("Failed to save to test file");

        // Load into a new App instance
        let mut loaded_app = App::new();
        loaded_app
            .load_from_file(temp_file)
            .expect("Failed to load from test file");

        // Clean up temp file immediately
        let _ = std::fs::remove_file(temp_file);

        // Verify state is restored exactly
        assert_eq!(loaded_app.round_number, app.round_number);
        assert_eq!(loaded_app.current_queue_index, app.current_queue_index);
        assert_eq!(loaded_app.queue, app.queue);
        assert_eq!(loaded_app.participants.len(), app.participants.len());

        assert_eq!(loaded_app.participants[id_a].callsign, "W1AW");
        assert_eq!(loaded_app.participants[id_a].points_trivia, 1);
        assert_eq!(loaded_app.participants[id_a].points_bonus, 1);

        assert_eq!(loaded_app.participants[id_b].callsign, "K1ABC");
        assert_eq!(loaded_app.participants[id_b].points_trivia, 0);
    }

    #[test]
    fn test_clear_session() {
        let temp_file = "test_clear_session.json";
        let mut app = App::new();
        app.add_participant("W1AW".to_string());
        app.save_to_file(temp_file).expect("Failed to save");

        assert_eq!(app.participants.len(), 1);
        assert!(std::path::Path::new(temp_file).exists());

        app.clear_session(temp_file)
            .expect("Failed to clear session");

        assert_eq!(app.participants.len(), 0);
        assert_eq!(app.queue.len(), 0);
        assert_eq!(app.round_number, 1);
        assert_eq!(app.current_queue_index, 0);
        assert!(!std::path::Path::new(temp_file).exists());
    }
}
