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

use crate::trivia::{TriviaQuestion, TriviaTopic};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    AddCheckin,
    ClearConfirm,
    EditCallsign,
    DeleteConfirm,
    ExportDialog { format: ExportFormat },
    FinalScores,
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
    pub scoreboard_scroll: usize,
    pub error_message: Option<String>,
    pub status_message: Option<String>,
    pub api_status: String,
    pub trivia_topic: Option<TriviaTopic>,
    pub current_question_index: usize,
    pub show_answer: bool,
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
            scoreboard_scroll: 0,
            error_message: None,
            status_message: None,
            api_status: String::from("Offline (Mock Mode)"),
            trivia_topic: None,
            current_question_index: 0,
            show_answer: true,
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

    /// Advances the round, rotating the queue left by 1 and advancing the question.
    pub fn next_round(&mut self) {
        if self.queue.is_empty() && self.trivia_topic.is_none() {
            return;
        }

        if !self.queue.is_empty() {
            // Rotate queue left by 1.
            // E.g. [A, B, C, D] -> [B, C, D, A]
            self.queue.rotate_left(1);
        }
        self.round_number += 1;
        self.current_queue_index = 0;

        // Advance current_question_index if trivia topic is loaded
        if let Some(topic) = &self.trivia_topic
            && !topic.questions.is_empty()
        {
            if self.round_number - 1 < topic.questions.len() {
                self.current_question_index = self.round_number - 1;
            } else {
                self.current_question_index = (self.round_number - 1) % topic.questions.len();
            }
        }
    }

    /// Returns the currently active trivia question, if any
    pub fn current_question(&self) -> Option<&TriviaQuestion> {
        self.trivia_topic
            .as_ref()
            .and_then(|t| t.questions.get(self.current_question_index))
    }

    /// Manually moves to the next trivia question
    pub fn next_question(&mut self) {
        if let Some(topic) = &self.trivia_topic
            && !topic.questions.is_empty()
            && self.current_question_index + 1 < topic.questions.len()
        {
            self.current_question_index += 1;
        }
    }

    /// Manually moves to the previous trivia question
    pub fn prev_question(&mut self) {
        if self.current_question_index > 0 {
            self.current_question_index -= 1;
        }
    }

    /// Toggles visibility of the trivia answer and fact
    pub fn toggle_show_answer(&mut self) {
        self.show_answer = !self.show_answer;
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
            current_question_index: self.current_question_index,
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
        self.current_question_index = dump.current_question_index;
        Ok(())
    }

    /// Resets the application state to a clean slate and deletes the saved log file from disk.
    pub fn clear_session(&mut self, filepath: &str) -> Result<(), std::io::Error> {
        self.participants.clear();
        self.queue.clear();
        self.current_queue_index = 0;
        self.round_number = 1;
        self.current_question_index = 0;
        self.error_message = None;
        self.status_message = None;

        if std::path::Path::new(filepath).exists() {
            std::fs::remove_file(filepath)?;
        }
        Ok(())
    }

    /// Edits the callsign of an existing participant.
    /// Resets QRZ metadata so updated information can be looked up.
    pub fn edit_participant_callsign(
        &mut self,
        participant_id: usize,
        new_callsign: String,
    ) -> Result<String, String> {
        let call_upper = new_callsign.trim().to_uppercase();
        if call_upper.is_empty() {
            return Err("Callsign cannot be empty.".to_string());
        }

        // Check if another participant already has this callsign
        if self
            .participants
            .iter()
            .any(|p| p.id != participant_id && p.callsign == call_upper)
        {
            return Err(format!("Callsign {} is already registered in the net.", call_upper));
        }

        if let Some(p) = self.participants.get_mut(participant_id) {
            if p.callsign == call_upper {
                return Ok(call_upper);
            }
            p.callsign = call_upper.clone();
            p.name = None;
            p.location = None;
            p.qrz_fetched = false;
            Ok(call_upper)
        } else {
            Err("Participant not found.".to_string())
        }
    }

    /// Deletes a participant by ID, removing them from queue and roster,
    /// and updating all participant IDs and queue references.
    pub fn delete_participant(&mut self, participant_id: usize) -> Result<String, String> {
        if participant_id >= self.participants.len() {
            return Err("Participant not found.".to_string());
        }

        let removed_callsign = self.participants[participant_id].callsign.clone();

        // Count how many occurrences of participant_id were before current_queue_index
        let before_count = self
            .queue
            .iter()
            .take(self.current_queue_index)
            .filter(|&&id| id == participant_id)
            .count();

        // 1. Remove from queue
        self.queue.retain(|&id| id != participant_id);

        // 2. Adjust current_queue_index for items removed before it
        self.current_queue_index = self.current_queue_index.saturating_sub(before_count);

        // 3. Remove from participants
        self.participants.remove(participant_id);

        // 4. Re-index participants
        for (idx, p) in self.participants.iter_mut().enumerate() {
            p.id = idx;
        }

        // 5. Re-index queue references
        for q in self.queue.iter_mut() {
            if *q > participant_id {
                *q -= 1;
            }
        }

        // 6. Ensure current_queue_index is within bounds
        if self.queue.is_empty() {
            self.current_queue_index = 0;
        } else if self.current_queue_index >= self.queue.len() {
            self.current_queue_index = self.queue.len() - 1;
        }

        Ok(removed_callsign)
    }

    /// Returns participants sorted by score (descending) with rank (1-indexed).
    /// Breaks ties by callsign alphabetically.
    pub fn get_scoreboard(&self) -> Vec<(usize, Participant)> {
        let mut sorted = self.participants.clone();
        sorted.sort_by(|a, b| {
            b.total_score()
                .cmp(&a.total_score())
                .then_with(|| a.callsign.cmp(&b.callsign))
        });
        sorted
            .into_iter()
            .enumerate()
            .map(|(idx, p)| (idx + 1, p))
            .collect()
    }

    /// Exports the current contest results to a CSV file.
    pub fn export_to_csv(&self, filepath: &str) -> Result<(), std::io::Error> {
        let sorted = self.get_scoreboard();

        let mut csv = String::new();
        csv.push_str("Rank,Callsign,Name,Location,CheckIn_Points,Trivia_Points,Bonus_Points,Total_Score\n");

        for (rank, p) in sorted {
            let call = csv_escape(&p.callsign);
            let name = csv_escape(p.name.as_deref().unwrap_or(""));
            let loc = csv_escape(p.location.as_deref().unwrap_or(""));
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                rank,
                call,
                name,
                loc,
                p.points_checkin,
                p.points_trivia,
                p.points_bonus,
                p.total_score()
            ));
        }

        std::fs::write(filepath, csv)?;
        Ok(())
    }

    /// Exports the current contest results to a JSON file.
    pub fn export_to_json(&self, filepath: &str) -> Result<(), std::io::Error> {
        let sorted = self.get_scoreboard();

        let export_participants: Vec<ContestParticipantExport> = sorted
            .into_iter()
            .map(|(rank, p)| {
                let total_score = p.total_score();
                ContestParticipantExport {
                    rank,
                    callsign: p.callsign,
                    name: p.name,
                    location: p.location,
                    points_checkin: p.points_checkin,
                    points_trivia: p.points_trivia,
                    points_bonus: p.points_bonus,
                    total_score,
                }
            })
            .collect();

        let export = ContestExport {
            title: "TriviaNetDMR Contest Results".to_string(),
            export_timestamp: chrono::Utc::now().to_rfc3339(),
            round_number: self.round_number,
            total_participants: export_participants.len(),
            topic: self.trivia_topic.as_ref().map(|t| t.title.clone()),
            participants: export_participants,
        };

        let serialized = serde_json::to_string_pretty(&export).map_err(std::io::Error::other)?;
        std::fs::write(filepath, serialized)?;
        Ok(())
    }
}

/// Helper to escape a field for CSV according to RFC 4180
fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ContestExport {
    pub title: String,
    pub export_timestamp: String,
    pub round_number: usize,
    pub total_participants: usize,
    pub topic: Option<String>,
    pub participants: Vec<ContestParticipantExport>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ContestParticipantExport {
    pub rank: usize,
    pub callsign: String,
    pub name: Option<String>,
    pub location: Option<String>,
    pub points_checkin: u32,
    pub points_trivia: u32,
    pub points_bonus: u32,
    pub total_score: u32,
}

/// Helper struct for serializing and deserializing the state of the application.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AppStateDump {
    pub participants: Vec<Participant>,
    pub queue: Vec<usize>,
    pub current_queue_index: usize,
    pub round_number: usize,
    #[serde(default)]
    pub current_question_index: usize,
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
        assert_eq!(app.current_question_index, 0);
        assert!(!std::path::Path::new(temp_file).exists());
    }

    #[test]
    fn test_trivia_round_sync_and_navigation() {
        let mut app = App::new();
        let topic = TriviaTopic::new(
            "Test Trivia".to_string(),
            vec![
                TriviaQuestion {
                    number: 1,
                    question: "Q1".to_string(),
                    answer: "A1".to_string(),
                    fact: None,
                },
                TriviaQuestion {
                    number: 2,
                    question: "Q2".to_string(),
                    answer: "A2".to_string(),
                    fact: None,
                },
                TriviaQuestion {
                    number: 3,
                    question: "Q3".to_string(),
                    answer: "A3".to_string(),
                    fact: None,
                },
            ],
        );
        app.trivia_topic = Some(topic);

        assert_eq!(app.current_question_index, 0);
        assert_eq!(app.current_question().unwrap().question, "Q1");

        // Next round automatically advances question
        app.next_round();
        assert_eq!(app.round_number, 2);
        assert_eq!(app.current_question_index, 1);
        assert_eq!(app.current_question().unwrap().question, "Q2");

        // Manual question navigation
        app.prev_question();
        assert_eq!(app.current_question_index, 0);
        app.prev_question(); // Should not underflow
        assert_eq!(app.current_question_index, 0);

        app.next_question();
        assert_eq!(app.current_question_index, 1);
        app.next_question();
        assert_eq!(app.current_question_index, 2);
        app.next_question(); // Should not exceed bounds
        assert_eq!(app.current_question_index, 2);

        // Toggle answer
        assert!(app.show_answer);
        app.toggle_show_answer();
        assert!(!app.show_answer);
    }

    #[test]
    fn test_edit_participant_callsign() {
        let mut app = App::new();
        let id_a = app.add_participant("W1AW".to_string());
        let id_b = app.add_participant("K1ABC".to_string());

        // Successfully edit callsign
        let res = app.edit_participant_callsign(id_a, "W1XYZ".to_string());
        assert!(res.is_ok());
        assert_eq!(app.participants[id_a].callsign, "W1XYZ");
        assert!(!app.participants[id_a].qrz_fetched);

        // Edit to existing callsign of another operator should fail
        let dup_res = app.edit_participant_callsign(id_a, "K1ABC".to_string());
        assert!(dup_res.is_err());
        assert_eq!(app.participants[id_a].callsign, "W1XYZ");

        // Edit to empty should fail
        let empty_res = app.edit_participant_callsign(id_a, "   ".to_string());
        assert!(empty_res.is_err());

        // Same callsign is a no-op success
        let same_res = app.edit_participant_callsign(id_b, "K1ABC".to_string());
        assert!(same_res.is_ok());
    }

    #[test]
    fn test_delete_participant() {
        let mut app = App::new();
        let id_a = app.add_participant("W1AW".to_string());  // 0
        let id_b = app.add_participant("K1ABC".to_string()); // 1
        let id_c = app.add_participant("N1AAA".to_string()); // 2

        assert_eq!(app.queue, vec![id_a, id_b, id_c]);

        // Move active turn to K1ABC (queue index 1)
        app.next_turn();
        assert_eq!(app.current_queue_index, 1);
        assert_eq!(app.active_participant().unwrap().callsign, "K1ABC");

        // Delete W1AW (id 0)
        let del_res = app.delete_participant(0);
        assert_eq!(del_res.unwrap(), "W1AW");

        // Participants len should now be 2
        assert_eq!(app.participants.len(), 2);
        assert_eq!(app.participants[0].callsign, "K1ABC");
        assert_eq!(app.participants[0].id, 0);
        assert_eq!(app.participants[1].callsign, "N1AAA");
        assert_eq!(app.participants[1].id, 1);

        // Queue should now contain re-indexed values [0, 1]
        assert_eq!(app.queue, vec![0, 1]);

        // Active participant should still be K1ABC
        assert_eq!(app.active_participant().unwrap().callsign, "K1ABC");

        // Delete all remaining
        app.delete_participant(0).unwrap();
        app.delete_participant(0).unwrap();
        assert_eq!(app.participants.len(), 0);
        assert_eq!(app.queue.len(), 0);
        assert_eq!(app.current_queue_index, 0);
        assert!(app.active_participant().is_none());
    }

    #[test]
    fn test_export_csv_and_json() {
        let temp_csv = "test_export.csv";
        let temp_json = "test_export.json";

        let mut app = App::new();
        let id_a = app.add_participant("W1AW".to_string());
        let id_b = app.add_participant("K1ABC".to_string());

        app.participants[id_a].name = Some("Hiram Percy Maxim".to_string());
        app.participants[id_a].location = Some("Newington, CT, USA".to_string()); // contains comma
        app.participants[id_a].points_trivia = 3;

        app.participants[id_b].name = Some("Alice \"Sky\" Wonder".to_string()); // contains quotes
        app.participants[id_b].location = Some("Boston, MA".to_string());
        app.participants[id_b].points_trivia = 1;
        app.participants[id_b].points_bonus = 1;

        // Export to CSV
        app.export_to_csv(temp_csv).expect("Failed to export CSV");
        let csv_content = std::fs::read_to_string(temp_csv).expect("Failed to read CSV");
        let _ = std::fs::remove_file(temp_csv);

        assert!(csv_content.contains("Rank,Callsign,Name,Location"));
        assert!(csv_content.contains("\"Newington, CT, USA\""));
        assert!(csv_content.contains("\"Alice \"\"Sky\"\" Wonder\""));
        assert!(csv_content.contains("W1AW"));
        assert!(csv_content.contains("K1ABC"));

        // Export to JSON
        app.export_to_json(temp_json).expect("Failed to export JSON");
        let json_content = std::fs::read_to_string(temp_json).expect("Failed to read JSON");
        let _ = std::fs::remove_file(temp_json);

        let parsed: ContestExport = serde_json::from_str(&json_content).expect("Failed to parse JSON");
        assert_eq!(parsed.total_participants, 2);
        assert_eq!(parsed.participants[0].callsign, "W1AW");
        assert_eq!(parsed.participants[0].total_score, 4); // 1 checkin + 3 trivia
        assert_eq!(parsed.participants[1].callsign, "K1ABC");
        assert_eq!(parsed.participants[1].total_score, 3); // 1 checkin + 1 trivia + 1 bonus
    }

    #[test]
    fn test_delete_active_participant() {
        let mut app = App::new();
        app.add_participant("W1AW".to_string());  // 0
        app.add_participant("K1ABC".to_string()); // 1
        app.add_participant("N1AAA".to_string()); // 2

        // Active participant is W1AW at queue index 0
        assert_eq!(app.active_participant().unwrap().callsign, "W1AW");

        // Delete active participant (W1AW, id 0)
        app.delete_participant(0).unwrap();

        // Queue is now [0, 1] with K1ABC, N1AAA; index is 0 -> active is now K1ABC
        assert_eq!(app.active_participant().unwrap().callsign, "K1ABC");
        assert_eq!(app.current_queue_index, 0);

        // Move to last participant (index 1 -> N1AAA)
        app.next_turn();
        assert_eq!(app.active_participant().unwrap().callsign, "N1AAA");
        assert_eq!(app.current_queue_index, 1);

        // Delete active participant (N1AAA, id 1 in participants)
        app.delete_participant(1).unwrap();

        // Index was 1, now queue length is 1, so index clamps to 0 (K1ABC)
        assert_eq!(app.current_queue_index, 0);
        assert_eq!(app.active_participant().unwrap().callsign, "K1ABC");
    }

    #[test]
    fn test_edit_callsign_preserves_score() {
        let mut app = App::new();
        let id = app.add_participant("W1AW".to_string());
        app.participants[id].points_trivia = 5;
        app.participants[id].points_bonus = 2;
        app.participants[id].notes = "Net regular".to_string();

        app.edit_participant_callsign(id, "W1XYZ".to_string()).unwrap();

        let p = &app.participants[id];
        assert_eq!(p.callsign, "W1XYZ");
        assert_eq!(p.points_checkin, 1);
        assert_eq!(p.points_trivia, 5);
        assert_eq!(p.points_bonus, 2);
        assert_eq!(p.total_score(), 8);
        assert_eq!(p.notes, "Net regular");
        assert!(!p.qrz_fetched);
    }

    #[test]
    fn test_get_scoreboard() {
        let mut app = App::new();
        assert!(app.get_scoreboard().is_empty());

        let id_a = app.add_participant("W1AW".to_string());
        let id_b = app.add_participant("K1ABC".to_string());
        let id_c = app.add_participant("N1AAA".to_string());

        app.participants[id_a].points_trivia = 2; // Total: 3
        app.participants[id_b].points_trivia = 4; // Total: 5
        app.participants[id_c].points_trivia = 2; // Total: 3 (tie with W1AW, but N1AAA comes first alphabetically)

        let scoreboard = app.get_scoreboard();
        assert_eq!(scoreboard.len(), 3);

        // 1st: K1ABC (score 5)
        assert_eq!(scoreboard[0].0, 1);
        assert_eq!(scoreboard[0].1.callsign, "K1ABC");
        assert_eq!(scoreboard[0].1.total_score(), 5);

        // 2nd: N1AAA (score 3)
        assert_eq!(scoreboard[1].0, 2);
        assert_eq!(scoreboard[1].1.callsign, "N1AAA");
        assert_eq!(scoreboard[1].1.total_score(), 3);

        // 3rd: W1AW (score 3)
        assert_eq!(scoreboard[2].0, 3);
        assert_eq!(scoreboard[2].1.callsign, "W1AW");
        assert_eq!(scoreboard[2].1.total_score(), 3);
    }
}
