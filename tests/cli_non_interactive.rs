use anyhow::Result;
use git2::Repository;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_path(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    std::env::temp_dir().join(format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ))
}

struct TempHome {
    path: PathBuf,
}

struct TestRepo {
    path: PathBuf,
}

impl TempHome {
    fn new() -> Result<Self> {
        let path = unique_path("gitlogue-cli-home");
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn config_path(&self) -> PathBuf {
        self.path.join(".config/gitlogue/config.toml")
    }
}

impl TestRepo {
    fn new() -> Result<Self> {
        let path = unique_path("gitlogue-cli-repo");
        fs::create_dir_all(&path)?;
        Repository::init(&path)?;
        Ok(Self { path })
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn gitlogue_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gitlogue"))
}

fn command_with_home(home: &TempHome) -> Command {
    let mut command = gitlogue_command();
    command
        .env("HOME", &home.path)
        .env("USERPROFILE", &home.path)
        .env_remove("HOMEDRIVE")
        .env_remove("HOMEPATH");
    command
}

fn run_command(command: &mut Command) -> Result<Output> {
    Ok(command.output()?)
}

fn stdout(output: Output) -> String {
    String::from_utf8(output.stdout).unwrap()
}

fn repo_path(repo: &TestRepo) -> &Path {
    repo.path.as_path()
}

#[test]
fn license_flag_prints_third_party_licenses() -> Result<()> {
    let output = run_command(gitlogue_command().arg("--license"))?;
    assert!(output.status.success());
    let stdout = stdout(output);

    assert!(stdout.starts_with(include_str!("../LICENSE-THIRD-PARTY")));

    Ok(())
}

#[test]
fn theme_subcommands_list_and_set_default_theme() -> Result<()> {
    let home = TempHome::new()?;

    let list_output = run_command(command_with_home(&home).args(["theme", "list"]))?;
    assert!(list_output.status.success());
    let list_stdout = stdout(list_output);
    assert!(list_stdout.contains("Available themes:"));
    assert!(list_stdout.contains("  - nord"));

    let set_output = run_command(command_with_home(&home).args(["theme", "set", "nord"]))?;
    assert!(set_output.status.success());
    let set_stdout = stdout(set_output);
    let config = fs::read_to_string(home.config_path())?;

    assert!(set_stdout.contains("Theme set to 'nord'"));
    assert!(config.contains("theme = \"nord\""));

    Ok(())
}

#[test]
fn diff_subcommand_reports_no_changes_for_clean_repo() -> Result<()> {
    let repo = TestRepo::new()?;

    let output = run_command(gitlogue_command().args([
        "--path",
        repo_path(&repo).to_str().unwrap(),
        "diff",
    ]))?;
    assert!(output.status.success());

    assert_eq!(stdout(output), "No changes to display\n");

    Ok(())
}
