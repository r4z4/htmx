use std::borrow::Borrow;

use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse, Responder, Scope,
};

use handlebars::Handlebars;
use crate::{config::{FilterOptions, SelectOption, self, ResponsiveTableData, UserAlert, ACCEPTED_SECONDARIES, ValidationResponse}, models::model_client::{ClientList, ClientFormTemplate, ClientPostRequest, ClientPostResponse}, AppState};

pub fn client_scope() -> Scope {
    web::scope("/client")
        // .route("/users", web::get().to(get_users_handler))
        .service(client_form)
        .service(create_client)
        .service(get_clients_handler)
}

#[get("/list")]
pub async fn get_clients_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
) -> impl Responder {
    println!("get_clients_handler firing");
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let query_result = sqlx::query_as!(
        ClientList,
        "SELECT 
            client_id, 
            slug,
            client_company_name,
            client_f_name,
            client_l_name,
            client_email,
            client_address_one,
            client_address_two,
            client_city,
            client_zip,
            client_primary_phone
        FROM clients
        ORDER by client_id
        LIMIT $1 OFFSET $2",
        limit as i32,
        offset as i32
    )
    .fetch_all(&data.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let err = "Error occurred while fetching all client records";
        // return HttpResponse::InternalServerError()
        //     .json(json!({"status": "error","message": message}));
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let clients = query_result.unwrap();

    //     let consultants_response = ConsultantListResponse {
    //         consultants: consultants,
    //         name: "Hello".to_owned()
    // ,    };

    // let table_headers = ["client_id".to_owned(),"Specialty".to_owned(),"First NAme".to_owned()].to_vec();
    // let load_more_url_base = "/client/list?page=".to_owned();
    let clients_table_data = ResponsiveTableData {
        entity_type_id: 7,
        vec_len: clients.len(),
        lookup_url: "/client/list?page=".to_string(),
        page: opts.page.unwrap_or(1),
        entities: clients,
    };

    dbg!(&clients_table_data);

    let body = hb
        .render("responsive-table", &clients_table_data)
        .unwrap();
    return HttpResponse::Ok().body(body);
}

#[get("/form")]
async fn client_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    // path: web::Path<i32>,
) -> impl Responder {
    println!("client_form firing");

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

    let template_data = ClientFormTemplate {
        state_options: config::states(),
    };

    let body = hb
        .render("client/client-form", &template_data)
        .unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}


fn validate_client_input(body: &ClientPostRequest) -> bool {
    // Woof
    dbg!(&body);
    if let Some(addr_two) = &body.client_address_two {
        let apt_ste: Vec<&str> = addr_two.split(" ").collect::<Vec<&str>>().to_owned();
        let first = apt_ste[0].to_owned();
        dbg!(&first);
        if ACCEPTED_SECONDARIES.contains(first.borrow()) {
            true
        } else {
            false
        }
    } else {
        true
    }
}

#[post("/form")]
async fn create_client(
    body: web::Form<ClientPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
) -> impl Responder {
    dbg!(&body);

    if validate_client_input(&body) {
        match sqlx::query_as::<_, ClientPostResponse>(
            "INSERT INTO clients (client_f_name, client_l_name, client_company_name, client_address_one, client_address_two, client_city, client_state, client_zip, client_primary_phone) 
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, DEFAULT) RETURNING client_id",
        )
        .bind(&body.client_f_name)
        .bind(&body.client_l_name)
        .bind(&body.client_company_name)
        .bind(&body.client_address_one)
        .bind(&body.client_address_two)
        .bind(&body.client_city)
        .bind(&body.client_state)
        .bind(&body.client_zip)
        .bind(&body.client_primary_phone)
        .fetch_one(&state.db)
        .await
        {
            Ok(loc) => {
                dbg!(loc.client_id);
                let user_alert = UserAlert {
                    msg: format!("Client added successfully: client_id #{:?}", loc.client_id),
                    class: "alert_success".to_owned(),
                };
                let body = hb.render("crud-api", &user_alert).unwrap();
                return HttpResponse::Ok().body(body);
            }
            Err(err) => {
                dbg!(&err);
                let user_alert = UserAlert {
                    msg: format!("Error adding client: {:?}", err),
                    class: "alert_error".to_owned(),
                };
                let body = hb.render("crud-api", &user_alert).unwrap();
                return HttpResponse::Ok().body(body);
            }
        }
    } else {
        println!("Val error");
        let validation_response = ValidationResponse {
            msg: "Validation error".to_owned(),
            class: "validation_error".to_owned(),
        };
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);

        // // To test the alert more easily
        // let user_alert = UserAlert {
        //     msg: "Error adding client:".to_owned(),
        //     class: "alert_error".to_owned(),
        // };
        // let body = hb.render("crud-api", &user_alert).unwrap();
        // return HttpResponse::Ok().body(body);
    }
}