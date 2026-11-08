<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/flopha-icon-dark.svg">
  <img src="assets/brand/flopha-icon.svg" alt="" width="64">
</picture>

# flopha

[English](README.md) · [한국어](README.ko.md) · [日本語](README.ja.md) · [简体中文](README.zh-CN.md)

**マージしたコミットから、次のリリースへ。**

Gitで管理するプロジェクトのリリースを担うメンテナー向けのツールです。flophaはタグとコミットメッセージから次のセマンティックバージョンを算出し、マニフェストの更新、タグのプッシュ、変更履歴を添えたGitHub Releaseの作成まで実行できます。プロジェクトの言語を問わず、ターミナルでもCIでも同じバイナリを使えます。CLIの実行にNode.jsやPythonのランタイムは不要です。

```console
$ git log --oneline v1.2.3..
3f2c1a9 fix: handle empty input
a1b2c3d feat: add release notes export

$ flopha next-version --auto
v1.3.0
```

## まずは次のバージョンを確認

macOS（Intel、Apple silicon）とLinux x86_64向けのビルド済みバイナリを提供しています。以下のインストーラーはGitHub Releasesからバイナリをダウンロードしてインストールします。

```sh
curl -fsSL https://raw.githubusercontent.com/sjquant/flopha/main/scripts/install.sh | sh
```

標準のインストール先は`~/.flopha/bin`です。`flopha`が見つからない場合は[PATHの設定](https://flopha.solaqua.dev/ja/docs/installation)を確認してください。

インストールを確認したら、該当するバージョンタグがあるGitリポジトリ内で次のバージョンを算出します。標準のタグ形式は`v1.2.3`です。該当するタグがまだなければ、下の`--auto`のコマンドの代わりに`flopha next-version --initial-version 0.1.0`で最初のバージョン`v0.1.0`を確認してください。

```sh
flopha --version
flopha next-version --auto
```

`v1.2.3`以降に上の例のコミットがあれば、結果は`v1.3.0`です。`feat:`はminor、互換性を壊す変更（`feat!:`または`BREAKING CHANGE`）はmajor、それ以外はpatchを上げます。

算出だけではタグを作成・プッシュしません。まず`origin`からのfetchを試み、失敗した場合はローカルの履歴を使います。CIでは全履歴とタグを取得してください。

その他のプラットフォームでは、Rust 1.85以降で[ソースからビルド](https://flopha.solaqua.dev/ja/docs/installation#build-from-source)できます。

タグだけでリリースする場合は、結果を確認してからタグを作成し、`origin`にプッシュします。リモートへの書き込み権限が必要です。

```sh
flopha next-version --auto --create --push
```

最初のタグには、`--auto`の代わりに`--initial-version 0.1.0`を使ってください。

## マニフェストのバージョンもまとめて更新

ファイルにもバージョンを記録している場合は、`flopha.toml`を作成してコミットします。以下は既存のnpmの`package.json`を更新する例です。Cargo、Pythonのマニフェストや、正規表現によるファイルの更新にも対応しています。

```toml
[release]
create = true

[[manifest]]
path = "package.json"
type = "npm"
```

```sh
flopha release --dry-run
flopha release
```

`--dry-run`はマニフェストの変更、コミット、タグ作成、プッシュ、公開を行わず、実行計画を表示します。2つ目のコマンドはこれらを実行します。ワーキングツリーに未コミットの変更がないことを確認し、プッシュ可能な`origin`を設定したブランチで実行してください。GitHub Releaseの作成には`GH_TOKEN`、`GITHUB_TOKEN`、またはローカルでの`gh auth login`による認証が必要です。バージョンタグがない場合は、両方のコマンドに`--initial-version 0.1.0`を追加します。

HEADにすでにタグがある場合は、再度バージョンを上げる代わりに、未完了のプッシュやGitHub Releaseの作成を試みます。新しいコミットがなければ、別のバージョンは作りません。公開前にレビューするには、`flopha release --prepare`でマニフェストだけを更新し、[バージョンPRのワークフロー](https://flopha.solaqua.dev/ja/docs/release-workflows)に進んでください。

## GitHub Actionsからタグとリリースを公開

タグをバージョンの基準にするプロジェクトでは、ワークフローのジョブに以下の権限とステップを追加できます。タグをプッシュし、変更履歴付きのGitHub Releaseを作成します。マニフェストファイルは**更新しません**。起点となるバージョンタグがリポジトリに必要です。

```yaml
permissions:
  contents: write

steps:
  - uses: actions/checkout@v4
    with:
      fetch-depth: 0
  - uses: sjquant/flopha@v1
    with:
      create-release: true
      changelog: true
```

入出力は[Actionリファレンス（英語）](docs/github-action.md)、CIでマニフェストも更新する方法は[リリースワークフロー](https://flopha.solaqua.dev/ja/docs/release-workflows)を参照してください。

## いつものリリース規約に合わせる

`mobile@1.2.3`のようなタグ、リリースブランチ、`rc`のプレリリースは[バージョンパターン](https://flopha.solaqua.dev/ja/docs/version-patterns)で設定できます。独自のコミット規約もカスタムルールで扱えます。詳しいオプションは[コマンドと`flopha.toml`のリファレンス](https://flopha.solaqua.dev/ja/docs/command-reference)を参照してください。

## コントリビューション

[CONTRIBUTING.md（英語）](CONTRIBUTING.md)を参照してください。

## ライセンス

[MIT](LICENSE)
