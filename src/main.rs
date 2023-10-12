use std::env;
use config::Post;
use dotenv::dotenv;
use actix_files::Files;
use actix_web::{get, web::{self, post, Data}, App, HttpResponse, HttpServer, Responder, post, http::{Error, header::{Header, HeaderValue}}, HttpRequest};
use handlebars::Handlebars;
use serde::{Serialize, Deserialize};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, Postgres, Pool, FromRow};
use validator::Validate;

use crate::{scopes::auth::ResponseUser};

use scopes::{auth::auth_scope, consult::consult_scope, consultant::consultant_scope, user::user_scope};
mod config;
mod scopes;
mod models;

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

#[get("/")]
async fn index(hb: web::Data<Handlebars<'_>>, data: web::Data<AppState>, state: Data<AppState>, req: HttpRequest, config: web::Data<config::Config>) -> impl Responder {
    let headers = req.headers();
    for (pos, e) in headers.iter().enumerate() {
        println!("Element at position {}: {:?}", pos, e);
    }
    if let Some(cookie) = headers.get(actix_web::http::header::COOKIE) {
        dbg!(cookie.clone());
        match validate_and_get_user(cookie, state).await {
            Ok(user_option) => {
                if let Some(user) = user_option { 
                    let user = ResponseUser {
                        username: "Jim".to_owned(),
                        email: "Jim@jim.com".to_owned(),
                    };
                    let body = hb.render("homepage", &user).unwrap();
                    return HttpResponse::Ok()
                        .header("HX-Redirect", "/homepage")
                        .body(body);
                } else {
                    let message = "Your session seems to have expired. Please login again.".to_owned();
                    let body = hb.render("index", &message).unwrap();
                
                    HttpResponse::Ok().body(body)
                }
            },
            Err(_err) => {
                // User's cookie is invalud or expired. Need to get a new one via logging in.
                // They had a session. Could give them details about that. Get from DB.
                let message = "Error in validate and get user.".to_owned();
                let body = hb.render("index", &message).unwrap();
            
                HttpResponse::Ok().body(body)
            },
        }
    } else {
        let data = json!({
            "header": "Login Form",
        });
        let body = hb.render("index", &data).unwrap();

        HttpResponse::Ok().body(body)
    }
}

#[get("/about-us")]
async fn about_us(hb: web::Data<Handlebars<'_>>) -> impl Responder {
    let data = json!({
        "name": "ExtRev",
        "title": "Best",
    });
    let body = hb.render("about-us", &data).unwrap();

    HttpResponse::Ok().body(body)
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HbError {
    str: String,
}

#[get("/crud")]
async fn crud_api(hb: web::Data<Handlebars<'_>>) -> impl Responder {
    let data = json!({
        "name": "CRUD Ops",
        "title": "Create / Remove / Update / Delete",
    });
    let body = hb.render("crud-api", &data).unwrap();

    HttpResponse::Ok().body(body)
}

#[get("/list")]
async fn list_api(hb: web::Data<Handlebars<'_>>) -> impl Responder {
    let data = json!({
        "name": "Lists",
        "title": "View Records",
    });
    let body = hb.render("list-api", &data).unwrap();

    HttpResponse::Ok().body(body)
}

#[get("/homepage")]
async fn homepage(hb: web::Data<Handlebars<'_>>, state: Data<AppState>, req: HttpRequest) -> impl Responder {
    dbg!(&req);
    // FIXME unwrap()
    let headers = req.headers();
    dbg!(&headers);
    if let Some(cookie) = headers.get(actix_web::http::header::COOKIE) {
        dbg!(&cookie);
        match validate_and_get_user(cookie, state).await {
            Ok(user_option) => {
                if let Some(user) = user_option {
                    let body = hb.render("homepage", &user).unwrap();
                    dbg!(&body);
                    HttpResponse::Ok().body(body)
                } else {
                    let data = HbError {
                        str: "Seems your session has expired. Please login again".to_owned()
                    };
                    let body = hb.render("homepage", &data).unwrap();
                    HttpResponse::Ok().body(body)
                }
            },
            Err(err) => {
                // User's cookie is invalud or expired. Need to get a new one via logging in.
                // They had a session. Could give them details about that. Get from DB.
                let data = HbError {
                    str: format!("Something quite unexpected has happened in your session: {}", err.error)
                };
                let body = hb.render("homepage", &data).unwrap();
                HttpResponse::Ok().body(body)
            }
        }
    } else {
        let data = HbError {
            str: "Cookie is missing.".to_owned()
        };
        let body = hb.render("homepage", &data).unwrap();
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
    // current(hb, config, state, req, path.into_inner())
    let data = ArticleData {
        title: config.title.clone(),
        description: config.description.clone(),
        posts: config.posts.clone(),
    };

    let body = hb.render("articles", &data).unwrap();

    HttpResponse::Ok().body(body)
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
        let err = "Error retrieving content".to_owned();
        let body = hb.render("error", &err).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[derive(Debug, Validate, Serialize, Deserialize)]
pub struct ValidationError {
    error: String,
}
#[derive(Debug, FromRow, Validate, Serialize, Deserialize)]
pub struct ValidatedUser {
    username: String,
    email: String,
}

pub trait HeaderValueExt {
    fn to_string(&self) -> String;
  }
  
  impl HeaderValueExt for HeaderValue {
    fn to_string(&self) -> String {
      self.to_str().unwrap_or_default().to_string()
    }
  }

async fn validate_and_get_user(cookie: &actix_web::http::header::HeaderValue, state: Data<AppState>,) -> Result<Option<ValidatedUser>, ValidationError>{
    println!("Validating {}", format!("{:?}", cookie.clone()));
    match sqlx::query_as::<_, ValidatedUser>(
        "SELECT username, email 
        FROM users
        LEFT JOIN user_sessions on user_sessions.user_id = users.user_id 
        WHERE session_id = $1
        AND expires > NOW()",
    )
    .bind(cookie.to_string())
    .fetch_optional(&state.db)
    .await
    {
        Ok(user_option) => Ok(user_option),
        Err(err) => Err(ValidationError {
            error: format!("You must not be verfied: {}", err)
        }),
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
    // let query_result = sqlx::query_as!(
    //     EngagementModel,
    //     "INSERT INTO engagements (text,rating) VALUES ($1, $2) RETURNING *",
    //     body.text.to_string(),
    //     body.rating,
    // )
    // .fetch_one(&data.db)
    // .await;
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
        Err(err) => {
            let body = hb.render("error", &err).unwrap();
            return HttpResponse::Ok().body(body);
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

    let mut handlebars = Handlebars::new();

    handlebars
        .register_templates_directory(".hbs", "./templates")
        .unwrap();

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
            .service(consult_scope())
            .service(consultant_scope())
            .service(index)
            .service(about_us)
            .service(homepage)
            .service(crud_api)
            .service(list_api)
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
