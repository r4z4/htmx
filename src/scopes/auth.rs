use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpRequest, HttpResponse, Responder, Scope,
};
use argonautica::{Hasher, Verifier};
use chrono::{DateTime, Duration, Utc};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::{ops::Deref, sync::Arc};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::{config::ValidationResponse, AppState, HeaderValueExt};
use crate::{
    config::{
        get_ip, send_email, user_feed, SendEmailInput, RE_EMAIL, RE_SPECIAL_CHAR, RE_USERNAME,
    },
    HomepageTemplate, ValidatedUser,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginResponse {
    pub username: String,
    pub cookie: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginError {
    pub error: String,
}

pub struct CryptoService {
    pub key: Arc<String>,
}
#[derive(FromRow, Serialize, Deserialize)]
pub struct UserNoPassword {
    id: i32,
    username: String,
}

// impl MessageBody for ValidatedUser {
//     type Error = Infallible;

//     fn size(&self) -> BodySize {
//         BodySize::Sized((self.username.len() + self.email.len()) as u64)
//     }

//     fn poll_next(
//         self: Pin<&mut Self>,
//         _cx: &mut Context<'_>,
//     ) -> Poll<Option<Result<Bytes, Self::Error>>> {
//         let payload_string = self.username.clone() + &self.email;
//         let payload_bytes = Bytes::from(payload_string);
//         Poll::Ready(Some(Ok(payload_bytes)))
//     }
// }

#[derive(Serialize, Deserialize)]
pub struct LoginUser {
    username: String,
    password: String,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct AuthUser {
    id: i32,
    username: String,
    password: String,
    user_type_id: i32,
    list_view: String,
    email: String,
    user_subs: Vec<i32>,
    client_subs: Vec<i32>,
    consult_subs: Vec<i32>,
    location_subs: Vec<i32>,
    consultant_subs: Vec<i32>,
}

pub fn auth_scope() -> Scope {
    web::scope("/auth")
        // .route("/users", web::get().to(get_users_handler))
        .service(register_user)
        .service(basic_auth)
        .service(validate_email)
        .service(register_form)
        .service(forgot_password_form)
        .service(forgot_password)
        .service(reset_password_form)
        .service(reset_password)
        .service(logout)
}

fn validate_password(password: &str) -> Result<(), ValidationError> {
    let mut has_whitespace = false;
    let mut has_upper = false;
    let mut has_lower = false;
    let mut has_digit = false;

    for c in password.chars() {
        has_whitespace |= c.is_whitespace();
        has_lower |= c.is_lowercase();
        has_upper |= c.is_uppercase();
        has_digit |= c.is_digit(10);
    }
    if !has_whitespace && has_upper && has_lower && has_digit && password.len() >= 8 {
        Ok(())
    } else {
        return Err(ValidationError::new("Password Validation Failed"));
    }
}

#[derive(Debug, Validate, Serialize, Deserialize)]
pub struct CreateUserBody {
    #[validate(regex(
        path = "RE_USERNAME",
        message = "Username must contain number & alphabets only & must be 6 characters long"
    ))]
    username: String,
    #[validate(length(min = 3, message = "Email must be greater than 3 chars"))]
    email: String,
    #[validate(
        custom(
            function = "validate_password",
            message = "Must Contain At Least One Upper Case, Lower Case and Number. No spaces."
        ),
        regex(
            path = "RE_SPECIAL_CHAR",
            message = "Must Contain At Least One Special Character"
        )
    )]
    password: String,
}

#[post("/register")]
async fn register_user(
    state: Data<AppState>,
    body: Json<CreateUserBody>,
    hb: web::Data<Handlebars<'_>>,
) -> impl Responder {
    let is_valid = body.validate();
    if is_valid.is_err() {
        return HttpResponse::InternalServerError().json(format!("{:?}", is_valid.err().unwrap()));
    }
    let _ = dbg!(is_valid);
    let user: CreateUserBody = body.into_inner();
    let hash_secret = std::env::var("HASH_SECRET").unwrap_or("Ugh".to_owned());
    let mut hasher = Hasher::default();
    let hash = hasher
        .with_password(user.password)
        .with_secret_key(hash_secret)
        .hash()
        .unwrap();

    match sqlx::query_as::<_, UserNoPassword>(
        "INSERT INTO users (id, username, email, password)
        VALUES (DEFAULT, $1, $2, $3)
        RETURNING id, username",
    )
    .bind(user.username)
    .bind(user.email)
    .bind(hash)
    .fetch_one(&state.db)
    .await
    {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(err) => HttpResponse::InternalServerError().json(format!("{:?}", err)),
    }
}

#[derive(Debug, Validate, FromRow, Serialize, Deserialize)]
pub struct SessionUpdate {
    // user_id: i32,
    session_id: String,
    // expires: DateTime<Utc>,
}

#[post("/login")]
async fn basic_auth(
    state: Data<AppState>,
    body: web::Form<LoginRequest>,
    hb: web::Data<Handlebars<'_>>,
) -> impl Responder {
    // let jwt_secret: Hmac<Sha256> = Hmac::new_from_slice(
    //     std::env::var("JWT_SECRET")
    //         .expect("JWT_SECRET must be set")
    //         .as_bytes()
    // ).unwrap();
    let secret = std::env::var("JWT_SECRET").unwrap();
    let username = &body.username;
    let password = &body.password;

    match sqlx::query_as::<_, AuthUser>(
        "SELECT users.id, username, password, email, user_type_id, user_subs, client_subs, consult_subs, location_subs, consultant_subs, user_settings.list_view
        FROM users 
        INNER JOIN user_settings ON user_settings.user_id = users.id
        WHERE username = $1",
    )
    .bind(username.to_string())
    .fetch_one(&state.db)
    .await
    {
        Ok(user) => {
            let hash_secret = std::env::var("HASH_SECRET").unwrap();
            // Build the verifier
            let mut verifier = Verifier::default();
            let is_valid = verifier
                .with_hash(user.password)
                .with_password(password)
                .with_secret_key(hash_secret)
                .verify()
                .unwrap();

            if is_valid {
                let cookie_token = Uuid::new_v4().to_string();
                let cookie = format!("{}; Path=/; HttpOnly; Max-Age=1209600", cookie_token);
                // FIXME: Sync these expires
                let expires = Utc::now() + Duration::days(137);
                match sqlx::query_as::<_, SessionUpdate>(
                    "INSERT INTO user_sessions (user_session_id, session_id, user_id, expires)
                    VALUES (DEFAULT, $1, $2, $3)
                    RETURNING session_id",
                )
                .bind(cookie_token)
                .bind(user.id)
                .bind(expires)
                .fetch_one(&state.db)
                .await
                {
                    Ok(session) => {
                        let user = ValidatedUser {
                            username: user.username,
                            email: user.email,
                            user_type_id: user.user_type_id,
                            list_view: user.list_view,
                            user_subs: user.user_subs,
                            client_subs: user.client_subs,
                            consult_subs: user.consult_subs,
                            location_subs: user.location_subs,
                            consultant_subs: user.consultant_subs,
                        };
                        let feed_data = user_feed(&user, &state.db).await;
                        let template_data = HomepageTemplate {
                            err: None,
                            user: Some(user),
                            feed_data: feed_data,
                        };
                        let body = hb.render("homepage", &template_data).unwrap();

                        return HttpResponse::Ok()
                            .header("HX-Redirect", "/homepage")
                            .header("Set-Cookie", cookie)
                            .body(body);
                    }
                    Err(err) => {
                        dbg!(&err);
                        let error_msg = "Invalid Login Request".to_owned() + format!("{}", err).as_str();
                        let validation_response = ValidationResponse::from((error_msg.as_str(), "validation_error"));
                        let body = hb.render("validation", &validation_response).unwrap();
                        return HttpResponse::Ok().body(body);
                    }
                }
            } else {
                let error_msg = "Invalid Login Request";
                let validation_response = ValidationResponse::from((error_msg, "validation_error"));
                let body = hb.render("validation", &validation_response).unwrap();
                return HttpResponse::Ok().body(body);
            }
        }
        Err(err) => {
            let validation_response = ValidationResponse::from((format!("{:?}", err).as_str(), "validation_error"));
            let body = hb.render("validation", &validation_response).unwrap();
            return HttpResponse::Ok().body(body);
        }
    }
}

fn decode_and_login(body: LoginRequest) -> Result<LoginResponse, LoginError> {
    if body.username.len() > 1 {
        Ok(LoginResponse {
            username: body.username,
            cookie: "cooke".to_owned(),
        })
    } else {
        Err(LoginError {
            error: "Error".to_owned(),
        })
    }
}

#[derive(Debug, Validate, FromRow, Serialize, Deserialize)]
pub struct LogoutResult {
    expires: DateTime<Utc>,
}

#[get("/register")]
async fn register_form(
    state: Data<AppState>,
    req: HttpRequest,
    hb: web::Data<Handlebars<'_>>,
) -> impl Responder {
    let message = "No cookie present at logout".to_owned();
    let body = hb
        .render("forms/register-form", &format!("{:?}", message))
        .unwrap();
    return HttpResponse::Ok().body(body);
}

#[get("/logout")]
async fn logout(
    state: Data<AppState>,
    req: HttpRequest,
    hb: web::Data<Handlebars<'_>>,
) -> impl Responder {
    let headers = req.headers();
    if let Some(cookie) = headers.get(actix_web::http::header::COOKIE) {
        // Do I need to alter DB at all?
        match sqlx::query_as::<_, LogoutResult>(
            "UPDATE user_sessions SET expires = NOW(), updated_at = NOW(), logout = TRUE WHERE session_id = $1 RETURNING expires",
        )
        .bind(cookie.to_string())
        .fetch_one(&state.db)
        .await
        {
            Ok(expires) => {
                dbg!(&expires);
                let body = hb.render("index", &expires).unwrap();
                return HttpResponse::Ok()
                .header("HX-Redirect", "/")
                .header("Set-Cookie", "")
                .body(body);
            }
            Err(err) => {
                dbg!(&err);
                // let static_err = "Error occurred while logging in (DB).";
                let body = hb.render("index", &format!("{:?}", err)).unwrap();
                // Notify someone
                return HttpResponse::Ok().body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
    } else {
        let error_msg = "No cookie present at logout";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }
}

#[derive(Deserialize, FromRow, Validate)]
struct EmailParam {
    email: String,
}

#[derive(Deserialize, FromRow, Validate)]
pub struct QueryBool {
    exists: bool,
}

pub fn validate_email_fmt(email: String) -> bool {
    if RE_EMAIL.is_match(&email) {
        true
    } else {
        false
    }
}

#[get("/validate/email")]
async fn validate_email(
    state: Data<AppState>,
    param: web::Query<EmailParam>,
    req: HttpRequest,
    hb: web::Data<Handlebars<'_>>,
) -> impl Responder {
    dbg!(req);
    let submitted_email = &param.email;
    if validate_email_fmt(submitted_email.to_owned()) {
        match sqlx::query_as::<_, QueryBool>("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
            .bind(submitted_email.to_string())
            .fetch_one(&state.db)
            .await
        {
            Ok(result) => {
                if result.exists {
                    let error_msg = "Email already taken!";
                    let validation_response =
                        ValidationResponse::from((error_msg, "validation_error"));
                    let body = hb.render("validation", &validation_response).unwrap();
                    return HttpResponse::Ok().body(body);
                } else {
                    let success_msg = "Email is available for use";
                    let validation_response =
                        ValidationResponse::from((success_msg, "validation_success"));
                    let body = hb.render("validation", &validation_response).unwrap();
                    return HttpResponse::Ok().body(body);
                }
            }
            Err(err) => {
                dbg!(&err);
                let error_msg = "Error occurred in (DB layer).";
                let validation_response = ValidationResponse::from((error_msg, "validation_error"));
                let body = hb.render("validation", &validation_response).unwrap();
                return HttpResponse::Ok().body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
    } else {
        let error_msg = "Incorrect Format.";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }
}

#[derive(Deserialize, FromRow, Validate)]
pub struct ResetPasswordBody {
    username: String,
    password: String,
    #[validate(must_match = "password")]
    re_password: String,
}

#[get("/forgot-password")]
async fn forgot_password_form(
    state: Data<AppState>,
    req: HttpRequest,
    hb: web::Data<Handlebars<'_>>,
) -> impl Responder {
    let message = "No cookie present at logout".to_owned();
    let body = hb
        .render("forms/forgot-password-form", &format!("{:?}", message))
        .unwrap();
    return HttpResponse::Ok().body(body);
}

#[derive(Deserialize, FromRow, Validate)]
pub struct ForgotPasswordBody {
    email: String,
}

#[derive(Deserialize, FromRow, Validate)]
pub struct ForgotPasswordResponse {
    created_at: DateTime<Utc>,
}

#[post("/forgot-password")]
async fn forgot_password(
    state: Data<AppState>,
    req: HttpRequest,
    body: web::Form<ForgotPasswordBody>,
    hb: web::Data<Handlebars<'_>>,
) -> impl Responder {
    println!("in forgot pass");
    let ip_addr = get_ip(req);
    let is_valid = body.validate();
    if is_valid.is_err() {
        println!("validation_err");
        return HttpResponse::InternalServerError().json(format!("{:?}", is_valid.err().unwrap()));
    }
    let _ = dbg!(is_valid);

    match sqlx::query_as::<_, ForgotPasswordResponse>(
        "INSERT INTO reset_password_requests (request_id, user_id, req_ip, created_at)
        VALUES (DEFAULT, (SELECT user_id FROM users WHERE email = $1), $2, now())
        RETURNING created_at",
    )
    .bind(body.email.deref())
    .bind(ip_addr.to_string())
    .fetch_one(&state.db)
    .await
    {
        Ok(resp) => {
            let created_at_fmt = resp.created_at.format("%b %-d, %-I:%M").to_string();
            let email_input = SendEmailInput::from((
                body.email.as_str(),
                format!("A password reset was requested on {}", created_at_fmt).as_str(),
            ));
            match send_email(email_input).await {
                Ok(_) => {
                    let success_msg = "Reset Password link has been sent.";
                    let validation_response =
                        ValidationResponse::from((success_msg, "validation_success"));
                    let body = hb.render("validation", &validation_response).unwrap();
                    return HttpResponse::Ok().body(body);
                }
                Err(e) => {
                    let error_msg = "Unable to send Reset Password link. Please ensure you have entered a valid email address.";
                    let validation_response =
                        ValidationResponse::from((error_msg, "validation_error"));
                    let body = hb.render("validation", &validation_response).unwrap();
                    return HttpResponse::Ok().body(body);
                }
            }
        }
        Err(err) => {
            let error_msg = format!("Error at the DB layer. {}", err);
            let validation_response =
                ValidationResponse::from((error_msg.as_str(), "validation_error"));
            let body = hb.render("validation", &validation_response).unwrap();
            return HttpResponse::Ok().body(body);
        }
    }
}

#[get("/reset-password")]
async fn reset_password_form(
    state: Data<AppState>,
    req: HttpRequest,
    hb: web::Data<Handlebars<'_>>,
) -> impl Responder {
    let message = "No cookie present at logout".to_owned();
    let body = hb
        .render("forms/reset-password-form", &format!("{:?}", message))
        .unwrap();
    return HttpResponse::Ok().body(body);
}

#[post("/reset-password")]
async fn reset_password(
    state: Data<AppState>,
    param: web::Query<EmailParam>,
    req: HttpRequest,
    body: Json<ResetPasswordBody>,
    hb: web::Data<Handlebars<'_>>,
) -> impl Responder {
    let is_valid = body.validate();
    if is_valid.is_err() {
        return HttpResponse::InternalServerError().json(format!("{:?}", is_valid.err().unwrap()));
    }
    let _ = dbg!(is_valid);
    let user: ResetPasswordBody = body.into_inner();
    let hash_secret = std::env::var("HASH_SECRET").unwrap_or("Ugh".to_owned());
    let mut hasher = Hasher::default();
    let hash = hasher
        .with_password(user.password)
        .with_secret_key(hash_secret)
        .hash()
        .unwrap();

    match sqlx::query_as::<_, UserNoPassword>(
        "UPDATE users SET password = $1, updated_at = now()
        WHERE username = $3
        RETURNING id, username",
    )
    .bind(hash)
    .bind(user.username)
    .fetch_one(&state.db)
    .await
    {
        Ok(user) => {
            let message =
                "Your Password has been reset. You may now login using these credentials."
                    .to_owned();
            let body = hb
                .render("forms/reset-password-form", &format!("{:?}", message))
                .unwrap();
            return HttpResponse::Ok().body(body);
        }
        Err(err) => {
            let message = "Unable to Reset Password. Please contact site administrator.".to_owned();
            let body = hb
                .render("forms/reset-password-form", &format!("{:?}", message))
                .unwrap();
            return HttpResponse::Ok().body(body);
        }
    }
}
// email_regex.is_match(email_address)
// match sqlx::query_as::<_, LogoutResult>(
//     "UPDATE user_sessions SET expires = NOW(), updated_at = NOW(), logout = TRUE WHERE session_id = $1 RETURNING expires",
// )
// .bind(cookie.to_string())
// .fetch_one(&state.db)
// .await
// {
//     Ok(expires) => {
//         dbg!(&expires);
//         let body = hb.render("index", &expires).unwrap();
//         return HttpResponse::Ok()
//         .header("HX-Redirect", "/")
//         .header("Set-Cookie", "")
//         .body(body);
//     }
//     Err(err) => {
//         dbg!(&err);
//         // let static_err = "Error occurred while logging in (DB).";
//         let body = hb.render("index", &format!("{:?}", err)).unwrap();
//         // Notify someone
//         return HttpResponse::Ok().body(body);
//         // HttpResponse::InternalServerError().json(format!("{:?}", err))
//     }
// }

// #[post("/login")]
// async fn login_user(
//     body: web::Form<LoginRequest>,
//     hb: web::Data<Handlebars<'_>>,
//     // data: web::Data<AppState>,
// ) -> impl Responder {
//     // let query_result = sqlx::query_as!(
//     //     EngagementModel,
//     //     "INSERT INTO engagements (text,rating) VALUES ($1, $2) RETURNING *",
//     //     body.text.to_string(),
//     //     body.rating,
//     // )
//     // .fetch_one(&data.db)
//     // .await;
//     let body_clone = body.clone();
//     dbg!(&body_clone);
//     let user = decode_and_login(body_clone);

//     let cookie_token = Uuid::new_v4().to_string();
//     let cookie = format!("token={}; Path=/; HttpOnly; Max-Age=1209600", cookie_token);

//     match user {
//         Ok(user) => {
//             let body = hb.render("homepage", &user).unwrap();
//             return HttpResponse::Ok()
//             .header("HX-Redirect", "/homepage")
//             .header("Set-Cookie", cookie)
//             .body(body);
//         }
//         Err(err) => {
//             let body = hb.render("validation", &err).unwrap();
//             return HttpResponse::Ok().body(body);
//         }
//     }
// }
