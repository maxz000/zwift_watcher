use zwift_capture::Player;
use std::collections::HashMap;


pub const PLAYER_GROUP_CAPACITY: usize = 10;
const PLAYER_HISTORY_CAPACITY: usize = 50; // ~ 100ms per player update == 5sec history
const PLAYER_HISTORY_INTERPOLATION_MAX_TIME_DIFF: i64 = 100;
const MAX_WORLD_TIME_DIFF: i64 = 5000; // 5 sec

pub mod server;

#[derive(Debug,Default)]
struct PlayerHistory {
    // latest first
    data: Vec<Player>
}

impl PlayerHistory {
    pub fn new() -> Self {
        PlayerHistory {
            data: Vec::with_capacity(PLAYER_HISTORY_CAPACITY)
        }
    }

    pub fn from(new_player: Player) -> Self {
        let mut history = PlayerHistory {
            data: Vec::with_capacity(PLAYER_HISTORY_CAPACITY)
        };
        history.push(new_player);
        history
    }

    pub fn push(&mut self, new_player: Player) {
        let mut insert_index = 0;
        for (ix, player_data) in self.data.iter().enumerate() {
            if player_data.world_time > new_player.world_time {
                insert_index = ix + 1;
            } else {
                break;
            }
        }
        self.data.insert(insert_index, new_player);
        if self.data.len() > PLAYER_HISTORY_CAPACITY - 1 {
            self.data.pop();
        }
    }

    fn interpolate(&self, before: &Player, after: &Player, time: i64) -> Player {
        let time_delta = after.world_time - before.world_time;
        let requested_time_delta = time - before.world_time;
        let ratio = requested_time_delta as f64 / time_delta as f64;

        let mut player = before.clone();
        player.world_time = time;
        player.time = before.time + requested_time_delta as i32 / 1000;
        player.x = before.x + (after.x - before.x) * ratio;
        player.y = before.y + (after.y - before.y) * ratio;
        player.distance = before.distance + ((after.distance - before.distance) as f64 * ratio) as i32;

        player
    }

    fn find_nearest_known_points(&self, time: i64) -> (Option<&Player>, Option<&Player>) {
        let mut before: Option<&Player> = None;
        let mut after: Option<&Player> = None;

        for player in self.data.iter() {
            if player.world_time == time {
                return (Some(player), Some(player));
            }

            if player.world_time > time {
                after = Some(player);
            } else if player.world_time < time {
                before = Some(player);
                break;
            }
        }

        (before, after)
    }

    pub fn get_at_time(&self, time: i64) -> Option<Player> {

        let (before, after) = self.find_nearest_known_points(time);

        return match (before, after) {
            (Some(before), Some(after)) => {
                if before.world_time == after.world_time {
                    Some(before.clone())
                } else {
                    Some(self.interpolate(before, after, time))
                }
            },
            // show nearest available data if time difference not so big
            (Some(before), None) => {
                if before.world_time - time < PLAYER_HISTORY_INTERPOLATION_MAX_TIME_DIFF {
                    Some(before.clone())
                } else {
                    None
                }
            },
            (None, Some(after)) => {
                if time - after.world_time < PLAYER_HISTORY_INTERPOLATION_MAX_TIME_DIFF {
                    Some(after.clone())
                } else {
                    None
                }
            }
            _ => None
        }
    }
}


#[derive(Debug)]
pub struct PlayerData {
    pub id: i32,
    pub world_time: i64,
    history: PlayerHistory,
}

impl PlayerData {
    pub fn new(player: Player) -> Self {
        PlayerData {
            id: player.id,
            world_time: player.world_time,
            history: PlayerHistory::from(player),
        }
    }

    pub fn update(&mut self, player: Player) -> Result<i64, &str>{
        if self.id != player.id {
            return Err("Invalid player id");
        }

        if self.world_time < player.world_time {
            self.world_time = player.world_time;
        }

        self.history.push(player);

        Ok(self.world_time)
    }

    pub fn get_latest(&self) -> Option<Player> {
        self.history.get_at_time(self.world_time)
    }

    pub fn get_at_time(&self, time: i64) -> Option<Player> {
        self.history.get_at_time(time)
    }

}


#[derive(Debug,Clone,Default)]
pub struct PlayerGroup {
    players: Vec<i32>
}

impl PlayerGroup {
    pub fn new() -> Self {
        PlayerGroup {
            players: Vec::with_capacity(PLAYER_GROUP_CAPACITY)
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

    pub fn clear(&mut self) {
        self.players.clear();
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


#[derive(Debug,Default)]
pub struct World {
    pub world_time: i64,

    pub players_by_id: HashMap<i32, PlayerData>,
    pub groups_by_id: HashMap<i32, PlayerGroup>,
    pub group_to_watch: PlayerGroup
}

impl World {
    pub fn new() -> Self {
        World {
            world_time: 0,
            players_by_id: HashMap::new(),
            groups_by_id: HashMap::new(),
            group_to_watch: PlayerGroup::new()
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

    pub fn push_player(&mut self, player: Player) -> Option<i64> {
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

        Some(world_time)
    }

    pub fn push_players_batch(&mut self, players: Vec<Player>) -> Option<Vec<i64>> {
        let mut result = Vec::new();
        for player in players.into_iter() {
            if let Some(time) = self.push_player(player) {
                result.push(time);
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

    pub fn get_groups_list(&self) -> Option<Vec<i32>> {
        let keys: Vec<i32> = self.groups_by_id.keys().cloned().collect();
        Some(keys)
    }

    pub fn get_group(&self, group_id: i32) -> Option<&PlayerGroup> {
        self.groups_by_id.get(&group_id)
    }

    pub fn get_players_list(&self) -> Option<Vec<i32>> {
        let keys: Vec<i32> = self.players_by_id.keys().cloned().collect();
        Some(keys)
    }

    pub fn get_player_data(&self, player_id: i32) -> Option<&PlayerData> {
        self.players_by_id.get(&player_id)
    }

    pub fn get_latest_world_time_for_group(&self, group: &PlayerGroup) -> i64 {
        let mut min_time = self.world_time;
        for player_id in group.iter() {
            if let Some(player_data) = self.get_player_data(player_id) {
                if player_data.world_time < min_time {
                    min_time = player_data.world_time;
                }
            }
        }
        min_time
    }

    pub fn add_player_to_watch(&mut self, player_id: i32) {
        self.group_to_watch.add_player(player_id);
    }

    pub fn clear_group_to_watch(&mut self) {
        self.group_to_watch.clear();
    }

}


#[cfg(test)]
mod tests {

    use hex_literal::hex;
    use zwift_capture::ZwiftMessage;
    use super::*;

    fn get_player_instance() -> Player {
        // some sample captured data
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

    #[test]
    fn player_history_push() {
        let mut player_history = PlayerHistory::new();
        let base_player = get_player_instance();
        for x in (0..2000).step_by(100) {
            let mut player = base_player.clone();
            player.world_time = x;
            player.time = x as i32;
            player.distance = player.distance + (5 * x) as i32;
            player.x = player.x + (5 * x) as f64;
            player_history.push(player);
        }
        // test elements count
        assert_eq!(player_history.data.len(), PLAYER_HISTORY_CAPACITY - 1);
        // test order - latest first
        assert_eq!(player_history.data.iter().fold(i64::MAX, |accumulator, x| {
            if accumulator > x.world_time {
                x.world_time
            } else {
                i64::MIN
            }
        }), player_history.data[PLAYER_HISTORY_CAPACITY - 2].world_time);
    }

    #[test]
    fn player_history_push_unordered() {
        let mut player_history = PlayerHistory::new();
        let base_player = get_player_instance();
        for x in (0..2000).step_by(100) {
            let mut player = base_player.clone();
            player.world_time = x + x % 300;
            player.time = x as i32;
            player.distance = player.distance + (5 * x) as i32;
            player.x = player.x + (5 * x) as f64;
            player_history.push(player);
        }
        // test elements count
        assert_eq!(player_history.data.len(), PLAYER_HISTORY_CAPACITY - 1);
        // test order - latest first
        assert_eq!(player_history.data.iter().fold(i64::MAX, |accumulator, x| {
            if accumulator > x.world_time {
                x.world_time
            } else {
                i64::MIN
            }
        }), player_history.data[PLAYER_HISTORY_CAPACITY - 2].world_time);
    }

    #[test]
    fn player_history_get_at_time() {
        let mut player_history = PlayerHistory::new();
        let base_player = get_player_instance();
        let mut one = base_player.clone();
        let mut two = base_player.clone();
        one.world_time = 0;
        one.time = 0;
        one.x = 0.;
        one.distance = 100;
        two.world_time = 100;
        two.time = 100;
        two.x = 100.;
        two.distance = 200;
        player_history.push(one);
        player_history.push(two);
        let mid = player_history.get_at_time(50).unwrap();
        assert_eq!(mid.world_time, 50);
        assert_eq!(mid.time, 0);
        assert_eq!(mid.x, 50.);
        assert_eq!(mid.distance, 150);
    }
}
