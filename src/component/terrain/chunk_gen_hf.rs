use std::collections::HashMap;
use noise::Perlin;
use crate::chunk_mesh::{Chunk, ChunkGeneratable, ChunkPosition};
use crate::component::camera::Length3D;
use crate::component::RenderDataPurpose;
use crate::component::terrain::{Block, BlockCullType, BlockData, FaceDir, MeshType, TransparencyType};
use crate::component::terrain::chunk_gen::BlockGenerator;
use crate::component::terrain::mesh_util::ChunkMeshUtil;
use crate::component::texture::TextureIDMapper;
use crate::measurement::{blox, chux_hf};
use crate::shader::chunk::ChunkVertex;

pub(super) struct BlockGeneratorHF<'b> {
    chunk_size: u32,
    block_ind: Vec<BlockData<'b>>,
    txtr_id_mapper: TextureIDMapper,
}

impl<'b> BlockGeneratorHF<'b> {
    pub(super) fn new(chunk_size: u32, block_ind: Vec<BlockData<'b>>, txtr_id_mapper: TextureIDMapper,) -> Self {
        Self {
            chunk_size, block_ind, txtr_id_mapper
        }
    }
}

impl ChunkMeshUtil for BlockGeneratorHF<'_> {
    fn chunk_size(&self) -> u32 {self.chunk_size}

    fn texture_id_mapper(&self) -> TextureIDMapper {self.txtr_id_mapper.clone()}
}

impl ChunkGeneratable for BlockGeneratorHF<'_> {
    type M = chux_hf;
    type P = BlockCullType;
    type V = ChunkVertex;
    type I = u32;

    fn generate_chunk(&self, pos: Length3D) -> Box<[Self::P]> {
        let mut voxel = (0..self.chunk_size*self.chunk_size*self.chunk_size)
            .into_iter()
            .map(|i| {
                let (x,y,z) = self.reverse_access(pos, i as f32);
                // TERRAIN GENERATION (NO SIDE EFFECT)

                if y > 40.0 {
                    BlockCullType::Empty
                } else {
                    BlockCullType::BorderVisible(Block(0))
                }
            })
            .collect::<Box<[BlockCullType]>>();

        self.block_culling(&mut voxel);

        voxel
    }

    fn generate_mesh(&self, chunks: &HashMap<ChunkPosition<Self::M>, Chunk<Self::P, Self::M>>)
        -> Vec<(Vec<Self::V>, Vec<Self::I>, RenderDataPurpose)>
    {
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
            let chunk_pos = |x: u32, y: u32, z: u32| (
                chunk.pos.x.get::<blox>()+x as f32,
                chunk.pos.y.get::<blox>()+y as f32,
                -chunk.pos.z.get::<blox>()-z as f32
            );

            let cull_border_face = |x, y, z, face_dir: FaceDir| {
                match face_dir {
                    FaceDir::FRONT => {
                        if let Some(ref hpos) = chunk.adjacency.front {
                            let adj_block = chunks.get(hpos).unwrap().voxels[self.access(x,y,0)];
                            Self::check_block_obscured(adj_block)
                        } else { true }
                        // if theres no chunk, then it probably means the player can't see it anyways
                        // no need to render the whole face at the border
                    }
                    FaceDir::RIGHT => {
                        if let Some(ref hpos) = chunk.adjacency.right {
                            let adj_block = chunks.get(hpos).unwrap().voxels[self.access(0,y,z)];
                            Self::check_block_obscured(adj_block)
                        } else { true }
                    }
                    FaceDir::BACK => {
                        if let Some(ref hpos) = chunk.adjacency.back {
                            let adj_block = chunks.get(hpos).unwrap().voxels[self.access(x,y,self.chunk_size-1)];
                            Self::check_block_obscured(adj_block)
                        } else { true }
                    }
                    FaceDir::LEFT => {
                        if let Some(ref hpos) = chunk.adjacency.left {
                            let adj_block = chunks.get(hpos).unwrap().voxels[self.access(self.chunk_size-1,y,z)];
                            Self::check_block_obscured(adj_block)
                        } else { true }
                    }
                    FaceDir::TOP => {
                        if let Some(ref hpos) = chunk.adjacency.top {
                            let adj_block = chunks.get(hpos).unwrap().voxels[self.access(x,0,z)];
                            Self::check_block_obscured(adj_block)
                        } else { true }
                    }
                    FaceDir::BOTTOM => {
                        if let Some(ref hpos) = chunk.adjacency.bottom {
                            let adj_block = chunks.get(hpos).unwrap().voxels[self.access(x,self.chunk_size-1,z)];
                            Self::check_block_obscured(adj_block)
                        } else { true }
                    }
                }
            };

            for x in 0..self.chunk_size {
                for y in 0..self.chunk_size {
                    for z in 0..self.chunk_size {
                        let mut local_checked_gen_face = |
                            total_verts: &mut Vec<ChunkVertex>, total_inds: &mut Vec<u32>, total_faces: &mut u32,
                            dx, dy, dz, face_dir, txtr_mapping, fluid: bool, border_always_clear: bool| {
                            if (dx == -1 && x == 0) || (dy == -1 && y == 0) || (dz == 1 && z == self.chunk_size-1) ||
                                ((dx == 1 && x == self.chunk_size-1) || (dy == 1 && y == self.chunk_size-1) || (dz == -1 && z == 0)) {
                                // chunk border mesh culling (more like checking whether any exposed border faces that needs to be shown/added)
                                if !cull_border_face(x, y, z, face_dir) && !border_always_clear {
                                    let (mut verts, mut inds) = self.gen_face(
                                        chunk_pos(x,y,z), *total_faces*4, face_dir, txtr_mapping, fluid
                                    );
                                    total_verts.append(&mut verts);
                                    total_inds.append(&mut inds);
                                    *total_faces += 1;
                                }
                            } else if self.check_coord_within_chunk((x as i32+dx) as u32,(y as i32+dy) as u32,(z as i32+dz) as u32) {
                                // inner face mesh culling
                                let block_cull = chunk.voxels[self.access((x as i32+dx) as u32,(y as i32+dy) as u32,(z as i32+dz) as u32)];
                                if (!fluid && !Self::check_block_obscured(block_cull)) || (fluid && !Self::check_fluid_obscured(block_cull)) {
                                    // if delta face coord is in chunk and not obscured
                                    let (mut verts, mut inds) = self.gen_face(
                                        chunk_pos(x,y,z), *total_faces*4, face_dir, txtr_mapping, fluid
                                    );
                                    total_verts.append(&mut verts);
                                    total_inds.append(&mut inds);
                                    *total_faces += 1;
                                }
                            }
                        };

                        if let BlockCullType::BorderVisible(block) | BlockCullType::BorderVisibleFluid(block) | BlockCullType::AlwaysVisible(block)
                            = &chunk.voxels[self.access(x,y,z)] {
                            let block = self.block_ind[block.0 as usize];

                            let txtr = block.texture_id;

                            let (verts, inds, faces) = match block.transparency {
                                TransparencyType::Opaque => {(&mut opaque_verts, &mut opaque_inds, &mut opaque_faces)}
                                TransparencyType::Transparent => {(&mut transparent_verts, &mut transparent_inds, &mut transparent_faces)}
                                TransparencyType::Translucent => {(&mut translucent_verts, &mut translucent_inds, &mut translucent_faces)}
                            };

                            match block.mesh {
                                MeshType::Cube => {
                                    local_checked_gen_face(verts, inds, faces, 0, 0, 1, FaceDir::FRONT, txtr, false, false);
                                    local_checked_gen_face(verts, inds, faces, 1, 0, 0, FaceDir::RIGHT, txtr, false, false);
                                    local_checked_gen_face(verts, inds, faces, 0, 0, -1, FaceDir::BACK, txtr, false, false);
                                    local_checked_gen_face(verts, inds, faces, -1, 0, 0, FaceDir::LEFT, txtr, false, false);
                                    local_checked_gen_face(verts, inds, faces, 0, 1, 0, FaceDir::TOP, txtr, false, false);
                                    local_checked_gen_face(verts, inds, faces, 0, -1, 0, FaceDir::BOTTOM, txtr, false, false);
                                }
                                MeshType::XCross => {
                                    let (mut xcross_verts, mut xcross_inds) = self.gen_xcross(
                                        chunk_pos(x,y,z), *faces*4, txtr,
                                    );
                                    verts.append(&mut xcross_verts);
                                    inds.append(&mut xcross_inds);
                                    *faces += 2;
                                }
                                MeshType::Fluid => {
                                    local_checked_gen_face(verts, inds, faces, 0, 0, 1, FaceDir::FRONT, txtr, true, true);
                                    local_checked_gen_face(verts, inds, faces, 1, 0, 0, FaceDir::RIGHT, txtr, true, true);
                                    local_checked_gen_face(verts, inds, faces, 0, 0, -1, FaceDir::BACK, txtr, true, true);
                                    local_checked_gen_face(verts, inds, faces, -1, 0, 0, FaceDir::LEFT, txtr, true, true);
                                    local_checked_gen_face(verts, inds, faces, 0, 1, 0, FaceDir::TOP, txtr, true, true);
                                    local_checked_gen_face(verts, inds, faces, 0, -1, 0, FaceDir::BOTTOM, txtr, true, true);
                                }
                            }
                        }
                    }
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

