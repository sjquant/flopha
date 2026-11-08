<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/brand/flopha-icon-dark.svg">
  <img src="assets/brand/flopha-icon.svg" alt="" width="64">
</picture>

# flopha

[English](README.md) · [한국어](README.ko.md) · [日本語](README.ja.md) · [简体中文](README.zh-CN.md)

**把合并的提交，变成下一次发布。**

flopha 面向负责发布 Git 项目的维护者：根据标签和提交信息计算下一个语义化版本，并可接着更新清单文件中的版本、推送标签、创建附带变更日志的 GitHub Release。无论项目使用什么语言，都能在终端或 CI 中使用同一个二进制程序；运行 CLI 无需 Node.js 或 Python 运行时。

```console
$ git log --oneline v1.2.3..
3f2c1a9 fix: handle empty input
a1b2c3d feat: add release notes export

$ flopha next-version --auto
v1.3.0
```

## 预览下一个版本

提供适用于 macOS（Intel、Apple silicon）和 Linux x86_64 的预编译二进制程序。以下安装脚本会从 GitHub Releases 下载并安装程序：

```sh
curl -fsSL https://raw.githubusercontent.com/sjquant/flopha/main/scripts/install.sh | sh
```

默认安装到 `~/.flopha/bin`。如果找不到 `flopha` 命令，请参阅 [PATH 设置](https://flopha.solaqua.dev/zh/docs/installation)。

确认安装成功后，在已有匹配版本标签的 Git 仓库内计算下一个版本，默认标签格式为 `v1.2.3`。如果还没有匹配的标签，请用 `flopha next-version --initial-version 0.1.0` 替代下方的 `--auto` 命令，预览首个版本 `v0.1.0`：

```sh
flopha --version
flopha next-version --auto
```

如果 `v1.2.3` 之后有上例中的提交，结果就是 `v1.3.0`。`feat:` 升级 minor，破坏兼容性的变更（`feat!:` 或 `BREAKING CHANGE`）升级 major，其余提交升级 patch。

只计算版本不会创建或推送标签；flopha 会先尝试从 `origin` 获取数据，获取失败则使用本地历史。在 CI 中，请拉取完整历史和标签。

其他平台可使用 Rust 1.85 或更高版本[从源码构建](https://flopha.solaqua.dev/zh/docs/installation#build-from-source)。

如果只需通过标签发布，确认结果后即可创建标签并推送到 `origin`，这需要远程仓库的写入权限：

```sh
flopha next-version --auto --create --push
```

创建首个标签时，请用 `--initial-version 0.1.0` 替代 `--auto`。

## 让清单版本与发布同步

如果项目清单文件中也记录了版本号，请创建并提交 `flopha.toml`。以下示例更新已有的 npm `package.json`，也支持 Cargo、Python 清单文件以及通过正则表达式更新其他文件。

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

`--dry-run` 仅预览执行计划，不修改清单、不提交、不创建标签、不推送，也不发布。第二条命令会实际执行这些步骤。请确保工作区没有未提交的更改，并在配置了 `origin` 且有推送权限的分支上运行。创建 GitHub Release 需要 `GH_TOKEN`、`GITHUB_TOKEN`，或在本地通过 `gh auth login` 完成认证。如果还没有版本标签，请为这两条命令都添加 `--initial-version 0.1.0`。

如果 HEAD 已有标签，`release` 会尝试完成尚未成功的推送或 GitHub Release 创建，而不会再次升级版本。没有新提交时，不会生成另一个版本。如需在发布前审查，请用 `flopha release --prepare` 仅更新清单，再按[版本 PR 工作流](https://flopha.solaqua.dev/zh/docs/release-workflows)操作。

## 通过 GitHub Actions 创建标签并发布

如果项目以标签作为版本依据，可以把以下权限和步骤加入工作流的作业中。这会推送标签并创建附带变更日志的 GitHub Release，**不会更新清单文件**。仓库中需要已有匹配的版本标签作为起点。

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

输入和输出请参阅 [Action 参考（英文）](docs/github-action.md)；在 CI 中同步清单版本的方法请参阅[发布工作流](https://flopha.solaqua.dev/zh/docs/release-workflows)。

## 沿用现有发布规则

通过[版本模式](https://flopha.solaqua.dev/zh/docs/version-patterns)，可以使用 `mobile@1.2.3` 这样的标签、发布分支或 `rc` 预发布版本。自定义提交规则也能适配现有的提交规范。完整选项请参阅[命令及 `flopha.toml` 参考](https://flopha.solaqua.dev/zh/docs/command-reference)。

## 参与贡献

请参阅 [CONTRIBUTING.md（英文）](CONTRIBUTING.md)。

## 许可证

[MIT](LICENSE)
