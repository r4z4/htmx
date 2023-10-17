use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse, Responder, Scope,
};

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::{config::{FilterOptions, SelectOption, self, ResponsiveTableData}, models::{admin::{AdminUserList, AdminUserFormTemplate, AdminUserPostRequest}, admin::AdminUserFormQuery}, AppState};

pub fn admin_scope() -> Scope {
    web::scope("/admin")
        // .route("/users", web::get().to(get_users_handler))
        .service(user_form)
        .service(get_users_handler)
}

#[get("/list")]
pub async fn get_users_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
) -> impl Responder {
    println!("get_admin_users_handler firing");
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let query_result = sqlx::query_as!(
        AdminUserList,
        "SELECT user_id, username, email, created_at, avatar_path
        FROM users
        ORDER by created_at
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

#[get("/form/{id}")]
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
        let err = "Error occurred while fetching account option KVs";
        let body = hb.render("validation", &err).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let user = user_result.unwrap();

    let updated_at = if user.updated_at.is_some() {user.updated_at.unwrap().format("%b %-d, %-I:%M").to_string()} else {"Never Updated".to_owned()};

    let template_data = AdminUserFormTemplate {
        user_type_options: config::user_type_options(),
        state_options: config::states(),
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