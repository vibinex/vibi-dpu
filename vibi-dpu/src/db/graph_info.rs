use sled::IVec;

use crate::{db::config::get_db, graph::graph_info::GraphInfo};
pub fn save_graph_info_to_db(review_key: &str, commit_id: &str, graph_info: &GraphInfo) {
    let db = get_db();
    let graph_info_key = format!("graph_info/{}/{}", review_key, commit_id);
    // Serialize repo struct to JSON 
    let json = serde_json::to_vec(graph_info).expect("Failed to serialize review");
    // Insert JSON into sled DB
    let insert_res = db.insert(IVec::from(graph_info_key.as_bytes()), json);
    if insert_res.is_err() {
        let e = insert_res.expect_err("No error in insert_res");
        log::error!("[save_graph_info_to_db] Failed to upsert graph info into sled DB: {e}");
        return;
    }
    log::debug!("[save_graph_info_to_db] Graph Info succesfully upserted: {:#?}", graph_info);
}

pub fn get_graph_info_from_db(review_key: &str, commit_id: &str) -> Option<GraphInfo> {
    let db = get_db();
    let graph_info_key = format!("graph_info/{}/{}", review_key, commit_id);
    let graph_info_res = db.get(IVec::from(graph_info_key.as_bytes()));
    if let Err(e) = graph_info_res {
        log::error!("[get_graph_info_from_db] GraphInfo key not found in db - {}, error: {:?}",
            &graph_info_key, e);
        return None;
    }
    let ivec_opt = graph_info_res.expect("Uncaught error in graph_info_res");
    log::debug!("[get_graph_info_from_db] ivec_opt: {:?}", ivec_opt);
    if ivec_opt.is_none() {
        log::error!("[get_graph_info_from_db] No graph info found for {}/{}", review_key, commit_id);
        return None;
    }
    let ivec = ivec_opt.expect("Empty ivec_opt");
    let graph_info_res = serde_json::from_slice(&ivec);
    if let Err(e) = graph_info_res {
        log::error!(
            "[get_graph_info_from_db] Failed to deserialize review from json: {:?}",
            e
        );
        return None;
    }
    let graph_info: GraphInfo = graph_info_res.expect("Uncaught error in graph_info_res");
    return Some(graph_info);
}