import { decisionLabel, riskLabel } from "../labels";
import type { Decision, Risk } from "../types";

export function DecisionBadge({ value }: { value: Decision }) {
  return <span className={`badge badge-${value}`}>{decisionLabel(value)}</span>;
}

export function RiskBadge({ value }: { value: Risk }) {
  return <span className={`badge badge-risk badge-risk-${value}`}>リスク {riskLabel(value)}</span>;
}

export function EvidenceChip({ hasEvidence }: { hasEvidence: boolean }) {
  return <span className="chip">{hasEvidence ? "根拠あり" : "根拠なし"}</span>;
}
