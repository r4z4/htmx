use std::{fs::create_dir, ops::Deref, borrow::Borrow};

use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse, Responder, Scope,
};

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::{config::{FilterOptions, SelectOption, self, ResponsiveTableData, UserAlert, ACCEPTED_SECONDARIES}, models::location::{LocationList, LocationFormTemplate, LocationPostRequest, LocationPostResponse}, AppState};

pub fn location_scope() -> Scope {
    web::scope("/location")
        // .route("/users", web::get().to(get_users_handler))
        .service(location_form)
        .service(create_location)
        .service(get_locations_handler)
}

#[get("/list")]
pub async fn get_locations_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
) -> impl Responder {
    println!("get_locations_handler firing");
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let query_result = sqlx::query_as!(
        LocationList,
        "SELECT 
            location_id, 
            location_name,
            location_address_one,
            location_address_two,
            location_city,
            location_zip,
            location_phone
        FROM locations
        ORDER by location_name
        LIMIT $1 OFFSET $2",
        limit as i32,
        offset as i32
    )
    .fetch_all(&data.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let err = "Error occurred while fetching all location records";
        // return HttpResponse::InternalServerError()
        //     .json(json!({"status": "error","message": message}));
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let locations = query_result.unwrap();

    //     let consultants_response = ConsultantListResponse {
    //         consultants: consultants,
    //         name: "Hello".to_owned()
    // ,    };

    // let table_headers = ["ID".to_owned(),"Specialty".to_owned(),"First NAme".to_owned()].to_vec();
    // let load_more_url_base = "/location/list?page=".to_owned();
    let locations_table_data = ResponsiveTableData {
        table_title: "Locations".to_owned(),
        vec_len: locations.len(),
        lookup_url: "/location/list?page=".to_string(),
        page: opts.page.unwrap_or(1),
        entities: locations,
    };

    dbg!(&locations_table_data);

    let body = hb
        .render("responsive-table", &locations_table_data)
        .unwrap();
    return HttpResponse::Ok().body(body);
}

#[get("/form")]
async fn location_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    // path: web::Path<i32>,
) -> impl Responder {
    println!("location_form firing");

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

    let template_data = LocationFormTemplate {
        state_options: config::states(),
        location_contact_options: config::location_contacts(),
    };

    let body = hb
        .render("location/location-form", &template_data)
        .unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}


fn validate_location_input(body: &LocationPostRequest) -> bool {
    // Woof
    dbg!(&body);
    if let Some(addr_two) = &body.location_address_two {
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
async fn create_location(
    body: web::Form<LocationPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
) -> impl Responder {
    dbg!(&body);

    if validate_location_input(&body) {
        match sqlx::query_as::<_, LocationPostResponse>(
            "INSERT INTO locations (location_name, location_address_one, location_address_two, location_city, location_state, location_zip, location_phone, location_contact_id, territory_id) 
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, DEFAULT) RETURNING location_id",
        )
        .bind(&body.location_name)
        .bind(&body.location_address_one)
        .bind(&body.location_address_two)
        .bind(&body.location_city)
        .bind(&body.location_state)
        .bind(&body.location_zip)
        .bind(&body.location_phone)
        .bind(&body.location_contact_id)
        .fetch_one(&state.db)
        .await
        {
            Ok(loc) => {
                dbg!(loc.location_id);
                let user_alert = UserAlert {
                    msg: format!("Location added successfully: ID #{:?}", loc.location_id),
                };
                let body = hb.render("crud-api", &user_alert).unwrap();
                return HttpResponse::Ok().body(body);
            }
            Err(err) => {
                dbg!(&err);
                let user_alert = UserAlert {
                    msg: format!("Error adding location: {:?}", err),
                };
                let body = hb.render("crud-api", &user_alert).unwrap();
                return HttpResponse::Ok().body(body);
            }
        }
    } else {
        let msg = "Validation error".to_owned();
        let body = hb.render("validation", &msg).unwrap();
        return HttpResponse::Ok().body(body);
    }
}