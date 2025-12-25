use std::process::Command;

pub fn get_disc_type(devnode: &str) -> (String, Option<String>) {
    let output = Command::new("blkid")
        .arg("-o")
        .arg("export")
        .arg(devnode)
        .output()
        .expect("Failed to execute blkid command");

    let output_str = String::from_utf8_lossy(&output.stdout);
    let disc_type = output_str.lines().find(|line| line.starts_with("TYPE="));
    let disc_label = output_str.lines().find(|line| line.starts_with("LABEL="));

    if output.status.success() {
        let disc_type = disc_type
            .map(|line| line[5..].to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let disc_label = disc_label.map(|line| line[6..].to_string());
        (disc_type, disc_label)
    } else {
        ("Unknown".to_string(), None)
    }
}
