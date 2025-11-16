use crate::departure::Departure;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::error::Error;
use tracing::{info, error, warn, instrument};

// API response structures for VBB HAFAS API
#[derive(Debug, Deserialize)]
struct ApiResponse {
    departures: Vec<ApiDeparture>,
}

// TODO: might want to parse more fields like platform, provenance, etc.

#[derive(Debug, Deserialize)]
struct ApiDeparture {
    line: Line,
    direction: Option<String>,  // Can be null in API response
    when: Option<String>,        // Can be null in API response
    #[serde(default)]
    #[allow(dead_code)] // Reserved for future delay/disruption display
    delay: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct Line {
    name: String,
}

// Fetch departures for a specific stop
// stop_id: Station ID (e.g., "900120003" for S+U Warschauer Str.)
#[instrument(skip(agent))]
pub fn fetch_departures(agent: &ureq::Agent, stop_id: &str) -> Result<Vec<Departure>, Box<dyn Error>> {
    const WARSCHAUER_STOP_ID: &str = "900120003";
    
    let url = if stop_id == WARSCHAUER_STOP_ID {
        "https://v6.vbb.transport.rest/stops/900120003/departures?duration=15"
    } else {
        return Err(format!("Unsupported stop_id: {}", stop_id).into());
    };
    
    info!("Fetching departures from API: {}", url);
    let start_time = std::time::Instant::now();
    
    let response = match agent.get(url).call() {
        Ok(r) => r,
        Err(e) => {
            let elapsed = start_time.elapsed();
            error!(elapsed_ms = elapsed.as_millis(), "HTTP error: {}", e);
            return Err(format!("HTTP error: {}", e).into());
        }
    };

    let body = match response.into_string() {
        Ok(b) => b,
        Err(e) => {
            let elapsed = start_time.elapsed();
            error!(elapsed_ms = elapsed.as_millis(), "HTTP read error: {}", e);
            return Err(format!("HTTP read error: {}", e).into());
        }
    };
    
    let mut api_response: ApiResponse = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            let elapsed = start_time.elapsed();
            error!(elapsed_ms = elapsed.as_millis(), "JSON parse error: {}", e);
            return Err(format!("JSON parse error: {}", e).into());
        }
    };
    
    drop(body);
    
    let now = Utc::now();
    let mut departures = Vec::with_capacity(15);

    // Extract and immediately drop api_response to free memory
    let departures_vec = std::mem::take(&mut api_response.departures);
    drop(api_response); // Explicitly free ApiResponse struct
    
    for api_dep in departures_vec {
        // Skip if missing required fields
        let direction = match api_dep.direction {
            Some(d) => d,
            None => continue,
        };
        
        // Skip departures going TO Warschauer Str. (we're already here!)
        // TODO: make this configurable for other stations
        if direction.contains("Warschauer") {
            drop(direction);
            continue;
        }
        
        let when = match api_dep.when {
            Some(w) => w,
            None => {
                drop(direction);
                continue;
            }
        };
        
        let line_name = &api_dep.line.name;
        
        // Filter out unwanted lines
        // Keep only: S-Bahn (except Ringbahn), U-Bahn, Trams (M-lines)
        if line_name.starts_with("RE") ||    // Regional Express
           line_name.starts_with("RB") ||    // RegionalBahn
           line_name.starts_with("IC") ||    // InterCity
           line_name.starts_with("EC") ||    // EuroCity
           line_name.starts_with("EN") ||    // EuroNight
           line_name.starts_with("FEX") ||   // Flughafen Express
           line_name.starts_with("ICE") ||   // InterCity Express
           line_name == "S41" ||             // Ringbahn clockwise
           line_name == "S42" ||             // Ringbahn counter-clockwise
           line_name.chars().all(|c| c.is_numeric()) {  // Buses (pure numbers)
            drop(direction);
            drop(when);
            continue;
        }
        
        // Parse departure time
        if let Ok(departure_time) = DateTime::parse_from_rfc3339(&when) {
            let departure_utc = departure_time.with_timezone(&Utc);
            let diff = departure_utc.signed_duration_since(now);
            let minutes = diff.num_minutes();

            // Only include future departures (at least 1 minute away, max 15 minutes)
            if minutes >= 1 && minutes <= 15 {
                // Clean up destination name (optimized: single pass where possible)
                let destination = clean_destination(&direction);
                
                let line_name = api_dep.line.name;
                
                drop(direction);
                drop(when);
                
                departures.push(Departure::new(
                    line_name,
                    destination,
                    minutes as u32,
                ));
            } else {
                drop(direction);
                drop(when);
            }
        } else {
            drop(direction);
            drop(when);
        }
    }

    // Sort by minutes (closest first)
    departures.sort_by_key(|d| d.minutes);
    
    // Shrink to fit to free unused capacity immediately
    departures.shrink_to_fit();

    let elapsed = start_time.elapsed();
    info!(
        elapsed_ms = elapsed.as_millis(),
        count = departures.len(),
        "API request successful, received {} departures",
        departures.len()
    );

    Ok(departures)
}
    
// Optimized destination cleaning - single pass where possible
fn clean_destination(dest: &str) -> String {
    let mut result = String::with_capacity(dest.len());
    let mut chars = dest.chars().peekable();
    
    // Skip "S " or "U " prefix
    let first_char = chars.peek().copied();
    if first_char == Some('S') || first_char == Some('U') {
        chars.next();
        if chars.peek() == Some(&' ') {
            chars.next();
        } else {
            // Put back the S/U if it's not followed by space
            if let Some(c) = first_char {
                result.push(c);
            }
        }
    }
    
    // Process rest of string, skipping " (Berlin)" and " Bhf"
    let mut buffer = String::with_capacity(dest.len());
    while let Some(ch) = chars.next() {
        buffer.push(ch);
        
        // Check for " (Berlin)" suffix
        if buffer.ends_with(" (Berlin)") {
            buffer.truncate(buffer.len() - 9);
            break;
        }
        
        // Check for " Bhf" suffix
        if buffer.ends_with(" Bhf") {
            buffer.truncate(buffer.len() - 4);
            break;
        }
        
        // Check for Ringbahn symbols
        if buffer.ends_with(" ⟲") || buffer.ends_with(" ⟳") {
            buffer.truncate(buffer.len() - 2);
            break;
        }
    }
    
    result.push_str(&buffer);
    drop(buffer); // Explicitly free buffer
    result
}

// Hardcoded for Warschauer Str for now
// TODO: make station ID configurable via config file or CLI args
pub fn fetch_warschauer_str(agent: &ureq::Agent) -> Result<Vec<Departure>, Box<dyn Error>> {
    fetch_departures(agent, "900120003")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_fetch() {
        // Test that fetch function exists and can be called
        // (actual API call would require network, so just check it compiles)
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(10))
            .build();
        let _ = fetch_warschauer_str(&agent);
    }

    #[test]
    fn test_departure_time_calculation() {
        // Test time parsing logic
        let now = Utc::now();
        let future = now + chrono::Duration::minutes(5);
        let future_str = future.to_rfc3339();
        
        let parsed = DateTime::parse_from_rfc3339(&future_str);
        assert!(parsed.is_ok());
    }
}

