/*
use udev::Device;
*/
use crate::disc_detect::get_disc_type;
use std::thread;
use std::time::Duration;
use std::path::Path;

mod disc_detect;

fn main() {
    println!("Waiting for disc events...");

    let mut last_disc_state: Option<String> = None;

    loop {
        let devnode = "/dev/sr0"; // Change this to the appropriate device node of your Blu-ray drive
        let path = Path::new(devnode);
        
        if path.exists() {
            let (disc_type, title) = get_disc_type(devnode);
            if let Some(ref last_state) = last_disc_state {
                if disc_type != *last_state {
                    println!("Disc inserted: {} ({})", devnode, disc_type);
                    if let Some(movie_title) = title {
                        println!("label=\"{}\"", movie_title);
                    }
                    last_disc_state = Some(disc_type);
                }
            } else {
                println!("Disc inserted: {} ({})", devnode, disc_type);
                if let Some(movie_title) = title {
                    println!("label=\"{}\"", movie_title);
                }
                last_disc_state = Some(disc_type);
            }
        } else if last_disc_state.is_some() {
            println!("Disc removed");
            last_disc_state = None;
        }

        thread::sleep(Duration::from_secs(2));
    }
}


