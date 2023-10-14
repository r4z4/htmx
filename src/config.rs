use mini_markdown::render;
use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use sqlx::FromRow;
use std::fmt::Debug;
use std::fs::File;
use validator::Validate;

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
