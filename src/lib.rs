pub mod departure;
pub mod api;

#[cfg(feature = "display")]
pub mod display;

pub use departure::{Departure, get_mock_departures};
pub use api::BvgApiClient;

#[cfg(feature = "display")]
pub use display::{BvgDisplay, DisplayConfig};


