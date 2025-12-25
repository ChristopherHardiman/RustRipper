use std::process::{Command, Output};
//use tokio::runtime::Runtime;

async fn run_command(command: &str, args: &[&str]) -> Output {
    Command::new(command)
        .args(args)
        .output()
        .expect("Failed to execute command")
}

async fn check_flatpak_installed(app_id: &str) -> bool {
    let output = run_command("flatpak", &["list", "--app", "--columns=application"]).await;
    let installed_apps = String::from_utf8_lossy(&output.stdout);
    installed_apps.contains(app_id)
}

async fn install_flatpak(app_id: &str) {
    println!("Installing {}", app_id);
    run_command("flatpak", &["install", "-y", "flathub", app_id]).await;
}

pub async fn ensure_flatpaks_installed(apps: &[&str]) {
    for app_id in apps {
        if check_flatpak_installed(app_id).await {
            println!("{} is already installed.", app_id);
        } else {
            install_flatpak(app_id).await;
        }
    }
}
