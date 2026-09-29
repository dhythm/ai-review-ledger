import { Link } from "react-router-dom";

export function Missing({
  title = "レビューが見つかりません",
  body = "削除されたか、URLが違います。",
}: {
  title?: string;
  body?: string;
}) {
  return (
    <div className="missing">
      <h1>{title}</h1>
      <p>{body}</p>
      <Link to="/" className="btn btn-primary">
        台帳に戻る
      </Link>
    </div>
  );
}
