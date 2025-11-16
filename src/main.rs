use rusty_bvg::fetch_warschauer_str;
#[allow(unused_imports)]
use rusty_bvg::Departure;
use std::thread;
use std::time::Duration;

#[cfg(feature = "display")]
use rusty_bvg::BvgDisplay;

// Debug logging macros - only compile in debug mode
#[cfg(debug_assertions)]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        println!($($arg)*);
    };
}

#[cfg(not(debug_assertions))]
#[allow(unused_macros)]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        // No-op in release mode
    };
}

#[cfg(debug_assertions)]
macro_rules! debug_eprint {
    ($($arg:tt)*) => {
        eprintln!($($arg)*);
    };
}

#[cfg(not(debug_assertions))]
#[allow(unused_macros)]
macro_rules! debug_eprint {
    ($($arg:tt)*) => {
        // No-op in release mode
    };
}

// Helper to format time without chrono overhead (only used in debug mode)
#[cfg(all(feature = "display", debug_assertions))]
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
    debug_log!("BVG API Test Mode - Warschauer Straße");
    debug_log!("======================================");
    debug_log!("(Display mode disabled - run with --features display on RPi)\n");

    debug_log!("✓ API ready");
    debug_log!("Fetching departures every 20 seconds...");
    debug_log!("Press Ctrl+C to exit\n");

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();

    #[cfg(debug_assertions)]
    let mut last_departures: Vec<Departure> = Vec::new();

    loop {
        match fetch_warschauer_str(&agent) {
            Ok(departures) => {
                if !departures.is_empty() {
                    #[cfg(debug_assertions)]
                    {
                        println!("\n[{}] Fetched {} departures:", 
                            chrono::Local::now().format("%H:%M:%S"), 
                            departures.len()
                        );
                        for (i, dep) in departures.iter().take(3).enumerate() {
                            println!("  {}. {}", i + 1, dep.format());
                        }
                        last_departures = departures;
                    }
                } else {
                    debug_log!("\nNo departures found");
                }
            }
            Err(e) => {
                debug_eprint!("\n✗ API Error: {}", e);
                let _ = e; // Suppress unused warning in release
                #[cfg(debug_assertions)]
                {
                    if !last_departures.is_empty() {
                        println!("  (Using cached data)");
                    }
                }
            }
        }

        thread::sleep(Duration::from_secs(20));
    }
}

// Full mode with LED display (RPi)
#[cfg(feature = "display")]
fn main() {
    debug_log!("BVG Live Display - Warschauer Straße");
    debug_log!("=====================================");

    debug_log!("✓ API ready");

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();

    // Initialize display
    let mut display = match BvgDisplay::new() {
        Ok(d) => {
            debug_log!("✓ Display initialized");
            d
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize display: {}", e);
            eprintln!("  Make sure you're running on a Raspberry Pi with proper permissions.");
            std::process::exit(1);
        }
    };

    #[cfg(debug_assertions)]
    {
        let (width, height) = display.dimensions();
        debug_log!("✓ Display dimensions: {}x{}", width, height);
    }
    debug_log!("\nStarting live display...");
    debug_log!("  - Fetching data every 20 seconds");
    debug_log!("  - Cycling between top 3 departures every 10 seconds");
    debug_log!("Press Ctrl+C to exit\n");

    // Fetch initial data immediately
    #[cfg(debug_assertions)]
    {
        eprint!("\r");
        let _ = std::io::stderr().flush();
    }
    debug_log!("Fetching initial data...");
    let mut departures: Vec<Departure> = match fetch_warschauer_str(&agent) {
        Ok(new_departures) => {
            if !new_departures.is_empty() {
                let departures = new_departures.into_iter().take(3).collect::<Vec<_>>();
                debug_log!("✓ Fetched {} departures", departures.len());
                #[cfg(debug_assertions)]
                {
                    for dep in &departures {
                        debug_log!("  - {}", dep.format());
                    }
                }
                departures
            } else {
                debug_log!("⚠ No departures available");
                Vec::new()
            }
        }
        Err(e) => {
            debug_eprint!("✗ API Error: {}", e);
            let _ = e; // Suppress unused warning in release
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
            #[cfg(debug_assertions)]
            {
                eprint!("\r");
                let _ = std::io::stderr().flush();
            }
            debug_log!("Refreshing data...");
            #[cfg(debug_assertions)]
            {
                let _ = std::io::stdout().flush();
            }
            match fetch_warschauer_str(&agent) {
                Ok(mut new_departures) => {
                    if !new_departures.is_empty() {
                        // Take only first 3 and immediately free the rest
                        if new_departures.len() > 3 {
                            new_departures.truncate(3);
                            new_departures.shrink_to_fit();
                        }
                        // Explicitly free old departures before replacing
                        drop(std::mem::replace(&mut departures, new_departures));
                        debug_log!("✓ Fetched {} departures", departures.len());
                        #[cfg(debug_assertions)]
                        {
                            for dep in &departures {
                                debug_log!("  - {}", dep.format());
                            }
                        }
                        #[cfg(debug_assertions)]
                        {
                            let _ = std::io::stdout().flush();
                        }
                        needs_render = true; // New data, need to render
                    } else {
                        debug_log!("⚠ No departures available");
                    }
                }
                Err(e) => {
                    debug_eprint!("✗ API Error: {} (using cached data)", e);
                    let _ = e; // Suppress unused warning in release
                }
            }
            last_fetch = std::time::Instant::now();
        }

        // Change display every 10 seconds
        if last_display_change.elapsed() >= Duration::from_secs(10) {
            if departures.len() > 1 {
                display.next_departure(departures.len());
                #[cfg(debug_assertions)]
                {
                    let current_dep = &departures[display.current_index() % departures.len()];
                    eprint!("\r");
                    let _ = std::io::stderr().flush();
                    debug_log!("Showing: {}", current_dep.format());
                }
                #[cfg(debug_assertions)]
                {
                    let _ = std::io::stdout().flush();
                }
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


