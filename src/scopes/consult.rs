use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse, Responder, Scope,
};

use handlebars::Handlebars;

use crate::{
    config::{FilterOptions, SelectOption, ResponsiveTableData},
    models::model_consult::{
        ConsultFormRequest, ConsultFormTemplate, ConsultList, ConsultPost,
    },
    AppState,
};

pub fn consult_scope() -> Scope {
    web::scope("/consult")
        // .route("/users", web::get().to(get_users_handler))
        .service(consult_form)
        .service(consult_edit_form)
        .service(create_consult)
        .service(get_consults_handler)
}

async fn location_options(state: &web::Data<AppState>) -> Vec<SelectOption> {
    let location_result = sqlx::query_as!(
        SelectOption,
        "SELECT location_id AS value, location_name AS key 
        FROM locations 
        ORDER by location_name"
    )
    .fetch_all(&state.db)
    .await;

    if location_result.is_err() {
        let err = "Error occurred while fetching location option KVs";
        let default_options = SelectOption { 
            key: Some("No Locations Found".to_owned()), 
            value: 0 
        };
        // default_options
        dbg!("Incoming Panic");
    }

    let location_options = location_result.unwrap();
    location_options
}

async fn consultant_options(state: &web::Data<AppState>) -> Vec<SelectOption> {
    let consultant_result = sqlx::query_as!(
        SelectOption,
        "SELECT CONCAT(consultant_f_name, ' ',consultant_l_name) AS key, consultant_id AS value 
        FROM consultants ORDER BY key"
    )
    .fetch_all(&state.db)
    .await;

    if consultant_result.is_err() {
        let err = "Error occurred while fetching location option KVs";
        let default_options = SelectOption { 
            key: Some("No Consultant Found".to_owned()), 
            value: 0 
        };
        // default_options
        dbg!("Incoming Panic");
    }

    let consultant_options = consultant_result.unwrap();
    consultant_options
}

async fn client_options(state: &web::Data<AppState>) -> Vec<SelectOption> {
let client_result = sqlx::query_as!(
        SelectOption,
        "SELECT COALESCE(client_company_name, CONCAT(client_f_name, ' ', client_l_name)) AS key, client_id AS value 
        FROM clients ORDER BY key"
    )
    .fetch_all(&state.db)
    .await;

    if client_result.is_err() {
        let err = "Error occurred while fetching location option KVs";
        let default_options = SelectOption { 
            key: Some("No Clientt Found".to_owned()), 
            value: 0 
        };
        // default_options
        dbg!("Incoming Panic");
    }

    let client_options = client_result.unwrap();
    client_options
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

    let location_options = location_options(&state).await;
    let consultant_options = consultant_options(&state).await;
    let client_options = client_options(&state).await;

    let template_data = ConsultFormTemplate {
        entity: None,
        location_options: location_options,
        consultant_options: consultant_options,
        client_options: client_options,
    };

    let body = hb.render("consult/consult-form", &template_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

#[get("/form/{slug}")]
async fn consult_edit_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    println!("consults_form firing");
    let consult_slug = path.into_inner();

    let query_result = sqlx::query_as!(
        ConsultFormRequest,
        "SELECT consultant_id, location_id, client_id, consult_start, consult_end, notes 
            FROM consults 
            WHERE slug = $1
            ORDER by consult_start",
        consult_slug
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

    let location_options = location_options(&state).await;
    let consultant_options = consultant_options(&state).await;
    let client_options = client_options(&state).await;

    let consult_form_template = ConsultFormTemplate {
        entity: Some(consult),
        location_options: location_options,
        client_options: client_options,
        consultant_options: consultant_options,
    };

    let body = hb.render("consult/consult-form", &consult_form_template).unwrap();
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
        "SELECT consult_id, slug, consultant_id, location_id, client_id, consult_start, consult_end, notes 
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

    let consultants_table_data = ResponsiveTableData {
        entity_type_id: 6,
        vec_len: consults.len(),
        lookup_url: "/consult/list?page=".to_string(),
        page: opts.page.unwrap_or(1),
        entities: consults,
    };

    let body = hb
        .render("responsive-table", &consultants_table_data)
        .unwrap();
    return HttpResponse::Ok().body(body);
}
