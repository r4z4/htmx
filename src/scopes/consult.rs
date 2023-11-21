use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use actix_multipart::Multipart;
use actix_web::{
    get,
    http::header::CONTENT_LENGTH,
    post,
    web::{self, Data, Json},
    HttpRequest, HttpResponse, Responder, Scope,
};
use chrono::{DateTime, Utc};
use futures_util::TryStreamExt;
use handlebars::Handlebars;
use mime::{
    Mime, APPLICATION_JSON, APPLICATION_PDF, IMAGE_GIF, IMAGE_JPEG, IMAGE_PNG, TEXT_CSV,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, Error, FromRow, Pool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::{
    config::{FilterOptions, ResponsiveTableData, SelectOption, UserAlert, ValidationResponse, test_subs, consult_result_options, consult_purpose_options, mime_type_id_from_path, validate_and_get_user, subs_from_user},
    models::model_consult::{
        ConsultAttachments, ConsultFormRequest, ConsultFormTemplate, ConsultList, ConsultPost,
        ConsultWithDates,
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
        .service(get_attachments)
        .service(upload)
        .service(availability)
}

async fn location_options(state: &web::Data<AppState>) -> Vec<SelectOption> {
    let location_result = sqlx::query_as!(
        SelectOption,
        "SELECT id AS value, location_name AS key 
        FROM locations 
        ORDER by location_name"
    )
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
    location_options
}

async fn consultant_options(state: &web::Data<AppState>) -> Vec<SelectOption> {
    let consultant_result = sqlx::query_as!(
        SelectOption,
        "SELECT CONCAT(consultant_f_name, ' ',consultant_l_name) AS key, id AS value 
        FROM consultants ORDER BY key"
    )
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
    consultant_options
}

async fn client_options(state: &web::Data<AppState>) -> Vec<SelectOption> {
    let client_result = sqlx::query_as!(
        SelectOption,
        "SELECT COALESCE(client_company_name, CONCAT(client_f_name, ' ', client_l_name)) AS key, id AS value 
        FROM clients ORDER BY key"
    )
    .fetch_all(&state.db)
    .await;

    if client_result.is_err() {
        let err = "Error occurred while fetching location option KVs";
        let default_options = SelectOption {
            key: Some("No Clientt Found".to_owned()),
            value: 0,
        };
        // default_options
        dbg!("Incoming Panic");
    }

    let client_options = client_result.unwrap();
    client_options
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

#[derive(Debug, Serialize, FromRow, Deserialize)]
pub struct ConsultResponse {
    consult_id: i32,
}

#[post("/form")]
async fn create_consult(
    body: web::Form<ConsultPost>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
) -> impl Responder {
    if validate_consult_input(&body) {
        // FIXME Make Optional
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
            DateTime::parse_from_str(&consult_start_string, "%Y-%m-%d %H:%M:%S %z").unwrap();
        dbg!(&consult_start_datetime);
        let consult_start_datetime_utc = consult_start_datetime.with_timezone(&Utc);
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
                    // let consultant_id =
                    //     if body.linfa_assign {
                    //         linfa_pred()
                    //     } else {
                    //         body.consultant_id
                    //     };
                    match sqlx::query_as::<_, ConsultResponse>(
                        "INSERT INTO consults (consultant_id, client_id, location_id, consult_start, consult_end, notes, consult_attachments) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
                    )
                    .bind(body.consultant_id)
                    .bind(body.client_id)
                    .bind(body.location_id)
                    .bind(consult_start_datetime)
                    .bind(consult_end_datetime)
                    .bind(body.notes.clone())
                    .bind(consult_attachments_array)
                    .fetch_one(&state.db)
                    .await
                    {
                        Ok(consult_response) => {
                            let user_alert = UserAlert {
                                msg: format!("Consult added successfully: ID #{:?}", consult_response.consult_id),
                                alert_class: "alert_success".to_owned(),
                            };
                            let body = hb.render("crud-api", &user_alert).unwrap();
                            return HttpResponse::Ok().body(body);
                        }
                        Err(err) => {
                            dbg!(&err);
                            let user_alert = UserAlert {
                                msg: format!("Error Updating User After Adding Them As Consult: {:?}", err),
                                alert_class: "alert_error".to_owned(),
                            };
                            let body = hb.render("crud-api", &user_alert).unwrap();
                            return HttpResponse::Ok().body(body);
                        }
                    }
                }
                Err(err) => {
                    dbg!(&err);
                    let user_alert = UserAlert {
                        msg: format!("Error Adding the Attachment: {:?}", err),
                        alert_class: "alert_error".to_owned(),
                    };
                    let body = hb.render("crud-api", &user_alert).unwrap();
                    return HttpResponse::Ok().body(body);
                }
            }
        } else {
            match sqlx::query_as::<_, ConsultPost>(
                "INSERT INTO consults (consultant_id, client_id, location_id, consult_start, consult_end, notes) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
            )
            .bind(body.consultant_id)
            .bind(body.client_id)
            .bind(body.location_id)
            .bind(consult_start_datetime)
            .bind(consult_end_datetime)
            .bind(body.notes.clone())
            .fetch_one(&state.db)
            .await
            {
                Ok(consult) => {
                    let body = hb.render("consult/consult-list", &{}).unwrap();
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

    let location_options = location_options(&state).await;
    let consultant_options = consultant_options(&state).await;
    let client_options = client_options(&state).await;

    let consult_form_template = ConsultFormTemplate {
        entity: Some(consult_with_dates),
        location_options: location_options,
        client_options: client_options,
        consultant_options: consultant_options,
        consult_purpose_options: consult_purpose_options(),
        consult_result_options: consult_result_options(),
    };

    let body = hb
        .render("forms/consult-form", &consult_form_template)
        .unwrap();
    return HttpResponse::Ok().body(body);
}

async fn sort_query(
    opts: &FilterOptions,
    pool: &Pool<Postgres>,
) -> Result<Vec<ConsultList>, Error> {
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

    let q_build = query.build();
    let res = q_build.fetch_all(pool).await;

    // This almost got me there. Error on .as_str() for consult_start column
    // let consults = res.unwrap().iter().map(|row| row_to_consult_list(row)).collect::<Vec<ConsultList>>();

    let consults = res
        .unwrap()
        .iter()
        .map(|row| ConsultList::from_row(row).unwrap())
        .collect::<Vec<ConsultList>>();

    Ok(consults)

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
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match validate_and_get_user(cookie, &state).await 
        {
            Ok(user_opt) => {
                if let Some(user) = user_opt {
                    println!("get_consultants_handler firing");
                    let limit = opts.limit.unwrap_or(10);
                    let offset = (opts.page.unwrap_or(1) - 1) * limit;

                    // QueryBuilder gets the query correct but end up w/ Vec<PgRow>. Need to get to Vec<Consult> or impl Serialize for PgRow?
                    let query_result = sort_query(&opts, &state.db).await;

                    dbg!(&query_result);

                    if query_result.is_err() {
                        let error_msg = "Error occurred while fetching all consultant records";
                        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
                        let body = hb.render("validation", &validation_response).unwrap();
                        return HttpResponse::Ok().body(body);
                    }

                    let consults = query_result.unwrap();

                    let consults_table_data = ResponsiveTableData {
                        entity_type_id: 6,
                        vec_len: consults.len(),
                        lookup_url: "/consult/list?page=".to_string(),
                        page: opts.page.unwrap_or(1),
                        entities: consults,
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
                    return HttpResponse::Ok().body(body)
                };
            },
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
    scheduled: String
}

#[get("/availability")]
async fn availability(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
) -> impl Responder {
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
