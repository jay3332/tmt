#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;

fn main() {
    let smc = smc::SMC::shared().unwrap();

    apple::all_sensors().unwrap().for_each(|sensor| {
        smc.temperature(sensor.key.into())
            .map(|temp| println!("{}: {} C", sensor.name, temp))
            .ok();
    });
}
