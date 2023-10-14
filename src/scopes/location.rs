use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse, Responder, Scope,
};

use handlebars::Handlebars;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{config::FilterOptions, models::location::LocationList, AppState};

lazy_static! {
    static ref RE_USER_NAME: Regex = Regex::new(r"^[a-zA-Z0-9]{4,}$").unwrap();
    static ref RE_SPECIAL_CHAR: Regex = Regex::new("^.*?[@$!%*?&].*$").unwrap();
}

pub fn location_scope() -> Scope {
    web::scope("/location")
        // .route("/users", web::get().to(get_users_handler))
        // .service(consultant_form)
        .service(get_locations_handler)
}
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ResponsiveLocationData {
    table_title: String,
    locations: Vec<LocationList>,
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
        let body = hb.render("error", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let locations = query_result.unwrap();

    //     let consultants_response = ConsultantListResponse {
    //         consultants: consultants,
    //         name: "Hello".to_owned()
    // ,    };

    // let table_headers = ["ID".to_owned(),"Specialty".to_owned(),"First NAme".to_owned()].to_vec();

    let locations_table_data = ResponsiveLocationData {
        table_title: "Consultancy Locations".to_owned(),
        locations: locations,
    };

    let body = hb
        .render("responsive-table", &locations_table_data)
        .unwrap();
    return HttpResponse::Ok().body(body);
}
