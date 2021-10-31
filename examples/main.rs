use std::sync::{Arc, Mutex};
use std::io::stdin;
use std::thread;
use std::time;
use std::path;
use warp::Filter;
use zwift_capture::{Player, ZwiftCapture};
use zwift_watcher::{World, PlayerGroup, PlayerData, PLAYER_GROUP_CAPACITY};
use zwift_watcher::server::{handlers, models, Routes};
use std::iter::FromIterator;
use pcap::Device;


const TICK: i64 = 1000;


#[tokio::main]
async fn main() {
    println!("Start!");
    let world = Arc::new(Mutex::new(World::new()));
    let world_capture = world.clone();

    // local test file
    let capture = ZwiftCapture::from_file(path::Path::new("ws.pcapng"));

    let capture_thread = thread::spawn(move || {
        let mut counter: i64 = 0;
        println!("Capture thread: start");

        for players in capture { // .skip(20000) {
            let mut world_capture = world_capture.lock().unwrap();
            let _times = world_capture.push_players_batch(players).unwrap();

            // only for local file
            thread::sleep(time::Duration::from_millis(100));

            counter += 1;
            if counter % TICK == 0 {
                println!("Tick {}, time: {}", counter, world_capture.world_time);
                if let Some(outdated) = world_capture.find_outdated_players() {
                    println!("Outdated players: {}", outdated.len());
                    outdated.iter().for_each(|&id| world_capture.clear_player(id))
                }
            }
        }
        println!("Capture thread: done")
    });

    let routes = Routes::new(world).generate();
    let _ = warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    let _ = capture_thread.join();
    println!("End!");
}
