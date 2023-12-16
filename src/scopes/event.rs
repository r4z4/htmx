use actix_web::web::{Data, Form};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, Scope};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use ics::properties::{Categories, Description, DtEnd, DtStart, Organizer, Status, Summary};
use ics::{escape_text, Event, ICalendar};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::RedisState;
use crate::config::redis_validate_and_get_user;
use crate::models::model_consult::ConsultPost;
use crate::{
    config::{
        self, get_validation_response, subs_from_user, test_subs, validate_and_get_user,
        FilterOptions, ResponsiveTableData, SelectOption, UserAlert, ValidationResponse,
        ACCEPTED_SECONDARIES,
    },
    models::model_location::{
        LocationFormRequest, LocationFormTemplate, LocationList, LocationPostRequest,
    },
    AppState, ValidatedUser,
};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub fn event_scope() -> Scope {
    web::scope("/event")
        // .route("/users", web::get().to(get_users_handler))
        .service(location_form)
        .service(location_edit_form)
        .service(create_consult_event)
        .service(get_locations_handler)
        .service(search_location)
        .service(home)
        .service(next_month)
    //.service(prev_month)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableSearchRequest {
    search: String,
}

pub struct CalError {
    err: String,
}

fn date_to_cal_date(date: &DateTime<Utc>) -> String {
    date.format("%Y%m%dT%H%M%SZ").to_string()
}

fn category_from_purpose(id: i32) -> &'static str {
    match id {
        1 => "INTRODUCTION",
        2 => "WALKTHROUGH/INIT",
        3 => "CONTINUED",
        4 => "FINAL SERVICE",
        5 => "AUDIT",
        _ => "Oops",
    }
}

fn create_calendar_event(
    start: &DateTime<Utc>,
    end: &DateTime<Utc>,
    purpose_id: i32,
) -> ICalendar<'static> {
    // fn create_calendar_event() -> Result<(), CalError> {
    // create new iCalendar object
    let dt = NaiveDate::from_ymd_opt(2023, 12, 1)
        .unwrap()
        .and_hms_milli_opt(9, 10, 11, 12)
        .unwrap()
        .and_local_timezone(Utc)
        .unwrap(); // `2014-07-08T09:10:11.012Z`
    let weekday = dt.weekday();
    // Sunday is 1, Saturday is 7
    let weekday_int = weekday.number_from_sunday();
    dbg!(weekday);

    let cal_now = date_to_cal_date(&Utc::now());
    let dt_start = date_to_cal_date(start);
    let dt_end = date_to_cal_date(end);

    let mut calendar = ICalendar::new("2.0", "-//xyz Corp//NONSGML PDA Calendar Version 1.0//EN");
    // create event which contains the information regarding the conference
    let uuid_str = Uuid::new_v4().to_string();
    let mut event = Event::new(uuid_str, cal_now);
    // add properties
    let organizer = "mailto:jsmith@example.com";
    // let dt_start = "19960918T143000Z";
    // let dt_end = "19960920T220000Z";
    let status = if true {
        Status::confirmed()
    } else {
        Status::tentative()
    };
    let categories = category_from_purpose(purpose_id);
    let summary = "Networld+Interop Conference";
    // Values that are "TEXT" must be escaped (only if the text contains a comma,
    // semicolon, backslash or newline).
    let description = escape_text(
        "Networld+Interop Conference and Exhibit\n\
         Atlanta World Congress Center\n\
         Atlanta, Georgia",
    );
    event.push(Organizer::new(organizer));
    event.push(DtStart::new(dt_start));
    event.push(DtEnd::new(dt_end));
    event.push(status);
    event.push(Categories::new(categories));
    event.push(Summary::new(summary));
    event.push(Description::new(description));
    // add event to calendar
    calendar.add_event(event);

    // write calendar to file
    // calendar.save_file("event.ics")?;
    // let _ = calendar.save_file("event.ics");
    calendar
    // Ok::<(), CalError>(())
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
            id, 
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
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let locations = query_result.unwrap();

    let f_opts = FilterOptions::from(&opts);

    let locations_table_data = ResponsiveTableData {
        entity_type_id: 5,
        vec_len: locations.len(),
        lookup_url: "/location/list?page=".to_string(),
        opts: f_opts,
        // page: opts.page.unwrap_or(1),
        entities: locations,
        subscriptions: test_subs(),
    };

    dbg!(&locations_table_data);

    let body = hb
        .render("responsive-table", &locations_table_data)
        .unwrap();
    return HttpResponse::Ok().body(body);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarData {
    pub month: u32,
    pub year: u32,
    pub first_day_of_month: u32,
    pub num_days: u32,
    pub weekday_range: Vec<u32>,
    pub holidays: Vec<(u32, String)>,
}

fn get_month_holidays(month: u32) -> Vec<(u32, String)> {
    match month {
        1 => vec![
            (1, "New Year's Day".to_string()),
            (11, "Veteran's Day".to_string()),
        ],
        2 => vec![
            (2, "Groundhog's Day".to_string()),
            (14, "Valentine's Day".to_string()),
        ],
        3 => vec![(21, "Spring".to_string())],
        4 => vec![
            (1, "April Fools".to_string()),
            (14, "Valentine's Day".to_string()),
        ],
        5 => vec![
            (1, "New Year's Day".to_string()),
            (11, "Veteran's Day".to_string()),
        ],
        6 => vec![
            (2, "Groundhog's Day".to_string()),
            (14, "Valentine's Day".to_string()),
        ],
        7 => vec![
            (4, "Independence Day".to_string()),
            (11, "Veteran's Day".to_string()),
        ],
        8 => vec![
            (2, "Groundhog's Day".to_string()),
            (14, "Valentine's Day".to_string()),
        ],
        9 => vec![
            (1, "New Year's Day".to_string()),
            (11, "Veteran's Day".to_string()),
        ],
        10 => vec![
            (31, "Halloween".to_string()),
            (14, "Valentine's Day".to_string()),
        ],
        11 => vec![
            (24, "Thanksgiving".to_string()),
            (11, "Veteran's Day".to_string()),
        ],
        12 => vec![
            (25, "Christmas".to_string()),
            (31, "New Year's Eve".to_string()),
        ],
        _ => vec![
            (24, "Thanksgiving".to_string()),
            (11, "Veteran's Day".to_string()),
        ],
    }
}

fn get_num_days(month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        // FIXME leap year
        2 => 28,
        _ => 0,
    }
}

#[get("/")]
async fn home(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: Data<AppState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match validate_and_get_user(cookie, &state).await {
            Ok(user) => {
                if let Some(usr) = user {
                    // Homepage displays current Mo/Yr
                    let this_month = Utc::now().month();
                    let this_year = Utc::now().year();

                    let day_one: NaiveDate =
                        NaiveDate::from_ymd_opt(this_year, this_month, 1).unwrap();
                    let day_one_weekday = day_one.weekday();
                    // Sunday is 1, Saturday is 7
                    let day_one_int = day_one_weekday.number_from_sunday();
                    let cal = create_calendar_event(&Utc::now(), &Utc::now(), 1);
                    dbg!(cal);
                    let cal_data = CalendarData {
                        month: this_month,
                        year: this_year as u32,
                        first_day_of_month: day_one_int,
                        num_days: get_num_days(this_month),
                        weekday_range: vec![1, 2, 3, 4, 5, 6, 7],
                        // holidays: vec![(24, "Thanksgiving".to_string()),(11, "Veteran's Day".to_string())]
                        holidays: get_month_holidays(this_month),
                    };
                    let data = json! {{
                        "cal_data": cal_data,
                    }};
                    let body = hb.render("event-api", &data).unwrap();
                    HttpResponse::Ok().body(body)
                } else {
                    let message = "Cannot find you";
                    let body = hb.render("index", &message).unwrap();
                    return HttpResponse::Ok().body(body);
                }
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("index", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok().body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
        // FIXME: Is this else right? Redirect?
    } else {
        let message = "Your session seems to have expired. Please login again (3).".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[get("/calendar/next/{cur_year}/{cur_month}")]
async fn next_month(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    path: web::Path<(u32, u32)>,
    state: Data<AppState>,
    r_state: Data<RedisState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match redis_validate_and_get_user(cookie, &r_state).await {
            Ok(user) => {
                let (input_year, input_month) = path.into_inner();

                let cal_month = if input_month == 12 {
                    1
                } else {
                    input_month + 1
                };
                let cal_year = if input_month == 12 {
                    input_year + 1
                } else {
                    input_year
                };

                let day_one: NaiveDate =
                    NaiveDate::from_ymd_opt(cal_year as i32, cal_month, 1).unwrap();
                let day_one_weekday = day_one.weekday();
                // Sunday is 1, Saturday is 7
                let day_one_int = day_one_weekday.number_from_sunday();
                let cal_data = CalendarData {
                    month: cal_month,
                    year: cal_year,
                    first_day_of_month: day_one_int,
                    num_days: get_num_days(cal_month),
                    weekday_range: vec![1, 2, 3, 4, 5, 6, 7],
                    holidays: get_month_holidays(cal_month),
                };
                let month_data = json! {{
                    "cal_data": cal_data,
                }};
                let body = hb.render("calendar/month", &month_data).unwrap();
                HttpResponse::Ok().body(body)
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("index", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok().body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
        // FIXME: Is this else right? Redirect?
    } else {
        let message = "Your session seems to have expired. Please login again (3).".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[get("/list")]
pub async fn get_locations_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match validate_and_get_user(cookie, &state).await {
            Ok(user_opt) => {
                if let Some(user) = user_opt {
                    println!("get_locations_handler firing");
                    let limit = opts.limit.unwrap_or(10);
                    let offset = (opts.page.unwrap_or(1) - 1) * limit;

                    if let Some(like) = &opts.search {
                        let search_sql = format!("%{}%", like);
                        let query_result = sqlx::query_as!(
                            LocationList,
                            "SELECT 
                                id, 
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
                        .fetch_all(&state.db)
                        .await;

                        dbg!(&query_result);

                        if query_result.is_err() {
                            let error_msg =
                                "Error occurred while fetching searched location records";
                            let validation_response =
                                ValidationResponse::from((error_msg, "validation_error"));
                            let body = hb.render("validation", &validation_response).unwrap();
                            return HttpResponse::Ok().body(body);
                        }

                        let locations = query_result.unwrap();

                        let f_opts = FilterOptions::from(&opts);

                        let locations_table_data = ResponsiveTableData {
                            entity_type_id: 5,
                            vec_len: locations.len(),
                            lookup_url: "/location/list?page=".to_string(),
                            opts: f_opts,
                            // page: opts.page.unwrap_or(1),
                            entities: locations,
                            subscriptions: subs_from_user(&user),
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
                                id, 
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
                        .fetch_all(&state.db)
                        .await;

                        dbg!(&query_result);

                        if query_result.is_err() {
                            let error_msg = "Error occurred while fetching all location records";
                            let validation_response =
                                ValidationResponse::from((error_msg, "validation_error"));
                            let body = hb.render("validation", &validation_response).unwrap();
                            return HttpResponse::Ok().body(body);
                        }

                        let locations = query_result.unwrap();

                        let f_opts = FilterOptions::from(&opts);

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
                            opts: f_opts,
                            // page: opts.page.unwrap_or(1),
                            entities: locations,
                            subscriptions: subs_from_user(&user),
                        };

                        dbg!(&locations_table_data);

                        let body = hb
                            .render("responsive-table", &locations_table_data)
                            .unwrap();
                        return HttpResponse::Ok().body(body);
                    }
                } else {
                    let message = "Cannot find you";
                    let body = hb.render("index", &message).unwrap();
                    return HttpResponse::Ok().body(body);
                }
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("index", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok().body(body);
            }
        }
    } else {
        let message = "Your session seems to have expired. Please login again.".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
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
        "SELECT id AS value, account_name AS key 
        FROM accounts 
        ORDER by account_name"
    )
    .fetch_all(&state.db)
    .await;

    if account_result.is_err() {
        let error_msg = "Error occurred while fetching account option KVs";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
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
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
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
        let first = apt_ste[0];
        dbg!(&first);
        // No input comes in as blank Some("")
        if ACCEPTED_SECONDARIES.contains(&first) || addr_two == "" {
            true
        } else {
            false
        }
    } else {
        true
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FullPageTemplateData {
    user_alert: UserAlert,
    user: Option<ValidatedUser>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexData {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventPostRequest {
    pub month: u32,
    pub year: u32,
    pub day: u32,
    pub message: String,
}

fn validate_event_input(body: &Form<ConsultPost>) -> bool {
    true
}
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ConsultResponse {
    id: i32,
}

#[post("/form")]
async fn create_consult_event(
    body: web::Form<ConsultPost>,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> impl Responder {
    let headers = req.headers();
    if let Some(cookie) = headers.get(actix_web::http::header::COOKIE) {
        dbg!(cookie.clone());
        match validate_and_get_user(cookie, &state).await {
            Ok(user_option) => {
                dbg!(&user_option);
                let is_valid = body.validate();
                if is_valid.is_err() {
                    let validation_response = get_validation_response(is_valid);
                    let body = hb
                        .render("forms/form-validation", &validation_response)
                        .unwrap();
                    return HttpResponse::BadRequest()
                        .header("HX-Retarget", "#location_errors")
                        .body(body);
                }

                let consult_start_string =
                    body.consult_start_date.clone() + " " + &body.consult_start_time + ":00 -06:00";
                dbg!(&consult_start_string);
                let consult_end_string =
                    body.consult_end_date.clone() + " " + &body.consult_end_time + ":00 -06:00";
                dbg!(&consult_end_string);
                let consult_end_datetime =
                    DateTime::parse_from_str(&consult_end_string, "%Y-%m-%d %H:%M:%S %z").unwrap();
                dbg!(&consult_end_datetime);
                dbg!(&consult_start_string);
                let consult_start_datetime =
                    DateTime::parse_from_str(&consult_start_string, "%Y-%m-%d %H:%M:%S %z")
                        .unwrap();
                dbg!(&consult_start_datetime);
                let consult_start_datetime_utc = consult_start_datetime.with_timezone(&Utc);
                let consult_end_datetime_utc = consult_end_datetime.with_timezone(&Utc);

                // Create ICS calendar to send out
                let cal = create_calendar_event(
                    &consult_start_datetime_utc,
                    &consult_end_datetime_utc,
                    body.consult_purpose_id,
                );

                if let Some(user) = user_option {
                    // let user_body = hb.render("homepage", &user).unwrap();
                    if validate_event_input(&body) {
                        match sqlx::query_as::<_, ConsultResponse>(
                            "INSERT INTO consults (consult_purpose_id, client_id, location_id, consult_start, consult_end, notes) VALUES ($1, $2, $3, $4, $5) RETURNING id",
                        )
                        .bind(body.consult_purpose_id)
                        .bind(body.client_id)
                        .bind(body.location_id)
                        .bind(consult_start_datetime)
                        .bind(consult_end_datetime)
                        .bind(body.notes.clone())
                        .fetch_one(&state.db)
                        .await
                        {
                            Ok(consult_response) => {
                                let user_alert = UserAlert::from((format!("Consult added successfully: ID #{:?}", consult_response.id).as_str(), "alert_success"));
                                let body = hb.render("crud-api", &user_alert).unwrap();
                                return HttpResponse::Ok().body(body);
                            }
                            Err(err) => {
                                dbg!(&err);
                                let user_alert = UserAlert::from((format!("Error Updating User After Adding Them As Consult: {:?}", err).as_str(), "alert_error"));
                                let body = hb.render("crud-api", &user_alert).unwrap();
                                return HttpResponse::Ok().body(body);
                            }
                        }
                    } else {
                        println!("Val error");
                        let error_msg = "Validation error";
                        let validation_response =
                            ValidationResponse::from((error_msg, "validation_error"));
                        let body = hb.render("validation", &validation_response).unwrap();
                        return HttpResponse::Ok().body(body);
                    }
                } else {
                    let index_data = IndexData {
                        message: "Your session seems to have expired. Please login again."
                            .to_owned(),
                    };
                    let body = hb.render("index", &index_data).unwrap();

                    HttpResponse::Ok().body(body)
                }
            }
            Err(_err) => {
                dbg!(&_err);
                // User's cookie is invalud or expired. Need to get a new one via logging in.
                // They had a session. Could give them details about that. Get from DB.
                let index_data = IndexData {
                    message: format!("Error in validate and get user: {}", _err.error),
                };
                let body = hb.render("index", &index_data).unwrap();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{location_contacts, states};
    use crate::{
        hbs_helpers::{int_eq, str_eq},
        test_common::{self, *},
    };
    use test_context::{test_context, TestContext};

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
