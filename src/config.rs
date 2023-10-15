use mini_markdown::render;
use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use sqlx::FromRow;
use std::fmt::Debug;
use std::fs::File;
use validator::Validate;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub static ref RE_USER_NAME: Regex = Regex::new(r"^[a-zA-Z0-9]{4,}$").unwrap();
    pub static ref RE_SPECIAL_CHAR: Regex = Regex::new("^.*?[@$!%*?&].*$").unwrap();
    pub static ref RE_EMAIL: Regex = Regex::new(r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})").unwrap();
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Post {
    pub slug: String,
    pub title: String,
    pub author: String,
    pub date: String,
    pub body: String,
}

#[derive(Deserialize, Debug)]
pub struct FilterOptions {
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Validate, Serialize, FromRow, Clone, Deserialize)]
pub struct SelectOptions {
    pub value: i32,
    pub key: Option<String>,
}

#[derive(Debug, Validate, Serialize, FromRow, Clone, Deserialize)]
pub struct StringSelectOption {
    pub value: String,
    pub key: Option<String>,
}


#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct ResponseConsultant {
    pub consultant_id: i32,
    pub specialty_name: String,
    pub territory_name: String,
    pub consultant_f_name: String,
    pub consultant_l_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub todo: String,
    pub date: String,
}

impl Post {
    pub fn render(&self) -> String {
        render(&self.body)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub title: String,
    pub description: String,
    pub default: String,
    pub posts: Vec<Post>,
}

impl Config {
    pub fn new() -> Self {
        let file = File::open("./config/blog.yml").expect("Could not open file.");
        let config = serde_yaml::from_reader(file).expect("Could not read values.");
        config
    }
}

pub fn states() -> Vec<StringSelectOption> {
    vec![
        StringSelectOption{key:Some("AL".to_string()),value:"AL".to_string()},
        StringSelectOption{key:Some("AR".to_string()),value:"AK".to_string()},
        StringSelectOption{key:Some("AK".to_string()),value:"AR".to_string()},
        StringSelectOption{key:Some("AZ".to_string()),value:"AZ".to_string()},
    ]
}

// use handlebars::{Handlebars, RenderError, RenderContext, Helper, Context, Renderable, Output};

// const FACTOR_OF_INTEREST_IDX: usize = 0;
// const CANDIDATE_IDX: usize = 1;
// pub fn if_multiple_of_helper<'reg, 'rc>(
//     helper: &Helper<'reg, 'rc>,
//     r: &'reg Handlebars<'reg>,
//     ctx: &'rc Context,
//     rc: &mut RenderContext<'reg, 'rc>,
//     out: &mut dyn Output,) -> Result<(), RenderError> {
//     let factor_of_interest = 
//         helper.param(FACTOR_OF_INTEREST_IDX)
//             .map(|json| json.value())
//             .and_then(|val| val.as_u64())
//             .and_then(|u64_val| if u64_val > 0 { Some(u64_val) } else { None } )
//             .ok_or_else(|| RenderError::new("Factor of interest must be a number greater than 0."))
//     ?;

//     let candidate = 
//         helper.param(CANDIDATE_IDX)
//             .map(|json| json.value())
//             .and_then(|val| val.as_u64())
//             .ok_or_else(|| RenderError::new("Candidate must be a number greater than or equal to 0."))
//     ?;

//     let possible_template = if candidate % factor_of_interest == 0 {
//         helper.template()
//     } else {
//         helper.inverse()
//     };

//     out.write("Hey")?;

//     match possible_template {
//         Some(t) => t.render(r, ctx, rc, out),
//         None => Ok(()),
//     }
// }