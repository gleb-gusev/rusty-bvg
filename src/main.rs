use rusty_bvg::{fetch_warschauer_str, Departure};
use std::io::Write;
use std::thread;
use std::time::Duration;

#[cfg(feature = "display")]
use rusty_bvg::BvgDisplay;

// Helper to format time without chrono overhead
#[cfg(feature = "display")]
fn format_time() -> (u64, u64, u64) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let h = (now % 86400) / 3600;
    let m = (now % 3600) / 60;
    let s = now % 60;
    (h, m, s)
}

// API test mode (without LED matrix)
#[cfg(not(feature = "display"))]
fn main() {
    println!("BVG API Test Mode - Warschauer Straße");
    println!("======================================");
    println!("(Display mode disabled - run with --features display on RPi)\n");

    println!("✓ API ready");
    println!("Fetching departures every 20 seconds...");
    println!("Press Ctrl+C to exit\n");

    let mut last_departures: Vec<Departure> = Vec::new();

    loop {
        match fetch_warschauer_str() {
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

    println!("✓ API ready");

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
    let (h, m, s) = format_time();
    println!("[{:02}:{:02}:{:02}] Fetching initial data...", h, m, s);
    let mut departures: Vec<Departure> = match fetch_warschauer_str() {
        Ok(new_departures) => {
            if !new_departures.is_empty() {
                let departures = new_departures.into_iter().take(3).collect::<Vec<_>>();
                let (h, m, s) = format_time();
                println!("[{:02}:{:02}:{:02}] ✓ Fetched {} departures", h, m, s, departures.len());
                for dep in &departures {
                    println!("  - {}", dep.format());
                }
                departures
            } else {
                let (h, m, s) = format_time();
                println!("[{:02}:{:02}:{:02}] ⚠ No departures available", h, m, s);
                Vec::new()
            }
        }
        Err(e) => {
            let (h, m, s) = format_time();
            eprintln!("[{:02}:{:02}:{:02}] ✗ API Error: {}", h, m, s, e);
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
            let _ = std::io::stderr().flush(); // Flush stderr to prevent buffer growth
            let (h, m, s) = format_time();
            println!("[{:02}:{:02}:{:02}] Refreshing data...", h, m, s);
            let _ = std::io::stdout().flush(); // Flush stdout
            match fetch_warschauer_str() {
                Ok(new_departures) => {
                    if !new_departures.is_empty() {
                        departures = new_departures.into_iter().take(3).collect();
                        let (h, m, s) = format_time();
                        println!("[{:02}:{:02}:{:02}] ✓ Fetched {} departures", h, m, s, departures.len());
                        for dep in &departures {
                            println!("  - {}", dep.format());
                        }
                        let _ = std::io::stdout().flush();
                        needs_render = true; // New data, need to render
                    } else {
                        let (h, m, s) = format_time();
                        println!("[{:02}:{:02}:{:02}] ⚠ No departures available", h, m, s);
                    }
                }
                Err(e) => {
                    let (h, m, s) = format_time();
                    eprintln!("[{:02}:{:02}:{:02}] ✗ API Error: {} (using cached data)", h, m, s, e);
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
                let _ = std::io::stderr().flush();
                let (h, m, s) = format_time();
                println!("[{:02}:{:02}:{:02}] Showing: {}", h, m, s, current_dep.format());
                let _ = std::io::stdout().flush();
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


