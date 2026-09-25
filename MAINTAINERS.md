# Maintainers

## Responsibilities

- Review pull requests against the [coding rules](docs/development/CODING_RULES.md)
  and the [product constitution](docs/product/PRODUCT_CONSTITUTION.md).
- Keep the verification suite green on `main` (see CI).
- Maintain version sync across manifests (`scripts/check-versions.sh`).
- Execute releases per `docs/development/RELEASE_PROCESS.md`.

## Review checklist

- The change is the simplest correct option.
- Tests cover new logic and actually run (the suite fails when tests are
  empty).
- Documentation is updated when behavior changes.
- No dead code, placeholder implementations, or lint suppressions are added.
- The frontend never reaches for filesystem access; capabilities stay minimal.
