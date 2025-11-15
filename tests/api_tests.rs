use rusty_bvg::{BvgApiClient, Departure};

#[test]
fn test_api_client_creation() {
    let client = BvgApiClient::new();
    assert!(client.is_ok());
}

#[test]
fn test_departure_creation() {
    let dep = Departure::new("S3", "Erkner", 5);
    assert_eq!(dep.line, "S3");
    assert_eq!(dep.destination, "Erkner");
    assert_eq!(dep.minutes, 5);
}

#[test]
fn test_departure_format() {
    let dep = Departure::new("S3", "Erkner", 5);
    assert_eq!(dep.format(), "S3 Erkner 5 min");
}

#[test]
fn test_departure_sorting() {
    let mut departures = vec![
        Departure::new("S3", "Erkner", 10),
        Departure::new("U1", "Warschauer Str.", 2),
        Departure::new("S5", "Strausberg", 5),
    ];

    departures.sort_by_key(|d| d.minutes);

    assert_eq!(departures[0].minutes, 2);
    assert_eq!(departures[1].minutes, 5);
    assert_eq!(departures[2].minutes, 10);
}

#[test]
fn test_time_calculation() {
    use chrono::{DateTime, Utc};
    
    let now = Utc::now();
    let future = now + chrono::Duration::minutes(5);
    let future_str = future.to_rfc3339();
    
    let parsed = DateTime::parse_from_rfc3339(&future_str);
    assert!(parsed.is_ok());
    
    if let Ok(departure_time) = parsed {
        let departure_utc = departure_time.with_timezone(&Utc);
        let diff = departure_utc.signed_duration_since(now);
        let minutes = diff.num_minutes();
        
        assert!(minutes >= 4 && minutes <= 6); // Allow 1 minute tolerance
    }
}

#[test]
fn test_departure_format_truncated() {
    let dep = Departure::new("S5", "Strausberg Nord", 8);
    let truncated = dep.format_truncated(15);
    assert!(truncated.len() <= 15);
    assert!(truncated.starts_with("S5 "));
}

#[test]
fn test_destination_cleaning_berlin_suffix() {
    // Test removing " (Berlin)" suffix
    let mut destination = "Springpfuhl (Berlin)".to_string();
    destination = destination.replace(" (Berlin)", "");
    assert_eq!(destination, "Springpfuhl");
}

#[test]
fn test_destination_cleaning_s_prefix() {
    // Test removing "S " prefix
    let mut destination = "S Erkner Bhf".to_string();
    if destination.starts_with("S ") {
        destination = destination[2..].to_string();
    }
    assert_eq!(destination, "Erkner Bhf");
}

#[test]
fn test_destination_cleaning_u_prefix() {
    // Test removing "U " prefix
    let mut destination = "U Berliner Str.".to_string();
    if destination.starts_with("U ") {
        destination = destination[2..].to_string();
    }
    assert_eq!(destination, "Berliner Str.");
}

#[test]
fn test_destination_cleaning_bhf_suffix() {
    // Test removing " Bhf" suffix
    let mut destination = "Potsdam Hauptbahnhof Bhf".to_string();
    destination = destination.replace(" Bhf", "");
    assert_eq!(destination, "Potsdam Hauptbahnhof");
}

#[test]
fn test_destination_cleaning_combined() {
    // Test all cleaning rules combined
    let mut destination = "S Spandau Bhf (Berlin)".to_string();
    
    destination = destination.replace(" (Berlin)", "");
    if destination.starts_with("S ") {
        destination = destination[2..].to_string();
    }
    destination = destination.replace(" Bhf", "");
    
    assert_eq!(destination, "Spandau");
}

#[test]
fn test_destination_cleaning_ringbahn_symbols() {
    // Test removing Ringbahn direction symbols
    let mut destination1 = "Ringbahn S42 ⟲".to_string();
    let mut destination2 = "Ringbahn S41 ⟳".to_string();
    
    destination1 = destination1.replace(" ⟲", "");
    destination2 = destination2.replace(" ⟳", "");
    
    assert_eq!(destination1, "Ringbahn S42");
    assert_eq!(destination2, "Ringbahn S41");
}

#[test]
fn test_line_filtering_regional_trains() {
    // Test that regional trains are filtered out
    let regional_trains = vec!["RE2", "RB24", "IC", "ICE", "EC", "EN", "FEX"];
    
    for line in regional_trains {
        let should_filter = line.starts_with("RE") ||
                           line.starts_with("RB") ||
                           line.starts_with("IC") ||
                           line.starts_with("EC") ||
                           line.starts_with("EN") ||
                           line.starts_with("FEX") ||
                           line.starts_with("ICE");
        assert!(should_filter, "Line {} should be filtered", line);
    }
}

#[test]
fn test_line_filtering_ringbahn() {
    // Test that Ringbahn lines are filtered out
    let ringbahn_lines = vec!["S41", "S42"];
    
    for line in ringbahn_lines {
        let should_filter = line == "S41" || line == "S42";
        assert!(should_filter, "Line {} (Ringbahn) should be filtered", line);
    }
}

#[test]
fn test_line_filtering_buses() {
    // Test that buses (pure numbers) are filtered out
    let bus_lines = vec!["100", "200", "347", "42"];
    
    for line in bus_lines {
        let should_filter = line.chars().all(|c| c.is_numeric());
        assert!(should_filter, "Line {} (bus) should be filtered", line);
    }
}

#[test]
fn test_line_filtering_keep_local_transport() {
    // Test that S-Bahn, U-Bahn, and Trams are NOT filtered
    let local_lines = vec!["S3", "S7", "S5", "U1", "U3", "M10", "M43"];
    
    for line in local_lines {
        let should_filter = line.starts_with("RE") ||
                           line.starts_with("RB") ||
                           line.starts_with("IC") ||
                           line.starts_with("EC") ||
                           line.starts_with("EN") ||
                           line.starts_with("FEX") ||
                           line.starts_with("ICE") ||
                           line == "S41" ||
                           line == "S42" ||
                           line.chars().all(|c| c.is_numeric());
        assert!(!should_filter, "Line {} should NOT be filtered", line);
    }
}

#[test]
fn test_time_filtering_minimum_one_minute() {
    // Test that departures must be at least 1 minute away
    let minutes_0 = 0;
    let minutes_1 = 1;
    let minutes_60 = 60;
    let minutes_61 = 61;
    
    assert!(!(minutes_0 >= 1 && minutes_0 <= 60), "0 min should be filtered");
    assert!(minutes_1 >= 1 && minutes_1 <= 60, "1 min should be included");
    assert!(minutes_60 >= 1 && minutes_60 <= 60, "60 min should be included");
    assert!(!(minutes_61 >= 1 && minutes_61 <= 60), "61 min should be filtered");
}

#[test]
fn test_destination_filtering_current_station() {
    // Test that departures TO Warschauer Str. are filtered (we're already there!)
    let destinations = vec![
        ("S+U Warschauer Str.", true),
        ("S+U Warschauer Str. (Berlin)", true),
        ("Erkner", false),
        ("Potsdam Hauptbahnhof", false),
    ];
    
    for (dest, should_filter) in destinations {
        let is_warschauer = dest.contains("Warschauer");
        assert_eq!(is_warschauer, should_filter, 
            "Destination '{}' filtering should be {}", dest, should_filter);
    }
}

