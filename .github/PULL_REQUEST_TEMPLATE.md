<!--
Thanks for contributing to tpt-energy!
Please make sure you have read CONTRIBUTING.md before submitting.
-->

## Summary

<!-- One or two sentence summary of the change. -->

## Linked Issues

<!-- Link to the issue(s) this PR addresses, e.g. "Fixes #123". -->

## Type of Change

<!-- Check all that apply. -->

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Refactor (no functional change)
- [ ] Test addition or improvement
- [ ] RFC implementation

## Checklist

<!-- All of these must be true before requesting review. -->

- [ ] I have read [CONTRIBUTING.md](../blob/master/CONTRIBUTING.md)
- [ ] My code follows the project's style (`cargo fmt`, `cargo clippy`)
- [ ] I have added tests that prove my fix/feature works
- [ ] New and existing unit tests pass locally (`cargo test --workspace`)
- [ ] I have updated relevant documentation (rustdoc, README, book)
- [ ] My commits are signed off (`git commit -s`) per the DCO
- [ ] For breaking changes, I have documented the migration path

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo deny check licenses
```

## Additional Notes

Anything reviewers should pay particular attention to.
