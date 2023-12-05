use chrono::{DateTime, Utc};
use dotenv::dotenv;
use futures_util::StreamExt;
use redis::{Client, Commands, AsyncCommands, ControlFlow, PubSubCommands, from_redis_value, RedisResult, ErrorKind, Value, FromRedisValue, RedisWrite, ToRedisArgs, Msg};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::{collections::BTreeMap, env, sync::Arc, thread, time::Duration, ops::Deref};
use deadpool_redis::{redis::{cmd}, Config, Runtime, Pool, Connection};

use crate::redis_mod::{redis_subscriber::subscribe, redis_publisher::publish};

pub trait RedisState {
    fn client(&self) -> &Arc<Client>;
}

pub struct Ctx {
    pub client: Arc<Client>,
    pub conn_url: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel: String,
    pub payload: PubSubMsg,
}

impl Message {
    pub fn new(payload: PubSubMsg) -> Message {
        Message {
            id: Message::generate_id(),
            channel: String::from("order"),
            payload
        }
    }
    fn generate_id() -> String {
        Uuid::new_v4().simple().to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PubSubMsg {
    pub msg: String,
    pub from: i32,
    pub sent_at: DateTime<Utc>,
}

impl PubSubMsg {
    pub fn new(msg: String, from: i32, sent_at: DateTime<Utc>) -> PubSubMsg {
        PubSubMsg { msg, from, sent_at }
    }
}

impl FromRedisValue for PubSubMsg {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let v: String = from_redis_value(v)?;
        let result: Self = match serde_json::from_str::<Self>(&v) {
          Ok(v) => v,
          Err(_err) => return Err((ErrorKind::TypeError, "Parse to JSON Failed").into())
        };
        Ok(result)
    }
}

impl ToRedisArgs for &PubSubMsg {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        out.write_arg_fmt(serde_json::to_string(self).expect("Can't serialize Planet as string"))
    }
}

impl Ctx {
    pub fn new() -> Ctx {
        dotenv().ok();
        let redis_host_name = env::var("REDIS_HOSTNAME").unwrap_or(
            env::var("REDIS_HOSTNAME")
                .to_owned()
                .unwrap_or("NoURL".to_string()),
        );
        let redis_password = env::var("REDIS_PASSWORD").unwrap_or(
            env::var("REDIS_PASSWORD")
                .to_owned()
                .unwrap_or("NoURL".to_string()),
        );
        let redis_conn_url = format!("redis://:{}@{}:6379", redis_password, redis_host_name);
        let client = Client::open(redis_conn_url.clone()).unwrap();
        Ctx {
            client: Arc::new(client),
            conn_url: redis_conn_url,
        }
    }
}

impl RedisState for Ctx {
    fn client(&self) -> &Arc<Client> {
        &self.client
    }
}

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

pub async fn set_int(
    con: &mut deadpool_redis::Connection,
    key: &str,
    value: i32,
    ttl_seconds: i32,
) -> Result<(), String> {
    println!("Set Int");
    cmd("SET")
        .arg(&["deadpool/test_key", "42"])
        .query_async::<_, ()>(con)
        .await.unwrap();
    Ok(())
}

pub fn redis_connect() -> Pool {
    //format - host:port
    let redis_host_name = env::var("REDIS_HOSTNAME").unwrap_or(
        env::var("REDIS_HOSTNAME")
            .to_owned()
            .unwrap_or("NoURL".to_string()),
    );

    let redis_password = env::var("REDIS_PASSWORD").unwrap_or(
        env::var("REDIS_PASSWORD")
            .to_owned()
            .unwrap_or("NoURL".to_string()),
    );
    let redis_conn_url = format!("redis://:{}@{}:6379", redis_password, redis_host_name);

    let mut cfg = Config::from_url(redis_conn_url);
    let pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();
    pool

    // redis::Client::open(redis_conn_url)
    //     .expect("Invalid connection URL")
    //     .get_connection()
    //     .expect("failed to connect to Redis")
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

pub async fn redis_test_data(pool: &Pool) -> () {
    let mut con = pool.get().await.unwrap();
    let mut option: BTreeMap<String, i32> = BTreeMap::new();
    let prefix = "select-option";
    option.insert(String::from("location_one_new"), 1);
    option.insert(String::from("location_two_new"), 2);
    // Set it in Redis
    let _: () = redis::cmd("HSET")
        .arg(format!("{}:{}", prefix, "location"))
        .arg(option)
        .query_async::<_, ()>(&mut con)
        .await
        .expect("failed to execute HSET");
    let _ = set_int(&mut con, "answer", 44, 60);
    // let _: () = con.set("answer", 44).unwrap();
    // let answer: i32 = cmd("GET")
    //     .arg(&["deadpool/test_key"])
    //     .query_async(&mut con)
    //     .await.unwrap();
    // println!("Answer: {}", answer);

    let info: BTreeMap<String, String> = redis::cmd("HGETALL")
        .arg(format!("{}:{}", prefix, "location"))
        .query_async(&mut con)
        .await
        .expect("failed to execute HGETALL");
    println!("info for rust redis driver: {:?}", info);

    // let ctx = Ctx::new();
    // let handle = subscribe(&ctx);
    // publish(&ctx);
    // handle.join().unwrap();

    // let client = try!(redis::Client::open("redis://127.0.0.1/"));
    let ctx = Ctx::new();
    let mut cfg = Config::from_url(ctx.conn_url);
    let pool = cfg.create_pool(Some(Runtime::Tokio1)).unwrap();
    let mut new_con = ctx.client.get_connection().unwrap();
    let mut pubsub = new_con.as_pubsub();
    pubsub.subscribe("updates");

    // loop {
    //     let mut msg = pubsub.get_message();
    //     let (payload, channel) = 
    //         if msg.is_ok() {
    //             let uw = msg.unwrap();
    //             (uw.get_payload().unwrap(), String::from(uw.get_channel_name()))
    //         } else {
    //             (String::from("No payload"), String::from("No channel"))
    //         };
        
    //     println!("channel '{}': {}", channel, payload);
    // }
}
