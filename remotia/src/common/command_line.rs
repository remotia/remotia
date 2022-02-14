use std::str::FromStr;

pub fn parse_canvas_resolution_str(arg: &String) -> (u32, u32) {
    let canvas_resolution_split: Vec<&str> = arg.split("x").collect();

    let width_str = canvas_resolution_split[0];
    let height_str = canvas_resolution_split[1];

    let canvas_width: u32 = u32::from_str(width_str)
        .unwrap_or_else(|e| panic!("Unable to parse width '{}': {}", width_str, e));

    let canvas_height: u32 = u32::from_str(height_str)
        .unwrap_or_else(|e| panic!("Unable to parse height '{}': {}", height_str, e));

    (canvas_width, canvas_height)
}