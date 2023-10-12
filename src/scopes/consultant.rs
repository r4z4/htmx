use actix_web::{
    get,
    post,
    web::{Data, Json, self},
    HttpResponse, Responder, Scope
};

use handlebars::Handlebars;
use lazy_static::lazy_static;
use regex::Regex;


use crate::{AppState, config::{FilterOptions, SelectOptions, ResponseConsultant}, models::consultant::{ConsultantList, ConsultantFormTemplate, ConsultantListResponse}};

lazy_static! {
    static ref RE_USER_NAME: Regex = Regex::new(r"^[a-zA-Z0-9]{4,}$").unwrap();
    static ref RE_SPECIAL_CHAR: Regex = Regex::new("^.*?[@$!%*?&].*$").unwrap();
}

pub fn consultant_scope() -> Scope {
    web::scope("/consultant")
        // .route("/users", web::get().to(get_users_handler))
        .service(consultant_form)
        .service(get_consultants_handler)
}

#[get("/list")]
pub async fn get_consultants_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
) -> impl Responder {
    println!("get_consultants_handler firing");
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let query_result = sqlx::query_as!(
        ResponseConsultant,
        "SELECT consultant_id, specialty_id, consultant_f_name FROM consultants ORDER by consultant_id LIMIT $1 OFFSET $2",
        limit as i32,
        offset as i32
    )
    .fetch_all(&data.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let err = "Error occurred while fetching all consultant records";
        // return HttpResponse::InternalServerError()
        //     .json(json!({"status": "error","message": message}));
        let body = hb.render("error", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let consultants = query_result.unwrap();

    let consultants_response = ConsultantListResponse {
        consultants: consultants,
        name: "Hello".to_owned()
,    };

    let body = hb.render("consultant-list", &consultants_response).unwrap();
    return HttpResponse::Ok().body(body);

}

#[get("/form")]
async fn consultant_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    // path: web::Path<i32>,
) -> impl Responder {
    println!("consultant_form firing");

    let account_result = sqlx::query_as!(
        SelectOptions,
        "SELECT account_id AS value, account_name AS key 
        FROM accounts 
        ORDER by account_name"
    )
    .fetch_all(&state.db)
    .await;

    if account_result.is_err() {
        let err = "Error occurred while fetching account option KVs";
        let body = hb.render("error", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let account_options = account_result.unwrap();

    let template_data = ConsultantFormTemplate {
        account_options: account_options,
    };

    let body = hb.render("consultant/consultant-form", &template_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}