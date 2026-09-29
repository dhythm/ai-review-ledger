use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{NormalizedReview, Review};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS reviews (
    id TEXT PRIMARY KEY,
    purpose TEXT NOT NULL,
    prompt_summary TEXT NOT NULL,
    answer TEXT NOT NULL,
    decision TEXT NOT NULL CHECK (decision IN ('adopt', 'needs_revision', 'reject')),
    risk TEXT NOT NULL CHECK (risk IN ('low', 'mid', 'high')),
    has_evidence INTEGER NOT NULL CHECK (has_evidence IN (0, 1)),
    memo TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_reviews_updated ON reviews (updated_at DESC);
";

struct Seed {
    id: &'static str,
    purpose: &'static str,
    prompt_summary: &'static str,
    answer: &'static str,
    decision: &'static str,
    risk: &'static str,
    has_evidence: bool,
    memo: Option<&'static str>,
    created_at: &'static str,
    updated_at: &'static str,
}

const SEEDS: &[Seed] = &[
    Seed {
        id: "11111111-1111-4111-8111-111111111111",
        purpose: "配送遅延に関する一次返信",
        prompt_summary: "注文の配送が2日遅れている顧客への一次返信を書いてください。補償は案内せず、調査中であることと次の連絡時期だけ伝えてください。",
        answer: "お問い合わせありがとうございます。ご注文の配送状況を確認したところ、当初のご案内から遅れが生じております。現在、配送業者とともに状況を調査しております。進捗が分かり次第、本日中に改めてご連絡いたします。ご不便をおかけし申し訳ございません。",
        decision: "adopt",
        risk: "low",
        has_evidence: true,
        memo: Some("カスタマーサポート手順書の一次返信方針と一致。補償を案内しない点も確認済み。"),
        created_at: "2026-09-12T01:10:00Z",
        updated_at: "2026-09-12T02:40:00Z",
    },
    Seed {
        id: "22222222-2222-4222-8222-222222222222",
        purpose: "契約書の違約金条項の要約",
        prompt_summary: "一般的なSaaS利用規約を想定し、違約金と中途解約の条項を3点で要約してください。実契約の添付はありません。",
        answer: "1. 中途解約でも、残期間分の利用料を違約金として一括で請求できるのが一般的です。\n2. 損害賠償の上限は、直近12か月に支払った利用料です。\n3. 秘密保持義務違反には、この上限が適用されません。",
        decision: "needs_revision",
        risk: "high",
        has_evidence: false,
        memo: Some("実契約を読んでいない。法務確認の前に顧客へ渡さない。"),
        created_at: "2026-09-18T04:00:00Z",
        updated_at: "2026-09-20T06:15:00Z",
    },
    Seed {
        id: "33333333-3333-4333-8333-333333333333",
        purpose: "有給休暇の申請手順（社内FAQ）",
        prompt_summary: "就業規則では、有給は取得日の5営業日前までに勤怠システムで申請し、上長が承認する、という前提でFAQを1件書いてください。",
        answer: "Q. 有給休暇はどう申請しますか？\nA. 取得希望日の5営業日前までに、勤怠システムから申請してください。上長が承認すると取得が確定します。急な病気などで期限に間に合わない場合は、まず上長へ連絡し、出社後すみやかに同じ手順で申請してください。",
        decision: "adopt",
        risk: "mid",
        has_evidence: true,
        memo: Some("人事が就業規則の該当箇所と照合済み。社内FAQへ転記できる。"),
        created_at: "2026-08-02T00:30:00Z",
        updated_at: "2026-08-02T08:00:00Z",
    },
    Seed {
        id: "44444444-4444-4444-8444-444444444444",
        purpose: "新機能のプレスリリース草案",
        prompt_summary: "来月の分析機能リリースについて、導入社数1,200社、継続率99%を入れてプレスリリースのリード文を書いてください。",
        answer: "株式会社例は、1,200社に導入され継続率99%を達成した分析機能の提供を開始します。本機能により、現場はデータをもとに翌営業日の打ち手を決められます。",
        decision: "reject",
        risk: "mid",
        has_evidence: false,
        memo: Some("導入社数と継続率は未公表のドラフト数値。広報の承認前の数字を含むため不採用。"),
        created_at: "2026-09-01T07:20:00Z",
        updated_at: "2026-09-03T01:05:00Z",
    },
    Seed {
        id: "55555555-5555-4555-8555-555555555555",
        purpose: "障害の顧客向け要約",
        prompt_summary: "3月12日 10:14–11:02 JSTにAPIの応答が遅延した。原因は接続プールの枯渇。影響は請求APIの一部でタイムアウトが約6%。再発防止はプール上限の見直しと監視追加。顧客向けに5文以内で要約してください。",
        answer: "3月12日10時14分から11時02分（日本時間）にかけて、一部のAPIで応答遅延が発生しました。原因はデータベース接続プールの枯渇です。請求APIのリクエストのうち約6%がタイムアウトしました。現在は解消しています。再発防止として、接続数の上限見直しと監視の追加を進めています。",
        decision: "needs_revision",
        risk: "high",
        has_evidence: true,
        memo: Some("一次報告のタイムアウト率は4.8%。6%は概算なので、確定値に直してから送る。"),
        created_at: "2026-03-12T03:40:00Z",
        updated_at: "2026-03-12T09:10:00Z",
    },
    Seed {
        id: "66666666-6666-4666-8666-666666666666",
        purpose: "プルリクエストのレビューコメント",
        prompt_summary: "ユーザー入力の表示名をそのままHTMLへ入れている差分について、レビューコメントを1つ書いてください。指摘はエスケープ漏れに限定してください。",
        answer: "表示名をHTML文字列へ連結しています。入力にマークアップが含まれると、そのまま解釈される可能性があります。テキストとして挿入するか、出力時にエスケープしてください。",
        decision: "adopt",
        risk: "low",
        has_evidence: true,
        memo: Some("該当行を確認済み。指摘範囲も妥当。"),
        created_at: "2026-09-25T06:00:00Z",
        updated_at: "2026-09-25T06:12:00Z",
    },
];

pub fn default_db_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("ledger.db")
}

pub fn init_db(path: &Path) -> Result<Connection, Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.execute_batch(SCHEMA)?;
    seed_if_empty(&conn)?;
    Ok(conn)
}

fn seed_if_empty(conn: &Connection) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM reviews", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let tx = conn.unchecked_transaction()?;
    for seed in SEEDS {
        tx.execute(
            "INSERT INTO reviews (
                id, purpose, prompt_summary, answer, decision, risk, has_evidence, memo, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                seed.id,
                seed.purpose,
                seed.prompt_summary,
                seed.answer,
                seed.decision,
                seed.risk,
                seed.has_evidence as i64,
                seed.memo,
                seed.created_at,
                seed.updated_at,
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub struct ListFilter<'a> {
    pub query: Option<&'a str>,
    pub decision: Option<&'a str>,
    pub risk: Option<&'a str>,
}

pub struct ReviewList {
    pub items: Vec<Review>,
    pub total: i64,
}

pub fn list_reviews(conn: &Connection, filter: ListFilter<'_>) -> rusqlite::Result<ReviewList> {
    let pattern = filter
        .query
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(like_pattern);
    let decision = blank_to_none(filter.decision);
    let risk = blank_to_none(filter.risk);

    let mut stmt = conn.prepare(
        "SELECT id, purpose, prompt_summary, answer, decision, risk, has_evidence, memo, created_at, updated_at
         FROM reviews
         WHERE (?1 IS NULL OR purpose LIKE ?1 ESCAPE '\\'
             OR prompt_summary LIKE ?1 ESCAPE '\\'
             OR answer LIKE ?1 ESCAPE '\\'
             OR IFNULL(memo, '') LIKE ?1 ESCAPE '\\')
           AND (?2 IS NULL OR decision = ?2)
           AND (?3 IS NULL OR risk = ?3)
         ORDER BY updated_at DESC",
    )?;
    let items = stmt
        .query_map(params![pattern, decision, risk], map_review)?
        .collect::<Result<Vec<_>, _>>()?;
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM reviews", [], |row| row.get(0))?;
    Ok(ReviewList { items, total })
}

pub fn get_review(conn: &Connection, id: &str) -> rusqlite::Result<Option<Review>> {
    conn.query_row(
        "SELECT id, purpose, prompt_summary, answer, decision, risk, has_evidence, memo, created_at, updated_at
         FROM reviews WHERE id = ?1",
        params![id],
        map_review,
    )
    .optional()
}

pub fn create_review(conn: &Connection, review: NormalizedReview) -> rusqlite::Result<Review> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    conn.execute(
        "INSERT INTO reviews (
            id, purpose, prompt_summary, answer, decision, risk, has_evidence, memo, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            review.purpose,
            review.prompt_summary,
            review.answer,
            review.decision,
            review.risk,
            review.has_evidence as i64,
            review.memo,
            now,
            now,
        ],
    )?;
    Ok(get_review(conn, &id)?.expect("inserted review"))
}

pub fn update_review(
    conn: &Connection,
    id: &str,
    review: NormalizedReview,
) -> rusqlite::Result<Option<Review>> {
    let now = now_rfc3339();
    let changed = conn.execute(
        "UPDATE reviews SET
            purpose = ?1,
            prompt_summary = ?2,
            answer = ?3,
            decision = ?4,
            risk = ?5,
            has_evidence = ?6,
            memo = ?7,
            updated_at = ?8
         WHERE id = ?9",
        params![
            review.purpose,
            review.prompt_summary,
            review.answer,
            review.decision,
            review.risk,
            review.has_evidence as i64,
            review.memo,
            now,
            id,
        ],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    get_review(conn, id)
}

pub fn delete_review(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    let changed = conn.execute("DELETE FROM reviews WHERE id = ?1", params![id])?;
    Ok(changed > 0)
}

fn map_review(row: &rusqlite::Row<'_>) -> rusqlite::Result<Review> {
    let memo: Option<String> = row.get(7)?;
    Ok(Review {
        id: row.get(0)?,
        purpose: row.get(1)?,
        prompt_summary: row.get(2)?,
        answer: row.get(3)?,
        decision: row.get(4)?,
        risk: row.get(5)?,
        has_evidence: row.get::<_, i64>(6)? != 0,
        memo: memo.filter(|value| !value.trim().is_empty()),
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn blank_to_none(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn like_pattern(query: &str) -> String {
    let escaped = query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}
