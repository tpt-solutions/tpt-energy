//! `tpt-nrg new`: scaffold a study project from `templates/energy-system/`.
//!
//! The template directory is the single source of truth for a new project and
//! is embedded at compile time with `include_str!`, so `tpt-nrg new` writes
//! exactly what `cargo generate` would from the same repository. The
//! substitution rules are deliberately the two placeholders
//! `cargo-generate` also provides (`{{project-name}}` and `{{crate-name}}`),
//! which keeps the two paths from drifting apart.
//!
//! `--local` swaps the crates.io dependency versions for `path` dependencies
//! on a checkout of this repository, so a new project can be built and run
//! before anything has been published.

use std::path::{Path, PathBuf};

use tpt_nrg_interop::Format;

use crate::{CliError, NewArgs};

/// The template files, as `(path inside the project, contents)` pairs.
///
/// `_gitignore` is written out as `.gitignore`; a file literally named
/// `.gitignore` inside the template directory would apply to the template
/// itself, and `cargo-generate` has no rename directive.
const FILES: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        include_str!("../../../../templates/energy-system/Cargo.toml"),
    ),
    (
        "README.md",
        include_str!("../../../../templates/energy-system/README.md"),
    ),
    (
        "system.json",
        include_str!("../../../../templates/energy-system/system.json"),
    ),
    (
        "src/main.rs",
        include_str!("../../../../templates/energy-system/src/main.rs"),
    ),
    (
        ".gitignore",
        include_str!("../../../../templates/energy-system/_gitignore"),
    ),
];

/// The crates the generated project depends on, as `(crate, path in repo)`.
const DEPENDENCIES: &[(&str, &str)] = &[
    ("tpt-nrg-core", "crates/core/tpt-nrg-core"),
    ("tpt-nrg-powerflow", "crates/grid/tpt-nrg-powerflow"),
];

/// Render the project files for `name`.
///
/// Line endings are normalised to `\n` so the generated project is identical
/// on every platform, whatever the checkout used.
fn render(name: &str, crate_name: &str) -> Result<Vec<(String, String)>, CliError> {
    let mut out = Vec::with_capacity(FILES.len());
    for (path, template) in FILES {
        let text = template
            .replace("\r\n", "\n")
            .replace("{{project-name}}", name)
            .replace("{{crate-name}}", crate_name);
        if text.contains("{{") {
            return Err(CliError::Usage(format!(
                "template file {path} still contains an unsubstituted placeholder"
            )));
        }
        out.push(((*path).to_string(), text));
    }
    Ok(out)
}

/// Turn a project name into a valid crate name: `My-Study` -> `my_study`.
fn crate_name(name: &str) -> String {
    name.to_lowercase().replace(['-', ' '], "_")
}

/// Reject a name cargo could not use as a package name.
fn validate_name(name: &str) -> Result<(), CliError> {
    let mut chars = name.chars();
    let first = chars.next().ok_or_else(|| {
        CliError::Usage(
            "a project name is required, for example `tpt-nrg new my-study`".to_string(),
        )
    })?;
    let valid_first = first.is_ascii_alphabetic() || first == '_';
    let valid_rest = chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid_first || !valid_rest || name.len() > 64 {
        return Err(CliError::Usage(format!(
            "`{name}` is not a valid crate name: use letters, digits, `-`, and `_`, starting with a \
             letter"
        )));
    }
    Ok(())
}

/// The section header the `--local` rewrite replaces.
const DEPENDENCIES_HEADER: &str = "[dependencies]\n";

/// Rewrite the `[dependencies]` block to point at a local checkout.
///
/// The block is the last section of the template manifest, so it is replaced
/// wholesale rather than line by line; the replaced tail is checked to hold
/// only dependency lines so a future template edit cannot silently drop
/// something else.
fn use_local_paths(manifest: &mut String, root: &Path, project_dir: &Path) -> Result<(), CliError> {
    use std::fmt::Write as _;

    let mut block = String::from("[dependencies]\n");
    for (crate_name, relative) in DEPENDENCIES {
        let absolute = root.join(relative);
        if !absolute.join("Cargo.toml").is_file() {
            return Err(CliError::Usage(format!(
                "{} is not a tpt-energy checkout: {} is missing",
                root.display(),
                absolute.join("Cargo.toml").display()
            )));
        }
        let from_project = relative_to(project_dir, &absolute).ok_or_else(|| {
            CliError::Usage(format!(
                "cannot express {} relative to the project",
                absolute.display()
            ))
        })?;
        let _ = writeln!(
            block,
            "{crate_name} = {{ path = \"{}\" }}",
            toml_path(&from_project)
        );
    }

    let Some(start) = manifest.find(DEPENDENCIES_HEADER) else {
        return Err(CliError::Usage(
            "the template manifest no longer has a `[dependencies]` block to rewrite".to_string(),
        ));
    };
    let tail = &manifest[start + DEPENDENCIES_HEADER.len()..];
    if !tail
        .lines()
        .all(|line| line.trim_start().starts_with("tpt-nrg-"))
    {
        return Err(CliError::Usage(format!(
            "the template manifest has content after `[dependencies]` ({tail:?}); refusing to drop it"
        )));
    }
    manifest.truncate(start);
    manifest.push_str(&block);
    Ok(())
}

/// Express `to` relative to `from`, walking up with `..` as needed.
///
/// `from` is the project directory, which does not exist yet when this runs,
/// so the longest existing ancestor is canonicalized and the remaining
/// components are re-appended.
fn relative_to(from: &Path, to: &Path) -> Option<PathBuf> {
    let from = canonicalize_lenient(from)?;
    let to = to.canonicalize().ok()?;
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut out = PathBuf::new();
    for _ in common..from.len() {
        out.push("..");
    }
    for component in &to[common..] {
        out.push(component.as_os_str());
    }
    Some(out)
}

/// Render a path the way a `Cargo.toml` wants it.
///
/// `canonicalize` on Windows returns the `\\?\` verbatim prefix, which is
/// neither valid inside a TOML basic string nor something a reader
/// recognises, and its backslashes would have to be escaped. Cargo accepts
/// forward slashes on every platform, so strip the prefix and use them.
fn toml_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    let without_prefix = match text.strip_prefix(r"\\?\UNC\") {
        Some(unc) => format!("//{}", unc.replacen('\\', "/", 1)),
        None => text.strip_prefix(r"\\?\").unwrap_or(&text).to_string(),
    };
    without_prefix.replace('\\', "/")
}

/// `canonicalize` that tolerates a path whose tail does not exist yet.
fn canonicalize_lenient(path: &Path) -> Option<PathBuf> {
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut existing = path;
    loop {
        if existing.exists() {
            let mut out = existing.canonicalize().ok()?;
            for part in tail.iter().rev() {
                out.push(part);
            }
            return Some(out);
        }
        let name = existing.file_name()?.to_os_string();
        existing = existing.parent()?;
        tail.push(name);
    }
}

/// Handle `tpt-nrg new`.
pub fn scaffold(args: &NewArgs) -> Result<(), CliError> {
    validate_name(&args.name)?;
    let crate_name = crate_name(&args.name);
    let parent = args.dir.clone().unwrap_or_else(|| PathBuf::from("."));
    let project_dir = parent.join(&args.name);
    if project_dir.exists() && !args.force {
        return Err(CliError::Usage(format!(
            "{} already exists; pass --force to overwrite it",
            project_dir.display()
        )));
    }

    let mut files = render(&args.name, &crate_name)?;
    if let Some(root) = args.local.clone() {
        for (path, text) in &mut files {
            if path == "Cargo.toml" {
                use_local_paths(text, &root, &project_dir)?;
            }
        }
    }

    // The bundled case must be a valid system, otherwise the new project
    // fails on its first run and the template looks broken.
    let case = files
        .iter()
        .find(|(path, _)| path == "system.json")
        .map(|(_, text)| text.as_str())
        .unwrap_or_default();
    tpt_nrg_interop::from_text(case, Format::Json)
        .map_err(|e| CliError::Usage(format!("the template case is not valid: {e}")))?;

    for (path, text) in &files {
        let target = project_dir.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CliError::Io(format!("cannot create {}: {e}", parent.display())))?;
        }
        std::fs::write(&target, text)
            .map_err(|e| CliError::Io(format!("cannot write {}: {e}", target.display())))?;
    }

    println!("created {}", project_dir.display());
    println!("  cd {}", args.name);
    println!("  cargo run");
    if args.local.is_none() {
        println!(
            "note: the dependencies come from crates.io; before the first release, re-run with \
             --local to point them at a checkout"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tpt-nrg-new-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn args(name: &str, dir: &Path, local: Option<PathBuf>) -> NewArgs {
        NewArgs {
            name: name.to_string(),
            dir: Some(dir.to_path_buf()),
            force: true,
            local,
        }
    }

    #[test]
    fn a_new_project_has_every_template_file() {
        let dir = temp_dir("files");
        scaffold(&args("my-study", &dir, None)).expect("scaffold");
        for path in [
            "Cargo.toml",
            "README.md",
            "system.json",
            "src/main.rs",
            ".gitignore",
        ] {
            assert!(dir.join("my-study").join(path).is_file(), "{path}");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_project_name_is_substituted_everywhere() {
        let dir = temp_dir("substitute");
        scaffold(&args("my-study", &dir, None)).expect("scaffold");
        let project = dir.join("my-study");
        let manifest = std::fs::read_to_string(project.join("Cargo.toml")).expect("manifest");
        assert!(manifest.contains("name = \"my_study\""), "{manifest}");
        assert!(!manifest.contains("{{"), "{manifest}");
        let main = std::fs::read_to_string(project.join("src/main.rs")).expect("main");
        assert!(main.contains("`my-study`"), "{main}");
        let readme = std::fs::read_to_string(project.join("README.md")).expect("readme");
        assert!(readme.starts_with("# my-study"), "{readme}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_bundled_case_is_a_valid_system() {
        let dir = temp_dir("case");
        scaffold(&args("study", &dir, None)).expect("scaffold");
        let case = std::fs::read_to_string(dir.join("study").join("system.json")).expect("case");
        let system = tpt_nrg_interop::from_text(&case, Format::Json).expect("the template parses");
        assert_eq!(system.buses.len(), 3);
        assert_eq!(system.branches.len(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_mode_points_the_dependencies_at_a_checkout() {
        let dir = temp_dir("local");
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..");
        scaffold(&args("study", &dir, Some(root))).expect("scaffold");
        let manifest =
            std::fs::read_to_string(dir.join("study").join("Cargo.toml")).expect("manifest");
        assert!(manifest.contains("tpt-nrg-core = { path ="), "{manifest}");
        assert!(!manifest.contains("tpt-nrg-core = \"0.1\""), "{manifest}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn local_mode_rejects_a_directory_that_is_not_a_checkout() {
        let dir = temp_dir("not-a-checkout");
        let err = scaffold(&args("study", &dir, Some(dir.clone()))).expect_err("rejected");
        assert!(matches!(err, CliError::Usage(_)));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_invalid_project_name_is_rejected() {
        for name in ["", "9lives", "has space!", "with/slash"] {
            assert!(validate_name(name).is_err(), "`{name}` should be rejected");
        }
        assert!(validate_name("my-study_2").is_ok());
    }

    #[test]
    fn an_existing_project_is_not_overwritten_without_force() {
        let dir = temp_dir("existing");
        let mut a = args("study", &dir, None);
        a.force = false;
        scaffold(&a).expect("first scaffold");
        assert!(scaffold(&a).is_err(), "the second run must refuse");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_rendered_manifest_uses_unix_line_endings() {
        let dir = temp_dir("line-endings");
        scaffold(&args("study", &dir, None)).expect("scaffold");
        let manifest =
            std::fs::read_to_string(dir.join("study").join("Cargo.toml")).expect("manifest");
        assert!(
            !manifest.contains('\r'),
            "a generated manifest must use \\n"
        );
        assert!(manifest.contains("[dependencies]\n"), "{manifest}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn manifest_paths_drop_the_windows_verbatim_prefix() {
        // `canonicalize` on Windows hands back `\\?\D:\...`, which cargo
        // cannot read out of a TOML string.
        let rendered = toml_path(Path::new(r"\\?\D:\Programming\tpt-energy\crates"));
        assert_eq!(rendered, "D:/Programming/tpt-energy/crates");
        let rendered = toml_path(Path::new(r"\\?\UNC\server\share\crates"));
        assert_eq!(rendered, "//server/share/crates");
        let rendered = toml_path(Path::new("/home/user/tpt-energy/crates"));
        assert_eq!(rendered, "/home/user/tpt-energy/crates");
    }

    #[test]
    fn relative_paths_walk_up_and_back_down() {
        let from = std::env::temp_dir().join("tpt-nrg-relative-from");
        let to = std::env::temp_dir();
        std::fs::create_dir_all(&from).expect("temp dir");
        let relative = relative_to(&from, &to).expect("relative path");
        assert_eq!(
            from.join(relative).canonicalize().expect("resolves"),
            to.canonicalize().expect("canonical")
        );
        std::fs::remove_dir_all(&from).ok();
    }
}
