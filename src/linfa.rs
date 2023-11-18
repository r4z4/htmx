use std::{fs::File, io::Write};
use linfa::{Dataset, traits::Fit};
use linfa::prelude::Predict;
use linfa_trees::{DecisionTree, SplitQuality};
use ndarray::{Array2, Axis, array, s};
// use csv::{ReaderBuilder, WriterBuilder};
// use ndarray_csv::{Array2Reader, Array2Writer};
use std::error::Error;

// Which consultant should we send?
// What hour should we hold the meeting?

pub struct LinfaConsultScheduler {
    pub client_id: f32,
    pub consultant_id: f32,
    pub location_id: f32,
    pub hour_of_day: f32,
    pub length_of_meeting: f32,
    pub notes_length: f32,
    pub num_attachments: f32,
    // 0 no 1 yes
    pub received_follow_up: f32,
}

pub fn linfa_pred() {
    let original_data: Array2<f32> = array!(
        [6.,    1.,     12.,    13.,     30.,      33.,      1.,     1.],
        [3.,    2.,     3.,     8.,      122.,     44.,      2.,     1.],
        [1.,    5.,     1.,     9.,      34.,      32.,      2.,     1.],
        [5.,    1.,     6.,     9.,      13.,      123.,     1.,     0.],
        [2.,    8.,     6.,     10.,     35.,      744.,     0.,     1.],
        [8.,    2.,     6.,     16.,     66.,      0.,       2.,     1.],
        [9.,    4.,     12.,    13.,     43.,      32.,      1.,     1.],
        [4.,    6.,     3.,     11.,     15.,      0.,       3.,     0.],
        [3.,    5.,     1.,     7.,      77.,      44.,      4.,     1.],
        [5.,    7.,     4.,     8.,      111.,     122.,     0.,     0.],
        [12.,   8.,     4.,     16.,     31.,      522.,     1.,     0.],
        [3.,    3.,     3.,     15.,     52.,      0.,       0.,     1.]
    );

    let feature_names = vec!["client_id", "consultant_id", "location_id", "hour_of_day", "length_of_meeting", "notes_length", "num_attachments", "received_follow_up"];
    let num_features = original_data.len_of(Axis(1)) - 1;
    let features = original_data.slice(s![.., 0..num_features]).to_owned();
    let labels = original_data.column(num_features).to_owned();
    
    let linfa_dataset = Dataset::new(features, labels)
        .map_targets(|x| match x.to_owned() as i32 {
            0 => "Bad",
            1 => "Good",
            _ => "None",
        })
        .with_feature_names(feature_names);
    
    let model = DecisionTree::params()
        .split_quality(SplitQuality::Gini)
        .fit(&linfa_dataset)
        .unwrap();

    let test: Array2<f32> = array!(
        [3.,    3.,     3.,     15.,     52.,      0.,       0.]

    );
    let predictions = model.predict(&test);
        
    println!("{:?}", predictions);
    // println!("{:?}", test.targets);
    
    File::create("dt.tex")
        .unwrap()
        .write_all(model.export_to_tikz().with_legend().to_string().as_bytes())
        .unwrap();
}
