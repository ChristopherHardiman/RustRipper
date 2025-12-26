//! Example: Detect disc and fetch metadata
//! 
//! This example demonstrates Phase 1.3 implementation:
//! - Disc detection via blkid
//! - Metadata lookup via OMDb with URL encoding

use rustripper_disc::detect_disc;
use rustripper_metadata::OmdbProvider;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Detect disc
    println!("Checking for disc in /dev/sr0...");
    
    match detect_disc("/dev/sr0")? {
        Some(disc_info) => {
            println!("✓ Disc detected!");
            println!("  Label: {}", disc_info.label);
            println!("  Type: {:?}", disc_info.disc_type);
            println!("  Device: {}", disc_info.device);
            
            // Step 2: Clean up label for search
            let search_query = disc_info.label
                .replace('_', " ")
                .trim()
                .to_string();
            
            println!("\nSearching OMDb for: {}", search_query);
            
            // Step 3: Fetch metadata (requires OMDB_API_KEY environment variable)
            if let Ok(api_key) = env::var("OMDB_API_KEY") {
                let provider = OmdbProvider::new(api_key);
                
                match provider.search(&search_query).await {
                    Ok(results) if !results.is_empty() => {
                        let media = &results[0];
                        println!("✓ Found metadata!");
                        println!("  Title: {}", media.title);
                        println!("  Year: {:?}", media.year);
                        println!("  Type: {:?}", media.media_type);
                        println!("  IMDb: {:?}", media.imdb_id);
                        if let Some(plot) = &media.description {
                            println!("  Plot: {}", plot);
                        }
                    }
                    Ok(_) => println!("✗ No results found"),
                    Err(e) => println!("✗ Metadata error: {}", e),
                }
            } else {
                println!("ℹ Set OMDB_API_KEY environment variable to fetch metadata");
            }
        }
        None => {
            println!("✗ No disc present in /dev/sr0");
            println!("\nTo test disc detection:");
            println!("  1. Insert a DVD or Blu-ray disc");
            println!("  2. Run this example again");
        }
    }
    
    Ok(())
}
