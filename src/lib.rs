use zwift_capture::Player;
use std::collections::HashMap;

pub mod player_path;

pub struct PlayerData {
    pub id: i32,
    pub world_time: i64,
    data: Player,
    path: player_path::Path
}

impl PlayerData {
    pub fn new(player: Player) -> Self {
        let mut path = player_path::Path::new();
        path.push(player_path::WayPoint::from(&player));

        PlayerData {
            id: player.id,
            world_time: player.world_time,
            data: player,
            path: path
        }
    }

    pub fn update(&mut self, player: Player) -> Result<i64, &str>{
        if self.id != player.id {
            return Err("Invalid player id");
        }

        self.path.push(player_path::WayPoint::from(&player));

        if self.world_time < player.world_time {
            self.world_time = player.world_time;
            self.data = player;
        }
        Ok(self.world_time)
    }

    pub fn get(&self) -> &Player {
        &self.data
    }

    pub fn position_at_time(&self, time: i64) -> Option<player_path::WayPoint> {
        self.path.position_at_time(time)
    }

}


pub struct World {
    pub world_time: i64,
    pub players_by_id: HashMap<i32, PlayerData>,
    pub groups_by_id: HashMap<i32, Vec<i32>>
}

impl World {
    pub fn new() -> Self {
        World {
            world_time: 0,
            players_by_id: HashMap::new(),
            groups_by_id: HashMap::new()
        }
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
