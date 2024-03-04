use std::collections::HashMap;
use std::rc::Rc;
use noise::{NoiseFn};
use uom::si::f32::Length;
use crate::chunk_mesh::{Chunk, ChunkGeneratable, Position};
use crate::component::camera::Length3D;
use crate::component::RenderDataPurpose;
use crate::component::terrain::{BlockData, FaceDir};
use crate::component::terrain::mesh_util::ChunkMeshUtil;
use crate::component::terrain::terrain_gen::TerrainGenerator;
use crate::component::texture::TextureIDMapper;
use crate::measurement::{blox, chux, chux_hf};
use crate::shader::chunk::ChunkVertex;

pub struct ChunkGeneratorHF<'b> {
    chunk_size: u32,
    block_ind: Vec<BlockData<'b>>,
    txtr_id_mapper: TextureIDMapper,
    // noise: Perlin,
    // floral_noise: Perlin,
    terrain_gen: Rc<TerrainGenerator>,
}

impl<'b> ChunkGeneratorHF<'b> {
    pub fn new(block_ind: Vec<BlockData<'b>>, txtr_id_mapper: TextureIDMapper, terrain_gen: Rc<TerrainGenerator>) -> Self {
        Self {
            chunk_size: Length::new::<<Self as ChunkGeneratable>::B>(1.0).get::<blox>() as u32, block_ind, txtr_id_mapper,
            // noise: Perlin::new(50), floral_noise: Perlin::new(23),
            terrain_gen,
        }
    }
}

impl<'b> ChunkMeshUtil<'b> for ChunkGeneratorHF<'b> {
    fn chunk_size(&self) -> u32 {self.chunk_size}

    fn texture_id_mapper(&self) -> TextureIDMapper {self.txtr_id_mapper.clone()}

    fn block_ind(&self, ind: usize) -> BlockData<'b> {
        self.block_ind[ind]
    }

    fn terrain_gen(&self) -> Rc<TerrainGenerator> {
        self.terrain_gen.clone()
    }
}

impl ChunkGeneratable for ChunkGeneratorHF<'_> {
    type A = chux_hf;
    type B = chux;
    type V = ChunkVertex;
    type I = u32;

    fn generate_mesh(&self, pos: Length3D)
        -> Vec<(Vec<Self::V>, Vec<Self::I>, Option<FaceDir>, RenderDataPurpose)>
    {
        let ofs = (pos.x.get::<blox>().ceil() as i32, pos.y.get::<blox>().ceil() as i32, pos.z.get::<blox>().ceil() as i32);
        let chunk_pos = |x: u32, y: u32, z: u32| (
            pos.x.get::<blox>()+x as f32,
            pos.y.get::<blox>()+y as f32,
            -pos.z.get::<blox>()-z as f32
        );

        let opaque_cube_mesh = self.voluminous_opaque_cubes_mesh(ofs, chunk_pos);
        let transparent_floral_mesh = self.sparse_transparent_floral_mesh(ofs, chunk_pos);
        let translucent_fluid_mesh = self.temporary_fluid_mesher(ofs, chunk_pos);

        let mut all_mesh = Vec::new();

        for (v, i, f) in opaque_cube_mesh {
            all_mesh.push((v, i, Some(f), RenderDataPurpose::TerrainOpaque))
        }
        all_mesh.push((transparent_floral_mesh.0, transparent_floral_mesh.1, None, RenderDataPurpose::TerrainTransparent));
        all_mesh.push((translucent_fluid_mesh.0, translucent_fluid_mesh.1, None, RenderDataPurpose::TerrainTranslucent));

        all_mesh
    }

    fn aggregate_mesh(&self,
                      central_pos: Length3D,
                      chunks: &HashMap<Position<Self::B>, Chunk<Self::V, Self::I, Self::B>>
    ) -> Vec<(Vec<Self::V>, Vec<Self::I>, RenderDataPurpose)>
    {
        println!("GEN AGGREGATED MESH");

        let mut opaque_verts = vec![];
        let mut opaque_inds = vec![];
        let mut opaque_faces = 0;
        let mut transparent_verts = vec![];
        let mut transparent_inds = vec![];
        let mut transparent_faces = 0;
        let mut translucent_verts = vec![];
        let mut translucent_inds = vec![];
        let mut translucent_faces = 0;

        for chunk in chunks.values().filter(|c| c.visible()) {
            for (vert, raw_ind, _, purpose) in chunk.mesh.iter() {
                match purpose {
                    RenderDataPurpose::TerrainOpaque => {
                        let mut ind = raw_ind.clone().iter().map(|i| i+opaque_faces*4).collect();
                        opaque_faces += vert.len() as u32/4;  // 4 vertices in each face

                        opaque_verts.append(&mut vert.clone());
                        opaque_inds.append(&mut ind);
                    }
                    RenderDataPurpose::TerrainTransparent => {
                        let mut ind = raw_ind.clone().iter().map(|i| i+transparent_faces*4).collect();
                        transparent_faces += vert.len() as u32/4;  // 4 vertices in each face

                        transparent_verts.append(&mut vert.clone());
                        transparent_inds.append(&mut ind);
                    }
                    RenderDataPurpose::TerrainTranslucent => {
                        let mut ind = raw_ind.clone().iter().map(|i| i+translucent_faces*4).collect();
                        translucent_faces += vert.len() as u32/4;  // 4 vertices in each face

                        translucent_verts.append(&mut vert.clone());
                        translucent_inds.append(&mut ind);
                    }
                    _ => {}
                }
            }
        }

        vec![
            (opaque_verts, opaque_inds, RenderDataPurpose::TerrainOpaque),
            (transparent_verts, transparent_inds, RenderDataPurpose::TerrainTransparent),
            (translucent_verts, translucent_inds, RenderDataPurpose::TerrainTranslucent),
        ]
    }
}

