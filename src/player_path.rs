use zwift_capture::Player;


const PATH_CAPACITY: usize = 20;
const MOTION_VECTOR_TIME_DIFF: i64 = 2000; // 2 sec


#[derive(Debug)]
pub struct WayPoint {
    pub time: i64,
    pub x: f64,
    pub y: f64
}

impl WayPoint {
    pub fn new(x: f64, y: f64, time: i64) -> Self {
        WayPoint {
            time,
            x,
            y
        }
    }

    pub fn from(player: &Player) -> Self {
        WayPoint {
            time: player.world_time,
            x: player.x as f64,
            y: player.y as f64
        }
    }

    pub fn interpolate(from: &WayPoint, to: &WayPoint, time: i64) -> Option<Self> {
        if from.time > time || to.time < time {
            return None;
        }

        let time_delta = to.time - from.time;
        let requested_time_delta = time - from.time;
        let ratio = requested_time_delta as f64 / time_delta as f64;

        let delta_x = (to.x - from.x) * ratio;
        let delta_y = (to.y - from.y) * ratio;

        Some(WayPoint {
            time,
            x: from.x + delta_x,
            y: from.y + delta_y
        })
    }

    pub fn get_motion_vector(from: &WayPoint, to: &WayPoint) -> Vec<f64> {
        let motion_vec = vec![to.x - from.x, to.y - from.y];
        let length = (motion_vec[0].powi(2) + motion_vec[1].powi(2)).sqrt();
        vec![motion_vec[0] / length, motion_vec[1] / length]
    }

    pub fn calculate_distance(&self, other: &WayPoint) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

impl Clone for WayPoint {
    fn clone(&self) -> Self {
        WayPoint::new(self.x, self.y, self.time)
    }
}


pub struct Path {
    time: i64,
    path: Vec<WayPoint>
}

impl Path {
    pub fn new() -> Self {
        Path {
            time: 0,
            path: Vec::with_capacity(PATH_CAPACITY)
        }
    }

    pub fn push(&mut self, new: WayPoint) {

        if self.time < new.time {
            self.time = new.time
        }

        let mut insert_index: usize = 0;
        for (ix, point) in self.path.iter().enumerate() {
            if point.time > new.time {
                insert_index = ix + 1;
            } else {
                break;
            }
        }
        self.path.insert(insert_index, new);
        if self.path.len() > PATH_CAPACITY - 2 {
            self.path.pop();
        }

    }

    pub fn position_at_time(&self, time: i64) -> Option<WayPoint> {

        let mut before: Option<&WayPoint> = None;
        let mut after: Option<&WayPoint> = None;

        for waypoint in self.path.iter() {
            if waypoint.time == time {
                return Some(waypoint.clone());
            }

            if waypoint.time > time {
                after = Some(waypoint);
            } else if waypoint.time < time {
                before = Some(waypoint);
                break;
            }
        }

        if let Some(before) = before {
            if let Some(after) = after {
                return WayPoint::interpolate(before, after, time);
            }
        }
        None
    }

    pub fn motion_vector_for_waypoint(&self, waypoint: &WayPoint) -> Option<Vec<f64>> {
        for prev_point in self.path.iter() {
            if waypoint.time - prev_point.time > MOTION_VECTOR_TIME_DIFF {
                return Some(WayPoint::get_motion_vector(prev_point, waypoint));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{Path,WayPoint};
    use crate::player_path::PATH_CAPACITY;

    #[test]
    fn waypoint_interpolation() {
        let from = WayPoint::new(0., 0., 0);
        let to = WayPoint::new(2., 2., 2);
        let target = WayPoint::interpolate(&from, &to, 1).unwrap();
        assert!((target.x - 1.).abs() < 0.0001);
        assert!((target.y - 1.).abs() < 0.0001);
        assert_eq!(target.time, 1);
    }

    #[test]
    fn waypoint_distance() {
        let one = WayPoint::new(f32::MAX as f64, f32::MIN as f64, 0);
        let two = WayPoint::new(f32::MIN as f64, f32::MAX as f64, 0);
        let distance = one.calculate_distance(&two);
    }

    #[test]
    fn path_push() {
        let mut path = Path::new();
        for x in 0..20 {
            let point = WayPoint::new(x as f64 * 100., 0., x);
            path.push(point);
        }
        assert_eq!(path.path.len(), PATH_CAPACITY - 2);
        // check if all path items sorted by time
        assert_eq!(path.path.iter().fold(i64::MAX, |accumulator, x| {
            if accumulator > x.time {
                x.time
            } else {
                i64::MIN
            }
        }), path.path[PATH_CAPACITY - 3].time);
    }

    #[test]
    fn path_push_unordered() {
        let mut path = Path::new();
        for x in 0..20 {
            let point = WayPoint::new(x as f64 * 100., 0., x + x % 3);
            path.push(point);
        }
        // check if all path items sorted by time
        assert_eq!(path.path.iter().fold(i64::MAX, |accumulator, x| {
            if accumulator > x.time {
                x.time
            } else {
                i64::MIN
            }
        }), path.path[PATH_CAPACITY - 3].time);
    }

    #[test]
    fn path_position_at() {
        let mut path = Path::new();
        for x in 0..10 {
            let point = WayPoint::new(x as f64 * 200., 0., x * 2);
            path.push(point);
        }
        assert!(path.position_at_time(3).unwrap().x - 300. < 0.0001);
        assert_eq!(path.position_at_time(4).unwrap().x, 400.);
    }

}