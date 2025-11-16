use rusty_bvg::{BvgApiClient, Departure};
use std::thread;
use std::time::Duration;

#[cfg(feature = "display")]
use rusty_bvg::BvgDisplay;

// API test mode (without LED matrix)
#[cfg(not(feature = "display"))]
fn main() {
    println!("BVG API Test Mode - Warschauer Straße");
    println!("======================================");
    println!("(Display mode disabled - run with --features display on RPi)\n");

    let client = match BvgApiClient::new() {
        Ok(c) => {
            println!("✓ API client initialized");
            c
        }
        Err(e) => {
            eprintln!("✗ Failed to create API client: {}", e);
            std::process::exit(1);
        }
    };

    println!("Fetching departures every 20 seconds...");
    println!("Press Ctrl+C to exit\n");

    let mut last_departures: Vec<Departure> = Vec::new();

    loop {
        match client.fetch_warschauer_str() {
            Ok(departures) => {
                if !departures.is_empty() {
                    println!("\n[{}] Fetched {} departures:", 
                        chrono::Local::now().format("%H:%M:%S"), 
                        departures.len()
                    );
                    for (i, dep) in departures.iter().take(3).enumerate() {
                        println!("  {}. {}", i + 1, dep.format());
                    }
                    last_departures = departures;
                } else {
                    println!("\n[{}] No departures found", 
                        chrono::Local::now().format("%H:%M:%S")
                    );
                }
            }
            Err(e) => {
                eprintln!("\n[{}] ✗ API Error: {}", 
                    chrono::Local::now().format("%H:%M:%S"), 
                    e
                );
                if !last_departures.is_empty() {
                    println!("  (Using cached data)");
                }
            }
        }

        thread::sleep(Duration::from_secs(20));
    }
}

// Full mode with LED display (RPi)
#[cfg(feature = "display")]
fn main() {
    println!("BVG Live Display - Warschauer Straße");
    println!("=====================================");

    // Initialize API client
    let client = match BvgApiClient::new() {
        Ok(c) => {
            println!("✓ API client initialized");
            c
        }
        Err(e) => {
            eprintln!("✗ Failed to create API client: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize display
    let mut display = match BvgDisplay::new() {
        Ok(d) => {
            println!("✓ Display initialized");
            d
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize display: {}", e);
            eprintln!("  Make sure you're running on a Raspberry Pi with proper permissions.");
            std::process::exit(1);
        }
    };

    let (width, height) = display.dimensions();
    println!("✓ Display dimensions: {}x{}", width, height);
    println!("\nStarting live display...");
    println!("  - Fetching data every 20 seconds");
    println!("  - Cycling between top 3 departures every 10 seconds");
    println!("Press Ctrl+C to exit\n");

    // Fetch initial data immediately
    eprint!("\r"); // Clear any Hz stats from LED matrix
    println!("[{}] Fetching initial data...", chrono::Local::now().format("%H:%M:%S"));
    let mut departures: Vec<Departure> = match client.fetch_warschauer_str() {
        Ok(new_departures) => {
            if !new_departures.is_empty() {
                let departures = new_departures.into_iter().take(3).collect::<Vec<_>>();
                println!("[{}] ✓ Fetched {} departures", 
                    chrono::Local::now().format("%H:%M:%S"),
                    departures.len()
                );
                for dep in &departures {
                    println!("  - {}", dep.format());
                }
                departures
            } else {
                println!("[{}] ⚠ No departures available", 
                    chrono::Local::now().format("%H:%M:%S")
                );
                Vec::new()
            }
        }
        Err(e) => {
            eprintln!("[{}] ✗ API Error: {}", 
                chrono::Local::now().format("%H:%M:%S"),
                e
            );
            Vec::new()
        }
    };

    let mut last_fetch = std::time::Instant::now();
    let mut last_display_change = std::time::Instant::now();
    let mut needs_render = true;

    if !departures.is_empty() {
        display.render_departures(&departures);
    }

    loop {
        // Fetch new data every 20 seconds
        if last_fetch.elapsed() >= Duration::from_secs(20) {
            eprint!("\r"); // Clear any Hz stats from LED matrix
            println!("[{}] Refreshing data...", chrono::Local::now().format("%H:%M:%S"));
            match client.fetch_warschauer_str() {
                Ok(new_departures) => {
                    if !new_departures.is_empty() {
                        departures = new_departures.into_iter().take(3).collect();
                        println!("[{}] ✓ Fetched {} departures", 
                            chrono::Local::now().format("%H:%M:%S"),
                            departures.len()
                        );
                        for dep in &departures {
                            println!("  - {}", dep.format());
                        }
                        needs_render = true; // New data, need to render
                    } else {
                        println!("[{}] ⚠ No departures available", 
                            chrono::Local::now().format("%H:%M:%S")
                        );
                    }
                }
                Err(e) => {
                    eprintln!("[{}] ✗ API Error: {} (using cached data)", 
                        chrono::Local::now().format("%H:%M:%S"),
                        e
                    );
                }
            }
            last_fetch = std::time::Instant::now();
        }

        // Change display every 10 seconds
        if last_display_change.elapsed() >= Duration::from_secs(10) {
            if departures.len() > 1 {
                display.next_departure(departures.len());
                let current_dep = &departures[display.current_index() % departures.len()];
                eprint!("\r"); // Clear any Hz stats from LED matrix
                println!("[{}] Showing: {}", 
                    chrono::Local::now().format("%H:%M:%S"),
                    current_dep.format()
                );
                needs_render = true; // Changed departure, need to render
            }
            last_display_change = std::time::Instant::now();
        }

        // Render only when needed (not every loop iteration!)
        if needs_render && !departures.is_empty() {
            display.render_departures(&departures);
            needs_render = false;
        }

        // Sleep to avoid busy loop
        thread::sleep(Duration::from_millis(500)); // Increased from 100ms to 500ms
    }
}


