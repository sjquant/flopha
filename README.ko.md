<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/flopha-icon-dark.svg">
  <img src="assets/brand/flopha-icon.svg" alt="" width="64">
</picture>

# flopha

[English](README.md) · [한국어](README.ko.md) · [日本語](README.ja.md) · [简体中文](README.zh-CN.md)

**머지한 커밋을 다음 릴리스로.**

Git 기반 프로젝트를 릴리스하는 메인테이너를 위한 도구입니다. flopha는 태그와 커밋 메시지로 다음 시맨틱 버전을 계산하고, 매니페스트 버전 반영부터 태그 푸시, 변경 내역을 담은 GitHub Release 생성까지 이어서 처리할 수 있습니다. 프로젝트 언어에 상관없이 터미널과 CI에서 같은 바이너리를 사용합니다. CLI 실행에 Node.js나 Python 런타임은 필요하지 않습니다.

```console
$ git log --oneline v1.2.3..
3f2c1a9 fix: handle empty input
a1b2c3d feat: add release notes export

$ flopha next-version --auto
v1.3.0
```

## 다음 버전 미리 계산하기

macOS(Intel, Apple silicon)와 Linux x86_64용 바이너리를 제공합니다. 아래 설치 스크립트는 GitHub Releases에서 바이너리를 내려받아 설치합니다.

```sh
curl -fsSL https://raw.githubusercontent.com/sjquant/flopha/main/scripts/install.sh | sh
```

기본 설치 경로는 `~/.flopha/bin`입니다. `flopha` 명령을 찾지 못하면 [PATH 설정](https://flopha.solaqua.dev/ko/docs/installation)을 확인하세요.

설치를 확인한 뒤, 버전에 해당하는 태그가 있는 Git 저장소에서 다음 버전을 계산합니다. 기본 태그 형식은 `v1.2.3`입니다. 해당하는 태그가 아직 없다면 아래의 `--auto` 명령 대신 `flopha next-version --initial-version 0.1.0`으로 첫 버전 `v0.1.0`을 확인하세요.

```sh
flopha --version
flopha next-version --auto
```

`v1.2.3` 이후에 위 예시의 커밋이 있다면 `v1.3.0`이 나옵니다. `feat:`는 minor, 호환성을 깨는 변경(`feat!:` 또는 `BREAKING CHANGE`)은 major, 나머지는 patch를 올립니다.

계산만으로 태그를 만들거나 푸시하지는 않습니다. 먼저 `origin`에서 fetch를 시도하며, 실패하면 로컬 이력을 사용합니다. CI에서는 전체 커밋 이력과 태그를 받아 두세요.

다른 플랫폼에서는 Rust 1.85 이상으로 [소스에서 빌드](https://flopha.solaqua.dev/ko/docs/installation#build-from-source)할 수 있습니다.

태그만으로 릴리스한다면, 결과를 확인한 뒤 태그를 만들고 `origin`에 푸시합니다. 원격 저장소 쓰기 권한이 필요합니다.

```sh
flopha next-version --auto --create --push
```

첫 태그라면 `--auto` 대신 `--initial-version 0.1.0`을 사용하세요.

## 매니페스트 버전도 함께 갱신하기

파일에도 버전을 기록한다면 `flopha.toml`을 작성해 커밋하세요. 아래는 기존 npm `package.json`을 갱신하는 예입니다. Cargo, Python 매니페스트와 정규식 기반 파일 수정도 지원합니다.

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

`--dry-run`은 매니페스트 수정, 커밋, 태그 생성, 푸시, 게시 없이 실행 계획을 보여 줍니다. 두 번째 명령은 이 단계를 실제로 수행합니다. 작업 트리에 커밋하지 않은 변경이 없는지 확인하고, 푸시할 수 있는 `origin`이 설정된 브랜치에서 실행하세요. GitHub Release 생성에는 `GH_TOKEN`, `GITHUB_TOKEN` 또는 로컬의 `gh auth login` 인증이 필요합니다. 버전 태그가 없다면 두 명령 모두에 `--initial-version 0.1.0`을 추가하세요.

HEAD에 이미 태그가 있으면 새 버전을 올리는 대신 미완료 푸시나 GitHub Release 생성을 마저 시도합니다. 새 커밋이 없으면 다른 버전을 만들지 않습니다. 게시 전에 검토받으려면 `flopha release --prepare`로 매니페스트만 수정한 뒤 [버전 PR 워크플로](https://flopha.solaqua.dev/ko/docs/release-workflows)를 따르세요.

## GitHub Actions에서 태그와 릴리스 게시하기

태그를 버전의 기준으로 쓰는 프로젝트라면 워크플로 작업에 아래 권한과 단계를 넣으세요. 태그를 푸시하고 변경 내역이 담긴 GitHub Release를 생성하며, 매니페스트 파일은 **수정하지 않습니다**. 시작 기준이 될 버전 태그가 저장소에 있어야 합니다.

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

입출력은 [Action 레퍼런스(영문)](docs/github-action.md)에서, CI에서 매니페스트까지 갱신하는 방법은 [릴리스 워크플로](https://flopha.solaqua.dev/ko/docs/release-workflows)에서 확인하세요.

## 지금 쓰는 릴리스 규칙 그대로

`mobile@1.2.3` 같은 태그, 릴리스 브랜치, `rc` 사전 릴리스는 [버전 패턴](https://flopha.solaqua.dev/ko/docs/version-patterns)으로 설정할 수 있습니다. 커스텀 커밋 규칙을 지정하면 기존 커밋 관례도 유지할 수 있습니다. 자세한 옵션은 [명령어 및 `flopha.toml` 레퍼런스](https://flopha.solaqua.dev/ko/docs/command-reference)를 참고하세요.

## 기여하기

[CONTRIBUTING.md(영문)](CONTRIBUTING.md)를 참고하세요.

## 라이선스

[MIT](LICENSE)
