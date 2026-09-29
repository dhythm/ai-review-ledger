# AI回答レビュー台帳

AIの回答を、採用・要修正・不採用の判断、リスク、根拠の有無とともに残すローカル向けの台帳です。検索と絞り込みで、あとから判断の経緯をたどれます。

外部のAIサービスは呼び出しません。データはこのマシンの SQLite に保存されます。認証はありません。

## 前提

- Rust（stable）と Cargo
- Node.js 20 以上と npm

## 起動

API と Web を、別々のターミナルで起動します。

### 1. API

```bash
cd apps/api
cargo run
```

起動すると `http://127.0.0.1:8080` で待ち受けます。初回はサンプルのレビューが数件入り、一覧が空になりません。

データベースファイルは `apps/api/data/ledger.db` です。場所は起動時のカレントディレクトリではなく、API クレートの位置から決まります。

### 2. Web

```bash
cd apps/web
npm install
npm run dev
```

ブラウザで `http://127.0.0.1:5173` を開きます。開発サーバーは `/api` を API へ転送します。

## 画面

| パス | 内容 |
| --- | --- |
| `/` | 一覧。検索と、判断・リスクの絞り込み。カードに目的、判断、リスク、更新日 |
| `/reviews/new` | 新規記録。保存後は詳細へ |
| `/reviews/:id` | 本文とメタ情報。編集リンクと、確認ダイアログからの削除 |
| `/reviews/:id/edit` | 新規と同じフォーム |

必須項目は、プロンプト要約、AI回答、目的、リスク、判断です。根拠の有無は切り替え、メモは任意です。

## 構成

```
.
├── apps/
│   ├── api/    Rust (Axum) + SQLite
│   └── web/    React + TypeScript (Vite)
├── Cargo.toml  Rust workspace
└── README.md
```

## API

| メソッド | パス | 説明 |
| --- | --- | --- |
| `GET` | `/api/health` | 死活 |
| `GET` | `/api/reviews` | 一覧。`q` `decision` `risk` |
| `POST` | `/api/reviews` | 作成 |
| `GET` | `/api/reviews/:id` | 1件 |
| `PUT` | `/api/reviews/:id` | 更新 |
| `DELETE` | `/api/reviews/:id` | 削除 |

判断は `adopt` / `needs_revision` / `reject`、リスクは `low` / `mid` / `high` です。

## 環境変数

| 名前 | 説明 | 既定 |
| --- | --- | --- |
| `LEDGER_DB` | SQLite ファイルのパス | `apps/api/data/ledger.db` |
| `PORT` | API のポート | `8080` |

## 開発用の確認

```bash
cargo test
cd apps/web && npm run build
```

台帳をサンプル投入直後の状態に戻すには、API を止めて `apps/api/data/ledger.db` を削除し、API を再起動します。件数が 0 のときだけサンプルが入ります。
