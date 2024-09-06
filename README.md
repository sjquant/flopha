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

2.  Calculate the next version:

    ```
    flopha next-version
    ```

3.  Use a custom version pattern:

    ```
    flopha next-version --pattern "dekstop@{major}.{minor}.{patch}"
    ```

4.  Create a new version tag:

    ```
    flopha next-version --pattern "dekstop@{major}.{minor}.{patch} --action create
    ```

5.  Increment major version:

    ```
    flopha next-version --increment major
    ```

6.  Use branch-based versioning:

    ```
    flopha next-version --source branch
    ```

7.  Create a new version branch:

    ```
    flopha next-version --pattern "release/{major}.{minor}.{patch}" --source branch --action create
    ```

## CLI Options

#### Options for NextVersion

- `-i`, `--increment <INCREMENT>`: Specify the version part to increment. Options are:

  - `major`
  - `minor`
  - `patch`

  Default: `patch`

- `-p`, `--pattern <PATTERN>`: Specify a custom pattern for version matching and generation. Use placeholders `{major}`, `{minor}`, and `{patch}`. Example patterns:

  - `v{major}.{minor}.{patch}`
  - `release-{major}.{minor}.{patch}`

- `-v`, `--verbose`: Enable verbose output for detailed information.

- `-s`, `--source <SOURCE>`: Specify the source for versioning. Options are:

  - `tag` (default)
  - `branch`

- `-a`, `--action <ACTION>`: Action to perform. Options are:
  - `print` (default)
  - `create` (creates a new tag or branch)

#### Options for LastVersion

- `-p`, `--pattern <PATTERN>`: Get the last version based on a given pattern (e.g., `v{major}.{minor}.{patch}`).

- `-v`, `--verbose`: Enable verbose output for detailed information.

- `-s`, `--source <SOURCE>`: Specify the source for versioning. Options are:

  - `tag` (default)
  - `branch`

- `--action <ACTION>`: Action to perform. Options are:
  - `print` (default)
  - `checkout` (checks out the last version)

### LastVersion

Finds the latest version tag or branch in the repository matching a given pattern.

## Why Choose flopha?

- **Simplify Semantic Versioning**: Automate version calculations based on your preferred patterns.
- **Streamline Git Workflows**: Easily manage tags and versions across multiple branches and projects.
- **Flexible and Customizable**: Adapt to various versioning schemes and project structures.
- **Boost Productivity**: Reduce manual version management tasks and potential errors.

## License

flopha is released under the [MIT License](LICENSE).
