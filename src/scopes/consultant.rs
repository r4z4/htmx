use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse, Responder, Scope,
};

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::{
    config::{FilterOptions, ResponseConsultant, SelectOption, ResponsiveTableData, admin_user_options, specialty_options, territory_options},
    models::consultant::{ConsultantFormTemplate},
    AppState,
};

pub fn consultant_scope() -> Scope {
    web::scope("/consultant")
        // .route("/users", web::get().to(get_users_handler))
        .service(consultant_form)
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

    let query_result = sqlx::query_as!(
        ResponseConsultant,
        "SELECT 
            consultant_id, 
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

    //     let consultants_response = ConsultantListResponse {
    //         consultants: consultants,
    //         name: "Hello".to_owned()
    // ,    };

    // let table_headers = ["ID".to_owned(),"Specialty".to_owned(),"First NAme".to_owned()].to_vec();

    let consultants_table_data = ResponsiveTableData {
        vec_len: consultants.len(),
        lookup_url: "/consultant/list?page=".to_string(),
        page: opts.page.unwrap_or(1),
        table_title: "Consultants".to_owned(),
        entities: consultants,
    };

    let body = hb
        .render("responsive-table", &consultants_table_data)
        .unwrap();
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

    let account_options = account_result.unwrap();

    let template_data = ConsultantFormTemplate {
        account_options: account_options,
        territory_options: territory_options(),
        specialty_options: specialty_options(),
        admin_user_options: admin_user_options(),
    };

    let body = hb
        .render("consultant/consultant-form", &template_data)
        .unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}
