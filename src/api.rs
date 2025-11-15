use crate::departure::Departure;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::error::Error;
use std::time::Duration;

// VBB API client for fetching real-time departure data
pub struct BvgApiClient {
    base_url: String,
    client: reqwest::blocking::Client,
}

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

impl BvgApiClient {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            base_url: "https://v6.vbb.transport.rest".to_string(),
            client,
        })
    }

    // Fetch departures for a specific stop
    // stop_id: Station ID (e.g., "900120003" for S+U Warschauer Str.)
    pub fn fetch_departures(&self, stop_id: &str) -> Result<Vec<Departure>, Box<dyn Error>> {
        let url = format!("{}/stops/{}/departures", self.base_url, stop_id);
        
        println!("Fetching departures from: {}", url);
        
        let response = self.client
            .get(&url)
            .query(&[("duration", "60")]) // Next 60 minutes
            .send()?;

        if !response.status().is_success() {
            return Err(format!("API returned status: {}", response.status()).into());
        }

        let api_response: ApiResponse = response.json()?;
        
        let now = Utc::now();
        let mut departures = Vec::new();

        for api_dep in api_response.departures {
            // Skip if missing required fields
            let direction = match api_dep.direction {
                Some(d) => d,
                None => continue,  // Skip departures without destination
            };
            
            // Skip departures going TO Warschauer Str. (we're already here!)
            // TODO: make this configurable for other stations
            if direction.contains("Warschauer") {
                continue;
            }
            
            let when = match api_dep.when {
                Some(w) => w,
                None => continue,  // Skip departures without time
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
                continue;
            }
            
            // Parse departure time
            if let Ok(departure_time) = DateTime::parse_from_rfc3339(&when) {
                let departure_utc = departure_time.with_timezone(&Utc);
                let diff = departure_utc.signed_duration_since(now);
                let minutes = diff.num_minutes();

                // Only include future departures (at least 1 minute away)
                if minutes >= 1 && minutes <= 60 {
                    // Clean up destination name
                    let mut destination = direction.clone();
                    
                    // Remove " (Berlin)" suffix
                    destination = destination.replace(" (Berlin)", "");
                    
                    // Remove "S " prefix from S-Bahn station names
                    if destination.starts_with("S ") {
                        destination = destination[2..].to_string();
                    }
                    
                    // Remove "U " prefix from U-Bahn station names
                    if destination.starts_with("U ") {
                        destination = destination[2..].to_string();
                    }
                    
                    // Remove " Bhf" suffix (Bahnhof)
                    destination = destination.replace(" Bhf", "");
                    
                    // Remove Ringbahn direction symbols (⟲/⟳)
                    destination = destination.replace(" ⟲", "");
                    destination = destination.replace(" ⟳", "");
                    
                    departures.push(Departure::new(
                        api_dep.line.name,
                        destination,
                        minutes as u32,
                    ));
                }
            }
        }

        // Sort by minutes (closest first)
        departures.sort_by_key(|d| d.minutes);

        Ok(departures)
    }

    // Hardcoded for Warschauer Str for now
    // TODO: make station ID configurable via config file or CLI args
    pub fn fetch_warschauer_str(&self) -> Result<Vec<Departure>, Box<dyn Error>> {
        self.fetch_departures("900120003")
    }
}

impl Default for BvgApiClient {
    fn default() -> Self {
        Self::new().expect("Failed to create BVG API client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let client = BvgApiClient::new();
        assert!(client.is_ok());
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

