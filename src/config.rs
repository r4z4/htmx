use actix_web::{web::Data, HttpRequest};
use chrono::{DateTime, Utc};
use futures_util::{stream, Stream, StreamExt};
use lazy_static::lazy_static;
use lettre::{message::header::ContentType, transport::stub::StubTransport, Message, Transport};
use mini_markdown::render;
use rand::distributions::{Distribution, Uniform};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use sqlx::{FromRow, Pool, Postgres};
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::thread::sleep;
use std::time::{self, Instant};
use std::{fmt::Debug, net::IpAddr};
use struct_iterable::Iterable;
use validator::{Validate, ValidationError, ValidationErrors};

use crate::{AppState, HeaderValueExt, ValidatedUser};

lazy_static! {
    pub static ref RE_USERNAME: Regex = Regex::new(r"^[a-zA-Z0-9]{4,}$").unwrap();
    pub static ref RE_SPECIAL_CHAR: Regex = Regex::new("^.*?[@$!%*?&].*$").unwrap();
    pub static ref RE_EMAIL: Regex = Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})"
    )
    .unwrap();
    pub static ref ACCEPTED_SECONDARIES: Vec<&'static str> = vec![
        "Apt",
        "Apt.",
        "Ste",
        "Ste.",
        "Suite",
        "Apartment",
        "#",
        "Pt.",
        "No.",
        "No",
        "Unit",
        "Ut",
        "Un.",
        "Un",
        "Ut."
    ];
    pub static ref ACCEPTED_PRIMARIES: Vec<&'static str> = vec![
        "St.", "St", "Street", "Ave.", "Av.", "Ave", "Avenue", "Parkway", "Pkwy", "Pkwy.", "Dr.",
        "Dr", "Drive", "Ln", "Lane", "Ln."
    ];
    pub static ref VULGAR_LIST: Vec<&'static str> =
        vec!["shit", "fuck", "ass", "retard", "gay", "faggot", "jew"];
}

#[derive(Serialize, Debug)]
pub struct ValidationErrorMap {
    pub key: String,
    pub errs: Vec<ValidationError>,
}

#[derive(Serialize)]
pub struct FormErrorResponse {
    pub errors: Option<Vec<ValidationErrorMap>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Post {
    pub slug: String,
    pub title: String,
    pub author: String,
    pub date: String,
    pub body: String,
}

#[derive(Deserialize, Debug, Validate)]
pub struct FilterOptions {
    pub page: Option<usize>,
    pub limit: Option<usize>,
    #[validate(length(max = 36, message = "Cannot exceed 36 characters in a table search"))]
    pub search: Option<String>,
    pub key: Option<String>,
    pub dir: Option<String>,
}

#[derive(Debug, Validate, Serialize, FromRow, Clone, Deserialize)]
pub struct SelectOption {
    pub value: i32,
    pub key: Option<String>,
}

impl From<(i32, Option<String>)> for SelectOption {
    fn from(pair: (i32, Option<String>)) -> Self {
        let (value, key) = pair;
        SelectOption {
            key: key,
            value: value,
        }
    }
}

#[derive(Debug, Validate, Serialize, FromRow, Clone, Deserialize)]
pub struct StringSelectOption {
    pub value: String,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub todo: String,
    pub date: String,
}

impl Post {
    pub fn render(&self) -> String {
        render(&self.body)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub title: String,
    pub description: String,
    pub default: String,
    pub posts: Vec<Post>,
}

impl Config {
    pub fn new() -> Self {
        let file = File::open("./config/blog.yml").expect("Could not open file.");
        let config = serde_yaml::from_reader(file).expect("Could not read values.");
        config
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct UserAlert {
    pub msg: String,
    pub alert_class: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ValidationResponse {
    pub msg: String,
    pub class: String,
}

impl From<(&str, &str)> for ValidationResponse {
    fn from(pair: (&str, &str)) -> Self {
        let (msg, class) = pair;
        ValidationResponse {
            msg: msg.to_string(),
            class: class.to_string(),
        }
    }
}

pub fn test_subs() -> UserSubscriptions {
    UserSubscriptions {
        user_subs: vec![1],
        client_subs: vec![2,3],
        consult_subs: vec![3,4,5],
        location_subs: vec![4,6,7],
        consultant_subs: vec![3,5,6],
    }
}

pub fn subs_from_user(user: &ValidatedUser) -> UserSubscriptions {
    UserSubscriptions {
        user_subs: user.user_subs.clone(),
        client_subs: user.client_subs.clone(),
        consult_subs: user.consult_subs.clone(),
        location_subs: user.location_subs.clone(),
        consultant_subs: user.consultant_subs.clone(),
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct UserSubscriptions {
    pub user_subs: Vec<i32>,
    pub client_subs: Vec<i32>,
    pub consult_subs: Vec<i32>,
    pub location_subs: Vec<i32>,
    pub consultant_subs: Vec<i32>,
}

#[derive(Serialize, Validate, Deserialize, Debug, Default, Clone)]
pub struct ResponsiveTableData<T> {
    pub entity_type_id: i32,
    pub page: usize,
    pub vec_len: usize,
    pub subscriptions: UserSubscriptions,
    // #[validate(url)]
    pub lookup_url: String,
    pub entities: Vec<T>,
}

#[derive(Serialize, Validate, FromRow, Deserialize, Debug, Default, Clone)]
pub struct State {
    state_name: String,
}

pub fn is_dirty(msg: &str) -> bool {
    let words: Vec<&str> = msg.split(" ").collect::<Vec<&str>>().to_owned();
    let word_count = words.len();
    // Getting last two to account for 101 Hartford St. W etc..
    let dirty = words.iter().any(|word| VULGAR_LIST.contains(word));
    dirty
}

pub async fn get_state_options(pool: &Pool<Postgres>) -> Vec<StringSelectOption> {
    match sqlx::query_as::<_, State>("SELECT state_name FROM states")
        .fetch_all(pool)
        .await
    {
        Ok(state_list) => state_list
            .iter()
            .map(|state| StringSelectOption {
                key: Some(state.state_name.to_owned()),
                value: state.state_name.to_owned(),
            })
            .collect::<Vec<StringSelectOption>>(),
        Err(err) => {
            dbg!(&err);
            vec![StringSelectOption {
                key: Some("Select One".to_string()),
                value: "Select One".to_string(),
            }]
        }
    }
}

lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

async fn get_page(i: usize) -> Vec<usize> {
    println!("get_page()");
    let millis = Uniform::from(200..1000).sample(&mut rand::thread_rng());
    println!(
        "[{}] # get_page({}) will complete in {} ms",
        START_TIME.elapsed().as_millis(),
        i,
        millis
    );

    sleep(time::Duration::from_millis(millis));
    println!(
        "[{}] # get_page({}) completed",
        START_TIME.elapsed().as_millis(),
        i
    );

    (10 * i..10 * (i + 1)).collect()
}

pub async fn get_n_pages(n: usize) -> Vec<Vec<usize>> {
    println!("get_n_pages()");
    get_pages().take(n).collect().await
}

fn get_pages() -> impl Stream<Item = Vec<usize>> {
    println!("get_pages()");
    stream::iter(0..).then(|i| get_page(i))
}

#[derive(Serialize, Validate, FromRow, Deserialize, Debug, Default, Clone)]
pub struct Category {
    category_id: i32,
    category_name: String,
}

pub fn entity_name(entity_type_id: i32) -> &'static str {
    match entity_type_id {
        1 | 2 | 3 => "user",
        4 => "consultant",
        5 => "location",
        6 => "consult",
        7 => "client",
        _ => "user",
    }
}

pub async fn category_options(pool: &Pool<Postgres>) -> Vec<SelectOption> {
    match sqlx::query_as::<_, Category>("SELECT category_id, category_name FROM article_categories")
        .fetch_all(pool)
        .await
    {
        Ok(state_list) => state_list
            .iter()
            .map(|category| SelectOption {
                key: Some(category.category_name.to_owned()),
                value: category.category_id,
            })
            .collect::<Vec<SelectOption>>(),
        Err(err) => {
            dbg!(&err);
            vec![SelectOption::from((0, Some("Select One".to_string())))]
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserFeedData {
    pub posts: Option<Vec<UserPost>>,
    pub consults: Option<Vec<UserFeedDisplay>>,
    pub subs: Option<UserSubscriptions>,
}

#[derive(Serialize, Validate, FromRow, Deserialize, Debug, Clone)]
pub struct UserFeedDisplay {
    slug: String,
    consultant_id: i32,
    client_id: i32,
    location_id: i32,
    consult_start: String,
    notes: Option<String>,
    attachment_count: i32,
    created_at_fmt: String,
    updated_at_fmt: String,
}

#[derive(Serialize, Validate, FromRow, Deserialize, Debug, Iterable, Clone)]
pub struct UserFeedResponse {
    slug: String,
    consultant_id: i32,
    client_id: i32,
    location_id: i32,
    consult_start: DateTime<Utc>,
    notes: Option<String>,
    consult_attachments: Option<Vec<i32>>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}
// = ANY($1) is a workaround for SQLx & IN operator
pub async fn user_feed(
    user: &ValidatedUser,
    pool: &Pool<Postgres>,
) -> UserFeedData {
    match sqlx::query_as::<_, UserFeedResponse>(
        "SELECT id,slug,consultant_id,client_id,location_id,consult_start,notes,consult_attachments,created_at,updated_at FROM consults
        WHERE (client_id = ANY($1) OR location_id = ANY($2) OR consultant_id = ANY($3))
        AND created_at >= NOW() - INTERVAL '7 DAYS' OR updated_at >= NOW() - INTERVAL '7 DAYS'",
    )
    .bind(user.client_subs.clone())
    .bind(user.location_subs.clone())
    .bind(user.consultant_subs.clone())
    .fetch_all(pool)
    .await
    {
        Ok(resp) => {
            let post_file = read_yaml();
            let sub_posts =  post_file.posts.into_iter().filter(|post: &UserPost| {
                user.user_subs.contains(&post.author)
            }).collect::<Vec<UserPost>>();

            let user_feed_display = feed_display_from_resp(resp);

            let feed_data = UserFeedData {
                posts: Some(sub_posts),
                consults: Some(user_feed_display),
                subs: Some(subs_from_user(&user)),
            };
            feed_data
        },
        Err(err) => {
            dbg!(err);
            UserFeedData {
                posts: None,
                consults: None,
                subs: None,
            }
        }
    }
}

fn feed_display_from_resp(resp_arr: Vec<UserFeedResponse>) -> Vec<UserFeedDisplay> {
    resp_arr.iter().map(|resp| 
        UserFeedDisplay {
            slug: resp.slug.clone(),
            consultant_id: resp.consultant_id,
            client_id: resp.client_id,
            location_id: resp.location_id,
            consult_start: resp.consult_start.format("%b %-d, %-I:%M").to_string(),
            notes: resp.notes.clone(),
            attachment_count: if resp.consult_attachments.is_some() {resp.consult_attachments.clone().unwrap().len().try_into().unwrap_or(99)} else {0},
            created_at_fmt: resp.created_at.format("%b %-d, %-I:%M").to_string(),
            updated_at_fmt: resp.created_at.format("%b %-d, %-I:%M").to_string(),
        }
    ).collect::<Vec<UserFeedDisplay>>()
}

pub fn mime_type_id_from_path(path: &str) -> i32 {
    let extension = Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("none");
    match extension {
        ".png" => 1,
        ".jpeg" => 2,
        ".gif" => 3,
        ".webp" => 4,
        ".svg+xml" => 5,
        ".wav" => 6,
        ".mpeg" => 7,
        ".webm" => 8,
        ".webm" => 9,
        ".mpeg" => 10,
        ".mp4" => 11,
        ".json" => 12,
        ".pdf" => 13,
        ".csv" => 14,
        ".html" => 15,
        ".ics" => 16,
        "none" => 0,
        _ => 0,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPostFile {
    pub default: String,
    pub description: String,
    pub title: String,
    pub posts: Vec<UserPost>,
}
#[derive(Debug, Serialize, Deserialize, Iterable, Clone)]
pub struct UserPost {
    pub slug: String,
    pub title: String,
    pub author: i32,
    pub date: String,
    pub body: String,
}

pub fn read_yaml() -> UserPostFile {
    let file_path = "config/blog.yml";
    let contents = fs::read_to_string(file_path)
        .expect(format!("Should have been able to read the file: {file_path}").as_str());

    // don't unwrap like this in the real world! Errors will result in panic!
    let post_file: UserPostFile = serde_yaml::from_str::<UserPostFile>(&contents).unwrap();
    post_file
}

pub fn states() -> Vec<StringSelectOption> {
    vec![
        StringSelectOption {
            key: Some("AL".to_string()),
            value: "AL".to_string(),
        },
        StringSelectOption {
            key: Some("AR".to_string()),
            value: "AK".to_string(),
        },
        StringSelectOption {
            key: Some("AK".to_string()),
            value: "AR".to_string(),
        },
        StringSelectOption {
            key: Some("AZ".to_string()),
            value: "AZ".to_string(),
        },
    ]
}

pub fn location_contacts() -> Vec<SelectOption> {
    vec![
        SelectOption::from((1, Some("Location Admin".to_string()))),
        SelectOption::from((2, Some("Site Manager".to_string()))),
    ]
}

pub fn admin_user_options() -> Vec<SelectOption> {
    vec![
        SelectOption::from((1, Some("User 1".to_string()))),
        SelectOption::from((2, Some("User 2".to_string()))),
    ]
}

pub fn user_type_options() -> Vec<SelectOption> {
    vec![
        SelectOption::from((1, Some("admin".to_string()))),
        SelectOption::from((2, Some("subadmin".to_string()))),
        SelectOption::from((3, Some("regular".to_string()))),
        SelectOption::from((4, Some("guest".to_string()))),
    ]
}

pub fn territory_options() -> Vec<SelectOption> {
    // let file = read_yaml();
    // dbg!(&file);
    vec![
        SelectOption::from((1, Some("National".to_string()))),
        SelectOption::from((2, Some("Northeast".to_string()))),
        SelectOption::from((3, Some("West".to_string()))),
        SelectOption::from((4, Some("Southeast".to_string()))),
        SelectOption::from((5, Some("Midwest".to_string()))),
    ]
}

pub fn consult_result_options() -> Vec<SelectOption> {
    vec![
        SelectOption::from((1, Some("services fully rendered. next meeting scheduled".to_string()))),
        SelectOption::from((2, Some("services fully rendered. no follow up requested".to_string()))),
        SelectOption::from((3, Some("services partially rendered. next meeting scheduled".to_string()))),
        SelectOption::from((4, Some("services partially rendered. no follow up requested".to_string()))),
        SelectOption::from((5, Some("no services rendered. next meeting scheduled".to_string()))),
        SelectOption::from((6, Some("no services rendered. no follow up requested".to_string()))),
    ]
}

pub fn consult_purpose_options() -> Vec<SelectOption> {
    vec![
        SelectOption::from((1, Some("Informational".to_string()))),
        SelectOption::from((2, Some("Initial Service".to_string()))),
        SelectOption::from((3, Some("Continued Service".to_string()))),
        SelectOption::from((4, Some("Final Service".to_string()))),
        SelectOption::from((5, Some("Audit".to_string()))),
    ]
}

pub fn specialty_options() -> Vec<SelectOption> {
    vec![
        SelectOption::from((1, Some("Finance".to_string()))),
        SelectOption::from((2, Some("Insurance".to_string()))),
        SelectOption::from((3, Some("Technology".to_string()))),
        SelectOption::from((4, Some("Government".to_string()))),
    ]
}

// pub fn mock_responsive_table_data() -> ResponsiveTableData {
//     let table_headers = ["One".to_owned(), "Two".to_owned(), "Three".to_owned()].to_vec();
//     let table_row = ResponsiveTableRow {
//         tds: ["Steve".to_owned(), "Jim".to_owned(), "Lehr".to_owned()].to_vec(),
//     };
//     let table_row_2 = ResponsiveTableRow {
//         tds: ["Steve".to_owned(), "Jim".to_owned(), "Lehr".to_owned()].to_vec(),
//     };
//     let table_row_3 = ResponsiveTableRow {
//         tds: ["Steve".to_owned(), "Jim".to_owned(), "Lehr".to_owned()].to_vec(),
//     };
//     let table_row_4 = ResponsiveTableRow {
//         tds: ["Steve".to_owned(), "Jim".to_owned(), "Lehr".to_owned()].to_vec(),
//     };
//     let table_rows = [table_row, table_row_2, table_row_3, table_row_4].to_vec();
//     let responsive_table_data = ResponsiveTableData {
//         table_headers: table_headers,
//         table_rows: table_rows,
//     };

//     return responsive_table_data;
// }

/*************************
*** Validation Helpers ***
*************************/

pub fn validate_username(username: &str) -> Result<(), ValidationError> {
    if username.len() < 3 {
        Err(ValidationError {
            // FIXME: Use key? Make code a descriptor like 'length' or 'range'
            code: std::borrow::Cow::Borrowed("length"),
            message: Some(Cow::from("Username must be 3 chars.")),
            params: HashMap::new(),
        })
    } else {
        Ok(())
    }
}

pub fn validate_primary_address(addr: &str) -> Result<(), ValidationError> {
    if !addr.contains(" ") {
        return Err(ValidationError {
            code: std::borrow::Cow::Borrowed("contain"),
            message: Some(Cow::from("Primary Address must contain a space.")),
            params: HashMap::new(),
        });
    }
    let street_strings: Vec<&str> = addr.split(" ").collect::<Vec<&str>>().to_owned();
    let ss_len = street_strings.len();
    // Getting last two to account for 101 Hartford St. W etc..
    if ACCEPTED_PRIMARIES.contains(&street_strings[ss_len - 1])
        || ACCEPTED_PRIMARIES.contains(&street_strings[ss_len - 2])
    {
        Ok(())
    } else {
        Err(ValidationError {
            code: std::borrow::Cow::Borrowed("identifier"),
            message: Some(Cow::from(
                "Primary Address must contain a valid Identifier (St., Ave, Lane ...)",
            )),
            params: HashMap::new(),
        })
    }
}

pub fn validate_secondary_address(addr_two: &str) -> Result<(), ValidationError> {
    // No input comes in as blank Some(""). These get turned into NULLs in DB.
    if addr_two == "" {
        return Ok(());
    }
    let len_range = 3..15;
    if !len_range.contains(&addr_two.len()) {
        Err(ValidationError {
            code: std::borrow::Cow::Borrowed("length"),
            message: Some(Cow::from("Secondary address must be 3 to 15 characters")),
            params: HashMap::new(),
        })
    } else {
        let apt_ste: Vec<&str> = addr_two.split(" ").collect::<Vec<&str>>().to_owned();
        let first = apt_ste[0];
        dbg!(&first);
        if ACCEPTED_SECONDARIES.contains(&first) {
            Ok(())
        } else {
            Err(ValidationError {
                code: std::borrow::Cow::Borrowed("identifier"),
                message: Some(Cow::from(
                    "Secondary Address must contain a valid Identifier (Unit, Apt, # ...)",
                )),
                params: HashMap::new(),
            })
            // See if I can impl From with a message
            // Err(ValidationError::new(
            //     "Secondary Address must contain a valid Identifier (Unit, Apt, # ...)",
            // ))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
pub struct TableRow {
    pub th: String,
    pub tds: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
pub struct FixedTableData {
    pub table_headers: Vec<String>,
    pub table_rows: Vec<TableRow>,
}

pub fn mock_fixed_table_data() -> FixedTableData {
    let table_headers = [
        "One".to_owned(),
        "Two".to_owned(),
        "Three".to_owned(),
        "Four".to_owned(),
        "Five".to_owned(),
        "Six".to_owned(),
        "Seven".to_owned(),
        "Eight".to_owned(),
        "Nine".to_owned(),
    ]
    .to_vec();
    let th = "One".to_owned();
    let tds = [
        "Two".to_owned(),
        "Three".to_owned(),
        "Four".to_owned(),
        "Five".to_owned(),
        "Six".to_owned(),
        "Seven".to_owned(),
        "Eight".to_owned(),
        "Nine".to_owned(),
    ]
    .to_vec();
    let table_row_1 = TableRow {
        th: th.clone(),
        tds: tds.clone(),
    };
    let table_row_2 = TableRow { th: th, tds: tds };
    let table_row_3 = TableRow {
        th: "One".to_owned(),
        tds: [
            "Two".to_owned(),
            "Three".to_owned(),
            "Four".to_owned(),
            "Five".to_owned(),
            "Six".to_owned(),
            "Seven".to_owned(),
            "Eight".to_owned(),
            "Nine".to_owned(),
        ]
        .to_vec(),
    };
    let table_row_4 = TableRow {
        th: "One".to_owned(),
        tds: [
            "Two".to_owned(),
            "Three".to_owned(),
            "Four".to_owned(),
            "Five".to_owned(),
            "Six".to_owned(),
            "Seven".to_owned(),
            "Eight".to_owned(),
            "Nine".to_owned(),
        ]
        .to_vec(),
    };
    let table_row_5 = TableRow {
        th: "One".to_owned(),
        tds: [
            "Two".to_owned(),
            "Three".to_owned(),
            "Four".to_owned(),
            "Five".to_owned(),
            "Six".to_owned(),
            "Seven".to_owned(),
            "Eight".to_owned(),
            "Nine".to_owned(),
        ]
        .to_vec(),
    };
    let table_row_6 = TableRow {
        th: "One".to_owned(),
        tds: [
            "Two".to_owned(),
            "Three".to_owned(),
            "Four".to_owned(),
            "Five".to_owned(),
            "Six".to_owned(),
            "Seven".to_owned(),
            "Eight".to_owned(),
            "Nine".to_owned(),
        ]
        .to_vec(),
    };
    let table_row_7 = TableRow {
        th: "One".to_owned(),
        tds: [
            "Two".to_owned(),
            "Three".to_owned(),
            "Four".to_owned(),
            "Five".to_owned(),
            "Six".to_owned(),
            "Seven".to_owned(),
            "Eight".to_owned(),
            "Nine".to_owned(),
        ]
        .to_vec(),
    };
    let table_row_8 = TableRow {
        th: "One".to_owned(),
        tds: [
            "Two".to_owned(),
            "Three".to_owned(),
            "Four".to_owned(),
            "Five".to_owned(),
            "Six".to_owned(),
            "Seven".to_owned(),
            "Eight".to_owned(),
            "Nine".to_owned(),
        ]
        .to_vec(),
    };
    let table_row_9 = TableRow {
        th: "One".to_owned(),
        tds: [
            "Two".to_owned(),
            "Three".to_owned(),
            "Four".to_owned(),
            "Five".to_owned(),
            "Six".to_owned(),
            "Seven".to_owned(),
            "Eight".to_owned(),
            "Nine".to_owned(),
        ]
        .to_vec(),
    };
    let table_row_10 = TableRow {
        th: "One".to_owned(),
        tds: [
            "Two".to_owned(),
            "Three".to_owned(),
            "Four".to_owned(),
            "Five".to_owned(),
            "Six".to_owned(),
            "Seven".to_owned(),
            "Eight".to_owned(),
            "Nine".to_owned(),
        ]
        .to_vec(),
    };
    let table_rows = [
        table_row_1,
        table_row_2,
        table_row_3,
        table_row_4,
        table_row_5,
        table_row_6,
        table_row_7,
        table_row_8,
        table_row_9,
        table_row_10,
    ]
    .to_vec();
    let fixed_table_data = FixedTableData {
        table_headers: table_headers,
        table_rows: table_rows,
    };

    return fixed_table_data;
}

pub fn get_ip(req: HttpRequest) -> IpAddr {
    let socket = req
        .peer_addr()
        .unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9999));
    let ip_addr = socket.ip();
    ip_addr
}

pub fn get_validation_response(is_valid: Result<(), ValidationErrors>) -> FormErrorResponse {
    println!("get_validation_response firing");
    let val_errs = is_valid
        .err()
        .unwrap()
        .field_errors()
        .iter()
        .map(|x| {
            let (key, errs) = x;
            ValidationErrorMap {
                key: key.to_string(),
                errs: errs.to_vec(),
            }
        })
        .collect::<Vec<ValidationErrorMap>>();
    dbg!(&val_errs);
    // return HttpResponse::InternalServerError().json(format!("{:?}", is_valid.err().unwrap()));
    let validation_response = FormErrorResponse {
        errors: Some(val_errs),
    };
    validation_response
}

pub async fn validate_and_get_user(
    cookie: &actix_web::http::header::HeaderValue,
    state: &Data<AppState>,
) -> Result<Option<ValidatedUser>, crate::ValError> {
    println!("Validating {}", format!("{:?}", cookie.clone()));
    let session_id = cookie.to_string().split(" ").collect::<Vec<&str>>()[1].to_string();
    dbg!(&session_id);
    match sqlx::query_as::<_, ValidatedUser>(
        "SELECT username, email, user_type_id, user_subs, client_subs, consult_subs, location_subs, consultant_subs, user_settings.list_view
        FROM users
        LEFT JOIN user_sessions ON user_sessions.user_id = users.id
        LEFT JOIN user_settings ON user_settings.user_id = users.id
        WHERE session_id = $1
        AND expires > NOW()",
    )
    .bind(session_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(user_option) => Ok(user_option),
        Err(err) => {
            dbg!(&err);
            Err(crate::ValError {
                error: format!("You must not be verified: {}", err),
            })
        }
    }
}

pub struct SendEmailInput {
    to_email: String,
    msg: String,
}

impl From<(&str, &str)> for SendEmailInput {
    fn from(pair: (&str, &str)) -> Self {
        let (to_email, msg) = pair;
        SendEmailInput {
            to_email: to_email.to_string(),
            msg: msg.to_string(),
        }
    }
}

pub async fn send_email(email_input: SendEmailInput) -> Result<(), String> {
    let email = Message::builder()
        .from("NoBody <nobody@domain.tld>".parse().unwrap())
        .reply_to("Yuin <yuin@domain.tld>".parse().unwrap())
        .to(email_input.to_email.parse().unwrap())
        .subject("Happy new year")
        .header(ContentType::TEXT_PLAIN)
        .body(email_input.msg.to_owned())
        .unwrap();

    // dbg!(&email);

    // let smtp_user = env::var("SMTP_USER").unwrap_or("NoUsername".to_string());
    // let smtp_pass = env::var("SMTP_PASS").unwrap_or("NoPass".to_string());

    // let creds = Credentials::new(smtp_user, smtp_pass);
    // // Open a remote connection to gmail
    // let mailer = SmtpTransport::relay("smtp.gmail.com")
    //     .unwrap()
    //     .credentials(creds)
    //     .build();

    let mut sender = StubTransport::new_ok();
    let result = sender.send(&email);
    assert!(result.is_ok());
    assert_eq!(
        sender.messages(),
        vec![(
            email.envelope().clone(),
            String::from_utf8(email.formatted()).unwrap()
        )],
    );

    // Send the email
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err("Error Sending Email".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[test]
    async fn send_email_builds_correctly() {
        let to_email = "JimboTest@Test.com";
        let msg = "This is a test email so do not respond.";
        let email_input = SendEmailInput::from((to_email, msg));
        let result = send_email(email_input).await;
        assert!(result.is_ok());
    }
}
