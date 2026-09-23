pub mod api;
pub mod auth;
pub mod sync;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GoogleAuthStatus {
    pub is_signed_in: bool,
    pub email: Option<String>,
    pub client_id: Option<String>,
    pub has_client_secret: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GoogleCalendarItem {
    pub id: String,
    pub summary: String,
    pub description: Option<String>,
    pub primary: bool,
    pub background_color: String,
    pub foreground_color: String,
    pub is_visible: bool,
    pub is_synced: bool,
    pub is_events_synced: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GoogleOverlayEvent {
    pub id: String,
    pub calendar_id: String,
    pub calendar_summary: String,
    pub calendar_color: String,
    pub calendar_foreground_color: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub html_link: Option<String>,
    pub start_date_time: Option<String>,
    pub end_date_time: Option<String>,
    pub is_all_day: bool,
    pub day_of_week: i32, // 0 = Mon, 6 = Sun
    pub start_minute: i32,
    pub end_minute: i32,
}
