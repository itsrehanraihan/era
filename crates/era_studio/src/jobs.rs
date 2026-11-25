//! Background runs of the `era` CLI (`preview --png`, `build android`), so
//! the editor stays responsive. Output is streamed back line by line.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, channel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    Preview,
    BuildAndroid,
    /// `era export android --force` (Era Builder's Export button).
    ExportAndroid,
}

impl JobKind {
    pub fn label(self) -> &'static str {
        match self {
            JobKind::Preview => "Rendering preview",
            JobKind::BuildAndroid => "Building APK",
            JobKind::ExportAndroid => "Exporting Android Studio project",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEvent {
    Line(String),
    Finished { success: bool },
}

pub struct Job {
    pub kind: JobKind,
    pub events: Receiver<JobEvent>,
    pub started: std::time::Instant,
}

/// The `era` CLI: `ERA_BIN`, then an `era` next to this executable (both
/// are built together), then `era` on `PATH`.
pub fn era_binary() -> PathBuf {
    if let Some(p) = std::env::var_os("ERA_BIN") {
        return PathBuf::from(p);
    }
    let exe_name = if cfg!(windows) { "era.exe" } else { "era" };
    if let Ok(me) = std::env::current_exe()
        && let Some(dir) = me.parent()
    {
        let sibling = dir.join(exe_name);
        if sibling.is_file() {
            return sibling;
        }
    }
    PathBuf::from(exe_name)
}

pub fn args(kind: JobKind, root: &Path) -> Vec<String> {
    let root = root.display().to_string();
    match kind {
        JobKind::Preview => vec!["preview".into(), root, "--png".into()],
        JobKind::BuildAndroid => vec!["build".into(), "android".into(), root],
        JobKind::ExportAndroid => vec!["export".into(), "android".into(), root, "--force".into()],
    }
}

/// Start a job. Stdout and stderr lines arrive as `JobEvent::Line`.
pub fn start(kind: JobKind, root: &Path) -> Job {
    let (tx, rx) = channel();
    let bin = era_binary();
    let args = args(kind, root);
    std::thread::spawn(move || {
        let child = Command::new(&bin)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(JobEvent::Line(format!(
                    "error: cannot run `{}`: {e}. Install it with `cargo install --path crates/era_cli` or set ERA_BIN.",
                    bin.display()
                )));
                let _ = tx.send(JobEvent::Finished { success: false });
                return;
            }
        };
        let stderr = child.stderr.take().unwrap();
        let tx_err = tx.clone();
        let err_thread = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if !line.starts_with("Picked up ") {
                    let _ = tx_err.send(JobEvent::Line(line));
                }
            }
        });
        for line in BufReader::new(child.stdout.take().unwrap())
            .lines()
            .map_while(Result::ok)
        {
            let _ = tx.send(JobEvent::Line(line));
        }
        let _ = err_thread.join();
        let success = child.wait().map(|s| s.success()).unwrap_or(false);
        let _ = tx.send(JobEvent::Finished { success });
    });
    Job {
        kind,
        events: rx,
        started: std::time::Instant::now(),
    }
}
