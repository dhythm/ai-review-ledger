import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { ApiError, fetchReviews } from "../api";
import { DecisionBadge, EvidenceChip, RiskBadge } from "../components/Badges";
import { DECISION_OPTIONS, formatDate, RISK_OPTIONS } from "../labels";
import type { Decision, ReviewList, Risk } from "../types";
import { useTitle } from "../useTitle";

function isDecision(value: string): value is Decision {
  return DECISION_OPTIONS.some((option) => option.value === value);
}

function isRisk(value: string): value is Risk {
  return RISK_OPTIONS.some((option) => option.value === value);
}

export function ListPage() {
  const [params, setParams] = useSearchParams();
  const q = params.get("q") ?? "";
  const decisionParam = params.get("decision") ?? "";
  const riskParam = params.get("risk") ?? "";
  const decision = isDecision(decisionParam) ? decisionParam : "";
  const risk = isRisk(riskParam) ? riskParam : "";
  const [qInput, setQInput] = useState(q);
  const [data, setData] = useState<ReviewList | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useTitle("台帳");

  useEffect(() => {
    setQInput(q);
  }, [q]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setParams(
        (current) => {
          const currentQuery = current.get("q") ?? "";
          if (qInput === currentQuery) return current;
          const next = new URLSearchParams(current);
          if (qInput.trim()) next.set("q", qInput);
          else next.delete("q");
          return next;
        },
        { replace: true },
      );
    }, 200);
    return () => window.clearTimeout(timer);
  }, [qInput, setParams]);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    fetchReviews({ q, decision, risk }, controller.signal)
      .then((next) => {
        setData(next);
        setError(null);
      })
      .catch((err: unknown) => {
        if (err instanceof DOMException && err.name === "AbortError") return;
        setError(err instanceof ApiError ? err.message : "台帳を読み込めませんでした。APIが起動しているか確認してください。");
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [q, decision, risk]);

  function updateFilter(key: "decision" | "risk", value: string) {
    const next = new URLSearchParams(params);
    if (value) next.set(key, value);
    else next.delete(key);
    setParams(next, { replace: true });
  }

  function clearFilters() {
    setQInput("");
    setParams(new URLSearchParams(), { replace: true });
  }

  const filtered = Boolean(q || decision || risk);
  const items = data?.items ?? [];
  const total = data?.total ?? 0;

  return (
    <>
      <div className="page-head">
        <div>
          <h1>レビュー</h1>
          <p className="lede">AIの回答と、採用・修正・不採用の判断を残します。</p>
        </div>
      </div>

      <div className="filters">
        <label className="filter-field">
          <span>検索</span>
          <input
            type="search"
            value={qInput}
            placeholder="目的、プロンプト、回答、メモ"
            onChange={(event) => setQInput(event.target.value)}
          />
        </label>
        <label className="filter-field">
          <span>判断</span>
          <select value={decision} onChange={(event) => updateFilter("decision", event.target.value)}>
            <option value="">すべて</option>
            {DECISION_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label className="filter-field">
          <span>リスク</span>
          <select value={risk} onChange={(event) => updateFilter("risk", event.target.value)}>
            <option value="">すべて</option>
            {RISK_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="list-tools">
        <span>
          {data
            ? total === items.length
              ? `${items.length}件`
              : `${total}件中 ${items.length}件`
            : loading
              ? "読み込み中…"
              : ""}
        </span>
        {filtered ? (
          <button type="button" className="btn-text" onClick={clearFilters}>
            条件をクリア
          </button>
        ) : null}
      </div>

      {error ? (
        <p className="banner" role="alert">
          {error}
        </p>
      ) : null}

      {data && total === 0 ? (
        <div className="empty">
          <h2>レビューはまだありません</h2>
          <p>AIの回答と、その採用判断を最初の1件から残していきましょう。</p>
          <Link to="/reviews/new" className="btn btn-primary">
            最初のレビューを記録
          </Link>
        </div>
      ) : null}

      {data && total > 0 && items.length === 0 ? (
        <div className="empty">
          <h2>該当するレビューがありません</h2>
          <p>条件を変えるか、検索語を短くしてみてください。</p>
          <button type="button" className="btn btn-secondary" onClick={clearFilters}>
            条件をクリア
          </button>
        </div>
      ) : null}

      {items.length > 0 ? (
        <div className="cards">
          {items.map((review) => (
            <Link key={review.id} to={`/reviews/${review.id}`} className={`card risk-${review.risk}`}>
              <h2 className="card-purpose">{review.purpose}</h2>
              <div className="card-meta">
                <DecisionBadge value={review.decision} />
                <RiskBadge value={review.risk} />
                {review.hasEvidence ? <EvidenceChip hasEvidence /> : null}
                <span className="card-date">更新 {formatDate(review.updatedAt)}</span>
              </div>
            </Link>
          ))}
        </div>
      ) : null}

      {data && total > 0 ? (
        <p className="note">記録はこの端末のSQLiteに保存されます。外部のAIサービスへは送信しません。</p>
      ) : null}
    </>
  );
}
