use actix_web::web::{Data, Form};
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, Scope};
use serde_json::json;
use sqlx::FromRow;

use crate::{
    config::{
        validate_and_get_user,
    },
    AppState,
};
use handlebars::Handlebars;

pub fn service_scope() -> Scope {
    web::scope("/service")
        // .route("/users", web::get().to(get_users_handler))
        .service(home)
    //.service(prev_month)
}

#[get("/")]
async fn home(
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
    state: Data<AppState>,
) -> impl Responder {
    if let Some(cookie) = req.headers().get(actix_web::http::header::COOKIE) {
        match validate_and_get_user(cookie, &state).await {
            Ok(user) => {
                if let Some(usr) = user {
                    // Homepage displays current Mo/Yr
                    let data = json!({ 
                        "success": true
                    });
                    let body = hb.render("service-api", &data).unwrap();
                    HttpResponse::Ok().body(body)
                } else {
                    let message = "Cannot find you";
                    let body = hb.render("index", &message).unwrap();
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
        let message = "Your session seems to have expired. Please login again (3).".to_owned();
        let body = hb.render("index", &message).unwrap();
        HttpResponse::Ok().body(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hbs_helpers::{concat_str_args, int_eq, str_eq},
        models::{model_client::ClientWithDates, model_consult::ConsultFormTemplate},
        test_common::{self, *},
    };
    use serde::{Serialize, Deserialize};
    use test_context::{test_context, TestContext};
    #[derive(Debug, Serialize, Deserialize)]
    pub struct ServiceTemplate {
        username: &'static str,
    }
    #[test_context(Context)]
    #[test]
    fn page_renders(ctx: &mut Context) {
        let template_data = ServiceTemplate {
            username: "Steve",
        };
        let mut hb = Handlebars::new();
        hb.register_templates_directory(".hbs", "./templates")
            .unwrap();
        hb.register_helper("int_eq", Box::new(int_eq));
        hb.register_helper("str_eq", Box::new(str_eq));
        hb.register_helper("concat_str_args", Box::new(concat_str_args));
        let body = hb.render("service-api", &template_data).unwrap();
        let dom = tl::parse(&body, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();

        let element = dom
            .get_element_by_id("service_header")
            .expect("Failed to find element")
            .get(parser)
            .unwrap();

        // Assert
        assert_eq!(element.inner_text(parser), "Consult Services");
        // assert_eq!(1, 1);
    }
}