// Represents a single departure
#[derive(Debug, Clone, PartialEq)]
pub struct Departure {
    pub line: String,
    pub destination: String,
    pub minutes: u32,
}

impl Departure {
    pub fn new(line: impl Into<String>, destination: impl Into<String>, minutes: u32) -> Self {
        Self {
            line: line.into(),
            destination: destination.into(),
            minutes,
        }
    }

    // Format as "S3 Erkner 2 min"
    pub fn format(&self) -> String {
        format!("{} {} {} min", self.line, self.destination, self.minutes)
    }

    // Truncate destination to fit within max_chars
    pub fn format_truncated(&self, max_chars: usize) -> String {
        let formatted = self.format();
        if formatted.len() <= max_chars {
            return formatted;
        }

        // Calculate space needed for line, minutes, and formatting
        // Format: "LINE DEST X min"
        let min_text = format!(" {} min", self.minutes);
        let line_text = format!("{} ", self.line);
        let overhead = line_text.len() + min_text.len();

        if overhead >= max_chars {
            // Can't fit anything, return truncated version
            return formatted.chars().take(max_chars).collect();
        }

        let dest_max_len = max_chars - overhead;
        let truncated_dest: String = self.destination.chars().take(dest_max_len).collect();
        
        format!("{}{}{}", line_text, truncated_dest, min_text)
    }
}

/// Generate mock departure data for testing and static display
/// Returns multiple departures for cycling display
pub fn get_mock_departures() -> Vec<Departure> {
    vec![
        Departure::new("U3", "Krumme Lanke", 5),
        Departure::new("S7", "Potsdam Hbf", 8),
        Departure::new("S5", "Strausberg Nord", 2),
    ]
}


