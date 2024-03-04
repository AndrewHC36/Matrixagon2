use noise::{NoiseFn, Perlin};
use crate::component::terrain::{Block};



pub struct TerrainGenerator {
    noise: Perlin,
    floral_noise: Perlin,
}

impl TerrainGenerator {
    const SEA_LEVEL: f64 = 10.0;
    const SAND_LEVEL: f64 = 13.0;

    pub fn new() -> Self {
        Self {
            noise: Perlin::new(50), floral_noise: Perlin::new(23),
        }
    }

    pub(super) fn get_block(&self, x: f64, y: f64, z: f64) -> Option<Block> {
        let base_level = self.noise.get([x/20.0, z/20.0])*20.0+20.0;
        let floralness = self.floral_noise.get([x/40.0, z/40.0]);

        if y >= base_level+1.0 {
            if y <= Self::SEA_LEVEL {
                Some(Block(6))
            } else {
                None
            }
        } else if y >= base_level {
            if y <= Self::SEA_LEVEL {
                Some(Block(6))
            } else if 0.8 <= floralness && floralness <= 0.9 {
                if 0.84 <= floralness && floralness <= 0.86 {
                    Some(Block(5))
                } else {
                    Some(Block(4))
                }
            } else {
                None
            }
        } else if y <= Self::SAND_LEVEL {
            Some(Block(3))
        } else if y >= base_level-1.0 {
            Some(Block(0))
        } else if y >= base_level-3.0 {
            Some(Block(1))
        } else {
            Some(Block(2))
        }
    }

    // opaque block height-NBT
    // WHEN THE TERRAIN BEGINS TO BE NOTHING (AFTER OPAQUE BREAK)
    pub(super) fn opaque_block_height_bound_test(&self, x: f64, z: f64) -> f64 {
        let base_level = self.noise.get([x/20.0, z/20.0])*20.0+20.0;

        base_level
    }

    // floral block placement-NBT
    pub(super) fn floral_existence_bound_test(&self, x: f64, z: f64) -> Option<f64> {
        let base_level = self.noise.get([x/20.0, z/20.0])*20.0+20.0;
        let floralness = self.floral_noise.get([x/40.0, z/40.0]);

        if base_level > Self::SEA_LEVEL {
            if 0.8 <= floralness && floralness <= 0.9 {
                if 0.84 <= floralness && floralness <= 0.86 {
                    Some(base_level)
                } else {
                    Some(base_level)
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    // TODO: FLUID NBTs ARE TEMPORARY (FOR FUTURE BETTER FLUID GENERATION, RENDERING, & NEW SIM)
    // fluid block placement-NBT
    pub(super) fn fluid_height_existence_bound_test(&self, x: f64, z: f64) -> Option<f64> {
        let base_level = self.noise.get([x/20.0, z/20.0])*20.0+20.0;

        // covers base_level+1.0 and base_level
        if base_level+1.0 <= Self::SEA_LEVEL {
            Some(Self::SEA_LEVEL)
        } else {
            None
        }
    }
}
