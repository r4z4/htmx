use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse, Responder, Scope,
};

use chrono::{DateTime, NaiveDate, Utc};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Execute, Postgres, Pool, postgres::PgRow, Error, FromRow, Row};

use crate::{
    config::{FilterOptions, ResponsiveTableData, SelectOption},
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
}

async fn location_options(state: &web::Data<AppState>) -> Vec<SelectOption> {
    let location_result = sqlx::query_as!(
        SelectOption,
        "SELECT location_id AS value, location_name AS key 
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
        "SELECT CONCAT(consultant_f_name, ' ',consultant_l_name) AS key, consultant_id AS value 
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
        "SELECT COALESCE(client_company_name, CONCAT(client_f_name, ' ', client_l_name)) AS key, client_id AS value 
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

#[post("/form")]
async fn create_consult(
    body: web::Form<ConsultPost>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
) -> impl Responder {
    match sqlx::query_as::<_, ConsultPost>(
        "INSERT INTO consults (consultant_id, client_id, location_id, consult_start, consult_end, notes) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
    )
    .bind(body.consultant_id)
    .bind(body.client_id)
    .bind(body.location_id)
    .bind(body.consult_start)
    .bind(body.consult_end)
    .bind(body.notes.clone())
    .fetch_one(&state.db)
    .await
    {
        Ok(consult) => {
            let body = hb.render("consult/consult-list", &{}).unwrap();
            return HttpResponse::Ok().body(body);
        }
        Err(err) => {
            let body = hb.render("validation", &err.to_string()).unwrap();
            return HttpResponse::Ok().body(body);
        }
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
    };

    let body = hb.render("forms/consult-form", &template_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

fn get_consult_end_date(dt: Option<DateTime<Utc>>) -> Option<String> {
    if let Some(date) = dt {
        let end_dt_str = date.format("%Y-%m-%d %H:%M:%S.%f").to_string();
        let end_date = end_dt_str.split(" ").collect::<Vec<&str>>();
        Some(end_date[0].to_string())
    } else {
        None
    }
}

fn get_consult_end_time(dt: Option<DateTime<Utc>>) -> Option<String> {
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
        "SELECT consultant_id, slug, location_id, client_id, consult_start, consult_end, notes 
            FROM consults 
            WHERE slug = $1
            ORDER by consult_start",
        consult_slug
    )
    .fetch_one(&state.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let err = "Error occurred while fetching all consult records";
        // return HttpResponse::InternalServerError()
        //     .json(json!({"status": "error","message": message}));
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let consult = query_result.unwrap();
    let start_dt_str = consult
        .consult_start
        .format("%Y-%m-%d %H:%M:%S.%f")
        .to_string();
    let start_date = start_dt_str.split(" ").collect::<Vec<&str>>();
    let start_str = start_date[1].to_string();
    let time_extract = start_str.split(":").collect::<Vec<&str>>();
    let start_time = format!(
        "{}:{}",
        time_extract[0].to_string(),
        time_extract[1].to_string()
    );

    let consult_with_dates = ConsultWithDates {
        notes: consult.notes,
        slug: consult.slug,
        location_id: consult.location_id,
        consultant_id: consult.consultant_id,
        client_id: consult.client_id,
        consult_start_date: start_date[0].to_string(),
        consult_start_time: start_time,
        consult_end_date: get_consult_end_date(consult.consult_end),
        consult_end_time: get_consult_end_time(consult.consult_end),
    };

    let location_options = location_options(&state).await;
    let consultant_options = consultant_options(&state).await;
    let client_options = client_options(&state).await;

    let consult_form_template = ConsultFormTemplate {
        entity: Some(consult_with_dates),
        location_options: location_options,
        client_options: client_options,
        consultant_options: consultant_options,
    };

    let body = hb
        .render("forms/consult-form", &consult_form_template)
        .unwrap();
    return HttpResponse::Ok().body(body);
}


async fn sort_query(opts: &FilterOptions, pool: &Pool<Postgres>,) -> Result<Vec<ConsultList>, Error> {
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;
    dbg!(&opts);
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT 
        consults.slug, 
        CONCAT(consultant_f_name, ' ', consultant_l_name) AS consultant_name, 
        location_name, 
        COALESCE(client_company_name, CONCAT(client_f_name, ' ', client_l_name)) AS client_name, 
        consult_start, 
        consult_end, 
        notes 
    FROM consults
    INNER JOIN clients ON consults.client_id = clients.client_id
    INNER JOIN locations ON consults.location_id = locations.location_id
    INNER JOIN consultants ON consults.consultant_id = consultants.consultant_id"
    );

    if let Some(search) = &opts.search {
        query.push(" WHERE notes LIKE ");
        query.push(String::from("'%".to_owned() + &opts.search.clone().unwrap() + "%'"));
    }

    if let Some(sort_key) = &opts.key {
        query.push(" ORDER BY ");
        query.push(String::from(sort_key.to_owned() + " " + &opts.dir.clone().unwrap()));
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

    let consults = res.unwrap().iter().map(|row| ConsultList::from_row(row).unwrap()).collect::<Vec<ConsultList>>();

    Ok(consults)

    // let query_str = query.build().sql().into();
    // dbg!(&query_str);
    // query_str
    // res
}

// Had to remove conflicting FromRow in the derive list
impl<'r> FromRow<'r, PgRow> for ConsultList {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        let slug = row.try_get("slug")?;
        let consultant_name = row.try_get("consultant_name")?;
        let location_name = row.try_get("location_name")?;
        let client_name = row.try_get("client_name")?;
        let consult_start = row.try_get("consult_start")?;
        let consult_end = row.try_get("consult_end")?;
        let notes = row.try_get("notes")?;

        Ok(ConsultList{ slug, consultant_name, location_name, client_name, consult_start, consult_end, notes })
    }
}

#[get("/list")]
pub async fn get_consults_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
) -> impl Responder {
    println!("get_consultants_handler firing");
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    // QueryBuilder gets the query correct but end up w/ Vec<PgRow>. Need to get to Vec<Consult> or impl Serialize for PgRow?
    let query_result = sort_query(&opts, &data.db).await;

    // let query_result = sqlx::query_as!(
    //     ConsultList,
    //     "SELECT 
    //         consults.slug, 
    //         CONCAT(consultant_f_name, ' ', consultant_l_name) AS consultant_name, 
    //         location_name, 
    //         COALESCE(client_company_name, CONCAT(client_f_name, ' ', client_l_name)) AS client_name, 
    //         consult_start, 
    //         consult_end, 
    //         notes 
    //     FROM consults
    //     INNER JOIN clients ON consults.client_id = clients.client_id
    //     INNER JOIN locations ON consults.location_id = locations.location_id
    //     INNER JOIN consultants ON consults.consultant_id = consultants.consultant_id
    //     ORDER BY consults.updated_at DESC, consults.created_at DESC
    //     LIMIT $1 OFFSET $2",
    //     limit as i32,
    //     offset as i32
    // )
    // .fetch_all(&data.db)
    // .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let err = "Error occurred while fetching all consultant records";
        // return HttpResponse::InternalServerError()
        //     .json(json!({"status": "error","message": message}));
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let consults = query_result.unwrap();

    let consults_table_data = ResponsiveTableData {
        entity_type_id: 6,
        vec_len: consults.len(),
        lookup_url: "/consult/list?page=".to_string(),
        page: opts.page.unwrap_or(1),
        entities: consults,
    };

    // Only return whole Table if brand new
    if opts.key.is_none() && opts.search.is_none() {
        let body = hb
            .render("responsive-table", &consults_table_data)
            .unwrap();
        return HttpResponse::Ok().body(body);
    } else {
        let body = hb
            .render("responsive-table-inner", &consults_table_data)
            .unwrap();
        return HttpResponse::Ok().body(body);
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
        let err = "Error occurred while fetching attachments for consult";
        let body = hb.render("validation", &err).unwrap();
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
    use test_context::{test_context, TestContext};

    fn mock_locations() -> Vec<SelectOption> {
        [
            SelectOption {
                key: Some("Loc 1".to_owned()),
                value: 1,
            },
            SelectOption {
                key: Some("Loc 2".to_owned()),
                value: 2,
            },
        ]
        .to_vec()
    }

    fn mock_clients() -> Vec<SelectOption> {
        [
            SelectOption {
                key: Some("Client 1".to_owned()),
                value: 1,
            },
            SelectOption {
                key: Some("Client 2".to_owned()),
                value: 2,
            },
        ]
        .to_vec()
    }

    fn mock_consultants() -> Vec<SelectOption> {
        [
            SelectOption {
                key: Some("Consultant 1".to_owned()),
                value: 1,
            },
            SelectOption {
                key: Some("Consultant 2".to_owned()),
                value: 2,
            },
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
            consultant_id: 1,
            location_id: 1,
            client_id: 1,
            slug: "d574a28d-909f-4b44-99c3-43a30f618185".to_string(),
            notes: Some("Good meeting".to_string()),
            consult_start_date: "2023-09-10".to_string(),
            consult_start_time: "14:30".to_string(),
            consult_end_date: Some("2023-09-10".to_string()),
            consult_end_time: Some("15:30".to_string()),
        };
        let template_data = ConsultFormTemplate {
            entity: Some(mock_consult_with_dates),
            location_options: mock_locations(),
            client_options: mock_clients(),
            consultant_options: mock_consultants(),
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
