# mdrvserve サンプル — 競技プログラミングと日本語

このディレクトリは、mdrvserve のレンダリング機能を **競技プログラミング (CP)
のアルゴリズム解説** と **日本語テキスト** で気軽に試せるサンプル集です。

## ファイル一覧

| ファイル       | 扱う話題           | 確認できる機能                                     | 必要なフラグ                                         |
| -------------- | ------------------ | -------------------------------------------------- | ---------------------------------------------------- |
| `markdown.md`  | 二分探索法         | GFM (表・タスクリスト・注釈・引用・コードブロック) | `--with-gfm` (注釈 callout 用)                       |
| `diagrams.md`  | グラフアルゴリズム | D2 / LaTeX / Mermaid / Typst の各ブロック          | `--with-d2 --with-latex --with-mermaid --with-typst` |
| `page.html`    | 計算量チートシート | HTML 配信 (スタイル・インライン SVG・スクリプト)   | `--include-html`                                     |
| `document.typ` | セグメント木       | Typst 組版 (複数ページ・数式・表)                  | `--include-typst`                                    |

## 実行例

1 つのファイルを開く:

```sh
mdrvserve examples/markdown.md --with-gfm --open
mdrvserve examples/diagrams.md --with-d2 --with-latex --with-mermaid --with-typst --open
mdrvserve examples/page.html --include-html --open
mdrvserve examples/document.typ --include-typst --open
```

ディレクトリ全体を開く (サイドバーでファイルを切り替え + 全エンジンを有効化):

```sh
mdrvserve examples/ \
  --include-html --include-typst \
  --with-d2 --with-latex --with-mermaid --with-typst --with-gfm \
  --open
```

> `document.typ` のフリーフロー表示 (Typst の HTML 出力) には、Typst が
> `--features html` 付きでビルドされている必要があります。トグルが無効のときは
> ツールチップで案内されます (ページ表示は常に使えます)。

## 必要な外部コマンド

- `typst` — Typst ファイルと `typst` コードブロックのレンダリング
- `d2` — D2 図のレンダリング

インストールされていない場合は、該当ブロックはソースコードのまま表示され、
起動時に警告が出ます (Markdown・HTML・Mermaid は mdrvserve 内部で処理されます)。
