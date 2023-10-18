use std::borrow::Borrow;

use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse, Responder, Scope,
};

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::{config::{FilterOptions, SelectOption, self, ResponsiveTableData, ACCEPTED_SECONDARIES, UserAlert, ValidationResponse}, 
    models::{admin::{AdminUserList, AdminSubadminFormQuery, AdminUserFormTemplate, AdminUserPostRequest}, admin::{AdminUserFormQuery, AdminUserPostResponse, AdminSubadminFormTemplate, AdminSubadminPostRequest}}, AppState};

pub fn admin_scope() -> Scope {
    web::scope("/admin")
        // .route("/users", web::get().to(get_users_handler))
        .service(user_form)
        .service(subadmin_form)
        .service(edit_user)
        //.service(edit_subadmin)
        .service(get_users_handler)
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
        "SELECT user_id, user_type_id, username, email, created_at, avatar_path
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
        let err = "Error occurred while fetching all location records";
        // return HttpResponse::InternalServerError()
        //     .json(json!({"status": "error","message": message}));
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let users = query_result.unwrap();

    let users_table_data = ResponsiveTableData {
        table_title: "Users".to_owned(),
        vec_len: users.len(),
        lookup_url: "/admin/list?page=".to_string(),
        page: opts.page.unwrap_or(1),
        entities: users,
    };

    dbg!(&users_table_data);

    let body = hb
        .render("responsive-table", &users_table_data)
        .unwrap();
    return HttpResponse::Ok().body(body);
}

#[get("/form/user/{id}")]
async fn user_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<i32>,
) -> impl Responder {
    let user_id = path.into_inner();

    let user_result = sqlx::query_as!(
        AdminUserFormQuery,
        "SELECT username, email, user_type_id, avatar_path, updated_at
        FROM users 
        WHERE user_id = $1",
        user_id
    )
    .fetch_one(&state.db)
    .await;

    if user_result.is_err() {
        let err = "Error occurred while fetching user form";
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let user = user_result.unwrap();

    let updated_at = if user.updated_at.is_some() {user.updated_at.unwrap().format("%b %-d, %-I:%M").to_string()} else {"Never Updated".to_owned()};

    let template_data = AdminUserFormTemplate {
        user_type_options: config::user_type_options(),
        username: user.username,
        email: user.email,
        user_type_id: user.user_type_id,
        updated_at_fmt: updated_at,
        avatar_path: user.avatar_path,
    };

    let body = hb
        .render("admin/user-form", &template_data)
        .unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

#[get("/form/subadmin/{id}")]
async fn subadmin_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<i32>,
) -> impl Responder {
    let user_id = path.into_inner();

    let user_result = sqlx::query_as!(
        AdminSubadminFormQuery,
        "SELECT users.username, users.email, user_type_id, avatar_path, address_one, address_two, city, state, zip, primary_phone, user_details.updated_at
        FROM users
        INNER JOIN user_details ON user_details.user_id = users.user_id
        WHERE users.user_id = $1",
        user_id
    )
    .fetch_one(&state.db)
    .await;

    if user_result.is_err() {
        let err = "Error occurred while fetching subamin form";
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let user = user_result.unwrap();

    let updated_at = if user.updated_at.is_some() {user.updated_at.unwrap().format("%b %-d, %-I:%M").to_string()} else {"Never Updated".to_owned()};

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

    let body = hb
        .render("admin/subadmin-form", &template_data)
        .unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

fn validate_admin_subadmin_input(body: &AdminSubadminPostRequest) -> bool {
    // Woof
    dbg!(&body);
    if let Some(addr_two) = &body.address_two {
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

fn validate_admin_user_input(body: &AdminUserPostRequest) -> bool {
    // Woof
    dbg!(&body);
    true
}

#[post("/form/user/{id}")]
async fn edit_user(
    body: web::Form<AdminUserPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<i32>,
) -> impl Responder {
    dbg!(&body);

    if validate_admin_user_input(&body) {
        let user_id = path.into_inner();
        match sqlx::query_as::<_, AdminUserPostResponse>(
            "UPDATE user SET username = $1, email = $2, user_type_id = $3 WHERE user_id = $4 RETURNING user_id",
        )
        .bind(&body.username)
        .bind(&body.email)
        .bind(&body.user_type_id)
        .bind(&user_id)
        .fetch_one(&state.db)
        .await
        {
            Ok(usr) => {
                dbg!(usr.user_id);
                let user_alert = UserAlert {
                    msg: format!("User #{:?} successfully updated.", usr.user_id),
                    class: "alert_success".to_owned(),
                };
                let body = hb.render("admin-home", &user_alert).unwrap();
                return HttpResponse::Ok().body(body);
            }
            Err(err) => {
                dbg!(&err);
                let user_alert = UserAlert {
                    msg: format!("Error updated user DETAILS: {:?}", err),
                    class: "alert_error".to_owned(),
                };
                let body = hb.render("admin-home", &user_alert).unwrap();
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
    }
}



#[post("/form/subadmin/{id}")]
async fn edit_subadmin(
    body: web::Form<AdminSubadminPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<i32>,
) -> impl Responder {
    dbg!(&body);

    if validate_admin_subadmin_input(&body) {
        let user_id = path.into_inner();
        match sqlx::query_as::<_, AdminUserPostResponse>(
            "INSERT INTO user_details (user_id, address_one, address_two, city, state, zip, primary_phone) 
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, DEFAULT) RETURNING user_id",
        )
        .bind(&user_id)
        .bind(&body.address_one)
        .bind(&body.address_two)
        .bind(&body.city)
        .bind(&body.state)
        .bind(&body.zip)
        .bind(&body.primary_phone)
        .fetch_one(&state.db)
        .await
        {
            Ok(usr) => {
                dbg!(usr.user_id);
                let user_alert = UserAlert {
                    msg: format!("User #{:?} successfully updated.", usr.user_id),
                    class: "alert_success".to_owned(),
                };
                let body = hb.render("admin-home", &user_alert).unwrap();
                return HttpResponse::Ok().body(body);
            }
            Err(err) => {
                dbg!(&err);
                let user_alert = UserAlert {
                    msg: format!("Error updated user DETAILS: {:?}", err),
                    class: "alert_error".to_owned(),
                };
                let body = hb.render("admin-home", &user_alert).unwrap();
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
    }
}