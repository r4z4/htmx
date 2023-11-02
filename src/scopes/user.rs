use crate::{
    config::{self, SelectOption, ValidationResponse},
    models::model_user::{
        UserHomeModel, UserHomeQuery, UserModel, UserSettingsModel, UserSettingsObj,
        UserSettingsPost, UserSettingsQuery,
    },
    AppState, HeaderValueExt, ValidatedUser,
};
use actix_web::{
    get, post, put,
    web::{self, Data, Json},
    HttpMessage, HttpRequest, HttpResponse, Responder, Scope,
};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub fn user_scope() -> Scope {
    web::scope("/user")
        // .route("/users", web::get().to(get_users_handler))
        .service(home)
        .service(settings)
        //.service(profile)
        .service(edit_settings)
}

pub fn theme_options() -> Vec<SelectOption> {
    [
        SelectOption {
            key: Some("classic".to_owned()),
            value: 1,
        },
        SelectOption {
            key: Some("dark".to_owned()),
            value: 2,
        },
    ]
    .to_vec()
}

pub fn list_view_options() -> Vec<SelectOption> {
    [
        SelectOption {
            key: Some("consult".to_owned()),
            value: 1,
        },
        SelectOption {
            key: Some("consultant".to_owned()),
            value: 2,
        },
        SelectOption {
            key: Some("client".to_owned()),
            value: 3,
        },
        SelectOption {
            key: Some("location".to_owned()),
            value: 4,
        },
    ]
    .to_vec()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsFormTemplate {
    entity: Option<UserSettingsObj>,
    theme_options: Vec<SelectOption>,
    list_view_options: Vec<SelectOption>,
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
                        updated_at_fmt: updated_at_fmt,
                        username: usr.username,
                    };
                    let template_data = SettingsFormTemplate {
                        entity: Some(user_settings_obj),
                        theme_options: theme_options(),
                        list_view_options: list_view_options(),
                    };
                    let body = hb.render("user/user-settings", &template_data).unwrap();
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

fn validate_user_settings_input(body: &UserSettingsPost) -> bool {
    true
}

#[put("/settings")]
async fn edit_settings(
    hb: web::Data<Handlebars<'_>>,
    body: web::Form<UserSettingsPost>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    if validate_user_settings_input(&body) {
        match sqlx::query_as::<_, UserHomeQuery>(
            "UPDATE user_settings SET theme_id = $1 WHERE user_id = $2 RETURNING (
                SELECT users.user_id, username, email, user_type_id, users.created_at, users.updated_at, COALESCE(avatar_path, '/images/default_avatar.svg') AS avatar_path, user_settings.updated_at AS settings_updated
                FROM users
                LEFT JOIN user_settings on user_settings.user_id = users.user_id
                WHERE user_id = $2
            )",
        )
        .bind(body.theme_id)
        .bind(body.user_id.clone())
        .fetch_one(&state.db)
        .await
        {
            Ok(user) => {
                let user_home_model = UserHomeModel {
                    user_id: user.user_id,
                    user_type_id: user.user_type_id,
                    username: user.username,
                    theme_options: theme_options(),
                    list_view_options: list_view_options(),
                    avatar_path: user.avatar_path,
                    settings_updated: user.settings_updated.format("%b %-d, %-I:%M").to_string(),
                    created_at_fmt: user.created_at.format("%b %-d, %-I:%M").to_string(),
                    updated_at_fmt: user.updated_at.format("%b %-d, %-I:%M").to_string(),
                    email: user.email,
                };
                let template_data = json! {{
                    // Using an hx-get & swap so user should already be in the main-layout?
                    // "user": &validated_user,
                    "data": &user_home_model,
                }};
                let body = hb.render("user-home", &template_data).unwrap();
                return HttpResponse::Ok()
                .body(body);
            }
            // If error on DB level, take user back to user-home w/ user alert, vs a form validation response on the form itself
            Err(err) => {
                dbg!(&err);
                let body = hb.render("user-home", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok()
                .body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
    } else {
        let validation_response = ValidationResponse {
            msg: "Validation error".to_owned(),
            class: "validation_error".to_owned(),
        };
        let body = hb.render("validation", &format!("{:?}", validation_response)).unwrap();
        return HttpResponse::Ok()
        .header("HX-Retarget", "#validation")
        .body(body);
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
            "SELECT users.user_id, username, email, user_type_id, users.created_at, users.updated_at, COALESCE(avatar_path, '/images/default_avatar.svg') AS avatar_path, user_settings.updated_at AS settings_updated, user_settings.list_view
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
                let validated_user = ValidatedUser {
                    username: unwrapped_user.username.clone(),
                    email: unwrapped_user.email.clone(),
                    user_type_id: unwrapped_user.user_type_id,
                    list_view: unwrapped_user.list_view,
                };
                let user_home_model = UserHomeModel {
                    user_id: unwrapped_user.user_id,
                    user_type_id: unwrapped_user.user_type_id,
                    username: unwrapped_user.username,
                    theme_options: theme_options(),
                    list_view_options: list_view_options(),
                    avatar_path: unwrapped_user.avatar_path,
                    settings_updated: unwrapped_user.settings_updated.format("%b %-d, %-I:%M").to_string(),
                    created_at_fmt: unwrapped_user.created_at.format("%b %-d, %-I:%M").to_string(),
                    updated_at_fmt: unwrapped_user.updated_at.format("%b %-d, %-I:%M").to_string(),
                    email: unwrapped_user.email,
                };
                let template_data = json! {{
                    "user": &validated_user,
                    "data": &user_home_model,
                }};
                let body = hb.render("user-home", &template_data).unwrap();
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

// Why doesn't this work?
impl From<String> for HbError {
    fn from(item: String) -> Self {
        HbError { str: item }
    }
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
