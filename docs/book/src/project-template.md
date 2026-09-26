# New project template

`tpt-nrg new` writes a study project that already builds: a manifest wired to
TPT Energy, a case to study, and a `main.rs` that loads it, solves it, and
prints the result. It is the difference between cloning a repository and
running `cargo run` and writing all of that yourself.

## Use it

```sh
tpt-nrg new my-study
cd my-study
cargo run
```

```
created my-study
  cd my-study
  cargo run
```

```
case Three-bus study: 3 buses
converged in 4 iterations
total losses: 0.574 MW
  bus   1: |V| = 1.0600 pu
  bus   2: |V| = 1.0396 pu
  bus   3: |V| = 1.0283 pu
```

| Flag | Effect |
|------|--------|
| `--dir <path>` | create the project somewhere other than the current directory |
| `--force` | overwrite an existing directory of that name |
| `--local [<repo>]` | depend on a checkout by path instead of on crates.io |

`--local` is the one to use before the first release exists: it rewrites the
dependency block to `path` entries relative to the new project, so the study
builds against the working tree.

```sh
tpt-nrg new my-study --local .
```

## What you get

```
my-study/
├── Cargo.toml      # [workspace] so the project stands alone, even inside another one
├── README.md
├── system.json     # a small three-bus case
├── src/main.rs     # load, solve, report
└── .gitignore
```

The manifest declares an empty `[workspace]` table on purpose: a project
generated inside someone else's cargo workspace would otherwise try to inherit
that workspace's dependencies and lints.

`system.json` is a placeholder, and it is a *valid* system — the scaffolder
parses and validates it before writing anything, so a broken template can never
produce a project that fails on its first run.

## Use your own case

```sh
tpt-nrg convert case14.m -o system.json
cargo run
```

Or point `main.rs` at another file: it is a `const` with an `include_str!`, so
the case is compiled into the binary and the study is reproducible from the
source tree alone.

## The same template through cargo-generate

`templates/energy-system/` is a `cargo-generate` template as well, and the two
paths cannot drift because `tpt-nrg new` embeds the same files:

```sh
cargo generate --git https://github.com/tpt-solutions/tpt-energy \
    --name my-study --subfolder templates/energy-system
```

The template uses the two placeholders `cargo-generate` also provides,
`{{project-name}}` and `{{crate-name}}`. One difference: the `.gitignore` is
stored as `_gitignore` and renamed on write, because a file literally named
`.gitignore` inside the template directory would apply to the template itself
and `cargo-generate` has no rename directive.

## Where to go next

- [Quick Start](quick-start.md) — the library API behind the generated `main.rs`.
- [Tutorial: 14-bus Power Flow](tutorial-powerflow.md) — what the solver is doing.
- [Five-minute quickstart](cli-quickstart.md) — the same case through the CLI.
