use actix_web::{
    get,
    post,
    web::{Data, Json, self},
    HttpResponse, Responder, Scope, HttpRequest, HttpMessage, put
};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use crate::{config::{self, SelectOptions}, AppState, models::user::{UserModel, UserSettingsModel, UserHomeModel, UserSettingsPost, UserSettingsObj, UserSettingsQuery, UserHomeQuery}, HeaderValueExt};


pub fn user_scope() -> Scope {
    web::scope("/user")
        // .route("/users", web::get().to(get_users_handler))
        .service(home)
        .service(settings)
        //.service(profile)
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
        match sqlx::query_as::<_, UserSettingsQuery>(
            "SELECT users.user_id, username, email, users.created_at, users.updated_at AS user_updated, user_settings.updated_at AS settings_updated
            FROM users
            LEFT JOIN user_sessions on user_sessions.user_id = users.user_id 
            LEFT JOIN user_settings on user_settings.user_id = users.user_id
            WHERE session_id = $1
            AND expires > NOW()",
        )
        .bind(cookie.to_string())
        .fetch_optional(&state.db)
        .await
        {
            Ok(user) => {
                if let Some(usr) = user {
                    let usr_c = usr.clone();
                    let updated_at_fmt = usr_c.settings_updated.format("%b %-d, %-I:%M").to_string();
                    let user_settings_obj = UserSettingsObj {
                        theme_options: theme_options(),
                        updated_at_fmt: updated_at_fmt,
                    };
                    let body = hb.render("user/user-settings", &user_settings_obj).unwrap();
                    return HttpResponse::Ok()
                    .body(body);
                } else {
                    let message = "Cannot find you";
                    let body = hb.render("user/user-settings", &message).unwrap();
                    return HttpResponse::Ok()
                    .body(body);
                }
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
    match sqlx::query_as::<_, UserSettingsModel>(
        "UPDATE user_settings SET theme_id = $1 WHERE user_id = $4 RETURNING *",
    )
    .bind(body.theme_id)
    .bind(body.username.clone())
    .bind(body.email.clone())
    .fetch_one(&state.db)
    .await
    {
        Ok(user_settings) => {
            let user_c = user_settings.clone();
            let updated_at_fmt = user_c.updated_at.format("%b %-d, %-I:%M").to_string();
            let user_settings_obj = UserSettingsObj {
                theme_options: theme_options(),
                updated_at_fmt: updated_at_fmt
            };
            let body = hb.render("user/user-settings", &user_settings_obj).unwrap();
            return HttpResponse::Ok().body(body);
        }
        Err(err) => {
            let body = hb.render("error", &err.to_string()).unwrap();
            return HttpResponse::Ok().body(body);
        }
    }
}

// #[get("/profile")]
// async fn profile(
//     hb: web::Data<Handlebars<'_>>,
//     req: HttpRequest,
//     state: web::Data<AppState>,
// ) -> impl Responder {
//     // let user_id = get_user_id_from_token();
//     if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
//         match sqlx::query_as::<_, UserProfileModel>(
//             "SELECT users.user_id, username, email, 
//             TO_CHAR(users.created_at, 'YYYY/MM/DD HH:MI:SS') AS created_at_fmt, 
//             TO_CHAR(users.updated_at, 'YYYY/MM/DD HH:MI:SS') AS updated_at_fmt
//             FROM users
//             LEFT JOIN user_sessions on user_sessions.user_id = users.user_id 
//             WHERE session_id = $1
//             AND expires > NOW()",
//         )
//         .bind(cookie.to_string())
//         .fetch_optional(&state.db)
//         .await
//         {
//             Ok(user) => {
//                 dbg!(&user);
//                 let body = hb.render("user/user-profile", &user).unwrap();
//                 return HttpResponse::Ok()
//                 .body(body);
//             }
//             Err(err) => {
//                 dbg!(&err);
//                 let body = hb.render("user/user-profile", &format!("{:?}", err)).unwrap();
//                 return HttpResponse::Ok().body(body);
//                 // HttpResponse::InternalServerError().json(format!("{:?}", err))
//             }
//         }
//         // FIXME: Is this else right? Redirect?
//     } else {
//         let message = "Your session seems to have expired. Please login again.".to_owned();
//         let body = hb.render("index", &message).unwrap();
//         HttpResponse::Ok().body(body)
//     }
// }


#[get("/home")]
async fn home(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    // let user_id = get_user_id_from_token();
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match sqlx::query_as::<_, UserHomeQuery>(
            "SELECT users.user_id, username, email, users.created_at, users.updated_at, avatar_path, user_settings.updated_at AS settings_updated
                -- TO_CHAR(users.created_at, 'YYYY/MM/DD HH:MI:SS') AS created_at_fmt, 
                -- TO_CHAR(users.updated_at, 'YYYY/MM/DD HH:MI:SS') AS updated_at_fmt
            FROM users
            LEFT JOIN user_sessions on user_sessions.user_id = users.user_id
            LEFT JOIN user_settings on user_settings.user_id = users.user_id
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
                    user_id: unwrapped_user.user_id,
                    username: unwrapped_user.username,
                    theme_options: theme_options(),
                    avatar_path: unwrapped_user.avatar_path,
                    settings_updated: unwrapped_user.settings_updated.format("%b %-d, %-I:%M").to_string(),
                    created_at_fmt: unwrapped_user.created_at.format("%b %-d, %-I:%M").to_string(),
                    updated_at_fmt: unwrapped_user.updated_at.format("%b %-d, %-I:%M").to_string(),
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