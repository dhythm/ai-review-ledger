export type Decision = "adopt" | "needs_revision" | "reject";
export type Risk = "low" | "mid" | "high";

export type Review = {
  id: string;
  purpose: string;
  promptSummary: string;
  answer: string;
  decision: Decision;
  risk: Risk;
  hasEvidence: boolean;
  memo: string | null;
  createdAt: string;
  updatedAt: string;
};

export type ReviewInput = {
  purpose: string;
  promptSummary: string;
  answer: string;
  decision: Decision;
  risk: Risk;
  hasEvidence: boolean;
  memo: string | null;
};

export type ReviewList = {
  items: Review[];
  total: number;
};

export type ReviewFilters = {
  q: string;
  decision: Decision | "";
  risk: Risk | "";
};
