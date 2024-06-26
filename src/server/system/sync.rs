use bevy_ecs::{
    entity::Entity,
    query::{Changed, With},
    system::{Commands, NonSendMut, Query, ResMut},
};
use nalgebra::Vector3;

use crate::{
    common::{
        component::{
            chunk::{self, ChunkBundle, LoadedChunks},
            ChunkData, ChunkMesh, ChunkPos, Player, Pos,
        },
        ClientMessage,
    },
    server::{chunk::ChunkManager, client::ClientBroadcast, ClientComponent},
};

pub fn pos(query: Query<(Entity, &Pos), Changed<Pos>>, mut clients: ResMut<ClientBroadcast>) {
    for (e, pos) in query.iter() {
        clients.send(ClientMessage::PosUpdate(e, *pos));
    }
}

pub fn chunks(
    mut players: Query<(&Pos, &mut LoadedChunks, &mut ClientComponent), With<Player>>,
    chunks: Query<(&ChunkPos, &ChunkData, &ChunkMesh)>,
    mut loader: NonSendMut<ChunkManager>,
    mut commands: Commands,
) {
    for (pos, mut loaded, mut client) in &mut players {
        let fp = **pos / chunk::SIDE_LENGTH as f32;
        let player_chunk = Vector3::new(
            fp.x.floor() as i32,
            fp.y.floor() as i32,
            fp.z.floor() as i32,
        );
        let radius: i32 = 1;
        let width = radius * 2 - 1;
        let mut desired = Vec::new();
        for i in 0..width.pow(3) {
            let pos = Vector3::new(i % width, (i / width) % width, i / (width.pow(2)))
                - Vector3::from_element(radius - 1);
            let dist = pos.cast::<f32>().norm();
            if dist < radius as f32 {
                desired.push((dist, pos));
            }
        }
        desired.sort_by(|(da, ..), (db, ..)| da.total_cmp(db));
        let mut to_load = Vec::new();
        for (_, pos) in desired {
            let coords = pos - player_chunk;
            let pos = ChunkPos(coords);
            if !loaded.contains(&pos) {
                if let Some(id) = loader.entity_at(&pos) {
                    let (pos, data, mesh) = chunks.get(*id).unwrap();
                    client.send(ClientMessage::LoadChunk(
                        *id,
                        ChunkBundle {
                            pos: *pos,
                            data: data.clone(),
                            mesh: mesh.clone(),
                        },
                    ));
                    loaded.insert(*pos);
                } else {
                    to_load.push(pos);
                }
            }
        }
        for pos in to_load {
            if !loader.try_load(pos) {
                break;
            }
        }
    }
    loader.update(&mut commands);
}
