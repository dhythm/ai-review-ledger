import type { Review, ReviewFilters, ReviewInput, ReviewList } from "./types";

export class ApiError extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

async function readError(response: Response): Promise<ApiError> {
  try {
    const data: unknown = await response.json();
    if (
      data &&
      typeof data === "object" &&
      "error" in data &&
      typeof data.error === "string"
    ) {
      return new ApiError(response.status, data.error);
    }
  } catch {
    // The body was empty or not JSON.
  }
  return new ApiError(response.status, `リクエストに失敗しました (${response.status})`);
}

export async function fetchReviews(
  filters: ReviewFilters,
  signal?: AbortSignal,
): Promise<ReviewList> {
  const params = new URLSearchParams();
  if (filters.q.trim()) params.set("q", filters.q.trim());
  if (filters.decision) params.set("decision", filters.decision);
  if (filters.risk) params.set("risk", filters.risk);
  const query = params.toString();
  const response = await fetch(`/api/reviews${query ? `?${query}` : ""}`, { signal });
  if (!response.ok) throw await readError(response);
  return response.json() as Promise<ReviewList>;
}

export async function getReview(id: string, signal?: AbortSignal): Promise<Review> {
  const response = await fetch(`/api/reviews/${encodeURIComponent(id)}`, { signal });
  if (!response.ok) throw await readError(response);
  return response.json() as Promise<Review>;
}

export async function createReview(input: ReviewInput): Promise<Review> {
  const response = await fetch("/api/reviews", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
  if (!response.ok) throw await readError(response);
  return response.json() as Promise<Review>;
}

export async function updateReview(id: string, input: ReviewInput): Promise<Review> {
  const response = await fetch(`/api/reviews/${encodeURIComponent(id)}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
  if (!response.ok) throw await readError(response);
  return response.json() as Promise<Review>;
}

export async function deleteReview(id: string): Promise<void> {
  const response = await fetch(`/api/reviews/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });
  if (!response.ok) throw await readError(response);
}
