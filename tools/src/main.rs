mod tools_utils;

use crate::tools_utils::convert_videos_to_wav;

fn main() {
    if let Err(e) = convert_videos_to_wav() {
        eprintln!("Error: {}", e);
    }
}
