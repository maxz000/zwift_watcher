use std::sync::{Arc, Mutex};
use std::io::stdin;
use std::thread;
use std::time;
use std::path;
use warp::Filter;
use zwift_capture::{Player,ZwiftCapture};
use zwift_watcher::{World,PlayerGroup,PlayerData,PLAYER_GROUP_CAPACITY};
use std::iter::FromIterator;
use pcap::Device;


const TICK: i64 = 1000;

mod models {
    use serde::{Serialize,Deserialize};

    #[derive(Debug,Deserialize,Serialize,Clone)]
    pub struct PLayerQuery {
        pub id: i32
    }

    #[derive(Debug,Deserialize,Serialize,Clone)]
    pub struct WatchOptions {
        pub latest: Option<String>
    }
}

mod handlers {
    use super::*;

    pub async fn api_root(world: Arc<Mutex<World>>) -> Result<impl warp::Reply, warp::Rejection> {
        let world = world.lock().unwrap();
        Ok(warp::reply::json(&serde_json::json!({
            "result": "ok",
            "data": {
                "world_time": world.world_time,
                "group_to_watch": Vec::<i32>::from_iter(world.group_to_watch.iter())
            }
        })))
    }

    pub async fn get_group_to_watch(options: models::WatchOptions, world: Arc<Mutex<World>>) -> Result<impl warp::Reply, warp::Rejection> {
        let world = world.lock().unwrap();
        let mut result = Vec::with_capacity(PLAYER_GROUP_CAPACITY);
        let latest_time = match &options.latest {
            Some(_) => world.world_time,
            _ => world.get_latest_world_time_for_group(&world.group_to_watch)
        };
        for player_id in world.group_to_watch.iter() {
            if let Some(player) = world.get_player_data(player_id) {
                if let Some(data) = match &options.latest {
                    Some(_) => player.get_latest(),
                    _ => player.get_at_time(latest_time)
                } {
                    result.push(data);
                }
            }
        }
        Ok(warp::reply::json(&serde_json::json!({
            "result": "ok",
            "data": result
        })))
    }

    pub async fn add_player_to_watch(player: models::PLayerQuery, world: Arc<Mutex<World>>) -> Result<impl warp::Reply, warp::Rejection> {
        let mut world = world.lock().unwrap();
        world.add_player_to_watch(player.id);
        Ok(warp::reply::json(&serde_json::json!({
            "result": "ok",
            "data": {
                "id": player.id
            }
        })))
    }

    pub async fn clear_group_to_watch(world: Arc<Mutex<World>>) -> Result<impl warp::Reply, warp::Rejection> {
        let mut world = world.lock().unwrap();
        world.clear_group_to_watch();
        Ok(warp::reply::json(&serde_json::json!({
            "result": "ok",
            "data": {}
        })))
    }

}


#[tokio::main]
async fn main() {
    println!("Start!");
    let world = Arc::new(Mutex::new(World::new()));
    let world_capture = world.clone();
    let world_filter = warp::any().map(move || world.clone());

    // real capture device
    let mut devices_list = Device::list().unwrap();
    for (ix, device) in devices_list.iter().enumerate() {
        let desc = match &device.desc {
            Some(v) => v.clone(),
            _ => String::from("---")
        };
        println!("{}: {} {:?}", ix, &device.name ,desc);
    }
    println!("\nPlease choose device:");
    let mut input_str = String::new();
    stdin().read_line(&mut input_str).expect("invalid value");
    let choice: usize = input_str.trim().parse().unwrap();
    let selected_device = devices_list.remove(choice);

    println!("Selected device: {:?}", selected_device);
    let capture = ZwiftCapture::from_device(selected_device);

    // local test file
    // let capture = ZwiftCapture::from_file(path::Path::new("zwift_meetup.pcapng"));

    let capture_thread = thread::spawn(move || {
        let mut counter: i64 = 0;
        println!("Capture thread: start");

        for players in capture {
            let mut world_capture = world_capture.lock().unwrap();
            let _times = world_capture.push_players_batch(players).unwrap();

            // only for local file
            // thread::sleep(time::Duration::from_millis(100));

            counter += 1;
            if counter % TICK == 0 {
                println!("Tick {}, time: {}", counter, world_capture.world_time);
                if let Some(outdated) = world_capture.find_outdated_players() {
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

    let routes = root_url.or(get_group_to_watch_url)
        .or(add_player_url).or(clear_group_to_watch_url);
    let _ = warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    let _ =capture_thread.join();
    println!("End!");
}
