use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TriviaQuestion {
    pub number: usize,
    pub question: String,
    pub answer: String,
    pub fact: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TriviaTopic {
    pub title: String,
    pub questions: Vec<TriviaQuestion>,
}

impl TriviaTopic {
    pub fn new(title: String, questions: Vec<TriviaQuestion>) -> Self {
        Self { title, questions }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let content = fs::read_to_string(path)?;
        Ok(Self::parse_markdown(&content))
    }

    pub fn parse_markdown(content: &str) -> Self {
        let mut title = String::from("Trivia Net");
        let mut questions = Vec::new();
        let mut current_q: Option<TriviaQuestion> = None;
        let mut parsing_target = 0; // 0 = question, 1 = answer, 2 = fact

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Header line (e.g. ### **10 Trivia Questions & Answers: Talk Like a Pirate Day**)
            if trimmed.starts_with('#') {
                let cleaned = clean_markdown(trimmed.trim_start_matches('#'));
                if !cleaned.is_empty() {
                    title = if let Some((_prefix, suffix)) = cleaned.split_once(':') {
                        let s = suffix.trim();
                        if !s.is_empty() {
                            s.to_string()
                        } else {
                            cleaned
                        }
                    } else {
                        cleaned
                    };
                }
                continue;
            }

            // Question prompt
            if let Some((num, q_text)) = parse_question_line(trimmed, questions.len() + 1) {
                if let Some(q) = current_q.take() {
                    questions.push(q);
                }
                current_q = Some(TriviaQuestion {
                    number: num,
                    question: q_text,
                    answer: String::new(),
                    fact: None,
                });
                parsing_target = 0;
                continue;
            }

            // Answer
            if let Some(ans_text) = parse_answer_line(trimmed) {
                if let Some(ref mut q) = current_q {
                    q.answer = ans_text;
                }
                parsing_target = 1;
                continue;
            }

            // Net Control Fact
            if let Some(fact_text) = parse_fact_line(trimmed) {
                if let Some(ref mut q) = current_q {
                    q.fact = Some(fact_text);
                }
                parsing_target = 2;
                continue;
            }

            // Multiline continuations
            if let Some(ref mut q) = current_q {
                let cleaned = clean_markdown(trimmed);
                match parsing_target {
                    0 => {
                        if !q.question.is_empty() {
                            q.question.push(' ');
                        }
                        q.question.push_str(&cleaned);
                    }
                    1 => {
                        if !q.answer.is_empty() {
                            q.answer.push(' ');
                        }
                        q.answer.push_str(&cleaned);
                    }
                    2 => {
                        if let Some(ref mut f) = q.fact {
                            f.push(' ');
                            f.push_str(&cleaned);
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(q) = current_q {
            questions.push(q);
        }

        Self { title, questions }
    }
}

fn parse_question_line(line: &str, next_num: usize) -> Option<(usize, String)> {
    let lower = line.to_lowercase();
    let q_pos = lower.find("question:")?;
    let before = &line[..q_pos];
    let after = &line[q_pos + "question:".len()..];

    let digits: String = before.chars().filter(|c| c.is_ascii_digit()).collect();
    let num = digits.parse::<usize>().unwrap_or(next_num);
    let question_text = clean_markdown(after);
    Some((num, question_text))
}

fn parse_answer_line(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let ans_pos = lower.find("answer:")?;
    let after = &line[ans_pos + "answer:".len()..];
    Some(clean_markdown(after))
}

fn parse_fact_line(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let fact_pos = if let Some(pos) = lower.find("net control fact:") {
        Some(pos + "net control fact:".len())
    } else {
        lower.find("fact:").map(|pos| pos + "fact:".len())
    }?;
    let after = &line[fact_pos..];
    Some(clean_markdown(after))
}

pub fn clean_markdown(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(&next_c) = chars.peek()
            && matches!(
                next_c,
                '.' | '!' | '?' | '(' | ')' | '[' | ']' | '-' | '*' | '_' | '"' | '\'' | '`'
            )
        {
            out.push(next_c);
            chars.next();
            continue;
        }
        if c == '*' || c == '_' || c == '`' {
            continue;
        }
        out.push(c);
    }
    out.trim().to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IssueLevel {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ValidationIssue {
    pub level: IssueLevel,
    pub question_number: Option<usize>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeckValidation {
    pub file_path: String,
    pub filename: String,
    pub title: String,
    pub question_count: usize,
    pub issues: Vec<ValidationIssue>,
}

impl DeckValidation {
    pub fn is_valid(&self) -> bool {
        !self.issues.iter().any(|i| i.level == IssueLevel::Error)
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.level == IssueLevel::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.level == IssueLevel::Warning)
            .count()
    }
}

pub fn validate_deck_content(file_path: &str, content: &str) -> DeckValidation {
    let filename = Path::new(file_path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(file_path)
        .to_string();

    let topic = TriviaTopic::parse_markdown(content);
    let mut issues = Vec::new();

    if topic.questions.is_empty() {
        issues.push(ValidationIssue {
            level: IssueLevel::Error,
            question_number: None,
            message: "No trivia questions found (expected 'Question:' format)".to_string(),
        });
    }

    if topic.title == "Trivia Net" && !content.contains('#') {
        issues.push(ValidationIssue {
            level: IssueLevel::Warning,
            question_number: None,
            message: "Topic title not explicitly defined with Markdown header (#)".to_string(),
        });
    }

    let mut seen_numbers = std::collections::HashSet::new();
    let mut expected_seq_num = 1;
    let mut has_non_sequential = false;

    for q in &topic.questions {
        if seen_numbers.contains(&q.number) {
            issues.push(ValidationIssue {
                level: IssueLevel::Error,
                question_number: Some(q.number),
                message: format!("Duplicate question number: Q{}", q.number),
            });
        } else {
            seen_numbers.insert(q.number);
        }

        if q.number != expected_seq_num && !has_non_sequential {
            has_non_sequential = true;
            issues.push(ValidationIssue {
                level: IssueLevel::Warning,
                question_number: Some(q.number),
                message: format!(
                    "Non-sequential question numbering (expected Q{}, found Q{})",
                    expected_seq_num, q.number
                ),
            });
        }
        expected_seq_num += 1;

        if q.question.trim().is_empty() {
            issues.push(ValidationIssue {
                level: IssueLevel::Error,
                question_number: Some(q.number),
                message: format!("Question {} prompt is empty", q.number),
            });
        } else if q.question.trim().len() < 5 {
            issues.push(ValidationIssue {
                level: IssueLevel::Warning,
                question_number: Some(q.number),
                message: format!("Question {} prompt is very short (< 5 characters)", q.number),
            });
        }

        if q.answer.trim().is_empty() {
            issues.push(ValidationIssue {
                level: IssueLevel::Error,
                question_number: Some(q.number),
                message: format!("Question {} answer is missing or empty", q.number),
            });
        }

        if let Some(ref fact) = q.fact {
            if fact.trim().is_empty() {
                issues.push(ValidationIssue {
                    level: IssueLevel::Warning,
                    question_number: Some(q.number),
                    message: format!("Question {} Net Control fact is empty", q.number),
                });
            }
        }
    }

    DeckValidation {
        file_path: file_path.to_string(),
        filename,
        title: topic.title,
        question_count: topic.questions.len(),
        issues,
    }
}

pub fn validate_deck<P: AsRef<Path>>(path: P) -> DeckValidation {
    let p = path.as_ref();
    let path_str = p.to_string_lossy().to_string();
    let filename = p
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(&path_str)
        .to_string();

    match fs::read_to_string(p) {
        Ok(content) => validate_deck_content(&path_str, &content),
        Err(err) => DeckValidation {
            file_path: path_str,
            filename,
            title: "Unreadable File".to_string(),
            question_count: 0,
            issues: vec![ValidationIssue {
                level: IssueLevel::Error,
                question_number: None,
                message: format!("Cannot read file: {}", err),
            }],
        },
    }
}

pub fn scan_and_validate_directory<P: AsRef<Path>>(dir: P) -> Vec<DeckValidation> {
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        let mut paths: Vec<_> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|ext| ext.to_str())
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("md"))
            })
            .collect();
        paths.sort();
        for path in paths {
            results.push(validate_deck(path));
        }
    }
    results
}

pub fn find_all_deck_files() -> Vec<DeckValidation> {
    let mut validations = Vec::new();
    let mut seen_canonical = std::collections::HashSet::new();

    let mut check_and_add = |p: &Path| {
        if p.exists() {
            if let Ok(canonical) = p.canonicalize() {
                if seen_canonical.insert(canonical) {
                    validations.push(validate_deck(p));
                }
            } else if seen_canonical.insert(p.to_path_buf()) {
                validations.push(validate_deck(p));
            }
        }
    };

    // 1. Scan Topic/ directory
    if Path::new("Topic").is_dir() {
        if let Ok(entries) = fs::read_dir("Topic") {
            let mut paths: Vec<_> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .and_then(|ext| ext.to_str())
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("md"))
                })
                .collect();
            paths.sort();
            for path in paths {
                check_and_add(&path);
            }
        }
    }

    // 2. Scan current working directory for Questions*.md or Topic*.md
    if let Ok(entries) = fs::read_dir(".") {
        let mut paths: Vec<_> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                if let Some(file_name) = p.file_name().and_then(|f| f.to_str()) {
                    let lower = file_name.to_lowercase();
                    lower.ends_with(".md")
                        && (lower.starts_with("question") || lower.starts_with("topic"))
                } else {
                    false
                }
            })
            .collect();
        paths.sort();
        for path in paths {
            check_and_add(&path);
        }
    }

    validations
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_markdown() {
        assert_eq!(clean_markdown("**Long John Silver.**"), "Long John Silver.");
        assert_eq!(clean_markdown("\"Land ho\\!\""), "\"Land ho!\"");
        assert_eq!(clean_markdown("1995\\."), "1995.");
        assert_eq!(
            clean_markdown("* **Answer:** **Mark Summers** (or \"Slappy\")."),
            "Answer: Mark Summers (or \"Slappy\")."
        );
    }

    #[test]
    fn test_parse_questions_1() {
        let path = "Topic/Questions-1.md";
        let topic = TriviaTopic::load_from_file(path).expect("Should load Questions-1.md");
        assert_eq!(topic.title, "Talk Like a Pirate Day");
        assert_eq!(topic.questions.len(), 20);

        let q1 = &topic.questions[0];
        assert_eq!(q1.number, 1);
        assert!(q1.question.contains("International Talk Like a Pirate Day"));
        assert_eq!(q1.answer, "Mark Summers (or \"Slappy\").");
        assert!(q1.fact.is_some());
        assert!(q1.fact.as_ref().unwrap().contains("Summers and co-founder"));

        let q4 = &topic.questions[3];
        assert_eq!(q4.number, 4);
        assert_eq!(q4.answer, "Matey (or Shipmate).");
        assert!(q4.fact.is_none());

        let q10 = &topic.questions[9];
        assert_eq!(q10.number, 10);
        assert_eq!(q10.answer, "Port Royal.");

        let q11 = &topic.questions[10];
        assert_eq!(q11.number, 11);
        assert!(q11.question.contains("grog"));
        assert!(q11.answer.contains("Rum diluted with water"));

        let q20 = &topic.questions[19];
        assert_eq!(q20.number, 20);
        assert_eq!(q20.answer, "\"Land ho!\"");
    }

    #[test]
    fn test_parse_questions_merged() {
        let path = "Topic/Questions.md";
        let topic = TriviaTopic::load_from_file(path).expect("Should load Questions.md");
        assert_eq!(topic.title, "Talk Like a Pirate Day");
        assert_eq!(topic.questions.len(), 20);

        assert_eq!(topic.questions[0].number, 1);
        assert_eq!(topic.questions[19].number, 20);
    }

    #[test]
    fn test_parse_questions_2() {
        let path = "Topic/Questions-2.md";
        let topic = TriviaTopic::load_from_file(path).expect("Should load Questions-2.md");
        assert_eq!(topic.title, "Talk Like a Pirate Day");
        assert_eq!(topic.questions.len(), 10);

        let q1 = &topic.questions[0];
        assert_eq!(q1.number, 1);
        assert!(q1.question.contains("grog"));
        assert!(q1.answer.contains("Rum diluted with water"));
        assert!(q1.fact.is_some());

        let q10 = &topic.questions[9];
        assert_eq!(q10.number, 10);
        assert_eq!(q10.answer, "\"Land ho!\"");
    }

    #[test]
    fn test_validate_existing_decks() {
        let val1 = validate_deck("Topic/Questions-1.md");
        assert!(val1.is_valid());
        assert_eq!(val1.question_count, 20);
        assert_eq!(val1.error_count(), 0);

        let val2 = validate_deck("Topic/Questions-2.md");
        assert!(val2.is_valid());
        assert_eq!(val2.question_count, 10);
        assert_eq!(val2.error_count(), 0);
    }

    #[test]
    fn test_validate_deck_errors_and_warnings() {
        // Missing questions
        let empty_val = validate_deck_content("empty.md", "# Test Topic\nSome random text");
        assert!(!empty_val.is_valid());
        assert_eq!(empty_val.error_count(), 1);

        // Missing answer and duplicate question numbers
        let broken = r#"
# Broken Topic
1. Question: What is the speed of light?
Net Control Fact: It is very fast.

1. Question: What is gravity?
Answer: Attraction between masses.
"#;
        let broken_val = validate_deck_content("broken.md", broken);
        assert!(!broken_val.is_valid());
        assert!(broken_val.issues.iter().any(|i| i.message.contains("Duplicate question number")));
        assert!(broken_val.issues.iter().any(|i| i.message.contains("answer is missing or empty")));
    }

    #[test]
    fn test_find_all_deck_files() {
        let decks = find_all_deck_files();
        assert!(!decks.is_empty());
        assert!(decks.iter().any(|d| d.filename == "Questions-1.md"));
        assert!(decks.iter().any(|d| d.filename == "Questions-2.md"));
    }
}

