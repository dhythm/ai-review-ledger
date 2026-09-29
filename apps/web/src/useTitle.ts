import { useEffect } from "react";

export function useTitle(title: string) {
  useEffect(() => {
    document.title = title ? `${title} · AI回答レビュー台帳` : "AI回答レビュー台帳";
  }, [title]);
}
