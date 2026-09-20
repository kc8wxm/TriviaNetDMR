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
}
