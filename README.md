# flopha

flopha is a tool to help your github flow.

## Install

Shell (Mac, Linux):

```
curl -fsSL https://raw.githubusercontent.com/sjquant/flopha/main/scripts/install.sh | sh
```

## Getting Started

1. You can get the last version of current git repository based on a given pattern.

   ```sh
   flohpa last-version --pattern "v{major}.{minor}.{patch}"
   ```

   You can do like this.

   ```sh
   flopha last-version --pattern "desktop@{major}.{minor}.{patch}"
   ```

2. You can directly checkout the last-version with `--checkout` option. If you want to checkout to the last version for hotfix, this might be useful.

   ```sh
   flopha last-version --checkout
   ```

3. You can calculate and print the next version based on a given pattern.

   ```sh
   flopha next-version --pattern "pattern@{major}.{minor}.{patch}"
   ```

4. You can tag current head as next version. If you want to tag your head after hotfix, this might be useful.

   ```sh
   flopha next-version --pattern "pattern@{major}.{minor}.{patch}" --tag
   ```

## Notes

I wanted to make my github workflow at my company easier, and start my project with `Rust`. It might not fit your needs. If you have any opinions to improve `flopha`, I'm very open to them.
