use std::{env, sync::Arc, collections::BTreeMap};

use dotenv::dotenv;
use redis::{Commands, ControlFlow, Client, PubSubCommands};

use crate::{ValidatedUser, config::UserSubscriptions};

pub trait RedisState {
    fn client(&self) -> &Arc<Client>;
}

pub struct Ctx {
    pub client: Arc<Client>,
}

impl Ctx {
    fn new() -> Ctx {
        dotenv().ok();
        let redis_host_name =
            env::var("REDIS_HOSTNAME").unwrap_or(env::var("REDIS_HOSTNAME").to_owned().unwrap_or("NoURL".to_string()));
        let redis_password =
            env::var("REDIS_PASSWORD").unwrap_or(env::var("REDIS_PASSWORD").to_owned().unwrap_or("NoURL".to_string()));
        let redis_conn_url = format!("redis://:{}@{}:6379", redis_password, redis_host_name);
        let client = Client::open(redis_conn_url).unwrap();
        Ctx {
            client: Arc::new(client),
        }
    }
}

impl RedisState for Ctx {
    fn client(&self) -> &Arc<Client> {
        &self.client
    }
}

// pub fn subscribe(state: &impl RedisState) -> thread::JoinHandle<()> {
//     let client = Arc::clone(state.client());
//     thread::spawn(move || {
//         let mut conn = client.get_connection().unwrap();

//         conn.subscribe(&["updates"], |msg| {
//             let ch = msg.get_channel_name();
//             let payload: String = msg.get_payload().unwrap();
//             match payload.as_ref() {
//                 "10" => ControlFlow::Break(()),
//                 a => {
//                     println!("Channel '{}' received '{}'.", ch, a);
//                     ControlFlow::Continue
//                 }
//             }
//         })
//         .unwrap();
//     })
// }

pub fn set_str(
    con: &mut redis::Connection,
    key: &str,
    value: &str,
    ttl_seconds: i32,
) -> Result<(), String> {
    let _ = con
        .set::<&str, &str, String>(key, value)
        .map_err(|e| e.to_string());
    if ttl_seconds > 0 {
        let _ = con
            .expire::<&str, String>(key, ttl_seconds.try_into().unwrap())
            .map_err(|e| e.to_string());
    }
    Ok(())
}

pub fn set_int(
    con: &mut redis::Connection,
    key: &str,
    value: i32,
    ttl_seconds: i32,
) -> Result<(), String> {
    let _ = con
        .set::<&str, i32, String>(key, value)
        .map_err(|e| e.to_string());
    if ttl_seconds > 0 {
        let _ = con
            .expire::<&str, String>(key, ttl_seconds.try_into().unwrap())
            .map_err(|e| e.to_string());
    }
    Ok(())
}

// pub fn publish(state: &impl RedisState) {
//     let client = Arc::clone(state.client());
//     thread::spawn(move || {
//         let mut conn = client.get_connection().unwrap();

//         for x in 0..11 {
//             thread::sleep(Duration::from_millis(500));
//             println!("Publish {} to updates.", x);
//             let _: () = conn.publish("updates", x).unwrap();
//         }
//     });
// }

pub fn redis_connect() -> redis::Connection {
    //format - host:port
    let redis_host_name = env::var("REDIS_HOSTNAME").unwrap_or(env::var("REDIS_HOSTNAME").to_owned().unwrap_or("NoURL".to_string()));

    let redis_password = env::var("REDIS_PASSWORD").unwrap_or(env::var("REDIS_PASSWORD").to_owned().unwrap_or("NoURL".to_string()));
    let redis_conn_url = format!("redis://:{}@{}:6379", redis_password, redis_host_name);
    redis::Client::open(redis_conn_url)
        .expect("Invalid connection URL")
        .get_connection()
        .expect("failed to connect to Redis")
}

// pub fn insert_validated_user(mut con: redis::Connection, cookie_token: String, user: ValidatedUser) -> () {
//     let mut user_session: BTreeMap<String, String> = BTreeMap::new();
//     let prefix = "sessionId";
//     user_session.insert(String::from("username"), user.username);
//     user_session.insert(String::from("email"), user.email);

//     let subs = UserSubscriptions {
//         user_subs: user.user_subs,
//         client_subs: user.client_subs,
//         consult_subs: user.consult_subs,
//         location_subs: user.location_subs,
//         consultant_subs: user.consultant_subs,
//     };

//     let mut user_subs: BTreeMap<String, UserSubscriptions> = BTreeMap::new();
//     user_subs.insert(String::from("user_subs"), subs);
//     // Set it in Redis
//     let _: () = redis::cmd("HSET")
//         .arg(format!("{}:{}", prefix, cookie_token))
//         .arg(user_session)
//         .query(&mut con)
//         .expect("failed to execute HSET");

//     let _: () = redis::cmd("HSET")
//         .arg(format!("{}:{}", prefix, cookie_token))
//         .arg(user_subs)
//         .query(&mut con)
//         .expect("failed to execute HSET");

//     let info: BTreeMap<String, String> = redis::cmd("HGETALL")
//         .arg(format!("{}:{}", prefix, "location"))
//         .query(&mut con)
//         .expect("failed to execute HGETALL");
//     println!("info for rust redis driver: {:?}", info);
// }

pub fn redis_test_data(mut con: redis::Connection) -> () {
    let mut option: BTreeMap<String, i32> = BTreeMap::new();
        let prefix = "select-option";
        option.insert(String::from("location_one"), 1);
        option.insert(String::from("location_two"), 2);
        // Set it in Redis
        let _: () = redis::cmd("HSET")
            .arg(format!("{}:{}", prefix, "location"))
            .arg(option)
            .query(&mut con)
            .expect("failed to execute HSET");
        let _ = set_int(&mut con, "answer", 44, 60);
        // let _: () = con.set("answer", 44).unwrap();
        let answer: i32 = con.get("answer").unwrap();
        println!("Answer: {}", answer);

        let info: BTreeMap<String, String> = redis::cmd("HGETALL")
            .arg(format!("{}:{}", prefix, "location"))
            .query(&mut con)
            .expect("failed to execute HGETALL");
        println!("info for rust redis driver: {:?}", info);

        // let ctx = Ctx::new();
        // let handle = subscribe(&ctx);
        // publish(&ctx);
        // handle.join().unwrap();

        // let mut pubsub = con.as_pubsub();
        // pubsub.subscribe("channel_1")?;
        // pubsub.subscribe("channel_2")?;
        //
        // loop {
        //     let msg = pubsub.get_message()?;
        //     let payload : String = msg.get_payload()?;
        //     println!("channel '{}': {}", msg.get_channel_name(), payload);
        // }
}