import { useEffect, useState, type FormEvent } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { ApiError, createReview, getReview, updateReview } from "../api";
import { Missing } from "../components/Missing";
import { DECISION_OPTIONS, RISK_OPTIONS } from "../labels";
import type { Decision, ReviewInput, Risk } from "../types";
import { useTitle } from "../useTitle";

type FormState = {
  purpose: string;
  promptSummary: string;
  answer: string;
  decision: Decision | "";
  risk: Risk | "";
  hasEvidence: boolean;
  memo: string;
};

type FieldErrors = Partial<Record<keyof FormState, string>>;

const EMPTY: FormState = {
  purpose: "",
  promptSummary: "",
  answer: "",
  decision: "",
  risk: "",
  hasEvidence: false,
  memo: "",
};

export function FormPage({ mode }: { mode: "create" | "edit" }) {
  const { id } = useParams();
  const navigate = useNavigate();
  const [form, setForm] = useState<FormState>(EMPTY);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [status, setStatus] = useState<"ready" | "loading" | "missing" | "error">(
    mode === "edit" ? "loading" : "ready",
  );
  const [saving, setSaving] = useState(false);

  useTitle(mode === "create" ? "レビューを記録" : "レビューを編集");

  useEffect(() => {
    if (mode !== "edit" || !id) return;
    const controller = new AbortController();
    setStatus("loading");
    getReview(id, controller.signal)
      .then((review) => {
        setForm({
          purpose: review.purpose,
          promptSummary: review.promptSummary,
          answer: review.answer,
          decision: review.decision,
          risk: review.risk,
          hasEvidence: review.hasEvidence,
          memo: review.memo ?? "",
        });
        setStatus("ready");
      })
      .catch((err: unknown) => {
        if (err instanceof DOMException && err.name === "AbortError") return;
        if (err instanceof ApiError && err.status === 404) {
          setStatus("missing");
          return;
        }
        setFormError(err instanceof Error ? err.message : "読み込みに失敗しました");
        setStatus("error");
      });
    return () => controller.abort();
  }, [mode, id]);

  function update<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((current) => ({ ...current, [key]: value }));
    setFieldErrors((current) => ({ ...current, [key]: undefined }));
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    const errors = validate(form);
    setFieldErrors(errors);
    if (Object.keys(errors).length > 0) {
      setFormError("未入力の項目があります。");
      return;
    }
    if (form.decision === "" || form.risk === "") return;

    const input: ReviewInput = {
      purpose: form.purpose.trim(),
      promptSummary: form.promptSummary.trim(),
      answer: form.answer.trim(),
      decision: form.decision,
      risk: form.risk,
      hasEvidence: form.hasEvidence,
      memo: form.memo.trim() ? form.memo.trim() : null,
    };

    setSaving(true);
    setFormError(null);
    try {
      const saved =
        mode === "edit" && id ? await updateReview(id, input) : await createReview(input);
      navigate(`/reviews/${saved.id}`);
    } catch (err: unknown) {
      setSaving(false);
      setFormError(err instanceof Error ? err.message : "保存に失敗しました");
    }
  }

  if (status === "loading") return <p className="loading">読み込み中…</p>;
  if (status === "missing") return <Missing />;
  if (status === "error") {
    return (
      <>
        <Link to="/" className="back">
          台帳に戻る
        </Link>
        <p className="banner" role="alert">
          {formError ?? "読み込みに失敗しました"}
        </p>
      </>
    );
  }

  const cancelTo = mode === "edit" && id ? `/reviews/${id}` : "/";

  return (
    <>
      <Link to={cancelTo} className="back">
        {mode === "edit" ? "詳細に戻る" : "台帳に戻る"}
      </Link>
      <div className="form-card">
        <form onSubmit={onSubmit} noValidate>
          <div className="stack">
            <div>
              <h1>{mode === "create" ? "レビューを記録" : "レビューを編集"}</h1>
              <p className="lede">プロンプトの要約とAIの回答を残し、判断とリスクを付けます。</p>
            </div>

            {formError ? (
              <p className="banner" role="alert">
                {formError}
              </p>
            ) : null}

            <label className="field">
              <span className="label-row">
                <span>目的</span>
                <span className="req">必須</span>
              </span>
              <input
                type="text"
                value={form.purpose}
                maxLength={200}
                placeholder="例）顧客向け障害報告の要約"
                aria-invalid={Boolean(fieldErrors.purpose)}
                onChange={(event) => update("purpose", event.target.value)}
              />
              {fieldErrors.purpose ? <span className="field-error">{fieldErrors.purpose}</span> : null}
            </label>

            <label className="field">
              <span className="label-row">
                <span>プロンプト要約</span>
                <span className="req">必須</span>
              </span>
              <textarea
                value={form.promptSummary}
                placeholder="どんな指示をAIに出したか。全文でなくて構いません。"
                aria-invalid={Boolean(fieldErrors.promptSummary)}
                onChange={(event) => update("promptSummary", event.target.value)}
              />
              {fieldErrors.promptSummary ? (
                <span className="field-error">{fieldErrors.promptSummary}</span>
              ) : null}
            </label>

            <label className="field">
              <span className="label-row">
                <span>AI回答</span>
                <span className="req">必須</span>
              </span>
              <textarea
                className="answer"
                value={form.answer}
                placeholder="AIの回答をそのまま貼り付けます。"
                aria-invalid={Boolean(fieldErrors.answer)}
                onChange={(event) => update("answer", event.target.value)}
              />
              {fieldErrors.answer ? <span className="field-error">{fieldErrors.answer}</span> : null}
            </label>

            <fieldset className="field">
              <legend className="label-row">
                <span>判断</span>
                <span className="req">必須</span>
              </legend>
              <div className="segment decision" role="radiogroup" aria-label="判断">
                {DECISION_OPTIONS.map((option) => (
                  <label key={option.value}>
                    <input
                      type="radio"
                      name="decision"
                      value={option.value}
                      checked={form.decision === option.value}
                      onChange={() => update("decision", option.value)}
                    />
                    <span>{option.label}</span>
                  </label>
                ))}
              </div>
              {fieldErrors.decision ? <span className="field-error">{fieldErrors.decision}</span> : null}
            </fieldset>

            <fieldset className="field">
              <legend className="label-row">
                <span>リスク</span>
                <span className="req">必須</span>
              </legend>
              <div className="segment risk" role="radiogroup" aria-label="リスク">
                {RISK_OPTIONS.map((option) => (
                  <label key={option.value}>
                    <input
                      type="radio"
                      name="risk"
                      value={option.value}
                      checked={form.risk === option.value}
                      onChange={() => update("risk", option.value)}
                    />
                    <span>{option.label}</span>
                  </label>
                ))}
              </div>
              {fieldErrors.risk ? <span className="field-error">{fieldErrors.risk}</span> : null}
            </fieldset>

            <label className="toggle">
              <input
                type="checkbox"
                checked={form.hasEvidence}
                onChange={(event) => update("hasEvidence", event.target.checked)}
              />
              <span className="toggle-track" aria-hidden="true">
                <span className="toggle-thumb" />
              </span>
              <span>
                <span className="toggle-title">根拠を確認した</span>
                <span className="toggle-help">
                  参照資料や一次情報に基づいているときにオンにします。
                </span>
              </span>
            </label>

            <label className="field">
              <span className="label-row">
                <span>メモ</span>
                <span className="req">任意</span>
              </span>
              <textarea
                value={form.memo}
                placeholder="判断の理由、確認した資料、残っている作業など。"
                onChange={(event) => update("memo", event.target.value)}
              />
            </label>

            <div className="form-actions">
              <button className="btn btn-primary" type="submit" disabled={saving}>
                {saving ? "保存しています…" : "保存する"}
              </button>
              <Link className="btn btn-secondary" to={cancelTo}>
                キャンセル
              </Link>
            </div>
          </div>
        </form>
      </div>
    </>
  );
}

function validate(form: FormState): FieldErrors {
  const errors: FieldErrors = {};
  if (!form.purpose.trim()) errors.purpose = "目的を入力してください";
  else if ([...form.purpose.trim()].length > 200) errors.purpose = "目的は200文字以内で入力してください";
  if (!form.promptSummary.trim()) errors.promptSummary = "プロンプト要約を入力してください";
  if (!form.answer.trim()) errors.answer = "AI回答を入力してください";
  if (!form.decision) errors.decision = "判断を選択してください";
  if (!form.risk) errors.risk = "リスクを選択してください";
  return errors;
}
