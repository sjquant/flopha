# flopha

flopha is a powerful Git workflow tool designed to simplify version management and streamline your GitHub flow. It helps developers manage semantic versioning, automate tagging, and simplify branch management.

## Install

Shell (Mac, Linux):

```
curl -fsSL https://raw.githubusercontent.com/sjquant/flopha/main/scripts/install.sh | sh
```

## Quickstart

1.  Get the last version:

    ```
    flopha last-version
    ```

2.  Check out to the last version:

    ```
    flopha last-version --checkout
    ```

3.  Calculate the next version:

    ```
    flopha next-version
    ```

4.  Use a custom version pattern:

    ```
    flopha next-version --pattern "desktop@{major}.{minor}.{patch}"
    ```

5.  Auto-detect the next bump from conventional commits:

    ```
    flopha next-version --auto
    ```

6.  Override auto-detection with custom rules:

    ```
    flopha next-version --auto --rule 'major:BREAKING CHANGE' --rule 'minor:^feat'
    ```

7.  Create a pre-release version:

    ```
    flopha next-version --pre rc
    ```

8.  Create a new version tag:

    ```
    flopha next-version --pattern "desktop@{major}.{minor}.{patch}" --create
    ```

9.  Increment major version:

    ```
    flopha next-version --increment major
    ```

10.  Use branch-based versioning:

    ```
    flopha next-version --source branch
    ```

11.  Create a new version branch:

    ```
    flopha next-version --pattern "release/{major}.{minor}.{patch}" --source branch --create
    ```

12.  Show version history:

    ```
    flopha log --limit 10
    ```

## CLI Commands and Options

### NextVersion

Calculates and displays the next version based on the current version in the repository.
Aliases: `nv`

#### Options

- `-i`, `--increment <INCREMENT>`: Specify the version part to increment. Options are:

  - `major`
  - `minor`
  - `patch`

  Default: `patch`

- `--auto`: Auto-detect the bump level from commit messages since the last tag. This currently works with tag-based versioning. Built-in conventional commit behavior is:

  - `BREAKING CHANGE` or `BREAKING-CHANGE` -> `major`
  - `<type>!:` or `<type>(<scope>)!:` -> `major`
  - `feat:` or `feat(scope):` -> `minor`
  - anything else -> `patch`

- `--rule <LEVEL:PATTERN>`: Define custom bump rules used with `--auto`. Repeatable. When any `--rule` flags are provided, they replace the built-in conventional commit rules entirely.

- `-p`, `--pattern <PATTERN>`: Specify a custom pattern for version matching and generation. Use placeholders `{major}`, `{minor}`, and `{patch}`. Example patterns:

  - `v{major}.{minor}.{patch}`
  - `release-{major}.{minor}.{patch}`

- `--pre <CHANNEL>`: Create a pre-release tag on the given channel. Example: `--pre alpha` produces `v1.2.3-alpha.1`.

- `-s`, `--source <SOURCE>`: Specify the source for versioning. Options are:

  - `tag` (default)
  - `branch`

- `-c`, `--create`: Create the next tag or branch in Git.

### LastVersion

Retrieves and displays the most recent version tag or branch in the repository that matches a specified pattern.
Aliases: `lv`

#### Options

- `-p`, `--pattern <PATTERN>`: Get the last version based on a given pattern (e.g., `v{major}.{minor}.{patch}`).

- `-s`, `--source <SOURCE>`: Specify the source for versioning. Options are:

  - `tag` (default)
  - `branch`

- `-c`, `--checkout`: Check out the last matching version.

### Log

Shows matching versions. In tag mode, it also includes tag dates and the number of commits between releases.
Aliases: `lg`

#### Options

- `-p`, `--pattern <PATTERN>`: Filter versions by a pattern such as `v{major}.{minor}.{patch}`.

- `-s`, `--source <SOURCE>`: Specify the source for versioning. Options are:

  - `tag` (default)
  - `branch`

  Tag mode provides full timeline metadata. Branch mode still lists matching versions, but tag dates and commit counts are not available.

- `-n`, `--limit <LIMIT>`: Limit the number of versions shown.

### Global Options

- `-v`, `--verbose`: Enable verbose output for detailed information.

## Why Choose flopha?

- **Simplify Semantic Versioning**: Automate version calculations based on your preferred patterns.
- **Streamline Git Workflows**: Easily manage tags and versions across multiple branches and projects.
- **Flexible and Customizable**: Adapt to various versioning schemes and project structures.
- **Boost Productivity**: Reduce manual version management tasks and potential errors.

## License

flopha is released under the [MIT License](LICENSE).
