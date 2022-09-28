#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;

fn main() {
    println!("{:?}", apple::get_all_sensors());
}
