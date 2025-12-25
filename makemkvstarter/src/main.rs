use std::fs;
use std::process::Command;
use std::path::Path;
use toml::Value;
use std::io::{self, Write};

fn prompt_and_create_config(file_path: &str) -> Result<Value, String> {
    print!("Enter the input path: ");
    io::stdout().flush().unwrap();
    let mut input_path = String::new();
    io::stdin().read_line(&mut input_path).unwrap();
    let input_path = input_path.trim();

    print!("Enter the output path: ");
    io::stdout().flush().unwrap();
    let mut output_path = String::new();
    io::stdin().read_line(&mut output_path).unwrap();
    let output_path = output_path.trim();

    print!("Enter the path to the MakeMKV executable: ");
    io::stdout().flush().unwrap();
    let mut makemkv_path = String::new();
    io::stdin().read_line(&mut makemkv_path).unwrap();
    let makemkv_path = makemkv_path.trim();

    let config_content = format!(
        "input_path = \"{}\"\noutput_path = \"{}\"\nmakemkv_path = \"{}\"\n",
        input_path, output_path, makemkv_path
    );

    fs::write(file_path, config_content.clone())
        .map_err(|err| format!("Error writing configuration file: {}", err))?;

    toml::from_str(&config_content)
        .map_err(|err| format!("Error parsing TOML: {}", err))
}


fn read_config(file_path: &str) -> Result<Value, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|err| format!("Error reading file: {}", err))?;
    toml::from_str(&content)
        .map_err(|err| format!("Error parsing TOML: {}", err))
}


fn run_makemkv(config: &Value) -> Result<(), String> {
    let input_path = config.get("input_path")
        .and_then(Value::as_str)
        .ok_or("Missing 'input_path' in config")?;
    let output_path = config.get("output_path")
        .and_then(Value::as_str)
        .ok_or("Missing 'output_path' in config")?;
    let makemkv_path = config.get("makemkv_path")
        .and_then(Value::as_str)
        .ok_or("Missing 'makemkv_path' in config")?;

    let status = Command::new(makemkv_path)
        .arg("mkv")
        .arg(format!("--input={}", input_path))
        .arg(format!("--output={}", output_path))
        .status()
        .map_err(|err| format!("Error running MakeMKV: {}", err))?;

    if status.success() {
        Ok(())
    } else {
        Err("MakeMKV command failed".to_string())
    }
}


fn main() {
    let config_file_path = "conf.toml";
    let config = if Path::new(config_file_path).exists() {
        read_config(config_file_path)
    } else {
        println!("Configuration file not found. Please provide the required information.");
        prompt_and_create_config(config_file_path)
    };

    match config {
        Ok(config) => {
            if let Err(err) = run_makemkv(&config) {
                eprintln!("Error running MakeMKV: {}", err);
            }
        }
        Err(err) => eprintln!("Error reading configuration file: {}", err),
    }
}
