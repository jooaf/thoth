# Contribution

Contributions in any way is greatly appreciated :) ! Feel free to report problems, bug fixes, implementing features, etc.

## Opening an issue

### Bug reports

When filing a bug report, fill out the bug report [template](https://github.com/jooaf/thoth/issues/new?assignees=&labels=&projects=&template=bug_report.md&title=). Please add all the necessary details as it'll make it to reproduce the problem.

### Feature requests

Please fill out the feature request [template](https://github.com/jooaf/thoth/issues/new?assignees=&labels=&projects=&template=feature_request.md&title=). Please provide details about your feature idea, as well as why this suggestion will be useful.
Note: Thoth is basically feature complete as the point is to keep the application simple like Heynote. However, I don't want this to discourage people from bringing up feature ideas! There may be some features that actually make sense to incorporate into Thoth.

## Pull requests

If you want to directly contribute to the code, look no further! Here is an expected workflow for a pull request:

1. Fork the project.
2. Make your changes.
   - Make sure to run tests and `clippy` before pushing to your branch
   - `cargo test`
   - `cargo clippy -- -D warnings`
   - Note: if you don't have clippy installed you can add it to your toolchain via `rustup component add clippy`
4. If you are adding a new feature, please update the README.md.
5. Commit and create a pull request to merge into the main branch. Please fill out the pull request template.
6. Make sure that your PR is prepended with either of these in its title, Fix: , Documentation: , Improvement: , Feature: . 
6. Ask a maintainer to review your pull request.
7. Check if the CI workflow passes. These consist of clippy lints, rustfmt checks, and basic tests. If you are a first-time contributor, you may need to wait for a maintainer to let CI run.
8. If changes are suggested or any comments are made, they should probably be addressed.
9. Once it looks good, it'll be merged! PRs will be squashed to maintain repo cleanliness.


