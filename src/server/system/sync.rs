use bevy_ecs::{entity::Entity, query::Changed, system::{Query, Res}};

use crate::{sync::{ClientMessage, ClientSender}, world::component::{Pos, Synced}};

pub fn pos(query: Query<(Entity, &Synced, &Pos), Changed<Pos>>, client: Res<ClientSender>) {
    for (e, _, pos) in query.iter() {
        client.send(ClientMessage::PosUpdate(e, *pos));
    }
}
