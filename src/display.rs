#[cfg(feature = "display")]
use crate::departure::Departure;
#[cfg(feature = "display")]
use embedded_graphics::{
    mono_font::{ascii::FONT_4X6, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    text::Text,
};
#[cfg(feature = "display")]
use rpi_led_matrix::{LedCanvas, LedColor, LedMatrix, LedMatrixOptions};

#[cfg(feature = "display")]
pub struct DisplayConfig {
    /// Matrix width in pixels
    pub width: u32,
    /// Matrix height in pixels
    pub height: u32,
    /// Hardware mapping (e.g., "regular", "adafruit-hat", etc.)
    pub hardware_mapping: String,
}

#[cfg(feature = "display")]
impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            width: 64,
            height: 32,
            hardware_mapping: "regular".to_string(),
        }
    }
}

#[cfg(feature = "display")]
pub struct BvgDisplay {
    matrix: LedMatrix,
    config: DisplayConfig,
    current_index: usize,
}

#[cfg(feature = "display")]
impl BvgDisplay {
    pub fn new() -> Result<Self, String> {
        Self::with_config(DisplayConfig::default())
    }

    pub fn with_config(config: DisplayConfig) -> Result<Self, String> {
        let mut options = LedMatrixOptions::new();
        options.set_cols(config.width);
        options.set_rows(config.height);
        options.set_hardware_mapping(&config.hardware_mapping);

        let matrix = LedMatrix::new(Some(options), None)
            .map_err(|e| format!("Failed to initialize LED matrix: {}", e))?;

        Ok(Self { 
            matrix, 
            config,
            current_index: 0,
        })
    }

    /// Render departures to the LED matrix
    /// Displays 1 departure on 3 lines with smart word wrapping
    pub fn render_departures(&mut self, departures: &[Departure]) {
        let mut canvas = self.matrix.offscreen_canvas();
        
        // Clear the canvas (black background)
        canvas.fill(&LedColor { red: 0, green: 0, blue: 0 });

        // BVG yellow/amber color scheme
        let text_color = LedColor {
            red: 255,
            green: 200,
            blue: 0,
        };

        // Three-line format for one departure with smart wrapping
        let line_height = 9;   // Height between lines
        let start_y = 5;       // Top padding
        let max_width = 16;    // Max chars per line

        // Display current departure (cycling through list)
        if let Some(departure) = departures.get(self.current_index) {
            // Smart wrap: LINE + DESTINATION across multiple lines
            let mut full_text = String::with_capacity(departure.line.len() + departure.destination.len() + 1);
            full_text.push_str(&departure.line);
            full_text.push(' ');
            full_text.push_str(&departure.destination);
            let lines = self.smart_wrap(&full_text, max_width, 2); // max 2 lines for destination
            
            // Draw destination lines (skip empty lines)
            let mut last_line_index = 0;
            for (i, line) in lines.iter().enumerate() {
                if !line.is_empty() {
                    let y_pos = start_y + (i as i32 * line_height);
                    self.draw_text(&mut canvas, line, 2, y_pos, text_color);
                    last_line_index = i;
                }
            }
            
            // Time on the next line after last destination line
            let time_text = format!("{} min", departure.minutes);
            let time_y = start_y + ((last_line_index + 1) as i32 * line_height);
            self.draw_text(&mut canvas, &time_text, 2, time_y, text_color);
            
            drop(time_text);
            drop(full_text);
            drop(lines);
        }

        // Swap canvas to display
        let old_canvas = self.matrix.swap(canvas);
        drop(old_canvas);
    }
    
    /// Move to next departure in the list (cycle)
    pub fn next_departure(&mut self, total: usize) {
        self.current_index = (self.current_index + 1) % total;
    }
    
    pub fn current_index(&self) -> usize {
        self.current_index
    }
    
    /// Smart word wrapping - breaks text by spaces to fit within max_width
    fn smart_wrap(&self, text: &str, max_width: usize, max_lines: usize) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::with_capacity(max_lines);
        let mut current_line = String::with_capacity(max_width);
        
        for word in words {
            let test_len = if current_line.is_empty() {
                word.len()
            } else {
                current_line.len() + 1 + word.len()
            };
            
            if test_len <= max_width {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            } else {
                // Current line is full, start new line
                if !current_line.is_empty() {
                    lines.push(std::mem::take(&mut current_line));
                }
                
                if lines.len() >= max_lines {
                    break;
                }
                
                if word.len() > max_width {
                    current_line = word.chars().take(max_width).collect();
                } else {
                    current_line = word.to_string();
                }
            }
        }
        
        // Add remaining text
        if !current_line.is_empty() && lines.len() < max_lines {
            lines.push(current_line);
        } else if current_line.is_empty() {
            drop(current_line);
        }
        
        // Pad with empty lines if needed (avoid resize to prevent allocations)
        while lines.len() < max_lines {
            lines.push(String::new());
        }
        
        lines
    }

    /// Draw text on the canvas at specified position
    fn draw_text(&self, canvas: &mut LedCanvas, text: &str, x: i32, y: i32, color: LedColor) {
        // Convert LedColor to Rgb888 for embedded-graphics
        let rgb_color = Rgb888::new(color.red, color.green, color.blue);
        let style = MonoTextStyle::new(&FONT_4X6, rgb_color);

        // Create text with position and style
        let text_drawable = Text::new(text, Point::new(x, y), style);
        
        // Draw to canvas (using embedded-graphics integration)
        let _ = text_drawable.draw(canvas);
        
        drop(text_drawable);
        drop(style);
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }
}


