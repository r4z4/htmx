use crate::{
    config::{SelectOption, ValidationResponse, category_options, validate_and_get_user, UserAlert},
    models::model_user::{
        UserHomeModel, UserHomeQuery, UserSettingsObj,
        UserSettingsPost, UserSettingsQuery,
    },
    AppState, HeaderValueExt, ValidatedUser, scopes::location::IndexData,
};
use actix_web::{
    get, put,
    web::{self},
    HttpRequest, HttpResponse, Responder, Scope,
};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, Pool, Postgres};

pub fn user_scope() -> Scope {
    web::scope("/user")
        // .route("/users", web::get().to(get_users_handler))
        .service(home)
        .service(settings)
        .service(compose)
        .service(subscribe)
        //.service(profile)
        .service(edit_settings)
}

pub fn theme_options() -> Vec<SelectOption> {
    [
        SelectOption::from((1, Some("classic".to_string()))),
        SelectOption::from((2, Some("dark".to_string()))),
    ]
    .to_vec()
}

pub fn list_view_options() -> Vec<SelectOption> {
    [
        SelectOption::from((1, Some("consult".to_string()))),
        SelectOption::from((2, Some("consultant".to_string()))),
        SelectOption::from((3, Some("client".to_string()))),
        SelectOption::from((4, Some("location".to_string()))),
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComposeTemplate {
    category_options: Vec<SelectOption>,
    typ: String,
}

#[get("/compose")]
async fn compose(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {  
    let template_data = ComposeTemplate {
        typ: "article".to_owned(),
        category_options: category_options(&state.db).await,
    };

    let body = hb
        .render("compose", &template_data)
        .unwrap();
    return HttpResponse::Ok().body(body);
}

pub fn get_sub_sql(subscribed: bool, entity_id: i32, entity_type_id: i32) -> String {
    match entity_type_id {
        1 => {
            if subscribed {
                format!("UPDATE users SET user_subs = ARRAY_REMOVE(user_subs, {}) WHERE username = $1 RETURNING username", entity_id)
            } else {
                format!("UPDATE users SET user_subs = ARRAY_APPEND(user_subs, {}) WHERE username = $1 RETURNING username", entity_id)
            }
        },
        2 => {
            if subscribed {
                format!("UPDATE users SET user_subs = ARRAY_REMOVE(user_subs, {}) WHERE username = $1 RETURNING username", entity_id)
            } else {
                format!("UPDATE users SET user_subs = ARRAY_APPEND(user_subs, {}) WHERE username = $1 RETURNING username", entity_id)
            }
        },
        3 => {
            if subscribed {
                format!("UPDATE users SET user_subs = ARRAY_REMOVE(user_subs, {}) WHERE username = $1 RETURNING username", entity_id)
            } else {
                format!("UPDATE users SET user_subs = ARRAY_APPEND(user_subs, {}) WHERE username = $1 RETURNING username", entity_id)
            }
        },
        4 => "UPDATE users SET consultant_subs = ARRAY_APPEND(consultant_subs, (SELECT consultant_id FROM consultants WHERE slug = $1)) WHERE username = $2 RETURNING username".to_string(),
        5 => {
            if subscribed {
                format!("UPDATE users SET location_subs = ARRAY_REMOVE(location_subs, {}) WHERE username = $1 RETURNING username", entity_id)
            } else {
                format!("UPDATE users SET location_subs = ARRAY_APPEND(location_subs, {}) WHERE username = $1 RETURNING username", entity_id)
            }
        },
        6 => "UPDATE users SET consult_subs = ARRAY_APPEND(consult_subs, (SELECT consult_id FROM consults WHERE slug = $1)) WHERE username = $2 RETURNING username".to_string(),
        7 => "UPDATE users SET client_subs = ARRAY_APPEND(client_subs, (SELECT client_id FROM clients WHERE slug = $1)) WHERE username = $2 RETURNING username".to_string(),
        _ => "UPDATE users SET user_subs = ARRAY_APPEND(user_subs, (SELECT user_id FROM users WHERE slug = $1)) WHERE username = $2 RETURNING username".to_string(),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct EntityId {
    id: i32
}

async fn get_id(id: i32, slug: &str, pool: &Pool<Postgres>) -> i32 {
    match id {
        1 => sqlx::query_as::<_, EntityId>("SELECT user_id AS id FROM users WHERE slug = $1").bind(&slug).fetch_one(pool).await.unwrap().id,
        2 => sqlx::query_as::<_, EntityId>("SELECT user_id AS id FROM users WHERE slug = $1").bind(&slug).fetch_one(pool).await.unwrap().id,
        3 => sqlx::query_as::<_, EntityId>("SELECT user_id AS id FROM users WHERE slug = $1").bind(&slug).fetch_one(pool).await.unwrap().id,
        4 => sqlx::query_as::<_, EntityId>("SELECT consultant_id AS id FROM consultants WHERE slug = $1").bind(&slug).fetch_one(pool).await.unwrap().id,
        5 => sqlx::query_as::<_, EntityId>("SELECT location_id AS id FROM locations WHERE slug = $1").bind(&slug).fetch_one(pool).await.unwrap().id,
        6 => sqlx::query_as::<_, EntityId>("SELECT consult_id AS id FROM consults WHERE slug = $1").bind(&slug).fetch_one(pool).await.unwrap().id,
        7 => sqlx::query_as::<_, EntityId>("SELECT client_id AS id FROM clients WHERE slug = $1").bind(&slug).fetch_one(pool).await.unwrap().id,
        _ => sqlx::query_as::<_, EntityId>("SELECT user_id AS id FROM users WHERE slug = $1").bind(&slug).fetch_one(pool).await.unwrap().id,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct SubscribeResponse {
    username: String
}

#[get("/subscribe/{entity_type_id}/{entity_slug}")]
async fn subscribe(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<(i32, String)>,
) -> impl Responder {  
    let (entity_type_id, slug) = path.into_inner();
    let headers = req.headers();
    if let Some(cookie) = headers.get(actix_web::http::header::COOKIE) {
        dbg!(cookie.clone());
        match validate_and_get_user(cookie, &state).await {
            Ok(user_option) => {
                dbg!(&user_option);
                let user = user_option.clone().unwrap();
                let username = user.username;

                let entity_id = get_id(entity_type_id, &slug, &state.db).await;

                let subscribed =
                    match entity_type_id {
                        1 => user.user_subs.contains(&entity_id),
                        2 => user.user_subs.contains(&entity_id),
                        3 => user.user_subs.contains(&entity_id),
                        4 => user.consultant_subs.contains(&entity_id), 
                        5 => user.location_subs.contains(&entity_id),
                        6 => user.consult_subs.contains(&entity_id),
                        7 => user.client_subs.contains(&entity_id),
                        _ => user.user_subs.contains(&entity_id),
                    };

                let sql = get_sub_sql(subscribed, entity_id, entity_type_id);
                match sqlx::query_as::<_, SubscribeResponse>(
                    &sql,
                )
                .bind(&username)
                .fetch_one(&state.db)
                .await
                {
                    Ok(resp) => {
                        let user_alert = UserAlert {
                            msg: format!("Subscription {} successfully", {if subscribed {"removed"} else {"added"}}),
                            class: "alert_success".to_owned(),
                        };
                        let template_body = hb.render("user-alert", &user_alert).unwrap();
                        return HttpResponse::Ok().body(template_body);
                    }
                    Err(err) => {
                        dbg!(&err);
                        let user_alert = UserAlert {
                            msg: format!("Error adding subscription: {:?}", err),
                            class: "alert_error".to_owned(),
                        };
                        let body = hb.render("user-alert", &user_alert).unwrap();
                        return HttpResponse::Ok().body(body);
                    }
                }
            },
            Err(_err) => {
                dbg!(&_err);
                // User's cookie is invalid or expired. Need to get a new one via logging in.
                // They had a session. Could give them details about that. Get from DB.
                let index_data = IndexData {
                    message: format!("Error in validate and get user: {}", _err.error)
                };
                let body = hb.render("index", &index_data).unwrap();

                HttpResponse::Ok().body(body)
            }
        }
    } else {
        let data = json!({
            "header": "Login Form",
        });
        let body = hb.render("index", &data).unwrap();

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
        let validation_response = ValidationResponse::from(("Validation error", "validation_error"));
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
            "SELECT users.user_id, username, email, user_type_id, user_subs, client_subs, consult_subs, location_subs, consultant_subs, users.created_at, users.updated_at, 
                    COALESCE(avatar_path, '/images/default_avatar.svg') AS avatar_path, user_settings.updated_at AS settings_updated, user_settings.list_view
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
                    user_subs: unwrapped_user.user_subs,
                    client_subs: unwrapped_user.client_subs,
                    consult_subs: unwrapped_user.consult_subs,
                    location_subs: unwrapped_user.location_subs,
                    consultant_subs: unwrapped_user.consultant_subs,
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
