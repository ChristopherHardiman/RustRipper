use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;
use serde_derive::{Deserialize, Serialize};
use toml;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub omdb_api_key: String,
    pub output_path: String,
}

impl Config {
    pub fn new(omdb_api_key: String, output_path: String) -> Self {
        Config {
            omdb_api_key,
            output_path,
        }
    }

    pub fn prompt_for_config(omdb_api_key: Option<String>, output_path: Option<String>) -> Self {
        let omdb_api_key = match omdb_api_key {
            Some(key) => {
                println!("OMDb API key is already configured.");
                key
            }
            None => prompt("Please enter your OMDb API key: "),
        };

        let output_path = match output_path {
            Some(path) => {
                println!("Output path for MakeMKV is already configured.");
                path
            }
            None => prompt("Please enter the output path for MakeMKV: "),
        };

        Config::new(omdb_api_key, output_path)
    }
}

fn prompt(prompt: &str) -> String {
    print!("{}", prompt);
    std::io::stdout().flush().unwrap();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    input.trim().to_string()
}

pub fn check_and_create_config() -> Config {
    let config_path = "conf.toml";
    let path = Path::new(config_path);
    let mut config: Option<Config> = None;

    if path.exists() {
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(path)
            .expect("Failed to open conf.toml");

        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read conf.toml");
        config = Some(toml::from_str(&contents).expect("Failed to parse conf.toml"));
    }

    let omdb_api_key = config.as_ref().map(|c| c.omdb_api_key.clone());
    let output_path = config.as_ref().map(|c| c.output_path.clone());
    let config = Config::prompt_for_config(omdb_api_key, output_path);

    let config_toml = toml::to_string_pretty(&config).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .expect("Failed to open or create conf.toml");
    file.write_all(config_toml.as_bytes())
        .expect("Failed to write to conf.toml");

    config
}

