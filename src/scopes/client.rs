use std::borrow::Borrow;

use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse, Responder, Scope, patch,
};

use handlebars::Handlebars;
use validator::Validate;
use crate::{config::{FilterOptions, SelectOption, self, ResponsiveTableData, UserAlert, ACCEPTED_SECONDARIES, ValidationResponse, FormErrorResponse, ValidationErrorMap},
    models::model_client::{ClientList, ClientFormTemplate, ClientPostRequest, ClientPostResponse, ClientFormRequest}, AppState};

pub fn client_scope() -> Scope {
    web::scope("/client")
        // .route("/users", web::get().to(get_users_handler))
        .service(client_form)
        .service(create_client)
        .service(get_clients_handler)
        .service(patch_client)
        .service(client_edit_form)
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

    let account_options = account_options(&state).await;
    let template_data = ClientFormTemplate {
        entity: None,
        account_options: account_options,
        state_options: config::states(),
    };

    let body = hb
        .render("forms/client-form", &template_data)
        .unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

async fn account_options(state: &web::Data<AppState>) -> Vec<SelectOption> {
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
        let default_options = SelectOption { 
            key: Some("No accounts Found".to_owned()), 
            value: 0 
        };
        // default_options
        dbg!("Incoming Panic");
    }

    let account_options = account_result.unwrap();
    account_options
}

#[get("/form/{slug}")]
async fn client_edit_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let loc_slug = path.into_inner();

    let query_result = sqlx::query_as!(
        ClientFormRequest,
        "SELECT client_company_name, client_f_name, client_l_name, slug, client_address_one, client_address_two, client_city, client_state, client_zip, client_email, client_dob, account_id, client_primary_phone
            FROM clients 
            WHERE slug = $1",
            loc_slug
    )
    .fetch_one(&state.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let err = "Error occurred while fetching record for location form";
        // return HttpResponse::InternalServerError()
        //     .json(json!({"status": "error","message": message}));
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }
    let account_options = account_options(&state).await;
    let client = query_result.unwrap();

    let template_data = ClientFormTemplate {
        entity: Some(client),
        state_options: config::states(),
        account_options: account_options,
    };

    let body = hb.render("forms/client-form", &template_data).unwrap();
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

fn validate_patch(body: &ClientPostRequest) -> bool {
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

#[patch("/form/{slug}")]
async fn patch_client(
    body: web::Form<ClientPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let client_slug = path.into_inner();
    dbg!(&body);

    let is_valid = body.validate();
    if is_valid.is_err() {
        println!("Got err");
        dbg!(is_valid.is_err());
        let val_errs = is_valid.err().unwrap().field_errors().iter().map(|x| {
            let (key, errs) = x;
            ValidationErrorMap{key: key.to_string(), errs: errs.to_vec()}
        }).collect::<Vec<ValidationErrorMap>>();
        dbg!(&val_errs);
        // return HttpResponse::InternalServerError().json(format!("{:?}", is_valid.err().unwrap()));
        let validation_response = FormErrorResponse {
            errors: Some(val_errs),
        };
        let body = hb.render("forms/form-validation", &validation_response).unwrap();
        return HttpResponse::BadRequest()
        .header("HX-Retarget", "#location_errors")
        .body(body);
    }

    if validate_patch(&body) {
        match sqlx::query_as::<_, ClientPostResponse>(
            "UPDATE locations 
                SET client_company_name = $1,
                    client_f_name = $2,
                    client_l_name = $3,
                    client_address_one = $4.
                    client_address_two = $5.
                    client_city = $6,
                    client_state = $7,
                    client_zip = $8,
                    client_primary_phone = $9,
                    client_email = $10,
                WHERE slug = $11
                RETURNING client_id",
        )
        .bind(&body.client_company_name)
        .bind(&body.client_f_name)
        .bind(&body.client_l_name)
        .bind(&body.client_address_one)
        .bind(&body.client_address_two)
        .bind(&body.client_city)
        .bind(&body.client_state)
        .bind(&body.client_zip)
        .bind(&body.client_primary_phone)
        .bind(&body.client_email)
        .bind(client_slug)
        .fetch_one(&state.db)
        .await
        {
            Ok(client) => {
                dbg!(client.client_id);
                let user_alert = UserAlert {
                    msg: format!("Location added successfully: ID #{:?}", client.client_id),
                    class: "alert_success".to_owned(),
                };
                let body = hb.render("list-api", &user_alert).unwrap();
                return HttpResponse::Ok().body(body);
            }
            Err(err) => {
                dbg!(&err);
                let user_alert = UserAlert {
                    msg: format!("Error patching location: {:?}", err),
                    class: "alert_error".to_owned(),
                };
                let body = hb.render("list-api", &user_alert).unwrap();
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
        return HttpResponse::BadRequest().body(body);
    }
}