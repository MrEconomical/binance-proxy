// Imports

use std::time::SystemTime;
use std::time::UNIX_EPOCH;

// Get current timestamp

pub fn get_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}
