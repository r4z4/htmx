use std::{fs, ops::Deref};

use actix_multipart::{
    form::{tempfile::TempFile, MultipartForm},
    Multipart,
};
use actix_web::{
    get,
    http::{header::CONTENT_LENGTH, Error},
    post,
    web::{self, Data},
    HttpRequest, HttpResponse, Responder, Scope,
};
use futures_util::TryStreamExt;
use handlebars::Handlebars;
use image::{imageops::FilterType, DynamicImage};
use mime::{Mime, IMAGE_GIF, IMAGE_JPEG, IMAGE_PNG, IMAGE_SVG};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, FromRow, Row, QueryBuilder, Postgres, Pool};
use std::io::Write;
use uuid::Uuid;

use crate::{
    config::{
        specialty_options, territory_options, FilterOptions, ResponsiveTableData, SelectOption,
        UserAlert, ValidationResponse,
    },
    models::model_consultant::{
        ConsultantFormRequest, ConsultantFormTemplate, ConsultantPostRequest,
        ConsultantPostResponse, ResponseConsultant,
    },
    AppState,
};

pub fn consultant_scope() -> Scope {
    web::scope("/consultant")
        // .route("/users", web::get().to(get_users_handler))
        .service(consultant_form)
        .service(consultant_edit_form)
        .service(get_consultants_handler)
        .service(create_consultant)
        .service(upload)
}
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ResponsiveConsultantData {
    // table_headers: Vec<String>,
    table_title: String,
    entities: Vec<ResponseConsultant>,
}

async fn sort_query(opts: &FilterOptions, pool: &Pool<Postgres>,) -> Result<Vec<ResponseConsultant>, Error> {
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;
    dbg!(&opts);
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT 
        consultant_id,
        slug,
        specialty_name,
        territory_name,
        consultant_f_name,
        consultant_l_name
    FROM consultants
    INNER JOIN specialties ON specialties.specialty_id = consultants.specialty_id
    INNER JOIN territories ON territories.territory_id = consultants.territory_id"
    );

    if let Some(search) = &opts.search {
        query.push(" WHERE consultant_l_name LIKE ");
        query.push(String::from("'%".to_owned() + &opts.search.clone().unwrap() + "%'"));
        query.push(" OR consultant_f_name LIKE ");
        query.push(String::from("'%".to_owned() + &opts.search.clone().unwrap() + "%'"));
    }

    if let Some(sort_key) = &opts.key {
        query.push(" ORDER BY ");
        query.push(String::from(sort_key.to_owned() + " " + &opts.dir.clone().unwrap()));
    } else {
        query.push(" ORDER BY consultants.updated_at DESC, consultants.created_at DESC");
    }

    query.push(" LIMIT ");
    query.push_bind(limit as i32);
    query.push(" OFFSET ");
    query.push_bind(offset as i32);

    let q_build = query.build();
    let res = q_build.fetch_all(pool).await;

    // This almost got me there. Error on .as_str() for consult_start column
    // let consults = res.unwrap().iter().map(|row| row_to_consult_list(row)).collect::<Vec<ConsultList>>();

    let consultants = res.unwrap().iter().map(|row| ResponseConsultant::from_row(row).unwrap()).collect::<Vec<ResponseConsultant>>();

    Ok(consultants)

    // let query_str = query.build().sql().into();
    // dbg!(&query_str);
    // query_str
    // res
}

// Had to remove conflicting FromRow in the derive list
impl<'r> FromRow<'r, PgRow> for ResponseConsultant {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let slug = row.try_get("slug")?;
        let consultant_id = row.try_get("consultant_id")?;
        let specialty_name = row.try_get("specialty_name")?;
        let territory_name = row.try_get("territory_name")?;
        let consultant_f_name = row.try_get("consultant_f_name")?;
        let consultant_l_name = row.try_get("consultant_l_name")?;
        
        Ok(ResponseConsultant{ slug, consultant_id, specialty_name, territory_name, consultant_f_name, consultant_l_name })
    }
}

#[get("/list")]
pub async fn get_consultants_handler(
    opts: web::Query<FilterOptions>,
    hb: web::Data<Handlebars<'_>>,
    data: web::Data<AppState>,
) -> impl Responder {
    println!("get_consultants_handler firing");
    let limit = opts.limit.unwrap_or(10);
    let offset = (opts.page.unwrap_or(1) - 1) * limit;

    let query_result = sort_query(&opts, &data.db).await;

    dbg!(&query_result);

    if query_result.is_err() {
        let error_msg = "Error occurred while fetching all consultant records";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let consultants = query_result.unwrap();

    let consultants_table_data = ResponsiveTableData {
        entity_type_id: 4,
        vec_len: consultants.len(),
        lookup_url: "/consultant/list?page=".to_string(),
        page: opts.page.unwrap_or(1),
        entities: consultants,
    };

    // Only return whole Table if brand new
    if opts.key.is_none() && opts.search.is_none() {
        let body = hb
            .render("responsive-table", &consultants_table_data)
            .unwrap();
        return HttpResponse::Ok().body(body);
    } else {
        let body = hb
            .render("responsive-table-inner", &consultants_table_data)
            .unwrap();
        return HttpResponse::Ok().body(body);
    }

    // if let Some(like) = &opts.search {
    //     let search_sql = format!("%{}%", like);
    //     let query_result = sqlx::query_as!(
    //         ResponseConsultant,
    //         "SELECT 
    //             consultant_id,
    //             slug,
    //             specialty_name,
    //             territory_name,
    //             consultant_f_name,
    //             consultant_l_name
    //         FROM consultants
    //         INNER JOIN specialties ON specialties.specialty_id = consultants.specialty_id
    //         INNER JOIN territories ON territories.territory_id = consultants.territory_id
    //         WHERE consultant_f_name LIKE $3
    //         OR consultant_l_name LIKE $3
    //         ORDER by consultant_id 
    //         LIMIT $1 OFFSET $2",
    //         limit as i32,
    //         offset as i32,
    //         search_sql
    //     )
    //     .fetch_all(&data.db)
    //     .await;

    //     dbg!(&query_result);

    //     if query_result.is_err() {
    //         let err = "Error occurred while fetching all consultant records";
    //         // return HttpResponse::InternalServerError()
    //         //     .json(json!({"status": "error","message": message}));
    //         let body = hb.render("validation", &err).unwrap();
    //         return HttpResponse::Ok().body(body);
    //     }

    //     let consultants = query_result.unwrap();

    //     let consultants_table_data = ResponsiveTableData {
    //         entity_type_id: 4,
    //         vec_len: consultants.len(),
    //         lookup_url: "/consultant/list?page=".to_string(),
    //         page: opts.page.unwrap_or(1),
    //         entities: consultants,
    //     };

    //     let body = hb
    //         .render("responsive-table-inner", &consultants_table_data)
    //         .unwrap();
    //     return HttpResponse::Ok().body(body);
    // } else {
    //     let query_result = sqlx::query_as!(
    //         ResponseConsultant,
    //         "SELECT 
    //             consultant_id,
    //             slug,
    //             specialty_name,
    //             territory_name,
    //             consultant_f_name,
    //             consultant_l_name
    //         FROM consultants
    //         INNER JOIN specialties ON specialties.specialty_id = consultants.specialty_id
    //         INNER JOIN territories ON territories.territory_id = consultants.territory_id
    //         ORDER by consultant_id 
    //         LIMIT $1 OFFSET $2",
    //         limit as i32,
    //         offset as i32
    //     )
    //     .fetch_all(&data.db)
    //     .await;

    //     dbg!(&query_result);

    //     if query_result.is_err() {
    //         let err = "Error occurred while fetching all consultant records";
    //         // return HttpResponse::InternalServerError()
    //         //     .json(json!({"status": "error","message": message}));
    //         let body = hb.render("validation", &err).unwrap();
    //         return HttpResponse::Ok().body(body);
    //     }

    //     let consultants = query_result.unwrap();

    //     let consultants_table_data = ResponsiveTableData {
    //         entity_type_id: 4,
    //         vec_len: consultants.len(),
    //         lookup_url: "/consultant/list?page=".to_string(),
    //         page: opts.page.unwrap_or(1),
    //         entities: consultants,
    //     };

    //     let body = hb
    //         .render("responsive-table", &consultants_table_data)
    //         .unwrap();
    //     return HttpResponse::Ok().body(body);
    // }
}

#[get("/form")]
async fn consultant_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    // path: web::Path<i32>,
) -> impl Responder {
    println!("consultant_form firing");

    let user_result = sqlx::query_as!(
        SelectOption,
        "SELECT user_id AS value, username AS key 
        FROM users
        WHERE user_type_id = 3
        ORDER by user_id DESC"
    )
    .fetch_all(&state.db)
    .await;

    if user_result.is_err() {
        let error_msg = "Error occurred while fetching user option KVs";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let template_data = ConsultantFormTemplate {
        entity: None,
        user_options: Some(user_result.unwrap()),
        territory_options: territory_options(),
        specialty_options: specialty_options(),
    };

    let body = hb.render("forms/consultant-form", &template_data).unwrap();
    dbg!(&body);
    return HttpResponse::Ok().body(body);
}

#[get("/form/{slug}")]
async fn consultant_edit_form(
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let consultant_slug = path.into_inner();

    let query_result = sqlx::query_as!(
        ConsultantFormRequest,
        "SELECT consultant_f_name, consultant_l_name, slug, specialty_id, territory_id, COALESCE(img_path, '/images/consultants/default.svg') as img_path
            FROM consultants 
            WHERE slug = $1",
            consultant_slug
    )
    .fetch_one(&state.db)
    .await;

    dbg!(&query_result);

    if query_result.is_err() {
        let error_msg = "Error occurred while fetching record for consultant form";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);
    }

    let consultant = query_result.unwrap();

    let template_data = ConsultantFormTemplate {
        entity: Some(consultant),
        user_options: None,
        territory_options: territory_options(),
        specialty_options: specialty_options(),
    };

    let body = hb.render("forms/consultant-form", &template_data).unwrap();
    return HttpResponse::Ok().body(body);
}

fn validate_consultant_input(body: &ConsultantPostRequest) -> bool {
    true
}

#[post("/form")]
async fn create_consultant(
    body: web::Form<ConsultantPostRequest>,
    hb: web::Data<Handlebars<'_>>,
    state: web::Data<AppState>,
) -> impl Responder {
    dbg!(&body);

    // let is_valid = body.validate();
    // if is_valid.is_err() {
    //     let mut vec_errs = vec![];
    //     let val_errs = is_valid.err().unwrap().field_errors().iter().map(|x| {
    //         let (key, errs) = x;
    //         vec_errs.push(ValidationErrorMap{key: key.to_string(), errs: errs.to_vec()});
    //     });
    //     // return HttpResponse::InternalServerError().json(format!("{:?}", is_valid.err().unwrap()));
    //     let validation_response = FormErrorResponse {
    //         errors: Some(vec_errs),
    //     };
    //     let body = hb.render("validation", &validation_response).unwrap();
    //     return HttpResponse::BadRequest().body(body);
    // }

    if validate_consultant_input(&body) {
        // Using the NULLIF pattern, so just default to "" & DB will insert it as NULL.
        // If they uploaded we need to trim the input due to Hyperscript padding
        let image_path = if body.img_path.is_some() {
            if body.img_path.as_ref().unwrap().is_empty() {
                "".to_string()
            } else {
                let p = body.img_path.clone().unwrap().trim().to_string();
                p
                // dbg!(&p);
                // let path = &p[2..].to_string();
                // dbg!(&path);
                // path.to_owned()
            }
        } else {
            "".to_string()
        };

        match sqlx::query_as::<_, ConsultantPostResponse>(
            "INSERT INTO consultants (consultant_f_name, consultant_l_name, specialty_id, territory_id, img_path, user_id) 
                    VALUES ($1, $2, $3, $4, NULLIF($5, ''), $6) RETURNING user_id",
        )
        .bind(&body.consultant_f_name)
        .bind(&body.consultant_l_name)
        .bind(&body.specialty_id)
        .bind(&body.territory_id)
        .bind(image_path)
        .bind(&body.user_id)
        .fetch_one(&state.db)
        .await
        {
            Ok(consultant_response) => {
                dbg!(&consultant_response.user_id);
                match sqlx::query_as::<_, ConsultantPostResponse>(
                    "UPDATE users SET user_type_id = 2, updated_at = now() WHERE user_id = $1 RETURNING user_id",
                )
                .bind(&consultant_response.user_id)
                .fetch_one(&state.db)
                .await
                {
                    Ok(update_response) => {
                        let user_alert = UserAlert {
                            msg: format!("Consultant added successfully: ID #{:?}", update_response.user_id),
                            alert_class: "alert_success".to_owned(),
                        };
                        let body = hb.render("crud-api-inner", &user_alert).unwrap();
                        return HttpResponse::Ok().body(body);
                    }
                    Err(err) => {
                        dbg!(&err);
                        let user_alert = UserAlert {
                            msg: format!("Error Updating User After Adding Them As Consultant: {:?}", err),
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
                    msg: format!("Error adding consultant: {:?}", err),
                    alert_class: "alert_error".to_owned(),
                };
                let body = hb.render("crud-api", &user_alert).unwrap();
                return HttpResponse::Ok().body(body);
            }
        }
    } else {
        println!("Val error");
        let error_msg = "Validation error";
        let validation_response = ValidationResponse::from((error_msg, "validation_error"));
        let body = hb.render("validation", &validation_response).unwrap();
        return HttpResponse::Ok().body(body);

        // // To test the alert more easily
        // let user_alert = UserAlert {
        //     msg: "Error adding location:".to_owned(),
        //     alert_class: "alert_error".to_owned(),
        // };
        // let body = hb.render("crud-api", &user_alert).unwrap();
        // return HttpResponse::Ok().body(body);
    }
}

// #[derive(Debug, MultipartForm)]
// struct UploadForm {
//     #[multipart(rename = "file")]
//     // files: Vec<TempFile>,
//     attachment: TempFile,
// }

// async fn save_files(
//     MultipartForm(form): MultipartForm<UploadForm>,
// ) -> Result<impl Responder, Error> {
//     for f in form.files {
//         let path = format!("./tmp/{}", f.file_name.unwrap());
//         // log::info!("saving to {path}");
//         println!("saving to {path}");
//         f.file.persist(path).unwrap();
//     }

//     Ok(HttpResponse::Ok())
// }

#[post("/upload")]
async fn upload(
    mut payload: Multipart,
    hb: web::Data<Handlebars<'_>>,
    req: HttpRequest,
) -> HttpResponse {
    let max_file_size: usize = 20_000;
    let max_file_count: usize = 3;
    let legal_file_types: [Mime; 3] = [IMAGE_GIF, IMAGE_JPEG, IMAGE_PNG];

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
            let dir: &str = "./static/images/consultants/";

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

            let filename = format!("{}{}.png", dir, const_uuid);

            let mut to_save = filename.clone();
            if let Some((_, desired)) = to_save.split_once("./static") {
                to_save = desired.to_owned();
            }
            dbg!(&filename);
            dbg!(&to_save);

            filenames.push(to_save);

            web::block(move || async move {
                let updated_img: DynamicImage = image::open(&destination).unwrap();
                let _ = fs::remove_file(&destination).unwrap();
                let filename = format!("{}{}.png", dir, const_uuid);

                updated_img
                    .resize_exact(200, 200, FilterType::Nearest)
                    .save(filename)
                    .unwrap();
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
    let validation_response = ValidationResponse::from((success_msg.as_str(), "validation_success"));
    let body = hb.render("validation", &validation_response).unwrap();
    return HttpResponse::Ok().body(body);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hbs_helpers::int_eq,
        test_common::{self, *},
    };
    use test_context::{test_context, TestContext};

    #[test_context(Context)]
    #[test]
    fn create_form_does_not_render_image(ctx: &mut Context) {
        let template_data = ConsultantFormTemplate {
            entity: None,
            user_options: None,
            territory_options: territory_options(),
            specialty_options: specialty_options(),
        };
        let mut hb = Handlebars::new();
        hb.register_templates_directory(".hbs", "./templates")
            .unwrap();
        hb.register_helper("int_eq", Box::new(int_eq));
        let body = hb.render("forms/consultant-form", &template_data).unwrap();
        // Finishing without error is itself a pass. But can reach into the giant HTML string hb template too.
        let dom = tl::parse(&body, tl::ParserOptions::default()).unwrap();
        let parser = dom.parser();

        let element = dom
            .get_element_by_id("consultant_form_header")
            .expect("Failed to find element")
            .get(parser)
            .unwrap();

        let img = dom.query_selector("img[id=consultant_img]").unwrap().next();
        // Assert
        assert_eq!(element.inner_text(parser), "Add Consultant");
        assert!(img.is_none());

        // Assert
        // assert_eq!(1, 1);
    }
}
