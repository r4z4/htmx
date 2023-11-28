use chrono::{DateTime, Timelike, Utc};
use linfa::prelude::Predict;
use linfa::{traits::Fit, Dataset};
use linfa_trees::{DecisionTree, SplitQuality};
use ndarray::{array, s, Array2, Axis, ArrayBase, OwnedRepr, Dim};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use std::{fs::File, io::Write};

use crate::scopes::consult::LinfaPredictionInput;
// use csv::{ReaderBuilder, WriterBuilder};
// use ndarray_csv::{Array2Reader, Array2Writer};

// Which consultant should we send?
// What hour should we hold the meeting?
// If what hour - labels will be the hour_of_day and set received_follow_up to 1 (true)

#[derive(Debug, Serialize, FromRow, Deserialize)]
pub struct ModelData {
    consult_purpose_id: i32,
    client_type_id: i32,
    client_id: i32,
    specialty_id: i32,
    territory_id: i32,
    location_id: i32,
    notes: Option<String>,
    consult_result_id: i32,
    num_attendees: i32,
    consult_start: Option<DateTime<Utc>>,
    consult_end: Option<DateTime<Utc>>,
}

impl ModelData {
    pub fn as_f32array(&self) -> [f32; 11] {
        let diff = self.consult_end.unwrap() - self.consult_start.unwrap();
        let duration = diff.num_minutes() as i32;
        let notes_count = if self.notes.is_some() {
            self.notes.as_ref().unwrap().chars().count()
        } else {
            0
        };
        [
            self.consult_purpose_id as f32,
            self.client_id as f32,
            self.client_type_id as f32,
            self.specialty_id as f32,
            self.territory_id as f32,
            self.location_id as f32,
            notes_count as f32,
            duration as f32,
            self.consult_start.unwrap().naive_local().hour() as f32,
            self.consult_result_id as f32,
            self.num_attendees as f32,
            // Predicting consultant_id here
        ]
    }
}

async fn build_model_ndarray(db: &Pool<Postgres>) -> Result<Array2<f32>, String> {
    match sqlx::query_as::<_, ModelData>(
        "SELECT consult_purpose_id, client_type_id, consults.client_id, clients.specialty_id, clients.territory_id, location_id, notes, consult_result_id, num_attendees, consult_start, consult_end
                FROM consults INNER JOIN clients ON consults.client_id = clients.id WHERE consult_end < now()",
    )
    .fetch_all(db)
    // FIXME
    .await
    {
        Ok(model_data) => {
            let built_arr: Array2<f32> = model_data.iter()
                .map(|row| row.as_f32array())
                .collect::<Vec<_>>()
                .into();
            Ok(built_arr)
        },
        Err(e) => Err(format!("Error in DB {}", e).to_string())
    }
}

pub struct FakeRow {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
    pub g: f32,
    pub h: f32,
    pub j: f32,
    pub k: f32,
    pub l: f32,
}

impl LinfaPredictionInput {
    pub fn as_ndarray(&self) -> ArrayBase<OwnedRepr<f32>, Dim<[usize; 2]>> {
        array!([
            self.consult_purpose_id as f32,
            self.client_id as f32,
            self.client_type as f32,
            self.specialty_id as f32,
            self.territory_id as f32,
            self.location_id as f32,
            self.notes_length as f32,
            self.meeting_duration as f32,
            // FIXME: Fix DB Dates (Getting -17888 for diff)
            self.hour_of_day as f32,
            self.received_follow_up as f32,
            self.num_attendees as f32,
        ])
    }
}

pub struct LinfaPredictionResult(pub String, pub i32);

pub async fn linfa_pred(input: &LinfaPredictionInput, pool: &Pool<Postgres>) -> LinfaPredictionResult {
    // Convert input to ndarray

    let input_vec = input.as_ndarray();

    let test_rows: Vec<FakeRow> = vec![
        FakeRow {
            a: 1.,
            b: 6.,
            c: 3.,
            d: 12.,
            e: 13.,
            f: 30.,
            g: 33.,
            h: 1.,
            j: 1.,
            k: 2.,
            l: 1.,
        },
        FakeRow {
            a: 1.,
            b: 1.,
            c: 1.,
            d: 1.,
            e: 9.,
            f: 134.,
            g: 312.,
            h: 2.,
            j: 1.,
            k: 8.,
            l: 5.,
        },
        FakeRow {
            a: 1.,
            b: 1.,
            c: 1.,
            d: 1.,
            e: 9.,
            f: 34.,
            g: 32.,
            h: 2.,
            j: 1.,
            k: 4.,
            l: 5.,
        },
    ];

    // let built_arr: Array2<f32> = test_rows
    //     .iter()
    //     .map(|row| {
    //         [
    //             row.a, row.b, row.c, row.d, row.e, row.f, row.g, row.h, row.j, row.k, row.l,
    //         ]
    //     })
    //     .collect::<Vec<_>>()
    //     .into();

    // dbg!(&built_arr);

    // // Replace with Existing Records of Completed Consults
    // let original_data: Array2<f32> = array!(
    //     [1., 6., 3., 2., 1., 3., 30., 33., 7., 1., 7., 1.],
    //     [1., 3., 1., 3., 2., 8., 122., 44., 8., 1., 2., 2.],
    //     [1., 1., 1., 1., 3., 9., 134., 32., 8., 1., 1., 5.],
    //     [1., 5., 3., 4., 4., 9., 13., 123., 8., 0., 5., 1.],
    //     [1., 2., 2., 4., 2., 10., 135., 54., 10., 1., 3., 7.],
    //     [1., 8., 1., 4., 3., 16., 66., 44., 12., 1., 3., 2.],
    //     [1., 7., 2., 2., 1., 13., 43., 32., 11., 1., 2., 4.],
    //     [1., 4., 1., 3., 4., 11., 15., 24., 13., 0., 2., 6.],
    //     [1., 3., 1., 1., 4., 7., 77., 44., 7., 1., 1., 5.],
    //     [1., 5., 3., 5., 2., 8., 111., 122., 10., 0., 4., 7.],
    //     [1., 12., 5., 5., 5., 16., 131., 122., 11., 0., 3., 4.],
    //     [1., 13., 4., 3., 4., 15., 52., 0., 10., 1., 3., 3.]
    // );

    // dbg!(&original_data);

    let built_arr: Array2<f32> = build_model_ndarray(pool).await.unwrap();
    dbg!(&built_arr);

    let feature_names = vec![
        "consult_purpose_id",
        "client_type",
        "client_id",
        "specialty_id",
        "territory_id",
        "location_id",
        "notes_length",
        "meeting_duration",
        "hour_of_day",
        "received_follow_up",
        "num_attendees",
        "consultant_id",
    ];
    let num_features = built_arr.len_of(Axis(1)) - 1;
    let features = built_arr.slice(s![.., 0..num_features]).to_owned();
    let labels = built_arr.column(num_features).to_owned();

    let linfa_dataset = Dataset::new(features, labels)
        .map_targets(|x| match x.to_owned() as i32 {
            1 => "Hulk Hogan",
            2 => "Mike",
            3 => "Zardos",
            4 => "Greg",
            5 => "Rob",
            6 => "Vanessa",
            7 => "Joe",
            _ => "Nobody",
        })
        .with_feature_names(feature_names);

    let model = DecisionTree::params()
        .split_quality(SplitQuality::Gini)
        .fit(&linfa_dataset)
        .unwrap();

    // Replace with LinfaPredictionInput struct.as_ndarray() for pred
    // Last col as 1 for a received_follow_up. Predict consultant for the positive outcome.
    let test: Array2<f32> = array!(
        [1., 7., 3., 3., 15., 52., 0., 0., 1.],
        [1., 8., 4., 3., 11., 15., 0., 3., 1.],
        [1., 2., 5., 12., 13., 30., 33., 1., 1.]
    );
    let input: Array2<f32> = input.as_ndarray();
    let predictions = model.predict(&input);

    println!("{:?}", predictions);
    // println!("{:?}", test.targets);

    // Map back to int. FIXME
    let pred = predictions[0];
    let consultant_id =
        match pred {
            "Hulk Hogan" => 1,
            "Mike" => 2,
            "Zardos" => 3,
            "Greg" => 4,
            "Rob" => 5,
            "Vanessa" => 6,
            "Joe" => 7,
            _ => 0,
        };

    dbg!(&consultant_id);

    // Create Decision Tree file for each generation for audit/review/records. FIXME: Export to Storage (GCP)
    let path = "./static/linfa/consults/";
    let filename = Uuid::new_v4().to_string();
    let ext = ".tex";
    File::create(format!("{}{}{}", path, filename, ext))
        .unwrap()
        .write_all(model.export_to_tikz().with_legend().to_string().as_bytes())
        .unwrap();

    // return tuple struct (filename_uuid, consultant_id)
    LinfaPredictionResult(filename, consultant_id)
}