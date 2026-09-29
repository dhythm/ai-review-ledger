import type { ReactNode } from "react";
import { Link } from "react-router-dom";

export function Layout({ children }: { children: ReactNode }) {
  return (
    <>
      <header className="topbar">
        <Link to="/" className="brand">
          <svg className="brand-mark" viewBox="0 0 32 32" aria-hidden="true">
            <rect width="32" height="32" rx="8" fill="currentColor" />
            <rect x="8" y="8" width="16" height="2.2" rx="1" fill="white" />
            <rect x="8" y="15" width="16" height="2.2" rx="1" fill="white" />
            <rect x="8" y="22" width="10" height="2.2" rx="1" fill="white" />
          </svg>
          <span className="brand-name">AI回答レビュー台帳</span>
        </Link>
        <Link to="/reviews/new" className="btn btn-primary">
          レビューを記録
        </Link>
      </header>
      <main className="page">{children}</main>
    </>
  );
}
