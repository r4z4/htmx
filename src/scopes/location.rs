use std::borrow::Borrow;

use actix_web::{
    get, patch, post,
    web::{self, Data},
    HttpRequest, HttpResponse, Responder, Scope,
};
use serde_json::json;

use crate::{
    config::{
        self, FilterOptions, FormErrorResponse, ResponsiveTableData,
        SelectOption, UserAlert, ValidationErrorMap, ValidationResponse, ACCEPTED_SECONDARIES,
    },
    models::model_location::{
        LocationFormRequest, LocationFormTemplate, LocationList, LocationPatchRequest,
        LocationPostRequest, LocationPostResponse,
    },
    AppState, HeaderValueExt, ValidatedUser, ValError,
};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub fn location_scope() -> Scope {
    web::scope("/location")
        // .route("/users", web::get().to(get_users_handler))
        .service(location_form)
        .service(location_edit_form)
        .service(create_location)
        .service(get_locations_handler)
        .service(patch_location)
        .service(search_location)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableSearchRequest {
    search: String,
}

#[post("/search")]
async fn search_location(
    opts: web::Query<FilterOptions>,
    body: web::Form<TableSearchRequest>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
) -> impl Responder {
    dbg!(&body);
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let search_sql = "%".to_owned() + &body.search + "%";

    let query_result = sqlx::query_as!(
        LocationList,
        "SELECT 
            location_id, 
            slug,
            location_name,
            location_address_one,
            location_address_two,
            location_city,
            location_zip,
            location_phone
        FROM locations
        WHERE location_name LIKE $1
        ORDER by location_name",
        search_sql
    )
    .fetch_all(&state.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let error_msg = "Error occurred while fetching all location records";
        let validation_response = ValidationResponse {
            msg: error_msg.to_string(),
            class: "validation_error".to_owned(),
        };
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let locations = query_result.unwrap();

    let locations_table_data = ResponsiveTableData {
        entity_type_id: 5,
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

#[get("/list")]
pub async fn get_locations_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
) -> impl Responder {
    println!("get_locations_handler firing");
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    // let search_sql =
    // if opts.search.is_some() {
    //     let search = opts.search.as_ref().unwrap();
    //     format!(
    //         r#"
    //         WHERE location_name LIKE %{}%
    //         "#,
    //         search,
    //     )
    // } else {
    //     "".to_string()
    // };

    if let Some(like) = &opts.search {
        let search_sql = format!("%{}%", like);
        let query_result = sqlx::query_as!(
            LocationList,
            "SELECT 
                location_id, 
                slug,
                location_name,
                location_address_one,
                location_address_two,
                location_city,
                location_zip,
                location_phone
            FROM locations
            WHERE location_name LIKE $3
            ORDER by location_name
            LIMIT $1 OFFSET $2",
            limit as i32,
            offset as i32,
            search_sql
        )
        .fetch_all(&data.db)
        .await;

        dbg!(&query_result);

        if query_result.is_err() {
            let error_msg = "Error occurred while fetching searched location records";
            let validation_response = ValidationResponse {
                msg: error_msg.to_string(),
                class: "validation_error".to_owned(),
            };
            let body = hb.render("validation", &validation_response).unwrap();
            return HttpResponse::Ok().body(body);
        }

        let locations = query_result.unwrap();

        let locations_table_data = ResponsiveTableData {
            entity_type_id: 5,
            vec_len: locations.len(),
            lookup_url: "/location/list?page=".to_string(),
            page: opts.page.unwrap_or(1),
            entities: locations,
        };

        dbg!(&locations_table_data);

        let body = hb
            .render("responsive-table-inner", &locations_table_data)
            .unwrap();
        return HttpResponse::Ok().body(body);
    } else {
        let query_result = sqlx::query_as!(
            LocationList,
            "SELECT 
                location_id, 
                slug,
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
            let error_msg = "Error occurred while fetching all location records";
            let validation_response = ValidationResponse {
                msg: error_msg.to_string(),
                class: "validation_error".to_owned(),
            };
            let body = hb.render("validation", &validation_response).unwrap();
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
            entity_type_id: 5,
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
        let error_msg = "Error occurred while fetching account option KVs";
        let validation_response = ValidationResponse {
            msg: error_msg.to_string(),
            class: "validation_error".to_owned(),
        };
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let template_data = LocationFormTemplate {
        entity: None,
        state_options: config::get_state_options(&state.db).await,
        location_contact_options: config::location_contacts(),
    };

    let body = hb.render("forms/location-form", &template_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

#[get("/form/{slug}")]
async fn location_edit_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let loc_slug = path.into_inner();

    let query_result = sqlx::query_as!(
        LocationFormRequest,
        "SELECT location_name, slug, location_address_one, location_address_two, location_city, location_state, location_zip, location_phone, location_contact_id
            FROM locations 
            WHERE slug = $1",
            loc_slug
    )
    .fetch_one(&state.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let error_msg = "Error occurred while fetching record for location form";
        let validation_response = ValidationResponse {
            msg: error_msg.to_string(),
            class: "validation_error".to_owned(),
        };
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let location = query_result.unwrap();

    let template_data = LocationFormTemplate {
        entity: Some(location),
        state_options: config::get_state_options(&state.db).await,
        location_contact_options: config::location_contacts(),
    };

    let body = hb.render("forms/location-form", &template_data).unwrap();
    return HttpResponse::Ok().body(body);
}

fn validate_location_input(body: &LocationPostRequest) -> bool {
    // Woof
    dbg!(&body);
    if let Some(addr_two) = &body.location_address_two {
        let apt_ste: Vec<&str> = addr_two.split(" ").collect::<Vec<&str>>().to_owned();
        let first = apt_ste[0].to_owned();
        dbg!(&first);
        // No input comes in as blank Some("")
        if ACCEPTED_SECONDARIES.contains(first.borrow()) || addr_two == "" {
            true
        } else {
            false
        }
    } else {
        true
    }
}

async fn validate_and_get_user(
    cookie: &actix_web::http::header::HeaderValue,
    state: &Data<AppState>,
) -> Result<Option<ValidatedUser>, ValError> {
    println!("Validating {}", format!("{:?}", cookie.clone()));
    match sqlx::query_as::<_, ValidatedUser>(
        "SELECT username, email, user_type_id
        FROM users
        LEFT JOIN user_sessions on user_sessions.user_id = users.user_id
        WHERE session_id = $1
        AND expires > NOW()",
    )
    .bind(cookie.to_string())
    .fetch_optional(&state.db)
    .await
    {
        Ok(user_option) => Ok(user_option),
        Err(err) => Err(ValError {
            error: format!("You must not be verfied: {}", err),
        }),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FullPageTemplateData {
    user_alert: UserAlert,
    user: ValidatedUser,
}

#[post("/form")]
async fn create_location(
    body: web::Form<LocationPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    dbg!(&body);
    let headers = req.headers();
    // for (pos, e) in headers.iter().enumerate() {
    //     println!("Element at position {}: {:?}", pos, e);
    // }
    if let Some(cookie) = headers.get(actix_web::http::header::COOKIE) {
        dbg!(cookie.clone());
        match validate_and_get_user(cookie, &state).await {
            Ok(user_option) => {
                if let Some(user) = user_option {
                    let user = ValidatedUser {
                        username: user.username,
                        email: user.email,
                        user_type_id: user.user_type_id,
                        list_view: user.list_view,
                    };
                    // let user_body = hb.render("homepage", &user).unwrap();
                    if validate_location_input(&body) {
                        match sqlx::query_as::<_, LocationPostResponse>(
                            "INSERT INTO locations (location_name, location_address_one, location_address_two, location_city, location_state, location_zip, location_phone, location_contact_id, territory_id) 
                                    VALUES ($1, $2, NULLIF($3, ''), $4, $5, $6, NULLIF($7, ''), $8, DEFAULT) RETURNING location_id",
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
                                    class: "alert_success".to_owned(),
                                };
                                let template_data = json!({
                                    "user_alert": user_alert,
                                    "user": user,
                                });
                                let template_body = hb.render("crud-api", &template_data).unwrap();
                                return HttpResponse::Ok().body(template_body);
                            }
                            Err(err) => {
                                dbg!(&err);
                                let user_alert = UserAlert {
                                    msg: format!("Error adding location: {:?}", err),
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
                        //     msg: "Error adding location:".to_owned(),
                        //     class: "alert_error".to_owned(),
                        // };
                        // let body = hb.render("crud-api", &user_alert).unwrap();
                        // return HttpResponse::Ok().body(body);
                    }
                } else {
                    let message =
                        "Your session seems to have expired. Please login again.".to_owned();
                    let body = hb.render("index", &message).unwrap();

                    HttpResponse::Ok().body(body)
                }
            }
            Err(_err) => {
                // User's cookie is invalud or expired. Need to get a new one via logging in.
                // They had a session. Could give them details about that. Get from DB.
                let message = "Error in validate and get user.".to_owned();
                let body = hb.render("index", &message).unwrap();

                HttpResponse::Ok().body(body)
            }
        }
    } else {
        let data = json!({
            "header": "Login Form",
        });
        let body = hb.render("index", &data).unwrap();

        HttpResponse::Ok().body(body)
    }
}

fn validate_patch(req: &LocationPatchRequest) -> bool {
    true
}

fn validate_post(req: &LocationPostRequest) -> bool {
    true
}

#[patch("/form/{slug}")]
async fn patch_location(
    body: web::Form<LocationPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let loc_slug = path.into_inner();
    dbg!(&body);

    let is_valid = body.validate();
    if is_valid.is_err() {
        println!("Got err");
        dbg!(is_valid.is_err());
        let val_errs = is_valid
            .err()
            .unwrap()
            .field_errors()
            .iter()
            .map(|x| {
                let (key, errs) = x;
                ValidationErrorMap {
                    key: key.to_string(),
                    errs: errs.to_vec(),
                }
            })
            .collect::<Vec<ValidationErrorMap>>();
        dbg!(&val_errs);
        // return HttpResponse::InternalServerError().json(format!("{:?}", is_valid.err().unwrap()));
        let validation_response = FormErrorResponse {
            errors: Some(val_errs),
        };
        let body = hb
            .render("forms/form-validation", &validation_response)
            .unwrap();
        return HttpResponse::BadRequest()
            .header("HX-Retarget", "#location_errors")
            .body(body);
        // return HttpResponse::Ok()
        // .header("HX-Retarget", "#location_errors")
        // .body(body);
    }

    // For an actual Patch to only set altered fields
    // let mut generated_sql = String::new();
    // for (field_name, field_value) in body.iter() {
    //     let sql = String::from(format!("{} = {:?},", field_name, field_value));
    //     generated_sql += &sql;
    // }

    // // Remove that last comma
    // generated_sql.pop();

    if validate_post(&body) {
        // match sqlx::query_as::<_, LocationPostResponse>(
        //     "UPDATE locations SET $1 WHERE slug = $3",
        // )
        // .bind(&generated_sql)
        // .bind(loc_slug)
        // .fetch_one(&state.db)
        // .await
        match sqlx::query_as::<_, LocationPostResponse>(
            "UPDATE locations 
                SET location_name = $1,
                    location_address_one = $2,
                    location_address_two = $3,
                    location_city = $4,
                    location_state = $5,
                    location_zip = $6,
                    location_phone = $7,
                    location_contact_id = $8
                WHERE slug = $9",
        )
        .bind(&body.location_name)
        .bind(&body.location_address_one)
        .bind(&body.location_address_two)
        .bind(&body.location_city)
        .bind(&body.location_state)
        .bind(&body.location_zip)
        .bind(&body.location_phone)
        .bind(&body.location_contact_id)
        .bind(loc_slug)
        .fetch_one(&state.db)
        .await
        {
            Ok(loc) => {
                dbg!(loc.location_id);
                let user_alert = UserAlert {
                    msg: format!("Location added successfully: ID #{:?}", loc.location_id),
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

        // // To test the alert more easily
        // let user_alert = UserAlert {
        //     msg: "Error adding location:".to_owned(),
        //     class: "alert_error".to_owned(),
        // };
        // let body = hb.render("crud-api", &user_alert).unwrap();
        // return HttpResponse::Ok().body(body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hbs_helpers::{int_eq, str_eq},
        test_common::{self, *},
    };
    use test_context::{test_context, TestContext};
    use crate::config::{location_contacts, states};

    #[test_context(Context)]
    #[test]
    fn create_form_renders_add_header(ctx: &mut Context) {
        let template_data = LocationFormTemplate {
            entity: None,
            state_options: states(),
            location_contact_options: location_contacts(),
        };
        let mut hb = Handlebars::new();
        hb.register_templates_directory(".hbs", "./templates")
            .unwrap();
        hb.register_helper("int_eq", Box::new(int_eq));
        hb.register_helper("str_eq", Box::new(str_eq));
        let body = hb.render("forms/location-form", &template_data).unwrap();
        // Finishing without error is itself a pass. But can reach into the giant HTML string hb template too.
        let dom = tl::parse(&body, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();

        let element = dom
            .get_element_by_id("location_form_header")
            .expect("Failed to find element")
            .get(parser)
            .unwrap();

        // Assert
        assert_eq!(element.inner_text(parser), "Add Location");

        // Assert
        // assert_eq!(1, 1);
    }
}
