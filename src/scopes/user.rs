use actix_web::{
    get,
    post,
    web::{Data, Json, self},
    HttpResponse, Responder, Scope, HttpRequest, HttpMessage, put
};

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{config::{self, SelectOptions}, AppState, models::user::{UserModel, UserSettingsModel, UserHomeModel, UserSettingsPost}, HeaderValueExt};


pub fn user_scope() -> Scope {
    web::scope("/user")
        // .route("/users", web::get().to(get_users_handler))
        .service(home)
        .service(settings)
        .service(profile)
        .service(edit_settings)
}

pub fn theme_options() -> Vec<SelectOptions> {
    [SelectOptions {
            key: Some("classic".to_owned()),
            value: 1
        },
        SelectOptions {
            key: Some("dark".to_owned()),
            value: 2
        }].to_vec()
}

#[get("/settings")]
async fn settings(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    // let user_id = get_user_id_from_token();

    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match sqlx::query_as::<_, UserModel>(
            "SELECT users.user_id, username, email, users.created_at, users.updated_at
            FROM users
            LEFT JOIN user_sessions on user_sessions.user_id = users.user_id 
            WHERE session_id = $1
            AND expires > NOW()",
        )
        .bind(cookie.to_string())
        .fetch_optional(&state.db)
        .await
        {
            Ok(user) => {
                let user_c = user.clone();
                let user_settings_model = UserSettingsModel {
                    theme_options: theme_options(),
                    username: user.unwrap().username,
                    email: user_c.unwrap().email,
                };
                let body = hb.render("user/user-settings", &user_settings_model).unwrap();
                return HttpResponse::Ok()
                .body(body);
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("user/user-settings", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok().body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
        // FIXME: Is this else right? Redirect?
    } else {
        let message = "Your session seems to have expired. Please login again.".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[put("/settings")]
async fn edit_settings(
    hb: web::Data<Handlebars<'_>>,
    body: web::Form<UserSettingsPost>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    match sqlx::query_as::<_, UserSettingsPost>(
        "UPDATE user_settings SET theme_id = $1, username = $2, email = $3 WHERE user_id = $4 RETURNING *",
    )
    .bind(body.theme_id)
    .bind(body.username.clone())
    .bind(body.email.clone())
    .fetch_one(&state.db)
    .await
    {
        Ok(user_settings) => {
            let user_c = user_settings.clone();
            let user_settings_model = UserSettingsModel {
                theme_options: theme_options(),
                username: user_settings.username,
                email: user_c.email,
            };
            let body = hb.render("user/user-settings", &user_settings_model).unwrap();
            return HttpResponse::Ok().body(body);
        }
        Err(err) => {
            let body = hb.render("error", &err.to_string()).unwrap();
            return HttpResponse::Ok().body(body);
        }
    }
}

#[get("/profile")]
async fn profile(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    // let user_id = get_user_id_from_token();
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match sqlx::query_as::<_, UserModel>(
            "SELECT users.user_id, username, email, users.created_at, users.updated_at
            FROM users
            LEFT JOIN user_sessions on user_sessions.user_id = users.user_id 
            WHERE session_id = $1
            AND expires > NOW()",
        )
        .bind(cookie.to_string())
        .fetch_optional(&state.db)
        .await
        {
            Ok(user) => {
                dbg!(&user);
                let body = hb.render("user/user-profile", &user).unwrap();
                return HttpResponse::Ok()
                .body(body);
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("user/user-profile", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok().body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
        // FIXME: Is this else right? Redirect?
    } else {
        let message = "Your session seems to have expired. Please login again.".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
}


#[get("/home")]
async fn home(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    // let user_id = get_user_id_from_token();
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match sqlx::query_as::<_, UserModel>(
            "SELECT users.user_id, username, email, users.created_at, users.updated_at
            FROM users
            LEFT JOIN user_sessions on user_sessions.user_id = users.user_id 
            WHERE session_id = $1
            AND expires > NOW()",
        )
        .bind(cookie.to_string())
        .fetch_optional(&state.db)
        .await
        {
            Ok(user) => {
                let unwrapped_user = user.unwrap();
                let user_home_model = UserHomeModel {
                    username: unwrapped_user.username,
                    created_at: unwrapped_user.created_at,
                    email: unwrapped_user.email,
                };
                let body = hb.render("user-home", &user_home_model).unwrap();
                return HttpResponse::Ok()
                .body(body);
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("user-home", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok().body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
        // FIXME: Is this else right? Redirect?
    } else {
        let message = "Your session seems to have expired. Please login again.".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HbError {
    str: String,
}

// Move this to Redis at some point
// async fn get_id_from_cookie(cookie: &actix_web::http::header::HeaderValue, state: Data<AppState>,) -> Result<Option<ValidatedUser>, ValidationError>{
//     println!("Getting Id From Cookie {}", format!("{:?}", cookie.clone()));
//     match sqlx::query_as::<_, ValidatedUser>(
//         "SELECT username, email, created_at, updated_at
//         FROM users
//         LEFT JOIN user_sessions on user_sessions.user_id = users.user_id 
//         WHERE session_id = $1
//         AND expires > NOW()",
//     )
//     .bind(cookie.to_string())
//     .fetch_optional(&state.db)
//     .await
//     {
//         Ok(user_option) => Ok(user_option),
//         Err(err) => Err(ValidationError {
//             error: format!("You must not be verfied: {}", err)
//         }),
//     }
// }