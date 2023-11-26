use actix_web::web::{Data, Form};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, Scope};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use ics::properties::{Categories, Description, DtEnd, DtStart, Organizer, Status, Summary};
use ics::{escape_text, Event, ICalendar};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::model_consult::ConsultPost;
use crate::{
    config::{
        self, get_validation_response, subs_from_user, test_subs, validate_and_get_user,
        FilterOptions, ResponsiveTableData, SelectOption, UserAlert, ValidationResponse,
        ACCEPTED_SECONDARIES,
    },
    models::model_location::{
        LocationFormRequest, LocationFormTemplate, LocationList, LocationPostRequest,
    },
    AppState, ValidatedUser,
};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use validator::Validate;

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