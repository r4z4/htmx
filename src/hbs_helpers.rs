use convert_case::{Case, Casing};
use handlebars::handlebars_helper;
use serde::{Deserialize, Serialize};

use crate::{
    config::UserSubscriptions, models::model_location::LocationList, scopes::event::CalendarData,
    Entity,
};

handlebars_helper!(to_title_case: |s: String| {
    // FIXME Always just set as aliases on server? Or do I like explicit data fields?
    match s.as_str() {
        "client_type_id" => "Type".to_string(),
        "consult_start" => "Start".to_string(),
        "consult_end" => "End".to_string(),
        "consultant_name" => "Consultant".to_string(),
        "location_name" => "Location".to_string(),
        "location_phone" => "Phone".to_string(),
        "location_address_one" => "Addr".to_string(),
        "location_address_two" => "Addr 2".to_string(),
        "specialty_name" => "Specialty".to_string(),
        "client_email" => "Email".to_string(),
        "client_city" | "location_city" => "City".to_string(),
        "client_name" => "Name".to_string(),
        "client_zip" => "Zip".to_string(),
        _ => s.to_case(Case::Title),
    }
});

handlebars_helper!(str_eq: |s_1: String, s_2: String| {
    if s_1 == s_2 {
        true
    } else {
        false
    }
});

handlebars_helper!(preview_text: |str: String| {
    let prev = &str[0..50];
    prev
});

handlebars_helper!(form_rte: |slug: String, entity_type_id: i32| {
    match entity_type_id {
        1 => String::from("form/user/") + &slug,
        // Relative to /admin for admin
        2 => String::from("form/subadmin/") + &slug,
        // Admin & Subadmin same form (for now)
        3 => String::from("form/subadmin/") + &slug,
        4 => String::from("consultant/form/") + &slug,
        5 => String::from("location/form/") + &slug,
        6 => String::from("consult/form/") + &slug,
        7 => String::from("client/form/") + &slug,
        _ => String::from("form/user/") + &slug
    }
});

handlebars_helper!(sort_rte: |key: String, entity_type_id: i32, dir: String| {
    match entity_type_id {
        1 => String::from("/admin/list?key=") + &key + "&dir=" + &dir,
        // Relative to /admin for admin
        2 => String::from("subadmin/list?key=") + &key + "&dir=" + &dir,
        // Admin & Subadmin same form (for now)
        3 => String::from("subadmin/list?key=") + &key + "&dir=" + &dir,
        4 => String::from("consultant/list?key=") + &key + "&dir=" + &dir,
        5 => String::from("location/list?key=") + &key + "&dir=" + &dir,
        6 => String::from("consult/list?key=") + &key + "&dir=" + &dir,
        7 => String::from("client/list?key=") + &key + "&dir=" + &dir,
        _ => String::from("form/user/") + &key + "&dir=" + &dir
    }
});

handlebars_helper!(attachments_rte: |slug: String, entity_type_id: i32| {
    match entity_type_id {
        1 => String::from("/user/attachments/") + &slug,
        // Relative to /admin for admin
        2 => String::from("/user/attachments/") + &slug,
        // Admin & Subadmin same form (for now)
        3 => String::from("/user/attachments/") + &slug,
        4 => String::from("consultant/attachments/") + &slug,
        5 => String::from("location/attachments/") + &slug,
        6 => String::from("consult/attachments/") + &slug,
        7 => String::from("client/attachments/") + &slug,
        _ => String::from("form/attachments/") + &slug
    }
});

handlebars_helper!(subscribe_rte: |slug: String, entity_type_id: i32| {
    String::from("/user/subscribe/") + &entity_type_id.to_string().as_str() + "/" + &slug
});

handlebars_helper!(subscribe_icon: |id: i32, entity_type_id: i32, subs: UserSubscriptions| {
    dbg!(&id);
    dbg!(&subs);
    // No icon for Query entity type
    if entity_type_id == 8 {
        ""
    } else {
        let subscribed =
            match entity_type_id {
                1 | 2 | 3 => subs.user_subs.contains(&id),
                4 => subs.consultant_subs.contains(&id),
                5 => subs.location_subs.contains(&id),
                6 => subs.consult_subs.contains(&id),
                7 => subs.client_subs.contains(&id),
                _ => subs.user_subs.contains(&id),
            };
        if subscribed {
            "🔕"
        } else {
            "🔔"
        }
    }
});

handlebars_helper!(is_icon_col: |key: String| {
    if vec!["purpose".to_string(), "result".to_string(), "client_type_id".to_string()].contains(&key) {
        true
    } else {
        false
    }
});

handlebars_helper!(get_icon: |key: String, id: i32| {
    if key == "result" {
        match id {
            0 => "∅",
            1 => "🟢📅",
            2 => "🟢",
            3 => "🟡📅",
            4 => "🟡",
            5 => "🔴📅",
            6 => "🔴",
            _ => "🤷",
        }
    } else if key == "purpose" {
        match id {
            1 => "ℹ",
            2 => "🔼",
            3 => "🔄",
            4 => "🔽",
            5 => "💬",
            _ => "🤷",
        }
    } else if key == "client_type_id" {
        match id {
            1 => "👤",
            2 => "👥",
            3 => "🏢",
            4 => "🌎",
            5 => "🏛",
            _ => "🤷",
        }
    } else {
        "🤷"
    }
});

handlebars_helper!(get_list_view: |list_view: String| {
    match list_view.as_str() {
        "consult" => String::from("/consult/list"),
        "consultant" => String::from("/consultant/list"),
        "location" => String::from("/location/list"),
        "client" => String::from("/client/list"),
        _ => String::from("/consult/list")
    }
});

handlebars_helper!(int_eq: |int_1: usize, int_2: usize| {
    println!("int_eq firing w/ {} & {}", int_1, int_2);
    if int_1 == int_2 {
        true
    } else {
        false
    }
});

handlebars_helper!(int_in: |int: usize, vec: Vec<usize>| {
    println!("int_in firing w/ {} & {:?}", int, vec);
    if vec.iter().any(|v| v == &int) {
        true
    } else {
        false
    }
});

handlebars_helper!(lower_and_single: |plural: String| {
    let mut m_plural = plural;
    m_plural.pop();
    m_plural.to_case(Case::Lower)
});
handlebars_helper!(concat_args: |lookup_url: String, page_num: i32| {
    let added = page_num + 1;
    lookup_url.to_owned() + &added.to_string()
});

handlebars_helper!(concat_str_args: |url: String, slug: String| {
    url + &slug
});

handlebars_helper!(loc_vec_len_ten: |vec: Vec<LocationList>| {
    if vec.len() == 10 {
        true
    } else {
        false
    }
});

handlebars_helper!(get_search_rte: |entity_type_id: i32| {
    match entity_type_id {
        1 => String::from("/users/search"),
        2 => String::from("admin/search"),
        3 => String::from("/users/search"),
        4 => String::from("/consultant/list"),
        5 => String::from("/location/list"),
        6 => String::from("/consult/list"),
        7 => String::from("/client/search"),
        _ => String::from("/users/search"),
    }
});

handlebars_helper!(get_table_title: |entity_type_id: i32| {
    match entity_type_id {
        1 => String::from("Users"),
        2 => String::from("Admins"),
        3 => String::from("Subadmins"),
        4 => String::from("Consultants"),
        5 => String::from("Locations"),
        6 => String::from("Consults"),
        7 => String::from("Clients"),
        8 => String::from("Query"),
        _ => String::from("Unknown Entity"),
    }
});

// Calendar Helpers

handlebars_helper!(first_week: |day: u32, cal_data: CalendarData| {
    // n - first_d_o_m + 1 (Thurs = 5. Tues = 3.)
    // i32 for negative
    let offset_int: i32 = day as i32 - cal_data.first_day_of_month as i32 + 1 as i32;
    if offset_int > 0 {
        offset_int.to_string()
    } else {
        "".to_string()
    }
});

handlebars_helper!(second_week: |day: u32, cal_data: CalendarData| {
    // + 1 + 7
    let offset_int: i32 = day as i32 - cal_data.first_day_of_month as i32 + 8 as i32;
    offset_int.to_string()
});

handlebars_helper!(third_week: |day: u32, cal_data: CalendarData| {
    // + 1 + 14
    let offset_int: i32 = day as i32 - cal_data.first_day_of_month as i32 + 15 as i32;
    offset_int.to_string()
});

handlebars_helper!(fourth_week: |day: u32, cal_data: CalendarData| {
    // + 1 + 21
    let offset_int: i32 = day as i32 - cal_data.first_day_of_month as i32 + 22 as i32;
    offset_int.to_string()
});

handlebars_helper!(fifth_week: |day: u32, cal_data: CalendarData| {
    // + 1 + 28
    let offset_int: i32 = day as i32 - cal_data.first_day_of_month as i32 + 28 as i32;
    // FIXME Account for Feb & 30 day months
    if offset_int < 31 {
        offset_int.to_string()
    } else {
        "".to_string()
    }
});

handlebars_helper!(get_month_name: |month: u32| {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Oops",
    }
});

// Not Working

// handlebars_helper!(gen_vec_len_ten: |vec: EntityList<EntityType>| {
//     vec.list.len();
// });

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub enum EntityType {
//     Location(LocationList),
//     Consultant(ResponseConsultant),
// }

// fn vec_len_eq_ten<T>(vec: Vec<T>) -> bool {
//     if vec.len() == 10 {
//         true
//     } else {
//         false
//     }
// }
// #[derive(Debug, Serialize, Deserialize)]
// pub struct EntityList<T> {
//     list: Vec<T>,
// }

// handlebars_helper!(vec_len_ten: |vec: Vec<Entity>| {
//     match &vec[0] {
//         Entity::Location(first) => vec_len_eq_ten(vec),
//         Entity::Consultant(first) => vec_len_eq_ten(vec),
//     };
// });
