import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { ApiError, deleteReview, getReview } from "../api";
import { DecisionBadge, EvidenceChip, RiskBadge } from "../components/Badges";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { Missing } from "../components/Missing";
import { decisionLabel, formatDate, riskLabel } from "../labels";
import type { Review } from "../types";
import { useTitle } from "../useTitle";

export function DetailPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const [review, setReview] = useState<Review | null>(null);
  const [status, setStatus] = useState<"loading" | "ready" | "missing" | "error">("loading");
  const [error, setError] = useState<string | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [deleting, setDeleting] = useState(false);

  useTitle(review?.purpose ?? (status === "loading" ? "読み込み中" : "レビュー"));

  useEffect(() => {
    if (!id) return;
    const controller = new AbortController();
    setStatus("loading");
    getReview(id, controller.signal)
      .then((next) => {
        setReview(next);
        setStatus("ready");
      })
      .catch((err: unknown) => {
        if (err instanceof DOMException && err.name === "AbortError") return;
        if (err instanceof ApiError && err.status === 404) {
          setStatus("missing");
          return;
        }
        setError(err instanceof Error ? err.message : "読み込みに失敗しました");
        setStatus("error");
      });
    return () => controller.abort();
  }, [id]);

  async function onDelete() {
    if (!id) return;
    setDeleting(true);
    try {
      await deleteReview(id);
      navigate("/");
    } catch (err: unknown) {
      setDeleting(false);
      setConfirmOpen(false);
      setError(err instanceof Error ? err.message : "削除に失敗しました");
    }
  }

  if (status === "loading") return <p className="loading">読み込み中…</p>;
  if (status === "missing" || !review) {
    if (status === "error") {
      return (
        <p className="banner" role="alert">
          {error}
        </p>
      );
    }
    return <Missing />;
  }

  return (
    <>
      <Link to="/" className="back">
        台帳に戻る
      </Link>
      {error ? (
        <p className="banner" role="alert">
          {error}
        </p>
      ) : null}
      <article className={`detail risk-${review.risk}`}>
        <header className="detail-head">
          <div className="badge-row">
            <DecisionBadge value={review.decision} />
            <RiskBadge value={review.risk} />
            <EvidenceChip hasEvidence={review.hasEvidence} />
          </div>
          <h1>{review.purpose}</h1>
          <dl className="meta-grid">
            <div>
              <dt className="meta-label">判断</dt>
              <dd>{decisionLabel(review.decision)}</dd>
            </div>
            <div>
              <dt className="meta-label">リスク</dt>
              <dd>{riskLabel(review.risk)}</dd>
            </div>
            <div>
              <dt className="meta-label">根拠</dt>
              <dd>{review.hasEvidence ? "あり" : "なし"}</dd>
            </div>
            <div>
              <dt className="meta-label">更新</dt>
              <dd>{formatDate(review.updatedAt)}</dd>
            </div>
            <div>
              <dt className="meta-label">作成</dt>
              <dd>{formatDate(review.createdAt)}</dd>
            </div>
          </dl>
        </header>

        <section className="detail-section">
          <h2>プロンプト要約</h2>
          <p className="prose">{review.promptSummary}</p>
        </section>
        <section className="detail-section">
          <h2>AI回答</h2>
          <p className="prose">{review.answer}</p>
        </section>
        <section className="detail-section">
          <h2>メモ</h2>
          {review.memo ? <p className="prose">{review.memo}</p> : <p className="quiet">メモはありません</p>}
        </section>

        <div className="detail-actions">
          <Link to={`/reviews/${review.id}/edit`} className="btn btn-primary">
            編集
          </Link>
          <button type="button" className="btn btn-secondary" onClick={() => setConfirmOpen(true)}>
            削除
          </button>
        </div>
      </article>

      <ConfirmDialog
        open={confirmOpen}
        title="レビューを削除"
        body={`「${review.purpose}」を台帳から削除します。この操作は取り消せません。`}
        confirmLabel="削除する"
        busy={deleting}
        onConfirm={onDelete}
        onClose={() => setConfirmOpen(false)}
      />
    </>
  );
}
