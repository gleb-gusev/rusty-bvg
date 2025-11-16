use rusty_bvg::fetch_warschauer_str;
#[allow(unused_imports)]
use rusty_bvg::Departure;
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn, debug};

#[cfg(feature = "display")]
use rusty_bvg::BvgDisplay;


fn init_logging() {
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;

    // Set default log level based on build mode
    let default_level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };

    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(stderr_layer)
        .init();
}

/// Set up panic handler to log panics before crashing
fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown".to_string());

        let message = panic_info.payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| {
                panic_info.payload()
                    .downcast_ref::<String>()
                    .map(|s| s.as_str())
            })
            .unwrap_or("unknown panic");

        error!(
            location = %location,
            message = %message,
            "Application panicked"
        );

        // Try to get backtrace if available
        #[cfg(debug_assertions)]
        {
            let backtrace = std::backtrace::Backtrace::capture();
            error!("Backtrace:\n{}", backtrace);
        }

    }));
}

// API test mode (without LED matrix)
#[cfg(not(feature = "display"))]
fn main() {
    setup_panic_handler();
    init_logging();

    info!("BVG API Test Mode - Warschauer Straße");
    info!("======================================");
    info!("(Display mode disabled - run with --features display on RPi)");

    info!("API ready");
    info!("Fetching departures every 20 seconds...");
    info!("Press Ctrl+C to exit");

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();

    #[cfg(debug_assertions)]
    let mut last_departures: Vec<Departure> = Vec::new();

    loop {
        match fetch_warschauer_str(&agent) {
            Ok(departures) => {
                if !departures.is_empty() {
                    info!("Fetched {} departures", departures.len());
                    #[cfg(debug_assertions)]
                    {
                        for (i, dep) in departures.iter().take(3).enumerate() {
                            debug!("  {}. {}", i + 1, dep.format());
                        }
                        last_departures = departures;
                    }
                } else {
                    warn!("No departures found");
                }
            }
            Err(e) => {
                error!("API Error: {}", e);
                #[cfg(debug_assertions)]
                {
                    if !last_departures.is_empty() {
                        debug!("Using cached data");
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
    setup_panic_handler();
    init_logging();

    info!("BVG Live Display - Warschauer Straße");
    info!("=====================================");

    info!("API ready");

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();

    // Initialize display
    let mut display = match BvgDisplay::new() {
        Ok(d) => {
            let (width, height) = d.dimensions();
            info!("Display initialized: {}x{}", width, height);
            d
        }
        Err(e) => {
            error!("Failed to initialize display: {}", e);
            error!("Make sure you're running on a Raspberry Pi with proper permissions.");
            std::process::exit(1);
        }
    };

    info!("Starting live display...");
    info!("  - Fetching data every 20 seconds");
    info!("  - Cycling between top 3 departures every 10 seconds");
    info!("Press Ctrl+C to exit");

    // Fetch initial data immediately
    info!("Fetching initial data...");
    let mut departures: Vec<Departure> = match fetch_warschauer_str(&agent) {
        Ok(new_departures) => {
            if !new_departures.is_empty() {
                let departures = new_departures.into_iter().take(3).collect::<Vec<_>>();
                info!("Fetched {} departures", departures.len());
                #[cfg(debug_assertions)]
                {
                    for dep in &departures {
                        debug!("  - {}", dep.format());
                    }
                }
                departures
            } else {
                warn!("No departures available");
                Vec::new()
            }
        }
        Err(e) => {
            error!("API Error: {}", e);
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
            info!("Refreshing data...");
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
                        info!("Fetched {} departures", departures.len());
                        #[cfg(debug_assertions)]
                        {
                            for dep in &departures {
                                debug!("  - {}", dep.format());
                            }
                        }
                        needs_render = true; // New data, need to render
                    } else {
                        warn!("No departures available");
                    }
                }
                Err(e) => {
                    error!("API Error: {} (using cached data)", e);
                }
            }
            last_fetch = std::time::Instant::now();
        }

        // Change display every 10 seconds
        if last_display_change.elapsed() >= Duration::from_secs(10) {
            if departures.len() > 1 {
                display.next_departure(departures.len());
                let current_dep = &departures[display.current_index() % departures.len()];
                debug!("Showing: {}", current_dep.format());
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


