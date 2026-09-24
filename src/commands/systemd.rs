use crate::cli::SystemdAction;
use crate::error::AppError;
use std::path::PathBuf;
use std::process::Command;

fn systemd_dir() -> Result<PathBuf, AppError> {
    Ok(dirs::home_dir()
        .ok_or_else(|| AppError::Other("home directory not found".into()))?
        .join(".config/systemd/user"))
}

fn ensure_systemd_available() -> Result<(), AppError> {
    #[cfg(not(target_os = "linux"))]
    {
        return Err(AppError::Other(
            "systemd user services are only supported on Linux".to_string(),
        ));
    }

    #[cfg(target_os = "linux")]
    if Command::new("systemctl").arg("--version").output().is_err() {
        return Err(AppError::Other(
            "systemctl not found — is systemd installed?".to_string(),
        ));
    }

    Ok(())
}

fn systemd_quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('%', "%%")
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

fn service_file() -> String {
    format!("{}-sync.service", env!("CARGO_PKG_NAME"))
}

fn timer_file() -> String {
    format!("{}-sync.timer", env!("CARGO_PKG_NAME"))
}

pub async fn run(action: SystemdAction) -> Result<(), AppError> {
    match action {
        SystemdAction::Install => install().await?,
        SystemdAction::Uninstall => uninstall().await?,
    }
    Ok(())
}

async fn install() -> Result<(), AppError> {
    ensure_systemd_available()?;

    let exe = std::env::current_exe()?;
    let exe = exe
        .to_str()
        .ok_or_else(|| AppError::Other("executable path is not UTF-8".into()))?;
    let dir = systemd_dir()?;
    std::fs::create_dir_all(&dir)?;

    let service_path = dir.join(service_file());
    let service = format!(
        "\
[Unit]
Description=Banker Background Sync
After=network.target

[Service]
Type=oneshot
ExecStart={} sync

[Install]
WantedBy=default.target
",
        systemd_quote(exe)
    );
    std::fs::write(&service_path, service.as_bytes())?;
    eprintln!("Created {}", service_path.display());

    let timer_path = dir.join(timer_file());
    let timer = "\
[Unit]
Description=Run Banker Sync periodically

[Timer]
OnCalendar=*-*-* 00,06,12,18:00:00
Persistent=true

[Install]
WantedBy=timers.target
";
    std::fs::write(&timer_path, timer.as_bytes())?;
    eprintln!("Created {}", timer_path.display());

    run_systemctl(&["daemon-reload"])?;
    run_systemctl(&["enable", "--now", &timer_file()])?;

    eprintln!("systemd timer installed and enabled");
    Ok(())
}

async fn uninstall() -> Result<(), AppError> {
    ensure_systemd_available()?;

    let _ = Command::new("systemctl")
        .arg("--user")
        .args(["disable", "--now", &timer_file()])
        .stderr(std::process::Stdio::null())
        .status();

    let dir = systemd_dir()?;
    let service_path = dir.join(service_file());
    let timer_path = dir.join(timer_file());

    if service_path.exists() {
        std::fs::remove_file(&service_path)?;
        eprintln!("Removed {}", service_path.display());
    }
    if timer_path.exists() {
        std::fs::remove_file(&timer_path)?;
        eprintln!("Removed {}", timer_path.display());
    }

    run_systemctl(&["daemon-reload"])?;
    run_systemctl(&["reset-failed"])?;

    eprintln!("systemd timer removed");
    Ok(())
}

fn run_systemctl(args: &[&str]) -> Result<(), AppError> {
    let status = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .status()?;
    if !status.success() {
        return Err(AppError::Other(format!(
            "systemctl --user {} failed",
            args.join(" ")
        )));
    }
    Ok(())
}
