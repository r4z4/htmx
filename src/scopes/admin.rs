use std::{borrow::Borrow, vec};

use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse, Responder, Scope, HttpRequest,
};

use handlebars::Handlebars;

use crate::{config::{FilterOptions, self, ResponsiveTableData, ACCEPTED_SECONDARIES, UserAlert, ValidationResponse}, 
    models::{model_admin::{AdminUserList, AdminSubadminFormQuery, AdminUserFormTemplate, AdminUserPostRequest}, model_admin::{AdminUserFormQuery, AdminUserPostResponse, AdminSubadminFormTemplate, AdminSubadminPostRequest}}, AppState, ValidatedUser, HeaderValueExt};

pub fn admin_scope() -> Scope {
    web::scope("/admin")
        // .route("/users", web::get().to(get_users_handler))
        .service(user_form)
        .service(subadmin_form)
        .service(edit_user)
        .service(admin_home)
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
async fn admin_home(hb: web::Data<Handlebars<'_>>, req: HttpRequest, state: Data<AppState>) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
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
            Ok(user) => {
                if let Some(usr) = user {
                    let body = hb.render("admin-home", &usr).unwrap();
                    HttpResponse::Ok().body(body)
                } else {
                    let message = "Cannot find you";
                    let body = hb.render("index", &message).unwrap();
                    return HttpResponse::Ok()
                    .body(body);
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
        let message = "Your session seems to have expired. Please login again.".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
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
        "SELECT user_id, slug, user_type_id, username, email, created_at, avatar_path
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
        entity_type_id: entity_type_from_user_type(user_type_id),
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

#[get("/form/user/{slug}")]
async fn user_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let user_slug = path.into_inner();

    let user_result = sqlx::query_as!(
        AdminUserFormQuery,
        "SELECT username, email, user_type_id, avatar_path, updated_at
        FROM users 
        WHERE slug = $1",
        user_slug
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

#[get("/form/subadmin/{slug}")]
async fn subadmin_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let user_slug = path.into_inner();

    let user_result = sqlx::query_as!(
        AdminSubadminFormQuery,
        "SELECT users.username, users.email, user_type_id, avatar_path, address_one, address_two, city, state, zip, primary_phone, user_details.updated_at
        FROM users
        INNER JOIN user_details ON user_details.user_id = users.user_id
        WHERE users.slug = $1",
        user_slug
    )
    .fetch_one(&state.db)
    .await;

    if user_result.is_err() {
        dbg!(&user_result);
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

#[post("/form/user/{slug}")]
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
            "UPDATE users SET username = $1, email = $2, user_type_id = $3 WHERE slug = $4 RETURNING user_id",
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
                let admin_types = vec![1,2];
                if admin_types.iter().any(|&i| i == body.user_type_id) {
                    match sqlx::query_as::<_, AdminUserPostResponse>(
                        "INSERT INTO user_details (user_id) VALUES ($1) RETURNING user_id",
                    )
                    .bind(&usr.user_id)
                    .fetch_one(&state.db)
                    .await
                    {
                        Ok(usr) => {
                            dbg!(usr.user_id);
                            let user_alert = UserAlert {
                                msg: format!("User #{:?} successfully updated & Record inserted in Details.", usr.user_id),
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
                    let user_alert = UserAlert {
                        msg: format!("User #{:?} successfully updated.", usr.user_id),
                        class: "alert_success".to_owned(),
                    };
                    let body = hb.render("admin-home", &user_alert).unwrap();
                    return HttpResponse::Ok().body(body);
                }
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


// They'll edit regular user, user_type_id -> subadmin. Then go to subadmin list, edit them there to add this data.
#[post("/form/subadmin/{slug}")]
async fn edit_subadmin(
    body: web::Form<AdminSubadminPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    dbg!(&body);

    if validate_admin_subadmin_input(&body) {
        let user_slug = path.into_inner();
        match sqlx::query_as::<_, AdminUserPostResponse>(
            "UPDATE user_details 
                INNER JOIN users ON users.user_id = user_details.user_id
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