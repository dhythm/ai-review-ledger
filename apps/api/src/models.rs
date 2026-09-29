use serde::{Deserialize, Serialize};

pub const DECISIONS: &[&str] = &["adopt", "needs_revision", "reject"];
pub const RISKS: &[&str] = &["low", "mid", "high"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub id: String,
    pub purpose: String,
    pub prompt_summary: String,
    pub answer: String,
    pub decision: String,
    pub risk: String,
    pub has_evidence: bool,
    pub memo: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewInput {
    pub purpose: String,
    pub prompt_summary: String,
    pub answer: String,
    pub decision: String,
    pub risk: String,
    pub has_evidence: bool,
    pub memo: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NormalizedReview {
    pub purpose: String,
    pub prompt_summary: String,
    pub answer: String,
    pub decision: String,
    pub risk: String,
    pub has_evidence: bool,
    pub memo: Option<String>,
}

pub fn normalize(input: ReviewInput) -> Result<NormalizedReview, String> {
    let purpose = require_text(&input.purpose, "目的", 200)?;
    let prompt_summary = require_text(&input.prompt_summary, "プロンプト要約", 4_000)?;
    let answer = require_text(&input.answer, "AI回答", 20_000)?;
    let decision = one_of(input.decision.trim(), DECISIONS, "判断")?;
    let risk = one_of(input.risk.trim(), RISKS, "リスク")?;
    let memo = optional_text(input.memo.as_deref(), "メモ", 4_000)?;

    Ok(NormalizedReview {
        purpose,
        prompt_summary,
        answer,
        decision,
        risk,
        has_evidence: input.has_evidence,
        memo,
    })
}

fn require_text(value: &str, label: &str, max_chars: usize) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}を入力してください"));
    }
    if trimmed.chars().count() > max_chars {
        return Err(format!("{label}は{max_chars}文字以内で入力してください"));
    }
    Ok(trimmed.to_string())
}

fn optional_text(
    value: Option<&str>,
    label: &str,
    max_chars: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_chars {
        return Err(format!("{label}は{max_chars}文字以内で入力してください"));
    }
    Ok(Some(trimmed.to_string()))
}

fn one_of(value: &str, allowed: &[&str], label: &str) -> Result<String, String> {
    if allowed.contains(&value) {
        Ok(value.to_string())
    } else {
        Err(format!("{label}の値が不正です"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ReviewInput {
        ReviewInput {
            purpose: " 目的 ".into(),
            prompt_summary: "要約".into(),
            answer: "回答".into(),
            decision: "adopt".into(),
            risk: "low".into(),
            has_evidence: true,
            memo: Some("  ".into()),
        }
    }

    #[test]
    fn trims_required_text_and_blank_memo() {
        let normalized = normalize(input()).unwrap();
        assert_eq!(normalized.purpose, "目的");
        assert_eq!(normalized.memo, None);
        assert!(normalized.has_evidence);
    }

    #[test]
    fn rejects_blank_answer_and_unknown_decision() {
        let mut blank = input();
        blank.answer = "   ".into();
        assert_eq!(normalize(blank).unwrap_err(), "AI回答を入力してください");

        let mut bad = input();
        bad.decision = "maybe".into();
        assert_eq!(normalize(bad).unwrap_err(), "判断の値が不正です");
    }
}
