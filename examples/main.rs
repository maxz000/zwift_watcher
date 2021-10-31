use std::sync::{Arc, Mutex};
use std::io::stdin;
use std::thread;
use std::time;
use std::path;
use warp::Filter;
use zwift_capture::{Player, ZwiftCapture};
use zwift_watcher::{World, PlayerGroup, PlayerData, PLAYER_GROUP_CAPACITY};
use zwift_watcher::server::{handlers,models};
use std::iter::FromIterator;
use pcap::Device;


const TICK: i64 = 1000;


#[tokio::main]
async fn main() {
    println!("Start!");
    let world = Arc::new(Mutex::new(World::new()));
    let world_capture = world.clone();
    let world_filter = warp::any().map(move || world.clone());

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

    let root_url = warp::path::end()
        .and(world_filter.clone())
        .and_then(handlers::api_root);

    let get_group_to_watch_url = warp::get()
        .and(warp::path("watch"))
        .and(warp::path::end())
        .and(warp::query::<models::WatchOptions>())
        .and(world_filter.clone())
        .and_then(handlers::get_group_to_watch);

    let add_player_url = warp::post()
        .and(warp::path("watch"))
        .and(warp::path("add"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(world_filter.clone())
        .and_then(handlers::add_player_to_watch);

    let clear_group_to_watch_url = warp::delete()
        .and(warp::path("watch"))
        .and(warp::path("clear"))
        .and(warp::path::end())
        .and(world_filter.clone())
        .and_then(handlers::clear_group_to_watch);

    let world_users_url = warp::get()
        .and(warp::path("users"))
        .and(warp::path::end())
        .and(world_filter.clone())
        .and_then(handlers::world_users);

    let routes = root_url.or(get_group_to_watch_url)
        .or(add_player_url).or(clear_group_to_watch_url)
        .or(world_users_url);
    let _ = warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    let _ = capture_thread.join();
    println!("End!");
}
