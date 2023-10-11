use actix_web::{
    get,
    post,
    web::{Data, Json, self},
    HttpResponse, Responder, Scope, HttpRequest, HttpMessage
};

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::{config, AppState, models::user::UserModel, HeaderValueExt};


pub fn user_scope() -> Scope {
    web::scope("/user")
        // .route("/users", web::get().to(get_users_handler))
        .service(settings)
        //.service(edit_form)
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
                dbg!(&user);
                let body = hb.render("user-settings", &user).unwrap();
                return HttpResponse::Ok()
                .body(body);
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("user-settings", &format!("{:?}", err)).unwrap();
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