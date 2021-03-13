use zwift_capture::Player;
use std::collections::HashMap;

pub mod player_path;


const MAX_WORLD_TIME_DIFF: i64 = 5000; // 5 sec


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

    pub fn motion_vector(&self, waypoint: player_path::WayPoint) -> Option<Vec<f64>> {
        self.path.motion_vector_for_waypoint(&waypoint)
    }

}


pub struct PlayerGroup {
    players: Vec<i32>
}

impl PlayerGroup {
    pub fn new() -> Self {
        PlayerGroup {
            players: Vec::with_capacity(10)
        }
    }

    pub fn from(users: &[i32]) -> Self {
        PlayerGroup {
            players: Vec::from(users)
        }
    }

    pub fn iter(&self) -> PlayerGroupIter {
        PlayerGroupIter {
            players: self.players.clone(),
            index: 0
        }
    }

    pub fn add_player(&mut self, user_id: i32) {
        if !self.players.contains(&user_id) {
            self.players.push(user_id);
        }
    }

    pub fn remove_player(&mut self, user_id: i32) {
        if let Some(index) = self.players.iter().position(|&x| x == user_id) {
            self.players.remove(index);
        }
    }

    pub fn has_player(&self, user_id: i32) -> bool {
        self.players.contains(&user_id)
    }
}

pub struct PlayerGroupIter {
    players: Vec<i32>,
    index: usize
}

impl Iterator for PlayerGroupIter {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.players.len() {
            self.index += 1;
            return Some(self.players[self.index - 1]);
        }
        None
    }
}


pub struct World {
    pub world_time: i64,

    pub players_by_id: HashMap<i32, PlayerData>,
    pub groups_by_id: HashMap<i32, PlayerGroup>
}

impl World {
    pub fn new() -> Self {
        World {
            world_time: 0,
            players_by_id: HashMap::new(),
            groups_by_id: HashMap::new()
        }
    }

    fn update_world_time(&mut self, new_time: i64) {
        if self.world_time < new_time {
            self.world_time = new_time;
        }
    }

    fn update_players_group(&mut self, group_id: i32, player_id: i32) {
        if let Some(group) = self.groups_by_id.get_mut(&group_id) {
            group.add_player(player_id);
        } else {
            self.groups_by_id.insert(group_id, PlayerGroup::from(&[player_id]));
        }

        for (&key, group) in self.groups_by_id.iter_mut() {
            if key == group_id { continue }
            group.remove_player(player_id);
        }
    }

    pub fn push_player(&mut self, player: Player) -> Option<i32> {
        let player_id = player.id;
        let group_id = player.group_id;
        let world_time = player.world_time;

        if let Some(player_data) = self.players_by_id.get_mut(&player.id) {
            let _ = player_data.update(player);
        } else {
            let player_data = PlayerData::new(player);
            self.players_by_id.insert(player_id, player_data);
        }

        self.update_players_group(group_id, player_id);
        self.update_world_time(world_time);

        Some(player_id)
    }

    pub fn push_players_batch(&mut self, players: Vec<Player>) -> Option<Vec<i32>> {
        let mut result = Vec::new();
        for player in players.into_iter() {
            if let Some(player_id) = self.push_player(player) {
                result.push(player_id);
            }
        }
        Some(result)
    }

    pub fn clear_player(&mut self, player_id: i32) {
        for (_, group) in self.groups_by_id.iter_mut() {
            group.remove_player(player_id);
        }
        if self.players_by_id.contains_key(&player_id) {
            self.players_by_id.remove(&player_id);
        }
    }

    pub fn find_outdated_players(&mut self) -> Option<Vec<i32>> {
        let mut result = Vec::new();
        for (&player_id, player_data) in self.players_by_id.iter_mut() {
            if self.world_time - player_data.world_time > MAX_WORLD_TIME_DIFF {
                result.push(player_id);
            }
        }
        Some(result)
    }
}


#[cfg(test)]
mod tests {

    use hex_literal::hex;
    use zwift_capture::ZwiftMessage;
    use super::*;

    fn get_player_instance() -> Player {
        let packet_payload = hex!("0686a9010008011086d30618e1a6fbcce80520ab023a6e0886d30610e1a6fbcce8051800208fac3a2800300040f4fa860548005000584f600068cbd5aa0170c0843d7800800100980195809808a0018f808008a80100b80100c00100cd01ae378847d50119191a46dd01a0d52ec7e00186d306e80100f80100950200000000980206b002001f403176");
        let message = ZwiftMessage::ToServer(&packet_payload);
        let mut players = message.get_players().unwrap();
        players.pop().unwrap()
    }

    #[test]
    fn world_push_player() {
        let mut world = World::new();
        let player = get_player_instance();
        let player_two = get_player_instance();
        world.push_player(player);
        assert_eq!(world.world_time, player_two.world_time);
    }

    #[test]
    fn world_clear_player() {
        let mut world = World::new();
        world.push_player(get_player_instance());
        world.update_world_time(world.world_time + 1 + MAX_WORLD_TIME_DIFF);
        assert_eq!(world.find_outdated_players().unwrap().len(), 1);
    }

    #[test]
    fn user_group_iter() {
        let mut group = PlayerGroup::new();
        group.add_player(0);
        group.add_player(1);
        let mut a = 0;
        for x in group.iter() {
            assert_eq!(x, a);
            a += 1;
        }
    }
}
