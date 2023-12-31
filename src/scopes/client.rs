use actix_web::{get, patch, post, web, HttpRequest, HttpResponse, Responder, Scope};
use redis::{AsyncCommands, RedisResult, RedisError};

use crate::{
    config::{
        self, get_validation_response, subs_from_user, FilterOptions,
        FormErrorResponse, ResponsiveTableData, SelectOption, UserAlert, ValidationErrorMap,
        ValidationResponse, redis_validate_and_get_user, SimpleQuery, SelectOptionsVec, hash_query,
    },
    models::model_client::{
        ClientFormRequest, ClientFormTemplate, ClientList, ClientPostRequest, ClientPostResponse,
    },
    AppState, RedisState, redis_mod::{redis_mod::Ctx, redis_publisher::publish}, scopes::location::FullPageTemplateData,
};
use chrono::NaiveDate;
use handlebars::Handlebars;
use validator::Validate;

pub fn client_scope() -> Scope {
    web::scope("/client")
        // .route("/users", web::get().to(get_users_handler))
        .service(client_form)
        .service(create_client)
        .service(get_clients_handler)
        .service(patch_client)
        .service(client_edit_form)
}

// col names transformed into table headers. use aliases.
#[get("/list")]
pub async fn get_clients_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
    r_state: web::Data<RedisState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match redis_validate_and_get_user(cookie, &r_state).await {
            Ok(user) => {
                println!("get_clients_handler firing");
                let limit = opts.limit.unwrap_or(10);
                let offset = (opts.page.unwrap_or(1) - 1) * limit;

                let query_result = sqlx::query_as!(
                    ClientList,
                    "SELECT 
                        clients.id,
                        clients.client_type_id,
                        slug,
                        specialty_name,
                        COALESCE(client_company_name, CONCAT(client_f_name, ' ', client_l_name)) AS client_name,
                        client_email,
                        client_address_one AS address,
                        client_city,
                        client_zip,
                        client_primary_phone AS phone
                    FROM clients
                    INNER JOIN specialties ON specialties.id = clients.specialty_id
                    ORDER by id
                    LIMIT $1 OFFSET $2",
                    limit as i32,
                    offset as i32
                )
                .fetch_all(&state.db)
                .await;

                dbg!(&query_result);

                if query_result.is_err() {
                    let error_msg = "Error occurred while fetching all client records";
                    let validation_response =
                        ValidationResponse::from((error_msg, "validation_error"));
                    let body = hb.render("validation", &validation_response).unwrap();
                    return HttpResponse::Ok().body(body);
                }

                let clients = query_result.unwrap();

                let f_opts = FilterOptions::from(&opts);

                let clients_table_data = ResponsiveTableData {
                    entity_type_id: 7,
                    vec_len: clients.len(),
                    lookup_url: "/client/list?page=".to_string(),
                    opts: f_opts,
                    // page: opts.page.unwrap_or(1),
                    entities: clients,
                    subscriptions: subs_from_user(&user),
                };

                dbg!(&clients_table_data);

                let body = hb.render("responsive-table", &clients_table_data).unwrap();
                return HttpResponse::Ok().body(body);
            }
            Err(err) => {
                dbg!(&err);
                // FIXME - Display message about session expired
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
        // .append_header(header::ContentType(mime::APPLICATION_JSON))
        .append_header(("HX-Redirect", "/"))
        .body(body)
    }
}

#[get("/form")]
async fn client_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    r_state: web::Data<RedisState>,
    // path: web::Path<i32>,
) -> impl Responder {
    println!("client_form firing");

    let ctx = Ctx::new();
    // let handle = subscribe(&ctx);
    publish(&ctx);

    // let _ = get_n_pages(8).await;

    let account_options = account_options(&state, &r_state).await;
    let template_data = ClientFormTemplate {
        entity: None,
        account_options: account_options.vec,
        specialty_options: config::specialty_options(),
        state_options: config::get_state_options(&state.db).await,
    };

    let body = hb.render("forms/client-form", &template_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

async fn account_options(state: &web::Data<AppState>, r_state: &web::Data<RedisState>) -> SelectOptionsVec {
    let simple_query = SimpleQuery {
        query_str: "SELECT id AS value, account_name AS key 
                        FROM accounts 
                        ORDER by account_name",
        int_args: None,
        str_args: None,
    };
    let query_hash = hash_query(&simple_query);
    // FIXME: Search for key in Redis before reaching for DB
    let mut con = r_state.r_pool.get().await.unwrap();
    let prefix = "query";
    let key = format!("{:?}:{:?}", prefix, query_hash);

    let exists: Result<SelectOptionsVec, RedisError> = con.get(&key).await;

    dbg!(&exists);

    if exists.is_ok() {
        println!("Getting account_options from Redis");
        let result = exists.unwrap();
        return result;
    } else {
        println!("Getting account_options from DB");
        let account_result = sqlx::query_as::<_, SelectOption>(simple_query.query_str)
        .fetch_all(&state.db)
        .await;

        dbg!(&query_hash);
        if account_result.is_err() {
            let err = "Error occurred while fetching account option KVs";
            let default_options = SelectOption {
                key: Some("No accounts Found".to_owned()),
                value: 0,
            };
            // default_options
            dbg!("Incoming Panic");
        }

        let account_options = account_result.unwrap();
        let sov = SelectOptionsVec {
            vec: account_options,
        };
        // Cache the query in Redis -- `query:hash_value => result_hash`
        let val = serde_json::to_string(&sov).unwrap();
        let _: RedisResult<bool> = con.set_ex(key, &val, 86400).await;

        return sov;
    }
}

#[get("/form/{slug}")]
async fn client_edit_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    r_state: web::Data<RedisState>,
    path: web::Path<String>,
) -> impl Responder {
    let loc_slug = path.into_inner();

    let query_result = sqlx::query_as!(
        ClientFormRequest,
        "SELECT client_company_name, client_f_name, client_l_name, slug, client_address_one, client_address_two, client_city, client_state, client_zip, client_email, client_dob, account_id, specialty_id, client_primary_phone
            FROM clients 
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
    let account_options = account_options(&state, &r_state).await;
    let client = query_result.unwrap();

    let template_data = ClientFormTemplate {
        entity: Some(client),
        specialty_options: config::specialty_options(),
        state_options: config::get_state_options(&state.db).await,
        account_options: account_options.vec,
    };

    let body = hb.render("forms/client-form", &template_data).unwrap();
    return HttpResponse::Ok().body(body);
}

#[post("/form")]
async fn create_client(
    body: web::Form<ClientPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
    r_state: web::Data<RedisState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match redis_validate_and_get_user(cookie, &r_state).await {
            Ok(user) => {
                dbg!(&body);
                let is_valid = body.validate();
                if is_valid.is_err() {
                    let validation_response = get_validation_response(is_valid);
                    let body = hb
                        .render("forms/form-validation", &validation_response)
                        .unwrap();
                    return HttpResponse::BadRequest()
                        .header("HX-Retarget", "#client_errors")
                        .body(body);
                } else {
                    let dob_date = if body.client_dob.is_some() {
                        if body.client_dob.as_ref().unwrap().is_empty() {
                            NaiveDate::parse_from_str("1900-01-01", "%Y-%m-%d").unwrap()
                        } else {
                            NaiveDate::parse_from_str(&body.client_dob.as_deref().unwrap(), "%Y-%m-%d").unwrap()
                        }
                    } else {
                        NaiveDate::parse_from_str("1900-01-01", "%Y-%m-%d").unwrap()
                    };

                    match sqlx::query_as::<_, ClientPostResponse>(
                        "INSERT INTO clients (client_f_name, client_l_name, client_company_name, client_address_one, client_address_two, client_city, client_state, client_zip, client_dob, account_id, specialty_id, client_email, client_primary_phone) 
                                VALUES (NULLIF($1, ''), NULLIF($2, ''), NULLIF($3, ''), $4, NULLIF($5, ''), $6, $7, $8, NULLIF($9, '1900-01-01'), $10, $11, $12, $13) RETURNING id",
                    )
                    .bind(&body.client_f_name)
                    .bind(&body.client_l_name)
                    .bind(&body.client_company_name)
                    .bind(&body.client_address_one)
                    .bind(&body.client_address_two)
                    .bind(&body.client_city)
                    .bind(&body.client_state)
                    .bind(&body.client_zip)
                    .bind(dob_date)
                    .bind(&body.account_id)
                    .bind(&body.specialty_id)
                    .bind(&body.client_email)
                    .bind(&body.client_primary_phone)
                    //.bind(Uuid::new_v4().to_string())
                    .fetch_one(&state.db)
                    .await
                    {
                        Ok(loc) => {
                            dbg!(loc.id);
                            // Del / Invalidate Redis Key to force a DB fetch
                            let mut con = r_state.r_pool.get().await.unwrap();
                            let key = format!("{}:{}", "query", "client_options");
                            let deleted: RedisResult<bool> = con.del(&key).await;
                            match deleted {
                                Ok(true) => {
                                    println!("Key deleted");
                                },
                                Ok(false) => {
                                    println!("Key not found {}", &key);
                                },
                                Err(err) => println!("Error: {}", err)
                            }
                            let user_alert = UserAlert::from((format!("Client added successfully: client_id #{:?}", loc.id).as_str(), "alert_success"));
                            let body = hb.render("crud-api-inner", &user_alert).unwrap();
                            return HttpResponse::Ok().body(body);
                        }
                        Err(err) => {
                            dbg!(&err);
                            let user_alert = UserAlert::from((format!("Error adding client: {:?}", err).as_str(), "alert_error"));
                            let body = hb.render("crud-api", &user_alert).unwrap();
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

#[patch("/form/{slug}")]
async fn patch_client(
    body: web::Form<ClientPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
    r_state: web::Data<RedisState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match redis_validate_and_get_user(cookie, &r_state).await {
            Ok(user) => {
                let client_slug = path.into_inner();
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
                        .header("HX-Retarget", "#client_errors")
                        .body(body);
                }else{
                    // Valid input so perform query
                    match sqlx::query_as::<_, ClientPostResponse>(
                        "UPDATE clients 
                            SET client_company_name = $1,
                                client_f_name = $2,
                                client_l_name = $3,
                                client_address_one = $4,
                                client_address_two = $5,
                                client_city = $6,
                                client_state = $7,
                                client_zip = $8,
                                client_primary_phone = $9,
                                client_email = $10,
                                account_id = $11,
                                specialty_id = $12
                            WHERE slug = $13
                            RETURNING id",
                    )
                    .bind(&body.client_company_name)
                    .bind(&body.client_f_name)
                    .bind(&body.client_l_name)
                    .bind(&body.client_address_one)
                    .bind(&body.client_address_two)
                    .bind(&body.client_city)
                    .bind(&body.client_state)
                    .bind(&body.client_zip)
                    .bind(&body.client_primary_phone)
                    .bind(&body.client_email)
                    .bind(&body.account_id)
                    .bind(&body.specialty_id)
                    .bind(client_slug)
                    .fetch_one(&state.db)
                    .await
                    {
                        Ok(client) => {
                            dbg!(client.id);
                            let user_alert = UserAlert::from((
                                format!("Client edited successfully: client_id #{:?}", client.id).as_str(),
                                "alert_success",
                            ));
                            let full_page_data = FullPageTemplateData {
                                user_alert: user_alert.clone(),
                                user: Some(user),
                            };
                            let body = hb.render("list-api", &full_page_data).unwrap();
                            return HttpResponse::Ok().body(body);
                        }
                        Err(err) => {
                            dbg!(&err);
                            let user_alert = UserAlert::from((
                                format!("Error patching location: {:?}", err).as_str(),
                                "alert_error",
                            ));
                            let full_page_data = FullPageTemplateData {
                                user_alert: user_alert.clone(),
                                user: Some(user),
                            };
                            let body = hb.render("list-api", &full_page_data).unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::states,
        hbs_helpers::{concat_str_args, int_eq, str_eq},
        test_common::{self, *},
    };
    use chrono::NaiveDate;
    use test_context::{test_context, TestContext};
    fn mock_opts() -> Vec<SelectOption> {
        vec![
            SelectOption::from((1, Some("One".to_string()))),
            SelectOption::from((2, Some("Two".to_string()))),
        ]
    }

    #[test_context(Context)]
    #[test]
    fn create_form_renders_add_header(ctx: &mut Context) {
        let template_data = ClientFormTemplate {
            entity: None,
            state_options: states(),
            specialty_options: mock_opts(),
            account_options: mock_opts(),
        };
        let mut hb = Handlebars::new();
        hb.register_templates_directory(".hbs", "./templates")
            .unwrap();
        hb.register_helper("int_eq", Box::new(int_eq));
        hb.register_helper("str_eq", Box::new(str_eq));
        let body = hb.render("forms/client-form", &template_data).unwrap();
        // Finishing without error is itself a pass. But can reach into the giant HTML string hb template too.
        let dom = tl::parse(&body, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();

        let element = dom
            .get_element_by_id("client_form_header")
            .expect("Failed to find element")
            .get(parser)
            .unwrap();

        // Assert
        assert_eq!(element.inner_text(parser), "Add Client");

        // Assert
        // assert_eq!(1, 1);
    }

    #[test_context(Context)]
    #[test]
    fn edit_form_renders_edit_header(ctx: &mut Context) {
        let mock_client_with_dates = ClientFormRequest {
            client_company_name: Some("Test Company".to_string()),
            client_f_name: None,
            client_l_name: None,
            client_address_one: "1313 Test St".to_string(),
            client_address_two: None,
            client_city: "Omaha".to_string(),
            client_state: "NE".to_string(),
            client_zip: "68124".to_string(),
            client_dob: Some(NaiveDate::parse_from_str("1980-01-01", "%Y-%m-%d").unwrap()),
            slug: "64779369-4100-47d5-b126-37e6c030dd1d".to_string(),
            client_primary_phone: "555-555-5555".to_string(),
            client_email: "Email@email.com".to_string(),
            account_id: 1,
            specialty_id: 2,
        };
        let template_data = ClientFormTemplate {
            entity: Some(mock_client_with_dates),
            state_options: states(),
            specialty_options: mock_opts(),
            account_options: mock_opts(),
        };
        let mut hb = Handlebars::new();
        hb.register_templates_directory(".hbs", "./templates")
            .unwrap();
        hb.register_helper("int_eq", Box::new(int_eq));
        hb.register_helper("str_eq", Box::new(str_eq));
        hb.register_helper("concat_str_args", Box::new(concat_str_args));
        let body = hb.render("forms/client-form", &template_data).unwrap();
        let dom = tl::parse(&body, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();

        let element = dom
            .get_element_by_id("client_form_header")
            .expect("Failed to find element")
            .get(parser)
            .unwrap();

        // Assert
        assert_eq!(element.inner_text(parser), "Edit Client");
        // assert_eq!(1, 1);
    }
}
