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
use structopt::StructOpt;
use chrono::prelude::DateTime;
use std::time::{Duration,UNIX_EPOCH,SystemTime};
use chrono::Utc;


const TICK: i64 = 1000;


#[derive(StructOpt,Debug,Clone)]
struct Cli {
    dump_file: String,
    sleep: u64
}


#[tokio::main]
async fn main() {
    println!("Start!");

    let args = Cli::from_args();

    let world = Arc::new(Mutex::new(World::new()));
    let world_capture = world.clone();

    println!("Selected file: {:?}", &args.dump_file);
    println!("Wait time: {:?}", &args.sleep);
    // local test file
    let mut capture = ZwiftCapture::from_file(path::Path::new(&args.dump_file));

    let capture_thread = thread::spawn(move || {
        let mut counter: i64 = 0;
        println!("Capture thread: start");

        for players in &mut capture { // .skip(20000) {
            let mut world_capture = world_capture.lock().unwrap();
            let _times = world_capture.push_players_batch(players).unwrap();

            // only for local file
            thread::sleep(time::Duration::from_millis(args.sleep));

            counter += 1;
            if counter % TICK == 0 {
                let st = UNIX_EPOCH + Duration::from_millis(world_capture.world_time as u64);
                let datetime = DateTime::<Utc>::from(st);
                println!("Tick {}, time: [{}] {}", counter, world_capture.world_time, datetime.format("%Y-%m-%d %H:%M:%S.%f").to_string());
                if let Some(outdated) = world_capture.find_outdated_players() {
                    println!("Outdated players: {}", outdated.len());
                    outdated.iter().for_each(|&id| world_capture.clear_player(id))
                }
            }
        }
        capture.print_stat();
        println!("Capture thread: done")
    });

    let routes = Routes::new(world).generate();
    let _ = warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    let _ = capture_thread.join();
    println!("End!");
}
