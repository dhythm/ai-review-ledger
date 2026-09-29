import { describe, expect, it } from "vitest";
import { decisionLabel, formatDate, riskLabel } from "./labels";

describe("decisionLabel", () => {
  it("maps stored decision codes to the labels shown in the UI", () => {
    expect(decisionLabel("adopt")).toBe("採用");
    expect(decisionLabel("needs_revision")).toBe("要修正");
    expect(decisionLabel("reject")).toBe("不採用");
  });
});

describe("riskLabel", () => {
  it("maps stored risk codes to the labels shown in the UI", () => {
    expect(riskLabel("low")).toBe("低");
    expect(riskLabel("mid")).toBe("中");
    expect(riskLabel("high")).toBe("高");
  });
});

describe("formatDate", () => {
  it("formats an ISO timestamp in Asia/Tokyo", () => {
    // TZ is fixed in apps/web/vite.config.ts so this does not follow the machine timezone.
    expect(formatDate("2026-09-12T01:10:00Z")).toBe("2026年9月12日 10:10");
  });

  it("returns the original string when the timestamp cannot be parsed", () => {
    expect(formatDate("not-a-date")).toBe("not-a-date");
  });
});
