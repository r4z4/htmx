use actix_web::{
    get, post,
    web::{self, Data},
    HttpResponse, Responder, Scope,
};

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::{
    config::{FilterOptions, SelectOption, ResponsiveTableData, admin_user_options, specialty_options, territory_options},
    models::model_consultant::{ConsultantFormTemplate, ResponseConsultant, ConsultantFormRequest},
    AppState,
};

pub fn consultant_scope() -> Scope {
    web::scope("/consultant")
        // .route("/users", web::get().to(get_users_handler))
        .service(consultant_form)
        .service(consultant_edit_form)
        .service(get_consultants_handler)
}
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ResponsiveConsultantData {
    // table_headers: Vec<String>,
    table_title: String,
    entities: Vec<ResponseConsultant>,
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

    if let Some(like) = &opts.search {
        let search_sql = format!("%{}%", like);
        let query_result = sqlx::query_as!(
            ResponseConsultant,
            "SELECT 
                consultant_id,
                slug,
                specialty_name,
                territory_name,
                consultant_f_name,
                consultant_l_name
            FROM consultants
            INNER JOIN specialties ON specialties.specialty_id = consultants.specialty_id
            INNER JOIN territories ON territories.territory_id = consultants.territory_id
            WHERE consultant_f_name LIKE $3
            OR consultant_l_name LIKE $3
            ORDER by consultant_id 
            LIMIT $1 OFFSET $2",
            limit as i32,
            offset as i32,
            search_sql
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

        let consultants = query_result.unwrap();

        let consultants_table_data = ResponsiveTableData {
            entity_type_id: 4,
            vec_len: consultants.len(),
            lookup_url: "/consultant/list?page=".to_string(),
            page: opts.page.unwrap_or(1),
            entities: consultants,
        };

        let body = hb
            .render("responsive-table-inner", &consultants_table_data)
            .unwrap();
        return HttpResponse::Ok().body(body);
    } else {
        let query_result = sqlx::query_as!(
            ResponseConsultant,
            "SELECT 
                consultant_id,
                slug,
                specialty_name,
                territory_name,
                consultant_f_name,
                consultant_l_name
            FROM consultants
            INNER JOIN specialties ON specialties.specialty_id = consultants.specialty_id
            INNER JOIN territories ON territories.territory_id = consultants.territory_id
            ORDER by consultant_id 
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

        let consultants = query_result.unwrap();

        let consultants_table_data = ResponsiveTableData {
            entity_type_id: 4,
            vec_len: consultants.len(),
            lookup_url: "/consultant/list?page=".to_string(),
            page: opts.page.unwrap_or(1),
            entities: consultants,
        };

        let body = hb
            .render("responsive-table", &consultants_table_data)
            .unwrap();
        return HttpResponse::Ok().body(body);
    }
}

#[get("/form")]
async fn consultant_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    // path: web::Path<i32>,
) -> impl Responder {
    println!("consultant_form firing");

    let account_result = sqlx::query_as!(
        SelectOption,
        "SELECT account_id AS value, account_name AS key 
        FROM accounts 
        ORDER by account_name"
    )
    .fetch_all(&state.db)
    .await;

    if account_result.is_err() {
        let err = "Error occurred while fetching account option KVs";
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let template_data = ConsultantFormTemplate {
        entity: None,
        territory_options: territory_options(),
        specialty_options: specialty_options(),
    };

    let body = hb
        .render("consultant/consultant-form", &template_data)
        .unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

#[get("/form/{slug}")]
async fn consultant_edit_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let consultant_slug = path.into_inner();

    let query_result = sqlx::query_as!(
        ConsultantFormRequest,
        "SELECT consultant_f_name, consultant_l_name, slug, specialty_id, territory_id, img_path
            FROM consultants 
            WHERE slug = $1",
            consultant_slug
    )
    .fetch_one(&state.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let err = "Error occurred while fetching record for consultant form";
        // return HttpResponse::InternalServerError()
        //     .json(json!({"status": "error","message": message}));
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let consultant = query_result.unwrap();

    let template_data = ConsultantFormTemplate {
        entity: Some(consultant),
        territory_options: territory_options(),
        specialty_options: specialty_options(),
    };

    let body = hb.render("consultant/consultant-form", &template_data).unwrap();
    return HttpResponse::Ok().body(body);
}