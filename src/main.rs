#[cfg(target_os = "macos")]
mod apple;

fn main() {
    let smc = smc::SMC::shared().unwrap();

    apple::all_sensors().unwrap().for_each(|sensor| {
        smc.temperature(sensor.key.into())
            .map(|temp| println!("{}: {} C", sensor.name, temp))
            .ok();
    });
}
