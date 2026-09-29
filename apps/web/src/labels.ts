import type { Decision, Risk } from "./types";

export const DECISION_OPTIONS: { value: Decision; label: string }[] = [
  { value: "adopt", label: "採用" },
  { value: "needs_revision", label: "要修正" },
  { value: "reject", label: "不採用" },
];

export const RISK_OPTIONS: { value: Risk; label: string }[] = [
  { value: "low", label: "低" },
  { value: "mid", label: "中" },
  { value: "high", label: "高" },
];

export function decisionLabel(value: Decision): string {
  return DECISION_OPTIONS.find((option) => option.value === value)?.label ?? value;
}

export function riskLabel(value: Risk): string {
  return RISK_OPTIONS.find((option) => option.value === value)?.label ?? value;
}

export function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return new Intl.DateTimeFormat("ja-JP", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}
