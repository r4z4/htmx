use std::vec;

use actix_web::{
    get, post,
    web::{self, Data},
    HttpRequest, HttpResponse, Responder, Scope,
};

use chrono::{DateTime, Utc};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

use crate::{
    config::{
        self, subs_from_user, test_subs, FilterOptions, ResponsiveTableData,
        UserAlert, ValidationResponse, ACCEPTED_SECONDARIES, redis_validate_and_get_user, ValidationErrorMap, FormErrorResponse,
    },
    models::{
        model_admin::{
            AdminSubadminFormQuery, AdminUserFormTemplate, AdminUserList, AdminUserPostRequest,
        },
        model_admin::{
            AdminSubadminFormTemplate, AdminSubadminPostRequest, AdminUserFormQuery,
            AdminUserPostResponse,
        },
    },
    AppState, RedisState,
};

pub fn admin_scope() -> Scope {
    web::scope("/admin")
        // .route("/users", web::get().to(get_users_handler))
        .service(user_form)
        .service(subadmin_form)
        .service(edit_user)
        .service(admin_home)
        .service(recent_activity)
        .service(get_contact_submissions)
        //.service(edit_subadmin)
        .service(get_users_handler)
}

fn entity_type_from_user_type(user_type_id: i32) -> i32 {
    match user_type_id {
        1 => 3,
        2 => 3,
        3 => 1,
        4 => 1,
        // FIXME
        _ => 0,
    }
}

#[get("/home")]
async fn admin_home(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: Data<AppState>,
    r_state: Data<RedisState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        // FIXME: Add user_type check
        match redis_validate_and_get_user(cookie, &r_state)
        .await 
        {
            Ok(user) => {
                let body = hb.render("admin-home", &user).unwrap();
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
        let message = "Your session seems to have expired. Please login again.".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PgStat {
    schemaname: Option<String>,
    relname: Option<String>,
    id: Option<i32>,
    slug: Option<Uuid>,
    heap_blks_read: Option<i64>,
    heap_blks_hit: Option<i64>,
    idx_blks_read: Option<i64>,
    idx_blks_hit: Option<i64>,
    toast_blks_read: Option<i64>,
    toast_blks_hit: Option<i64>,
    tidx_blks_read: Option<i64>,
    tidx_blks_hit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableQuery {
    query: String,
    slug: String,
}

#[get("/recent-activity")]
async fn recent_activity(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
    r_state: web::Data<RedisState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match redis_validate_and_get_user(cookie, &r_state).await {
            Ok(user) => {
                let recent = sqlx::query_as!(
                    PgStat,
                    "SELECT schemaname, relname, relid::integer AS id, gen_random_uuid() AS slug, heap_blks_read, heap_blks_hit, idx_blks_read, idx_blks_hit, toast_blks_read, toast_blks_hit, tidx_blks_read, tidx_blks_hit FROM pg_statio_user_tables;
                    ",
                )
                .fetch_all(&state.db)
                .await;

                if recent.is_err() {
                    let error_msg = "Error occurred while fetching from pg_stat";
                    let validation_response =
                        ValidationResponse::from((error_msg, "validation_error"));
                    let body = hb.render("validation", &validation_response).unwrap();
                    return HttpResponse::Ok().body(body);
                }

                let recent_queries = recent.unwrap();

                let f_opts = FilterOptions::from(&opts);

                let recent_queries_table_data = ResponsiveTableData {
                    entity_type_id: 8,
                    vec_len: recent_queries.len(),
                    lookup_url: "/consultant/list?page=".to_string(),
                    opts: f_opts,
                    // page: opts.page.unwrap_or(1),
                    entities: recent_queries,
                    subscriptions: subs_from_user(&user),
                };

                // Only return whole Table if brand new
                if opts.key.is_none() && opts.search.is_none() {
                    let body = hb
                        .render("responsive-table", &recent_queries_table_data)
                        .unwrap();
                    return HttpResponse::Ok().body(body);
                } else {
                    let body = hb
                        .render("responsive-table-inner", &recent_queries_table_data)
                        .unwrap();
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContactSubmission {
    name: String,
    email: String,
    phone: String,
    ip_addr: String,
    message: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, FromRow)]
pub struct TableRow {
    pub th: String,
    pub tds: ContactSubmission,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
pub struct FixedTableData2 {
    pub table_headers: Vec<String>,
    pub table_rows: Vec<TableRow>,
}

#[get("/list/contact-us")]
pub async fn get_contact_submissions(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
) -> impl Responder {
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let query_result = sqlx::query_as!(
        ContactSubmission,
        "SELECT name, email, phone, ip_addr, message, created_at
        FROM contact_submissions
        ORDER by created_at
        LIMIT $1 OFFSET $2",
        limit as i32,
        offset as i32
    )
    .fetch_all(&data.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let error_msg = "Error occurred while fetching all contact submission records";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let messages = query_result.unwrap();

    let table_headers = vec![
        "name".to_owned(),
        "email".to_owned(),
        "phone".to_owned(),
        "ip_addr".to_owned(),
        "message".to_owned(),
        "created_at".to_owned(),
    ];

    let table_rows = messages
        .iter()
        .map(|msg| TableRow {
            th: (*msg.email).to_string(),
            tds: msg.clone(),
        })
        .collect::<Vec<TableRow>>();

    let fixed_table_data = FixedTableData2 {
        table_headers: table_headers,
        table_rows: table_rows,
    };

    dbg!(&fixed_table_data);

    let body = hb.render("fixed-table", &fixed_table_data).unwrap();
    return HttpResponse::Ok().body(body);
}

#[get("/list/{user_type_id}")]
pub async fn get_users_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
    path: web::Path<i32>,
) -> impl Responder {
    let user_type_id = path.into_inner();
    println!("get_admin_users_handler firing");
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let query_result = sqlx::query_as!(
        AdminUserList,
        "SELECT id, slug, user_type_id, username, email, created_at, avatar_path
        FROM users
        WHERE user_type_id = $1
        ORDER by created_at
        LIMIT $2 OFFSET $3",
        user_type_id,
        limit as i32,
        offset as i32
    )
    .fetch_all(&data.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let error_msg = "Error occurred while fetching all location records";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let users = query_result.unwrap();

    let f_opts = FilterOptions::from(&opts);

    let users_table_data = ResponsiveTableData {
        entity_type_id: entity_type_from_user_type(user_type_id),
        vec_len: users.len(),
        lookup_url: "/admin/list?page=".to_string(),
        opts: f_opts,
        // page: opts.page.unwrap_or(1),
        entities: users,
        subscriptions: test_subs(),
    };

    dbg!(&users_table_data);

    let body = hb.render("responsive-table", &users_table_data).unwrap();
    return HttpResponse::Ok().body(body);
}

#[get("/form/user/{slug}")]
async fn user_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let user_slug = path.into_inner();

    let user_result = sqlx::query_as!(
        AdminUserFormQuery,
        "SELECT username, email, user_type_id, COALESCE(avatar_path, '/images/default_avatar.svg') AS avatar_path, updated_at
        FROM users 
        WHERE slug = $1",
        user_slug
    )
    .fetch_one(&state.db)
    .await;

    if user_result.is_err() {
        let error_msg = "Error occurred while fetching user form";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let user = user_result.unwrap();

    let updated_at = if user.updated_at.is_some() {
        user.updated_at
            .unwrap()
            .format("%b %-d, %-I:%M")
            .to_string()
    } else {
        "Never Updated".to_owned()
    };

    let template_data = AdminUserFormTemplate {
        user_type_options: config::user_type_options(),
        username: user.username,
        email: user.email,
        user_type_id: user.user_type_id,
        updated_at_fmt: updated_at,
        avatar_path: user.avatar_path,
    };

    let body = hb.render("admin/user-form", &template_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

#[get("/form/subadmin/{slug}")]
async fn subadmin_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let user_slug = path.into_inner();

    let user_result = sqlx::query_as!(
        AdminSubadminFormQuery,
        "SELECT users.username, users.email, user_type_id, COALESCE(avatar_path, '/images/default_avatar.svg') AS avatar_path, address_one, address_two, city, state, zip, primary_phone, user_details.updated_at
        FROM users
        INNER JOIN user_details ON user_details.user_id = users.id
        WHERE users.slug = $1",
        user_slug
    )
    .fetch_one(&state.db)
    .await;

    if user_result.is_err() {
        dbg!(&user_result);
        let error_msg = "Error occurred while fetching subadmin form";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let user = user_result.unwrap();

    let updated_at = if user.updated_at.is_some() {
        user.updated_at
            .unwrap()
            .format("%b %-d, %-I:%M")
            .to_string()
    } else {
        "Never Updated".to_owned()
    };

    let template_data = AdminSubadminFormTemplate {
        user_type_options: config::user_type_options(),
        state_options: config::states(),
        username: user.username,
        email: user.email,
        address_one: user.address_one,
        address_two: user.address_two,
        city: user.city,
        state: user.state,
        zip: user.zip,
        primary_phone: user.primary_phone,
        user_type_id: user.user_type_id,
        updated_at_fmt: updated_at,
        avatar_path: user.avatar_path,
    };

    let body = hb.render("admin/subadmin-form", &template_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

#[post("/form/user/{slug}")]
async fn edit_user(
    body: web::Form<AdminUserPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i32>,
    r_state: web::Data<RedisState>,
) -> impl Responder {
    dbg!(&body);
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match redis_validate_and_get_user(cookie, &r_state).await {
            Ok(user) => {
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
                        .header("HX-Retarget", "#subadmin_errors")
                        .body(body);
                } else {
                    let user_id = path.into_inner();
                    match sqlx::query_as::<_, AdminUserPostResponse>(
                        "UPDATE users SET username = $1, email = $2, user_type_id = $3 WHERE slug = $4 RETURNING id",
                    )
                    .bind(&body.username)
                    .bind(&body.email)
                    .bind(&body.user_type_id)
                    .bind(&user_id)
                    .fetch_one(&state.db)
                    .await
                    {
                        Ok(usr) => {
                            dbg!(usr.id);
                            let admin_types = vec![1,2];
                            if admin_types.iter().any(|&i| i == body.user_type_id) {
                                match sqlx::query_as::<_, AdminUserPostResponse>(
                                    "INSERT INTO user_details (user_id) VALUES ($1) RETURNING user_id",
                                )
                                .bind(&usr.id)
                                .fetch_one(&state.db)
                                .await
                                {
                                    Ok(usr) => {
                                        dbg!(usr.id);
                                        // let user_alert = UserAlert {
                                        //     msg: format!("User #{:?} successfully updated & Record inserted in Details.", usr.id),
                                        //     alert_class: "alert_success".to_owned(),
                                        // };
                                        let user_alert = UserAlert::from((format!("User #{:?} successfully updated & Record inserted in Details.", usr.id).as_str(), "alert_success"));
                                        let body = hb.render("admin-home", &user_alert).unwrap();
                                        return HttpResponse::Ok().body(body);
                                    }
                                    Err(err) => {
                                        dbg!(&err);
                                        let user_alert = UserAlert::from((format!("Error updated user DETAILS: {:?}", err).as_str(), "alert_error"));
                                        let body = hb.render("admin-home", &user_alert).unwrap();
                                        return HttpResponse::Ok().body(body);
                                    }
                                }
                            } else {
                                let user_alert = UserAlert::from((format!("User #{:?} successfully updated.", usr.id).as_str(), "alert_success"));
                                let body = hb.render("admin-home", &user_alert).unwrap();
                                return HttpResponse::Ok().body(body);
                            }
                        }
                        Err(err) => {
                            dbg!(&err);
                            let user_alert = UserAlert::from((format!("Error updated user DETAILS (2): {:?}", err).as_str(), "alert_error"));
                            let body = hb.render("admin-home", &user_alert).unwrap();
                            return HttpResponse::Ok().body(body);
                        }
                    }
                }
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("index", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok()
                    .header("HX-Redirect", "/")
                    .body(body);
            }
        }
    } else {
        let message = "Your session seems to have expired. Please login again.".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok()
        .header("HX-Redirect", "/")
        .body(body)
    }
}

// They'll edit regular user, user_type_id -> subadmin. Then go to subadmin list, edit them there to add this data.
#[post("/form/subadmin/{slug}")]
async fn edit_subadmin(
    body: web::Form<AdminSubadminPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
    r_state: web::Data<RedisState>,
) -> impl Responder {
    dbg!(&body);
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match redis_validate_and_get_user(cookie, &r_state).await {
            Ok(user) => {
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
                        .header("HX-Retarget", "#subadmin_errors")
                        .body(body);
                } else {
                    let user_slug = path.into_inner();
                    match sqlx::query_as::<_, AdminUserPostResponse>(
                        "UPDATE user_details 
                            INNER JOIN users ON users.id = user_details.user_id
                            SET address_one = $1, 
                                address_two = $2, 
                                city = $3, 
                                state = $4, 
                                zip = $5, 
                                primary_phone = $6
                            WHERE users.slug = $7
                            RETURNING user_id",
                    )
                    .bind(&body.address_one)
                    .bind(&body.address_two)
                    .bind(&body.city)
                    .bind(&body.state)
                    .bind(&body.zip)
                    .bind(&body.primary_phone)
                    .bind(&user_slug)
                    .fetch_one(&state.db)
                    .await
                    {
                        Ok(usr) => {
                            dbg!(usr.id);
                            let user_alert = UserAlert::from((
                                format!("User #{:?} successfully updated.", usr.id).as_str(),
                                "alert_success",
                            ));
                            let body = hb.render("admin-home", &user_alert).unwrap();
                            return HttpResponse::Ok().body(body);
                        }
                        Err(err) => {
                            dbg!(&err);
                            let user_alert = UserAlert::from((
                                format!("Error updated user DETAILS (3): {:?}", err).as_str(),
                                "alert_error",
                            ));
                            let body = hb.render("admin-home", &user_alert).unwrap();
                            return HttpResponse::Ok().body(body);
                        }
                    }
                }
            }
            Err(err) => {
                dbg!(&err);
                let body = hb.render("index", &format!("{:?}", err)).unwrap();
                return HttpResponse::Ok()
                    .header("HX-Redirect", "/")
                    .body(body);
            }
        }
    } else {
        let message = "Your session seems to have expired. Please login again.".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok()
        .header("HX-Redirect", "/")
        .body(body)
    }
}
