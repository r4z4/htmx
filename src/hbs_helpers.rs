use convert_case::{Case, Casing};
use handlebars::handlebars_helper;

use crate::{models::model_location::LocationList, Entity};
handlebars_helper!(to_title_case: |s: String| s.to_case(Case::Title));
handlebars_helper!(str_eq: |s_1: String, s_2: String| {
        if s_1 == s_2 {
            true
        } else {
            false
        }
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

handlebars_helper!(attachments_rte: |slug: String, entity_type_id: i32| {
    match entity_type_id {
        1 => String::from("/user/attachements/") + &slug,
        // Relative to /admin for admin
        2 => String::from("/user/attachements/") + &slug,
        // Admin & Subadmin same form (for now)
        3 => String::from("/user/attachments/") + &slug,
        4 => String::from("consultant/attachments/") + &slug,
        5 => String::from("location/attachments/") + &slug,
        6 => String::from("consult/attachments/") + &slug,
        7 => String::from("client/attachments/") + &slug,
        _ => String::from("form/attachments/") + &slug
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
        _ => String::from("Unknown"),
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