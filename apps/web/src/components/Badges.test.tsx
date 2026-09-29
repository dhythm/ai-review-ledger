import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DecisionBadge, EvidenceChip, RiskBadge } from "./Badges";

describe("DecisionBadge", () => {
  it("shows the Japanese label for the decision code", () => {
    render(<DecisionBadge value="needs_revision" />);
    const badge = screen.getByText("要修正");
    expect(badge).toHaveClass("badge", "badge-needs_revision");
  });
});

describe("RiskBadge", () => {
  it("shows the risk label with the リスク prefix", () => {
    render(<RiskBadge value="high" />);
    const badge = screen.getByText("リスク 高");
    expect(badge).toHaveClass("badge-risk", "badge-risk-high");
  });
});

describe("EvidenceChip", () => {
  it("switches between 根拠あり and 根拠なし", () => {
    const { rerender } = render(<EvidenceChip hasEvidence />);
    expect(screen.getByText("根拠あり")).toBeInTheDocument();

    rerender(<EvidenceChip hasEvidence={false} />);
    expect(screen.getByText("根拠なし")).toBeInTheDocument();
    expect(screen.queryByText("根拠あり")).not.toBeInTheDocument();
  });
});
