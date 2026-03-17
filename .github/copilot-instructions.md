# Git Commit Guidelines

Always follow the Conventional Commits specification for all commit messages. This is a strict requirement for every commit suggestion.

## Format
- Structure: `<type>(<scope>): <description>`
- Follow with a blank line and a body/footer if necessary.

## Rules
- **Types**: 
  - `feat`: A new feature
  - `fix`: A bug fix
  - `docs`: Documentation only changes
  - `style`: Changes that do not affect the meaning of the code (white-space, formatting, etc)
  - `refactor`: A code change that neither fixes a bug nor adds a feature
  - `perf`: A code change that improves performance
  - `test`: Adding missing tests or correcting existing tests
  - `build`: Changes that affect the build system or external dependencies
  - `ci`: Changes to CI configuration files and scripts
  - `chore`: Other changes that don't modify src or test files
  - `revert`: Reverts a previous commit
- **Tense**: Use the **imperative, present tense** (e.g., "change" instead of "changed" or "changes").
- **Language**: The description and body must be in **English**.
- **Length**: The subject line (first line) must not exceed **50 characters**.
- **Body**: Include a body if more context is required. Wrap the body text at **72 characters**.
- **Footer**: Reference any closed issues in the footer (e.g., "Closes #123").

## Example
feat(auth): add JWT authentication

Implement token-based authentication using jsonwebtoken.
This allows secure access to protected API routes.

Closes #45