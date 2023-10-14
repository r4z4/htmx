use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse, Responder, Scope,
};

use handlebars::Handlebars;
use std::{
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use validator::Validate;

use crate::{
    config::{FilterOptions, SelectOptions},
    models::consult::{
        ConsultFormRequest, ConsultFormTemplate, ConsultList, ConsultListResponse, ConsultPost,
    },
    AppState,
};

pub fn consult_scope() -> Scope {
    web::scope("/consult")
        // .route("/users", web::get().to(get_users_handler))
        .service(consult_form)
        .service(create_consult)
        .service(get_consults_handler)
}

#[post("/form")]
async fn create_consult(
    body: web::Form<ConsultPost>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
) -> impl Responder {
    match sqlx::query_as::<_, ConsultPost>(
        "INSERT INTO consults (consultant_id, client_id, location_id, consult_start, consult_end, notes) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
    )
    .bind(body.consultant_id)
    .bind(body.client_id)
    .bind(body.location_id)
    .bind(body.consult_start)
    .bind(body.consult_end)
    .bind(body.notes.clone())
    .fetch_one(&state.db)
    .await
    {
        Ok(consult) => {
            let body = hb.render("consult/consult-list", &{}).unwrap();
            return HttpResponse::Ok().body(body);
        }
        Err(err) => {
            let body = hb.render("validation", &err.to_string()).unwrap();
            return HttpResponse::Ok().body(body);
        }
    }
}

#[get("/form")]
async fn consult_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    // path: web::Path<i32>,
) -> impl Responder {
    println!("consults_form firing");

    let location_result = sqlx::query_as!(
        SelectOptions,
        "SELECT location_id AS value, location_name AS key 
        FROM locations 
        ORDER by location_name"
    )
    .fetch_all(&state.db)
    .await;

    if location_result.is_err() {
        let err = "Error occurred while fetching location option KVs";
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let location_options = location_result.unwrap();

    let client_result = sqlx::query_as!(
        SelectOptions,
        "SELECT COALESCE(client_company_name, CONCAT(client_f_name, ' ', client_l_name)) AS key, client_id AS value 
        FROM clients ORDER BY key"
    )
    .fetch_all(&state.db)
    .await;

    if client_result.is_err() {
        let err = "Error occurred while fetching location option KVs";
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let client_options = client_result.unwrap();

    let consultant_result = sqlx::query_as!(
        SelectOptions,
        "SELECT CONCAT(consultant_f_name, ' ',consultant_l_name) AS key, consultant_id AS value 
        FROM consultants ORDER BY key"
    )
    .fetch_all(&state.db)
    .await;

    if consultant_result.is_err() {
        let err = "Error occurred while fetching location option KVs";
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let consultant_options = consultant_result.unwrap();

    let template_data = ConsultFormTemplate {
        location_options: location_options,
        consultant_options: consultant_options,
        client_options: client_options,
    };

    let body = hb.render("consult/consult-form", &template_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

#[get("/form/{consult_id}")]
async fn consult_edit_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<i32>,
) -> impl Responder {
    println!("consults_form firing");
    let consult_id = path.into_inner();

    let query_result = sqlx::query_as!(
        ConsultFormRequest,
        "SELECT consultant_id, location_id, client_id, consult_start, consult_end, notes 
            FROM consults 
            WHERE consult_id = $1
            ORDER by consult_id",
        consult_id
    )
    .fetch_one(&state.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let err = "Error occurred while fetching all consult records";
        // return HttpResponse::InternalServerError()
        //     .json(json!({"status": "error","message": message}));
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let consult = query_result.unwrap();

    let body = hb.render("consult/consult-form", &consult).unwrap();
    return HttpResponse::Ok().body(body);
}

#[get("/list")]
pub async fn get_consults_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
) -> impl Responder {
    println!("get_consultants_handler firing");
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let query_result = sqlx::query_as!(
        ConsultList,
        "SELECT consult_id, consultant_id, location_id, client_id, consult_start, consult_end, notes 
        FROM consults
        ORDER by updated_at, created_at 
        LIMIT $1 OFFSET $2",
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
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let consults = query_result.unwrap();

    let consults_response = ConsultListResponse {
        consults: consults,
        name: "Hello".to_owned(),
    };

    let body = hb
        .render("consult/consult-list", &consults_response)
        .unwrap();
    return HttpResponse::Ok().body(body);
}
