use std::{thread, time::Duration, sync::Arc};
use chrono::Utc;
use redis::Commands;
use super::redis_mod::{RedisState, PubSubMsg};

pub fn publish(state: &impl RedisState) {
    let client = Arc::clone(state.client());
    thread::spawn(move || {
        let mut conn = client.get_connection().unwrap();

        for x in 0..3 {
            thread::sleep(Duration::from_millis(500));
            println!("Publish {} to updates.", x);
            let _: () = conn.publish("updates", (&test_message(x))).unwrap();
        }
    });
}

fn test_message(from: i32) -> PubSubMsg {
    PubSubMsg {
        msg: String::from("This is the test message"),
        from: from,
        sent_at: Utc::now(),
    }
}