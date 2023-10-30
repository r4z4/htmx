use actix_web::{
    body::{BodySize, MessageBody},
    get, post,
    web::{self, Bytes, Data, Json},
    FromRequest, HttpRequest, HttpResponse, Responder, Scope,
};
use argonautica::{Hasher, Verifier};
use chrono::{DateTime, Duration, Utc};
use handlebars::Handlebars;
use jsonwebtoken::{encode, EncodingKey, Header};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use std::{
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::config::{RE_EMAIL, RE_SPECIAL_CHAR, RE_USER_NAME};
use crate::{config::ValidationResponse, AppState, HeaderValueExt};

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
    user_id: i32,
    username: String,
}
#[derive(FromRow, Serialize, Debug, Clone, Deserialize)]
pub struct ResponseUser {
    pub username: String,
    pub email: String,
    pub user_type_id: i32,
}

// impl MessageBody for ResponseUser {
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
    user_id: i32,
    username: String,
    password: String,
    user_type_id: i32,
    email: String,
}

pub fn auth_scope() -> Scope {
    web::scope("/auth")
        // .route("/users", web::get().to(get_users_handler))
        .service(register_user)
        .service(basic_auth)
        .service(validate_email)
        .service(register_form)
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
        path = "RE_USER_NAME",
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
        "INSERT INTO users (user_id, username, email, password)
        VALUES (DEFAULT, $1, $2, $3)
        RETURNING user_id, username",
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
        "SELECT user_id, username, password, email, user_type_id FROM users WHERE username = $1",
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
                .bind(user.user_id)
                .bind(expires)
                .fetch_one(&state.db)
                .await
                {
                    Ok(session) => {
                        let user = ResponseUser {
                            username: user.username,
                            email: user.email,
                            user_type_id: user.user_type_id,
                        };
                        let template_data = json!({
                            "user": user,
                        });
                        let body = hb.render("homepage", &template_data).unwrap();

                        return HttpResponse::Ok()
                            .header("HX-Redirect", "/homepage")
                            .header("Set-Cookie", cookie)
                            .body(body);
                    }
                    Err(err) => {
                        dbg!(err);
                        let err = "Invalid Login Request".to_owned();
                        let body = hb.render("validation", &err).unwrap();
                        return HttpResponse::Ok().body(body);
                    }
                }
            } else {
                let err = "Invalid Login Request".to_owned();
                let body = hb.render("validation", &err).unwrap();
                return HttpResponse::Ok().body(body);
            }
        }
        Err(err) => {
            // let static_err = "Error occurred while logging in (DB).";
            let body = hb.render("validation", &format!("{:?}", err)).unwrap();
            return HttpResponse::Ok().body(body);
            // HttpResponse::InternalServerError().json(format!("{:?}", err))
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
        let message = "No cookie present at logout".to_owned();
        let body = hb.render("validation", &format!("{:?}", message)).unwrap();
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
                    let error_msg = "Email already taken!".to_owned();
                    let validation_response = ValidationResponse {
                        msg: error_msg,
                        class: "validation_error".to_owned(),
                    };
                    let body = hb.render("validation", &validation_response).unwrap();
                    return HttpResponse::Ok().body(body);
                } else {
                    let success_msg = "Email is available for use".to_owned();
                    let validation_response = ValidationResponse {
                        msg: success_msg,
                        class: "validation_success".to_owned(),
                    };
                    let body = hb.render("validation", &validation_response).unwrap();
                    return HttpResponse::Ok().body(body);
                }
            }
            Err(err) => {
                dbg!(&err);
                let error_msg = "Error occurred in (DB layer).".to_owned();
                let validation_response = ValidationResponse {
                    msg: error_msg,
                    class: "validation_error".to_owned(),
                };
                let body = hb.render("validation", &validation_response).unwrap();
                return HttpResponse::Ok().body(body);
                // HttpResponse::InternalServerError().json(format!("{:?}", err))
            }
        }
    } else {
        let error_msg = "Incorrect Format.".to_owned();
        let validation_response = ValidationResponse {
            msg: error_msg,
            class: "validation_error".to_owned(),
        };
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
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
