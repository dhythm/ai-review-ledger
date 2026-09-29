# AI回答レビュー台帳

AIの回答を、採用・要修正・不採用の判断、リスク、根拠の有無とともに残すローカル向けの台帳です。検索と絞り込みで、あとから判断の経緯をたどれます。

外部のAIサービスは呼び出しません。データはこのマシンの SQLite に保存されます。認証はありません。

## 前提

- Rust stable と Cargo（1.85 以上。ロックされた依存に edition 2024 のクレートが含まれる）
- Node.js 24.15 以上
- pnpm 10（ルート `package.json` の `packageManager`）

pnpm が無いときは Corepack で入れます。

```bash
corepack enable
corepack install
```

## 構成

JavaScript は pnpm workspace、Rust は Cargo workspace です。アプリはどちらも `apps/` にあります。Nx や Turbo は使いません。

```
.
├── apps/
│   ├── api/                 Rust (Axum) + SQLite。Cargo workspace のメンバー
│   └── web/                 React + TypeScript (Vite)。pnpm workspace のメンバー
├── Cargo.toml               Cargo workspace（members: apps/api）
├── package.json             共通スクリプト（dev / test / build）
├── pnpm-workspace.yaml      pnpm workspace（apps/web）
└── pnpm-lock.yaml
```

依存のインストールはリポジトリ直下で一度だけ行います。`apps/web` で `npm install` はしないでください。ロックファイルは `pnpm-lock.yaml` です。

```bash
pnpm install
```

## 起動

API と Web を、別々のターミナルで起動します。どちらもリポジトリ直下から実行できます。

### 1. API

```bash
pnpm dev:api
```

これは `cargo run -p ai-review-ledger-api` と同じです。起動すると `http://127.0.0.1:8080` で待ち受けます。初回はサンプルのレビューが数件入り、一覧が空になりません。

データベースファイルは `apps/api/data/ledger.db` です。場所は起動時のカレントディレクトリではなく、API クレートの位置から決まります。

### 2. Web

```bash
pnpm dev:web
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

## テスト

フロントエンドは Vitest と Testing Library、API は `cargo test` です。期待する振る舞いのテストを先に書き、失敗を確認してから実装し、通ったあとに整えます（t-wada の TDD）。

リポジトリ直下で実行します。

```bash
pnpm test            # Web のあと API
pnpm test:web        # apps/web の Vitest
pnpm test:web:watch  # Web を監視して繰り返す
pnpm test:api        # cargo test --workspace --locked
```

`pnpm test:api` はカレントディレクトリが `apps/api` である必要はありません。ルートで `cargo test --workspace --locked` を直接実行しても同じテストです。

Web のテストは対象の隣に置きます（例: `apps/web/src/labels.test.ts`、`apps/web/src/components/Badges.test.tsx`）。日付表示のテストは `Asia/Tokyo` 固定です（`apps/web/vite.config.ts` の `test.env.TZ`）。API の HTTP テストは `apps/api/tests/` にあります。入力の正規化の単体テストは `apps/api/src/models.rs` の `#[cfg(test)]` にあります。

`main` への push と pull request では、GitHub Actions が `pnpm test:web` と `pnpm test:api` を実行します。

本番ビルドの確認は次です。型チェックと Vite のビルドを行います。

```bash
pnpm build:web
```

台帳をサンプル投入直後の状態に戻すには、API を止めて `apps/api/data/ledger.db` を削除し、API を再起動します。件数が 0 のときだけサンプルが入ります。
