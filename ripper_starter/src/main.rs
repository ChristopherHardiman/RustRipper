use flatpak_mod::ensure_flatpaks_installed;

mod config_setup;
mod flatpak_mod;


#[tokio::main]
async fn main() {
    let app_ids = ["com.makemkv.MakeMKV", "fr.handbrake.ghb"];
    ensure_flatpaks_installed(&app_ids).await;

    let config = config_setup::check_and_create_config();
    println!("Config: {:?}", config);
}