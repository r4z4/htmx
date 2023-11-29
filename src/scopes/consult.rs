use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use actix_multipart::Multipart;
use actix_web::{
    get, http::header::CONTENT_LENGTH, post, web, HttpRequest, HttpResponse, Responder, Scope,
};
use chrono::{DateTime, Timelike, Utc, Duration};
use deadpool_redis::{Connection, Manager, Pool as RedisPool};
use futures_util::TryStreamExt;
use handlebars::Handlebars;
use mime::{Mime, APPLICATION_JSON, APPLICATION_PDF, IMAGE_GIF, IMAGE_JPEG, IMAGE_PNG, TEXT_CSV};
use redis::{RedisResult, AsyncCommands, RedisError, ErrorKind, FromRedisValue, from_redis_value, Value};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, Error, FromRow, Pool, Postgres, QueryBuilder, Row};
use struct_iterable::Iterable;
use uuid::Uuid;
use validator::Validate;

use crate::{
    config::{
        consult_purpose_options, consult_result_options, mime_type_id_from_path, subs_from_user,
        FilterOptions, ResponsiveTableData, SelectOption, UserAlert, ValidationResponse, 
        get_validation_response, redis_validate_and_get_user, SelectOptionsVec, SimpleQuery, hash_query, hash_owned_query,
    },
    linfa::linfa_pred,
    models::model_consult::{
        ConsultAttachments, ConsultFormRequest, ConsultFormTemplate, ConsultList, ConsultPost,
        ConsultWithDates, ConsultListVec,
    },
    AppState, RedisState,
};

pub fn consult_scope() -> Scope {
    web::scope("/consult")
        // .route("/users", web::get().to(get_users_handler))
        .service(consult_form)
        .service(consult_edit_form)
        .service(create_consult)
        .service(get_consults_handler)
        .service(get_attachments)
        .service(upload)
        .service(availability)
}

async fn location_options(state: &web::Data<AppState>, r_state: &web::Data<RedisState>) -> SelectOptionsVec {
    let simple_query = SimpleQuery {
        query_str: "SELECT id AS value, location_name AS key 
        FROM locations 
        ORDER by location_name",
        int_args: None,
        str_args: None,
    };
    // Not hashing the query for options. Its always the same. Fixed Redis key so we can delete & invalidate when a record is added.
    // let query_hash = hash_query(&simple_query);
    // Search for key in Redis before reaching for DB
    let mut con = r_state.r_pool.get().await.unwrap();
    let prefix = "query";
    let key = format!("{}:{}", prefix, "location_options");

    let exists: Result<SelectOptionsVec, RedisError> = con.get(&key).await;
    dbg!(&exists);

    if exists.is_ok() {
        println!("Getting location_options from Redis");
        let result = exists.unwrap();
        return result;
    } else {
        println!("Getting location_options from DB");
        let location_result = sqlx::query_as::<>(simple_query.query_str)
        .fetch_all(&state.db)
        .await;

        if location_result.is_err() {
            let err = "Error occurred while fetching location option KVs";
            let default_options = SelectOption {
                key: Some("No Locations Found".to_owned()),
                value: 0,
            };
            // default_options
            dbg!("Incoming Panic");
        }

        let location_options = location_result.unwrap();
        let sov = SelectOptionsVec {
            vec: location_options,
        };
        // Cache the query in Redis -- `query:hash_value => result_hash`
        let val = serde_json::to_string(&sov).unwrap();
        let _: RedisResult<bool> = con.set_ex(key, &val, 86400).await;

        return sov;
    }   
}

// FIXME: Just pass pools?
async fn consultant_options(state: &web::Data<AppState>, r_state: &web::Data<RedisState>) -> SelectOptionsVec {
    let simple_query = SimpleQuery {
        query_str: "SELECT CONCAT(consultant_f_name, ' ',consultant_l_name) AS key, id AS value 
        FROM consultants ORDER BY key",
        int_args: None,
        str_args: None,
    };
    // Not hashing the query for options. Its always the same. Fixed Redis key so we can delete & invalidate when a record is added.
    // let query_hash = hash_query(&simple_query);
    // Search for key in Redis before reaching for DB
    let mut con = r_state.r_pool.get().await.unwrap();
    let prefix = "query";
    let key = format!("{}:{}", prefix, "consultant_options");

    let exists: Result<SelectOptionsVec, RedisError> = con.get(&key).await;
    dbg!(&exists);

    if exists.is_ok() {
        println!("Getting consultant_options from Redis");
        let result = exists.unwrap();
        return result;
    } else {
        println!("Getting consultant_options from DB");
        let consultant_result = sqlx::query_as::<_, SelectOption>(simple_query.query_str)
        .fetch_all(&state.db)
        .await;

        if consultant_result.is_err() {
            let err = "Error occurred while fetching location option KVs";
            let default_options = SelectOption {
                key: Some("No Consultant Found".to_owned()),
                value: 0,
            };
            // default_options
            dbg!("Incoming Panic");
        }

        let consultant_options = consultant_result.unwrap();
        let sov = SelectOptionsVec {
            vec: consultant_options,
        };
        // Cache the query in Redis -- `query:hash_value => result_hash`
        let val = serde_json::to_string(&sov).unwrap();
        let _: RedisResult<bool> = con.set_ex(key, &val, 120).await;

        return sov;
    }
}

async fn client_options(state: &web::Data<AppState>, r_state: &web::Data<RedisState>) -> SelectOptionsVec {
    let simple_query = SimpleQuery {
        query_str: "SELECT COALESCE(client_company_name, CONCAT(client_f_name, ' ', client_l_name)) AS key, id AS value 
        FROM clients ORDER BY key",
        int_args: None,
        str_args: None,
    };
    // Not hashing the query for options. Its always the same. Fixed Redis key so we can delete & invalidate when a record is added.
    // let query_hash = hash_query(&simple_query);
    // Search for key in Redis before reaching for DB
    let mut con = r_state.r_pool.get().await.unwrap();
    let prefix = "query";
    let key = format!("{}:{}", prefix, "client_options");

    let exists: Result<SelectOptionsVec, RedisError> = con.get(&key).await;

    dbg!(&exists);

    if exists.is_ok() {
        println!("Getting client_options from Redis");
        let result = exists.unwrap();
        return result;
    } else {
        println!("Getting client_options from DB");
        let client_result = sqlx::query_as::<_, SelectOption>(simple_query.query_str)
        .fetch_all(&state.db)
        .await;

        if client_result.is_err() {
            let err = "Error occurred while fetching location option KVs";
            let default_options = SelectOption {
                key: Some("No Client Found".to_owned()),
                value: 0,
            };
            // default_options
            dbg!("Incoming Panic");
        }

        let client_options = client_result.unwrap();
        let sov = SelectOptionsVec {
            vec: client_options,
        };
        // Cache the query in Redis -- `query:hash_value => result_hash`
        let val = serde_json::to_string(&sov).unwrap();
        let _: RedisResult<bool> = con.set_ex(key, &val, 86400).await;

        return sov;
    }
}

fn validate_consult_input(body: &ConsultPost) -> bool {
    // if body.consultant_id && body.linfa_assign {
    //     false
    // }
    true
}

#[derive(Debug, Serialize, FromRow, Deserialize)]
pub struct AttachmentResponse {
    attachment_id: i32,
}
#[derive(Debug, Serialize, Iterable, Deserialize)]
pub struct LinfaPredictionInput {
    pub meeting_duration: i32,
    pub consult_purpose_id: i32,
    pub territory_id: i32,
    pub specialty_id: i32,
    pub client_type: i32,
    pub hour_of_day: i32,
    pub location_id: i32,
    pub client_id: i32,
    pub notes_length: i32,
    pub received_follow_up: i32,
    pub num_attendees: i32,
}
#[derive(Debug, Serialize, FromRow, Deserialize)]
pub struct ClientDetailResult {
    pub client_type_id: i32,
    pub specialty_id: i32,
    pub territory_id: i32,
}

impl FromRedisValue for ClientDetailResult {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let v: String = from_redis_value(v)?;
        let result: Self = match serde_json::from_str::<Self>(&v) {
          Ok(v) => v,
          Err(_err) => return Err((ErrorKind::TypeError, "Parse to JSON Failed").into())
        };
        Ok(result)
    }
}

pub struct ClientDetails(i32, i32, i32);

pub async fn get_client_details(
    client_id: i32,
    db: &Pool<Postgres>,
    pool: &RedisPool,
) -> Result<ClientDetails, String> {
    let simple_query = SimpleQuery {
        query_str: "SELECT client_type_id, specialty_id, territory_id FROM clients WHERE id = $1",
        int_args: Some(vec![client_id]),
        str_args: None,
    };
    let query_hash = hash_query(&simple_query);
    // FIXME: Search for key in Redis before reaching for DB
    let mut con = pool.get().await.unwrap();
    let prefix = "query";
    let key = format!("{:?}:{:?}", prefix, query_hash);

    let exists: Result<ClientDetailResult, RedisError> = con.get(&key).await;

    dbg!(&exists);

    if exists.is_ok() {
        println!("Getting client_details from Redis");
        let client_details = exists.unwrap();
        return                 
            Ok(ClientDetails(
                client_details.client_type_id,
                client_details.specialty_id,
                client_details.territory_id,
            ));
    } else {
        println!("Getting client_details from DB");
        match sqlx::query_as::<_, ClientDetailResult>(simple_query.query_str)
        .bind(client_id)
        .fetch_optional(db)
        .await
        {
            Ok(client_details_opt) => {
                let client_details = client_details_opt.unwrap();
                // Cache the query in Redis -- `query:hash_value => result_hash`
                let val = serde_json::to_string(&client_details).unwrap();
                let _: RedisResult<bool> = con.set_ex(key, &val, 86400).await;

                Ok(ClientDetails(
                    client_details.client_type_id,
                    client_details.specialty_id,
                    client_details.territory_id,
                ))
            }
            Err(_) => Err("Error in Client Details".to_string()),
        }
    }
    // ClientDetails(1,1,2)
}

#[derive(Debug, Serialize, FromRow, Deserialize)]
pub struct ConsultResponse {
    id: i32,
}

use crate::linfa::LinfaPredictionResult;

#[post("/form")]
async fn create_consult(
    body: web::Form<ConsultPost>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    r_state: web::Data<RedisState>,
) -> impl Responder {
    println!("What");
    let is_valid = body.validate();
    if is_valid.is_err() {
        let validation_response = get_validation_response(is_valid);
        let body = hb
            .render("forms/form-validation", &validation_response)
            .unwrap();
        return HttpResponse::BadRequest()
            .header("HX-Retarget", "#consult_errors")
            .body(body);
    };
    if validate_consult_input(&body) {
        // FIXME Make Optional
        dbg!(&body);
        let consult_start_string =
            body.consult_start_date.clone() + " " + &body.consult_start_time + ":00 -06:00";
        let consult_start_dt =
            DateTime::parse_from_str(&consult_start_string, "%Y-%m-%d %H:%M:%S %z").unwrap();
        // End date
        let consult_end_string =
            if body.consult_end_date.is_empty() {
                body.consult_end_date.clone()
            } else {
                body.consult_end_date.clone() + " " + &body.consult_end_time + ":00 -06:00"
            };
        let consult_end_dt =
            if body.consult_end_date.is_empty() {
                consult_start_dt + Duration::hours(1)
            } else {
                DateTime::parse_from_str(&consult_end_string, "%Y-%m-%d %H:%M:%S %z").unwrap()
            };
        let consult_start_datetime_utc = consult_start_dt.with_timezone(&Utc);
        // Compute consultant_id based on Linfa assign
        let linfa_pred_result = if body.linfa_assign.is_some() {
            let cd = get_client_details(body.client_id, &state.db, &r_state.r_pool).await.unwrap();
            let diff = consult_end_dt - consult_start_dt;
            let duration = diff.num_minutes() as i32;
            println!("Meeting duration is {}", &duration);
            // Build Linfa
            let input = LinfaPredictionInput {
                client_type: cd.0,
                specialty_id: cd.1,
                territory_id: cd.2,
                meeting_duration: duration,
                hour_of_day: consult_start_dt.naive_local().hour() as i32,
                location_id: body.location_id,
                client_id: body.client_id,
                consult_purpose_id: body.consult_purpose_id,
                notes_length: body.notes.chars().count() as i32,
                // We are predicting for the optimal result, which is a follow up consult (1)
                received_follow_up: 1,
                num_attendees: body.num_attendees,
            };
            println!("Linfa will decide");
            let result = linfa_pred(&input, &state.db).await;
            result
            // let id = result.1;
            // id
        } else {
            LinfaPredictionResult("".to_string(), body.consultant_id)
        };
        
        let computed_consultant_id = linfa_pred_result.1;
        let texfile = linfa_pred_result.0;
        // Get Current User
        if body.attachment_path.is_some() && !body.attachment_path.as_ref().unwrap().is_empty() {
            let mime_type_id = mime_type_id_from_path(&body.attachment_path.as_ref().unwrap());
            let channel = "upload".to_string();
            let short_desc = "Replace me with genuine desc".to_string();
            match sqlx::query_as::<_, AttachmentResponse>(
                "INSERT INTO attachments (path, user_id, mime_type_id, channel, short_desc) VALUES ($1, $2, $3, $4, $5) RETURNING attachment_id",
            )
            .bind(body.attachment_path.clone().unwrap().trim().to_string())
            // FIXME
            .bind(body.client_id)
            .bind(mime_type_id)
            .bind(channel)
            .bind(short_desc)
            .fetch_one(&state.db)
            .await
            {
                Ok(attachment_resp) => {
                    let consult_attachments_array = vec![attachment_resp.attachment_id];
                    match sqlx::query_as::<_, ConsultResponse>(
                        "INSERT INTO consults (consult_purpose_id, consult_result_id, consultant_id, client_id, location_id, consult_start, consult_end, num_attendees, notes, consult_attachments, texfile) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULLIF($9, ''), $10, NULLIF($11, '')) RETURNING id",
                    )
                    .bind(body.consult_purpose_id as i32)
                    .bind(body.consult_result_id)
                    .bind(computed_consultant_id)
                    .bind(body.client_id)
                    .bind(body.location_id)
                    .bind(consult_start_dt)
                    .bind(consult_end_dt)
                    .bind(body.num_attendees)
                    .bind(body.notes.clone())
                    .bind(consult_attachments_array)
                    .bind(texfile)
                    .fetch_one(&state.db)
                    .await
                    {
                        Ok(consult_resp) => {
                            let user_alert = UserAlert::from((format!("Consult added successfully: ID #{:?}", consult_resp.id).as_str(), "alert_success"));
                            let body = hb.render("crud-api-inner", &user_alert).unwrap();
                            return HttpResponse::Ok().body(body);
                        }
                        Err(err) => {
                            dbg!(&err);
                            let user_alert = UserAlert::from((format!("Error Updating User After Adding Them As Consult: {:?}", err).as_str(), "alert_error"));
                            let body = hb.render("crud-api", &user_alert).unwrap();
                            return HttpResponse::Ok().body(body);
                        }
                    }
                }
                Err(err) => {
                    dbg!(&err);
                    let user_alert = UserAlert::from((format!("Error Adding the Attachment: {:?}", err).as_str(), "alert_error"));
                    let body = hb.render("crud-api", &user_alert).unwrap();
                    return HttpResponse::Ok().body(body);
                }
            }
        } else {
            // FIXME: If end_date null, just add an hour to start
            // NULLIF($2, 0) for Ints
            match sqlx::query_as::<_, ConsultResponse>(
                "INSERT INTO consults (consult_purpose_id, consult_result_id, consultant_id, client_id, location_id, consult_start, consult_end, num_attendees, notes, texfile) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULLIF($9, ''), NULLIF($10, '')) RETURNING id",
            )
            .bind(body.consult_purpose_id as i32)
            .bind(body.consult_result_id)
            .bind(computed_consultant_id)
            .bind(body.client_id)
            .bind(body.location_id)
            .bind(consult_start_dt)
            .bind(consult_end_dt)
            .bind(body.num_attendees)
            .bind(body.notes.clone())
            .bind(texfile)
            .fetch_one(&state.db)
            .await
            {
                Ok(consult_resp) => {
                    let user_alert = UserAlert::from((format!("Consult added successfully: ID #{:?}", consult_resp.id).as_str(), "alert_success"));
                    let body = hb.render("crud-api-inner", &user_alert).unwrap();
                    return HttpResponse::Ok().body(body);
                }
                Err(err) => {
                    dbg!(&err);
                    let error_msg = format!("Error occurred in (DB layer): {}.", err);
                    let validation_response = ValidationResponse::from((error_msg.as_str(), "validation_error"));
                    let body = hb.render("validation", &validation_response).unwrap();
                    return HttpResponse::Ok().body(body);
                }
            }
        }
    } else {
        let error_msg = "Validation error";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }
}

#[get("/form")]
async fn consult_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    r_state: web::Data<RedisState>,
    // path: web::Path<i32>,
) -> impl Responder {
    println!("consults_form firing");

    let location_options = location_options(&state, &r_state).await;
    let consultant_options = consultant_options(&state, &r_state).await;
    let client_options = client_options(&state, &r_state).await;

    let template_data = ConsultFormTemplate {
        entity: None,
        location_options: location_options.vec,
        consultant_options: consultant_options.vec,
        client_options: client_options.vec,
        consult_purpose_options: consult_purpose_options(),
        consult_result_options: consult_result_options(),
    };

    let body = hb.render("forms/consult-form", &template_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

fn get_consult_date(dt: Option<DateTime<Utc>>) -> Option<String> {
    if let Some(date) = dt {
        let end_dt_str = date.format("%Y-%m-%d %H:%M:%S.%f").to_string();
        let end_date = end_dt_str.split(" ").collect::<Vec<&str>>();
        Some(end_date[0].to_string())
    } else {
        None
    }
}

fn get_consult_time(dt: Option<DateTime<Utc>>) -> Option<String> {
    if let Some(date) = dt {
        let end_dt_str = date.format("%Y-%m-%d %H:%M:%S.%f").to_string();
        let end_date = end_dt_str.split(" ").collect::<Vec<&str>>();
        let end_date_str = end_date[1].to_string();
        let time_extract = end_date_str.split(":").collect::<Vec<&str>>();
        let end_time = format!(
            "{}:{}",
            time_extract[0].to_string(),
            time_extract[1].to_string()
        );
        Some(end_time)
    } else {
        None
    }
}

#[get("/form/{slug}")]
async fn consult_edit_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    r_state: web::Data<RedisState>,
    path: web::Path<String>,
) -> impl Responder {
    println!("consults_form firing");
    let consult_slug = path.into_inner();

    let query_result = sqlx::query_as!(
        ConsultFormRequest,
        "SELECT consultant_id, slug, consult_purpose_id, location_id, client_id, consult_result_id, consult_start, consult_end, notes 
            FROM consults 
            WHERE slug = $1
            ORDER by consult_start",
        consult_slug
    )
    .fetch_one(&state.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let error_msg = "Error occurred while fetching all consult records";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let consult = query_result.unwrap();

    let consult_with_dates = ConsultWithDates {
        notes: consult.notes,
        slug: consult.slug,
        consult_purpose_id: consult.consult_purpose_id,
        location_id: consult.location_id,
        consultant_id: consult.consultant_id,
        client_id: consult.client_id,
        consult_result_id: consult.consult_result_id,
        consult_start_date: get_consult_date(consult.consult_start),
        consult_start_time: get_consult_time(consult.consult_start),
        consult_end_date: get_consult_date(consult.consult_end),
        consult_end_time: get_consult_time(consult.consult_end),
    };

    let location_options = location_options(&state, &r_state).await;
    let consultant_options = consultant_options(&state, &r_state).await;
    let client_options = client_options(&state, &r_state).await;

    let consult_form_template = ConsultFormTemplate {
        entity: Some(consult_with_dates),
        location_options: location_options.vec,
        client_options: client_options.vec,
        consultant_options: consultant_options.vec,
        consult_purpose_options: consult_purpose_options(),
        consult_result_options: consult_result_options(),
    };

    let body = hb
        .render("forms/consult-form", &consult_form_template)
        .unwrap();
    return HttpResponse::Ok().body(body);
}

pub struct OwnedQuery {
    query_str: String,
    int_args: Vec<i32>,
    str_args: Vec<String>,
}

impl std::hash::Hash for OwnedQuery {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.query_str.hash(state);
        self.int_args.hash(state);
        self.str_args.hash(state);
    }
}

fn get_int_args(opts: &FilterOptions) -> Vec<i32> {
    let mut vec = vec![];
    if opts.page.is_some() {
        vec.push(opts.page.unwrap() as i32)
    };
    if opts.limit.is_some() {
        vec.push(opts.limit.unwrap() as i32)
    };
    vec
}

fn get_str_args(opts: &FilterOptions) -> Vec<String> {
    let mut vec = vec![];
    if opts.search.is_some() {
        vec.push(opts.search.as_ref().unwrap().to_owned())
    };
    if opts.key.is_some() {
        vec.push(opts.key.as_ref().unwrap().to_owned())
    };
    if opts.dir.is_some() {
        vec.push(opts.dir.as_ref().unwrap().to_owned())
    };
    vec
}

async fn sort_query(
    opts: &FilterOptions,
    pool: &Pool<Postgres>,
    r_pool: &RedisPool,
) -> Result<ConsultListVec, Error> {
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;
    dbg!(&opts);
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT 
        consults.id,
        consults.slug, 
        consults.consult_purpose_id AS purpose,
        consults.consult_result_id AS result,
        CONCAT(consultant_f_name, ' ', consultant_l_name) AS consultant_name, 
        location_name, 
        COALESCE(client_company_name, CONCAT(client_f_name, ' ', client_l_name)) AS client_name, 
        consult_start, 
        consult_end, 
        notes 
    FROM consults
    INNER JOIN clients ON consults.client_id = clients.id
    INNER JOIN locations ON consults.location_id = locations.id
    INNER JOIN consultants ON consults.consultant_id = consultants.id",
    );

    if let Some(search) = &opts.search {
        query.push(" WHERE notes LIKE ");
        query.push(String::from(
            "'%".to_owned() + &opts.search.clone().unwrap() + "%'",
        ));
    }

    if let Some(sort_key) = &opts.key {
        query.push(" ORDER BY ");
        query.push(String::from(
            sort_key.to_owned() + " " + &opts.dir.clone().unwrap(),
        ));
    } else {
        query.push(" ORDER BY consults.updated_at DESC, consults.created_at DESC");
    }

    query.push(" LIMIT ");
    query.push_bind(limit as i32);
    query.push(" OFFSET ");
    query.push_bind(offset as i32);

    let owned_query = OwnedQuery {
        query_str: query.sql().to_owned(),
        int_args: get_int_args(opts),
        str_args: get_str_args(opts),
    };

    dbg!(&owned_query.query_str);
    dbg!(&owned_query.int_args);
    dbg!(&owned_query.str_args);

    let query_hash = hash_owned_query(&owned_query);
    println!("query hash = {}", query_hash);
    // Search for key in Redis before reaching for DB
    let mut con = r_pool.get().await.unwrap();
    let prefix = "query";
    let key = format!("{}:{}", prefix, query_hash);

    let exists: Result<ConsultListVec, RedisError> = con.get(&key).await;
    dbg!(&exists);

    if exists.is_ok() {
        println!("Getting consultant_options from Redis");
        let result = exists.unwrap();
        return Ok(result);
    } else {
        println!("Getting consultant_options from DB");

        let q_build = query.build();
        let res = q_build.fetch_all(pool).await;

        // This almost got me there. Error on .as_str() for consult_start column
        // let consults = res.unwrap().iter().map(|row| row_to_consult_list(row)).collect::<Vec<ConsultList>>();

        let consults = res
            .unwrap()
            .iter()
            .map(|row| ConsultList::from_row(row).unwrap())
            .collect::<Vec<ConsultList>>();

        let clv = ConsultListVec {
            vec: consults,
        };
        // Cache the query in Redis -- `query:hash_value => result_hash`
        let val = serde_json::to_string(&clv).unwrap();
        let _: RedisResult<bool> = con.set_ex(key, &val, 120).await;

        return Ok(clv);
    }

    // let query_str = query.build().sql().into();
    // dbg!(&query_str);
    // query_str
    // res
}

// Had to remove conflicting FromRow in the derive list
impl<'r> FromRow<'r, PgRow> for ConsultList {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        let id = row.try_get("id")?;
        let slug = row.try_get("slug")?;
        let purpose = row.try_get("purpose")?;
        let consultant_name = row.try_get("consultant_name")?;
        let location_name = row.try_get("location_name")?;
        let client_name = row.try_get("client_name")?;
        let result = row.try_get("result")?;
        let consult_start = row.try_get("consult_start")?;
        let consult_end = row.try_get("consult_end")?;
        let notes = row.try_get("notes")?;

        Ok(ConsultList {
            id,
            slug,
            purpose,
            consultant_name,
            location_name,
            client_name,
            result,
            consult_start,
            consult_end,
            notes,
        })
    }
}

#[get("/list")]
pub async fn get_consults_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
    r_state: web::Data<RedisState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match redis_validate_and_get_user(cookie, &r_state).await {
            Ok(user_opt) => {
                if let Some(user) = user_opt {
                    println!("get_consultants_handler firing");
                    let limit = opts.limit.unwrap_or(10);
                    let offset = (opts.page.unwrap_or(1) - 1) * limit;

                    // QueryBuilder gets the query correct but end up w/ Vec<PgRow>. Need to get to Vec<Consult> or impl Serialize for PgRow?
                    let query_result = sort_query(&opts, &state.db, &r_state.r_pool).await;

                    dbg!(&query_result);

                    if query_result.is_err() {
                        let error_msg = "Error occurred while fetching all consultant records";
                        let validation_response =
                            ValidationResponse::from((error_msg, "validation_error"));
                        let body = hb.render("validation", &validation_response).unwrap();
                        return HttpResponse::Ok().body(body);
                    }

                    let consults = query_result.unwrap();

                    let consults_table_data = ResponsiveTableData {
                        entity_type_id: 6,
                        vec_len: consults.vec.len(),
                        lookup_url: "/consult/list?page=".to_string(),
                        page: opts.page.unwrap_or(1),
                        entities: consults.vec,
                        subscriptions: subs_from_user(&user),
                    };

                    // Only return whole Table if brand new
                    if opts.key.is_none() && opts.search.is_none() {
                        let body = hb.render("responsive-table", &consults_table_data).unwrap();
                        return HttpResponse::Ok().body(body);
                    } else {
                        let body = hb
                            .render("responsive-table-inner", &consults_table_data)
                            .unwrap();
                        return HttpResponse::Ok().body(body);
                    }
                } else {
                    let message = "User Option is a None".to_owned();
                    let body = hb.render("index", &message).unwrap();
                    return HttpResponse::Ok().body(body);
                };
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

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct ViewData {
    attachments: Vec<ConsultAttachments>,
}

#[get("/attachments/{slug}")]
async fn get_attachments(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let consult_slug = path.into_inner();
    println!("Get Attachments firing");

    let query_result = sqlx::query_as!(
        ConsultAttachments,
        "WITH attachs AS (
            SELECT consult_attachments AS ca FROM consults WHERE slug = $1
        )
        SELECT 
            attachment_id, 
            path,
            short_desc,
            mime_type_id 
        FROM attachments
        WHERE attachment_id = ANY ( SELECT UNNEST(ca) FROM attachs)",
        consult_slug
    )
    .fetch_all(&state.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let error_msg = "Error occurred while fetching attachments for consult";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let attachments = query_result.unwrap();

    let view_data = ViewData {
        attachments: attachments,
    };

    let body = hb.render("attachments-view", &view_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

#[derive(Debug, Serialize, FromRow, Deserialize)]
pub struct ConsultAvailability {
    scheduled: String,
}

#[get("/availability")]
async fn availability(hb: web::Data<Handlebars<'_>>, state: web::Data<AppState>) -> impl Responder {
    println!("Availability firing");

    let view_data = ConsultAvailability {
        scheduled: "Hey".to_string(),
    };

    let body = hb.render("consult-availability", &view_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

fn read_file_buffer(filepath: &str, new_filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    const BUFFER_LEN: usize = 512;
    let mut buffer = [0u8; BUFFER_LEN];
    let mut file = File::open(filepath)?;
    let mut new_file = fs::File::create(&new_filepath).unwrap();
    loop {
        let read_count = file.read(&mut buffer)?;
        let _ = new_file.write_all(&buffer[..read_count]);

        if read_count != BUFFER_LEN {
            break;
        }
    }
    Ok(())
}

#[post("/upload")]
async fn upload(
    mut payload: Multipart,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
) -> HttpResponse {
    let max_file_size: usize = 20_000;
    let max_file_count: usize = 3;
    let legal_file_types: [Mime; 6] = [
        IMAGE_GIF,
        IMAGE_JPEG,
        IMAGE_PNG,
        APPLICATION_JSON,
        APPLICATION_PDF,
        TEXT_CSV,
    ];

    let content_length: usize = match req.headers().get(CONTENT_LENGTH) {
        Some(header_value) => header_value.to_str().unwrap_or("0").parse().unwrap(),
        None => 0,
    };

    dbg!(&content_length);

    if content_length == 0 || content_length > max_file_size {
        let error_msg = "Content Length Error";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::BadRequest()
            .header("HX-Retarget", "#validation_response")
            .body(body);
    }

    let mut current_count: usize = 0;
    let mut filenames: Vec<String> = vec![];
    loop {
        if current_count >= max_file_count {
            break;
        }

        if let Ok(Some(mut field)) = payload.try_next().await {
            if field.name() != "upload" {
                continue;
            }
            let filetype: Option<&Mime> = field.content_type();
            dbg!(filetype);
            if filetype.is_none() {
                continue;
            }
            if !legal_file_types.contains(&filetype.unwrap()) {
                // continue;
                let error_msg = "File Type Not Allowed";
                let validation_response = ValidationResponse::from((error_msg, "validation_error"));
                let body = hb.render("validation", &validation_response).unwrap();
                return HttpResponse::BadRequest()
                    .header("HX-Retarget", "#validation_response")
                    .body(body);
            }
            let dir: &str = "./static/images/consults/";

            let const_uuid = Uuid::new_v4();

            let destination: String = format!(
                "{}{}-{}",
                dir,
                const_uuid,
                field.content_disposition().get_filename().unwrap(),
            );
            dbg!(&destination);
            let mut saved_file = fs::File::create(&destination).unwrap();
            while let Ok(Some(chunk)) = field.try_next().await {
                let _ = saved_file.write_all(&chunk).unwrap();
            }
            let filename = format!(
                "{}{}.{}",
                dir,
                const_uuid,
                Path::new(field.content_disposition().get_filename().unwrap())
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap_or("none")
            );
            dbg!(&filename);

            let mut to_save = filename.clone();
            if let Some((_, desired)) = to_save.split_once("./static") {
                to_save = desired.to_owned();
            }
            dbg!(&filename);
            dbg!(&to_save);

            filenames.push(to_save);

            web::block(move || async move {
                let updated_doc = fs::File::open(&destination).unwrap();
                // FIXME
                let extension = Path::new(&destination)
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap_or("none");
                let filename = format!("{}{}.{}", dir, const_uuid, extension);
                let contents = read_file_buffer(&destination, &filename);
                let _ = fs::remove_file(&destination).unwrap();
            })
            .await
            .unwrap()
            .await;
        } else {
            break;
        }
        current_count += 1;
    }
    // Message here is filename because we want that set to value via Hyperscript
    let success_msg = &filenames[0];
    let validation_response =
        ValidationResponse::from((success_msg.as_str(), "validation_success"));
    let body = hb.render("validation", &validation_response).unwrap();
    return HttpResponse::Ok().body(body);
}

/****
Tests
****/
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hbs_helpers::{concat_str_args, int_eq},
        test_common::{self, *},
    };
    use test_context::test_context;

    fn mock_locations() -> Vec<SelectOption> {
        [
            SelectOption::from((1, Some("Loc 1".to_string()))),
            SelectOption::from((2, Some("Loc 2".to_string()))),
        ]
        .to_vec()
    }

    fn mock_clients() -> Vec<SelectOption> {
        [
            SelectOption::from((1, Some("Client 1".to_string()))),
            SelectOption::from((2, Some("Client 2".to_string()))),
        ]
        .to_vec()
    }

    fn mock_consultants() -> Vec<SelectOption> {
        [
            SelectOption::from((1, Some("Consultant 1".to_string()))),
            SelectOption::from((2, Some("Consultant 2".to_string()))),
        ]
        .to_vec()
    }

    #[test_context(Context)]
    #[test]
    fn create_form_renders_add_header(ctx: &mut Context) {
        let template_data = ConsultFormTemplate {
            entity: None,
            location_options: mock_locations(),
            client_options: mock_clients(),
            consultant_options: mock_consultants(),
            consult_purpose_options: consult_purpose_options(),
            consult_result_options: consult_result_options(),
        };
        let mut hb = Handlebars::new();
        hb.register_templates_directory(".hbs", "./templates")
            .unwrap();
        hb.register_helper("int_eq", Box::new(int_eq));
        let body = hb.render("forms/consult-form", &template_data).unwrap();
        let dom = tl::parse(&body, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();

        let element = dom
            .get_element_by_id("consult_form_header")
            .expect("Failed to find element")
            .get(parser)
            .unwrap();

        // Assert
        assert_eq!(element.inner_text(parser), "Add Consult");
        // assert_eq!(1, 1);
    }

    #[test_context(Context)]
    #[test]
    fn edit_form_renders_edit_header(ctx: &mut Context) {
        let mock_consult_with_dates = ConsultWithDates {
            consultant_id: Some(1),
            location_id: 1,
            client_id: 1,
            consult_purpose_id: 1,
            slug: "d574a28d-909f-4b44-99c3-43a30f618185".to_string(),
            notes: Some("Good meeting".to_string()),
            consult_result_id: 2,
            consult_start_date: Some("2023-09-10".to_string()),
            consult_start_time: Some("14:30".to_string()),
            consult_end_date: Some("2023-09-10".to_string()),
            consult_end_time: Some("15:30".to_string()),
        };
        let template_data = ConsultFormTemplate {
            entity: Some(mock_consult_with_dates),
            location_options: mock_locations(),
            client_options: mock_clients(),
            consultant_options: mock_consultants(),
            consult_purpose_options: consult_purpose_options(),
            consult_result_options: consult_result_options(),
        };
        let mut hb = Handlebars::new();
        hb.register_templates_directory(".hbs", "./templates")
            .unwrap();
        hb.register_helper("int_eq", Box::new(int_eq));
        hb.register_helper("concat_str_args", Box::new(concat_str_args));
        let body = hb.render("forms/consult-form", &template_data).unwrap();
        let dom = tl::parse(&body, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();

        let element = dom
            .get_element_by_id("consult_form_header")
            .expect("Failed to find element")
            .get(parser)
            .unwrap();

        // Assert
        assert_eq!(element.inner_text(parser), "Edit Consult");
        // assert_eq!(1, 1);
    }
}
