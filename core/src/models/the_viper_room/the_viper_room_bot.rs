#[derive(Debug, Clone, Copy)]
pub enum MainMenuMessageType {
    Full,
    Minimal,
}

pub fn normalize_channel_id(id: i64) -> i64 {
    // Check if ID is in full format (negative with -100 prefix)
    // The -100 prefix means the number is less than -10^12
    if id < -1_000_000_000_000 {
        // Strip the -100 prefix: abs(id) - 10^12
        id.abs() - 1_000_000_000_000
    } else {
        // Already in raw format or positive - just ensure it's positive
        id.abs()
    }
}
