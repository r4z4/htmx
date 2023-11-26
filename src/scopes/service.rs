use actix_web::web::{Data, Form};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, Scope};
use serde_json::json;
use sqlx::FromRow;

use crate::{
    config::{
        validate_and_get_user,
    },
    AppState,
};
use handlebars::Handlebars;

pub fn service_scope() -> Scope {
    web::scope("/service")
        // .route("/users", web::get().to(get_users_handler))
        .service(home)
    //.service(prev_month)
}

#[get("/")]
async fn home(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: Data<AppState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match validate_and_get_user(cookie, &state).await {
            Ok(user) => {
                if let Some(usr) = user {
                    // Homepage displays current Mo/Yr
                    let data = json!({ 
                        "success": true
                    });
                    let body = hb.render("service-api", &data).unwrap();
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
            }
        }
    } else {
        let message = "Your session seems to have expired. Please login again (3).".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
}