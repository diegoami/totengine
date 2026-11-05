//! Engine-designed seeded surface worlds (#90), algorithm version 1.
//!
//! This is not a port of the original generator. Integer smoothing, ranked
//! sea level and latitude/moisture biomes are engine policy. `.MP` export
//! uses the already decoded stock slot layout; original-game import is not
//! claimed by a successful round trip through our format decoder.
use crate::rng::{GameRng, OrigRng};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldConfig {
    /// Logical diamond-grid width (twice the stored width).
    pub width: u16,
    pub height: u16,
    pub seed: u32,
    /// Requested percentage of real tiles above sea level.
    pub land_percent: u8,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            width: 80,
            height: 50,
            seed: 1,
            land_percent: 47,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedWorld {
    pub config: WorldConfig,
    /// Dense rows, indexed as y * (width / 2) + x / 2, as in saves.
    pub terrain: Vec<u8>,
    pub river: Vec<bool>,
}

const DIRECTIONS: [(i32, i32); 8] = [
    (1, -1),
    (2, 0),
    (1, 1),
    (0, 2),
    (-1, 1),
    (-2, 0),
    (-1, -1),
    (0, -2),
];

fn neighbors(config: WorldConfig, index: usize) -> impl Iterator<Item = usize> {
    let width = i32::from(config.width);
    let stride = width / 2;
    let y = index as i32 / stride;
    let x = 2 * (index as i32 % stride) + (y & 1);
    DIRECTIONS.into_iter().filter_map(move |(dx, dy)| {
        let ny = y + dy;
        (ny >= 0 && ny < i32::from(config.height))
            .then(|| (ny * stride + (x + dx).rem_euclid(width) / 2) as usize)
    })
}

fn field(config: WorldConfig, rng: &mut OrigRng, passes: usize) -> Vec<u32> {
    let count = usize::from(config.width / 2) * usize::from(config.height);
    let mut values: Vec<u32> = (0..count).map(|_| rng.next_u32() >> 16).collect();
    for _ in 0..passes {
        values = (0..count)
            .map(|i| {
                let mut sum = 2 * values[i];
                let mut n = 2;
                for j in neighbors(config, i) {
                    sum += values[j];
                    n += 1;
                }
                sum / n
            })
            .collect();
    }
    let lo = *values.iter().min().unwrap();
    let hi = *values.iter().max().unwrap();
    values
        .iter_mut()
        .for_each(|v| *v = (*v - lo) * 1000 / (hi - lo).max(1));
    values
}

pub fn generate(config: WorldConfig) -> Result<GeneratedWorld, String> {
    if !(32..=256).contains(&config.width)
        || !config.width.is_multiple_of(2)
        || !(16..=256).contains(&config.height)
        || !(10..=80).contains(&config.land_percent)
    {
        return Err(
            "world requires even width 32..256, height 16..256, land percentage 10..80".into(),
        );
    }
    let mut rng = OrigRng::new(config.seed);
    let elevation = field(config, &mut rng, 12);
    let moisture = field(config, &mut rng, 5);
    let relief = field(config, &mut rng, 2);
    let count = elevation.len();
    let land_count = (count * usize::from(config.land_percent) + 50) / 100;
    let mut order: Vec<usize> = (0..count).collect();
    order.sort_by_key(|&i| (elevation[i], i));
    let mut terrain = vec![10; count];
    for &i in &order[count - land_count..] {
        let y = i / usize::from(config.width / 2);
        let latitude = (2 * y as i32 + 1 - i32::from(config.height)).unsigned_abs() * 1000
            / u32::from(config.height);
        terrain[i] = if latitude > 880 {
            7
        } else if latitude > 700 {
            6
        } else if relief[i] > 850 {
            5
        } else if relief[i] > 690 {
            4
        } else if latitude < 350 && moisture[i] > 600 {
            9
        } else if moisture[i] > 760 {
            8
        } else if moisture[i] > 520 {
            3
        } else if moisture[i] < 300 && latitude < 650 {
            0
        } else if moisture[i] > 380 {
            2
        } else {
            1
        };
    }
    // Strictly decreasing (elevation,index) makes drainage acyclic, including
    // across the east/west seam. Inland sinks do not receive a fabricated outlet.
    let mut river = vec![false; count];
    for &i in &order[count - land_count..] {
        if moisture[i] < 600 || relief[i] < 650 || rng.next_below(16) != 0 {
            continue;
        }
        let mut path = vec![i];
        let mut at = i;
        let mut drains = false;
        for _ in 0..count {
            let Some(next) = neighbors(config, at)
                .filter(|&j| (elevation[j], j) < (elevation[at], at))
                .min_by_key(|&j| (elevation[j], j))
            else {
                break;
            };
            if terrain[next] == 10 || river[next] {
                drains = true;
                break;
            }
            path.push(next);
            at = next;
        }
        if drains && path.len() >= 2 {
            for tile in path {
                river[tile] = true;
            }
        }
    }
    Ok(GeneratedWorld {
        config,
        terrain,
        river,
    })
}

impl GeneratedWorld {
    pub fn terrain_histogram(&self) -> [usize; 11] {
        let mut result = [0; 11];
        for &t in &self.terrain {
            result[t as usize] += 1;
        }
        result
    }

    /// Stock .MP-shaped export: eight-byte header, three bytes per display
    /// slot, alternating padding slots (as observed in World.mp). No unknown
    /// 90-byte scenario metadata or original assets are copied.
    pub fn mp_bytes(&self) -> Vec<u8> {
        let c = self.config;
        let mut bytes = Vec::with_capacity(8 + usize::from(c.width) * usize::from(c.height) * 3);
        bytes.extend(c.width.to_le_bytes());
        bytes.extend(c.height.to_le_bytes());
        bytes.extend((self.terrain.len() as u32).to_le_bytes());
        for y in 0..usize::from(c.height) {
            for x in 0..usize::from(c.width) {
                if (x + y).is_multiple_of(2) {
                    bytes.extend([0xff, 0xff, 0xf0]);
                } else {
                    let i = y * usize::from(c.width / 2) + x / 2;
                    bytes.extend([self.terrain[i] | (u8::from(self.river[i]) << 7), 0, 0]);
                }
            }
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeds_export_through_the_real_mp_decoder() {
        for seed in [0, 1, 7, u32::MAX] {
            let config = WorldConfig {
                seed,
                ..WorldConfig::default()
            };
            let w = generate(config).unwrap();
            assert_eq!(w, generate(config).unwrap());
            let parsed = tot_kit::parse_mp(&w.mp_bytes()).unwrap();
            assert_eq!(parsed.real_tile_count(), 2000);
            assert_eq!(parsed.declared_real_tiles, Some(2000));
            let mut hist = [0; 11];
            for y in 0..usize::from(config.height) {
                for x in 0..usize::from(config.width) {
                    let slot = y * usize::from(config.width) + x;
                    if let Some(t) = parsed.terrain(slot) {
                        let i = y * usize::from(config.width / 2) + x / 2;
                        assert_eq!(t, w.terrain[i]);
                        assert_eq!(parsed.is_river(slot), w.river[i]);
                        hist[t as usize] += 1;
                    }
                }
            }
            assert_eq!(hist, w.terrain_histogram());
            assert_eq!(hist[10], 1060);
            assert!(!w.river.iter().zip(&w.terrain).any(|(&r, &t)| r && t == 10));
        }
        assert_ne!(
            generate(WorldConfig::default()).unwrap(),
            generate(WorldConfig {
                seed: 2,
                ..WorldConfig::default()
            })
            .unwrap()
        );
    }
    #[test]
    fn dimensions_and_land_targets_are_checked() {
        for (width, height, land_percent) in [
            (31, 50, 35),
            (33, 50, 35),
            (80, 0, 35),
            (80, 257, 35),
            (80, 50, 0),
        ] {
            assert!(generate(WorldConfig {
                width,
                height,
                land_percent,
                ..WorldConfig::default()
            })
            .is_err());
        }
        for land_percent in [10, 35, 80] {
            let w = generate(WorldConfig {
                width: 32,
                height: 16,
                land_percent,
                ..WorldConfig::default()
            })
            .unwrap();
            assert_eq!(
                256 - w.terrain_histogram()[10],
                (256 * usize::from(land_percent) + 50) / 100
            );
        }
    }
}
