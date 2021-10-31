

pub mod models {
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct PLayerQuery {
        pub id: i32
    }

    #[derive(Debug, Deserialize, Serialize, Clone)]
    pub struct WatchOptions {
        pub latest: Option<String>
    }
}

pub mod handlers {
    use std::sync::{Arc, Mutex};
    use std::iter::FromIterator;

    use super::models;
    use crate::{World,PLAYER_GROUP_CAPACITY};

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

    pub async fn world_users(world: Arc<Mutex<World>>) -> Result<impl warp::Reply, warp::Rejection> {
        let world = world.lock().unwrap();
        Ok(warp::reply::json(&serde_json::json!({
            "result": "ok",
            "data": {
                "world_time": world.world_time,
                "users": Vec::<i32>::from_iter(world.players_by_id.keys().cloned())
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