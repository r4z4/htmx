use std::{thread, sync::Arc};
use redis::{ControlFlow, PubSubCommands};
use super::redis_mod::{RedisState, PubSubMsg};

pub fn subscribe(state: &impl RedisState) -> thread::JoinHandle<()> {
    let client = Arc::clone(state.client());
    println!("Subscribing");
    thread::spawn(move || {
        let mut conn = client.get_connection().unwrap();

        conn.subscribe(&["updates"], |msg| {
            let ch = msg.get_channel_name();
            let payload: String = msg.get_payload().unwrap();
            let pubsub_msg = serde_json::from_str::<PubSubMsg>(&payload).unwrap();
            match pubsub_msg.from {
                3 => ControlFlow::Break(()),
                a => {
                    println!("Channel '{}' received '{}'.", ch, a);
                    ControlFlow::Continue
                }
            }
        })
        .unwrap();
    })
}