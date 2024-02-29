use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use super::relevance::Relevance;

#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct Coverage {
    provider: String,
    handle: String,
    coverage_num: f32,
}

impl Coverage {
    // Constructor
    pub fn new(
        provider: String,
        handle: String,
        coverage_num: f32,
    ) -> Self {
        Self {
            provider,
            handle,
            coverage_num,
        }
    }

    // Public getter methods
    pub fn provider(&self) -> &String {
        &self.provider
    }

    pub fn handle(&self) -> &String {
        &self.handle
    }

    pub fn coverage_str(&self) -> String {
        let coverage_str = format!("{:.2}", self.coverage_num);
        coverage_str
    }

    pub fn coverage_num(&self) -> f32 {
        self.coverage_num
    }

    pub fn update_coverage(&mut self, update: f32) {
        self.coverage_num = self.coverage_num + update;
    }
}

pub struct CoverageMap {
    provider: String,
    handle_map: HashMap<String, Coverage>,
    coverage_total: f32,
}

impl CoverageMap {
    // Constructor
    pub fn calculate_coverage_map(
        provider: String,
        reviewer_handles: Vec<String>,
        relevance_vec: Vec<Relevance>,
    ) -> Self {
        let mut coverage_map_obj = Self {
            provider,
            handle_map: HashMap::<String, Coverage>::new(),
            coverage_total: 0.0,
        };
        for relevance_obj in relevance_vec {
            let handles_opt = relevance_obj.handles();
            let relevance_num = relevance_obj.relevance_num();
            if handles_opt.is_none() {
                log::debug!("[process_approval] handles not in db for {}", relevance_obj.git_alias());
                continue;
            }
            let handles = handles_opt.to_owned().expect("Empty handles_opt");
            for handle in handles {
                if reviewer_handles.contains(&handle) {
                    coverage_map_obj.update_coverage(&handle, relevance_num);
                    break;
                }
            }
        }
        return coverage_map_obj;
    }

    // Public getter methods
    pub fn provider(&self) -> &String {
        &self.provider
    }

    pub fn handle_map(&self) -> &HashMap<String, Coverage> {
        &self.handle_map
    }

    pub fn coverage_total_str(&self) -> String {
        let coverage_total_str = format!("{:.2}", self.coverage_total);
        coverage_total_str
    }

    pub fn coverage_total(&self) -> f32 {
        self.coverage_total
    }

    pub fn update_coverage(&mut self, handle: &str, relevance: f32) {
        self.coverage_total += relevance;
        if let Some(coverage) = self.handle_map.get_mut(handle) {
            coverage.update_coverage(relevance);
            return;
        }
        let coverage = Coverage::new(
            self.provider.clone(), handle.to_owned(), relevance);
        self.handle_map.insert(handle.to_owned(), coverage);
    }
}