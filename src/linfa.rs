use std::{fs::File, io::Write};
use linfa::{Dataset, traits::Fit};
use linfa::prelude::Predict;
use linfa_trees::{DecisionTree, SplitQuality};
use ndarray::{Array2, Axis, array, s};
// use csv::{ReaderBuilder, WriterBuilder};
// use ndarray_csv::{Array2Reader, Array2Writer};

// Which consultant should we send?
// What hour should we hold the meeting?
// If what hour - labels will be the hour_of_day and set received_follow_up to 1 (true)

pub struct LinfaConsultScheduler {
    pub consult_purpose_id: f32,
    pub client_id: f32,
    pub client_type: f32,
    pub location_id: f32,
    pub hour_of_day: f32,
    pub length_of_meeting: f32,
    pub notes_length: f32,
    pub num_attachments: f32,
    // 0 no 1 yes
    pub received_follow_up: f32,
    pub num_attendees: f32,
    pub consultant_id: f32,
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

pub fn linfa_pred() {

    let test_rows: Vec<FakeRow> = vec![FakeRow{a:1.,b:6.,c:3.,d:12.,e:13.,f:30.,g:33.,h:1.,j:1.,k:2.,l:1.}, FakeRow{a:1.,b:1.,c:1.,d:1.,e:9.,f:134.,g:312.,h:2.,j:1.,k:8.,l:5.}, FakeRow{a:1.,b:1.,c:1.,d:1.,e:9.,f:34.,g:32.,h:2.,j:1.,k:4.,l:5.}];

    let built_arr: Array2<f32> = test_rows.iter()
        .map(|row| [row.a, row.b, row.c, row.d, row.e, row.f, row.g, row.h, row.j, row.k, row.l])
        .collect::<Vec<_>>()
        .into();

    dbg!(&built_arr);

    let original_data: Array2<f32> = array!(
        [1.,    6.,    3.,     12.,    13.,     30.,      33.,      1.,     1.,    7.,      1.],
        [1.,    3.,    1.,     3.,     8.,      122.,     44.,      2.,     1.,    2.,      2.],
        [1.,    1.,    1.,     1.,     9.,      34.,      32.,      2.,     1.,    1.,      5.],
        [1.,    5.,    3.,     6.,     9.,      13.,      123.,     1.,     0.,    5.,      1.],
        [1.,    2.,    2.,     6.,     10.,     35.,      744.,     0.,     1.,    3.,      7.],
        [1.,    8.,    1.,     6.,     16.,     66.,      0.,       2.,     1.,    3.,      2.],
        [1.,    7.,    2.,     12.,    13.,     43.,      32.,      1.,     1.,    2.,      4.],
        [1.,    4.,    1.,     3.,     11.,     15.,      0.,       3.,     0.,    2.,      6.],
        [1.,    3.,    1.,     1.,     7.,      77.,      44.,      4.,     1.,    1.,      5.],
        [1.,    5.,    3.,     4.,     8.,      111.,     122.,     0.,     0.,    4.,      7.],
        [1.,    12.,   5.,     4.,     16.,     31.,      522.,     1.,     0.,    3.,      4.],
        [1.,    13.,   4.,     3.,     15.,     52.,      0.,       0.,     1.,    3.,      3.]
    );

    dbg!(&original_data);

    let feature_names = vec!["consult_purpose_id", "client_id", "client_type", "location_id", "hour_of_day", "length_of_meeting", "notes_length", "num_attachments", "received_follow_up", "num_attendees", "consultant_id"];
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

    // Last col as 1 for a received_follow_up. Predict consultant for the positive outcome.
    let test: Array2<f32> = array!(
        [1.,    7.,    3.,    3.,     15.,     52.,      0.,       0.,      1.],
        [1.,    8.,    4.,    3.,     11.,     15.,      0.,       3.,      1.],
        [1.,    2.,    5.,    12.,    13.,     30.,      33.,      1.,      1.]

    );
    let predictions = model.predict(&test);
        
    println!("{:?}", predictions);
    // println!("{:?}", test.targets);
    
    File::create("dt.tex")
        .unwrap()
        .write_all(model.export_to_tikz().with_legend().to_string().as_bytes())
        .unwrap();
}
