use chrono::prelude::DateTime;
use chrono::Utc;
use pcap::Device;
use std::io::stdin;
use std::iter::FromIterator;
use std::path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use warp::Filter;
use zwift_capture::{Player, ZwiftCapture};
use zwift_watcher::server::{handlers, models, Routes};
use zwift_watcher::{PlayerData, PlayerGroup, World, PLAYER_GROUP_CAPACITY};

const TICK: i64 = 1000;

#[tokio::main]
async fn main() {
    println!("Start!");
    let world = Arc::new(Mutex::new(World::new()));
    let world_capture = world.clone();

    let capture_thread = thread::spawn(move || {
        let mut counter: i64 = 0;
        println!("Capture thread: start");
        // real capture device
        let mut devices_list = Device::list().unwrap();
        for (ix, device) in devices_list.iter().enumerate() {
            let desc = match &device.desc {
                Some(v) => v.clone(),
                _ => String::from("---"),
            };
            println!("{}: {} {:?}", ix, &device.name, desc);
        }
        println!("\nPlease choose device:");
        let mut input_str = String::new();
        stdin().read_line(&mut input_str).expect("invalid value");
        let choice: usize = input_str.trim().parse().unwrap();
        let selected_device = devices_list.remove(choice);

        loop {
            println!("Open device: {:?}", &selected_device);
            let mut capture = ZwiftCapture::from_device(selected_device.clone());
            for players in &mut capture {
                // .skip(20000) {
                let mut world_capture = world_capture.lock().unwrap();
                let _times = world_capture.push_players_batch(players).unwrap();

                counter += 1;
                if counter % TICK == 0 {
                    let st = UNIX_EPOCH + Duration::from_millis(world_capture.world_time as u64);
                    let datetime = DateTime::<Utc>::from(st);
                    println!(
                        "Tick {}, time: [{}] {}",
                        counter,
                        world_capture.world_time,
                        datetime.format("%Y-%m-%d %H:%M:%S.%f").to_string()
                    );
                    if let Some(outdated) = world_capture.find_outdated_players() {
                        println!("Outdated players: {}", outdated.len());
                        outdated
                            .iter()
                            .for_each(|&id| world_capture.clear_player(id))
                    }
                }
            }
            capture.print_stat();
            println!("Close device");
        }
        println!("Capture thread: done")
    });

    let routes = Routes::new(world).generate();
    let _ = warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    let _ = capture_thread.join();
    println!("End!");
}
