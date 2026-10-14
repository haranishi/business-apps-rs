# business-apps-rs

[English](README.md) · [参加ガイド](CONTRIBUTING.md) · [設計と構成図](docs/design.md)

項目を選んで業務アプリを作り、ブラウザでデータを登録・検索・編集するRust製OSSです。
顧客管理、在庫管理、タスク管理など、小さな業務を手元で試すところから始めます。

現在はローカル開発プレビューです。項目構成は作成後に変更できません。
チームのログイン・権限・承認フローは[今後の段階](docs/roadmap.md)で実装します。

## まず1つ作る

Rust stableを用意し、このリポジトリで実行します。最低対応バージョンは1.88です。

```sh
cargo run -p business-apps-server
```

`http://127.0.0.1:8080` を開き、アプリ名と項目を決めて、レコードを追加してください。
テキスト・数値・チェックの項目に対応し、保存した内容は再起動後も残ります。
データの保存先は `.data/business-apps.sqlite` です。Gitには含めません。
このプレビューは同じ端末からの接続に限定しています。認証・権限がないため、架空のデータで試してください。

## 設計を理解して参加する

[設計書](docs/design.md)でRust API・ドメイン・SQLiteの境界と、機能を追加する手順を説明しています。
[UI設計](docs/ui.md)には画面遷移と配置の図、[API仕様](docs/api.md)にはリクエスト例を用意しました。
文書は英語を基本とし、相談・貢献は日本語でも受け付けます。

初めての方は[CONTRIBUTING](CONTRIBUTING.md)の対象ファイルと確認方法から始めてください。
不具合の再現、説明の補足、翻訳、操作しやすさの改善も歓迎します。
大きなAPI追加や保存形式の変更は、実装前にIssueで相談します。

## テストと開発手順

```sh
scripts/check.sh
npm ci
npx playwright install chromium
npm run test:e2e
cargo deny check licenses bans sources
```

開発時はRustに加えてNode.js 22以上、Python 3.10以上を使います。
[テスト環境](docs/testing.md)は一時DBを使い、更新競合・データ分離・再起動・画面操作を検証します。
[開発ハーネス](HARNESS.md)と[専用スキル](.agents/skills/business-apps/SKILL.md)で、変更ごとの受け入れ条件と検証をそろえます。
速度はreleaseビルドとベンチマークで測り、条件と結果を併記します。

## ライセンス

オリジナルのコード・文書・図は[Apache-2.0](LICENSE)です。[依存物と出典の方針](docs/licenses.md)も確認してください。
サイボウズとは独立した実装です。kintoneはサイボウズ株式会社の登録商標であり、API互換には対応していません。
