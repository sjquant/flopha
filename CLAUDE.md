# flopha

## Setup

After cloning, install dev tools and activate git hooks:

```sh
cargo install prek
prek install
prek install --hook-type commit-msg
```

Hooks enforce formatting (`cargo fmt`), linting (`cargo clippy`), secret detection, and [gitmoji](https://gitmoji.dev) commit messages.

## Commit style

All commits and PR titles must start with a gitmoji emoji followed by a space — e.g. `✨ Add feature`. See https://gitmoji.dev for the full list.
