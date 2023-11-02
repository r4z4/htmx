use actix_web::{web::Data, HttpResponse};
use lazy_static::lazy_static;
use lettre::{Message, message::header::ContentType, transport::stub::StubTransport, Transport};
use mini_markdown::render;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use sqlx::{FromRow, Postgres, Pool};
use std::fmt::Debug;
use std::fs::File;
use validator::{Validate, ValidationError};

use crate::{AppState, HeaderValueExt, ValidatedUser};

lazy_static! {
    pub static ref RE_USER_NAME: Regex = Regex::new(r"^[a-zA-Z0-9]{4,}$").unwrap();
    pub static ref RE_SPECIAL_CHAR: Regex = Regex::new("^.*?[@$!%*?&].*$").unwrap();
    pub static ref RE_EMAIL: Regex = Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})"
    )
    .unwrap();
    pub static ref ACCEPTED_SECONDARIES: Vec<String> = vec![
        "Apt".to_owned(),
        "Apt.".to_owned(),
        "Ste".to_owned(),
        "Ste.".to_owned(),
        "Suite".to_owned(),
        "Apartment".to_owned(),
        "#".to_owned(),
        "No.".to_owned(),
        "No".to_owned()
    ];
    pub static ref ACCEPTED_PRIMARIES: Vec<&'static str> = vec![
        "St.", "St", "Street", "Ave.", "Av.", "Ave", "Avenue", "Parkway", "Pkwy", "Pkwy.", "Dr.",
        "Dr", "Drive", "Ln", "Lane", "Ln."
    ];
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
    pub class: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ValidationResponse {
    pub msg: String,
    pub class: String,
}

#[derive(Serialize, Validate, Deserialize, Debug, Default, Clone)]
pub struct ResponsiveTableData<T> {
    pub entity_type_id: i32,
    pub page: usize,
    pub vec_len: usize,
    // #[validate(url)]
    pub lookup_url: String,
    pub entities: Vec<T>,
}

#[derive(Serialize, Validate, FromRow, Deserialize, Debug, Default, Clone)]
pub struct State {
    state_name: String
}

pub async fn get_state_options(pool: &Pool<Postgres>) -> Vec<StringSelectOption> {
    match sqlx::query_as::<_, State>(
        "SELECT state_name FROM states",
    )
    .fetch_all(pool)
    .await
    {
        Ok(state_list) => {
            state_list.iter().map(|state| StringSelectOption {
                key: Some(state.state_name.to_owned()),
                value: state.state_name.to_owned(),
            }).collect::<Vec<StringSelectOption>>()
        },
        Err(err) => {
            dbg!(&err);
            vec![
                StringSelectOption {
                    key: Some("Select One".to_string()),
                    value: "Select One".to_string(),
                }
            ]
        }
    }
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
        SelectOption {
            key: Some("Location Admin".to_string()),
            value: 1,
        },
        SelectOption {
            key: Some("Site Manager".to_string()),
            value: 2,
        },
    ]
}

pub fn admin_user_options() -> Vec<SelectOption> {
    vec![
        SelectOption {
            key: Some("User 1".to_string()),
            value: 1,
        },
        SelectOption {
            key: Some("User 2".to_string()),
            value: 2,
        },
    ]
}

pub fn user_type_options() -> Vec<SelectOption> {
    vec![
        SelectOption {
            key: Some("admin".to_string()),
            value: 1,
        },
        SelectOption {
            key: Some("subadmin".to_string()),
            value: 2,
        },
        SelectOption {
            key: Some("regular".to_string()),
            value: 3,
        },
        SelectOption {
            key: Some("guest".to_string()),
            value: 4,
        },
    ]
}

pub fn territory_options() -> Vec<SelectOption> {
    vec![
        SelectOption {
            key: Some("National".to_string()),
            value: 1,
        },
        SelectOption {
            key: Some("Northeast".to_string()),
            value: 2,
        },
        SelectOption {
            key: Some("West".to_string()),
            value: 3,
        },
        SelectOption {
            key: Some("Southeast".to_string()),
            value: 4,
        },
        SelectOption {
            key: Some("Midwest".to_string()),
            value: 5,
        },
    ]
}

pub fn specialty_options() -> Vec<SelectOption> {
    vec![
        SelectOption {
            key: Some("Finance".to_string()),
            value: 1,
        },
        SelectOption {
            key: Some("Insurance".to_string()),
            value: 2,
        },
        SelectOption {
            key: Some("Technology".to_string()),
            value: 3,
        },
        SelectOption {
            key: Some("Government".to_string()),
            value: 4,
        },
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

pub async fn validate_and_get_user(
    cookie: &actix_web::http::header::HeaderValue,
    state: &Data<AppState>,
) -> Result<Option<ValidatedUser>, crate::ValidationError> {
    println!("Validating {}", format!("{:?}", cookie.clone()));
    match sqlx::query_as::<_, ValidatedUser>(
        "SELECT username, email, user_type_id, user_settings.list_view
        FROM users
        LEFT JOIN user_sessions ON user_sessions.user_id = users.user_id
        LEFT JOIN user_settings ON user_settings.user_id = users.user_id
        WHERE session_id = $1
        AND expires > NOW()",
    )
    .bind(cookie.to_string())
    .fetch_optional(&state.db)
    .await
    {
        Ok(user_option) => Ok(user_option),
        Err(err) => Err(crate::ValidationError {
            error: format!("You must not be verfied: {}", err),
        }),
    }
}

pub async fn send_email(
    to_email: &str,
    msg: &str,
) -> Result<(), String> {
    let email = Message::builder()
        .from("NoBody <nobody@domain.tld>".parse().unwrap())
        .reply_to("Yuin <yuin@domain.tld>".parse().unwrap())
        .to(to_email.parse().unwrap())
        .subject("Happy new year")
        .header(ContentType::TEXT_PLAIN)
        .body(msg.to_owned())
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
