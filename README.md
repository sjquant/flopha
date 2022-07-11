# flopha

flopha is a tool to help your github flow.

## Install

Shell (Mac, Linux):

```
curl -fsSL https://raw.githubusercontent.com/sjquant/flopha/main/scripts/install.sh | sh
```

## Getting Started

1. You can create feature branch with

   ```sh
   flohpa start-feature -b <feature-name>
   ```

   which creates branch with a given name and checkout to the branch.

2. You can finish feature branch with

   ```sh
   flopha finish-feature
   ```

   which pushes your branch to remote. You don't have to do things like `git push --set-upstream origin <feature-name>`.

3. You can start hotfix with

   ```sh
   flohpa start-hotfix
   ```

   which searches the latest tag on the remote, and checkout to the tag.

4. You can finish hotfix with

   ```sh
   flopha finish-hotfix
   ```

   which creates new tag with patch version up, and push it to the remote.

## Notes

I wanted to make my github workflow at my company easier, and start my project with `Rust`. It might not be fit to your needs. If you have any opinions to improve `flopha`, I'm very open to them.
