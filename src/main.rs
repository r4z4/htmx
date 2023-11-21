use std::env;

use actix_files::Files;
use actix_web::{
    get,
    http::header::HeaderValue,
    post,
    web::{self, Data},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use config::{is_dirty, Post, UserFeedData};
use dotenv::dotenv;
use handlebars::Handlebars;
use hbs_helpers::{
    attachments_rte, concat_args, concat_str_args, form_rte, get_list_view, get_search_rte,
    get_table_title, int_eq, int_in, loc_vec_len_ten, lower_and_single, sort_rte, str_eq,
    subscribe_rte, to_title_case, subscribe_icon, preview_text, is_icon_col, get_icon, first_week, second_week, third_week, fourth_week, fifth_week, get_month_name,
};
use models::{
    model_admin::AdminUserList, model_consultant::ResponseConsultant, model_location::LocationList,
};
use redis::{redis_connect, redis_test_data};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, FromRow, Pool, Postgres};
use validator::{Validate, ValidationError};

use crate::{config::{
    get_ip, mock_fixed_table_data, user_feed, validate_and_get_user, ValidationResponse,
}, linfa::linfa_pred};

use scopes::{
    admin::admin_scope, auth::auth_scope, client::client_scope, consult::consult_scope,
    consultant::consultant_scope, location::location_scope, user::user_scope, event::event_scope
};
mod config;
mod linfa;
mod hbs_helpers;
mod models;
mod redis;
mod scopes;
#[cfg(test)]
mod test_common;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Entity {
    Location(LocationList),
    Consultant(ResponseConsultant),
    User(AdminUserList),
}

#[derive(Debug)]
pub struct AppState {
    db: Pool<Postgres>,
    secret: String,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TodoRequest {
    pub todo: String,
    pub date: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexData {
    pub title: String,
    pub description: String,
}

// #[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
// pub struct ResponsiveTableData {
//     pub table_headers: Vec<String>,
//     pub table_rows: Vec<ResponsiveTableRow>,
// }
#[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
pub struct ResponsiveTableRow {
    pub tds: Vec<String>,
}

// #[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
// pub struct ResponsiveTd {
//     pub table_data: String,
//     pub value: String,
// }
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HomepageTemplate {
    err: Option<String>,
    user: Option<ValidatedUser>,
    feed_data: UserFeedData,
}

#[get("/")]
async fn index(
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
    state: Data<AppState>,
    req: HttpRequest,
    config: web::Data<config::Config>,
) -> impl Responder {
    let headers = req.headers();
    // for (pos, e) in headers.iter().enumerate() {
    //     println!("Element at position {}: {:?}", pos, e);
    // }
    if let Some(cookie) = headers.get(actix_web::http::header::COOKIE) {
        dbg!(cookie.clone());
        match validate_and_get_user(cookie, &state).await {
            Ok(user_option) => {
                if let Some(user) = user_option {
                    let feed_data = user_feed(
                        &user,
                        &state.db,
                    )
                    .await;
                    let template_data = HomepageTemplate {
                        err: None,
                        user: Some(user),
                        feed_data: feed_data,
                    };
                    let body = hb.render("homepage", &template_data).unwrap();
                    return HttpResponse::Ok()
                        .header("HX-Redirect", "/homepage")
                        .body(body);
                } else {
                    let message =
                        "Your session seems to have expired. Please login again (1).".to_owned();
                    let body = hb.render("index", &message).unwrap();

                    HttpResponse::Ok().body(body)
                }
            }
            Err(_err) => {
                // User's cookie is invalud or expired. Need to get a new one via logging in.
                // They had a session. Could give them details about that. Get from DB.
                let message = "Error in validate and get user.".to_owned();
                let body = hb.render("index", &message).unwrap();

                HttpResponse::Ok().body(body)
            }
        }
    } else {
        let data = json!({
            "header": "Login Form",
        });
        let body = hb.render("index", &data).unwrap();
        HttpResponse::Ok().header("HX-Redirect", "/").body(body)
    }
}

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::stub::StubTransport;
use lettre::{Message, SmtpTransport, Transport};
#[get("/send-email")]
async fn send_email(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: Data<AppState>,
) -> impl Responder {
    let email = Message::builder()
        .from("NoBody <nobody@domain.tld>".parse().unwrap())
        .reply_to("Yuin <yuin@domain.tld>".parse().unwrap())
        .to("r4z4 <r4z4aa@gmail.com>".parse().unwrap())
        .subject("Happy new year")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from("Be happy!"))
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
        Ok(_) => {
            let msg = "Email sent successfully!";
            dbg!(msg);
            let template_data = json! {{
                // "user": user,
                "data": &msg,
            }};
            let body = hb.render("about-us", &template_data).unwrap();
            HttpResponse::Ok().body(body)
        }
        Err(e) => {
            let msg = format!("Could not send email: {e:?}");
            dbg!(&msg);
            let template_data = json! {{
                // "user": user,
                "data": &msg,
            }};
            let body = hb.render("about-us", &template_data).unwrap();
            HttpResponse::Ok().body(body)
        }
    }
}

#[get("/about-us")]
async fn about_us(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: Data<AppState>,
) -> impl Responder {
    let headers = req.headers();
    let data = json!({
        "name": "ExtRev",
        "title": "Best",
        "contact": "Phone: (555) 555-5555. Email: McGillicuddy@Con.com",
        "history": "McGillicuddy Consultancy was founded on the principal of Alouette, gentille alouette, Alouette, je te plumerai."
    });
    if let Some(cookie) = headers.get(actix_web::http::header::COOKIE) {
        dbg!(cookie.clone());
        let _ = linfa_pred();
        match validate_and_get_user(cookie, &state).await {
            Ok(user_option) => {
                if let Some(user) = user_option {
                    let template_data = json! {{
                        "user": user,
                        "data": &data,
                    }};
                    let body = hb.render("about-us", &template_data).unwrap();

                    HttpResponse::Ok().body(body)
                } else {
                    let template_data = json! {{
                        // "user": user,
                        "data": &data,
                    }};
                    let body = hb.render("about-us", &template_data).unwrap();
                    HttpResponse::Ok().body(body)
                }
            }
            Err(err) => {
                // User's cookie is invalud or expired. Need to get a new one via logging in.
                // They had a session. Could give them details about that. Get from DB.
                let template_data = HomepageTemplate {
                    err: Some(format!(
                        "Something quite unexpected has happened in your session: {}",
                        err.error
                    )),
                    user: None,
                    feed_data: UserFeedData {
                        posts: None,
                        consults: None,
                        subs: None,
                    },
                };
                // let data = HbError {
                //     str: format!(
                //         "Something quite unexpected has happened in your session: {}",
                //         err.error
                //     ),
                // };
                let body = hb.render("homepage", &template_data).unwrap();
                HttpResponse::Ok().body(body)
            }
        }
    } else {
        let template_data = json! {{
            // "user": user,
            "data": &data,
        }};
        let body = hb.render("about-us", &template_data).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[get("/crud")]
async fn crud_api(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: Data<AppState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match validate_and_get_user(cookie, &state).await {
            Ok(user) => {
                if let Some(usr) = user {
                    let template_data = json! {{
                        "user": &usr,
                        //"data": &data,
                    }};
                    let body = hb.render("crud-api", &template_data).unwrap();
                    HttpResponse::Ok().body(body)
                } else {
                    let message = "Cannot find you";
                    let body = hb.render("index", &message).unwrap();
                    return HttpResponse::Ok().body(body);
                }
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("index", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok().body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
        // FIXME: Is this else right? Redirect?
    } else {
        let message = "Your session seems to have expired. Please login again (2).".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[get("/list")]
async fn list_api(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: Data<AppState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match validate_and_get_user(cookie, &state).await {
            Ok(user) => {
                if let Some(usr) = user {
                    let template_data = json! {{
                        "user": &usr,
                        //"data": &data,
                    }};
                    let body = hb.render("list-api", &template_data).unwrap();
                    HttpResponse::Ok().body(body)
                } else {
                    let message = "Cannot find you";
                    let body = hb.render("index", &message).unwrap();
                    return HttpResponse::Ok().body(body);
                }
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("index", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok().body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
        // FIXME: Is this else right? Redirect?
    } else {
        let message = "Your session seems to have expired. Please login again (3).".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[get("/schedule")]
async fn schedule_api(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: Data<AppState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match validate_and_get_user(cookie, &state).await {
            Ok(user) => {
                if let Some(usr) = user {
                    let template_data = json! {{
                        "user": &usr,
                        //"data": &data,
                    }};
                    let body = hb.render("schedule-api", &template_data).unwrap();
                    HttpResponse::Ok().body(body)
                } else {
                    let message = "Cannot find you";
                    let body = hb.render("index", &message).unwrap();
                    return HttpResponse::Ok().body(body);
                }
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("index", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok().body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
        // FIXME: Is this else right? Redirect?
    } else {
        let message = "Your session seems to have expired. Please login again (3).".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[get("/fixed")]
async fn fixed_table(hb: web::Data<Handlebars<'_>>) -> impl Responder {
    let fixed_table_data = mock_fixed_table_data();
    let body = hb.render("fixed-table", &fixed_table_data).unwrap();
    HttpResponse::Ok().body(body)
}

#[get("/homepage")]
async fn homepage(
    hb: web::Data<Handlebars<'_>>,
    state: Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    dbg!(&req);
    // FIXME unwrap()
    let headers = req.headers();
    dbg!(&headers);
    if let Some(cookie) = headers.get(actix_web::http::header::COOKIE) {
        dbg!(&cookie);
        match validate_and_get_user(cookie, &state).await {
            Ok(user_option) => {
                if let Some(user) = user_option {
                    let feed_data = user_feed(
                        &user,
                        &state.db,
                    )
                    .await;
                    let template_data = HomepageTemplate {
                        err: None,
                        user: Some(user),
                        feed_data: feed_data,
                    };
                    let body = hb.render("homepage", &template_data).unwrap();
                    dbg!(&body);
                    HttpResponse::Ok().body(body)
                } else {
                    let template_data = HomepageTemplate {
                        err: Some("Seems your session has expired. Please login again (4)".to_owned()),
                        user: None,
                        feed_data: UserFeedData {
                            posts: None,
                            consults: None,
                            subs: None,
                        },
                    };
                    let body = hb.render("homepage", &template_data).unwrap();
                    HttpResponse::Ok().body(body)
                }
            }
            Err(err) => {
                // User's cookie is invalud or expired. Need to get a new one via logging in.
                // They had a session. Could give them details about that. Get from DB.
                let template_data = HomepageTemplate {
                    err: Some(format!(
                        "Something quite unexpected has happened in your session: {}",
                        err.error
                    )),
                    user: None,
                    feed_data: UserFeedData {
                        posts: None,
                        consults: None,
                        subs: None,
                    },
                };
                let body = hb.render("homepage", &template_data).unwrap();
                HttpResponse::Ok().body(body)
            }
        }
    } else {
        let template_data = HomepageTemplate {
            err: Some("Cookie is missing".to_owned()),
            user: None,
            feed_data: UserFeedData {
                posts: None,
                consults: None,
                subs: None,
            },
        };
        let body = hb.render("homepage", &template_data).unwrap();
        HttpResponse::Ok().body(body)
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArticleData {
    pub title: String,
    pub description: String,
    pub posts: Vec<Post>,
}

#[get("/articles")]
async fn detail(
    hb: web::Data<Handlebars<'_>>,
    config: web::Data<config::Config>,
    state: Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    println!("Articles");
    let headers = req.headers();
    // for (pos, e) in headers.iter().enumerate() {
    //     println!("Element at position {}: {:?}", pos, e);
    // }
    let data = ArticleData {
        title: config.title.clone(),
        description: config.description.clone(),
        posts: config.posts.clone(),
    };
    if let Some(cookie) = headers.get(actix_web::http::header::COOKIE) {
        dbg!(cookie.clone());
        match validate_and_get_user(cookie, &state).await {
            Ok(user_option) => {
                if let Some(user) = user_option {
                    let user = ValidatedUser {
                        username: user.username,
                        email: user.email,
                        user_type_id: user.user_type_id,
                        list_view: user.list_view,
                        user_subs: user.user_subs,
                        client_subs: user.client_subs,
                        consult_subs: user.consult_subs,
                        location_subs: user.location_subs,
                        consultant_subs: user.consultant_subs,
                    };
                    let template_data = json! {{
                        "user": user,
                        "data": &data,
                    }};
                    let body = hb.render("articles", &template_data).unwrap();

                    HttpResponse::Ok().body(body)
                } else {
                    let template_data = json! {{
                        // "user": user,
                        "data": &data,
                    }};
                    let body = hb.render("articles", &template_data).unwrap();
                    HttpResponse::Ok().body(body)
                }
            }
            Err(err) => {
                // User's cookie is invalud or expired. Need to get a new one via logging in.
                // They had a session. Could give them details about that. Get from DB.
                let template_data = HomepageTemplate {
                    err: Some(format!(
                        "Something quite unexpected has happened in your session: {}",
                        err.error
                    )),
                    user: None,
                    feed_data: UserFeedData {
                        posts: None,
                        consults: None,
                        subs: None,
                    },
                };
                let body = hb.render("homepage", &template_data).unwrap();
                HttpResponse::Ok().body(body)
            }
        }
    } else {
        let template_data = json! {{
            // "user": user,
            "data": &data,
        }};
        let body = hb.render("articles", &template_data).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[get("/content/{slug}")]
async fn content(
    config: web::Data<config::Config>,
    hb: web::Data<Handlebars<'_>>,
    path: web::Path<String>,
) -> impl Responder {
    let slug = path.into_inner();
    if let Some(post) = config.posts.iter().find(|post| post.slug == slug) {
        let data = json!({
            "slug": slug,
            "title": post.title,
            "author": post.author,
            "date": post.date,
            "body": post.render(),
        });
        let body = hb.render("content", &data).unwrap();

        HttpResponse::Ok().body(body)
    } else {
        let error_msg = "Error retrieving content";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[derive(Debug, Validate, Serialize, Deserialize)]
pub struct ValError {
    error: String,
}
#[derive(Debug, FromRow, Validate, Clone, Serialize, Deserialize)]
pub struct ValidatedUser {
    email: String,
    username: String,
    user_type_id: i32,
    list_view: String,
    user_subs: Vec<i32>,
    client_subs: Vec<i32>,
    consult_subs: Vec<i32>,
    location_subs: Vec<i32>,
    consultant_subs: Vec<i32>,
}

pub trait HeaderValueExt {
    fn to_string(&self) -> String;
}

impl HeaderValueExt for HeaderValue {
    fn to_string(&self) -> String {
        self.to_str().unwrap_or_default().to_string()
    }
}

pub fn make_todo(todo: String) -> Result<String, String> {
    if todo == "" {
        return Err("Error".to_owned());
    } else {
        return Ok(todo);
    }
}
#[post("/todos")]
async fn create_todo(
    body: web::Form<TodoRequest>,
    hb: web::Data<Handlebars<'_>>,
    // data: web::Data<AppState>,
) -> impl Responder {
    let body_clone = body.clone();
    dbg!(&body_clone);
    let todo = make_todo(body_clone.todo);

    let data = json!({
        "todo": todo,
        "date": body_clone.date,
    });

    match todo {
        Ok(todo) => {
            let body = hb.render("todo-list", &data).unwrap();
            return HttpResponse::Ok().body(body);
        }
        Err(error_msg) => {
            let validation_response =
                ValidationResponse::from((error_msg.as_str(), "validation_error"));
            let body = hb.render("validation", &validation_response).unwrap();
            return HttpResponse::Ok().body(body);
        }
    }
}

#[get("/contact-us")]
async fn contact_us(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    let data = json!({
        "type": "article",
    });

    let body = hb.render("forms/contact-us", &data).unwrap();
    return HttpResponse::Ok().body(body);
}

fn validate_contact_message(message: &str) -> Result<(), ValidationError> {
    if is_dirty(message) {
        Err(ValidationError::new(
            "Message cannot contain vulgar language. Take that **** elsewhere :/",
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug, FromRow, Validate, Clone, Serialize, Deserialize)]
pub struct ContactUsRequest {
    #[validate(length(min = 3, message = "Name must be greater than 3 chars"))]
    name: String,
    #[validate(length(equal = 10, message = "Invalid Phone Number"))]
    phone: String,
    #[validate(length(min = 3, message = "Invalid Email"))]
    email: String,
    #[validate(
        length(
            min = 10,
            max = 255,
            message = "Invalid Message. Please see the message criteria."
        ),
        custom = "validate_contact_message"
    )]
    message: String,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct ContactReturn {
    contact_submission_id: i32,
}

#[post("/contact-us")]
async fn contact_us_submission(
    hb: web::Data<Handlebars<'_>>,
    body: web::Form<ContactUsRequest>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    let is_valid = body.validate();
    let ip_addr = get_ip(req);
    let ip_addr_str = ip_addr.to_string();
    if is_valid.is_err() {
        // return HttpResponse::InternalServerError().json(format!("{:?}", is_valid.err().unwrap()));
        let error_msg =
            "Validation Error".to_owned() + format!("{}", is_valid.err().unwrap()).as_str();
        let validation_response =
            ValidationResponse::from((error_msg.as_str(), "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::BadRequest()
            .header("HX-Retarget", "#validation_response")
            .body(body);
    } else {
        match sqlx::query_as::<_, ContactReturn>(
            "INSERT INTO contact_submissions (contact_submission_id, name, email, phone, message, ip_addr)
            VALUES (DEFAULT, $1, $2, $3, $4, $5)
            RETURNING contact_submission_id",
        )
        .bind(&body.name)
        .bind(&body.email)
        .bind(&body.phone)
        .bind(&body.message)
        .bind(&ip_addr_str)
        .fetch_one(&state.db)
        .await
        {
            Ok(_) => {
                let success_msg = "Message successfully sent. Thank you :)";
                let validation_response = ValidationResponse::from((success_msg, "validation_success"));
                let body = hb.render("validation", &validation_response).unwrap();
                return HttpResponse::Ok().body(body);
            }
            Err(err) => {
                dbg!(&err);
                let error_msg = "Error Adding Record".to_owned() + format!("{}", is_valid.err().unwrap()).as_str();
                let validation_response = ValidationResponse::from((error_msg.as_str(), "validation_error"));
                let body = hb.render("validation", &validation_response).unwrap();
                return HttpResponse::InternalServerError()
                    .header("HX-Retarget", "#validation_response")
                    .body(body);
            }
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "actix_web=info");
    }
    env_logger::init();
    dotenv().ok();
    let config = config::Config::new();
    let database_url = env::var("DATABASE_URL").unwrap_or("NoURL".to_string());
    // let database_url = env!("DATABASE_URL");
    // let secret = std::env::var("JWT_SECRET").unwrap_or(env!("JWT_SECRET").to_owned());
    let secret = "temp_secret";
    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("✅Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("🔥 Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    let mut con = redis_connect();
    redis_test_data(con);

    // Using GlitchTip. Works with the Rust Sentry SDK
    let _guard = sentry::init("https://ec778decf4e94595b5a48520185298c3@app.glitchtip.com/5073");

    let mut handlebars = Handlebars::new();

    handlebars
        .register_templates_directory(".hbs", "./templates")
        .unwrap();

    handlebars.register_helper("to_title_case", Box::new(to_title_case));
    handlebars.register_helper("str_eq", Box::new(str_eq));
    handlebars.register_helper("int_eq", Box::new(int_eq));
    handlebars.register_helper("int_in", Box::new(int_in));
    handlebars.register_helper("get_month_name", Box::new(get_month_name));
    handlebars.register_helper("first_week", Box::new(first_week));
    handlebars.register_helper("second_week", Box::new(second_week));
    handlebars.register_helper("third_week", Box::new(third_week));
    handlebars.register_helper("fourth_week", Box::new(fourth_week));
    handlebars.register_helper("fifth_week", Box::new(fifth_week));
    handlebars.register_helper("lower_and_single", Box::new(lower_and_single));
    handlebars.register_helper("concat_args", Box::new(concat_args));
    handlebars.register_helper("concat_str_args", Box::new(concat_str_args));
    handlebars.register_helper("loc_vec_len_ten", Box::new(loc_vec_len_ten));
    handlebars.register_helper("form_rte", Box::new(form_rte));
    handlebars.register_helper("sort_rte", Box::new(sort_rte));
    handlebars.register_helper("preview_text", Box::new(preview_text));
    handlebars.register_helper("attachments_rte", Box::new(attachments_rte));
    handlebars.register_helper("subscribe_rte", Box::new(subscribe_rte));
    handlebars.register_helper("subscribe_icon", Box::new(subscribe_icon));
    handlebars.register_helper("get_icon", Box::new(get_icon));
    handlebars.register_helper("is_icon_col", Box::new(is_icon_col));
    handlebars.register_helper("get_search_rte", Box::new(get_search_rte));
    handlebars.register_helper("get_table_title", Box::new(get_table_title));
    handlebars.register_helper("get_list_view", Box::new(get_list_view));

    // handlebars.register_helper("gen_vec_len_ten", Box::new(gen_vec_len_ten));

    handlebars.set_dev_mode(true);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                db: pool.clone(),
                secret: secret.to_string(),
                token: "".to_string().clone(),
            }))
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(handlebars.clone()))
            .service(auth_scope())
            .service(user_scope())
            .service(admin_scope())
            .service(consult_scope())
            .service(consultant_scope())
            .service(location_scope())
            .service(client_scope())
            .service(event_scope())
            .service(send_email)
            .service(contact_us)
            .service(contact_us_submission)
            .service(index)
            .service(about_us)
            .service(fixed_table)
            // .service(responsive_table)
            .service(homepage)
            .service(crud_api)
            .service(list_api)
            .service(schedule_api)
            .service(detail)
            .service(content)
            .service(create_todo)
            .service(
                Files::new("/", "./static")
                    .prefer_utf8(true)
                    .use_last_modified(true),
            )
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hbs_helpers::int_eq,
        test_common::{self, *},
    };
    use test_context::{test_context, TestContext};

    #[test]
    fn contact_us_clean_message_validated() {
        let msg = "Hey there big guy how are you today?";
        let clean = validate_contact_message(msg);
        assert_eq!(clean, Ok(()));
    }

    #[test]
    fn contact_us_dirty_message_validated() {
        let msg = "Hey there big guy shit how are you today?";
        let dirty = validate_contact_message(msg);
        // assert_eq!(dirty, ValidationError {});
        assert_ne!(dirty, Ok(()));
    }

    #[test_context(Context)]
    #[test]
    fn homepage_displays_the_error(ctx: &mut Context) {
        let err_msg = "Test error message".to_string();
        let template_data = HomepageTemplate {
            err: Some(err_msg.clone()),
            user: None,
            feed_data: UserFeedData {
                posts: None,
                consults: None,
                subs: None,
            },
        };
        let mut hb = Handlebars::new();
        hb.register_templates_directory(".hbs", "./templates")
            .unwrap();
        let body = hb.render("homepage", &template_data).unwrap();
        let dom = tl::parse(&body, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();

        let element = dom
            .get_element_by_id("error_text")
            .expect("Failed to find element")
            .get(parser)
            .unwrap();

        assert_eq!(element.inner_text(parser), err_msg);
    }
}
